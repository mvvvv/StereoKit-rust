use crate::{
    font::Font,
    maths::{Bounds, Matrix, Pose, Quat, Vec2, Vec3},
    sprite::Sprite,
    system::{Input, Log, Pivot, Text, TextBuilder, TextStyle},
    ui::{Ui, UiSettings},
    util::{Color128, named_colors},
};
#[cfg(feature = "placement")]
use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

/// How the [`Appearence::scale_handle`] knob resizes and zooms the window. It combines the former aspect-ratio
/// flag with the choice of the drag axes the knob cares about:
/// - the in-plane axes (local X = width, local Y = height) either resize [`Appearence::window_size`] freely, resize
///   it keeping its aspect ratio, or are ignored,
/// - the local Z axis (towards / away from the user) either drives the ui scale (zoom, see
///   [`Appearence::get_ui_scale`]) or is ignored.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Resizing {
    /// Free resize (local X = width, local Y = height) and zoom (local Z). This is the default.
    #[default]
    Free,
    /// Uniform resize — one size factor for both axes, keeping the `window_size` aspect ratio — and zoom.
    ///
    /// A [`Appearence::window_size`] coordinate left at `0.0` marks an unmanaged axis, which this mode cannot cope
    /// with: [`Appearence::start`] turns it into [`Resizing::Free`] with a warning.
    KeepRatio,
    /// Zoom only (local Z): the in-plane axes are ignored and `window_size` never changes, so the whole window zooms
    /// uniformly — its diagonal, for a window with a locked aspect ratio. Meant for the video
    /// [`crate::framework::Screen`], whose knob then only manages the diagonal. While grabbed, only the zoom label is
    /// drawn.
    ZoomOnly,
    /// Free resize (local X = width, local Y = height), zoom forbidden (the local Z axis is ignored).
    FreeNoZoom,
    /// Uniform resize keeping the `window_size` aspect ratio, zoom forbidden (the local Z axis is ignored).
    KeepRatioNoZoom,
}

impl Resizing {
    /// Does this mode resize [`Appearence::window_size`] with the in-plane drag axes?
    pub fn resizes_window(self) -> bool {
        matches!(self, Resizing::Free | Resizing::KeepRatio | Resizing::FreeNoZoom | Resizing::KeepRatioNoZoom)
    }

    /// Does this mode keep the `window_size` aspect ratio while resizing?
    pub fn keeps_ratio(self) -> bool {
        matches!(self, Resizing::KeepRatio | Resizing::KeepRatioNoZoom)
    }

    /// Does the local Z axis drive the ui scale (zoom)?
    pub fn zooms(self) -> bool {
        matches!(self, Resizing::Free | Resizing::KeepRatio | Resizing::ZoomOnly)
    }

    /// The same resize behavior with the aspect-ratio lock removed, the zoom flag unchanged.
    fn without_ratio(self) -> Self {
        match self {
            Resizing::KeepRatio => Resizing::Free,
            Resizing::KeepRatioNoZoom => Resizing::FreeNoZoom,
            other => other,
        }
    }
}

/// The look & feel of a UI window: size, scaling, text styles and extra tints.
/// - the ui scale (read with [`Appearence::get_ui_scale`], set with [`Appearence::start`] or
///   [`Appearence::set_ui_scale`]) uniformly scales the whole window: its size, every [`UiSettings`] value and
///   the `layout_height` of the four text styles,
/// - [`Appearence::scale_handle`] draws the grab-able knob that interactively drives [`Appearence::window_size`]
///   (local X = width, local Y = height) and the ui scale (local Z), as chosen by [`Appearence::resizing`] — see
///   [`Resizing`] — and [`Appearence::handle_sprite`], when set, replaces the built-in knob visual with a custom
///   sprite,
/// - the [`Appearence::scale_handle`] knob turns to face the head while an interactor points at it (based on the
///   previous frame's focus), so aiming at it is easy to see,
/// - the four text styles, from the biggest ([`Appearence::title_style`]) to the smallest
///   ([`Appearence::small_style`]), give the UI some relief,
/// - the three tints color directory buttons, input fields and error entries,
/// - [`Appearence::double_click_delay`] is the maximum delay between the two presses of a "double-click",
/// - the `placement` option (the `placement` feature) memorizes where the window stands, how big it is and the ui scale
///   it was left at, so the next session opens it there.
///
/// Call [`Appearence::start`] once when the window stepper starts (it captures the base text heights and applies the
/// current scale), then [`Appearence::scale_handle`] every frame after the window itself has been drawn.
pub struct Appearence {
    /// The `window_size` set by [`Appearence::default`], and the reference size the released scale-handle
    /// anchor is proportional to, see [`Appearence::scale_handle`]. Default is `Vec2::new(0.6, 0.8)`. A
    /// coordinate left at `0.0` marks an unmanaged axis (e.g. an auto-fit height): it turns a ratio-preserving
    /// [`Appearence::resizing`] off with a warning.
    pub window_size: Vec2,
    /// The reference size of the window used to apply scale and position of the [`Appearence::scale_handle`] this is
    /// always Vec2::new(0.6, 0.8),
    reference_window_size: Vec2,
    /// Interactive resize floor for [`Appearence::window_size`] (meters) applied while dragging the scale
    /// handle, see [`Appearence::scale_handle`]. Default is `Vec2::new(0.45, 0.45)`.
    pub min_window_size: Vec2,
    /// How the [`Appearence::scale_handle`] knob resizes [`Appearence::window_size`] and whether it zooms.Default is
    /// [`Resizing::Free`]: free in-plane resize and zoom.
    pub resizing: Resizing,
    /// The [`UiSettings`] used to draw the window. It is multiplied by the current ui scale (see
    /// [`Appearence::get_ui_scale`]) to give the settings returned by [`Appearence::get_ui_settings_scaled`].
    pub ui_settings: UiSettings,
    ui_settings_scaled: UiSettings,
    /// Scale factor of the whole window UI: the actual window size is `window_size * ui_scale` and every `UiSettings`
    /// value is multiplied by it during the window drawing. Default is 1.0 (no scaling).
    ui_scale: f32,
    /// How much the ui scale (see [`Appearence::get_ui_scale`]) grows per meter of scale-handle drag along the
    /// window-local Z axis (dragged towards the user = bigger, away from it = smaller). Default is 2.0.
    pub scale_per_meter: f32,
    /// Zoom bounds for the scale handle, see [`Appearence::scale_handle`]. Default is 0.5 to 2.0.
    pub scale_bounds: (f32, f32),
    /// Default window-local offset of the scale handle: on release, the handle springs back here, scaled
    /// proportionally to the current drawn window size (`window_size * ui_scale`) relative to
    /// `Appearence::reference_window_size`, so it hugs the window edge at its current size,
    /// see [`Appearence::scale_handle`]. Default is `Vec3::new(0.31, 0.04, 0.006)` you can change it as long as it's
    /// relative to `Appearence::reference_window_size` and on the right of the window .
    pub scale_handle_default_offset: Vec3,
    /// Current window-local offset of the scale handle, the grab-able knob that drives the ui scale (see
    /// [`Appearence::get_ui_scale`]) and [`Appearence::window_size`]: while held, dragging it resizes / scales
    /// the window relative to where it was grabbed. Initialized to [`Appearence::scale_handle_default_offset`],
    /// recomputed on each release.
    scale_handle_offset: Vec3,
    /// Current scale-grab session: the handle offset, the `ui_scale` and the `window_size` captured when the
    /// handle was grabbed, so each drag axis is applied as a delta from them.
    scale_grab: Option<(Vec3, f32, Vec2)>,

