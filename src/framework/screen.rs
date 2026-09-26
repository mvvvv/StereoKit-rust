use std::f32::consts::PI;

use crate::framework::{Appearence, Resizing};
use crate::sound::{SoundBus, SoundPlay};
use crate::util::{Color128, Time, named_colors};
use crate::{
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Ray, Vec2, Vec3},
    mesh::{Inds, Mesh, Vertex},
    render::Renderer,
    sk::MainThreadToken,
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{Align, BtnState, Input, Lines, Log, TextFit},
    tex::Tex,
    ui::{Ui, UiBtnLayout, UiConfirm, UiMove, UiWin},
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
            radius: 1.0,
            central_angle: 1.0,
            aspect_ratio: 1.0,
            local_offset: Vec3::ZERO,
            layer_orientation: Quat::IDENTITY,
        }
    }
}

/// UI ids and sprites of the toolbars owned by a [`Screen`], derived from its user id.
struct ScreenRepo {
    id_handle: String,
    id_focus_probe: String,
    id_toolbar: String,
    id_btn_show_param: String,
    id_btn_close_param: String,
    id_window_param: String,
    sprite_close: Sprite,
    sprite_menu: Sprite,
    id_material: String,
}

impl ScreenRepo {
    fn new(id: String) -> Self {
        Self {
            id_handle: id.clone() + "_handle",
            id_focus_probe: id.clone() + "_focus_probe",
            id_toolbar: id.clone() + "_toolbar",
            id_btn_show_param: id.clone() + "_btn_show",
            id_btn_close_param: id.clone() + "_btn_close",
            id_window_param: id.clone() + "_window_param",
            sprite_close: Sprite::close(),
            sprite_menu: Sprite::list(),
            id_material: id + "_material",
        }
    }
}

/// A virtual curved screen that can display a [`Tex`] or an OpenXR swapchain cylinder layer.
///
/// This is the refactored `Screen`: focus-driven toolbars, the [`Appearence`] scale knob and the parameter window
/// described below.
/// * [`Appearence`] is applied at the screen level: the screen is drawn at `Appearence::window_size * ui_scale`,
///   exactly like an `Appearence` window, and the [`Appearence::scale_handle`] knob anchored to the screen is THE
///   interactive diagonal / zoom control.
///
/// Like `Screen`, it is a concave mesh whose curvature, diagonal, and distance from the viewer are adjustable at
/// runtime. It also ships with:
/// * two spatial stereo audio streams (left / right)
/// * an optional single-line overlay text rendered above the content
/// * an optional extra-param UI callback injected into the parameter window
///
/// Two texture slots (`0` and `1`) allow cross-fading between images without dropping GPU handles.
/// Use [`Screen::set_texture`] to upload a new frame into the inactive slot, then
/// [`Screen::set_tex_curr`] to flip to it.
///
/// For OpenXR deployments, plug in a [`crate::tools::xr_comp_layers::SwapchainSk`] handle via
/// [`Screen::set_swapchain`] to submit a composition cylinder layer instead of rendering the mesh —
/// this bypasses the StereoKit render pipeline and gives compositor-level reprojection.
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
/// // Build the screen — default distance is 2.2 m, default resolution 3840x2160
/// let mut screen = Screen::new("doc_screen", &tex);
///
/// // Bring the screen close, give it a tight diagonal, and an overlay.
/// // `always_visible` forces the toolbars on: headless tests have no pointer to focus them.
/// screen.resolution(320, 240)
///       .screen_distance(2.3)   // 2.3 m away from the viewer
///       .screen_diagonal(1.2)   // 1.2 m diagonal (compact)
///       .set_overlay_text("Hello, Screen!")
///       .always_visible(true);
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
    /// When `true` (default), the screen is a cylinder: the curvature slider snaps to `0.0` (nearly
    /// flat cylinder) or `1.0` (tight cylinder). When `false`, the screen is a sphere and any value in
    /// `[0.0, 1.0]` is accepted for intermediate curvatures.
    cylindrical: bool,
    /// Shape of the screen: `0.0` = nearly flat, `1.0` = tightest curve (cylinder or sphere).
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

    /// Cached availability of the `XR_KHR_composition_layer_cylinder` extension.
    cylinder_layer_available: bool,

    /// Text displayed above the screen while a pointer focuses it. Set with [`Screen::set_overlay_text`].
    overlay_text: String,

    /// Optional callback invoked at the end of the settings panel (while it is visible).
    /// Set with [`Screen::set_extra_param_ui`].
    extra_param_ui: Option<Box<dyn FnMut() + Send + 'static>>,

    /// Look & feel applied at the screen level: its `window_size` mirrors the screen size, and its scale handle is the
    /// interactive diagonal / zoom control ([`Resizing::ZoomOnly`]).
    appearence: Appearence,
    /// Has [`Appearence::start`] already run? Deferred to the first focused frame, so pre-draw property tweaks
    /// (`window_size`, text styles...) are always taken into account.
    appearence_started: bool,

    /// Grab session of the distance-driving grab bar: `(distance at grab, view axis at grab, projection of the pose on
    /// that axis at grab)`. While the bar is held, the drag along the frozen view axis moves the screen closer /
    /// farther; reset when the bar is released.
    bar_grab: Option<(f32, Vec3, f32)>,

    /// Is the parameter window currently open? Opened by the hamburger button.
    show_param: bool,

    /// Seconds left before the toolbars hide themselves.
    focus_timer: f32,
    /// Are the toolbars currently visible? .
    screen_focused: bool,
    /// Extra meters added around the focus volume.
    focus_margin: f32,
    /// Seconds the toolbars stay visible after the last pointer interaction. Default is 0.4 s.
    focus_grace: f32,
    /// When `true`, the toolbars ignore the pointer focus and stay always visible.
    always_visible: bool,
}

// SAFETY: every field is either plain data or a StereoKit handle (`Mesh`, `Material`, `Tex`,
// `Sprite`, `Sound`, `SoundInst`) which may only be used from the main thread. `Screen` is always
// driven by an `IStepper` on the main thread (see the `MainThreadToken` of `Screen::draw` and
// `Screen::touched`), the `Send` bound only exists so a `Screen` can be stored inside a `Send`
// stepper.
unsafe impl Send for Screen {}

