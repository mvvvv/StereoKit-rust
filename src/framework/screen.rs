use std::f32::consts::PI;

use crate::util::named_colors;
use crate::{
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Ray, Vec2, Vec3},
    mesh::{Inds, Mesh, Vertex},
    render::Renderer,
    sk::MainThreadToken,
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{Align, Input, Lines, TextFit},
    tex::Tex,
    ui::{Ui, UiBtnLayout, UiMove, UiWin},
};
use crate::{maths::Rect, tools::xr_comp_layers::XrCompLayers};
use openxr_sys::Swapchain;

/// Values derived from screen parameters and pose that are constant between parameter/pose changes.
/// Cached to avoid redundant computation inside [`Screen::draw_swapchain`] every frame.
/// - Layer-param fields are updated by [`Screen::adapt_screen`].
/// - Pose fields (`local_offset`, `layer_orientation`) are updated by [`Screen::update_pose_cache`],
///   which is called only when `screen_pose.position` changes.
#[derive(Clone, Copy)]
struct SwapchainLayerCache {
    // --- layer-param fields (updated by adapt_screen) ---
    rect: Rect,
    bounds_center_z: f32,
    use_cylinder: bool,
    radius: f32,
    central_angle: f32,
    aspect_ratio: f32,
    // --- pose fields (updated by update_pose_cache) ---
    /// `screen_pose.orientation * Vec3(0, 0, bounds_center_z)` — offset from position to layer centre.
    local_offset: Vec3,
    /// Orientation for the XR layer, derived from `screen_pose` orientation.
    layer_orientation: Quat,
}

impl Default for SwapchainLayerCache {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            bounds_center_z: 0.0,
            use_cylinder: false,
            radius: 1.0,
            central_angle: 1.0,
            aspect_ratio: 1.0,
            local_offset: Vec3::ZERO,
            layer_orientation: Quat::IDENTITY,
        }
    }
}

pub struct ScreenRepo {
    id_btn_show_hide_param: String,
    id_window_param: String,
    show_param: bool,
    sprite_hide_param: Sprite,
    sprite_show_param: Sprite,
    id_handle: String,
    id_material: String,
    // id_left_sound: String,
    // id_right_sound: String,
    id_slider_distance: String,
    id_slider_size: String,
    id_slider_flattening: String,
}

impl ScreenRepo {
    pub fn new(id: String) -> Self {
        Self {
            show_param: false,
            sprite_hide_param: Sprite::close(),
            sprite_show_param: Sprite::from_file("icons/hamburger.png", None, None).unwrap_or_default(),
            id_btn_show_hide_param: id.clone() + "_btn_show_hide",
            id_window_param: id.clone() + "_window_param",
            id_handle: id.clone() + "_handle",
            id_material: id.clone() + "_material",
            // id_left_sound: id.clone() + "_left_sound",
            // id_right_sound: id.clone() + "_right_sound",
            id_slider_distance: id.clone() + "_slider_distance",
            id_slider_size: id.clone() + "_slider_size",
            id_slider_flattening: id.clone() + "_slider_radius",
        }
    }
}