    /// Optional custom visual for the scale handle. Width is important as it's use to scale the sprite :
    /// `sprite.get_width() as f32 / 2000.0 * self.ui_scale`. 128 pixels for regular windows or 256 for large Screen.
    pub handle_sprite: Option<Sprite>,

    /// Did the scale handle have focus on the previous frame?
    handle_focused: bool,

    /// Text style of the header
    pub title_style: TextStyle,
    /// Text style of the list entries
    pub list_style: TextStyle,
    /// Text style of the secondary controls
    pub label_style: TextStyle,
    /// Text style of the small annotations
    pub small_style: TextStyle,
    /// Base (unscaled) `layout_height`s of the four text styles above at `start` so we can multiply them by `ui_scale`.
    text_base_heights: [f32; 4],

    /// Tints of the three main UI elements: directory buttons, input fields and error entries.
    pub button_tint: Color128,
    pub input_tint: Color128,
    pub error_tint: Color128,

    /// Maximum delay in seconds between the two presses (`JustActive`) of a "double-click" on an entry. Default is 0.5.
    pub double_click_delay: f32,

    /// Where the window is memorized between two sessions: its key, and what it takes to hand its placement.
    #[cfg(feature = "placement")]
    pub placement: Placement,
}

impl Default for Appearence {
    fn default() -> Self {
        // Font shared by the four text styles below.
        let font = Font::default();
        Self {
            reference_window_size: Vec2::new(0.6, 0.8),
            min_window_size: Vec2::new(0.45, 0.45),
            resizing: Resizing::default(),
            window_size: Vec2::new(0.6, 0.8),
            ui_settings: Ui::get_settings(),
            ui_scale: 1.0,
            scale_per_meter: 2.0,
            // Scale handle at its default anchor.
            scale_handle_default_offset: Vec3::new(0.31, 0.04, 0.006),
            scale_handle_offset: Vec3::ZERO,
            scale_grab: None,
            handle_sprite: None,
            handle_focused: false,
            scale_bounds: (0.5, 2.0),

            // Four text styles give the window some relief
            title_style: Text::make_style(&font, 0.012, named_colors::WHITE),
            list_style: Text::make_style(&font, 0.010, named_colors::LIGHT_GRAY),
            label_style: Text::make_style(&font, 0.009, named_colors::WHITE),
            small_style: Text::make_style(&font, 0.0075, named_colors::GRAY),
            text_base_heights: [0.0; 4],

            button_tint: named_colors::DARK_SLATE_GRAY.into(),
            input_tint: named_colors::SADDLE_BROWN.into(),
            error_tint: named_colors::RED.into(),
            double_click_delay: 0.5,
            #[cfg(feature = "placement")]
            placement: Placement::default(),

            ui_settings_scaled: Ui::get_settings(),
        }
    }
}