/// All the code here run in the main thread
impl Screen {
    /// Largest distance the grab bar can pull the screen to (in meters).
    pub const MAX_DISTANCE: f32 = 6.0;
    /// Largest screen diagonal accepted by [`Screen::screen_diagonal`] (in meters).
    pub const MAX_DIAGONAL: f32 = 15.0;
    /// Smallest screen diagonal accepted by [`Screen::screen_size`] and [`Screen::screen_diagonal`] (in meters).
    pub const MIN_DIAGONAL: f32 = 0.2;
    /// Distance the screen toolbars are calibrated for (the default [`Screen::screen_distance`].
    pub const UI_REFERENCE_DISTANCE: f32 = 2.2;
    /// Radius a `curvature = 0.0` screen is capped to, on top of [`Screen::screen_distance`]: far
    /// enough that the screen looks flat, close enough to keep the vertex math accurate (the sagitta
    /// of a one metre wide screen is below a millimetre). Legacy value of the spherical `Screen`.
    const FLAT_RADIUS: f32 = 500.0;

    /// Distance below which the grab-bar drag is not applied.
    const DRAG_TOLERANCE: f32 = 0.001;
    /// Window-local Z offset of the parameter window: floating in front of the screen plane.
    const PARAM_WINDOW_Z: f32 = 0.6;
    /// Fixed width of the parameter window (its height auto-fits the content).
    const PARAM_WINDOW_WIDTH: f32 = 0.4;

    /// Create the screen. Use a distinct `id` if several screens live in the same app.
    pub fn new(id: &str, screen_tex: impl AsRef<Tex>) -> Self {
        let width = 3840u32;
        let height = 2160u32;
        let screen_size = Vec2::new(width as f32 / 1000.0, height as f32 / 1000.0);
        let screen_diagonal = (screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt();
        let screen_material = Material::unlit().copy();

        let mut appearence = Appearence::default();
        appearence.window_size = screen_size;
        appearence.scale_bounds = (0.5, 4.0);
        appearence.min_window_size = Vec2::new(0.01, 0.01);
        appearence.resizing = Resizing::ZoomOnly;
        appearence.handle_sprite = Sprite::from_file("icons/zoom2.png", None, None).ok();
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
            cylinder_layer_available: XrCompLayers::cylinder_layer_available(),

            overlay_text: String::new(),
            extra_param_ui: None,
            cylindrical: true,
            show_param: false,

            appearence,
            appearence_started: false,
            bar_grab: None,
            focus_timer: 0.0,
            screen_focused: false,
            focus_margin: 0.1,
            focus_grace: 0.4,
            always_visible: false,
        };

        let screen_tex = screen_tex.as_ref().clone_ref();

        this.screen_textures[0] = Some(screen_tex.clone_ref());
        this.screen_material.id(&this.repo.id_material);
        this.update_material_texture();

        this.sound_left = Self::create_sound_stream("left");
        this.sound_right = Self::create_sound_stream("right");

        this.screen_pose = Input::get_head() * Matrix::Y_180;
        this.update_pose_cache(Input::get_head().position);
        this.adapt_screen();
        // Anchor the scale knob right next to the grab bar.
        this.update_knob_anchor(this.factor_size(), this.screen.get_bounds().center.z);

        let sound_settings = SoundPlay { bus: SoundBus::Music, volume: 1.0, ..Default::default() };
        this.sound_left_inst = Some(this.sound_left.play_with(this.sound_position(-1), &sound_settings));
        this.sound_right_inst = Some(this.sound_right.play_with(this.sound_position(1), &sound_settings));

        this
    }

    /// Set the distance from the viewer to the centre of the screen, clamped to [`Screen::MAX_DISTANCE`] and to the
    /// screen-diameter floor `Self::min_distance_for`: a screen cannot get closer than the view circle it fits in
    /// (`size / PI`). The curvature is preserved, only the distance changes (see `adapt_screen`).
    pub fn screen_distance(&mut self, distance: f32) -> &mut Self {
        let clamped = distance.clamp(Self::min_distance_for(self.screen_size), Self::MAX_DISTANCE);
        if clamped != self.screen_distance {
            self.screen_distance = clamped;
            self.adapt_screen();
        }
        self
    }

    /// The largest size coordinate that fits the view circle of `distance`: the screen can never be wider than the
    /// circle it wraps in, `distance * PI`.
    fn max_size_for(distance: f32) -> f32 {
        distance * PI
    }

    /// The smallest legal distance for a screen of that size: the inverse of [`Self::max_size_for`], driven by the
    /// wider of the two axes.
    fn min_distance_for(size: Vec2) -> f32 {
        size.x.max(size.y) / PI
    }

    /// Clamp `diagonal` to [`Self::MIN_DIAGONAL`] / [`Self::MAX_DIAGONAL`], then return the diagonal really reachable
    /// for a screen of `size` / `current_diagonal` at `distance`: the requested size is shrunk uniformly (the aspect
    /// ratio is preserved) when it would not fit the view circle.
    fn clamp_diagonal(size: Vec2, current_diagonal: f32, distance: f32, diagonal: f32) -> f32 {
        let clamped = diagonal.clamp(Self::MIN_DIAGONAL, Self::MAX_DIAGONAL);
        let scaled = size * (clamped / current_diagonal.max(f32::EPSILON));
        let shrink = (scaled.x.max(scaled.y) / Self::max_size_for(distance).max(f32::EPSILON)).max(1.0);
        clamped / shrink
    }

    /// Create one of the two spatial stereo audio streams, falling back on a silent default sound (with a warning)
    /// when the stream cannot be created.
    fn create_sound_stream(side: &str) -> Sound {
        match Sound::create_stream(2.0) {
            Ok(sound) => sound,
            Err(e) => {
                Log::warn(format!("Screen: could not create the {side} audio stream: {e}"));
                Sound::default()
            }
        }
    }

    /// Set the curvature of the screen: `0.0` = nearly flat, `1.0` = tightest curve — a cylinder matching the XR
    /// cylinder composition layer when [`Screen::cylindrical`], a sphere centred on the viewer otherwise. Intermediate
    /// values are only accepted when `cylindrical` is `false`. The centre of the screen stays at
    /// [`Screen::screen_distance`]: bending the screen never moves it away from the viewer, it only wraps it more or
    /// less around them.
    pub fn curvature(&mut self, curvature: f32) -> &mut Self {
        self.curvature =
            if self.cylindrical { if curvature >= 0.5 { 1.0 } else { 0.0 } } else { curvature.clamp(0.0, 1.0) };
        self.adapt_screen();
        self
    }

