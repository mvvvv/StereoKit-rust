use crate::{
    font::Font,
    framework::Appearence,
    maths::{Matrix, Pose, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Align, LogItem, LogLevel, Pivot, Text, TextBuilder, TextFit, TextStyle},
    tools::ui_widgets::Scrollbar,
    ui::Ui,
    util::Color128,
};
use std::sync::Mutex;

pub const SHOW_LOG_WINDOW: &str = "Tool_ShowLogWindow";

/// A simple log window to display the logs.
/// ### Fields that can be changed before initialization:
/// * `log_log` - The log mutex to listen to.
/// * `enabled` - Whether the tool is enabled or not at start.
/// * `window_pose` - The pose where to show the log window.
/// * `appearence` - The look of the window: size, scaling and text styles, see [`Appearence`]. The default
///   `window_size` is a wide, low log panel (`0.8 x 0.3` meters, resizable down to `0.25 x 0.12`); set
///   `appearence.window_size` before start for another size. The log lines are drawn with
///   [`Appearence::list_style`] ONLY, the levels are told apart by color, through the tints below.
/// * `tint_diag` / `tint_info` / `tint_warn` / `tint_err` - The tints applied over [`Appearence::list_style`] to
///   the lines of each log level, see [`LogWindow::level_tint`].
///
/// The log area itself is fully managed at draw time: a vertical [`Scrollbar`] scrolls the lines (whole lines,
/// like the old `vslider`), and a horizontal [`Scrollbar`] scrolls the lines that are too long to fit — they are
/// clipped, not wrapped. In simulation, the plain mouse wheel (or the controller stick Y in XR) scrolls the
/// lines, and Shift+wheel (or the stick X) scrolls their length, see [`Scrollbar::apply_input_scroll`].
///
/// ### Events this stepper is listening to:
/// * `SHOW_LOG_WINDOW` - Event that triggers when the window is visible ("true") or hidden ("false").
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ui::Ui, maths::Vec2, system::{LogLevel, LogItem,  Log}, sprite::Sprite,
///                      tools::log_window::{LogWindow, basic_log_fmt, SHOW_LOG_WINDOW}};
/// use std::sync::Mutex;
///
/// // Somewhere to copy the log
/// static LOG_LOG: Mutex<Vec<LogItem>> = Mutex::new(vec![]);
/// let fn_mut = |level: LogLevel, log_text: &str| {
///    let items = LOG_LOG.lock().expect("Failed to lock log mutex");
///    basic_log_fmt(level, log_text, items);
/// };
/// Log::subscribe(fn_mut);
/// let mut log_window = LogWindow::new(&LOG_LOG);
/// log_window.window_pose = Ui::popup_pose([0.0, 0.10, 1.39]);
/// log_window.appearence.window_size = Vec2 { x: 0.25, y: 0.25 };
/// log_window.appearence.handle_sprite = Some(Sprite::from_file(
///              "icons/log_viewer.png", None, None).expect("Icon should be loaded"));
///
/// sk.send_event(StepperAction::add("LogWindow", log_window));
///
/// filename_scr = "screenshots/log_window.jpeg"; fov_scr = 110.0;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     if iter == 0  {
///         for i in 0..40 {
///             Log::info(format!("Info log message {i}"));
///         }
///         Log::info("Repeated log message"); // the same line twice: shown as "Repeated log message x2"
///         Log::info("Repeated log message");
///         Log::warn("Warning log message");
///         Log::err ("Error log message");
///         Log::err ("A rather long error message that does not fit the window width and needs the horizontal scrollbar");
///     } else  if iter == number_of_steps  {
///        sk.send_event(StepperAction::event( "main", SHOW_LOG_WINDOW, "false",));
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/log_window.jpeg" alt="screenshot" width="200">
#[derive(IStepper)]
pub struct LogWindow<'a> {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub enabled: bool,

    pub window_pose: Pose,
    /// The look of the window: size, scaling, text styles and tints, see [`Appearence`]. The default
    /// `window_size` is a wide, low log panel (`0.8 x 0.3` meters) that can shrink well below the file
    /// browser's minimum (`min_window_size` of `0.25 x 0.12`); the number of visible lines adapts to the
    /// actual window size at draw time.
    pub appearence: Appearence,

    /// Tint of the diagnostic lines over [`Appearence::list_style`], see [`LogWindow::level_tint`].
    pub tint_diag: Color128,
    /// Tint of the info lines over [`Appearence::list_style`], see [`LogWindow::level_tint`].
    pub tint_info: Color128,
    /// Tint of the warning lines over [`Appearence::list_style`], see [`LogWindow::level_tint`].
    pub tint_warn: Color128,
    /// Tint of the error lines over [`Appearence::list_style`], see [`LogWindow::level_tint`].
    pub tint_err: Color128,

    pub log_log: &'a Mutex<Vec<LogItem>>,
    /// The vertical scrollbar of the log lines, see [`Scrollbar`].
    scrollbar: Scrollbar,
    /// The horizontal scrollbar of the too-long lines, see [`Scrollbar`].
    h_scrollbar: Scrollbar,
    /// Number of items in `log_log` at the last draw: a change means new logs arrived (or the log was cleared),
    /// so the list scrolls back to the newest lines and the longest line is re-measured.
    items_size: usize,
    /// Length in characters of the longest display line (text + repeat suffix, see [`LogWindow::item_display`]):
    /// the content width the horizontal scrollbar scrolls in. Re-measured when `items_size` changes.
    max_line_chars: usize,
}

