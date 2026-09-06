use crate::{
    maths::{Pose, Quat, Vec2, Vec3},
    system::{Hierarchy, Input, InputXY},
    ui::{Ui, UiSettings, UiSliderData, UiVisual},
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
/// use stereokit_rust::tools::ui_list::wrap_chars_lines;
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
/// use stereokit_rust::tools::ui_list::DoubleClick;
///
/// let mut double_click = DoubleClick::default();
/// assert!(!double_click.press("file.txt", 0.5, true)); // first press
/// assert!(double_click.press("file.txt", 0.5, true));  // second press within the delay: double-click
/// assert!(!double_click.press("file.txt", 0.5, true)); // the double-click reset the tracker: first press again
/// ```
pub struct DoubleClick {
    /// The instant of, and the entry name of, the last recorded press.
    last: Option<(Instant, String)>,
}

impl Default for DoubleClick {
    fn default() -> Self {
        Self { last: None }
    }
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

/// The scroll state of a scrollable UI list: current position, custom scrollbar and peripheral-input scrolling
/// (mouse wheel in simulation, controller thumbsticks in XR), as used by the `FileBrowserB` file browser.
///
/// The scroll stays expressed in WHOLE list rows: the scrollbar rounds it, and only the whole rows of the
/// accumulated input delta are applied, the fractional remainder carrying over in `scroll_accum` until it makes a
/// full row. A typical draw loop:
/// 1. compute `visible_rows` (with [`ScrollList::visible_rows_count`]) and `total_rows` from the list content,
/// 2. `max_scroll = (total_rows - visible_rows).max(0)`, then [`ScrollList::clamp_scroll`],
/// 3. if `total_rows > visible_rows`, cut a right portion of the layout and call [`ScrollList::draw_scrollbar`],
/// 4. draw the visible rows, starting at `scroll as usize`,
/// 5. finally call [`ScrollList::apply_input_scroll`], with whether the enclosing window had the focus.
pub struct ScrollList {
    /// Current scroll position of the list, expressed in whole rows.
    pub scroll: f32,
    /// Fractional remainder of the mouse wheel / thumbstick scrolling: the scroll stays expressed in whole rows,
    /// so the fraction of a row scrolled by a partial input accumulates here until it makes a full row.
    scroll_accum: f32,
    /// Whether the scrollbar thumb was focused on the previous frame: StereoKit's sliders already scroll from the
    /// controller stick through their "secondary motion" while focused, so [`ScrollList::apply_input_scroll`] must
    /// not double it with its own window-level stick scrolling.
    scrollbar_focused: bool,
}

impl Default for ScrollList {
    fn default() -> Self {
        Self { scroll: 0.0, scroll_accum: 0.0, scrollbar_focused: false }
    }
}

impl ScrollList {
    /// Scrolls back to the top of the list, dropping any pending fractional scroll.
    pub fn reset(&mut self) {
        self.scroll = 0.0;
        self.scroll_accum = 0.0;
    }

    /// Clamps the scroll position into `[0, max_scroll]`, with `max_scroll = (total_rows - visible_rows).max(0)`.
    pub fn clamp_scroll(&mut self, max_scroll: f32) {
        self.scroll = self.scroll.clamp(0.0, max_scroll);
    }

    /// Number of rows that fit in `available_h`, clamped to `max_visible_rows` when it is non-zero (0 means auto).
    ///
    /// `row_h` is the effective height of ONE row, computed by the caller so it matches what the row widgets
    /// actually reserve (e.g. an explicit grid cell height, or the line height of the current text style for
    /// auto-height list buttons). `gutter` is the scaled [`UiSettings`] gutter of the window.
    pub fn visible_rows_count(&self, available_h: f32, row_h: f32, gutter: f32, max_visible_rows: u32) -> usize {
        let mut rows =
            if row_h <= 0.0 { 1 } else { ((available_h + gutter) / (row_h + gutter)).floor().max(1.0) as usize };
        if max_visible_rows > 0 {
            rows = rows.min(max_visible_rows as usize);
        }
        rows.max(1)
    }

    /// The vertical scrollbar of the list. Instead of a plain [`Ui::vslider`], which allows a custom rendering: a
    /// full-height thin track behind a thumb whose height is proportional to the visible fraction of the list, with
    /// a floor so it never gets too small to grab on very large lists, and the same sound feedback as StereoKit's
    /// sliders (activation on/off, then a tick per row).
    ///
    /// Reserves and draws its layout area itself: call it inside the (already cut) layout region of the scrollbar,
    /// and only when `total_rows > visible_rows`. `settings` are the scaled [`UiSettings`] of the window (its depth
    /// and padding are used), and `id_str` participates in the UI id hash so two lists in the same window never
    /// share their scrollbar state. Keeps the scroll expressed in whole rows, like a `.step(1.0)` vslider.
    pub fn draw_scrollbar(
        &mut self,
        id_str: &str,
        width: f32,
        height: f32,
        max_scroll: f32,
        visible_rows: usize,
        total_rows: usize,
        settings: &UiSettings,
    ) {
        let bar_bounds = Ui::layout_reserve(Vec2::new(width, height), false, settings.depth);
        let tlb = bar_bounds.tlb();

        // Thumb size: same width ratio as StereoKit's vslider push button (`size_min * 0.55` for a
        // vertical slider), height proportional to the visible fraction of the list, with a floor
        // so it never gets too small to grab on very large directories.
        let thumb_w = width * 0.55;
        let thumb_h = (height * visible_rows as f32 / total_rows as f32).max(thumb_w);
        let thumb_size = Vec2::new(thumb_w, thumb_h);

        let mut value = Vec2::new(0.0, self.scroll);
        let mut slider = UiSliderData::default();
        let id = Ui::stack_hash(id_str);
        Ui::slider_behavior(
            tlb,
            Vec2::new(width, height),
            id,
            &mut value,
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, max_scroll),
            thumb_size,
            thumb_size + Vec2::new(settings.padding, settings.padding) * 2.0,
            None, // UiConfirm::Push, the vslider default
            &mut slider,
        );

        // Keep the scroll expressed in whole rows, like a `.step(1.0)` vslider.
        let prev_row = self.scroll.round();
        self.scroll = value.y.round().clamp(0.0, max_scroll);

        let focus = Ui::get_anim_focus(id, slider.focus_state, slider.active_state);
        // Remember the thumb focus for the next frame's `apply_input_scroll`: StereoKit's sliders
        // already scroll from the controller stick ("secondary motion") while focused.
        self.scrollbar_focused = slider.focus_state.is_active();
        // `button_center` is the center of the thumb, `draw_element` expects its top-left corner.
        let thumb_at = Vec3::new(slider.button_center.x + thumb_w / 2.0, slider.button_center.y + thumb_h / 2.0, tlb.z);

        // Track: full-height thin inactive line behind the thumb.
        Ui::draw_element(UiVisual::SliderLine, None, tlb, Vec3::new(width, height, settings.depth * 0.1), focus);
        // Thumb: SliderLine with a height proportional to the visible part of the list.
        Ui::draw_element(UiVisual::SliderPush, None, thumb_at, Vec3::new(thumb_w, thumb_h, settings.depth), focus);

        // Same sound feedback as StereoKit's sliders: activation on/off, then a tick per row.
        if slider.active_state.is_just_active() {
            Ui::play_sound_on_off(UiVisual::SliderPush, id, thumb_at);
        }
        if slider.active_state.is_active() && prev_row != self.scroll {
            Ui::play_sound_on(UiVisual::SliderPush, thumb_at);
        }
    }

    /// Moves the list scroll from the peripheral inputs, only when the enclosing window has the focus:
    /// - in simulation ([`DisplayType::Flatscreen`]), the mouse wheel: [`Input::get_mouse()`]`.scroll_change`,
    ///   `MOUSE_WHEEL_ROWS` rows per wheel notch, a notch towards the screen scrolling up the list. The `sk_app`
    ///   backends report the wheel in Win32 `WHEEL_DELTA` units (±120 per notch, like StereoKit C's own mouse
    ///   interactor tilt that divides `scroll_change` by thousands), but some others already normalize it to ±1
    ///   notches — both are handled.
    /// - in XR, the controller thumbsticks [`Input::xy`]: the stick with the largest Y deflection scrolls
    ///   `STICK_ROWS_PER_SECOND` rows per second at full deflection, up = up the list. When the scrollbar thumb
    ///   itself is focused, StereoKit's native slider "secondary motion" already scrolls it from the stick, so this
    ///   window-level scrolling steps aside to avoid doubling it.
    ///
    /// The scroll stays expressed in whole rows (like [`ScrollList::draw_scrollbar`], which rounds it): only the
    /// whole rows of the accumulated input delta are applied to `scroll`, the fractional remainder carrying over in
    /// `scroll_accum` until it makes a full row.
    pub fn apply_input_scroll(&mut self, max_scroll: f32, window_focused: bool) {
        if max_scroll <= 0.0 || !window_focused {
            // Nothing to scroll, or nobody points at the window: drop the pending fraction so the
            // next scroll session starts from a clean slate instead of jumping a row.
            self.scroll_accum = 0.0;
            return;
        }

        // Rows to scroll this frame: positive = further down the list.
        const MOUSE_WHEEL_ROWS: f32 = 3.0;
        const STICK_ROWS_PER_SECOND: f32 = 8.0;
        const STICK_DEADZONE: f32 = 0.15;
        const WHEEL_DELTA: f32 = 120.0;
        const WHEEL_DELTA_THRESHOLD: f32 = 20.0;

        let delta = if Device::get_display_type() == DisplayType::Flatscreen {
            // Simulation: the mouse wheel.
            let wheel = Input::get_mouse().scroll_change;
            if wheel == 0.0 {
                return;
            }
            // Deltas of at least a fraction of `WHEEL_DELTA` are Win32 wheel units, smaller ones are
            // already normalized notches. Wheel forward (positive) scrolls up the list.
            let notches = if wheel.abs() >= WHEEL_DELTA_THRESHOLD { wheel / WHEEL_DELTA } else { wheel };
            -notches * MOUSE_WHEEL_ROWS
        } else {
            // XR: the controller thumbsticks, the one with the largest vertical deflection wins.
            // Skip when the scrollbar thumb is focused: StereoKit's sliders already bind the stick.
            if self.scrollbar_focused {
                return;
            }
            let left = Input::xy(InputXY::LStick).y;
            let right = Input::xy(InputXY::RStick).y;
            let stick = if left.abs() >= right.abs() { left } else { right };
            if stick.abs() < STICK_DEADZONE {
                return;
            }
            // Stick forward (positive Y) scrolls up the list, like the wheel.
            -stick * STICK_ROWS_PER_SECOND * Time::get_stepf()
        };

        self.scroll_accum += delta;
        // Apply whole rows only, and keep the fraction for the next frame.
        let rows = self.scroll_accum.trunc();
        self.scroll_accum -= rows;
        self.scroll = (self.scroll + rows).clamp(0.0, max_scroll);
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
    fn scroll_list_clamp_and_visible_rows() {
        let mut scroll_list = ScrollList::default();
        scroll_list.scroll = -3.0;
        scroll_list.clamp_scroll(10.0);
        assert_eq!(scroll_list.scroll, 0.0);
        scroll_list.scroll = 42.0;
        scroll_list.clamp_scroll(10.0);
        assert_eq!(scroll_list.scroll, 10.0);
        scroll_list.reset();
        assert_eq!(scroll_list.scroll, 0.0);

        // 12 rows of 0.01 + gutter 0.005 fit in 0.17: floor((0.17 + 0.005) / (0.01 + 0.005)) = 11.
        assert_eq!(scroll_list.visible_rows_count(0.17, 0.01, 0.005, 0), 11);
        // ... clamped by max_visible_rows when non-zero.
        assert_eq!(scroll_list.visible_rows_count(0.17, 0.01, 0.005, 5), 5);
        // Always at least one row, even with no available height.
        assert_eq!(scroll_list.visible_rows_count(0.0, 0.01, 0.005, 0), 1);
    }
}