impl Appearence {
    /// Builds an [`Appearence`] with the provided `font` and a base `title_layout_height`: the four text styles
    /// share that font and their sizes are all derived from `title_layout_height` with the same proportions as
    /// [`Appearence::default`] (list 5/6, label 3/4 and small 5/8 of the title height). Every other property
    /// matches [`Appearence::default`], so the styles / tints can still be tweaked afterwards, before
    /// [`Appearence::start`].
    ///
    /// * `font` - Font shared by the four text styles of the window.
    /// * `title_layout_height` - Base `layout_height` (meters) of [`Appearence::title_style`], from which the
    ///   three other styles are scaled.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// # use stereokit_rust::{font::Font, framework::Appearence};
    /// let font = Font::default();
    /// let appearence = Appearence::new(&font, 0.024);
    /// assert!((appearence.title_style.get_layout_height() - 0.024).abs() < 0.00000001);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new(font: &Font, title_layout_height: f32) -> Self {
        Self {
            // Same size hierarchy as `Default` (title > list > label > small), scaled from `title_layout_height`.
            title_style: Text::make_style(font, title_layout_height, named_colors::WHITE),
            list_style: Text::make_style(font, title_layout_height * 5.0 / 6.0, named_colors::LIGHT_GRAY),
            label_style: Text::make_style(font, title_layout_height * 3.0 / 4.0, named_colors::WHITE),
            small_style: Text::make_style(font, title_layout_height * 5.0 / 8.0, named_colors::GRAY),
            ..Default::default()
        }
    }

    /// To be called once when the window stepper starts. Clamp the properties set before launch.
    pub fn start(&mut self) {
        // Bounds checks of the properties settable before launch, so no zero / negative / out-of-range
        // value can break the draw loop or the scale-handle interactions.
        const ABS_MIN_WINDOW: Vec2 = Vec2::new(0.01, 0.01); // hard floor for the resize floor itself
        self.min_window_size = Vec2::max(self.min_window_size, ABS_MIN_WINDOW);
        // A `window_size` coordinate at 0.0 (negatives are normalized to 0.0) is an unmanaged axis, which the
        // ratio-preserving resizing cannot cope with: it gets turned off.
        if self.resizing.keeps_ratio() && (self.window_size.x <= 0.0 || self.window_size.y <= 0.0) {
            Log::warn(
                "Appearence::start: a `window_size` coordinate is 0.0 (unmanaged axis), turning the ratio-preserving resizing off",
            );
            self.resizing = self.resizing.without_ratio();
        }
        if self.resizing.keeps_ratio() {
            // Ratio locked: lift `window_size` to the floor with one uniform factor, so the aspect ratio
            // survives the `min_window_size` clamp as well.
            let factor = (self.min_window_size.x / self.window_size.x.max(0.001))
                .max(self.min_window_size.y / self.window_size.y.max(0.001))
                .max(1.0);
            self.window_size *= factor;
        } else {
            // Unmanaged axes (coordinate at 0.0) stay at 0.0, exempt from the `min_window_size` floor.
            let window_size = self.window_size;
            self.set_window_size(window_size);
        }

        self.scale_handle_offset = self.scale_handle_default_offset;
        self.scale_per_meter = self.scale_per_meter.max(0.01);
        self.double_click_delay = self.double_click_delay.max(0.0);

        self.text_base_heights = [
            self.title_style.get_layout_height() / self.ui_scale,
            self.list_style.get_layout_height() / self.ui_scale,
            self.label_style.get_layout_height() / self.ui_scale,
            self.small_style.get_layout_height() / self.ui_scale,
        ];

        self.set_ui_scale(self.ui_scale);
    }

    /// Scale the window before or at start or after. Useful when you want to have the same look than the calling
    /// window or to adjust lazily your window. The scale is clamped to the same range the scale handle can drag it to.
    ///
    /// Call this when all the TextStyles have been set.
    pub fn set_ui_scale(&mut self, ui_scale: f32) {
        self.ui_scale = ui_scale.clamp(self.scale_bounds.0, self.scale_bounds.1);
        // Scale the four text styles with the new ui scale.
        self.scale_all();
    }

    /// Sets the base [`Appearence::window_size`] the window is drawn at in metres.
    pub fn set_window_size(&mut self, window_size: Vec2) {
        let size = Vec2::max(window_size, Vec2::ZERO);
        self.window_size = Vec2::new(
            if size.x > 0.0 { size.x.max(self.min_window_size.x) } else { size.x },
            if size.y > 0.0 { size.y.max(self.min_window_size.y) } else { size.y },
        );
    }

    /// Gives the window its memorized placement back — where it stands, how big it is and the ui scale it was left at,
    /// see [`Appearence::placement`].
    /// * `window_pose` - The pose of the window, given back its memorized one.
    #[cfg(feature = "placement")]
    pub fn restore_placement(&mut self, window_pose: &mut Pose) {
        let Some(placement) = self.placement.memorized() else {
            return;
        };
        Log::diag(format!(
            "[placement] {}: restored at ({:.2}, {:.2}, {:.2}), size {:?}, ui scale {:.2}",
            self.placement.key(),
            placement.pose.position.x,
            placement.pose.position.y,
            placement.pose.position.z,
            placement.window_size,
            placement.ui_scale,
        ));
        placement.apply_to(window_pose, self);
        self.placement.know(placement);
    }

    /// Offers the live placement of the window — where it stands, [`Appearence::window_size`] and its ui scale.
    /// * `window_pose` - Where the window stands.
    #[cfg(feature = "placement")]
    pub fn track_placement(&mut self, window_pose: &Pose) {
        let placement = WindowPlacement::of(window_pose, self);
        self.placement.offer(placement, false);
    }

    /// The same as [`Appearence::track_placement`], whatever the throttle: what the `close` of a window uses, so a
    /// window moved, resized or scaled since the last write keeps its last gesture for the next run.
    /// * `window_pose` - Where the window stands.
    #[cfg(feature = "placement")]
    pub fn flush_placement(&mut self, window_pose: &Pose) {
        let placement = WindowPlacement::of(window_pose, self);
        self.placement.offer(placement, true);
    }

    /// Scale the font sizes with ui_scale as well: `UiSettings` scaling does NOT affect text styles, so each of the
    /// four styles gets its base `layout_height` multiplied by the current scale.
    /// The size hierarchy title > list > label > small gives the UI some relief at every scale.
    fn scale_all(&mut self) {
        // Before start, the four text styles may have been tweaked by the user, so we capture their base heights here.
        if self.text_base_heights == [0.0; 4] {
            self.text_base_heights = [
                self.title_style.get_layout_height(),
                self.list_style.get_layout_height(),
                self.label_style.get_layout_height(),
                self.small_style.get_layout_height(),
            ]
        }
        let [title_h, list_h, label_h, small_h] = self.text_base_heights;
        self.title_style.layout_height(title_h * self.ui_scale);
        self.list_style.layout_height(list_h * self.ui_scale);
        self.label_style.layout_height(label_h * self.ui_scale);
        self.small_style.layout_height(small_h * self.ui_scale);
        self.ui_settings_scaled = self.ui_settings * self.ui_scale;
    }

    /// The [`UiSettings`] actually used to draw the window: [`Appearence::ui_settings`] already multiplied by the
    /// current ui scale (see [`Appearence::get_ui_scale`]) by the internal `scale_all`. Push them with
    /// [`Ui::settings`] before drawing the window, and restore the caller's settings afterwards.
    pub fn get_ui_settings_scaled(&self) -> UiSettings {
        self.ui_settings_scaled
    }

    /// The current ui scale. You should use [`Appearence::scaled_window_size`] [`Appearence::scale`] or [`Appearence::scale_size`] /
    /// [`Appearence::scale_pos`] to scale your own values, so they follow the window scaling.
    pub fn get_ui_scale(&self) -> f32 {
        self.ui_scale
    }

    /// The current window size scaled.
    pub fn scaled_window_size(&self) -> Vec2 {
        self.window_size * self.ui_scale
    }

    /// Multiplies `value` by the current ui scale ([`Appearence::get_ui_scale`]), so any size or offset of your
    /// own controls follows the window scaling: `appearence.scale(0.03)`
    pub fn scale(&self, value: f32) -> f32 {
        value * self.ui_scale
    }

    /// Same as [`Appearence::scale`], for `f64` values, handy for the APIs working in double precision.
    pub fn scale_f64(&self, value: f64) -> f64 {
        value * self.get_ui_scale() as f64
    }

    /// Multiplies size by the current ui scale ([`Appearence::get_ui_scale`]), so any size or offset of your
    /// own controls follows the window scaling: `appearence.scale(Vec2::new(0.03, 0.03))`
    pub fn scale_size(&self, size: Vec2) -> Vec2 {
        size * self.ui_scale
    }

    /// Multiplies position by the current ui scale ([`Appearence::get_ui_scale`]), so any size or offset of your
    /// own controls follows the window scaling: `appearence.scale(Vec3::new(0.03, 0.03, 0.001))`
    pub fn scale_pos(&self, position: Vec3) -> Vec3 {
        position * self.ui_scale
    }
    /// Scale handle: a small grab-able knob in world space, anchored to `window_pose` in its local space so it
    /// follows the window when it moves. While held, each drag axis drives its own appearance property, as chosen by
    /// [`Appearence::resizing`]:
    /// - the local X axis (the window width direction) modifies [`Appearence::window_size`].x,
    /// - the local Y axis (the window height direction) modifies [`Appearence::window_size`].y,
    /// - the local Z axis (towards / away from the user) modifies the ui scale (see [`Appearence::get_ui_scale`]),
    ///   which uniformly scales the whole window, see the internal `scale_all`.
    ///
    /// With [`Resizing::KeepRatio`], the local X and Y drag deltas are merged into a single uniform size factor
    /// instead, so the window keeps its aspect ratio while resizing. With [`Resizing::ZoomOnly`] the two in-plane axes
    /// are ignored and only the local Z axis is used: the `window_size` never changes, so the whole window zooms
    /// uniformly — its diagonal, for a window with a locked aspect ratio like [`crate::framework::Screen`]. The
    /// `NoZoom` variants ignore the local Z axis. A [`Appearence::window_size`] coordinate left at `0.0` marks an
    /// unmanaged axis.
    ///
    /// If the knob had focus on the previous frame, it is turned to face the head, so it is obvious that it is being
    /// aimed at. The cue is binary (no fade / timeout), and using the previous frame's focus keeps this frame's
    /// orientation from feeding back into the focus test.
    ///
    /// On release, the knob springs back to its default anchor, scaled proportionally to the current drawn
    /// window size (`window_size * ui_scale`) so it keeps hugging the window edge. While the handle is
    /// grabbed, two or three small labels around the knob show the live values it drives: the scale factor in
    /// percent in front of the knob (towards the user) when the mode zooms, the window width below it and the
    /// window height on its right when it resizes.
    ///
    /// When [`Appearence::handle_sprite`] is set, its sprite is drawn in place of the built-in knob visual,
    /// but the grab volume and the drag behavior stay the same.
    ///
    /// * `window_pose` - The world-space pose of the window the handle is anchored to.
    /// * `id` - The unique StereoKit UI id of the handle element. "h" is ok as long as you stay inside the window
    ///   [`Ui::push_id`]
    ///
    /// Returns `Some(ui_scale)` on every frame the handle is grabbed, so the caller can propagate the live
    /// scale to its child windows and `None` when the handle is not grabbed.
    pub fn scale_handle(&mut self, window_pose: &Pose, id: &str) -> Option<f32> {
        let (right, up, forward) = (window_pose.get_right(), window_pose.get_up(), window_pose.get_forward());

        let handle_offset = self.scale_handle_offset;
        let handle_center =
            window_pose.position + right * handle_offset.x + up * handle_offset.y + forward * handle_offset.z;
        let mut handle_pose = Pose::new(handle_center, Some(window_pose.orientation));
        // Focus cue: if the knob had focus on the previous frame, turn it to face the head.
        if self.handle_focused {
            handle_pose.orientation = Quat::look_at(handle_pose.position, Input::get_head().position, None);
        }
        // A custom handle sprite replaces the built-in knob visual, see the drawing after the grab logic.
        let draw_default_handle = self.handle_sprite.is_none();
        let grabbed =
            Ui::handle(id, &mut handle_pose, Bounds::bounds_centered(Vec3::new(1.0, 1.0, 0.3) * 0.035 * self.ui_scale))
                .draw_handle(draw_default_handle)
                .grab();
        self.handle_focused = grabbed || Ui::get_last_element_focused().is_active();
        let result = if grabbed {
            let delta = handle_pose.position - window_pose.position;
            let offset = Vec3::new(Vec3::dot(delta, right), Vec3::dot(delta, up), Vec3::dot(delta, forward));
            // Drag session start state: the handle offset, ui_scale and window_size at grab time, so each axis below
            // is applied as a delta from them.
            let (start_offset, start_scale, start_size) =
                *self.scale_grab.get_or_insert((handle_offset, self.ui_scale, self.window_size));

            // The in-plane axes resize the window (freely or keeping its aspect ratio); `ZoomOnly` ignores them.
            if self.resizing.resizes_window() {
                // A `window_size` coordinate at 0.0 is an unmanaged axis.
                if self.resizing.keeps_ratio() && start_size.x > 0.0 && start_size.y > 0.0 {
                    // Ratio locked: the X and Y drag deltas are averaged into one uniform size factor applied to
                    // both dimensions, and the `min_window_size` floor is applied to the factor itself, so the
                    // aspect ratio survives even the clamp.
                    let drag = (offset.x - start_offset.x + offset.y - start_offset.y) * 0.5;
                    let reference = (start_size.x + start_size.y) * 0.5;
                    let factor = ((reference + drag) / reference.max(0.001))
                        .max(self.min_window_size.x / start_size.x.max(0.001))
                        .max(self.min_window_size.y / start_size.y.max(0.001));
                    self.window_size = start_size * factor;
                } else {
                    // Unmanaged axes (coordinate at 0.0) stay at 0.0 whatever the drag.
                    if start_size.x > 0.0 {
                        self.window_size.x = (start_size.x + offset.x - start_offset.x).max(self.min_window_size.x);
                    }
                    if start_size.y > 0.0 {
                        self.window_size.y = (start_size.y + offset.y - start_offset.y).max(self.min_window_size.y);
                    }
                }
            }
            // The local Z axis (towards / away from the user) drives the ui scale, unless the mode forbids zoom.
            if self.resizing.zooms() {
                self.ui_scale = (start_scale + (offset.z - start_offset.z) * self.scale_per_meter)
                    .clamp(self.scale_bounds.0, self.scale_bounds.1);
            }
            self.scale_all();
            self.scale_handle_offset = offset;

            // While grabbed, small labels around the knob show the live values it drives.
            let label_size = Vec2::new(0.05, 0.02) * self.ui_scale;
            let orientation = handle_pose.orientation;
            if self.resizing.zooms() {
                TextBuilder::new(format!("{:.0}%", self.ui_scale * 100.0))
                    .transform(Matrix::t_r(handle_pose.position + forward * 0.05, orientation))
                    .style(self.title_style)
                    .size(label_size)
                    .add();
            }
            if self.resizing.resizes_window() {
                if start_size.x > 0.0 {
                    TextBuilder::new(format!("{:.2}", self.window_size.x))
                        .transform(Matrix::t_r(handle_pose.position - up * 0.03, orientation))
                        .style(self.small_style)
                        .size(label_size)
                        .add();
                }
                if start_size.y > 0.0 {
                    TextBuilder::new(format!("{:.2}", self.window_size.y))
                        .transform(Matrix::t_r(handle_pose.position + right * 0.04, orientation))
                        .style(self.small_style)
                        .size(label_size)
                        .add();
                }
            }
            // Live scale of this drag frame, for the caller to propagate to its child windows.
            Some(self.ui_scale)
        } else {
            // Released: the knob springs back to its default anchor, scaled proportionally to the current
            // drawn window size (`window_size * ui_scale`) so it keeps hugging the window edge.
            self.scale_grab = None;
            let drawn = self.window_size * self.ui_scale;
            self.scale_handle_offset = Vec3::new(
                self.scale_handle_default_offset.x * drawn.x / self.reference_window_size.x.max(0.001),
                self.scale_handle_default_offset.y * drawn.y / self.reference_window_size.y.max(0.001),
                self.scale_handle_default_offset.z,
            );
            None
        };

        // The placement of the window — where it stands, how big it is and the ui scale it was left at.
        #[cfg(feature = "placement")]
        self.track_placement(window_pose);

        // Custom handle visual: when set, the sprite is drawn in place of the built-in knob, centered on it
        // and scaled to its footprint (`0.055 * ui_scale` meters on its largest axis, aspect ratio preserved),
        // so it follows both the drag position and the window scaling. Drawn after the grab logic so the pose
        // used is the one updated by the drag of this frame.
        if let Some(sprite) = &self.handle_sprite {
            let size = sprite.get_width() as f32 / 2000.0 * self.ui_scale;
            let aspect = sprite.get_aspect();
            let scale = size / aspect.max(1.0);
            sprite.draw(handle_pose.to_matrix(Some(Vec3::new(scale * aspect, scale, 1.0))), Pivot::Center, None, None);
        }

        result
    }
}

