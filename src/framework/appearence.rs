use crate::{
    font::Font,
    maths::{Bounds, Matrix, Pose, Vec2, Vec3},
    system::{Text, TextBuilder, TextStyle},
    ui::{Ui, UiSettings},
    util::{Color128, named_colors},
};

/// Lower bound of [`Appearence::ui_scale`], both for a value set before launch (checked in [`Appearence::start`])
/// and while dragging the scale handle, see [`Appearence::scale_handle`].
const MIN_UI_SCALE: f32 = 0.5;

/// Upper bound of [`Appearence::ui_scale`], both for a value set before launch (checked in [`Appearence::start`])
/// and while dragging the scale handle, see [`Appearence::scale_handle`].
const MAX_UI_SCALE: f32 = 4.0;

/// The look & feel of a UI window: size, scaling, text styles and extra tints.
/// - [`Appearence::ui_scale`] uniformly scales the whole window: its size, every [`UiSettings`] value and the
///   `layout_height` of the four text styles,
/// - [`Appearence::scale_handle`] draws the grab-able knob that interactively drives both
///   [`Appearence::window_size`] (local X = width, local Y = height) and [`Appearence::ui_scale`] (local Z),
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

    /// The [`UiSettings`] used to draw the window. It is multiplied by [`Appearence::ui_scale`] to give
    /// [`Appearence::ui_settings_scaled`]
    pub ui_settings: UiSettings,
    ui_settings_scaled: UiSettings,
    /// Scale factor of the whole window UI: the actual window size is `window_size * ui_scale` and every `UiSettings`
    /// value is multiplied by it during the window drawing. Default is 1.0 (no scaling).
    pub ui_scale: f32,
    /// How much [`Appearence::ui_scale`] grows per meter of scale-handle drag along the window-local Z axis
    /// (dragged towards the user = bigger, away from it = smaller). Default is 2.0.
    pub scale_per_meter: f32,
    /// Default window-local offset of the scale handle: on release, the handle springs back here, scaled
    /// proportionally to the current drawn window size (`window_size * ui_scale`) relative to
    /// [`Appearence::reference_window_size`], so it hugs the window edge at its current size,
    /// see [`Appearence::scale_handle`]. Default is `Vec3::new(0.30, 0.035, 0.006)` you can change it as long as it's
    /// relative to [`Appearence::reference_window_size`] and on the right of the window .
    pub scale_handle_default_offset: Vec3,
    /// Current window-local offset of the scale handle, the grab-able knob that drives [`Appearence::ui_scale`]
    /// and [`Appearence::window_size`]: while held, dragging it resizes / scales the window relative to where it
    /// was grabbed. Initialized to [`Appearence::scale_handle_default_offset`], recomputed on each release.
    pub scale_handle_offset: Vec3,
    /// Current scale-grab session: the handle offset, the `ui_scale` and the `window_size` captured when the
    /// handle was grabbed, so each drag axis is applied as a delta from them.
    scale_grab: Option<(Vec3, f32, Vec2)>,

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
            window_size: Vec2::new(0.6, 0.8),
            ui_settings: Ui::get_settings(),
            ui_scale: 1.0,
            scale_per_meter: 2.0,
            // Scale handle at its default anchor.
            scale_handle_default_offset: Vec3::new(0.30, 0.035, 0.006),
            scale_handle_offset: Vec3::new(0.30, 0.035, 0.006),
            scale_grab: None,

            // Four text styles give the window some relief
            title_style: Text::make_style(&font, 0.012, named_colors::WHITE),
            list_style: Text::make_style(&font, 0.010, named_colors::LIGHT_GRAY),
            label_style: Text::make_style(&font, 0.009, named_colors::WHITE),
            small_style: Text::make_style(&font, 0.0075, named_colors::GRAY),
            text_base_heights: [0.012, 0.010, 0.009, 0.0075],
            button_tint: named_colors::DARK_SLATE_GRAY.into(),
            input_tint: named_colors::SADDLE_BROWN.into(),
            error_tint: named_colors::RED.into(),
            double_click_delay: 0.5,

            ui_settings_scaled: Ui::get_settings(),
        }
    }
}