    /// When `true` (default), the screen is a cylinder and the curvature slider only allows `0.0` (nearly flat) or
    /// `1.0` (tight cylinder, the XR cylinder layer geometry). When `false`, the screen is a sphere (`radius =
    /// distance + 1 / curvature - 1`) and the slider accepts any value in `[0.0, 1.0]`. Reapplies the current
    /// curvature value under the new mode.
    pub fn cylindrical(&mut self, cylindrical: bool) -> &mut Self {
        self.cylindrical = cylindrical;
        // re-snap or re-clamp the current curvature value
        let cur = self.curvature;
        self.curvature(cur)
    }

    /// Set the screen orientation and recompute the pose-dependent layer cache fields.
    pub fn screen_orientation(&mut self, orientation: impl Into<Quat>) -> &mut Self {
        self.screen_pose.orientation = orientation.into();
        self.refresh_layer_pose();
        self
    }

    /// Set the screen size (in meters).
    pub fn screen_size(&mut self, size: impl Into<Vec2>) -> &mut Self {
        let size = size.into();
        let max_size = Self::max_size_for(self.screen_distance);
        let screen_diagonal = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        if size.x > 0.0
            && size.y > 0.0
            && size.x <= max_size
            && size.y <= max_size
            && screen_diagonal > Self::MIN_DIAGONAL
        {
            self.screen_size = size;
            self.screen_diagonal = screen_diagonal;
            self.adapt_screen();
            self.sync_appearence_to_screen();
        }
        self
    }

