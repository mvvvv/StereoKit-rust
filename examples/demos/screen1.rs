use openxr_sys::SwapchainUsageFlags;
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use stereokit_rust::{
    font::Font,
    framework::Screen,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    prelude::*,
    sk::{MainThreadToken, SkInfo},
    sound::Sound,
    sprite::Sprite,
    system::{Assets, Backend, BackendXRType, Text, TextBuilder, TextStyle},
    tex::{Tex, TexFormat},
    tools::xr_comp_layers::{SwapchainSk, XrCompLayers},
    ui::{Ui, UiBtnLayout},
    util::{Color32, Color128, Time, named_colors::RED},
};

/// Demo test for Screen: sound playback (right / left) and a choice between
/// texture mode (default) or swapchain quad-layer mode.
#[derive(IStepper)]
pub struct Screen1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    screen: Screen,
    textures: Vec<Tex>,
    current_texture_index: usize,
    last_switch_time: f32,
    /// Shared with the `extra_param_ui` closure so the slider can mutate it.
    switch_interval: Arc<Mutex<f32>>,
    paused: bool,
    always_visible: bool,

    // Display mode: texture (default) or swapchain quad-layer
    use_swapchain: bool,
    swapchain_sk: Option<SwapchainSk>,
    tex_width: usize,
    tex_height: usize,

    sprite_prev: Sprite,
    sprite_next: Sprite,
    sprite_play: Sprite,
    sprite_pause: Sprite,
    radio_off: Sprite,
    radio_on: Sprite,

    window_pose: Pose,
    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Screen1 {}

impl Default for Screen1 {
    fn default() -> Self {
        let texture_paths = [
            "textures/exit.jpeg",
            "textures/log_viewer.jpeg",
            "textures/micro.jpeg",
            "textures/open_gltf.jpeg",
            "textures/screenshot.jpeg",
            "textures/sound.jpeg",
        ];

        let mut textures = Vec::new();
        for path in texture_paths {
            if let Ok(tex) = Tex::from_file(path, true, None) {
                textures.push(tex);
            }
        }

        let initial_texture = textures.first().map(|t| t.clone_ref()).unwrap_or_else(Tex::default);

        let mut screen = Screen::new("Screen1_demo", initial_texture);
        screen.resolution(1024, 1024).screen_orientation(Quat::Y_180);
        if textures.len() > 1 {
            screen.set_texture(1, Some(textures[1].clone_ref()));
        }

        Self {
            id: "Screen1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            screen,
            textures,
            current_texture_index: 0,
            last_switch_time: 0.0,
            switch_interval: Arc::new(Mutex::new(3.0)),
            paused: false,
            always_visible: false,

            use_swapchain: false,
            swapchain_sk: None,
            tex_width: 0,
            tex_height: 0,

            sprite_prev: Sprite::arrow_left(),
            sprite_next: Sprite::arrow_right(),
            sprite_play: Sprite::toggle_off(),
            sprite_pause: Sprite::toggle_on(),
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),

            window_pose: Pose::new(Vec3::new(0.35, 1.5, -0.6), Some(Quat::Y_180)),
            text: "Screen1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r(Vec3::NEG_Y * 1.7, Quat::look_dir(Vec3::new(-0.3, 0.0, 1.0))),
        }
    }
}