/// A virtual curved screen that can display a [`Tex`] or an OpenXR swapchain quad/cylinder layer.
///
/// The screen is a concave spherical mesh whose curvature, diagonal, and distance from
/// the viewer are adjustable at runtime. It ships with:
/// * a grab handle (drag to reposition)
/// * a hamburger settings panel (distance, diagonal, curvature sliders)
/// * two spatial stereo audio streams (left / right)
/// * an optional single-line overlay text rendered above the content
/// * an optional extra-param UI callback injected into the settings panel
///
/// Two texture slots (`0` and `1`) allow cross-fading between images without dropping GPU handles.
/// Use [`Screen::set_texture`] to upload a new frame into the inactive slot, then
/// [`Screen::set_tex_curr`] to flip to it.
///
/// For OpenXR deployments, plug in a [`crate::tools::xr_comp_layers::SwapchainSk`] handle via
/// [`Screen::set_swapchain`] to submit a composition quad/cylinder layer instead of rendering the mesh —
/// this bypasses the StereoKit render pipeline and gives compositor-level reprojection.
///
/// See the `screen1` demo for a full example with a slideshow, transport controls, and optional
/// swapchain quad/cylinder layer rendering.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{framework::Screen, tex::{Tex, TexFormat, TexType},
///                      util::named_colors::DOGER_BLUE};
///
/// // Create a solid-colour texture to display on the screen
/// let tex = Tex::gen_color(DOGER_BLUE, 64, 36, TexType::Image, TexFormat::Rgba32Srgb);
///
/// // Build the screen — default distance is 2.2 m, default diagonal ≈ 4.4 m
/// let mut screen = Screen::new("doc_screen", &tex);
///
/// // Bring the screen close, give it a tight diagonal, slight curvature, and an overlay
/// screen.resolution(320,240)
///       .screen_distance(2.3)   // 2.3 m away from the viewer
///       .screen_diagonal(1.2)   // 1.2 m diagonal (compact)
///       .set_overlay_text("Hello, Screen!");
///
/// filename_scr = "screenshots/screen.jpeg"; fov_scr = 20.0;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     screen.draw(&token);
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screen.jpeg" alt="screenshot" width="200">
pub struct Screen {
    repo: ScreenRepo,
    width: u32,
    height: u32,
    screen_distance: f32,
    /// When `true` (default), screen is a cylinder. Curvature slider snaps to `0.0` (flat) or `1.0` (cylinder).
    /// When `false`, screen is spherical and any value in `[0.0, 1.0]` is accepted for intermediate curvatures.
    cylindrical: bool,
    /// Shape of the screen: `0.0` = flat plane (quad layer), `1.0` = cylinder or spherical.
    curvature: f32,
    screen_size: Vec2,
    screen_diagonal: f32,
    screen_pose: Pose,
    screen: Mesh,
    sound_spacing_factor: f32,
    ray_thickness: f32,

    screen_material: Material,
    screen_textures: [Option<Tex>; 2],
    tex_curr: usize,

    sound_left: Sound,
    sound_left_inst: Option<SoundInst>,
    sound_right: Sound,
    sound_right_inst: Option<SoundInst>,

    openxr_swapchain: Option<Swapchain>,
    layer_cache: SwapchainLayerCache,

    /// Text displayed via [`Ui::text_at`] inside the control window on every frame.
    /// Set with [`Screen::set_overlay_text`]. Empty string disables the display.
    overlay_text: String,

    /// Optional callback invoked at the end of the params panel (when the hamburger menu is open).
    /// Set with [`Screen::set_extra_param_ui`].
    extra_param_ui: Option<Box<dyn FnMut() + Send + 'static>>,

    /// When `true`, the hamburger settings button is not rendered.
    /// Set with [`Screen::hide_hamburger`]. Defaults to `false`.
    hide_hamburger: bool,
}

unsafe impl Send for Screen {}

/// All the code here run in the main thread
impl Screen {
    pub const MAX_DISTANCE: f32 = 6.0;
    pub const MAX_DIAGONAL: f32 = 15.0;
    pub const MIN_DIAGONAL: f32 = 0.2;