    /// Set the screen diagonal (the size is adjusted proportionally, so the aspect ratio never changes).
    pub fn screen_diagonal(&mut self, diagonal: f32) -> &mut Self {
        let effective = Self::clamp_diagonal(self.screen_size, self.screen_diagonal, self.screen_distance, diagonal);
        if (effective - self.screen_diagonal).abs() > f32::EPSILON {
            let size = self.screen_size * (effective / self.screen_diagonal.max(f32::EPSILON));
            self.screen_size = size;
            self.screen_diagonal = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
            self.adapt_screen();
            self.sync_appearence_to_screen();
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

    /// Seconds the toolbars around the screen stay visible after the last pointer interaction. Default is 0.4 s.
    pub fn focus_grace(&mut self, seconds: f32) -> &mut Self {
        self.focus_grace = seconds.max(0.05);
        self.focus_timer = self.focus_timer.min(self.focus_grace);
        self
    }

    /// Extra meters added around the focus volume.
    pub fn focus_margin(&mut self, meters: f32) -> &mut Self {
        self.focus_margin = meters.max(0.0);
        self
    }

    /// When `true`, the toolbars ignore the pointer focus and stay always visible. Default is `false`.
    pub fn always_visible(&mut self, force: bool) -> &mut Self {
        self.always_visible = force;
        self
    }

    /// Are the toolbars around the screen currently visible.
    pub fn is_focused(&self) -> bool {
        self.screen_focused
    }

    /// Is the parameter window currently open?.
    pub fn is_param_open(&self) -> bool {
        self.show_param
    }

    /// The [`Appearence`] applied at the screen level.
    pub fn appearence(&self) -> &Appearence {
        &self.appearence
    }

    /// Mutable access to the [`Appearence`] applied at the screen level.
    pub fn appearence_mut(&mut self) -> &mut Appearence {
        &mut self.appearence
    }

    /// How the [`Appearence::scale_handle`] knob resizes and zooms the screen. Default is [`Resizing::ZoomOnly`]: as
    /// the video / swapchain has a locked aspect ratio, the knob only manages the screen diagonal (the ui zoom). Switch
    /// to [`Resizing::KeepRatio`] (or [`Resizing::Free`]) to let the knob resize the screen too.
    pub fn resizing(&mut self, resizing: Resizing) -> &mut Self {
        self.appearence.resizing = resizing;
        self
    }

    /// The current ui scale of the screen [`Appearence`].
    pub fn get_ui_scale(&self) -> f32 {
        self.appearence.get_ui_scale()
    }

    /// Set text to display above the screen while a pointer focuses it.
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

    /// Register a closure that will be drawn inside the parameter window (opened from the hamburger button, visible on
    /// focus). Use it to append app controls (sliders, toggles, labels...) without subclassing `Screen`.
    ///
    /// The closure is called on every frame the window is drawn, via `Option::take` to avoid borrow conflicts with the
    /// rest of `Screen`.
    ///
    /// # Example
    /// ```ignore
    /// screen.set_extra_param_ui(move || {
    ///     Ui::label("Slide speed").use_padding(true).draw();
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

    /// Update the material's diffuse texture based on the current texture index. When the current slot has been
    /// emptied ([`Screen::set_texture`] with `None`), the other slot is used when it is filled, so the screen never
    /// keeps displaying a texture it does not own anymore.
    fn update_material_texture(&mut self) {
        let texture = self.screen_textures[self.tex_curr]
            .as_ref()
            .or_else(|| self.screen_textures[1 - self.tex_curr].as_ref());
        if let Some(texture) = texture {
            self.screen_material.diffuse_tex(texture);
        }
    }

    /// Mirror the screen size into `Appearence::window_size` (divided by the current ui scale, as the screen is drawn
    /// at `window_size * ui_scale`). Keeps the scale knob consistent when the size is changed programmatically through
    /// [`Screen::screen_size`], [`Screen::screen_diagonal`] or [`Screen::resolution`].
    fn sync_appearence_to_screen(&mut self) {
        let scale = self.appearence.get_ui_scale();
        self.appearence.window_size = self.screen_size / scale;
    }

    /// Apply the size driven by the [`Appearence::scale_handle`] knob: the screen is drawn at
    /// `Appearence::window_size * ui_scale`, exactly like an `Appearence` window. Sizes rejected by the screen
    /// constraints ([`Screen::screen_size`]) are written back into `window_size`, so the knob never lies about the
    /// real screen size.
    fn sync_screen_to_appearence(&mut self) {
        let target = self.appearence.scaled_window_size();
        if target != self.screen_size {
            // `screen_size` mirrors the (possibly clamped) size back into `window_size` through
            // `sync_appearence_to_screen`, so the knob never lies about the real screen size.
            self.screen_size(target);
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene.

    pub fn draw(&mut self, _token: &MainThreadToken) {
        // Focus bookkeeping
        self.focus_timer = (self.focus_timer - Time::get_stepf()).max(0.0);
        self.screen_focused = self.always_visible || self.show_param || self.focus_timer > 0.0;

        // The grab handle first.
        self.draw_grab_handle();
        let bounds = self.screen.get_bounds();
        let factor_size = self.factor_size();
        let d = self.reference_distance_factor();
        let screen_transform = self.screen_pose.to_matrix(None);

        // The toolbars around the screen
        if self.screen_focused {
            // First focused frame: clamp the properties set before launch.
            if !self.appearence_started {
                self.appearence.start();
                self.appearence_started = true;
            }
            Ui::push_id(&self.repo.id_toolbar);
            // The scale knob resizes / zooms the screen directly
            self.draw_scale_knob(factor_size, bounds.center.z);
            // The hamburger button opens the parameter window.
            self.draw_hamburger(screen_transform, bounds.center.z, factor_size, d);
            Ui::pop_id();
        }
        // The open parameter window keeps the toolbars visible but it never closes itself: only its close button does.
        self.draw_param_window(screen_transform, bounds.center);
        self.draw_overlay(screen_transform, bounds.center.z, factor_size, d);
        // The focus probe is registered after every toolbar element.
        self.draw_focus_probe();

        if self.draw_swapchain() {
            // The swapchain quad/cylinder layer is submitted at sort order -1 (behind the main scene).
            // Draw the screen mesh with BLACK_TRANSPARENT so those pixels are punched out and the
            // compositor layer shows through from behind.
            Renderer::add_mesh(
                &self.screen,
                &self.screen_material,
                screen_transform,
                Some(Color128::BLACK_TRANSPARENT),
                None,
            );
        } else {
            // No swapchain set — render the mesh normally with the current texture.
            Renderer::add_mesh(&self.screen, &self.screen_material, screen_transform, None, None);
        }
    }

    /// Keep the toolbars visible for [`Screen::focus_grace`] seconds from now.
    fn refresh_focus(&mut self) {
        self.focus_timer = self.focus_grace;
        self.screen_focused = true;
    }

    /// [`Screen::refresh_focus`] when the UI element drawn just before is focused by a pointer.
    fn refresh_focus_from_last_element(&mut self) {
        if Ui::get_last_element_focused().is_active() {
            self.refresh_focus();
        }
    }

    /// The focus probe: an invisible UI volume covering the toolbar strip.
    fn draw_focus_probe(&mut self) {
        let mut focused = BtnState::Inactive;
        Ui::push_surface(self.screen_pose, Vec3::ZERO, Vec2::ZERO);
        Ui::volume_at(&self.repo.id_focus_probe, self.focus_bounds(), UiConfirm::Push, None, Some(&mut focused));
        Ui::pop_surface();
        if focused.is_active() {
            self.refresh_focus();
        }
    }

    /// The screen grab bar.
    fn draw_grab_handle(&mut self) {
        // Read before the drag is applied: the bar must not jump under the user's hand.
        let factor_size = self.factor_size();
        let bounds_center_z = self.screen.get_bounds().center.z;
        let grab_position = Vec3::new(
            0.0, //
            self.screen_size.y / 2.0 + 0.05 * factor_size,
            bounds_center_z,
        );
        let grab_dimension = Vec3::new(
            factor_size * 0.2, //
            factor_size * 0.01,
            factor_size * 0.0025,
        );
        let grabbed =
            Ui::handle(&self.repo.id_handle, &mut self.screen_pose, Bounds::new(grab_position, grab_dimension))
                .draw_handle(self.screen_focused)
                .grab();
        self.refresh_focus_from_last_element();
        if grabbed {
            let head = Input::get_head();
            // The bar motion along the view axis drives the distance. .
            let (dist_start, axis, proj_start) = match self.bar_grab {
                Some(session) => session,
                None => {
                    let axis = head.get_forward();
                    let session = (self.screen_distance, axis, Vec3::dot(self.screen_pose.position, axis));
                    self.bar_grab = Some(session);
                    session
                }
            };
            let proj = Vec3::dot(self.screen_pose.position, axis);
            let desired =
                (dist_start + proj - proj_start).clamp(Self::min_distance_for(self.screen_size), Self::MAX_DISTANCE);
            // Skip the mesh rebuild while the hand is still.
            if (desired - self.screen_distance).abs() > Self::DRAG_TOLERANCE {
                self.screen_distance(desired);
            }
            // Keep the screen anchored in front of the head while its distance is being driven.
            self.update_pose_cache(head.position);
            // Grabbing is a pointer interaction: keep the toolbars visible.
            self.refresh_focus();
        } else {
            self.bar_grab = None;
        }
    }

    /// The [`Appearence`] scale handle knob.
    fn draw_scale_knob(&mut self, factor_size: f32, bounds_center_z: f32) {
        self.update_knob_anchor(factor_size, bounds_center_z);
        let scale_grabbed = self.appearence.scale_handle(&self.screen_pose, "h").is_some();
        self.sync_screen_to_appearence();
        // A pointer on the knob — the only resize / zoom control — is focus: keep the toolbars alive.
        self.refresh_focus_from_last_element();
        if scale_grabbed {
            // Grabbing is a pointer interaction too, even when the focus report above lags a frame.
            self.refresh_focus();
        }
    }

    /// Recompute the [`Appearence`] scale-handle anchor (`scale_handle_default_offset`).
    fn update_knob_anchor(&mut self, factor_size: f32, bounds_center_z: f32) {
        let drawn = self.appearence.scaled_window_size();
        // Right of the bar: the bar half-width is `0.1 * factor_size`, plus a small gap. Same height as the bar.
        let knob_x = factor_size * 0.1 + 0.08;
        let knob_y = self.screen_size.y / 2.0 + 0.05 * factor_size;
        // `forward` points from the screen towards the user, hence the negated plane depth.
        self.appearence.scale_handle_default_offset =
            Vec3::new(knob_x * 0.8 / drawn.x.max(0.001), knob_y * 0.85 / drawn.y.max(0.001), -bounds_center_z + 0.02);
    }

    /// The hamburger button, on its own surface anchored above the screen: it opens the parameter window.
    fn draw_hamburger(&mut self, screen_transform: Matrix, bounds_center_z: f32, factor_size: f32, d: f32) {
        if self.show_param {
            return;
        }

        let info_position = Vec3::new(
            0.0, //
            self.screen_size.y / 2.0 + 0.04 * factor_size,
            bounds_center_z,
        );
        let button_pose = Pose::new(info_position, None) * screen_transform;
        let btn_size = Vec2::new(0.06 * d, 0.06 * d);
        let surface_size = btn_size * 1.1;
        Ui::push_surface(button_pose, Vec3::X * 0.02 * d, surface_size);
        let pressed = Ui::button(&self.repo.id_btn_show_param)
            .image(&self.repo.sprite_menu)
            .image_layout(UiBtnLayout::CenterNoText)
            .size(btn_size)
            .press();
        // A pointer hovering the button is focus too: keep the toolbars alive.
        self.refresh_focus_from_last_element();
        if pressed {
            self.show_param = true;
            let head = Input::get_head();
            self.update_pose_cache(head.position);
        }
        Ui::pop_surface();
    }

    /// The parameter window, floating in front of the screen (fixed width, auto height).
    fn draw_param_window(&mut self, screen_transform: Matrix, bounds_center: Vec3) {
        if !self.show_param {
            return;
        }

        let info_position = Vec3::new(bounds_center.x, bounds_center.y, Self::PARAM_WINDOW_Z);
        let mut window_pose = Pose::new(info_position, None) * screen_transform;

        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());

        // The width is the scaled `PARAM_WINDOW_WIDTH`, so the window follows the ui zoom.
        let window_width = self.appearence.scale_size(Vec2::new(Self::PARAM_WINDOW_WIDTH, 0.0)).x;
        Ui::window(&self.repo.id_window_param)
            .pose(&mut window_pose)
            .size(Vec2::new(window_width, 0.0))
            .window_type(UiWin::Body)
            .move_type(UiMove::None)
            .begin();

        // Header: a close button (button tint) plus the panel title in the title style.
        let line = Ui::get_line_height();
        let btn = self.appearence.scale_size(Vec2::new(line * 1.0, line * 1.0));
        Ui::push_tint(self.appearence.button_tint);
        let close = Ui::button(&self.repo.id_btn_close_param)
            .image(&self.repo.sprite_close)
            .image_layout(UiBtnLayout::CenterNoText)
            .size(btn)
            .press();
        Ui::pop_tint();
        Ui::same_line();
        Ui::push_text_style(self.appearence.title_style);
        Ui::label("Screen").draw();
        Ui::pop_text_style();

        Ui::push_text_style(self.appearence.label_style);

        // The curvature row: label + value + slider. With `cylindrical` (default) the slider snaps
        // to 0.0 (nearly flat cylinder) or 1.0 (tight cylinder); otherwise any value in [0, 1].
        Ui::push_tint(self.appearence.input_tint);
        Ui::label("Curvature").use_padding(true).draw();
        Ui::same_line();
        Ui::label(format!("{:.2}", self.curvature)).use_padding(true).draw();
        Ui::same_line();
        let mut curvature = self.curvature;
        let step = if self.cylindrical { 1.0 } else { 0.0 };
        if let Some(new_value) = Ui::hslider("curvature", &mut curvature, 0.0, 1.0).step(step).interact() {
            self.curvature(new_value);
        }
        Ui::pop_tint();

        // The continuous-curvature option: when on, the screen is a sphere.
        let mut free_curvature = !self.cylindrical;
        if Ui::toggle("Free curvature", &mut free_curvature).interact().is_some() {
            self.cylindrical(!free_curvature);
        }

        // Invoke the user-supplied param UI (e.g. quality sliders, mode toggles), separated from the
        // built-in row. Uses `take` + restore to avoid a simultaneous borrow of `self`.
        if let Some(mut f) = self.extra_param_ui.take() {
            Ui::hseparator();
            f();
            self.extra_param_ui = Some(f);
        }

        Ui::pop_text_style();

        Ui::window_end();
        // Restore the caller's UiSettings exactly as they were before this window was drawn.
        Ui::settings(prev_settings);

        if close {
            self.show_param = false;
        }
    }

    /// The overlay text, on its own surface anchored above the screen centre. Only drawn while the screen is focused.
    fn draw_overlay(&mut self, screen_transform: Matrix, bounds_center_z: f32, factor_size: f32, d: f32) {
        if self.overlay_text.is_empty() || !self.screen_focused {
            return;
        }

        let overlay_y = self.screen_size.y / 2.0 + 0.04 * factor_size;
        let overlay_pos = Vec3::new(-0.05, overlay_y, bounds_center_z);
        let overlay_pose = Pose::new(overlay_pos, None) * screen_transform;
        Ui::push_surface(overlay_pose, Vec3::X * -0.01 * d, Vec2::ZERO);
        Ui::text(&self.overlay_text)
            .size(Vec2::new(0.15, 0.04) * Self::UI_REFERENCE_DISTANCE * self.appearence.get_ui_scale())
            .text_align(Align::Center)
            .fit(TextFit::Exact)
            .draw();
        Ui::pop_surface();
    }

    /// Submit a quad/cylinder layer using the OpenXR swapchain if one is set.
    /// Returns `true` if the frame was submitted via swapchain (mesh rendering should be skipped).
    ///
    /// The XR compositor only knows the quad and the cylinder, so the screen is always submitted as a
    /// cylinder layer — a spherical or flat screen being a cylinder of large radius. The quad layer is
    /// kept as a fallback for runtimes without the `XR_KHR_composition_layer_cylinder` extension.
    fn draw_swapchain(&mut self) -> bool {
        if let Some(swapchain) = &self.openxr_swapchain {
            let cache = self.layer_cache;
            // Only per-frame computation: translate cached local_offset by current position.
            let at = self.screen_pose.position + cache.local_offset;

            if self.cylinder_layer_available {
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

    /// Radius of the arc the screen is bent on, in meters, for the current shape and curvature.
    ///
    /// - A `cylindrical` screen follows the XR cylinder layer: `radius = screen_distance / curvature`, so
    ///   `curvature = 1.0` is the tight cylinder whose axis sits on the viewer. At `curvature = 0.0` the radius would
    ///   be infinite: it is capped at `screen_distance + Self::FLAT_RADIUS`, a cylinder flat to the eye.
    /// - A spherical screen keeps the legacy [`Screen`] flattening: `radius = screen_distance + 1 / curvature - 1`, so
    ///   `curvature = 1.0` is the tight sphere centred on the viewer and `curvature = 0.0` a nearly flat sphere.
    fn screen_radius(&self) -> f32 {
        if self.curvature <= 0.0 {
            self.screen_distance + Self::FLAT_RADIUS
        } else if self.cylindrical {
            self.screen_distance / self.curvature
        } else {
            self.screen_distance + (1.0 / self.curvature - 1.0)
        }
    }

    /// Rebuild the mesh and refresh the [`SwapchainLayerCache`] after a change of size, distance or curvature.
    ///
    /// The mesh follows the current shape: [`Screen::adapt_screen_cylinder`] for a `cylindrical` screen (curvature
    /// `0.0` being a nearly flat cylinder of radius `screen_distance + FLAT_RADIUS`, `1.0` the tight cylinder matching
    /// the XR cylinder layer), [`Screen::adapt_screen_spherical`] for a spherical one (any curvature in `[0.0, 1.0]`).
    ///
    /// The [`SwapchainLayerCache`] is always configured for a cylinder layer: the XR compositor knows only the quad
    /// and the cylinder (see [`Screen::draw_swapchain`]).
    fn adapt_screen(&mut self) {
        let radius = self.screen_radius();
        if self.cylindrical {
            self.adapt_screen_cylinder(radius);
        } else {
            self.adapt_screen_spherical(radius);
        }
        let central_angle = self.screen_size.x / radius;

        // Empirical correction, tuned so that the submitted cylinder layer lines up with the mesh cylinder: the layer
        // wraps slightly tighter than the mesh. It only ever applies to the layer radius, never to the mesh geometry.
        let radius_factor = 1.0 - (central_angle.powi(2) * 0.06);
        let bounds = self.screen.get_bounds();
        self.layer_cache = SwapchainLayerCache {
            rect: Rect::new(0.0, 0.0, self.width as f32, self.height as f32),
            bounds_center_z: bounds.center.z,
            radius: radius * radius_factor,
            central_angle,
            aspect_ratio: self.screen_size.x / self.screen_size.y,
            // Pose fields, refreshed from the current orientation at the end of this function.
            local_offset: Vec3::ZERO,
            layer_orientation: Quat::IDENTITY,
        };
        self.refresh_layer_pose();
    }

    /// Build a cylindrical mesh with the given `radius` (see [`Screen::screen_radius`]). At `curvature = 1.0` the mesh
    /// exactly matches the XR cylinder composition layer geometry; at `curvature = 0.0` the radius is large enough
    /// that the cylinder is flat to the eye.
    ///
    /// The arc is translated along Z so that its centre always stays at `screen_distance`: a wider radius (a flatter
    /// screen) only changes the wrap, it never moves the screen away from the viewer. At `curvature = 1.0` (`radius ==
    /// screen_distance`) this offset is `0.0`.
    ///
    /// Each triangle is pushed in both windings: the material is single-sided, and the concave screen must stay
    /// visible when looked at from behind (through the mesh).
    fn adapt_screen_cylinder(&mut self, radius: f32) {
        let central_angle = self.screen_size.x / radius;
        let height = self.screen_size.y;
        // Mesh-local Z of the cylinder axis, so that `z(0.0) == screen_distance` at any curvature.
        let center_z = self.screen_distance - radius;

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
                let z = center_z + radius * angle.cos();
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

    /// Build the mesh of a spherical screen: a cap of the sphere of radius `radius` (see [`Screen::screen_radius`]),
    /// so that `curvature = 1.0` is a sphere centred on the viewer and `curvature = 0.0` a sphere large enough to look
    /// flat.
    ///
    /// The grid follows the legacy [`Screen`] layout: `subdiv_v` rows and `subdiv_u` columns (driven by the aspect
    /// ratio), every vertex with a [`Vec3::FORWARD`] normal, and each triangle pushed in both windings because the
    /// material is single-sided (like [`Screen::adapt_screen_cylinder`]).
    fn adapt_screen_spherical(&mut self, radius: f32) {
        // Distance between the screen centre and the sphere centre: `z = screen_distance` at the
        // screen centre whatever the radius.
        let flattening = radius - self.screen_distance;
        let width = self.screen_size.x;
        let height = self.screen_size.y;

        let aspect_ratio = width / height;

        let perimeter = 2.0 * PI * radius;

        let subdiv_v = 30u32;
        // At least two columns, so the UV division below cannot divide by zero on a very tall screen.
        let subdiv_u = ((subdiv_v as f32 * aspect_ratio) as u32).max(2);

        let angle_v = 2.0 * PI * height / perimeter;
        let angle_u = 2.0 * PI * width / perimeter;
        let delta_v = angle_v / subdiv_v as f32;
        let delta_u = angle_u / subdiv_u as f32;

        let mut verts: Vec<Vertex> = vec![];
        let mut inds: Vec<Inds> = vec![];

        for j in 0..subdiv_v {
            let v = -angle_v / 2.0 + (j as f32 * delta_v) + PI / 2.0;
            for i in 0..subdiv_u {
                let u = -angle_u / 2.0 + (i as f32 * delta_u) + PI / 2.0;
                let x = radius * v.sin() * u.cos();
                let y = radius * v.cos();
                let z = radius * v.sin() * u.sin() - flattening;

                verts.push(Vertex::new(
                    Vec3::new(x, y, z),
                    Vec3::FORWARD,
                    Some(Vec2::new(i as f32 / (subdiv_u - 1) as f32, j as f32 / (subdiv_v - 1) as f32)),
                    None,
                ));

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
                        }
                    }
                }
            }
        }

        let mut mesh = Mesh::new();
        mesh.set_data(verts.as_slice(), inds.as_slice(), None, None);
        self.screen = mesh;
    }

    /// Set `screen_pose.position` and recompute the pose-dependent fields of `layer_cache`.
    /// Must be called only when `screen_pose.position` actually changes.
    pub fn update_pose_cache(&mut self, position: Vec3) {
        self.screen_pose.position = position;
        self.refresh_layer_pose();
    }

    /// Recompute the pose-dependent fields of [`SwapchainLayerCache`] (`local_offset`, `layer_orientation`) from the
    /// current `screen_pose`. Shared by [`Screen::screen_orientation`], [`Screen::update_pose_cache`] and
    /// `adapt_screen`, so the layer pose can only be derived one way.
    fn refresh_layer_pose(&mut self) {
        let local_offset = self.screen_pose.orientation * Vec3::new(0.0, 0.0, self.layer_cache.bounds_center_z);
        self.layer_cache.layer_orientation = Quat::look_at(Vec3::ZERO, local_offset, Some(self.screen_pose.get_up()));
        self.layer_cache.local_offset = local_offset;
    }

    /// Return the position (x, y) in normalized screen coordinates (`0.0..=1.0`) when the pointer of that index is
    /// released on the screen, `None` otherwise.
    ///
    /// Note this is not a pure query: while the pointer hits the screen, an aim ray is added to [`Lines`] (a
    /// debug/inspection visual, see [`Screen::ray_thickness`]), whatever the return value. Nothing is drawn while the
    /// parameter window is open, or when the ray misses.
    pub fn touched(&self, _token: &MainThreadToken, index: i32) -> Option<(f32, f32)> {
        // no ray while the parameter window is open
        if self.show_param {
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

    /// Set the pixel resolution of the screen content (1 pixel = 1 mm), updating the physical size — and with it the
    /// `SwapchainLayerCache` rect — accordingly.
    ///
    /// A resolution whose physical size would be illegal for the current [`Screen::screen_distance`] is rejected:
    /// empty size (a `0` coordinate), diagonal below [`Screen::MIN_DIAGONAL`], or an axis wider than the view circle
    /// (`distance * PI`). [`Screen::screen_size`] applies exactly the same constraints. The rejection is silent by
    /// design: a user resizing the screen already sees the handle stop at the limit, and a rejected app call is a
    /// programming error, not a runtime event worth a per-frame log.
    pub fn resolution(&mut self, width: u32, height: u32) -> &mut Self {
        let size = Vec2::new(width as f32 / 1000.0, height as f32 / 1000.0);
        let diagonal = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        let max_size = Self::max_size_for(self.screen_distance);
        if width > 0 && height > 0 && diagonal > Self::MIN_DIAGONAL && size.x <= max_size && size.y <= max_size {
            self.width = width;
            self.height = height;
            self.screen_size = size;
            self.screen_diagonal = diagonal;
            self.adapt_screen();
            self.sync_appearence_to_screen();
        }
        self
    }

    /// Is an OpenXR swapchain currently plugged in (see [`Screen::set_swapchain`])?
    pub fn is_swapchain_set(&self) -> bool {
        self.openxr_swapchain.is_some()
    }

    /// Set the OpenXR swapchain handle to use for cylinder-layer submission.
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
        if let Some(mut inst) = self.sound_left_inst.take() {
            inst.stop();
        }
        if let Some(mut inst) = self.sound_right_inst.take() {
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

    /// Get the current curvature (0.0 = nearly flat, 1.0 = tightest curve)
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

    /// Get the current screen position (world-space)
    pub fn get_screen_position(&self) -> Vec3 {
        self.screen_pose.position
    }

    /// Get the full screen pose (position + orientation)
    pub fn get_screen_pose(&self) -> Pose {
        self.screen_pose
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
        Self::toolbar_scale_for(self.screen_diagonal, self.appearence.get_ui_scale())
    }

    /// [`Screen::factor_size`] as a pure function: screen diagonal + ui zoom, and nothing else.
    fn toolbar_scale_for(screen_diagonal: f32, ui_scale: f32) -> f32 {
        (Self::UI_REFERENCE_DISTANCE.powf(2.0) + screen_diagonal.max(1.0).powf(2.0)).sqrt() * ui_scale
    }

    /// `sqrt` of [`Screen::UI_REFERENCE_DISTANCE`], times the ui zoom.
    fn reference_distance_factor(&self) -> f32 {
        Self::UI_REFERENCE_DISTANCE.sqrt() * self.appearence.get_ui_scale()
    }

    /// Bounds of the focus volume, in the screen's model space.
    fn focus_bounds(&self) -> Bounds {
        Self::focus_bounds_for(self.screen.get_bounds(), self.factor_size(), self.focus_margin)
    }

    /// [`Screen::focus_bounds`] as a pure function (mesh bounds, toolbar scale, margin), so the
    /// geometry it guarantees can be unit tested.
    fn focus_bounds_for(mesh_bounds: Bounds, factor_size: f32, margin: f32) -> Bounds {
        let half = mesh_bounds.dimensions * 0.5;
        // Sitting on the top edge, half a screen high (plus the grazing margin on top).
        let bottom = mesh_bounds.center.y + half.y;
        let height = half.y + margin;
        // On a screen narrower than its toolbar strip, the scale knob hangs right of the mesh (see
        // `update_knob_anchor`): the volume is then as wide as the strip, not as the screen.
        let half_width = half.x.max(factor_size * 0.1 + 0.08) + margin;
        // From the toolbar plane backwards: a slab in front of the plane would beat the toolbar
        // elements themselves in the UI focus test, making the bar, the knob and the button unusable.
        let depth = half.z + margin;
        Bounds::new(
            Vec3::new(mesh_bounds.center.x, bottom + height * 0.5, mesh_bounds.center.z + depth * 0.5),
            Vec3::new(half_width * 2.0, height, depth),
        )
    }

    /// Returns a world-space [`Pose`] at the top-centre edge of the screen. The window origin is placed just above the
    /// top edge, outside the screen content area, at the same height as the hamburger button.
    /// `offset` is added in screen-local space before applying the screen transform, allowing the caller to shift the
    /// pose horizontally to avoid overlapping other windows.
    /// Useful for anchoring a UI window (e.g. transport controls) with [`UiMove::None`].
    ///
    /// Size the window and its `offset` from [`Screen::UI_REFERENCE_DISTANCE`] and [`Screen::get_ui_scale`] — not from
    /// the screen distance — so that a distance drag carries the window instead of resizing it.
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
    ///
    /// Like [`Screen::get_top`], size the window and its `offset` from [`Screen::UI_REFERENCE_DISTANCE`] and
    /// [`Screen::get_ui_scale`], never from the screen distance.
    pub fn get_bottom(&self, offset: impl Into<Vec3>) -> Pose {
        let bounds = self.screen.get_bounds();
        let factor_size = self.factor_size();
        let screen_transform = self.screen_pose.to_matrix(None);
        let pos = Vec3::new(0.0, -(self.screen_size.y / 2.0 + 0.04 * factor_size), bounds.center.z) + offset.into();
        Pose::new(pos, None) * screen_transform
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The floor is driven by the wider axis, and is the exact inverse of the view circle.
    #[test]
    fn min_distance_floor_uses_the_wider_axis() {
        assert!((Screen::min_distance_for(Vec2::new(2.0, 1.0)) - 2.0 / PI).abs() < 1e-6);
        assert!((Screen::min_distance_for(Vec2::new(1.0, 3.0)) - 3.0 / PI).abs() < 1e-6);
        assert!((Screen::max_size_for(2.0) - 2.0 * PI).abs() < 1e-6);
    }

    #[test]
    fn diagonal_is_clamped_between_min_and_max() {
        let size = Vec2::new(1.6, 0.9); // a 16:9 screen
        let current = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        let distance = 100.0; // a view circle wide enough for the MAX_DIAGONAL ceiling

        assert!((Screen::clamp_diagonal(size, current, distance, 1.0) - 1.0).abs() < 1e-6);
        assert!((Screen::clamp_diagonal(size, current, distance, 0.0) - Screen::MIN_DIAGONAL).abs() < 1e-6);
        assert!((Screen::clamp_diagonal(size, current, distance, 100.0) - Screen::MAX_DIAGONAL).abs() < 1e-6);
    }

    /// The toolbars are sized from the screen size and the ui zoom only: `toolbar_scale_for` has no
    /// distance parameter, which is what keeps a grab-bar drag from resizing them.
    #[test]
    fn toolbar_scale_follows_size_and_zoom_only() {
        let scale = Screen::toolbar_scale_for(4.4, 1.0);
        assert!(scale > Screen::UI_REFERENCE_DISTANCE);
        // The ui zoom is a plain multiplier.
        assert!((Screen::toolbar_scale_for(4.4, 2.0) - scale * 2.0).abs() < 1e-6);
        // A bigger screen means a bigger factor.
        assert!(Screen::toolbar_scale_for(8.0, 1.0) > scale);
        // A degenerate size falls back on the 1 m floor instead of collapsing the toolbars.
        assert!((Screen::toolbar_scale_for(0.0, 1.0) - Screen::toolbar_scale_for(1.0, 1.0)).abs() < 1e-6);
    }

    /// The focus volume must cover the strip above the screen — the grab bar, the hamburger button and
    /// first of all the scale knob, the only resize / zoom control — and must **not** include the
    /// screen surface itself: pointing at, or touching, the content is not a chrome interaction.
    #[test]
    fn focus_volume_is_the_strip_above_the_screen() {
        // A square screen (the demo) and a narrow one, where the knob hangs right of the mesh.
        for screen_size in [Vec2::new(1.024, 1.024), Vec2::new(0.32, 0.24)] {
            let mesh_z = 2.2;
            let mesh = Bounds::new(Vec3::new(0.0, 0.0, mesh_z), Vec3::new(screen_size.x, screen_size.y, 0.0));
            let factor = Screen::toolbar_scale_for((screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt(), 1.0);
            let focus = Screen::focus_bounds_for(mesh, factor, 0.1);

            // Element anchors written the way the drawing code writes them: the hamburger surface is
            // `0.06 * sqrt(UI_REFERENCE_DISTANCE) * 1.1` wide (see `draw_hamburger`) so its top is
            // half of that above its own anchor, and the knob sprite is `0.055 * ui_scale` wide (see
            // `Appearence::scale_handle`).
            let top = screen_size.y / 2.0;
            let bar = Vec3::new(0.0, top + 0.05 * factor, mesh_z);
            let hamburger_top = top + 0.04 * factor + 0.06 * Screen::UI_REFERENCE_DISTANCE.sqrt() * 1.1 * 0.5;
            let hamburger = Vec3::new(0.0, hamburger_top, mesh_z);
            let knob = Vec3::new(0.1 * factor + 0.08, top + 0.05 * factor, mesh_z);
            // Tested just behind the toolbar plane: the volume starts on it and only extends behind
            // (see `focus_volume_never_reaches_in_front_of_the_toolbar_plane`), while the real
            // pointer test works on any element on or behind that plane (see `draw_focus_probe`).
            for p in [bar, hamburger, knob, knob + Vec3::Y * 0.0275] {
                let behind = Vec3::new(p.x, p.y, p.z + 0.001);
                assert!(
                    focus.contains_point(behind),
                    "the focus volume must contain {behind:?} for a {screen_size:?} screen"
                );
            }
            // The strip stops on the top edge: the screen surface is not part of the volume.
            assert!(!focus.contains_point(Vec3::new(0.0, top - 0.01, mesh_z + 0.001)));
            assert!(!focus.contains_point(Vec3::new(0.0, 0.0, mesh_z + 0.001)));
        }
    }

    /// The focus volume must never extend towards the viewer, in front of the toolbar plane: the UI
    /// system keeps the **nearest** claim on an interactor, so the probe — an interactive UI element
    /// like any other (see `draw_focus_probe`) — would take the focus away from the grab bar, the
    /// scale knob and the hamburger button, which are anchored on or in front of that plane, and none
    /// of them could be grabbed or pressed anymore.
    #[test]
    fn focus_volume_never_reaches_in_front_of_the_toolbar_plane() {
        // A flat screen (both bounds), a curved one (the mesh wraps towards the viewer, so its bounds
        // have a Z extent while the toolbars stay on the bounds centre).
        for (screen_size, z_extent) in
            [(Vec2::new(1.024, 1.024), 0.0), (Vec2::new(0.32, 0.24), 0.0), (Vec2::new(3.84, 2.16), 0.35)]
        {
            let mesh = Bounds::new(
                Vec3::new(0.0, 0.0, 2.2 - z_extent * 0.5),
                Vec3::new(screen_size.x, screen_size.y, z_extent),
            );
            let factor = Screen::toolbar_scale_for((screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt(), 1.0);
            let focus = Screen::focus_bounds_for(mesh, factor, 0.1);
            let front_face = focus.center.z - focus.dimensions.z * 0.5;
            assert!(
                front_face >= mesh.center.z - 1e-6,
                "the focus volume starts at {front_face} for a {screen_size:?} screen, in front of the toolbar plane {}",
                mesh.center.z
            );
        }
    }

    /// A diagonal that does not fit the view circle is reduced, keeping the aspect ratio, and the
    /// effectiveness of the reduction is measurable: the size always fits `distance * PI`.
    #[test]
    fn clamped_diagonal_always_fits_the_view_circle() {
        let size = Vec2::new(1.6, 0.9);
        let current = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        let distance = 1.0;

        let effective = Screen::clamp_diagonal(size, current, distance, Screen::MAX_DIAGONAL);
        assert!(effective < Screen::MAX_DIAGONAL);
        let scaled = size * (effective / current);
        assert!(scaled.x <= Screen::max_size_for(distance) + 1e-6);
        assert!(scaled.y <= Screen::max_size_for(distance) + 1e-6);
    }
}