unsafe impl Send for LogWindow<'_> {}

impl<'a> LogWindow<'a> {
    /// Design-width of the space a log line is counted in, see [`LogWindow::item_chars`]: the horizontal
    /// scrollbar scrolls the lines by full characters, so the width of ONE character has to be known.
    /// It is MEASURED at draw time on a sample of [`LogWindow::CHAR_WIDTH_SAMPLE`] with the actual
    /// [`Appearence::list_style`], see [`LogWindow::measure_char_width`]: for the monospace (fixed-width)
    /// fonts the log uses by default, every glyph advances the same, so the measured value IS the cell
    /// width and the scrollbar lines up with the drawn lines exactly. Only when the style can't be
    /// measured yet (e.g. its font asset is still being loaded) does the fallback ratio below apply.
    const CHAR_WIDTH_RATIO: f32 = 0.6;
    /// Characters whose advance width is averaged to measure the cell width of the list style,
    /// see [`LogWindow::measure_char_width`].
    const CHAR_WIDTH_SAMPLE: &'static str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz ,.:;!?()";
    /// Vertical spacing between two consecutive log lines, in multiples of the line height of
    /// [`Appearence::list_style`]: 1.0 packs the lines with no leading at all (wall-of-text, hard to read),
    /// 2.0 gives a slight terminal-like separation without ever looking like a blank line between entries.
    const LINE_SPACING: f32 = 2.0;