/// The shortest delay between two writes of the same placement: a window dragged for a second costs one write — once
/// it came to rest — not sixty.
#[cfg(feature = "placement")]
pub const SAVE_THROTTLE: Duration = Duration::from_secs(1);

/// The placement of one window: what it takes to put it back where, at the size and at the scale it was left.
#[cfg(feature = "placement")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlacement {
    /// Where the window stands, in world space: the pose [`crate::ui::Ui::window`] was given.
    pub pose: Pose,
    /// The base [`Appearence::window_size`] of the window, in metres, *before* [`WindowPlacement::ui_scale`].
    /// `None` — the way a placement memorized before the size was tracked (or written by hand) reads — leaves the
    /// window the size its stepper gave it, instead of shrinking it to an arbitrary default.
    pub window_size: Option<Vec2>,
    /// The ui scale of the window, see [`Appearence::get_ui_scale`].
    pub ui_scale: f32,
}

#[cfg(feature = "placement")]
impl WindowPlacement {
    /// The live placement of a window: where `pose` puts it, what `appearence` sizes and scales it at.
    /// * `pose` - The pose of the window.
    /// * `appearence` - The look of the window.
    pub fn of(pose: &Pose, appearence: &Appearence) -> Self {
        Self { pose: *pose, window_size: Some(appearence.window_size), ui_scale: appearence.get_ui_scale() }
    }

