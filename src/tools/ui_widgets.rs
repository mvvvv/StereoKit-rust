use crate::{
    maths::{Bounds, Pose, Quat, Vec2, Vec3},
    sprite::Sprite,
    system::{Align, BtnState, Hierarchy, Input, InputButton, InputXY, Key, Text},
    ui::{Ui, UiColor, UiColorState, UiCut, UiDir, UiPad, UiSettings, UiSliderData, UiVisual},
    util::{Device, DisplayType, Time},
};
use std::time::Instant;

/// Wraps `text` into lines of at most `max_chars` columns, preferring to break lines AFTER `/` or `\` path
/// separators so paths stay readable. Only a segment without any separator that is longer than `max_chars` (a too
/// long file name) gets hard-split. The lines are joined with `\n`, so a single [`Ui::text`] call draws the whole
/// note with one uniform `TextFit::Exact` scale — the adjustment stays proportional.
///
/// The result is also padded with blank lines up to a minimum of 3 lines, so the scale of a short annotation note
/// matches the one of a longer note. Use [`wrap_chars_lines`] for the raw wrapped lines WITHOUT that padding (e.g.
/// for button texts, where a trailing blank line would shift the text upward).
pub fn wrap_chars(text: &str, max_chars: usize) -> String {
    let mut lines = wrap_chars_lines(text, max_chars);
    if lines.len() == 1 {
        lines.push(String::from(" "));
        lines.push(String::from(" "));
    } else if lines.len() == 2 {
        lines.push(String::from(" "));
    }
    lines.join("\n")
}

