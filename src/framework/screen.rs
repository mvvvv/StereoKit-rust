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
    ui::{Ui, UiBtnLayout, UiConfirm, UiMove, UiSettings, UiWin},
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
    id_slider_diagonal: String,
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
            id_slider_diagonal: id.clone() + "_slider_diagonal",
            sprite_close: Sprite::close(),
            sprite_menu: Sprite::list(),
            id_material: id + "_material",
        }
    }
}

/// The chrome scale and the boxes of the toolbar strip above a [`Screen`], in the screen's local space (`+x` to the
/// user's left, `+y` up, `+z` towards the viewer).
///
/// It is the single source of truth of the toolbar: the grab bar, the zoom knob, the parameter (hamburger) button, the
/// overlay text and the room an app reserves for its own row (see [`Screen::set_app_toolbar_width`]) are laid out from
/// it, and the focus probe of the screen wraps the strip. Every size is derived from [`Screen::toolbar_scale`] — a
/// bounded scale — so the whole strip follows the screen without ever dwarfing it.
///
/// The elements share one row, from the user's left to their right:
/// `[app row] · [knob] · [bar] · [hamburger] · [overlay text]`
/// — the **grab bar is centred on the screen's vertical centre line**, the zoom knob and the room of the app hang to
/// its left, the parameter button and the overlay text to its right, and the boxes never overlap whatever the screen
/// size, the ui zoom and the widths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarLayout {
    /// The bounded chrome scale of the screen (see [`Screen::toolbar_scale`]).
    pub scale: f32,
    /// The factor applied to the screen's own elements when the app row leaves too little room for them
    /// (see [`ToolbarLayout::for_size`]). `1.0` when everything fits.
    pub fit: f32,
    /// The gap between two elements of the row.
    pub gap: f32,
    /// The height of the centre line of the row, in the screen's local space.
    pub row_y: f32,
    /// The `z` of the row: the centre of the screen's bounds, where the framework's toolbars stand.
    pub z: f32,
    /// The distance-driving grab bar, on the row (its `z` dimension is the depth of its grab volume).
    pub bar: Bounds,
    /// The centre and the width of the zoom knob ([`Appearence::scale_handle`]).
    pub knob: (Vec3, f32),
    /// The parameter (hamburger) button.
    pub hamburger: Bounds,
    /// The centre and the size of the overlay text box.
    pub overlay: (Vec3, Vec2),
    /// The right edge (towards the user's right) of the room left for an app row, one gap past the knob.
    pub app_row_right: f32,
    /// The size of a **square button** of the row at this scale, [`ToolbarLayout::fit`] applied: the size an app row
    /// gives its own buttons, so the icons it wears are the same as the ones of the screen — the parameter button and
    /// the zoom knob — at any zoom.
    pub button: f32,
    /// The strip that covers the whole row; the focus probe of the screen adds the focus margin around it.
    pub strip: Bounds,
}

impl ToolbarLayout {
    /// The width of the parameter button at `scale = 1`: the height of a toolbar button, so the icon it wears
    /// (`button - 2 * padding` = 0.096 m) is the size the buttons of an app row on the strip give their own icons.
    const HAMBURGER: f32 = 0.116;
    /// The width of the zoom knob at `scale = 1`: the size of an icon of the row, optical bump included, so the knob
    /// reads as one of them.
    const KNOB: f32 = 0.105;
    /// The length of the grab bar at `scale = 1`, bounded so a large screen does not get a several metre grip.
    const BAR_LENGTH: f32 = 0.40;
    /// The shortest grip of the grab bar, in meters.
    const BAR_LENGTH_MIN: f32 = 0.20;
    /// The longest grip of the grab bar, in meters: it follows the zoom, up to this length.
    const BAR_LENGTH_MAX: f32 = 0.90;
    /// The thickness of the grab bar, as a fraction of a button of the row: a band under the icons, never a slab as
    /// tall as they are.
    const BAR_THICKNESS_RATIO: f32 = 0.45;
    /// The thinnest grab band, in meters.
    const BAR_THICKNESS_MIN: f32 = 0.030;
    /// The thickest grab band, in meters: a zoomed in screen does not turn the bar into a block.
    const BAR_THICKNESS_MAX: f32 = 0.070;
    /// The overlay text box at `scale = 1` (33 x 5.2 cm): the text of a `Screen` is a short status line, so the box is
    /// wider than it is high — [`TextFit::Exact`] then sizes the glyphs from its width, and its height is what the
    /// dark blue backplate that carries it takes.
    const OVERLAY_TEXT: Vec2 = Vec2::new(0.33, 0.052);
    /// The gap between two elements of the row, as a fraction of the parameter button.
    const GAP: f32 = 0.25;
    /// The smallest factor the screen's own elements are ever shrunk to: an app row wider than the screen overflows,
    /// but the framework's toolbar never disappears.
    const FIT_MIN: f32 = 0.25;