impl Screen1 {
    fn start(&mut self) -> bool {
        Assets::block_for_priority(i32::MAX);

        // Read texture dimensions from the first texture for swapchain creation.
        if let Some(tex) = self.textures.first()
            && let Some((w, h, _)) = tex.get_data_infos(0)
        {
            self.tex_width = w;
            self.tex_height = h;
        }

        // Register the slide-speed slider inside Screen's settings panel.
        let shared_interval = Arc::clone(&self.switch_interval);
        self.screen.set_extra_param_ui(move || {
            let mut interval = shared_interval.lock().unwrap();
            Ui::label("Slide speed").use_padding(true).draw();
            Ui::same_line();
            Ui::label(format!("{:.1}s", *interval)).use_padding(true).draw();
            Ui::same_line();
            Ui::hslider("slide_interval", &mut interval, 0.5, 10.0).interact();
        });

        // Create an OpenXR swapchain when the backend supports it.
        if Backend::xr_type() == BackendXRType::OpenXR
            && let Some(comp_layer) = XrCompLayers::new()
        {
            if let Some(handle) = comp_layer.try_make_swapchain(
                self.tex_width as u32,
                self.tex_height as u32,
                TexFormat::Rgba32Srgb,
                SwapchainUsageFlags::COLOR_ATTACHMENT,
                false,
            ) {
                self.swapchain_sk = SwapchainSk::wrap(
                    handle,
                    TexFormat::Rgba32Srgb,
                    self.tex_width as u32,
                    self.tex_height as u32,
                    Some(comp_layer),
                );
            } else {
                Log::warn("Screen1: Failed to create XR swapchain");
            }
        }
        true
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, token: &MainThreadToken) {
        let current_time = Time::get_totalf();
        let interval = *self.switch_interval.lock().unwrap();

        // Show current image index and time remaining until next switch in Screen's overlay.
        let n = self.textures.len();
        let remaining = (interval - (current_time - self.last_switch_time)).max(0.0);
        self.screen.set_overlay_text(format!(
            "{}/{} | {:.1}s / {:.1}s{}",
            self.current_texture_index + 1,
            n,
            remaining,
            interval,
            if self.paused { " [PAUSED]" } else { "" },
        ));

        // Auto-advance textures every switch_interval seconds.
        let slide_changed = !self.paused && current_time - self.last_switch_time > interval;
        if slide_changed {
            self.next_texture();
            self.last_switch_time = current_time;
        }

        // In swapchain mode, acquire+release every frame (required by OpenXR: any swapchain
        // referenced in a composition layer must be acquired and released in the same frame).
        // Re-render only when the slide changed; on other frames acquire+release with no draw.
        let mut swapchain_failed = false;
        if self.use_swapchain
            && let Some(sc) = &mut self.swapchain_sk
        {
            if let Err(e) = sc.acquire_image(None) {
                Log::warn(format!("Screen1: Failed to acquire swapchain image: {e}"));
                swapchain_failed = true;
            } else {
                if slide_changed && let Some(render_tex) = sc.get_render_target_mut() {
                    let idx = self.current_texture_index;
                    if let Some(tex) = self.textures.get(idx) {
                        if let Some((w, h, count)) = tex.get_data_infos(0) {
                            // The wrapped swapchain image has a fixed size: a slide with other
                            // dimensions could not be uploaded into it.
                            if w != self.tex_width || h != self.tex_height {
                                Log::warn(format!(
                                    "Screen1: slide {idx} is {w}x{h}, the swapchain is {}x{}: not rendered.",
                                    self.tex_width, self.tex_height
                                ));
                            } else {
                                let pixels = vec![Color32::default(); count];
                                if tex.get_color_data::<Color32>(&pixels, 0) {
                                    render_tex.set_colors32(w, h, &pixels);
                                } else {
                                    Log::warn(format!("Screen1: could not read the pixels of slide {idx}"));
                                }
                            }
                        }
                    } else {
                        Log::warn(format!("Screen1: No texture for index {idx}"));
                    }
                }
                if let Err(e) = sc.release_image() {
                    Log::warn(format!("Screen1: Failed to release swapchain image: {e}"));
                    swapchain_failed = true;
                } else {
                    let handle = sc.handle;
                    self.screen.set_swapchain(handle);
                }
            }
        }
        if swapchain_failed {
            self.disable_swapchain();
        }

        // Draw Screen: In swapchain mode, a composition cylinder or quad-layer is submitted instead of rendering the mesh.
        self.screen.draw(token);

        // Video-player transport controls anchored just above the screen via get_top().
        if self.screen.is_focused() {
            // The transport toolbar follows the screen zoom
            let scale = self.screen.get_ui_scale();
            let d = Screen::UI_REFERENCE_DISTANCE;
            let btn_size = Vec2::new(0.06 * d.sqrt(), 0.06 * d.sqrt()) * scale;
            let surface_size = Vec2::new(0.4 * d, 0.1 * d) * scale;
            let controls_pose = self.screen.get_top(Vec3::new(0.30 * d.sqrt() * scale, 0.0, 0.0));
            Ui::push_surface(controls_pose, Vec3::ZERO, surface_size);
            if Ui::button("prev")
                .image(&self.sprite_prev)
                .image_layout(UiBtnLayout::CenterNoText)
                .size(btn_size)
                .press()
            {
                self.prev_texture();
                self.last_switch_time = current_time;
            }
            Ui::same_line();
            if self.paused {
                if Ui::button("play").image(&self.sprite_play).image_layout(UiBtnLayout::Center).size(btn_size).press()
                {
                    self.paused = false;
                    self.last_switch_time = current_time;
                }
            } else {
                Ui::push_tint(Color128::new(0.3, 1.0, 0.3, 1.0));
                let clicked = Ui::button("pause")
                    .image(&self.sprite_pause)
                    .image_layout(UiBtnLayout::Center)
                    .size(btn_size)
                    .press();
                Ui::pop_tint();
                if clicked {
                    self.paused = true;
                }
            }
            Ui::same_line();
            if Ui::button("next")
                .image(&self.sprite_next)
                .image_layout(UiBtnLayout::CenterNoText)
                .size(btn_size)
                .press()
            {
                self.next_texture();
                self.last_switch_time = current_time;
            }
            Ui::pop_surface();
        }

        // Control window
        Ui::window("Screen1").pose(&mut self.window_pose).size(Vec2::new(0.24, 0.0)).begin();

        if Ui::button("Sound Left").press() {
            let (left_id, _) = self.screen.get_sound_ids();
            if let Ok(mut stream) = Sound::find(left_id) {
                let samples: Vec<f32> =
                    (0..48000).map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5).collect();
                stream.write_samples(&samples, None);
            }
        }
        Ui::same_line();
        if Ui::button("Sound Right").press() {
            let (_, right_id) = self.screen.get_sound_ids();
            if let Ok(mut stream) = Sound::find(right_id) {
                let samples: Vec<f32> =
                    (0..48000).map(|i| (i as f32 * 880.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5).collect();
                stream.write_samples(&samples, None);
            }
        }

        Ui::hseparator();

        // Display mode: Texture (default) or Swapchain quad-layer
        let want_swapchain = self.use_swapchain;
        if Ui::radio("Texture", !want_swapchain)
            .images(&self.radio_off, &self.radio_on)
            .image_layout(UiBtnLayout::Left)
            .press()
            && want_swapchain
        {
            // Back to the mesh rendering: the Screen must stop submitting the composition layer.
            self.use_swapchain = false;
            self.screen.clear_swapchain();
        }
        Ui::same_line();
        if Ui::radio("Swapchain", want_swapchain)
            .images(&self.radio_off, &self.radio_on)
            .image_layout(UiBtnLayout::Left)
            .press()
            && !want_swapchain
        {
            if let Some(sc) = &self.swapchain_sk {
                let handle = sc.handle;
                self.screen.set_swapchain(handle);
                self.use_swapchain = true;
            } else {
                Log::warn(
                    "Screen1: no swapchain handle: it is created in start() and requires OpenXR (it is \
                           dropped for good after a swapchain failure)",
                );
            }
        }

        // Focus-driven toolbars may be locked
        if let Some(always_visible) = Ui::toggle("Toolbars always visible", &mut self.always_visible).interact() {
            self.screen.always_visible(always_visible);
        }

        Ui::window_end();

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Stop spatial audio streams, clear the swapchain reference in Screen.
            self.screen.shutdown();
            self.disable_swapchain();
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }

    /// Drop the swapchain and tell the Screen to stop submitting a composition layer. The two must always go together:
    /// dropping `SwapchainSk` destroys the underlying OpenXR swapchain, and `Screen::draw` would keep submitting that
    /// dead handle until `clear_swapchain` is called.
    fn disable_swapchain(&mut self) {
        self.screen.clear_swapchain();
        self.use_swapchain = false;
        self.swapchain_sk = None;
    }

    fn next_texture(&mut self) {
        if self.textures.is_empty() {
            return;
        }
        self.current_texture_index = (self.current_texture_index + 1) % self.textures.len();
        self.update_display();
    }

    fn prev_texture(&mut self) {
        if self.textures.is_empty() {
            return;
        }
        self.current_texture_index = (self.current_texture_index + self.textures.len() - 1) % self.textures.len();
        self.update_display();
    }

    /// Update the screen texture slot.
    fn update_display(&mut self) {
        if self.current_texture_index >= self.textures.len() {
            return;
        }
        let new_tex = self.textures[self.current_texture_index].clone_ref();
        let tex_slot = self.current_texture_index % 2;
        self.screen.set_texture(tex_slot, Some(new_tex.clone_ref()));
        self.screen.set_tex_curr(tex_slot);
    }
}

#[allow(dead_code)]
fn id() -> StepperId {
    "Screen1".to_string()
}