/// The core of [`wrap_chars`]: wraps `text` into lines of at most `max_chars` columns, preferring to break lines
/// AFTER `/` or `\` path separators so paths stay readable, and hard-splitting only a separator-less segment longer
/// than a full line (a too long file name). Returns the raw lines, WITHOUT the vertical padding [`wrap_chars`] adds
/// for the annotation notes.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::ui_widgets::wrap_chars_lines;
///
/// let lines = wrap_chars_lines("/a/very/long/path.txt", 8);
/// assert!(lines.iter().all(|line| line.chars().count() <= 8));
/// assert!(lines.len() > 1);
/// ```
pub fn wrap_chars_lines(text: &str, max_chars: usize) -> Vec<String> {
    // Split into separator-terminated segments, hard-splitting any segment longer than a
    // full line (a too long name with no separator to break on).
    let mut chunks: Vec<String> = Vec::new();
    for seg in text.split_inclusive(|c: char| ['/', '\\', '_', ' '].contains(&c)) {
        let mut chars = seg.chars();
        loop {
            let chunk: String = chars.by_ref().take(max_chars).collect();
            if chunk.is_empty() {
                break;
            }
            chunks.push(chunk);
        }
    }

    // Greedily pack the chunks into lines of at most `max_chars`, breaking after separators.
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for chunk in chunks {
        if !current.is_empty() && current.chars().count() + chunk.chars().count() > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        current.push_str(&chunk);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Double-click tracking for the buttons of a UI list: tells whether the press being processed completes a
/// "double-click" — a second press on the SAME entry within a delay — as used by the `FileBrowserB` file browser.
///
/// [`DoubleClick::press`] must be called right after the `Ui::button`/`Ui::radio` call of a list entry, while it is
/// still the "last element": the caller first checks [`Ui::get_last_element_active`] itself, then lets
/// [`DoubleClick::press`] consume the timing.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::ui_widgets::DoubleClick;
///
/// let mut double_click = DoubleClick::default();
/// assert!(!double_click.press("file.txt", 0.5, true)); // first press
/// assert!(double_click.press("file.txt", 0.5, true));  // second press within the delay: double-click
/// assert!(!double_click.press("file.txt", 0.5, true)); // the double-click reset the tracker: first press again
/// ```
#[derive(Default)]
pub struct DoubleClick {
    /// The instant of, and the entry name of, the last recorded press.
    last: Option<(Instant, String)>,
}

impl DoubleClick {
    /// Records a press on the entry `name` and returns `true` when it completes a double-click: the previously
    /// recorded press was on the same `name` less than `delay` seconds ago (a `delay` of 0 disables the
    /// double-click), AND `extra_condition` holds — an extra caller-side requirement, e.g. "that name is the only
    /// one of the selection set".
    ///
    /// A completed double-click resets the tracker (`last` back to `None`), so three quick presses give ONE
    /// double-click and not two. The press is recorded even when it is not a double-click, or when `extra_condition`
    /// is false: it can still be the first click of the next double-click.
    pub fn press(&mut self, name: &str, delay: f32, extra_condition: bool) -> bool {
        let now = Instant::now();
        // The previous press must be recent (`delay` of 0 disables the double-click) and on that same entry.
        let double = match &self.last {
            Some((at, prev)) => {
                delay > 0.0 && now.duration_since(*at).as_secs_f32() < delay && prev.as_str() == name && extra_condition
            }
            None => false,
        };
        self.last = if double { None } else { Some((now, name.to_string())) };
        double
    }

    /// Forgets the previous press, so the next [`DoubleClick::press`] can never complete a double-click with it.
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// A drop-down (combo box) button: a [`Ui::button`] showing the current choice with an optional icon — typically a
/// [`Sprite::arrow_down`] — that opens a real drop-down list above or below it.
///
/// The list is at most [`Dropdown::max_visible`] rows high: a vertical [`Scrollbar`] appears when there are more
/// choices, and the rows are plain [`Ui::text`] made interactive with [`Ui::button_behavior`] (with a hover
/// highlight), so the list never renders as a stack of buttons.
///
/// The widget keeps the state a drop-down needs (whether the list is open, how many rows it shows, its scroll and
/// where its button was drawn); the caller keeps the choices and the selection. A typical frame:
/// 1. [`Dropdown::button`] draws the button at the current layout position, on the same line as the other elements,
/// 2. draw the rest of the UI,
/// 3. [`Dropdown::list`] draws the choices, floating in front of the UI — call it after the enclosing window or panel
///    so the list covers its content — and returns the index of the pressed choice.
///
/// [`Dropdown::draw`] does the two calls in a row, for a drop-down whose list may float over what follows it.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!();
/// use stereokit_rust::{sprite::Sprite, tools::ui_widgets::Dropdown, ui::Ui};
///
/// // In the app's frame, inside the current `Ui` layout:
/// let mut dropdown = Dropdown::default();
/// let choices = ["English", "French", "German"];
/// let mut selected = 0;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if let Some(index) = dropdown.draw("language", choices[selected], Some(&Sprite::arrow_down()), &choices) {
///         selected = index;
///     }
///     Ui::label(format!("Language: {}", choices[selected])).draw();
/// );
/// # sk::Sk::shutdown();
/// ```
/// How far in front of the current UI the drop-down list floats, in meters: at the depth of the window / panel behind
/// it, its backplate would share its plane with theirs and flicker (z-fighting).
const DROPDOWN_LIFT: f32 = 0.05;

/// Extra lift of the hover highlight and of the row texts over the backplate of the list, so they never share their
/// plane with it.
const ROW_LIFT: f32 = 0.002;

pub struct Dropdown {
    /// Whether the list of choices is open. [`Dropdown::button`] toggles it, [`Dropdown::list`] closes it when a
    /// choice is pressed or when its button was not drawn.
    pub open: bool,
    /// Whether the list opens above the button (`true`) or below it (`false`, the default).
    pub up: bool,
    /// The maximum number of rows displayed at once: the list is at most that many rows high, and shows a vertical
    /// scrollbar when it has more choices. Default is 6, and at least 1 row is always shown.
    pub max_visible: usize,
    /// The vertical scroll of the list, expressed in whole rows.
    scrollbar: Scrollbar,
    /// Whether an interactor pointed at the button of the last [`Dropdown::button`] call: the list then also scrolls
    /// from the mouse wheel / controller stick while only the button is focused.
    button_focused: bool,
    /// The layout bounds of the button drawn by [`Dropdown::button`] on the current frame: the anchor of the list,
    /// see [`Dropdown::list`]. `None` until the button has been drawn this frame.
    anchor: Option<Bounds>,
}

impl Default for Dropdown {
    /// A closed drop-down, opening its list below the button and showing at most 6 rows at once.
    fn default() -> Self {
        Self {
            open: false,
            up: false,
            max_visible: 6,
            scrollbar: Scrollbar::vertical(),
            button_focused: false,
            anchor: None,
        }
    }
}

impl Dropdown {
    /// A closed drop-down, opening its list below the button, like [`Dropdown::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// A closed drop-down whose list opens above the button, e.g. for a UI sitting at the bottom of the view, see
    /// [`Dropdown::up`].
    pub fn upwards() -> Self {
        Self { up: true, ..Self::default() }
    }

    /// Draws the button of the drop-down at the current layout position: `text` is the current choice, `image` an
    /// optional icon drawn on the left of it (e.g. [`Sprite::arrow_down`]). Pressing it opens or closes the list, and
    /// its layout bounds are remembered for [`Dropdown::list`], which must be called later in the same frame.
    ///
    /// `id` is a root id for the button, making two drop-downs showing the same text independent.
    ///
    /// Returns whether the button was pressed this frame.
    pub fn button(&mut self, id: &str, text: &str, image: Option<&Sprite>) -> bool {
        // The anchor of the previous frame is stale: the enclosing layout may have moved since.
        self.anchor = None;
        Ui::push_id(id);
        let mut button = Ui::button(text);
        if let Some(image) = image {
            button = button.image(image);
        }
        let pressed = button.press();
        self.anchor = Some(Ui::get_layout_last());
        self.button_focused = Ui::get_last_element_focused().is_active();
        Ui::pop_id();
        if pressed {
            self.open = !self.open;
            if self.open {
                // A freshly opened list starts at its first row.
                self.scrollbar.reset();
            }
        }
        pressed
    }

    /// Draws the list of `choices` when the drop-down is open, floating in front of the UI as a real drop-down: at
    /// most [`Dropdown::max_visible`] rows high, with a vertical [`Scrollbar`] when there are more choices. The list
    /// is at least as wide as the drop-down button and wide enough for the longest choice, and opens above the
    /// button when [`Dropdown::up`], below it otherwise.
    ///
    /// The rows are [`Ui::text`] made interactive with [`Ui::button_behavior`]: the hovered / pressed row is
    /// highlighted, and pressing one closes the list and returns its index. The list also scrolls from the mouse
    /// wheel in simulation or the controller thumbsticks in XR while the button or one of its rows is focused, see
    /// [`Scrollbar::apply_input_scroll`].
    ///
    /// Nothing is drawn (and `None` is returned) when the list is closed, when it is empty, or when
    /// [`Dropdown::button`] was not called this frame.
    pub fn list<S: AsRef<str>>(&mut self, id: &str, choices: &[S]) -> Option<usize> {
        if !self.open {
            return None;
        }
        let Some(anchor) = self.anchor else {
            self.open = false;
            return None;
        };
        if choices.is_empty() {
            self.open = false;
            return None;
        }

        let settings = Ui::get_settings();
        let style = Ui::get_text_style();
        let line = Ui::get_line_height();
        let total = choices.len();
        let visible_rows = total.min(self.max_visible.max(1));

        // `UiCut::Right` needs a fixed width: the scrollbar takes a strip on the right of the list.
        let slider_w = if total > visible_rows { line * 0.7 } else { 0.0 };

        // At least as wide as the button, wide enough for the longest choice, plus the scrollbar strip.
        let width = choices
            .iter()
            .map(|choice| Text::size_layout(choice.as_ref(), Some(style), None).x + settings.padding * 2.0)
            .fold(anchor.dimensions.x, f32::max)
            + slider_w;

        // A real drop-down: a bounded height, not a list following the number of choices.
        let height = visible_rows as f32 * line;

        // The layout grows downward from its top-left corner (X+ goes left, Y+ goes up): when the list opens above
        // the button, its top is one list height higher. The negative Z offset lifts the list in front of the UI it
        // covers instead of blending with it (see `DROPDOWN_LIFT`).
        let mut start = Vec3::new(
            anchor.center.x + anchor.dimensions.x / 2.0,
            anchor.center.y - anchor.dimensions.y / 2.0 - settings.padding * 0.5,
            anchor.center.z - DROPDOWN_LIFT,
        );
        if self.up {
            start.y += height + settings.padding;
        }

        Ui::push_id(format!("{id}_list"));
        Ui::layout_push(start, Vec2::new(width, height), false);
        // The list is an explicit box: the rows are placed exactly `line` apart, without the layout gutter that used
        // to space them too far apart and pushed the last choices out of reach.
        let list_at = Ui::get_layout_at();
        Ui::panel_at(list_at, Vec2::new(width, height), Some(UiPad::None));

        let max_scroll = (total - visible_rows) as f32;
        self.scrollbar.clamp_scroll(max_scroll);

        // Cut the scrollbar strip, then the rows use the width left of it.
        if slider_w > 0.0 {
            Ui::layout_push_cut(UiCut::Right, slider_w, false);
            self.scrollbar.draw_scrollbar(
                &format!("{id}_scroll"),
                slider_w,
                height,
                max_scroll,
                visible_rows,
                total,
                &settings,
            );
            Ui::layout_pop();
        }
        let content_w = width - slider_w;

        let mut chosen = None;
        // Whether the user points at the button or at one of the rows: `Ui::get_last_element_focused` only knows the
        // LAST row drawn, so the focus of every row is accumulated here for `Scrollbar::apply_input_scroll`.
        let mut list_focused = self.button_focused;
        let (start_row, visible_rows) = dropdown_rows(total, self.max_visible, self.scrollbar.scroll);
        let row_size = Vec2::new(content_w, line);
        for row in 0..visible_rows {
            let index = start_row + row;
            if index >= total {
                break;
            }
            // `line` per row: no gutter, so `visible_rows` rows always fit the fixed height and the scroll can
            // reach every choice.
            let row_tlb = list_at - Vec3::new(0.0, row as f32 * line, 0.0);

            // The row is not a button: `Ui::button_behavior` only brings the interaction (hover, press, focus) to
            // the row bounds, with an id unique per choice.
            let mut finger_offset = 0.0;
            let mut button_state = BtnState::empty();
            let mut focus_state = BtnState::empty();
            Ui::push_id_int(index as i32);
            Ui::button_behavior(
                row_tlb,
                row_size,
                choices[index].as_ref(),
                &mut finger_offset,
                &mut button_state,
                &mut focus_state,
                None,
            );
            Ui::pop_id();
            list_focused |= button_state.is_active() || focus_state.is_active();

            // Hover / press highlight, behind the text and just in front of the list backplate.
            if button_state.is_active() {
                Ui::push_tint(Ui::get_theme_color(UiColor::Common, Some(UiColorState::Active)));
                Ui::panel_at(row_tlb - Vec3::new(0.0, 0.0, ROW_LIFT), row_size, Some(UiPad::None));
                Ui::pop_tint();
            }
            Ui::text(choices[index].as_ref())
                .at(row_tlb - Vec3::new(settings.padding * 0.5, 0.0, ROW_LIFT), row_size)
                .text_align(Align::CenterLeft)
                .draw();

            if button_state.is_just_inactive() {
                chosen = Some(index);
            }
        }

        Ui::layout_pop();
        Ui::pop_id();

        // Mouse wheel (simulation) / controller thumbsticks (XR): scroll the list while the user points at the
        // dropdown button or at one of its rows, see `Scrollbar::apply_input_scroll`.
        self.scrollbar.apply_input_scroll(max_scroll, list_focused);

        if chosen.is_some() {
            self.open = false;
        }
        chosen
    }

    /// [`Dropdown::button`] followed by [`Dropdown::list`]: the shortcut for a drop-down whose list is allowed to
    /// float over the UI drawn after it (the list is lifted in front of that UI).
    pub fn draw<S: AsRef<str>>(
        &mut self,
        id: &str,
        text: &str,
        image: Option<&Sprite>,
        choices: &[S],
    ) -> Option<usize> {
        self.button(id, text, image);
        self.list(id, choices)
    }

    /// Closes the list, if it is open.
    pub fn close(&mut self) {
        self.open = false;
    }
}

/// The first choice shown by a [`Dropdown`] list and the number of rows it displays, for `total` choices: at most
/// `max_visible` rows (and at least one), with `scroll` clamped to the last page. The result is expressed the same way
/// as [`Scrollbar::scroll`], so `start + rows` always reaches `total` at the end of the scroll, even with many choices.
fn dropdown_rows(total: usize, max_visible: usize, scroll: f32) -> (usize, usize) {
    let visible = total.min(max_visible.max(1));
    let max_scroll = total.saturating_sub(visible) as f32;
    (scroll.clamp(0.0, max_scroll) as usize, visible)
}

/// The scroll state of ONE scroll axis of a scrollable UI element: current position, custom scrollbar and
/// peripheral-input scrolling (mouse wheel in simulation, controller thumbsticks in XR). A vertical
/// [`Scrollbar`] scrolls the ROWS of a list, as in the `FileBrowserB` file browser; a horizontal one scrolls the
/// COLUMNS of a wide content strip.
///
/// The scroll stays expressed in WHOLE cells — rows for a vertical scrollbar, columns for a horizontal one: the
/// scrollbar rounds it, and only the whole cells of the accumulated input delta are applied, the fractional
/// remainder carrying over in `scroll_accum` until it makes a full cell. A typical draw loop:
/// 1. compute `visible_cells` (with [`Scrollbar::visible_cells_count`]) and `total_cells` from the content,
/// 2. `max_scroll = (total_cells - visible_cells).max(0)`, then [`Scrollbar::clamp_scroll`],
/// 3. if `total_cells > visible_cells`, cut a strip of the layout along the axis (`UiCut::Right` for a vertical
///    scrollbar, `UiCut::Bottom` for a horizontal one) and call [`Scrollbar::draw_scrollbar`],
/// 4. draw the visible cells, starting at `scroll as usize`,
/// 5. finally call [`Scrollbar::apply_input_scroll`], with whether the enclosing window had the focus.
pub struct Scrollbar {
    /// Orientation of the scroll axis.
    dir: UiDir,
    /// Current scroll position along the axis, expressed in whole cells (rows for a vertical scrollbar, columns
    /// for a horizontal one).
    pub scroll: f32,
    /// Fractional remainder of the mouse wheel / thumbstick scrolling: the scroll stays expressed in whole
    /// cells, so the fraction of a cell scrolled by a partial input accumulates here until it makes a full cell.
    scroll_accum: f32,
    /// Whether the scrollbar thumb was focused on the previous frame: StereoKit's sliders already scroll from the
    /// controller stick through their "secondary motion" while focused, so [`Scrollbar::apply_input_scroll`] must
    /// not double it with its own window-level stick scrolling.
    scrollbar_focused: bool,
}

impl Default for Scrollbar {
    /// A vertical [`Scrollbar`], like [`Scrollbar::vertical`].
    fn default() -> Self {
        Self::new(UiDir::Vertical)
    }
}

impl Scrollbar {
    /// A scrollbar of the given orientation: vertical to scroll the rows of a list, horizontal to scroll the
    /// columns of a wide content strip.
    pub fn new(dir: UiDir) -> Self {
        Self { dir, scroll: 0.0, scroll_accum: 0.0, scrollbar_focused: false }
    }

    /// A vertical scrollbar, scrolling the rows of a list from top to bottom: the default orientation.
    pub fn vertical() -> Self {
        Self::new(UiDir::Vertical)
    }

    /// A horizontal scrollbar, scrolling the columns of a wide content strip from left to right.
    pub fn horizontal() -> Self {
        Self::new(UiDir::Horizontal)
    }

    /// The orientation of the scroll axis, see [`Scrollbar::new`].
    pub fn dir(&self) -> UiDir {
        self.dir
    }

    /// Scrolls back to the start of the content — the top of a vertical list, the left of a horizontal strip —
    /// dropping any pending fractional scroll.
    pub fn reset(&mut self) {
        self.scroll = 0.0;
        self.scroll_accum = 0.0;
    }

    /// Clamps the scroll position into `[0, max_scroll]`, with `max_scroll = (total_cells - visible_cells).max(0)`.
    pub fn clamp_scroll(&mut self, max_scroll: f32) {
        self.scroll = self.scroll.clamp(0.0, max_scroll);
    }

    /// Number of cells that fit in `available` — the height of the content area for a vertical scrollbar, its
    /// width for a horizontal one — clamped to `max_visible_cells` when it is non-zero (0 means auto).
    ///
    /// `cell_size` is the effective size of ONE cell along the axis, computed by the caller so it matches what
    /// the cell widgets actually reserve (e.g. an explicit grid cell height, or the line height of the current
    /// text style for auto-height list buttons). `gutter` is the scaled [`UiSettings`] gutter of the window.
    pub fn visible_cells_count(&self, available: f32, cell_size: f32, gutter: f32, max_visible_cells: u32) -> usize {
        let mut cells = if cell_size <= 0.0 {
            1
        } else {
            ((available + gutter) / (cell_size + gutter)).floor().max(1.0) as usize
        };
        if max_visible_cells > 0 {
            cells = cells.min(max_visible_cells as usize);
        }
        cells.max(1)
    }

    /// The scrollbar itself, vertical or horizontal per [`Scrollbar::new`]. Instead of a plain
    /// [`Ui::vslider`]/[`Ui::hslider`], which allows a custom rendering: a full-length thin track behind a thumb
    /// whose size along the axis is proportional to the visible fraction of the content, with a floor so it never
    /// gets too small to grab on very large contents, and the same sound feedback as StereoKit's sliders
    /// (activation on/off, then a tick per cell).
    ///
    /// Reserves and draws its layout area itself: call it inside the (already cut) layout region of the scrollbar,
    /// and only when `total_cells > visible_cells`. `settings` are the scaled [`UiSettings`] of the window (its
    /// depth and padding are used), and `id_str` participates in the UI id hash so two scrollbars in the same
    /// window never share their state. Keeps the scroll expressed in whole cells, like a `.step(1.0)` slider.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_scrollbar(
        &mut self,
        id_str: &str,
        width: f32,
        height: f32,
        max_scroll: f32,
        visible_cells: usize,
        total_cells: usize,
        settings: &UiSettings,
    ) {
        let bar_bounds = Ui::layout_reserve(Vec2::new(width, height), false, settings.depth);
        let tlb = bar_bounds.tlb();
        let vertical = self.dir == UiDir::Vertical;

        // Thumb size: same cross-axis ratio as StereoKit's slider push buttons (`size_min * 0.55`), length
        // along the axis proportional to the visible fraction of the content, with a floor so it never
        // gets too small to grab on very large contents.
        let thumb_size = scrollbar_thumb_size(self.dir, width, height, visible_cells, total_cells);

        // A vertical scrollbar binds the scroll to the Y of the element like a vslider, a horizontal one
        // to the X like an hslider; the other axis stays pinned at 0.
        let mut value = if vertical { Vec2::new(0.0, self.scroll) } else { Vec2::new(self.scroll, 0.0) };
        let max = if vertical { Vec2::new(0.0, max_scroll) } else { Vec2::new(max_scroll, 0.0) };
        let mut slider = UiSliderData::default();
        let id = Ui::stack_hash(id_str);
        Ui::slider_behavior(
            tlb,
            Vec2::new(width, height),
            id,
            &mut value,
            Vec2::new(0.0, 0.0),
            max,
            thumb_size,
            thumb_size + Vec2::new(settings.padding, settings.padding) * 2.0,
            None, // UiConfirm::Push, the slider default
            &mut slider,
        );

        // Keep the scroll expressed in whole cells, like a `.step(1.0)` slider.
        let prev_cell = self.scroll.round();
        let raw_scroll = if vertical { value.y } else { value.x };
        self.scroll = raw_scroll.round().clamp(0.0, max_scroll);

        let focus = Ui::get_anim_focus(id, slider.focus_state, slider.active_state);
        // Remember the thumb focus for the next frame's `apply_input_scroll`: StereoKit's sliders
        // already scroll from the controller stick ("secondary motion") while focused.
        self.scrollbar_focused = slider.focus_state.is_active();
        // `button_center` is the center of the thumb, `draw_element` expects its top-left corner.
        let thumb_at =
            Vec3::new(slider.button_center.x + thumb_size.x / 2.0, slider.button_center.y + thumb_size.y / 2.0, tlb.z);

        // Track: full-length thin inactive line behind the thumb.
        Ui::draw_element(UiVisual::SliderLine, None, tlb, Vec3::new(width, height, settings.depth * 0.1), focus);
        // Thumb: SliderPush with a length proportional to the visible part of the content.
        Ui::draw_element(
            UiVisual::SliderPush,
            None,
            thumb_at,
            Vec3::new(thumb_size.x, thumb_size.y, settings.depth),
            focus,
        );

        // Same sound feedback as StereoKit's sliders: activation on/off, then a tick per cell.
        if slider.active_state.is_just_active() {
            Ui::play_sound_on_off(UiVisual::SliderPush, id, thumb_at);
        }
        if slider.active_state.is_active() && prev_cell != self.scroll {
            Ui::play_sound_on(UiVisual::SliderPush, thumb_at);
        }
    }

    /// Moves the scroll from the peripheral inputs, only when the enclosing window has the focus:
    /// - in simulation ([`DisplayType::Flatscreen`]), the mouse wheel: [`Input::get_mouse()`]`.scroll_change`,
    ///   `CELLS_PER_WHEEL_NOTCH` cells per wheel notch, a notch towards the screen scrolling towards the start of
    ///   the content. The `sk_app` backends report the wheel in Win32 `WHEEL_DELTA` units (±120 per notch, like
    ///   StereoKit C's own mouse interactor tilt that divides `scroll_change` by thousands), but some others
    ///   already normalize it to ±1 notches — both are handled. `mouse_t` only has a vertical wheel, so a
    ///   HORIZONTAL scrollbar takes the wheel only while Shift is held — the usual desktop horizontal-scroll
    ///   convention — and a vertical one only while it is not, so a window hosting both scrollbars splits the
    ///   wheel between them without double scrolling.
    /// - in XR, the controller thumbsticks [`Input::xy`]: the stick with the largest deflection ALONG THE AXIS
    ///   (Y for a vertical scrollbar, X for a horizontal one) scrolls `STICK_CELLS_PER_SECOND` cells per second at
    ///   full deflection, stick forward scrolling up a vertical list, stick right towards the end of a horizontal
    ///   strip. Clicking that winning stick inward (its [`InputButton::LStick`]/[`InputButton::RStick`] button,
    ///   like the `fly_over` tool boosts its move speed) multiplies the scroll speed by
    ///   `STICK_CLICK_SPEED_BOOST`. When the scrollbar thumb itself is focused, StereoKit's native slider
    ///   "secondary motion" already scrolls it from the stick, so this window-level scrolling steps aside to
    ///   avoid doubling it.
    ///
    /// The scroll stays expressed in whole cells (like [`Scrollbar::draw_scrollbar`], which rounds it): only the
    /// whole cells of the accumulated input delta are applied to `scroll`, the fractional remainder carrying over in
    /// `scroll_accum` until it makes a full cell.
    pub fn apply_input_scroll(&mut self, max_scroll: f32, window_focused: bool) {
        if max_scroll <= 0.0 || !window_focused {
            // Nothing to scroll, or nobody points at the window: drop the pending fraction so the
            // next scroll session starts from a clean slate instead of jumping a cell.
            self.scroll_accum = 0.0;
            return;
        }

        // Cells to scroll this frame: positive = further towards the end of the content.
        const CELLS_PER_WHEEL_NOTCH: f32 = 3.0;
        const STICK_CELLS_PER_SECOND: f32 = 8.0;
        const STICK_CLICK_SPEED_BOOST: f32 = 3.0;
        const STICK_DEADZONE: f32 = 0.15;
        const WHEEL_DELTA: f32 = 120.0;
        const WHEEL_DELTA_THRESHOLD: f32 = 20.0;

        let delta = if Device::get_display_type() == DisplayType::Flatscreen {
            // Simulation: the mouse wheel, plain for a vertical scrollbar, Shift-held for a horizontal one.
            if Input::key(Key::Shift).is_active() != (self.dir == UiDir::Horizontal) {
                return;
            }
            let wheel = Input::get_mouse().scroll_change;
            if wheel == 0.0 {
                return;
            }
            // Deltas of at least a fraction of `WHEEL_DELTA` are Win32 wheel units, smaller ones are
            // already normalized notches. Wheel forward (positive) scrolls towards the start of the content.
            let notches = if wheel.abs() >= WHEEL_DELTA_THRESHOLD { wheel / WHEEL_DELTA } else { wheel };
            -notches * CELLS_PER_WHEEL_NOTCH
        } else {
            // XR: the controller thumbsticks, the one with the largest deflection along the axis wins.
            // Skip when the scrollbar thumb is focused: StereoKit's sliders already bind the stick.
            if self.scrollbar_focused {
                return;
            }
            let left = Input::xy(InputXY::LStick);
            let right = Input::xy(InputXY::RStick);
            let (left, right) = if self.dir == UiDir::Vertical { (left.y, right.y) } else { (left.x, right.x) };
            let left_wins = left.abs() >= right.abs();
            let stick = if left_wins { left } else { right };
            if stick.abs() < STICK_DEADZONE {
                return;
            }
            // Clicking the winning stick inward boosts the scroll speed, like the move speed of `fly_over`.
            let stick_button = if left_wins { InputButton::LStick } else { InputButton::RStick };
            let speed_boost = if Input::button(stick_button).is_active() { STICK_CLICK_SPEED_BOOST } else { 1.0 };
            // Stick forward (positive Y) scrolls towards the start of a vertical list, like the wheel; stick
            // right (positive X) scrolls towards the end of a horizontal strip, like dragging its thumb.
            let stick_sign = if self.dir == UiDir::Vertical { -1.0 } else { 1.0 };
            stick_sign * stick * STICK_CELLS_PER_SECOND * speed_boost * Time::get_stepf()
        };

        self.scroll_accum += delta;
        // Apply whole cells only, and keep the fraction for the next frame.
        let cells = self.scroll_accum.trunc();
        self.scroll_accum -= cells;
        self.scroll = (self.scroll + cells).clamp(0.0, max_scroll);
    }
}

/// Size of the thumb drawn by [`Scrollbar::draw_scrollbar`]: StereoKit's slider push buttons are
/// `size_min * 0.55` square, with `size_min` the SHORT (cross) axis of the bar — the width of a vslider, the
/// height of an hslider. The thumb keeps that cross size, and its size ALONG the axis is proportional to the
/// visible fraction of the content, with a floor at the cross size so it never gets too small to grab on very
/// large contents.
fn scrollbar_thumb_size(dir: UiDir, width: f32, height: f32, visible_cells: usize, total_cells: usize) -> Vec2 {
    let (cross, along) = if dir == UiDir::Vertical { (width, height) } else { (height, width) };
    let thumb_cross = cross * 0.55;
    let thumb_along = if total_cells == 0 {
        thumb_cross
    } else {
        (along * visible_cells as f32 / total_cells as f32).max(thumb_cross)
    };
    if dir == UiDir::Vertical {
        Vec2::new(thumb_cross, thumb_along)
    } else {
        Vec2::new(thumb_along, thumb_cross)
    }
}

/// Whether the last UI element drawn (typically a `Ui::button` / `Ui::radio` of a list) is focused: an interactor
/// is in or near it, see [`Ui::get_last_element_focused`].
///
/// Must be called right after the widget call, while it is still the "last element".
pub fn is_last_element_focused() -> bool {
    Ui::get_last_element_focused().is_active()
}

/// The world-space pose of the last UI element drawn, while it is still the "last element": [`Ui::get_layout_last`]
/// gives its layout bounds, whose center (window-local layout coordinates) is converted through the current UI
/// hierarchy — exactly like StereoKit's `ui_popup_pose` does when it attaches a popup to the focused element.
/// Useful to draw a preview, a tooltip or a callout right next to a focused list button.
pub fn last_element_world_pose() -> Pose {
    let bounds = Ui::get_layout_last();
    Pose {
        position: Hierarchy::to_world_point(bounds.center),
        orientation: Hierarchy::to_world_rotation(Quat::IDENTITY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_chars_lines_respects_max_chars() {
        let lines = wrap_chars_lines("/a/very/long/path.txt", 8);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| line.chars().count() <= 8));
        assert!(lines.len() > 1);
    }

    #[test]
    fn wrap_chars_lines_breaks_after_separators() {
        let lines = wrap_chars_lines("/a/very/long/path.txt", 8);
        // Breaks happen AFTER the `/` separators, never in the middle of a segment.
        assert!(lines.iter().all(|line| line.ends_with(['/', '\\', '_', ' ']) || line.chars().count() <= 8));
    }

    #[test]
    fn wrap_chars_lines_hard_splits_unseparated_names() {
        let lines = wrap_chars_lines("averyveryverylongfilename", 8);
        assert!(lines.iter().all(|line| line.chars().count() <= 8));
        assert!(lines.len() > 1);
    }

    #[test]
    fn wrap_chars_pads_to_three_lines() {
        // A short text is padded with blank lines up to a minimum of 3 lines.
        assert_eq!(wrap_chars("short", 35).lines().count(), 3);
        // A 2-lines wrap is padded to 3.
        let two_lines = wrap_chars_lines("aaaaaaaa/bbbbbbbb", 9);
        assert_eq!(two_lines.len(), 2);
        assert_eq!(wrap_chars("aaaaaaaa/bbbbbbbb", 9).lines().count(), 3);
    }

    #[test]
    fn double_click_detects_and_resets() {
        let mut double_click = DoubleClick::default();
        assert!(!double_click.press("file.txt", 0.5, true));
        assert!(double_click.press("file.txt", 0.5, true));
        // The completed double-click reset the tracker: this is a first press again.
        assert!(!double_click.press("file.txt", 0.5, true));
    }

    #[test]
    fn double_click_requires_same_name_and_extra_condition() {
        let mut double_click = DoubleClick::default();
        assert!(!double_click.press("a.txt", 0.5, true));
        // A press on a different entry never completes the double-click, but is recorded: the next
        // press on IT can.
        assert!(!double_click.press("b.txt", 0.5, true));
        assert!(double_click.press("b.txt", 0.5, true));
        // The completed double-click reset the tracker: first press again.
        assert!(!double_click.press("b.txt", 0.5, true));
        // The extra condition must hold on the second press: same-name presses with it false never double-click.
        assert!(!double_click.press("c.txt", 0.5, false));
        assert!(!double_click.press("c.txt", 0.5, false));
        assert!(!double_click.press("c.txt", 0.5, false));
        // A delay of 0 disables the double-click.
        assert!(!double_click.press("d.txt", 0.0, true));
        assert!(!double_click.press("d.txt", 0.0, true));
        // And `reset` forgets the previous press.
        double_click.reset();
        assert!(!double_click.press("e.txt", 0.5, true));
    }

    #[test]
    fn scrollbar_clamp_and_visible_cells() {
        let mut scrollbar = Scrollbar::default();
        assert_eq!(scrollbar.dir(), UiDir::Vertical);
        scrollbar.scroll = -3.0;
        scrollbar.clamp_scroll(10.0);
        assert_eq!(scrollbar.scroll, 0.0);
        scrollbar.scroll = 42.0;
        scrollbar.clamp_scroll(10.0);
        assert_eq!(scrollbar.scroll, 10.0);
        scrollbar.reset();
        assert_eq!(scrollbar.scroll, 0.0);

        // 12 cells of 0.01 + gutter 0.005 fit in 0.17: floor((0.17 + 0.005) / (0.01 + 0.005)) = 11.
        assert_eq!(scrollbar.visible_cells_count(0.17, 0.01, 0.005, 0), 11);
        // ... clamped by max_visible_cells when non-zero.
        assert_eq!(scrollbar.visible_cells_count(0.17, 0.01, 0.005, 5), 5);
        // Always at least one cell, even with no available space.
        assert_eq!(scrollbar.visible_cells_count(0.0, 0.01, 0.005, 0), 1);

        // The horizontal constructor starts from the same clean state.
        let horizontal = Scrollbar::horizontal();
        assert_eq!(horizontal.dir(), UiDir::Horizontal);
        assert_eq!(horizontal.scroll, 0.0);
    }

    #[test]
    fn scrollbar_thumb_size_is_proportional_with_floor() {
        // Vertical: a 0.01-wide bar gives a `0.01 * 0.55` thumb, half of the 0.02 track for 1 of 2 visible rows.
        let thumb = scrollbar_thumb_size(UiDir::Vertical, 0.01, 0.02, 1, 2);
        assert!((thumb.x - 0.0055).abs() < 1e-6);
        assert!((thumb.y - 0.01).abs() < 1e-6);
        // Horizontal: the same sizes, mirrored on the other axis.
        let thumb = scrollbar_thumb_size(UiDir::Horizontal, 0.02, 0.01, 1, 2);
        assert!((thumb.x - 0.01).abs() < 1e-6);
        assert!((thumb.y - 0.0055).abs() < 1e-6);
        // Very large contents never shrink the thumb below its cross size, so it stays grabbable.
        let thumb = scrollbar_thumb_size(UiDir::Vertical, 0.01, 1.0, 1, 1000);
        assert!((thumb.x - 0.0055).abs() < 1e-6);
        assert!((thumb.y - 0.0055).abs() < 1e-6);
        let thumb = scrollbar_thumb_size(UiDir::Horizontal, 1.0, 0.01, 1, 1000);
        assert!((thumb.x - 0.0055).abs() < 1e-6);
        assert!((thumb.y - 0.0055).abs() < 1e-6);
    }

    #[test]
    fn dropdown_rows_reach_the_last_choice() {
        assert_eq!(dropdown_rows(9, 6, 0.0), (0, 6));
        // Scrolled to the end, the last choice (Chinese, after Portuguese) is part of the visible rows.
        assert_eq!(dropdown_rows(9, 6, 3.0), (3, 6));
        // An out of range scroll is clamped to the last page.
        assert_eq!(dropdown_rows(9, 6, 99.0), (3, 6));
        // Everything fits: no scroll.
        assert_eq!(dropdown_rows(3, 6, 2.0), (0, 3));
        // At least one row, even with a zero `max_visible`.
        assert_eq!(dropdown_rows(4, 0, 0.0), (0, 1));
    }

    #[test]
    fn dropdown_starts_closed() {
        let dropdown = Dropdown::default();
        assert!(!dropdown.open);
        assert!(!dropdown.up);
        assert!(dropdown.anchor.is_none());

        // The upward constructor only changes the direction of the list.
        let upwards = Dropdown::upwards();
        assert!(!upwards.open);
        assert!(upwards.up);
        assert!(upwards.anchor.is_none());
        // A real drop-down shows a bounded number of rows, so a scrollbar can appear.
        assert_eq!(upwards.max_visible, 6);

        // Closing an already closed drop-down is a no-op.
        let mut dropdown = Dropdown::new();
        dropdown.close();
        assert!(!dropdown.open);
    }
}