    /// Builds the layout of a screen `size` meters big — `zoom` times its base size, see [`Screen::toolbar_scale`] —
    /// with `app_width` meters reserved on the user's left for the row of an app, at the `z` of the screen's bounds.
    /// `margin` is the margin of the screen's scaled [`UiSettings`], the room kept free on both sides of the screen.
    ///
    /// The screen's own elements are shrunk by [`ToolbarLayout::fit`] when the app row leaves them less room than they
    /// need; the app row itself is never resized here — its owner reads [`ToolbarLayout::fit`] to follow.
    pub fn for_size(size: Vec2, zoom: f32, app_width: f32, z: f32, margin: f32) -> Self {
        let scale = Screen::toolbar_scale_for_zoom(zoom);
        let app_width = app_width.max(0.0);

        // The sizes at this scale, before the fit. `button` is the square button an app row gives its own buttons: its
        // icon is then the one of the parameter button, at any zoom.
        let button = Self::HAMBURGER * scale;
        let knob = Self::KNOB * scale;
        let bar_length = (Self::BAR_LENGTH * scale).clamp(Self::BAR_LENGTH_MIN, Self::BAR_LENGTH_MAX);
        let bar_thickness =
            (Self::BAR_THICKNESS_RATIO * button).clamp(Self::BAR_THICKNESS_MIN, Self::BAR_THICKNESS_MAX);
        let gap = Self::GAP * button;
        let overlay_height = Self::OVERLAY_TEXT.y * scale;
        // The overlay text keeps its designed width whatever room is left: squeezed into the margin of a small screen,
        // it would be unreadable. It may overflow a little rather than shrink.
        let overlay_width = Self::OVERLAY_TEXT.x * scale;

        // The fit only ever shrinks the screen's own chrome: the app row keeps the width its owner gave it.
        let own_width = overlay_width + gap + button + gap + bar_length + gap + knob + gap;
        let own_room = (size.x - 2.0 * margin - app_width).max(0.0);
        let fit = if own_width > own_room && own_width > f32::EPSILON {
            (own_room / own_width).clamp(Self::FIT_MIN, 1.0)
        } else {
            1.0
        };
        let (overlay_width, gap, button, bar_length, bar_thickness, knob) =
            (overlay_width * fit, gap * fit, button * fit, bar_length * fit, bar_thickness * fit, knob * fit);
        let overlay_height = overlay_height * fit;

        // One row, centred vertically on `row_y`, `gap / 2` above the top edge, and laid out around the grab bar, which
        // is centred on the screen's vertical centre line: the knob and the app's room to the user's left, the
        // parameter button and the overlay text to their right.
        let row_y = size.y * 0.5 + gap * 0.5 + button * 0.5;
        let knob_x = bar_length * 0.5 + gap + knob * 0.5;
        let hamburger_x = -(bar_length * 0.5 + gap + button * 0.5);
        let overlay_right = -(bar_length * 0.5 + gap + button + gap);
        let overlay_center = Vec3::new(overlay_right - overlay_width * 0.5, row_y, z);
        let bar_depth = (bar_thickness * 0.6).clamp(0.004, 0.020);

        let bar = Bounds::new(Vec3::new(0.0, row_y, z), Vec3::new(bar_length, bar_thickness, bar_depth));
        let hamburger = Bounds::new(Vec3::new(hamburger_x, row_y, z), Vec3::new(button, button, 0.0));
        let knob = (Vec3::new(knob_x, row_y, z), knob);
        let app_row_right = knob_x + knob.1 * 0.5 + gap;

        // The strip covers the row, the room of the app included.
        let left = overlay_center.x - overlay_width * 0.5;
        let right = app_row_right + app_width;
        let half_height = (button * 0.5).max(knob.1 * 0.5).max(overlay_height * 0.5).max(bar_thickness * 0.5);
        let strip = Bounds::new(
            Vec3::new((left + right) * 0.5, row_y, z),
            Vec3::new((right - left).max(0.0), half_height * 2.0, 0.0),
        );

        Self {
            scale,
            fit,
            gap,
            row_y,
            z,
            bar,
            knob,
            hamburger,
            overlay: (overlay_center, Vec2::new(overlay_width, overlay_height)),
            app_row_right,
            button,
            strip,
        }
    }
}