    /// Create the screen
    pub fn new(id: &str, screen_tex: impl AsRef<Tex>) -> Self {
        let width = 3840u32;
        let height = 2160u32;
        let screen_size = Vec2::new(width as f32 / 1000.0, height as f32 / 1000.0);
        let screen_diagonal = (screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt();
        let screen_material = Material::unlit().copy();

        let mut this = Self {
            repo: ScreenRepo::new(id.to_string()),

            width,
            height,
            screen_distance: 2.20,
            curvature: 1.0,
            screen_size,
            screen_diagonal,
            screen_pose: Pose::IDENTITY,
            screen: Mesh::new(),
            sound_spacing_factor: 3.0,
            ray_thickness: 0.005,

            screen_material,
            screen_textures: [None, None],
            tex_curr: 0,

            sound_left: Sound::click(),
            sound_left_inst: None,
            sound_right: Sound::click(),
            sound_right_inst: None,

            openxr_swapchain: None,
            layer_cache: SwapchainLayerCache::default(),

            overlay_text: String::new(),
            extra_param_ui: None,
            hide_hamburger: false,
            cylindrical: true,
        };

        let screen_tex = screen_tex.as_ref().clone_ref();

        this.screen_textures[0] = Some(screen_tex.clone_ref());
        this.screen_material.id(&this.repo.id_material);
        this.update_material_texture();

        this.sound_left = Sound::create_stream(2.0).unwrap_or_default();
        this.sound_right = Sound::create_stream(2.0).unwrap_or_default();

        this.screen_pose = Input::get_head() * Matrix::Y_180;
        this.update_pose_cache(Input::get_head().position);
        this.adapt_screen();

        this.sound_left_inst = Some(this.sound_left.play(this.sound_position(-1), Some(1.0)));
        this.sound_right_inst = Some(this.sound_right.play(this.sound_position(1), Some(1.0)));

        this
    }

    /// Set the screen distance
    pub fn screen_distance(&mut self, distance: f32) -> &mut Self {
        let max_size = distance * PI;
        let screen_size = self.screen_size;
        if screen_size.x > max_size || self.screen_size.y > max_size {
            // self.screen_distance = old_value;
        } else {
            let min_distance = (self.screen_size.x.max(self.screen_size.y)) / PI;
            self.screen_distance = distance.max(min_distance).min(Self::MAX_DISTANCE);
            self.adapt_screen();
        }
        self
    }

    /// Set the curvature of the screen: `0.0` = flat plane (quad layer), `1.0` = tight cylinder
    /// (cylinder layer) matching the XR cylinder composition layer. Intermediate values produce a
    /// wider-radius cylinder and are only accepted when `cylindrical` is `false`.
    pub fn curvature(&mut self, curvature: f32) -> &mut Self {
        self.curvature =
            if self.cylindrical { if curvature >= 0.5 { 1.0 } else { 0.0 } } else { curvature.clamp(0.0, 1.0) };
        self.adapt_screen();
        self
    }

    /// When `true` (default), the curvature slider only allows `0.0` (flat) or `1.0` (cylinder).
    /// When `false`, the slider accepts any value in `[0.0, 1.0]` for intermediate curvatures.
    /// Reapplies the current curvature value under the new mode.
    pub fn cylindrical(&mut self, cylindrical: bool) -> &mut Self {
        self.cylindrical = cylindrical;
        // re-snap or re-clamp the current curvature value
        let cur = self.curvature;
        self.curvature(cur)
    }

    /// Set the screen orientation and recompute the pose-dependent layer cache fields.
    pub fn screen_orientation(&mut self, orientation: impl Into<Quat>) -> &mut Self {
        self.screen_pose.orientation = orientation.into();
        let local_offset = self.screen_pose.orientation * Vec3::new(0.0, 0.0, self.layer_cache.bounds_center_z);
        let layer_orientation = Quat::look_at(Vec3::ZERO, local_offset, Some(self.screen_pose.get_up()));
        self.layer_cache.local_offset = local_offset;
        self.layer_cache.layer_orientation = layer_orientation;
        self
    }

    /// Set the screen size
    pub fn screen_size(&mut self, size: impl Into<Vec2>) -> &mut Self {
        let size = size.into();
        let max_size = self.screen_distance * PI;
        let screen_diagonal = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        if size.x <= max_size
            && size.y <= max_size
            && size.x > 0.0
            && size.y > 0.0
            && screen_diagonal > Self::MIN_DIAGONAL
        {
            self.screen_size = size;
            self.screen_diagonal = screen_diagonal;
            self.adapt_screen();
        }
        self
    }

    /// Set the screen diagonal (automatically adjusts screen_size proportionally)
    pub fn screen_diagonal(&mut self, diagonal: f32) -> &mut Self {
        let max_size = self.screen_distance * PI;
        let screen_size = self.screen_size * diagonal / self.screen_diagonal;
        if screen_size.x > max_size || screen_size.y > max_size {
            // self.screen_diagonal = old_value;
        } else {
            self.screen_size = screen_size;
            self.screen_diagonal = diagonal;

            self.adapt_screen();
        }
        self
    }

    /// Set the sound spacing factor
    pub fn sound_spacing_factor(&mut self, factor: f32) -> &mut Self {
        self.sound_spacing_factor = factor;
        self
    }

    /// Set the ray thickness
    pub fn ray_thickness(&mut self, thickness: f32) -> &mut Self {
        self.ray_thickness = thickness.max(0.001);
        self
    }

    /// Set text to display inside the control window on every frame via [`Ui::text`].
    /// Pass an empty string to clear.
    ///
    /// # Example
    /// ```ignore
    /// screen.set_overlay_text(format!("{:.0} FPS", fps));
    /// ```
    pub fn set_overlay_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.overlay_text = text.into();
        self
    }

    /// Hide or show the hamburger settings button. Defaults to `false` (visible). If you hide it you'll have to manage
    /// the screen parameters via your own UI, but this can be useful if you want a more permanent control panel or
    /// want to avoid accidental adjustments.
    pub fn hide_hamburger(&mut self, hide: bool) -> &mut Self {
        self.hide_hamburger = hide;
        self
    }