    /// The placement of a window held as plain components:
    /// * `pos` - World-space position of the window, in metres.
    /// * `quat` - World-space orientation quaternion `(x, y, z, w)`.
    /// * `window_size` - The base size of the window (see [`WindowPlacement::window_size`]), `None` when the store holds
    ///   none.
    /// * `ui_scale` - The ui scale of the window.
    pub fn from_components(pos: [f32; 3], quat: [f32; 4], window_size: Option<[f32; 2]>, ui_scale: f32) -> Self {
        Self {
            pose: Pose::new(Vec3::new(pos[0], pos[1], pos[2]), Some(orientation_of(quat))),
            window_size: window_size.map(|size| Vec2::new(size[0], size[1])),
            ui_scale,
        }
    }

    /// Puts this placement back on a window.
    /// * `pose` - The pose of the window, replaced by the memorized one.
    /// * `appearence` - The look of the window, given back its memorized size and ui scale.
    pub fn apply_to(&self, pose: &mut Pose, appearence: &mut Appearence) {
        *pose = self.pose;
        if let Some(window_size) = self.window_size {
            appearence.set_window_size(window_size);
        }
        appearence.set_ui_scale(self.ui_scale);
    }
}

#[cfg(feature = "placement")]
impl Default for WindowPlacement {
    fn default() -> Self {
        Self { pose: Pose::IDENTITY, window_size: None, ui_scale: 1.0 }
    }
}