/// A virtual curved screen that can display a [`Tex`] or an OpenXR swapchain cylinder layer.
///
/// This is the refactored `Screen`: focus-driven toolbars, the [`Appearence`] scale knob and the parameter window
/// described below.
/// * [`Appearence`] is applied at the screen level: the screen is drawn at `Appearence::window_size * ui_scale`,
///   exactly like an `Appearence` window, and the [`Appearence::scale_handle`] knob anchored to the screen is THE
///   interactive diagonal / zoom control.The zoom follows the diagonal.
///
/// Like `Screen`, it is a concave mesh whose curvature, diagonal, and distance from the viewer are adjustable at
/// runtime. It also ships with:
/// * two spatial stereo audio streams (left / right)
/// * an optional single-line overlay text rendered above the content
/// * a parameter window (opened from the toolbar) with the screen diagonal and curvature sliders
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
/// screen.resolution(800, 600)
///       .screen_distance(2.3)   // 2.3 m away from the viewer
///       .set_overlay_text("Hello, Screen!")
///       .always_visible(true);
///
/// filename_scr = "screenshots/screen.jpeg"; fov_scr = 20.0;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     screen.draw(&token);
///
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
    /// Physical size of the screen in meters, the single source of truth for its geometry: the diagonal is
    /// derived from it (see [`Screen::get_screen_diagonal`]) and never stored.
    screen_size: Vec2,
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
    /// The width, in meters, an app reserves on the user's left of the toolbar for its own row (see
    /// [`Screen::set_app_toolbar_width`]): the strip covers it and the screen's own elements make room for it.
    app_toolbar_width: f32,
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
    /// The smallest chrome scale: a shrunk screen keeps a readable toolbar, but never bigger than itself.
    pub const TOOLBAR_SCALE_MIN: f32 = 0.75;
    /// The largest chrome scale: the toolbar follows the zoom of the screen (see [`Screen::toolbar_scale`]), up to
    /// the maximum zoom of the [`Appearence`] of the screen — four times the default size.
    pub const TOOLBAR_SCALE_MAX: f32 = 4.0;
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
            app_toolbar_width: 0.0,
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
        let layout = this.toolbar_layout();
        this.update_knob_anchor(&layout);

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

    /// Diagonal (meters) of a `size`.
    fn diagonal_of(size: Vec2) -> f32 {
        (size.x.powf(2.0) + size.y.powf(2.0)).sqrt()
    }

    /// Diagonal (meters) of the current [`Screen::screen_size`], always derived from it: the size is the single stored
    /// source of truth (it follows `Appearence::window_size * ui_scale`), so the diagonal is never cached in a field
    /// of its own. Only the ui zoom can make it change as long as the size stays fixed.
    fn diagonal(&self) -> f32 {
        Self::diagonal_of(self.screen_size)
    }

    /// Diagonal range the zoom can drive: [`Appearence::scale_bounds`] mapped through the base (unzoomed) size — the
    /// zoom is the diagonal of that base size — then clipped by [`Screen::MIN_DIAGONAL`] / [`Screen::MAX_DIAGONAL`] and
    /// by the view circle. [`Appearence::scale_bounds`] is the app's choice and is respected as-is: the zoom, and the
    /// diagonal with it, never leave it.
    pub fn diagonal_range(&self) -> (f32, f32) {
        Self::diagonal_range_for(
            self.appearence.window_size,
            self.appearence.scale_bounds,
            self.screen_size,
            self.screen_distance,
        )
    }

    /// [`Screen::diagonal_range`] as a pure function: `zoom_bounds` mapped through the diagonal of `base_size`, then
    /// clipped by [`Screen::MIN_DIAGONAL`] / [`Screen::MAX_DIAGONAL`] and by the view circle of `distance`.
    fn diagonal_range_for(base_size: Vec2, zoom_bounds: (f32, f32), size: Vec2, distance: f32) -> (f32, f32) {
        let base = Self::diagonal_of(base_size).max(f32::EPSILON);
        let min = (base * zoom_bounds.0).clamp(Self::MIN_DIAGONAL, Self::MAX_DIAGONAL);
        let max =
            Self::clamp_diagonal(size, distance, base * zoom_bounds.1).clamp(Self::MIN_DIAGONAL, Self::MAX_DIAGONAL);
        (min, max.max(min))
    }

    /// Clamp `diagonal` to [`Self::MIN_DIAGONAL`] / [`Self::MAX_DIAGONAL`], then return the diagonal really reachable
    /// for a screen of `size` at `distance`: the requested size is shrunk uniformly (the aspect ratio is preserved)
    /// when it would not fit the view circle. The diagonal of `size` is always derived, see [`Screen::diagonal_of`].
    fn clamp_diagonal(size: Vec2, distance: f32, diagonal: f32) -> f32 {
        let clamped = diagonal.clamp(Self::MIN_DIAGONAL, Self::MAX_DIAGONAL);
        let scaled = size * (clamped / Self::diagonal_of(size).max(f32::EPSILON));
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

    /// Set the screen size (in meters). The diagonal is derived from it, see [`Screen::get_screen_diagonal`].
    pub fn screen_size(&mut self, size: impl Into<Vec2>) -> &mut Self {
        let size = size.into();
        let max_size = Self::max_size_for(self.screen_distance);
        let diagonal = Self::diagonal_of(size);
        if size.x > 0.0 && size.y > 0.0 && size.x <= max_size && size.y <= max_size && diagonal > Self::MIN_DIAGONAL {
            self.screen_size = size;
            self.adapt_screen();
            self.sync_appearence_to_screen();
        }
        self
    }

    /// Set the screen diagonal. The zoom follows it — the screen is the base (unzoomed) size scaled by the diagonal
    /// ratio, and the [`Appearence`] zoom is set to that ratio — so the toolbars and the texts scale with the video. The
    /// zoom respects [`Appearence::scale_bounds`], so the reachable diagonals are
    /// `base diagonal * scale_bounds`, see [`Screen::diagonal_range`]; a request outside that range is clamped. The size
    /// is adjusted proportionally, so the aspect ratio never changes.
    pub fn screen_diagonal(&mut self, diagonal: f32) -> &mut Self {
        let (min, max) = self.diagonal_range();
        let effective = Self::clamp_diagonal(self.screen_size, self.screen_distance, diagonal).clamp(min, max);
        if (effective - self.diagonal()).abs() > f32::EPSILON {
            // The zoom is the diagonal relative to the base size, and the screen is the base size at that zoom — which
            // keeps `sync_appearence_to_screen` a no-op on the base. `scale_bounds` is respected: `effective` already
            // comes from `diagonal_range`, so `set_ui_scale` does not clamp.
            let base = Self::diagonal_of(self.appearence.window_size).max(f32::EPSILON);
            self.appearence.set_ui_scale(effective / base);
            self.screen_size = self.appearence.window_size * self.appearence.get_ui_scale();
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

    /// Is the screen a cylinder (`true`, the default) rather than a sphere centred on the viewer? What the "Free
    /// curvature" toggle of the parameter window flips (see [`Screen::cylindrical`]): a cylindrical screen snaps its
    /// curvature to one of the two ends of the slider, a free one takes any value in between. A caller that memorizes
    /// the choice of the user reads it here.
    pub fn is_cylindrical(&self) -> bool {
        self.cylindrical
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

    /// The [`UiSettings`] of the screen's [`Appearence`], scaled by its ui zoom: the padding, gutter and margin an
    /// app row hanging on the screen's toolbar should space itself with (see [`Screen::app_row_pose`]).
    pub fn ui_settings_scaled(&self) -> UiSettings {
        self.appearence.get_ui_settings_scaled()
    }

    /// The bounded scale of the screen's chrome (the grab bar, the zoom knob, the parameter button and the overlay
    /// text): it is the **zoom of the screen** — the ui scale of its [`Appearence`] — so the toolbar grows and shrinks
    /// with the video exactly like the zoom percentage the knob shows, the icons and their buttons at the same rate as
    /// the image, whatever the base size of the screen. It is capped between [`Screen::TOOLBAR_SCALE_MIN`] and
    /// [`Screen::TOOLBAR_SCALE_MAX`].
    ///
    /// It follows the screen zoom and **not** the distance: a grab-bar drag carries the toolbars with the screen
    /// instead of resizing them.
    pub fn toolbar_scale(&self) -> f32 {
        Self::toolbar_scale_for_zoom(self.appearence.get_ui_scale())
    }

    /// The size of a **square button** of the screen's toolbar at the current scale: an app row that hangs on the strip
    /// gives its own buttons that size (see [`Screen::app_row_pose`]), so their icons are the ones of the screen — the
    /// parameter button and the zoom knob — at any zoom.
    pub fn toolbar_button(&self) -> f32 {
        ToolbarLayout::HAMBURGER * self.toolbar_scale()
    }

    /// [`Screen::toolbar_scale`] as a pure function of the screen zoom: the ui scale of the screen's [`Appearence`],
    /// bounded. A zoom of `1.0` is the base size of the screen, whatever it is.
    fn toolbar_scale_for_zoom(zoom: f32) -> f32 {
        zoom.clamp(Self::TOOLBAR_SCALE_MIN, Self::TOOLBAR_SCALE_MAX)
    }

    /// The layout of the toolbar strip above the screen: the grab bar, the zoom knob, the parameter button, the
    /// overlay text and the room reserved for an app row, all in the screen's local space (see [`ToolbarLayout`]).
    pub fn toolbar_layout(&self) -> ToolbarLayout {
        ToolbarLayout::for_size(
            self.screen_size,
            self.toolbar_scale(),
            self.app_toolbar_width,
            self.screen.get_bounds().center.z,
            self.ui_settings_scaled().margin,
        )
    }

    /// Reserves `width` meters on the user's left of the toolbar for the row an app hangs there (see
    /// [`Screen::app_row_pose`]): the strip the focus probe covers grows with it, and the screen's own elements make
    /// room for it (see [`ToolbarLayout::fit`]). Pass `0.0` when the app shows no row.
    pub fn set_app_toolbar_width(&mut self, width: f32) -> &mut Self {
        self.app_toolbar_width = width.max(0.0);
        self
    }

    /// The world pose of a point of the screen's local space.
    pub fn local_pose(&self, local: Vec3) -> Pose {
        Pose::new(local, None) * self.screen_pose.to_matrix(None)
    }

    /// The pose to hand to [`Ui::push_surface`] (with [`Vec3::ZERO`] as its layout start) for an app row `width` x
    /// `height` meters, hanging one gap to the user's left of the screen's own toolbar (past the zoom knob), at the
    /// height of that toolbar.
    ///
    /// Declare the width first with [`Screen::set_app_toolbar_width`], so the strip of the screen covers the row and
    /// its own elements make room for it: the two widths are expected to be the same.
    pub fn app_row_pose(&self, width: f32, height: f32) -> Pose {
        let layout = self.toolbar_layout();
        let margin = self.ui_settings_scaled().margin;
        self.local_pose(Vec3::new(
            layout.app_row_right + width.max(0.0) + margin,
            layout.row_y + height * 0.5 + margin,
            layout.z,
        ))
    }

    /// The icon of the parameter (hamburger) button. Default is [`Sprite::list`]; an app that has its own hamburger
    /// icon sets it here, the way [`Appearence::handle_sprite`] replaces the visual of the zoom knob.
    pub fn set_param_icon(&mut self, sprite: Sprite) -> &mut Self {
        self.repo.sprite_menu = sprite;
        self
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

        // The strip of this frame: one row above the screen, its boxes never overlapping (see `ToolbarLayout`).
        let layout = self.toolbar_layout();
        // The grab handle first.
        self.draw_grab_handle(&layout);
        let bounds = self.screen.get_bounds();
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
            self.draw_scale_knob(&layout);
            // The hamburger button opens the parameter window.
            self.draw_hamburger(&layout);
            Ui::pop_id();
        }
        // The open parameter window keeps the toolbars visible but it never closes itself: only its close button does.
        self.draw_param_window(screen_transform, bounds.center);
        self.draw_overlay(screen_transform, &layout);
        // The focus probe is registered after every toolbar element.
        self.draw_focus_probe(&layout);

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
    fn draw_focus_probe(&mut self, layout: &ToolbarLayout) {
        let mut focused = BtnState::Inactive;
        Ui::push_surface(self.screen_pose, Vec3::ZERO, Vec2::ZERO);
        Ui::volume_at(&self.repo.id_focus_probe, self.focus_bounds(layout), UiConfirm::Push, None, Some(&mut focused));
        Ui::pop_surface();
        if focused.is_active() {
            self.refresh_focus();
        }
    }

    /// The screen grab bar.
    fn draw_grab_handle(&mut self, layout: &ToolbarLayout) {
        // Read before the drag is applied: the bar must not jump under the user's hand.
        let grabbed = Ui::handle(&self.repo.id_handle, &mut self.screen_pose, layout.bar)
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
    fn draw_scale_knob(&mut self, layout: &ToolbarLayout) {
        self.update_knob_anchor(layout);
        let scale_grabbed = self.appearence.scale_handle(&self.screen_pose, None, "h").is_some();
        self.sync_screen_to_appearence();
        // A pointer on the knob — the only resize / zoom control — is focus: keep the toolbars alive.
        self.refresh_focus_from_last_element();
        if scale_grabbed {
            // Grabbing is a pointer interaction too, even when the focus report above lags a frame.
            self.refresh_focus();
        }
    }

    /// Recompute the [`Appearence`] scale-handle anchor ([`crate::framework::ScaleHandleAnchor`]) from the layout of
    /// the strip: the knob rests where [`ToolbarLayout::knob`] says, and it is as big as its box — the knob that is
    /// seen is the knob that can be grabbed (see [`Appearence::handle_size`]).
    ///
    /// The screen is not a `Ui::window`, so the handle gets no bounds: its anchor starts from the screen centre line
    /// and from its top edge (`x` and `y` at `0.0`), and the offset holds the local place of the knob — on the toolbar
    /// row, 2 cm in front of its plane. The pose's forward axis points towards the user and the screen's local `+z`
    /// goes the other way, hence the negated plane depth.
    fn update_knob_anchor(&mut self, layout: &ToolbarLayout) {
        let ui_scale = self.appearence.get_ui_scale().max(f32::EPSILON);
        // `handle_size` is expressed at `ui_scale = 1`, while the layout already carries the current zoom.
        self.appearence.handle_size = Some(layout.knob.1 / ui_scale);
        self.appearence.scale_handle_anchor.x = 0.0;
        self.appearence.scale_handle_anchor.y = 0.0;
        self.appearence.scale_handle_anchor.offset =
            Vec3::new(layout.knob.0.x, layout.knob.0.y - self.screen_size.y * 0.5, -layout.z + 0.02);
    }

    /// The hamburger button, on its own surface anchored above the screen: it opens the parameter window.
    fn draw_hamburger(&mut self, layout: &ToolbarLayout) {
        if self.show_param {
            return;
        }

        let margin = self.ui_settings_scaled().margin;
        let btn_size = Vec2::new(layout.hamburger.dimensions.x, layout.hamburger.dimensions.y);
        let surface_size = btn_size * 1.1;
        // A `Ui::push_surface` lays its content out from its start towards the user's right (`-x`), one margin below
        // it: the pose is the top-right corner of the button, margin included, so the button lands on `hamburger`.
        let button_pose = self.local_pose(Vec3::new(
            layout.hamburger.center.x + margin + btn_size.x * 0.5,
            layout.hamburger.center.y + margin + btn_size.y * 0.5,
            layout.hamburger.center.z,
        ));
        Ui::push_surface(button_pose, Vec3::ZERO, surface_size);
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

    /// The parameter window, floating in front of the screen (fixed width, auto height). Holds the screen diagonal
    /// and curvature sliders, plus the optional app-provided extra-param UI. The diagonal slider keeps the aspect
    /// ratio, so it is the fallback when the [`Appearence::scale_handle`] knob cannot be grabbed.

    fn draw_param_window(&mut self, screen_transform: Matrix, bounds_center: Vec3) {
        if !self.show_param {
            return;
        }

        let info_position = Vec3::new(bounds_center.x, bounds_center.y, Self::PARAM_WINDOW_Z);
        let mut window_pose = Pose::new(info_position, None) * screen_transform;

        // The zoom follows the diagonal and this panel holds the diagonal slider.
        let zoom = self.appearence.get_ui_scale();
        self.appearence.set_ui_scale(1.0);
        let mut new_diagonal = None;

        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());

        // The width is `PARAM_WINDOW_WIDTH`: the panel is drawn at scale 1, so its width does not follow the zoom.
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

        // The diagonal row: label + value + slider. The aspect ratio is kept, so the slider uniformly resizes the
        // screen (`Screen::screen_diagonal`), exactly what the [`Appearence::scale_handle`] knob does when it is
        // grabbable. It is the fallback when the knob cannot be grabbed.
        Ui::push_tint(self.appearence.input_tint);
        Ui::label("Diagonal").use_padding(true).draw();
        Ui::same_line();
        Ui::label(format!("{:.2}", self.diagonal())).use_padding(true).draw();
        Ui::same_line();
        // The slider spans exactly the diagonals the zoom can reach.
        let (min_diagonal, max_diagonal) = self.diagonal_range();
        let mut diagonal = self.diagonal();
        if let Some(new_value) =
            Ui::hslider(&self.repo.id_slider_diagonal, &mut diagonal, min_diagonal, max_diagonal).interact()
        {
            new_diagonal = Some(new_value);
        }
        Ui::pop_tint();

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
        // Restore the caller's UiSettings and the zoom exactly as they were before this window was drawn.
        Ui::settings(prev_settings);
        self.appearence.set_ui_scale(zoom);
        if let Some(diagonal) = new_diagonal {
            self.screen_diagonal(diagonal);
        }

        if close {
            self.show_param = false;
        }
    }

    /// The overlay text, on its own surface anchored above the screen, to the user's right of the parameter button
    /// (see [`ToolbarLayout::overlay`]), on a **dark blue backplate** so it reads whatever the video behind it. Only
    /// drawn while the screen is focused.
    fn draw_overlay(&mut self, screen_transform: Matrix, layout: &ToolbarLayout) {
        if self.overlay_text.is_empty() || !self.screen_focused {
            return;
        }

        let (centre, size) = layout.overlay;
        let margin = self.ui_settings_scaled().margin;

        // One surface for the backplate **and** the text, so both live in the same frame and cannot drift apart: the
        // panel is the box of the text, a touch wider, centred on it.
        //
        // A `Ui` surface starts its content one margin towards the user's right and down from its pose, so the pose is
        // the top-right corner of the box (margin included) and the box's centre sits at `-margin - size / 2` in the
        // surface's frame. `Ui::panel_at` takes the **top-right corner** of the element it draws, as the layout does:
        // its centre is half a panel back from there.
        let overlay_pose =
            Pose::new(Vec3::new(centre.x + margin + size.x * 0.5, centre.y + margin + size.y * 0.5, centre.z), None)
                * screen_transform;
        Ui::push_surface(overlay_pose, Vec3::ZERO, Vec2::ZERO);

        let box_centre = Vec3::new(-margin - size.x * 0.5, -margin - size.y * 0.5, 0.0);
        let panel = Vec2::new(size.x * 1.03, size.y * 1.15);
        Ui::push_tint(named_colors::DARK_BLUE);
        Ui::panel_at(Vec3::new(box_centre.x + panel.x * 0.5, box_centre.y + panel.y * 0.5, box_centre.z), panel, None);
        Ui::pop_tint();

        // The text, drawn a little in front of the surface, so it stays on top of the panel.
        Ui::text(&self.overlay_text).size(size).text_align(Align::Center).fit(TextFit::Exact).draw();
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
        let diagonal = Self::diagonal_of(size);
        let max_size = Self::max_size_for(self.screen_distance);
        if width > 0 && height > 0 && diagonal > Self::MIN_DIAGONAL && size.x <= max_size && size.y <= max_size {
            self.width = width;
            self.height = height;
            self.screen_size = size;
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

    /// Get the current screen diagonal (meters), derived from [`Screen::get_screen_size`].
    pub fn get_screen_diagonal(&self) -> f32 {
        self.diagonal()
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

    /// Bounds of the focus volume, in the screen's model space: the toolbar strip of `layout`, widened by the focus
    /// margin, and extending behind its plane (never in front of it, see [`Screen::focus_bounds_for`]).
    fn focus_bounds(&self, layout: &ToolbarLayout) -> Bounds {
        Self::focus_bounds_for(self.screen.get_bounds(), layout.strip, self.focus_margin)
    }

    /// [`Screen::focus_bounds`] as a pure function (mesh bounds, toolbar strip, margin), so the geometry it guarantees
    /// can be unit tested. The volume sits on the strip and extends **backwards** from the toolbar plane: a slab in
    /// front of that plane would beat the toolbar elements in the UI focus test and make them unusable.
    fn focus_bounds_for(mesh_bounds: Bounds, strip: Bounds, margin: f32) -> Bounds {
        let depth = mesh_bounds.dimensions.z * 0.5 + margin;
        // The volume stops on the top edge of the screen: the surface itself is never part of the chrome.
        let screen_top = mesh_bounds.center.y + mesh_bounds.dimensions.y * 0.5;
        let bottom = (strip.center.y - strip.dimensions.y * 0.5 - margin).max(screen_top);
        let top = strip.center.y + strip.dimensions.y * 0.5 + margin;
        let height = (top - bottom).max(0.0);
        Bounds::new(
            Vec3::new(strip.center.x, bottom + height * 0.5, mesh_bounds.center.z + depth * 0.5),
            Vec3::new(strip.dimensions.x + 2.0 * margin, height, depth),
        )
    }

    /// Returns a world-space [`Pose`] **above the toolbar strip** of the screen, at the top-right corner of a surface
    /// anchored there (see [`Ui::push_surface`]): the content of the surface starts one margin below and to the user's
    /// right of it, so a row of [`Ui::push_surface(pose, Vec3::ZERO, size)`](Ui::push_surface) stands above the
    /// screen's own toolbar, extending towards the user's right.
    ///
    /// The pose is one gap and one toolbar row above the top of the strip (see [`Screen::toolbar_layout`]), which is
    /// where a row as high as the screen's own buttons (`ToolbarLayout::hamburger`) lands: a taller row overlaps the
    /// strip — add its extra height to `offset.y`, or better, hang it on the strip itself with
    /// [`Screen::set_app_toolbar_width`] and [`Screen::app_row_pose`], so it shares the screen's row and never covers
    /// the grab bar. `offset` is added in screen-local space before applying the screen transform.
    ///
    /// Size the window and its `offset` from [`Screen::toolbar_scale`] — never from the screen distance — so that a
    /// distance drag carries the window instead of resizing it.
    pub fn get_top(&self, offset: impl Into<Vec3>) -> Pose {
        let layout = self.toolbar_layout();
        let margin = self.ui_settings_scaled().margin;
        let row = layout.hamburger.dimensions.y.max(layout.knob.1);
        let top = layout.strip.center.y + layout.strip.dimensions.y * 0.5 + layout.gap + row;
        let pos = Vec3::new(0.0, top + margin, layout.z) + offset.into();
        Pose::new(pos, None) * self.screen_pose.to_matrix(None)
    }

    /// Returns a world-space [`Pose`] on the bottom edge of the screen, one gap below it, at the **top-right corner of
    /// a surface** anchored there (see [`Ui::push_surface`]): the content of the surface starts one margin below and to
    /// the user's right of it, so a row hangs under the screen and extends towards the user's right.
    ///
    /// `offset` is added in screen-local space before applying the screen transform.
    /// Useful for anchoring a UI window (e.g. transport controls) with [`UiMove::None`].
    ///
    /// Like [`Screen::get_top`], size the window and its `offset` from [`Screen::toolbar_scale`], never from the screen
    /// distance.
    pub fn get_bottom(&self, offset: impl Into<Vec3>) -> Pose {
        let layout = self.toolbar_layout();
        let margin = self.ui_settings_scaled().margin;
        let pos = Vec3::new(0.0, -(self.screen_size.y * 0.5 + layout.gap) + margin, layout.z) + offset.into();
        Pose::new(pos, None) * self.screen_pose.to_matrix(None)
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

    /// `scale_bounds` is respected: the reachable diagonals are the bounds mapped through the base size.
    #[test]
    fn diagonal_range_respects_scale_bounds() {
        let base = Vec2::new(0.32, 0.24); // a 0.4 m diagonal
        let base_diagonal = Screen::diagonal_of(base);
        // A wide view circle: the zoom bounds alone decide.
        let (min, max) = Screen::diagonal_range_for(base, (0.5, 4.0), base, 100.0);
        assert!((min - 0.5 * base_diagonal).abs() < 1e-4);
        assert!((max - 4.0 * base_diagonal).abs() < 1e-4);
        // `MIN_DIAGONAL` stays a floor even when the zoom bounds ask for less.
        let (min, _) = Screen::diagonal_range_for(base, (0.1, 4.0), base, 100.0);
        assert!((min - Screen::MIN_DIAGONAL).abs() < 1e-6);
        // The view circle can cap the max below the zoom bound.
        let (_, max) = Screen::diagonal_range_for(base, (0.5, 4.0), base, 0.1);
        assert!(max < 4.0 * base_diagonal);
    }

    #[test]
    fn diagonal_is_clamped_between_min_and_max() {
        let size = Vec2::new(1.6, 0.9); // a 16:9 screen
        let distance = 100.0; // a view circle wide enough for the MAX_DIAGONAL ceiling

        assert!((Screen::clamp_diagonal(size, distance, 1.0) - 1.0).abs() < 1e-6);
        assert!((Screen::clamp_diagonal(size, distance, 0.0) - Screen::MIN_DIAGONAL).abs() < 1e-6);
        assert!((Screen::clamp_diagonal(size, distance, 100.0) - Screen::MAX_DIAGONAL).abs() < 1e-6);
    }

    /// The chrome scale is the zoom of the screen: `1.0` for its base size, `2.0` zoomed twice, capped at the maximum
    /// zoom of the screen's `Appearence` and floored for a shrunk one.
    #[test]
    fn toolbar_scale_follows_the_zoom_once() {
        assert!((Screen::toolbar_scale_for_zoom(1.0) - 1.0).abs() < 1e-6);
        assert!((Screen::toolbar_scale_for_zoom(2.0) - 2.0).abs() < 1e-6);
        assert!((Screen::toolbar_scale_for_zoom(4.0) - 4.0).abs() < 1e-6);
        // The bounds hold whatever the zoom.
        assert!((Screen::toolbar_scale_for_zoom(8.0) - Screen::TOOLBAR_SCALE_MAX).abs() < 1e-6);
        assert!((Screen::toolbar_scale_for_zoom(0.1) - Screen::TOOLBAR_SCALE_MIN).abs() < 1e-6);
        // The zoom is the sole input: the base size of the screen plays no part in it.
        for zoom in [0.5_f32, 1.0, 1.5, 2.0, 3.0, 4.0] {
            let scale = Screen::toolbar_scale_for_zoom(zoom);
            assert!((Screen::TOOLBAR_SCALE_MIN..=Screen::TOOLBAR_SCALE_MAX).contains(&scale));
            assert!((scale - zoom.max(Screen::TOOLBAR_SCALE_MIN)).abs() < 1e-6);
        }
    }

    /// A box centred on `centre`, `size` meters wide and high, in the plane of the toolbar.
    fn box_at(centre: Vec3, size: Vec2) -> Bounds {
        Bounds::new(centre, Vec3::new(size.x, size.y, 0.0))
    }

    /// Do two boxes of the toolbar strip share any surface?
    fn overlaps(a: &Bounds, b: &Bounds) -> bool {
        (a.center.x - b.center.x).abs() < (a.dimensions.x + b.dimensions.x) * 0.5 - 1e-6
            && (a.center.y - b.center.y).abs() < (a.dimensions.y + b.dimensions.y) * 0.5 - 1e-6
    }

    /// Is `inner` entirely inside `outer`?
    fn contains(outer: &Bounds, inner: &Bounds) -> bool {
        (outer.center.x - inner.center.x).abs() + inner.dimensions.x * 0.5 <= outer.dimensions.x * 0.5 + 1e-6
            && (outer.center.y - inner.center.y).abs() + inner.dimensions.y * 0.5 <= outer.dimensions.y * 0.5 + 1e-6
    }

    /// One row, no overlaps, the whole chrome inside the strip — for every realistic screen size and for an app row of
    /// the width the remote sub-menu of Sk-Streams takes (0.63 m at scale 1). The bar stays a bounded grip.
    #[test]
    fn toolbar_layout_is_one_row_without_overlaps() {
        // No app row, a small one, and the three-button sub-menu of the remote desktop of Sk-Streams.
        let app_widths = [0.0, 0.30, 0.63];
        // (screen size, zoom of the screen): the chrome scale follows the zoom, whatever the base size.
        let cases = [
            (Vec2::new(0.32, 0.24), 1.0),
            (Vec2::new(1.024, 1.024), 1.0),
            (Vec2::new(1.024, 1.024), 4.0),
            (Vec2::new(1.6, 0.9), 1.0),
            (Vec2::new(3.84, 2.16), 1.0),
            (Vec2::new(7.68, 4.32), 2.0),
            (Vec2::new(13.33, 7.5), 4.0),
        ];
        for (size, zoom) in cases {
            for app_width in app_widths {
                let layout = ToolbarLayout::for_size(size, zoom, app_width, 2.2, 0.01);
                // The bar is a grip, never a beam.
                assert!(layout.bar.dimensions.x <= ToolbarLayout::BAR_LENGTH_MAX + 1e-6);
                assert!(layout.bar.dimensions.y <= ToolbarLayout::BAR_THICKNESS_MAX + 1e-6);
                assert!(
                    layout.bar.dimensions.y < layout.button,
                    "the bar must stay a band under the icons, not a block as tall as they are"
                );
                // The button size the layout hands out is the one of the parameter button, knobs included.
                assert!((layout.hamburger.dimensions.x - layout.button).abs() < 1e-6);
                assert!((layout.knob.1 - ToolbarLayout::KNOB / ToolbarLayout::HAMBURGER * layout.button).abs() < 1e-6);
                // The row holds the overlay, the hamburger, the bar and the knob, and none of them overlap.
                let boxes = [
                    layout.bar,
                    layout.hamburger,
                    box_at(layout.knob.0, Vec2::new(layout.knob.1, layout.knob.1)),
                    box_at(layout.overlay.0, layout.overlay.1),
                ];
                for (index, a) in boxes.iter().enumerate() {
                    for b in &boxes[index + 1..] {
                        assert!(!overlaps(a, b), "{a:?} overlaps {b:?} for {size:?} and an app row of {app_width}");
                    }
                }
                // The strip covers them all, and reaches the room of the app row.
                for b in boxes {
                    assert!(contains(&layout.strip, &b), "{b:?} is outside the strip {strip:?}", strip = layout.strip);
                }
                assert!(
                    layout.strip.center.x + layout.strip.dimensions.x * 0.5 >= layout.app_row_right + app_width - 1e-6
                );
                // The knob is the grab target: its box is on the row.
                assert!((layout.knob.0.y - layout.row_y).abs() < 1e-6);
                // The row is regular: one gap between every two elements, and the grab bar is centred on the screen.
                let hamburger_left = layout.hamburger.center.x - layout.hamburger.dimensions.x * 0.5;
                let hamburger_right = layout.hamburger.center.x + layout.hamburger.dimensions.x * 0.5;
                let overlay_right = layout.overlay.0.x + layout.overlay.1.x * 0.5;
                let bar_left = layout.bar.center.x - layout.bar.dimensions.x * 0.5;
                let bar_right = layout.bar.center.x + layout.bar.dimensions.x * 0.5;
                let knob_left = layout.knob.0.x - layout.knob.1 * 0.5;
                let knob_right = layout.knob.0.x + layout.knob.1 * 0.5;
                assert!(layout.bar.center.x.abs() < 1e-6, "the grab bar is centred on the screen");
                assert!((hamburger_left - overlay_right - layout.gap).abs() < 1e-6);
                assert!((bar_left - hamburger_right - layout.gap).abs() < 1e-6);
                assert!((knob_left - bar_right - layout.gap).abs() < 1e-6);
                assert!((layout.app_row_right - knob_right - layout.gap).abs() < 1e-6);
            }
        }
    }

    /// The focus volume must cover the strip above the screen — the grab bar, the hamburger button, the overlay text
    /// and first of all the scale knob, the only resize / zoom control — and must **not** include the screen surface
    /// itself: pointing at, or touching, the content is not a chrome interaction.
    #[test]
    fn focus_volume_is_the_strip_above_the_screen() {
        for screen_size in [Vec2::new(1.024, 1.024), Vec2::new(0.32, 0.24), Vec2::new(3.84, 2.16)] {
            let mesh_z = 2.2;
            let mesh = Bounds::new(Vec3::new(0.0, 0.0, mesh_z), Vec3::new(screen_size.x, screen_size.y, 0.0));
            let layout = ToolbarLayout::for_size(screen_size, 1.0, 0.63, mesh_z, 0.01);
            let focus = Screen::focus_bounds_for(mesh, layout.strip, 0.1);
            // The corners of the strip are in the volume — tested just behind the toolbar plane: the volume starts on
            // it and only extends behind (see `focus_volume_never_reaches_in_front_of_the_toolbar_plane`), while the
            // real pointer test works on any element on or behind that plane (see `draw_focus_probe`).
            let strip = layout.strip;
            for (x, y) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let behind = Vec3::new(
                    strip.center.x + x * strip.dimensions.x * 0.5,
                    strip.center.y + y * strip.dimensions.y * 0.5,
                    mesh_z + 0.001,
                );
                assert!(
                    focus.contains_point(behind),
                    "the focus volume must contain {behind:?} for a {screen_size:?} screen"
                );
            }
            // The strip stops on the top edge: the screen surface is not part of the volume.
            assert!(!focus.contains_point(Vec3::new(0.0, screen_size.y / 2.0 - 0.01, mesh_z + 0.001)));
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
            let layout = ToolbarLayout::for_size(screen_size, 1.0, 0.0, mesh.center.z, 0.0);
            let focus = Screen::focus_bounds_for(mesh, layout.strip, 0.1);
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
        let current = Screen::diagonal_of(size);
        let distance = 1.0;

        let effective = Screen::clamp_diagonal(size, distance, Screen::MAX_DIAGONAL);
        assert!(effective < Screen::MAX_DIAGONAL);
        let scaled = size * (effective / current);
        assert!(scaled.x <= Screen::max_size_for(distance) + 1e-6);
        assert!(scaled.y <= Screen::max_size_for(distance) + 1e-6);
    }
}