    /// Register a closure that will be called at the end of the params panel (hamburger menu).
    /// Use it to append extra sliders, toggles, or labels without subclassing `Screen`.
    ///
    /// The closure is called on every frame while the panel is open, via `Option::take` to
    /// avoid borrow conflicts with the rest of `Screen`.
    ///
    /// # Example
    /// ```ignore
    /// screen.set_extra_param_ui(move || {
    ///     Ui::label("Quality", None, true);
    ///     Ui::same_line();
    ///     Ui::hslider("quality", &mut quality, 0.0, 1.0).interact();
    /// });
    /// ```
    pub fn set_extra_param_ui(&mut self, f: impl FnMut() + Send + 'static) -> &mut Self {
        self.extra_param_ui = Some(Box::new(f));
        self
    }

    /// Set the current texture index (0 or 1)
    pub fn set_tex_curr(&mut self, tex_index: usize) -> &mut Self {
        if tex_index < 2 {
            self.tex_curr = tex_index;
            self.update_material_texture();
        }
        self
    }

    /// Set a texture at the specified index (0 or 1)
    pub fn set_texture(&mut self, index: usize, texture: Option<Tex>) -> &mut Self {
        if index < 2 {
            self.screen_textures[index] = texture;
            if index == self.tex_curr {
                self.update_material_texture();
            }
        }
        self
    }

    /// Update the material's diffuse texture based on the current texture index
    fn update_material_texture(&mut self) {
        if let Some(ref texture) = self.screen_textures[self.tex_curr] {
            self.screen_material.diffuse_tex(texture);
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    pub fn draw(&mut self, _token: &MainThreadToken) {
        let screen_transform = self.screen_param();

        // When the param menu is open, always render the mesh so the user can see shape changes.
        // Otherwise, prefer the swapchain quad/cylinder layer when one is set.
        if self.repo.show_param || !self.draw_swapchain() {
            Renderer::add_mesh(&self.screen, &self.screen_material, screen_transform, None, None);
        }
    }

    /// Here is managed the screen position, its rotundity, size and distance
    fn screen_param(&mut self) -> Matrix {
        const GRAB_X_MARGIN: f32 = 0.4;

        let bounds = self.screen.get_bounds();

        let factor_size = self.factor_size();

        let grab_position = Vec3::new(
            0.0, //
            self.screen_size.y / 2.0 + 0.05 * factor_size,
            bounds.center.z,
        );
        let grab_dimension = Vec3::new(
            factor_size * 0.2, //
            factor_size * 0.01,
            factor_size * 0.01,
        );
        if Ui::handle(&self.repo.id_handle, &mut self.screen_pose, Bounds::new(grab_position, grab_dimension))
            .draw_handle(true)
            .grab()
        {
            let head = Input::get_head();
            self.update_pose_cache(head.position);
        }

        let screen_transform = self.screen_pose.to_matrix(None);
        let d = self.screen_distance.sqrt(); // Adjust the UI element sizes based on distance.

        if self.repo.show_param {
            let info_position = Vec3::new(bounds.center.x, bounds.center.y, GRAB_X_MARGIN * 1.5);
            let mut window_pose = Pose::new(info_position, None) * screen_transform;
            Ui::window(&self.repo.id_window_param)
                .pose(&mut window_pose)
                .size(Vec2::new(0.4, 0.2))
                .window_type(UiWin::Body)
                .move_type(UiMove::None)
                .begin();

            if Ui::button(&self.repo.id_btn_show_hide_param)
                .image(&self.repo.sprite_hide_param)
                .image_layout(UiBtnLayout::CenterNoText)
                .press()
            {
                self.repo.show_param = false;
            }
            Ui::label("Distance").use_padding(true).draw();
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_distance)).use_padding(true).draw();
            Ui::same_line();
            let mut screen_distance = self.screen_distance;
            if let Some(new_value) = Ui::hslider(
                &self.repo.id_slider_distance,
                &mut screen_distance,
                GRAB_X_MARGIN * 2.0,
                Self::MAX_DISTANCE,
            )
            .interact()
            {
                self.screen_distance(new_value);
            }

            Ui::label("Diagonal").use_padding(true).draw();
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_diagonal)).use_padding(true).draw();
            Ui::same_line();
            let mut screen_diagonal = self.screen_diagonal;
            if let Some(new_value) =
                Ui::hslider(&self.repo.id_slider_size, &mut screen_diagonal, Self::MIN_DIAGONAL, Self::MAX_DIAGONAL)
                    .interact()
            {
                self.screen_diagonal(new_value);
            }

