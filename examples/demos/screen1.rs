use openxr_sys::SwapchainUsageFlags;
use std::rc::Rc;
use stereokit_rust::{
    font::Font,
    framework::Screen,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    prelude::*,
    sk::{MainThreadToken, SkInfo},
    sound::Sound,
    sprite::Sprite,
    system::{Assets, Backend, BackendXRType, Text, TextStyle},
    tex::{Tex, TexFormat},
    tools::xr_comp_layers::{SwapchainSk, XrCompLayers},
    ui::{Ui, UiBtnLayout},
    util::{Color32, Time, named_colors::RED},
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
    switch_interval: f32,

    // Display mode: texture (default) or swapchain quad-layer
    use_swapchain: bool,
    swapchain_sk: Option<SwapchainSk>,
    tex_width: usize,
    tex_height: usize,

    window_pose: Pose,
    pub text: String,
    pub text_style: Option<TextStyle>,
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

        let mut screen = Screen::new("screen1_demo", initial_texture);
        screen.screen_orientation([0.0, 180.0, 0.0]).resolution(1024, 1024);
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
            switch_interval: 3.0,

            use_swapchain: false,
            swapchain_sk: None,
            tex_width: 0,
            tex_height: 0,

            window_pose: Pose::new(Vec3::new(-0.3, 1.5, -0.6), Some(Quat::Y_180)),
            text: "Screen1".to_owned(),
            text_style: None,
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::Y_180),
        }
    }
}

impl Screen1 {
    fn start(&mut self) -> bool {
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));

        // Read texture dimensions from the first texture for swapchain creation.
        Assets::block_for_priority(i32::MAX);
        if let Some(tex) = self.textures.first()
            && let Some((w, h, _)) = tex.get_data_infos(0)
        {
            self.tex_width = w;
            self.tex_height = h;
        }

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

        // Auto-advance textures every switch_interval seconds.
        if current_time - self.last_switch_time > self.switch_interval {
            self.next_texture();
            self.last_switch_time = current_time;

            // In swapchain mode, copy the current texture pixels directly into the swapchain image.
            if self.use_swapchain
                && let Some(sc) = &mut self.swapchain_sk
            {
                if let Err(e) = sc.acquire_image(None) {
                    Log::warn(format!("Screen1: Failed to acquire swapchain image: {e}"));
                    self.swapchain_sk = None;
                } else if let Some(render_tex) = sc.get_render_target_mut() {
                    let idx = self.current_texture_index;
                    if let Some(tex) = self.textures.get(idx) {
                        Log::info(format!(
                            "Screen1: format {:?}, dimensions {}x{}",
                            tex.get_format(),
                            tex.get_width().unwrap(),
                            tex.get_height().unwrap()
                        ));
                        if let Some((w, h, count)) = tex.get_data_infos(0) {
                            let pixels = vec![Color32::default(); count];
                            tex.get_color_data::<Color32>(&pixels, 0);
                            render_tex.set_colors32(w, h, &pixels);
                        }
                    } else {
                        Log::warn(format!("Screen1: No texture for index {idx}"));
                    }
                    if let Err(e) = sc.release_image() {
                        Log::warn(format!("Screen1: Failed to release swapchain image: {e}"));
                        self.swapchain_sk = None;
                    } else {
                        let handle = sc.handle;
                        self.screen.set_swapchain(handle);
                    }
                }
            }
        }

        // Draw the Screen: submits a quad-layer if a swapchain is set, otherwise renders the mesh.
        self.screen.draw(token);

        // Control window
        Ui::window_begin("Screen1", &mut self.window_pose, Some(Vec2::new(0.24, 0.0)), None, None);

        // Sound buttons — write a 1-second beep into Screen's spatial audio streams.
        if Ui::button("Sound Left", None) {
            let (left_id, _) = self.screen.get_sound_ids();
            if let Ok(stream) = Sound::find(left_id) {
                let samples: Vec<f32> =
                    (0..48000).map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5).collect();
                stream.write_samples(&samples, None);
            }
        }
        Ui::same_line();
        if Ui::button("Sound Right", None) {
            let (_, right_id) = self.screen.get_sound_ids();
            if let Ok(stream) = Sound::find(right_id) {
                let samples: Vec<f32> =
                    (0..48000).map(|i| (i as f32 * 880.0 * 2.0 * std::f32::consts::PI / 48000.0).sin() * 0.5).collect();
                stream.write_samples(&samples, None);
            }
        }

        Ui::hseparator();

        // Display mode: Texture (default) or Swapchain quad-layer
        let want_swapchain = self.use_swapchain;
        if Ui::radio_img("Texture", !want_swapchain, Sprite::radio_off(), Sprite::radio_on(), UiBtnLayout::Left, None)
            && want_swapchain
        {
            self.use_swapchain = false;
            self.screen.clear_swapchain();
        }
        Ui::same_line();
        if Ui::radio_img("Swapchain", want_swapchain, Sprite::radio_off(), Sprite::radio_on(), UiBtnLayout::Left, None)
            && !want_swapchain
        {
            if let Some(sc) = &self.swapchain_sk {
                let handle = sc.handle;
                self.screen.set_swapchain(handle);
                self.use_swapchain = true;
            } else {
                Log::warn("Screen1: Swapchain not available (requires OpenXR)");
            }
        }

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Stop spatial audio streams, clear swapchain reference in Screen.
            self.screen.shutdown();
            // Drop SwapchainSk: its Drop impl calls XrCompLayers::destroy_swapchain once.
            self.swapchain_sk = None;
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }

    fn next_texture(&mut self) {
        if self.textures.is_empty() {
            return;
        }
        self.current_texture_index = (self.current_texture_index + 1) % self.textures.len();
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