/// The orientation a placement is memorized with, repaired from the four components a store holds (see
/// [`WindowPlacement::from_components`]).
#[cfg(feature = "placement")]
fn orientation_of(quat: [f32; 4]) -> Quat {
    let mut orientation = Quat { x: quat[0], y: quat[1], z: quat[2], w: quat[3] };
    let length_sq: f32 = quat.iter().map(|component| component * component).sum();
    if length_sq > f32::EPSILON {
        orientation.normalize();
    } else {
        orientation = Quat::IDENTITY;
    }
    orientation
}

/// Where the placements of the windows of the session are kept between two runs: the app implements this once (a JSON
/// file, a configuration...) and installs it with [`set_sink`]. It is called on the main thread only, from
/// [`Appearence::restore_placement`] and the writes of [`Appearence::track_placement`] /
/// [`Appearence::flush_placement`].
#[cfg(feature = "placement")]
pub trait PlacementSink: Send + Sync {
    /// The placement memorized for the window named `key`, if any. Called once per window, when it starts.
    fn load(&self, key: &str) -> Option<WindowPlacement>;

    /// Memorizes `placement` for the window named `key`. Called when the placement really changed, at most once per
    /// [`SAVE_THROTTLE`] (and immediately on an [`Appearence::flush_placement`]).
    fn save(&self, key: &str, placement: WindowPlacement);
}

/// The sink of the session, `None` until the app installs one (see [`set_sink`]).
#[cfg(feature = "placement")]
static SINK: OnceLock<Arc<dyn PlacementSink>> = OnceLock::new();

/// Installs the sink where the placements of every window are kept, once at start-up. Ignored when a sink is already
/// installed (the first one wins, like [`crate::system::Log::subscribe`]): a session has one place to remember windows
/// in, whichever part of the app asks for it first.
/// * `sink` - The store of the app, see [`PlacementSink`].
#[cfg(feature = "placement")]
pub fn set_sink(sink: Arc<dyn PlacementSink>) {
    let _ = SINK.set(sink);
}

/// The sink of the session, if the app installed one: without it, no window is memorized at all.
#[cfg(feature = "placement")]
pub fn sink() -> Option<&'static Arc<dyn PlacementSink>> {
    SINK.get()
}

/// The placement option of one window, held by [`Appearence`] (see [`Appearence::placement`]): the **key** the window
/// is memorized under, and what it takes to hand its placement to the [`PlacementSink`] of the app when it changed.
///
/// The key must mean the same window at the next run: the id the window stepper is added under for a window there is
/// only one of (a tool names itself with it, see [`Placement::ensure_key`]), the rank it is opened at for a window
/// several of which live side by side (the id of a file browser is unique to its session, the rank of its launcher is
/// not: the app gives the key, see [`Placement::ensure_key`]).
///
/// The option is inert — nothing is loaded, nothing is written — when the app installed no sink or when the window has
/// no key, which is what [`Placement::default`] is.
#[cfg(feature = "placement")]
#[derive(Debug, Default)]
pub struct Placement {
    /// The key the window is memorized under. Empty: the window is not memorized.
    key: String,
    /// The last placement the caller offered, whether it was written or held back by the throttle.
    observed: Option<WindowPlacement>,
    /// The last placement handed to the sink (or restored from it), for the change detection.
    written: Option<WindowPlacement>,
    /// When the last write happened, for the throttle. `None` before the first write of the session.
    last_save: Option<Instant>,
}

#[cfg(feature = "placement")]
impl Placement {
    /// The placement option of the window memorized under `key`.
    /// * `key` - The key the window is memorized under, see [`Placement::ensure_key`].
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into(), observed: None, written: None, last_save: None }
    }

    /// The key the window is memorized under.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Gives the option `key` **when it has none**: a tool names itself with the id it is added under, and the app names
    /// the windows it opens several times — the key must mean the same window at the next run, which the id of a window
    /// opened this session does not. This way, whichever of the two asks, the other one does not overwrite it.
    /// * `key` - The key to use when the option has none.
    pub fn ensure_key(&mut self, key: &str) {
        if self.key.is_empty() {
            self.key = key.to_string();
        }
    }

    /// The placement memorized for this window by the previous run, if any (see [`PlacementSink::load`]).
    pub(crate) fn memorized(&self) -> Option<WindowPlacement> {
        sink().and_then(|sink| sink.load(&self.key))
    }

    /// Remembers `placement` as the one now in place: nothing is written back before the user moves the window (see
    /// [`WindowPlacement::apply_to`], which is what applies it).
    pub(crate) fn know(&mut self, placement: WindowPlacement) {
        self.observed = Some(placement);
        self.written = Some(placement);
    }

    /// Hands `placement` to the [`PlacementSink`] of the app when it changed, at most once per [`SAVE_THROTTLE`] — or
    /// immediately when `force`, which is what the `close` of a window does.
    pub(crate) fn offer(&mut self, placement: WindowPlacement, force: bool) {
        self.observed = Some(placement);
        if self.written.as_ref() == Some(&placement) {
            return;
        }
        if self.key.is_empty() {
            return;
        }
        let Some(sink) = sink() else {
            return;
        };
        let due = force || self.last_save.is_none_or(|last_save| last_save.elapsed() >= SAVE_THROTTLE);
        if !due {
            return;
        }
        sink.save(&self.key, placement);
        self.written = Some(placement);
        self.last_save = Some(Instant::now());
    }
}

// ── A store ready to use: one JSON map file (the `placement` feature) ───────────────────────────────────────────
//
// Any app that wants its windows back where they were can stop here: [`set_json_file`] installs a [`JsonPlacementStore`]
// — one JSON map file, keyed like the store of any app — and the `Appearence::placement` option does the rest. The
// file holds one [`StoredPlacement`] per window, converted from and to [`WindowPlacement`] at the boundary; an app that
// keeps its placements elsewhere (a registry, a configuration of its own) implements [`PlacementSink`] instead.

#[cfg(feature = "placement")]
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[cfg(feature = "placement")]
use serde::{Deserialize, Serialize};