    pub fn new(log_log: &'a Mutex<Vec<LogItem>>) -> Self {
        let enabled = true;
        let pose = Pose::IDENTITY;

        // A log panel is wide rather than tall, and can shrink far below the file browser's minimum: the
        // number of visible lines adapts to the actual window size at draw time anyway.
        let font_mono = if cfg!(target_os = "android") {
            ["/system/fonts/CutiveMono.ttf", "/system/fonts/DroidSansMono.ttf"]
                .iter()
                .find_map(|path| Font::from_file(path).ok())
                .ok_or(())
                .unwrap_or_default()
        } else {
            Font::from_family("Consolas, Monaco, Courier New, monospace").unwrap_or_default()
        };
        let mut appearence = Appearence::default();
        appearence.window_size = Vec2::new(0.8, 0.3);
        appearence.min_window_size = Vec2::new(0.25, 0.25);
        appearence.handle_sprite = Some(Sprite::from_file("icons/log_viewer.png", None, None).unwrap_or_default());
        appearence.list_style = TextStyle::from_font(font_mono, 0.01, Color128::WHITE);
        Self {
            id: "LogWindow".to_string(),
            sk_info: None,
            enabled,

            window_pose: pose,
            appearence,

            // The levels share the single Appearence::list_style font, they are told apart by color only.
            tint_diag: Color128::hsv(1.0, 0.0, 0.3, 1.0),
            tint_info: Color128::hsv(1.0, 0.0, 1.0, 1.0),
            tint_warn: Color128::hsv(0.17, 0.7, 1.0, 1.0),
            tint_err: Color128::hsv(1.0, 0.7, 0.7, 1.0),

            log_log,
            scrollbar: Scrollbar::vertical(),
            h_scrollbar: Scrollbar::horizontal(),
            items_size: 0,
            max_line_chars: 0,
        }
    }

    /// The tint applied over [`Appearence::list_style`] to the lines of the given `level`: the log uses a
    /// single font, the levels are told apart by color only. `Inform` and `None` (unset) read the same.
    pub fn level_tint(&self, level: LogLevel) -> Color128 {
        match level {
            LogLevel::Diagnostic => self.tint_diag,
            LogLevel::Warning => self.tint_warn,
            LogLevel::Error => self.tint_err,
            LogLevel::Inform | LogLevel::None => self.tint_info,
        }
    }

    /// The line displayed for `item`: its trimmed text, with a `×count` suffix when the same line repeated.
    fn item_display(item: &LogItem) -> String {
        let text = item.text.trim();
        if item.count > 1 { format!("{text} ×{}", item.count) } else { text.to_owned() }
    }

    /// Number of characters of the line displayed for `item`, see [`LogWindow::item_display`].
    fn item_chars(item: &LogItem) -> usize {
        let mut chars = item.text.trim().chars().count();
        if item.count > 1 {
            chars += format!(" ×{}", item.count).chars().count();
        }
        chars
    }

