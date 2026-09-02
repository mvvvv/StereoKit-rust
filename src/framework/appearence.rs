use crate::{
    font::Font,
    maths::{Bounds, Matrix, Pose, Vec2, Vec3},
    sprite::Sprite,
    system::{Pivot, Text, TextBuilder, TextStyle},
    ui::{Ui, UiSettings},
    util::{Color128, named_colors},
};

/// The look & feel of a UI window: size, scaling, text styles and extra tints.
/// - the ui scale (read with [`Appearence::get_ui_scale`], set with [`Appearence::start`] or
///   [`Appearence::set_ui_scale`]) uniformly scales the whole window: its size, every [`UiSettings`] value and
///   the `layout_height` of the four text styles,
/// - [`Appearence::scale_handle`] draws the grab-able knob that interactively drives both
///   [`Appearence::window_size`] (local X = width, local Y = height) and the ui scale (local Z),
///   and [`Appearence::handle_sprite`], when set, replaces the built-in knob visual with a custom sprite,
/// - [`Appearence::keep_window_ratio`] locks the `window_size` aspect ratio while resizing with the handle,
/// - the four text styles, from the biggest ([`Appearence::title_style`]) to the smallest
///   ([`Appearence::small_style`]), give the UI some relief,
/// - the three tints color directory buttons, input fields and error entries,
/// - [`Appearence::double_click_delay`] is the maximum delay between the two presses of a "double-click".
///
/// Call [`Appearence::start`] once when the window stepper starts (it captures the base text heights and applies the
/// current scale), then [`Appearence::scale_handle`] every frame after the window itself has been drawn.
pub struct Appearence {
    /// The `window_size` set by [`Appearence::default`], and the reference size the released scale-handle
    /// anchor is proportional to, see [`Appearence::scale_handle`]. Default is `Vec2::new(0.6, 0.8)`.
    pub window_size: Vec2,
    /// The reference size of the window used to apply scale and position of the [`Appearence::scale_handle`] this is
    /// always Vec2::new(0.6, 0.8),
    reference_window_size: Vec2,
    /// Interactive resize floor for [`Appearence::window_size`] (meters) applied while dragging the scale
    /// handle, see [`Appearence::scale_handle`]. Default is `Vec2::new(0.45, 0.45)`.
    pub min_window_size: Vec2,
    /// When `true`, the interactive resize keeps the `window_size` aspect ratio. Default is `false` (free X / Y resize).
    pub keep_window_ratio: bool,

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
    /// see [`Appearence::scale_handle`]. Default is `Vec3::new(0.30, 0.035, 0.006)` you can change it as long as it's
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

    /// Optional custom visual for the scale handle: when `Some`, [`Appearence::scale_handle`] no longer draws
    /// the built-in knob and draws this sprite instead, centered on the knob and scaled to its footprint
    /// (`0.035 * ui_scale` meters on its largest axis, aspect ratio preserved). The grab volume and the drag
    /// behavior are unchanged. Default is `None` (built-in knob).
    pub handle_sprite: Option<Sprite>,

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
}

impl Default for Appearence {
    fn default() -> Self {
        // Font shared by the four text styles below.
        let font = Font::default();
        Self {
            reference_window_size: Vec2::new(0.6, 0.8),
            min_window_size: Vec2::new(0.45, 0.45),
            keep_window_ratio: false,
            window_size: Vec2::new(0.6, 0.8),
            ui_settings: Ui::get_settings(),
            ui_scale: 1.0,
            scale_per_meter: 2.0,
            // Scale handle at its default anchor.
            scale_handle_default_offset: Vec3::new(0.30, 0.035, 0.006),
            scale_handle_offset: Vec3::ZERO,
            scale_grab: None,
            handle_sprite: None,
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

            ui_settings_scaled: Ui::get_settings(),
        }
    }
}