/// One [`WindowPlacement`] as the JSON file holds it: `Pose` and `Vec2` come from the maths of this crate and are not
/// `Serialize`, so their components are stored in plain floats and converted at the boundary (see
/// [`WindowPlacement::from_components`] and the public fields of [`WindowPlacement`]).
#[cfg(feature = "placement")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
struct StoredPlacement {
    /// World-space position of the window, in metres.
    #[serde(default)]
    pos: [f32; 3],
    /// World-space orientation quaternion `(x, y, z, w)`, identity by default.
    #[serde(default = "identity_quat")]
    quat: [f32; 4],
    /// The base [`WindowPlacement::window_size`]: written since the size is memorized, absent — and then left alone — in
    /// a file written before, or by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_size: Option<[f32; 2]>,
    /// The ui scale of the window ([`WindowPlacement::ui_scale`]), `1.0` by default.
    #[serde(default = "unit_scale")]
    ui_scale: f32,
}

/// The quaternion of a file that holds none: the window faces where its default pose put it.
#[cfg(feature = "placement")]
fn identity_quat() -> [f32; 4] {
    [Quat::IDENTITY.x, Quat::IDENTITY.y, Quat::IDENTITY.z, Quat::IDENTITY.w]
}

/// The ui scale of a file that holds none: no scaling, which is the scale an [`Appearence`] starts with.
#[cfg(feature = "placement")]
fn unit_scale() -> f32 {
    1.0
}

#[cfg(feature = "placement")]
impl From<WindowPlacement> for StoredPlacement {
    /// Writes a placement as the file holds it.
    fn from(placement: WindowPlacement) -> Self {
        Self {
            pos: [placement.pose.position.x, placement.pose.position.y, placement.pose.position.z],
            quat: [
                placement.pose.orientation.x,
                placement.pose.orientation.y,
                placement.pose.orientation.z,
                placement.pose.orientation.w,
            ],
            window_size: placement.window_size.map(|size| [size.x, size.y]),
            ui_scale: placement.ui_scale,
        }
    }
}

#[cfg(feature = "placement")]
impl From<StoredPlacement> for WindowPlacement {
    /// Reads a placement as a window takes it (the broken orientations are repaired, see
    /// [`WindowPlacement::from_components`]).
    fn from(stored: StoredPlacement) -> Self {
        Self::from_components(stored.pos, stored.quat, stored.window_size, stored.ui_scale)
    }
}

/// Writes `content` to `path` **atomically**: it goes to a temporary neighbour of `path`, then is renamed onto it. An
/// interrupted write therefore leaves the previous content (or none) instead of a truncated file, and a reader never
/// sees half of the content. The parent directory is created when it is missing.
#[cfg(feature = "placement")]
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content)?;
    fs::rename(temporary, path)
}

/// A [`PlacementSink`] keeping every placement in one JSON map file — the store a typical app wants (see
/// [`set_json_file`]). The file is written atomically (see `write_atomic`) and read again before every write, so
/// several writers of one file — the app and, in a hot reloading session, the plugin it loads — never drop each other's
/// entries.
#[cfg(feature = "placement")]
pub struct JsonPlacementStore {
    /// The placements, as the file held them when it was read and as they were updated since.
    store: Mutex<HashMap<String, StoredPlacement>>,
    /// The file the store is read from and written to.
    path: PathBuf,
}

#[cfg(feature = "placement")]
impl JsonPlacementStore {
    /// The store of the JSON map file at `path`, which needs not exist yet: the first run of an app has no placement to
    /// give back.
    /// * `path` - The file the placements are read from and written to.
    pub fn load_from(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self { store: Mutex::new(read_map(&path)), path }
    }

    /// The file the placements are read from and written to.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(feature = "placement")]
impl PlacementSink for JsonPlacementStore {
    fn load(&self, key: &str) -> Option<WindowPlacement> {
        self.store.lock().ok()?.get(key).copied().map(WindowPlacement::from)
    }

    fn save(&self, key: &str, placement: WindowPlacement) {
        let Ok(mut store) = self.store.lock() else {
            return;
        };
        // The file is read again before the write: another writer of the same file must not have the entries it
        // memorized since this store was loaded dropped by this write.
        *store = read_map(&self.path);
        store.insert(key.to_string(), placement.into());
        let Ok(json) = serde_json::to_string_pretty(&*store) else {
            Log::err("placement store: a placement can not be serialized");
            return;
        };
        if let Err(error) = write_atomic(&self.path, &json) {
            Log::err(format!("placement store: can not write {}: {error}", self.path.display()));
        }
    }
}

/// The placements held by the JSON map file at `path`. A file that is not there — the first run of an app — is not a
/// failure: there is no placement to give back. A file that can not be parsed (a hand edit, a file of another version)
/// is the same story, and the next write replaces it.
#[cfg(feature = "placement")]
fn read_map(path: &Path) -> HashMap<String, StoredPlacement> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Installs the JSON map file at `path` as the store of the placements of the session, exactly like [`set_sink`] does
/// with a store of your own. One call at start-up, and every window carrying the [`Appearence::placement`] option comes
/// back where — at the size and at the scale — it was left.
/// * `path` - The file the placements are read from and written to.
#[cfg(feature = "placement")]
pub fn set_json_file(path: impl Into<PathBuf>) {
    set_sink(Arc::new(JsonPlacementStore::load_from(path)));
}

#[cfg(all(test, feature = "placement"))]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    /// How many placements the sink of the test was handed, and which ones: the sink is installed globally (the first
    /// one wins), so the test reads what it receives through statics instead of an instance.
    static SAVES: AtomicUsize = AtomicUsize::new(0);
    static SAVED: Mutex<Vec<(String, WindowPlacement)>> = Mutex::new(Vec::new());

    /// A sink counting what it is handed, so the test watches the change detection and the throttle without any file.
    struct CountingSink;

    impl PlacementSink for CountingSink {
        fn load(&self, _key: &str) -> Option<WindowPlacement> {
            None
        }

        fn save(&self, key: &str, placement: WindowPlacement) {
            SAVES.fetch_add(1, Ordering::Relaxed);
            SAVED.lock().unwrap_or_else(|error| error.into_inner()).push((key.to_string(), placement));
        }
    }

    /// A placement of the test, differing from the others by its size (no `Appearence` is built: that needs a session).
    fn placement_of(width: f32) -> WindowPlacement {
        WindowPlacement { pose: Pose::IDENTITY, window_size: Some(Vec2::new(width, 0.0)), ui_scale: 1.5 }
    }