    /// Average advance, in meters, of ONE character of `style` as it is really drawn: the layout width of
    /// [`LogWindow::CHAR_WIDTH_SAMPLE`] measured with [`Text::size_layout`], divided by the sample length.
    ///
    /// For a monospace (fixed-width) font — the default log font family — every glyph advances exactly
    /// this much, so one horizontal scroll cell shifts the drawn lines by one real character: the
    /// horizontal [`Scrollbar`] then appears exactly when the longest line stops fitting the content
    /// (`max_line_chars > visible_cols`), and scrolling it to its end brings the WHOLE line into view —
    /// the line end is never left clipped out of reach against the vertical scrollbar strip. For a
    /// proportional font this is the average advance of the sample, a fair estimate to count lines in
    /// whole `item_chars` as before.
    ///
    /// Falls back on [`LogWindow::CHAR_WIDTH_RATIO`] while the style cannot be measured yet (its font
    /// asset is still loading, so the sample measures 0).
    fn measure_char_width(style: TextStyle) -> f32 {
        let size = Text::size_layout(Self::CHAR_WIDTH_SAMPLE, Some(style), None);
        if size.x > 0.0 {
            size.x / Self::CHAR_WIDTH_SAMPLE.chars().count() as f32
        } else {
            style.get_layout_height() * Self::CHAR_WIDTH_RATIO
        }
    }

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // Captures the current (possibly user-tweaked) text heights as the base the ui scale multiplies,
        // and applies the current scale, see `Appearence::start`.
        self.appearence.start();
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(SHOW_LOG_WINDOW) {
            self.enabled = value.parse().unwrap_or(false)
        }
    }
    /// Called from IStepper::step, after check_event here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        // The window is drawn with the Appearence-scaled UiSettings, restored exactly as they were afterwards.
        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());
        Ui::push_id(&self.id);

        Ui::window("Log viewer")
            .pose(&mut self.window_pose)
            .size(self.appearence.scaled_window_size())
            .begin();

        // Useful to know if the window is focused, so the wheel / thumbsticks can scroll the logs.
        let window_focused = Ui::get_last_element_focused().is_active();
        self.draw_logs(window_focused);
        Ui::window_end();

        Ui::pop_id();
        Ui::settings(prev_settings);

        // Grab-able knob anchored to the window: dragging it along the window local X resizes the width,
        // along Y the height, and along Z (towards the user) the whole scale, see `Appearence::scale_handle`.
        self.appearence.scale_handle(&self.window_pose, "log_window_scale");
    }

    /// The scrollable log list: a vertical [`Scrollbar`] for the lines, a horizontal one for the too-long
    /// lines (clipped, not wrapped), the lines themselves drawn with [`Appearence::list_style`] ONLY and
    /// tinted per level with [`LogWindow::level_tint`].
    fn draw_logs(&mut self, window_focused: bool) {
        let settings = self.appearence.get_ui_settings_scaled();

        // The log lines use the single list style of the Appearence, like the file browser's list.
        Ui::push_text_style(self.appearence.list_style);
        // Two line measures: `Ui::get_line_height` is the height of a full UI ROW (text baseline + 2 paddings,
        // what a button reserves); the log lines themselves step by the text style's OWN line height with a
        // slight leading (`LINE_SPACING`), for a terminal-like density that stays readable.
        let ui_line = Ui::get_line_height();
        let line = self.appearence.list_style.get_layout_height();
        let row_h = line * Self::LINE_SPACING;

        // The whole log area
        let remaining = Ui::get_layout_remaining();
        let area_w = remaining.x;
        let area_h = remaining.y.max(row_h * 2.0);

        // Reserve the whole area as one block of the window flow, and keep its top-LEFT corner (`tlb`): the
        // lines and the scrollbars are drawn at absolute positions inside it. Mind the SK layout axes when
        // offsetting from it: X+ points LEFT and Y+ points UP ("Left (X+), Top (Y+)", see `Bounds::tlb`),
        // so the RIGHT edge of the area is at `at.x - area_w`, its BOTTOM edge at `at.y - area_h`.
        let bounds = Ui::layout_reserve(Vec2::new(area_w, area_h), false, 0.0);
        let at = bounds.tlb();

        let items = self.log_log.lock().expect("Failed to lock log_log");

        // New logs (or a cleared log): re-measure the longest line, and remember to follow the newest ones.
        let mut follow_newest = false;
        if self.items_size != items.len() {
            self.items_size = items.len();
            self.max_line_chars = items.iter().map(Self::item_chars).max().unwrap_or(0);
            follow_newest = true;
        }

        // Geometry: the vertical scrollbar strip is always reserved on the right (drawn only when needed,
        // like the file browser's list); the horizontal one takes a bottom strip when the longest line
        // overflows the content width. The character cell is the MEASURED advance of the actual list
        // style, not a guess: in the monospace log font every glyph advances the same, so `visible_cols`
        // and the horizontal scroll match the drawn lines one-for-one.
        let bar_w = ui_line * 0.7;
        let content_w = (area_w - bar_w).max(line * 2.0);
        let char_w = Self::measure_char_width(self.appearence.list_style);
        let visible_cols = self.h_scrollbar.visible_cells_count(content_w, char_w, 0.0, 0);
        let h_bar_shown = self.max_line_chars > visible_cols;
        let bar_h = bar_w;
        let content_h = if h_bar_shown { (area_h - bar_h).max(line) } else { area_h };

        let total_rows = items.len();
        let visible_rows = self.scrollbar.visible_cells_count(content_h, row_h, 0.0, 0);
        let max_scroll = total_rows.saturating_sub(visible_rows) as f32;
        let max_h_scroll = self.max_line_chars.saturating_sub(visible_cols) as f32;

        if follow_newest {
            self.scrollbar.scroll = max_scroll;
        }
        self.scrollbar.clamp_scroll(max_scroll);
        self.h_scrollbar.clamp_scroll(max_h_scroll);

        // The vertical scrollbar, spanning the whole area height on its RIGHT strip: `layout_push` takes the
        // top-LEFT corner of the new region — the max X, since X+ points LEFT — so the strip is anchored at
        // `at.x - content_w`, the boundary between the content and the strip: its box then spans exactly
        // [at.x - area_w, at.x - content_w], inside the area, flush with its right edge.
        let v_bar_shown = total_rows > visible_rows;
        if v_bar_shown {
            Ui::layout_push(Vec3::new(at.x - content_w, at.y, at.z), Vec2::new(bar_w, area_h), false);
            self.scrollbar.draw_scrollbar(
                "log_scroll_v",
                bar_w,
                area_h,
                max_scroll,
                visible_rows,
                total_rows,
                &settings,
            );
            Ui::layout_pop();
        }

        // The horizontal scrollbar, at the bottom of the area, stopping left of the vertical strip when
        // both are shown.
        if h_bar_shown {
            let h_bar_w = if v_bar_shown { content_w } else { area_w };
            Ui::layout_push(Vec3::new(at.x, at.y - content_h, at.z), Vec2::new(h_bar_w, bar_h), false);
            self.h_scrollbar.draw_scrollbar(
                "log_scroll_h",
                h_bar_w,
                bar_h,
                max_h_scroll,
                visible_cols,
                self.max_line_chars,
                &settings,
            );
            Ui::layout_pop();
        }

        // The visible lines: ONE single font (the Appearence list style), the level told apart by its TINT
        // only (`TextBuilder::tint` multiplies the style color — `Ui::push_tint` does NOT apply to text).
        // Too-long lines are clipped (they scroll horizontally), never wrapped: one LogItem always stays on
        // one single line.
        let start_row = self.scrollbar.scroll as usize;
        let x_shift = self.h_scrollbar.scroll * char_w;
        for row in 0..visible_rows {
            let Some(item) = items.get(start_row + row) else { break };
            let y = at.y - row as f32 * row_h;
            TextBuilder::new(Self::item_display(item))
                // The clip box stays FIXED on the content area: the horizontal scroll is a text OFFSET
                // inside that box (`TextBuilder::offset` slides the glyphs within the clip bounds,
                // `text_add_in_g` in C), NOT a transform shift that would drag the clip along and reveal
                // nothing. Positive off_x slides the glyphs towards X+ = left on screen, uncovering the end
                // of the line.
                .transform(Matrix::t(Vec3::new(at.x, y, at.z - 0.004)))
                .size([content_w, line * Self::LINE_SPACING])
                .fit(TextFit::Clip)
                .style(self.appearence.list_style)
                .tint(self.level_tint(item.level))
                .position(Pivot::TopLeft)
                .align(Align::CenterLeft)
                .offset(x_shift, 0.0, 0.0)
                .add();
        }
        Ui::pop_text_style();

        // Peripheral scrolling: the plain wheel (or the stick Y in XR) scrolls the lines, Shift+wheel (or
        // the stick X) scrolls their length, see `Scrollbar::apply_input_scroll`.
        self.scrollbar.apply_input_scroll(max_scroll, window_focused);
        self.h_scrollbar.apply_input_scroll(max_h_scroll, window_focused);
    }
}

/// A basic log formatter that splits long lines and counts repeated lines.
/// * `level` - The log level.
/// * `log_text` - The log text.
pub fn basic_log_fmt(level: LogLevel, log_text: &str, mut items: std::sync::MutexGuard<'_, Vec<LogItem>>) {
    for line_text in log_text.lines() {
        if let Some(item) = items.last_mut()
            && item.text == line_text
        {
            item.count += 1;
            continue;
        }

        items.push(LogItem { level, text: line_text.to_owned(), count: 1 });
    }
}