impl Appearence {
    /// To be called once when the window stepper starts. Clamp the properties set before launch.
    pub fn start(&mut self) {
        // Bounds checks of the properties settable before launch, so no zero / negative / out-of-range
        // value can break the draw loop or the scale-handle interactions.
        const ABS_MIN_WINDOW: Vec2 = Vec2::new(0.01, 0.01); // hard floor for the resize floor itself
        self.min_window_size = Vec2::max(self.min_window_size, ABS_MIN_WINDOW);
        if self.keep_window_ratio {
            // Ratio locked: lift `window_size` to the floor with one uniform factor, so the aspect ratio
            // survives the `min_window_size` clamp as well.
            let factor = (self.min_window_size.x / self.window_size.x.max(0.001))
                .max(self.min_window_size.y / self.window_size.y.max(0.001))
                .max(1.0);
            self.window_size *= factor;
        } else {
            self.window_size = Vec2::max(self.window_size, self.min_window_size);
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
    /// follows the window when it moves. While held, each drag axis drives its own appearance property:
    /// - the local X axis (the window width direction) modifies [`Appearence::window_size`].x,
    /// - the local Y axis (the window height direction) modifies [`Appearence::window_size`].y,
    /// - the local Z axis (towards / away from the user) modifies the ui scale (see [`Appearence::get_ui_scale`]),
    ///   which uniformly scales the whole window, see the internal `scale_all`.
    ///
    /// When [`Appearence::keep_window_ratio`] is `true`, the local X and Y drag deltas are merged into a single
    /// uniform size factor instead, so the window keeps its aspect ratio while resizing.
    ///
    /// On release, the knob springs back to its default anchor, scaled proportionally to the current drawn
    /// window size (`window_size * ui_scale`) so it keeps hugging the window edge. While the handle is
    /// grabbed, three small labels around the knob show the live values it drives: the scale factor in
    /// percent in front of the knob (towards the user), the window width below it and the window height
    /// on its right.
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
        // A custom handle sprite replaces the built-in knob visual, see the drawing after the grab logic.
        let draw_default_handle = self.handle_sprite.is_none();
        let grabbed =
            Ui::handle(id, &mut handle_pose, Bounds::bounds_centered(Vec3::new(1.0, 1.0, 0.3) * 0.035 * self.ui_scale))
                .draw_handle(draw_default_handle)
                .grab();
        let result = if grabbed {
            let delta = handle_pose.position - window_pose.position;
            let offset = Vec3::new(Vec3::dot(delta, right), Vec3::dot(delta, up), Vec3::dot(delta, forward));
            // Drag session start state: the handle offset, ui_scale and window_size at grab time, so each axis below
            // is applied as a delta from them.
            let (start_offset, start_scale, start_size) =
                *self.scale_grab.get_or_insert((handle_offset, self.ui_scale, self.window_size));

            if self.keep_window_ratio {
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
                self.window_size.x = (start_size.x + offset.x - start_offset.x).max(self.min_window_size.x);
                self.window_size.y = (start_size.y + offset.y - start_offset.y).max(self.min_window_size.y);
            }
            self.ui_scale = (start_scale + (offset.z - start_offset.z) * self.scale_per_meter)
                .clamp(self.scale_bounds.0, self.scale_bounds.1);
            self.scale_all();
            self.scale_handle_offset = offset;

            // While grabbed, three small labels around the knob show the live values it drives:
            let label_size = Vec2::new(0.05, 0.02) * self.ui_scale;
            let orientation = handle_pose.orientation;
            TextBuilder::new(format!("{:.0}%", self.ui_scale * 100.0))
                .transform(Matrix::t_r(handle_pose.position + forward * 0.05, orientation))
                .style(self.title_style)
                .size(label_size)
                .add();
            TextBuilder::new(format!("{:.2}", self.window_size.x))
                .transform(Matrix::t_r(handle_pose.position - up * 0.03, orientation))
                .style(self.small_style)
                .size(label_size)
                .add();
            TextBuilder::new(format!("{:.2}", self.window_size.y))
                .transform(Matrix::t_r(handle_pose.position + right * 0.04, orientation))
                .style(self.small_style)
                .size(label_size)
                .add();

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

        // Custom handle visual: when set, the sprite is drawn in place of the built-in knob, centered on it
        // and scaled to its footprint (`0.055 * ui_scale` meters on its largest axis, aspect ratio preserved),
        // so it follows both the drag position and the window scaling. Drawn after the grab logic so the pose
        // used is the one updated by the drag of this frame.
        if let Some(sprite) = &self.handle_sprite {
            let size = 0.055 * self.ui_scale;
            let aspect = sprite.get_aspect();
            let scale = size / aspect.max(1.0);
            sprite.draw(handle_pose.to_matrix(Some(Vec3::new(scale, scale * aspect, 1.0))), Pivot::Center, None, None);
        }

        result
    }
}