            Ui::label("Curvature").use_padding(true).draw();
            Ui::same_line();
            Ui::label(format!("{:.2}", self.curvature)).use_padding(true).draw();
            Ui::same_line();
            let mut curvature = self.curvature;
            let step = if self.cylindrical { 1.0 } else { 0.0 };
            if let Some(new_value) =
                Ui::hslider(&self.repo.id_slider_flattening, &mut curvature, 0.0, 1.0).step(step).interact()
            {
                self.curvature(new_value);
            }

            // Invoke the user-supplied extra parameter UI (e.g. quality sliders, mode toggles).
            // Uses `take` + restore to avoid a simultaneous borrow of `self`.
            if let Some(mut f) = self.extra_param_ui.take() {
                f();
                self.extra_param_ui = Some(f);
            }

            Ui::window_end();
        } else if !self.hide_hamburger {
            let info_position = Vec3::new(
                0.0, //
                self.screen_size.y / 2.0 + 0.04 * factor_size,
                bounds.center.z,
            );
            let button_pose = Pose::new(info_position, None) * screen_transform;
            let btn_size = Vec2::new(0.06 * d, 0.06 * d);
            let surface_size = btn_size * 1.1;
            Ui::push_surface(button_pose, Vec3::X * 0.02 * d, surface_size);
            if Ui::button(&self.repo.id_btn_show_hide_param)
                .image(&self.repo.sprite_show_param)
                .image_layout(UiBtnLayout::CenterNoText)
                .size(btn_size)
                .press()
            {
                self.repo.show_param = true;
                let head = Input::get_head();
                self.update_pose_cache(head.position);
            }
            Ui::pop_surface();
        }

        // Overlay text — rendered on its own surface anchored above the screen centre,
        // independent of the control window state.
        if !self.overlay_text.is_empty() {
            let overlay_y = self.screen_size.y / 2.0 + 0.04 * factor_size;
            let overlay_pos = Vec3::new(-0.05, overlay_y, bounds.center.z);
            let overlay_pose = Pose::new(overlay_pos, None) * screen_transform;
            Ui::push_surface(overlay_pose, Vec3::X * -0.01 * d, Vec2::ZERO);
            Ui::text(&self.overlay_text)
                .size(Vec2::new(0.15, 0.04) * self.screen_distance)
                .text_align(Align::Center)
                .fit(TextFit::Exact)
                .draw();
            Ui::pop_surface();
        }

        screen_transform
    }

    /// Submit a quad/cylinder layer using the OpenXR swapchain if one is set.
    /// Returns `true` if the frame was submitted via swapchain (mesh rendering should be skipped).
    fn draw_swapchain(&mut self) -> bool {
        if let Some(swapchain) = &self.openxr_swapchain {
            let cache = self.layer_cache;
            // Only per-frame computation: translate cached local_offset by current position.
            let at = self.screen_pose.position + cache.local_offset;

            if cache.use_cylinder {
                let cylinder_position = at + cache.layer_orientation * Vec3::new(0.0, 0.0, cache.radius);
                let cylinder_pose = Pose::new(cylinder_position, Some(cache.layer_orientation));
                XrCompLayers::submit_cylinder_layer(
                    cylinder_pose,
                    cache.radius,
                    cache.central_angle,
                    cache.aspect_ratio,
                    *swapchain,
                    cache.rect,
                    0,
                    -1,
                    None,
                    None,
                );
            } else {
                let swapchain_pose = Pose::new(at, Some(cache.layer_orientation));
                XrCompLayers::submit_quad_layer(
                    swapchain_pose,
                    self.screen_size,
                    *swapchain,
                    cache.rect,
                    0,
                    -1,
                    None,
                    None,
                );
            }
            return true;
        }
        false
    }

    /// Calculate sound position. If factor < 0 this is for left else for right
    fn sound_position(&self, factor: i8) -> Vec3 {
        let up = self.screen_pose.get_up();
        let forward = self.screen_pose.get_forward();
        let cross = Vec3::cross(up, forward);
        cross * factor as f32 * self.sound_spacing_factor
    }

    fn adapt_screen(&mut self) {
        let radius = if self.curvature <= 0.0 { f32::MAX } else { self.screen_distance / self.curvature };
        if self.curvature <= 0.0 {
            self.adapt_screen_spherical();
        } else {
            self.adapt_screen_cylinder(radius);
        }

        let bounds = self.screen.get_bounds();
        self.layer_cache = SwapchainLayerCache {
            rect: Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
            bounds_center_z: bounds.center.z,
            use_cylinder: self.curvature > 0.0,
            radius,
            central_angle: self.screen_size.x / radius,
            aspect_ratio: self.screen_size.x / self.screen_size.y,
            // pose fields computed from current orientation below
            local_offset: Vec3::ZERO,
            layer_orientation: Quat::IDENTITY,
        };
        let local_offset = self.screen_pose.orientation * Vec3::new(0.0, 0.0, self.layer_cache.bounds_center_z);
        self.layer_cache.layer_orientation = Quat::look_at(Vec3::ZERO, local_offset, Some(self.screen_pose.get_up()));
        self.layer_cache.local_offset = local_offset;
    }

    /// Build a cylindrical mesh with the given `radius`. `radius = screen_distance / curvature`.
    /// At `curvature = 1.0` the mesh exactly matches the XR cylinder composition layer geometry.
    fn adapt_screen_cylinder(&mut self, radius: f32) {
        let central_angle = self.screen_size.x / radius;
        let height = self.screen_size.y;

        let subdiv_u = 60u32;
        let subdiv_v = 30u32;
        let cols = subdiv_u + 1;

        let mut verts: Vec<Vertex> = vec![];
        let mut inds: Vec<Inds> = vec![];

        for j in 0..=subdiv_v {
            let t_v = j as f32 / subdiv_v as f32;
            let y = -height / 2.0 + t_v * height;
            for i in 0..=subdiv_u {
                let t_u = i as f32 / subdiv_u as f32;
                let angle = -central_angle / 2.0 + t_u * central_angle;
                let x = radius * angle.sin();
                let z = radius * angle.cos();
                // inward-pointing normal (concave face toward the viewer at origin)
                let normal = Vec3::new(-angle.sin(), 0.0, -angle.cos());
                verts.push(Vertex::new(Vec3::new(x, y, z), normal, Some(Vec2::new(1.0 - t_u, 1.0 - t_v)), None));

                if i < subdiv_u && j < subdiv_v {
                    let a = j * cols + i;
                    let b = j * cols + i + 1;
                    let c = (j + 1) * cols + i;
                    let d = (j + 1) * cols + i + 1;
                    // double-sided: push each triangle in both windings
                    inds.push(a);
                    inds.push(b);
                    inds.push(c);
                    inds.push(a);
                    inds.push(c);
                    inds.push(b);
                    inds.push(b);
                    inds.push(d);
                    inds.push(c);
                    inds.push(b);
                    inds.push(c);
                    inds.push(d);
                }
            }
        }

        let mut mesh = Mesh::new();
        mesh.set_data(verts.as_slice(), inds.as_slice(), None, None);
        self.screen = mesh;
    }

    fn adapt_screen_spherical(&mut self) {
        let distance = self.screen_distance;
        let flattening = 500.0_f32; // legacy: always flat sphere, kept for reference
        let radius = distance + flattening;

        let width = self.screen_size.x;
        let height = self.screen_size.y;

        self.screen = {
            let mut verts: Vec<Vertex> = vec![];
            let mut inds: Vec<Inds> = vec![];

            let aspect_ratio = width / height;

            let perimeter = 2.0 * PI * radius;

            let subdiv_v = 30u32;
            let subdiv_u = (subdiv_v as f32 * aspect_ratio) as u32;

            let angle_v = 2.0 * PI * height / perimeter;
            let angle_u = 2.0 * PI * width / perimeter;
            let delta_v = angle_v / subdiv_v as f32;
            let delta_u = angle_u / subdiv_u as f32;

            for j in 0..subdiv_v {
                let v = -angle_v / 2.0 + (j as f32 * delta_v) + PI / 2.0;
                for i in 0..subdiv_u {
                    let u = -angle_u / 2.0 + (i as f32 * delta_u) + PI / 2.0;
                    let x = radius * v.sin() * u.cos();
                    let y = radius * v.cos();
                    let z = radius * v.sin() * u.sin() - flattening;

                    verts.push(Vertex::new(
                        Vec3::new(x, y, z), //
                        Vec3::FORWARD,
                        Some(Vec2::new(i as f32 / (subdiv_u - 1) as f32, j as f32 / (subdiv_v - 1) as f32)),
                        None,
                    ));

                    //Log::diag(format!("vertex: {} {} {}", x, y, z));

                    let nb_row = subdiv_u;
                    let last_line = j == subdiv_v - 1;
                    if !last_line {
                        let row_is_even = i % 2 == 0;
                        let last_row = i == nb_row - 1;
                        let a = j * nb_row + i;
                        let b = j * nb_row + i + 1;
                        let c = (j + 1) * nb_row + i;
                        if row_is_even {
                            if !last_row {
                                inds.push(a);
                                inds.push(b);
                                inds.push(c);
                                inds.push(a);
                                inds.push(c);
                                inds.push(b);
                                //Log::diag(format!("inds: a{} b{} c{}", a, b, c));
                            }
                        } else {
                            let c_previous = (j + 1) * nb_row + i - 1;
                            let c_following = (j + 1) * nb_row + i + 1;
                            inds.push(a);
                            inds.push(c);
                            inds.push(c_previous);
                            inds.push(a);
                            inds.push(c_previous);
                            inds.push(c);
                            if !last_row {
                                inds.push(a);
                                inds.push(c_following);
                                inds.push(c);
                                inds.push(a);
                                inds.push(c);
                                inds.push(c_following);

                                inds.push(a);
                                inds.push(b);
                                inds.push(c_following);
                                inds.push(a);
                                inds.push(c_following);
                                inds.push(b);
                                //Log::diag(format!("inds: a{} b{} c{} c-{} c+{} ", a, b, c, c_previous, c_following));
                            } else {
                                //Log::diag(format!("inds: a{} c{} c-{} ", a, c, c_previous));
                            }
                        }
                    }
                }
            }

            let mut mesh = Mesh::new();
            mesh.set_data(verts.as_slice(), inds.as_slice(), None, None);

            mesh
        };
    }

    /// Set `screen_pose.position` and recompute the pose-dependent fields of `layer_cache`.
    /// Must be called only when `screen_pose.position` actually changes.
    pub fn update_pose_cache(&mut self, position: Vec3) {
        self.screen_pose.position = position;
        let local_offset = self.screen_pose.orientation * Vec3::new(0.0, 0.0, self.layer_cache.bounds_center_z);
        let layer_orientation = Quat::look_at(Vec3::ZERO, local_offset, Some(self.screen_pose.get_up()));
        self.layer_cache.local_offset = local_offset;
        self.layer_cache.layer_orientation = layer_orientation;
    }

    /// Check if the screen has been touched and return the position (x,y) in screen coordinates
    /// Returns Some((x, y)) if touched, None otherwise
    /// Coordinates are normalized between 0.0 and 1.0
    pub fn touched(&self, _token: &MainThreadToken, index: i32) -> Option<(f32, f32)> {
        // no ray when adjusting params
        if self.repo.show_param {
            return None;
        }

        // Transform from world into the screen's local/model space
        // Our screen mesh is drawn with transform = self.screen_pose.to_matrix(None)
        // So to bring a world ray into model space, multiply by the inverse
        let screen_mtx = self.screen_pose.to_matrix(None);
        let inv = screen_mtx.get_inverse();

        let p = Input::pointer(index, None);

        // Bring the pointer ray into model space
        let local_ray = inv.transform_ray(p.ray);

        // Use a precise raycast that also gives us the first triangle index
        let (mut hit_ray, mut tri_start_index) = (Ray::default(), 0u32);
        let hit = self.screen.intersect_to_ptr(local_ray, None, &mut hit_ray, &mut tri_start_index);
        if !hit {
            return None;
        }

        // we draw the ray
        //self.draw_ray( p.ray);
        Lines::add_ray(p.ray, self.screen_distance, named_colors::WHITE, None, self.ray_thickness);

        if !p.state.is_just_inactive() {
            return None;
        }
        // Retrieve the triangle's vertices to barycentrically interpolate UV
        let tri = self.screen.get_triangle(tri_start_index)?;
        let [a, b, c] = tri;

        // Compute barycentric coordinates of hit point on triangle ABC
        let p_hit = hit_ray.position; // hit point in model space
        let v0 = b.pos - a.pos;
        let v1 = c.pos - a.pos;
        let v2 = p_hit - a.pos;
        let d00 = Vec3::dot(v0, v0);
        let d01 = Vec3::dot(v0, v1);
        let d11 = Vec3::dot(v1, v1);
        let d20 = Vec3::dot(v2, v0);
        let d21 = Vec3::dot(v2, v1);
        let denom = d00 * d11 - d01 * d01;
        if denom == 0.0 {
            return None;
        }
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;

        // Interpolate UVs and return normalized coordinates
        let hit_uv = a.uv * u + b.uv * v + c.uv * w;

        // UVs are already normalized [0,1] on our mesh
        Some((hit_uv.x, hit_uv.y))
    }

    /// Set the pixel resolution of the screen content, updating the physical size accordingly.
    pub fn resolution(&mut self, width: u32, height: u32) -> &mut Self {
        self.width = width;
        self.height = height;
        self.screen_size = Vec2::new(width as f32 / 1000.0, height as f32 / 1000.0);
        self.screen_diagonal = (self.screen_size.x.powf(2.0) + self.screen_size.y.powf(2.0)).sqrt();
        self.adapt_screen();
        self
    }

    /// Set the OpenXR swapchain handle to use for quad-layer submission.
    /// When set, [`Self::draw`] will submit a composition quad layer instead of rendering the mesh.
    /// The caller retains ownership of the swapchain lifecycle (e.g. via [`crate::tools::xr_comp_layers::SwapchainSk`]).
    pub fn set_swapchain(&mut self, swapchain: Swapchain) -> &mut Self {
        self.openxr_swapchain = Some(swapchain);
        self
    }

    /// Clear the swapchain handle. The caller is responsible for destroying the underlying swapchain.
    pub fn clear_swapchain(&mut self) -> &mut Self {
        self.openxr_swapchain = None;
        self
    }

    /// Stop the spatial audio streams and clear the swapchain handle.
    /// Call this when the owner stepper is shutting down.
    pub fn shutdown(&mut self) {
        if let Some(inst) = self.sound_left_inst.take() {
            inst.stop();
        }
        if let Some(inst) = self.sound_right_inst.take() {
            inst.stop();
        }
        self.openxr_swapchain = None;
    }

    /// Get the IDs of the left and right sounds as a tuple `(left_id, right_id)`.
    pub fn get_sound_ids(&self) -> (&str, &str) {
        (self.sound_left.get_id(), self.sound_right.get_id())
    }

    /// Get the screen mesh
    pub fn get_mesh(&self) -> &Mesh {
        &self.screen
    }

    /// Get the current screen distance
    pub fn get_screen_distance(&self) -> f32 {
        self.screen_distance
    }

    /// Get the current curvature (0.0 = flat, 1.0 = tight cylinder)
    pub fn get_curvature(&self) -> f32 {
        self.curvature
    }

    /// Get the current screen size
    pub fn get_screen_size(&self) -> Vec2 {
        self.screen_size
    }

    /// Get the current screen diagonal
    pub fn get_screen_diagonal(&self) -> f32 {
        self.screen_diagonal
    }

    /// Get the current screen orientation
    pub fn get_screen_orientation(&self) -> Quat {
        self.screen_pose.orientation
    }

    /// Get the current sound spacing factor
    pub fn get_sound_spacing_factor(&self) -> f32 {
        self.sound_spacing_factor
    }

    /// Get the current ray thickness
    pub fn get_ray_thickness(&self) -> f32 {
        self.ray_thickness
    }

    /// Shared factor used to scale UI elements relative to screen distance and diagonal.
    fn factor_size(&self) -> f32 {
        (self.screen_distance.max(1.0).powf(2.0) + self.screen_diagonal.max(1.0).powf(2.0)).sqrt()
    }

    /// Returns a world-space [`Pose`] at the top-centre edge of the screen. The window origin is placed just above the
    /// top edge, outside the screen content area, at the same height as the compact hamburger button.
    /// `offset` is added in screen-local space before applying the screen transform, allowing the caller to shift the
    /// pose horizontally to avoid overlapping other windows.
    /// Useful for anchoring a UI window (e.g. transport controls) with [`UiMove::None`].
    pub fn get_top(&self, offset: impl Into<Vec3>) -> Pose {
        let bounds = self.screen.get_bounds();
        let factor_size = self.factor_size();
        let screen_transform = self.screen_pose.to_matrix(None);
        let pos = Vec3::new(0.0, self.screen_size.y / 2.0 + 0.04 * factor_size, bounds.center.z) + offset.into();
        Pose::new(pos, None) * screen_transform
    }

    /// Returns a world-space [`Pose`] at the bottom-centre edge of the screen. The window origin is placed just below
    /// the bottom edge, outside the screen content area. `offset` is added in screen-local space before applying the
    /// screen transform.
    /// Useful for anchoring a UI window (e.g. status bar) with [`UiMove::None`].
    pub fn get_bottom(&self, offset: impl Into<Vec3>) -> Pose {
        let bounds = self.screen.get_bounds();
        let factor_size = self.factor_size();
        let screen_transform = self.screen_pose.to_matrix(None);
        let pos = Vec3::new(0.0, -(self.screen_size.y / 2.0 + 0.04 * factor_size), bounds.center.z) + offset.into();
        Pose::new(pos, None) * screen_transform
    }
}