    /// The option hands the sink what changed and nothing else: not the placement it just restored (the window is
    /// exactly where the store left it), not twice the same one, and not the one of a window without a key. The
    /// throttle holds the changes of a same second back, and the flush of a close writes the last gesture anyway.
    #[test]
    fn a_placement_is_handed_to_the_sink_only_when_it_changed() {
        set_sink(Arc::new(CountingSink));

        let mut placement = Placement::new("a_test_key");
        assert_eq!(placement.key(), "a_test_key");
        placement.ensure_key("another_key");
        assert_eq!(placement.key(), "a_test_key", "the key given by the app is not overwritten");

        let mut late = Placement::default();
        assert!(late.key().is_empty());
        late.ensure_key("a_late_key");
        assert_eq!(late.key(), "a_late_key", "an option with no key takes the one the window gives it");

        // What the store knows — what a `restore` just read back — is not written again.
        let restored = placement_of(0.3);
        placement.know(restored);
        placement.offer(restored, false);
        assert_eq!(SAVES.load(Ordering::Relaxed), 0, "a placement the store already holds is not written back");

        // A change is written at once: nothing was written yet in this session.
        let moved = placement_of(0.5);
        placement.offer(moved, false);
        assert_eq!(SAVES.load(Ordering::Relaxed), 1, "the first change is due straight away");

        // The throttle holds the following ones back...
        placement.offer(placement_of(0.6), false);
        assert_eq!(SAVES.load(Ordering::Relaxed), 1, "not twice in the same second");

        // ...and the flush of a close writes the last gesture anyway.
        let scaled = WindowPlacement { ui_scale: 1.8, ..placement_of(0.6) };
        placement.offer(scaled, true);
        assert_eq!(SAVES.load(Ordering::Relaxed), 2, "a flush ignores the throttle");

        let saved = SAVED.lock().unwrap_or_else(|error| error.into_inner());
        assert_eq!(saved.len(), 2, "the sink was handed exactly the two placements that differed from the store");
        assert_eq!(saved[0], ("a_test_key".to_string(), moved));
        assert_eq!(saved[1], ("a_test_key".to_string(), scaled));

        // A window without a key is memorized nowhere, whatever it does.
        let mut keyless = Placement::default();
        keyless.offer(placement_of(0.7), true);
        assert_eq!(SAVES.load(Ordering::Relaxed), 2, "a window with no key is not handed to the sink");
    }

    /// Two orientations are the same one within the rounding of a normalization: a store only holds the four components
    /// of the quaternion, and reading them repairs it again (see [`WindowPlacement::from_components`]).
    fn same_orientation(left: Quat, right: Quat) -> bool {
        (left.x - right.x).abs() < 1e-6
            && (left.y - right.y).abs() < 1e-6
            && (left.z - right.z).abs() < 1e-6
            && (left.w - right.w).abs() < 1e-6
    }

    /// A quaternion of zeros (a truncated or hand-written store) must not be used as is: an orientation of length zero
    /// can not be turned into a rotation, and the window would be drawn in an unusable direction, or not at all. One
    /// of another length is normalized, since it would scale the window instead of turning it.
    #[test]
    fn a_degenerate_orientation_falls_back_on_the_identity() {
        let broken = WindowPlacement::from_components([0.0; 3], [0.0; 4], None, 1.0);
        assert!(same_orientation(broken.pose.orientation, Quat::IDENTITY));

        // A quaternion of length two is the same rotation as its normalized form: the window must not be scaled by it.
        let rounded = WindowPlacement::from_components([0.0; 3], [0.0, 0.0, 0.0, 2.0], None, 1.0);
        assert!(same_orientation(rounded.pose.orientation, Quat::IDENTITY));
    }

    /// The JSON store gives back what a window was left at — its pose, its size, its ui scale — and writes the entries
    /// of the writers of the same file as its own (see [`JsonPlacementStore`]: the file is read again before every
    /// write, so an app and the plugin a hot reloading session loads never drop each other's placements).
    #[cfg(feature = "placement")]
    #[test]
    fn a_json_store_round_trips_a_placement_and_keeps_the_entries_of_another_writer() {
        let dir = std::env::temp_dir().join(format!("sk-placement-{}-json", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("window_placements.json");

        let store = JsonPlacementStore::load_from(&path);
        assert_eq!(store.path(), path.as_path());
        assert!(store.load("a_window").is_none(), "the first run of an app has no placement to give back");

        // What a window was left at comes back as it was.
        let placement =
            WindowPlacement::from_components([-0.7, 1.5, -0.5], [0.0, 0.383, 0.0, 0.924], Some([0.95, 0.0]), 1.35);
        store.save("a_window", placement);
        let read = store.load("a_window").expect("written");
        assert_eq!(read.window_size, Some(Vec2::new(0.95, 0.0)), "the width, and the height left unmanaged, as is");
        assert_eq!(read.ui_scale, 1.35);
        assert_eq!(read.pose.position.x, -0.7);
        assert_eq!(read.pose.position.y, 1.5);
        assert_eq!(read.pose.position.z, -0.5);
        assert!(same_orientation(read.pose.orientation, placement.pose.orientation));

        // A window never resized keeps the size its stepper gave it: the key is not written at all.
        let compact = serde_json::to_string(&StoredPlacement::from(WindowPlacement::default())).expect("serializable");
        assert!(!compact.contains("window_size"), "a window without a size writes no key: {compact}");
        store.save("another_window", WindowPlacement::default());

        // Another writer of the same file keeps its entries...
        let other = JsonPlacementStore::load_from(&path);
        other.save("another_writer", WindowPlacement::default());

        // ...and so does this one, whose next write is not lost either.
        store.save("a_window", WindowPlacement::from_components([0.0; 3], [0.0; 4], None, 1.0));
        let fresh = JsonPlacementStore::load_from(&path);
        assert!(fresh.load("a_window").is_some(), "the entry of the first writer is still in the file");
        assert!(fresh.load("another_writer").is_some(), "the entry of the other writer survived the write");
        assert!(fresh.load("another_window").is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