impl Appearence {
    /// To be called once when the window stepper starts: first clamp the properties set before launch to sane
    /// bounds — the interactive resize floor itself is at least 1 cm, `window_size` respect that floor, `ui_scale`
    /// stays within the range the scale handle can drag it to, and `scale_per_meter` stays positive — then capture the
    /// current (possibly user-tweaked) font sizes as the base heights the draw loop multiplies by `ui_scale`, so
    /// scaling never compounds over the frames.
    pub fn start(&mut self) {
        // Bounds checks of the properties settable before launch, so no zero / negative / out-of-range
        // value can break the draw loop or the scale-handle interactions.
        const ABS_MIN_WINDOW: Vec2 = Vec2::new(0.01, 0.01); // hard floor for the resize floor itself
        self.min_window_size = Vec2::max(self.min_window_size, ABS_MIN_WINDOW);
        self.window_size = Vec2::max(self.window_size, self.min_window_size);
        self.ui_scale = self.ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        self.scale_per_meter = self.scale_per_meter.max(0.01);
        self.double_click_delay = self.double_click_delay.max(0.0);

        self.text_base_heights = [
            self.title_style.get_layout_height(),
            self.list_style.get_layout_height(),
            self.label_style.get_layout_height(),
            self.small_style.get_layout_height(),
        ];
        self.scale_all();
    }
    /// Scale the window at start or after. Usefull when you want to have the same look than the calling window or
    /// to adjust lazily your window.
    pub fn set_ui_scale(&mut self, ui_scale: f32) {
        self.ui_scale = ui_scale;
        self.scale_all();
    }

    /// Scale the font sizes with ui_scale as well: `UiSettings` scaling does NOT affect text styles, so each of the
    /// four styles gets its base `layout_height` (captured at [`Appearence::start`]) multiplied by the current scale.
    /// The size hierarchy title > list > label > small gives the UI some relief at every scale.
    pub fn scale_all(&mut self) {
        let [title_h, list_h, label_h, small_h] = self.text_base_heights;
        self.title_style.layout_height(title_h * self.ui_scale);
        self.list_style.layout_height(list_h * self.ui_scale);
        self.label_style.layout_height(label_h * self.ui_scale);
        self.small_style.layout_height(small_h * self.ui_scale);
        self.ui_settings_scaled = self.ui_settings * self.ui_scale;
    }

    /// The [`UiSettings`] actually used to draw the window: [`Appearence::ui_settings`] already multiplied by
    /// [`Appearence::ui_scale`] by [`Appearence::scale_all`]. Push them with [`Ui::settings`] before drawing the
    /// window, and restore the caller's settings afterwards.
    pub fn ui_settings_scaled(&self) -> UiSettings {
        self.ui_settings_scaled
    }

    /// Scale handle: a small grab-able knob in world space, anchored to `window_pose` in its local space so it
    /// follows the window when it moves. While held, each drag axis drives its own appearance property:
    /// - the local X axis (the window width direction) modifies [`Appearence::window_size`].x,
    /// - the local Y axis (the window height direction) modifies [`Appearence::window_size`].y,
    /// - the local Z axis (towards / away from the user) modifies [`Appearence::ui_scale`], which uniformly
    ///   scales the whole window, see [`Appearence::scale_all`].
    ///
    /// On release, the knob springs back to its default anchor, scaled proportionally to the current drawn
    /// window size (`window_size * ui_scale`) so it keeps hugging the window edge. While the handle is
    /// grabbed, three small labels around the knob show the live values it drives: the scale factor in
    /// percent in front of the knob (towards the user), the window width below it and the window height
    /// on its right.
    ///
    /// * `window_pose` - The world-space pose of the window the handle is anchored to.
    /// * `id` - The unique StereoKit UI id of the handle element.
    ///
    /// Returns `Some(ui_scale)` on every frame the handle is grabbed, so the caller can propagate the live
    /// scale to its child windows ([`crate::tools::file_browser_b::FileBrowserB`] forwards it to its preview
    /// panel), and `None` when the handle is not grabbed.
    pub fn scale_handle(&mut self, window_pose: &Pose, id: &str) -> Option<f32> {
        let (right, up, forward) = (window_pose.get_right(), window_pose.get_up(), window_pose.get_forward());

        let handle_offset = self.scale_handle_offset;
        let handle_center =
            window_pose.position + right * handle_offset.x + up * handle_offset.y + forward * handle_offset.z;
        let mut handle_pose = Pose::new(handle_center, Some(window_pose.orientation));
        if Ui::handle(id, &mut handle_pose, Bounds::bounds_centered(Vec3::new(1.0, 1.0, 0.3) * 0.035 * self.ui_scale))
            .draw_handle(true)
            .grab()
        {
            let delta = handle_pose.position - window_pose.position;
            let offset = Vec3::new(Vec3::dot(delta, right), Vec3::dot(delta, up), Vec3::dot(delta, forward));
            // Drag session start state: the handle offset, ui_scale and window_size at grab time, so each axis below
            // is applied as a delta from them.
            let (start_offset, start_scale, start_size) =
                *self.scale_grab.get_or_insert((handle_offset, self.ui_scale, self.window_size));

            self.window_size.x = (start_size.x + offset.x - start_offset.x).max(self.min_window_size.x);
            self.window_size.y = (start_size.y + offset.y - start_offset.y).max(self.min_window_size.y);
            self.ui_scale =
                (start_scale + (offset.z - start_offset.z) * self.scale_per_meter).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
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
        }
    }
}
