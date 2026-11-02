//! Shared text state between the Android IME bridge
//! (see `crate::tools::android_soft_kdb`) and the StereoKit field it mirrors.
//!
//! The Android IME edits a hidden `EditText`; StereoKit's [`Ui::input`] edits
//! the application's `String`. Both sides exchange their whole `(text, caret)`
//! state here, so the IME can read and rewrite the real text (translation,
//! dictation, autocorrect) instead of a scratch buffer:
//!
//! * the bridge publishes each IME change as a *replacement*, applied by
//!   [`crate::ui_builders::UiInputBuilder::edit`] before it draws;
//! * `edit` publishes the field state, written back into the hidden `EditText`
//!   by the bridge.
//!
//! Everything is inert while no bridge is running (desktop, or the Android
//! stepper not registered), so `Ui::input` behaves exactly as before.
//!
//! [`Ui::input`]: crate::ui::Ui::input

// The bridge side of this module is only driven by the Android stepper; on other
// platforms only the `Ui::input` side is compiled in.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use std::sync::{Mutex, OnceLock};

use crate::system::TextContext;

/// Whole state of a text field: its text and its caret (in characters).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FieldState {
    pub text: String,
    pub caret: i32,
}

#[derive(Default)]
struct Shared {
    /// An IME bridge is running.
    active: bool,
    /// Latest state of the focused field, published by `Ui::input`.
    field: Option<FieldState>,
    /// Kind of text the focused field holds (drives the keyboard type).
    field_context: Option<TextContext>,
    /// State produced by the IME, waiting to be applied to the field.
    replacement: Option<FieldState>,
}

fn shared() -> &'static Mutex<Shared> {
    static SHARED: OnceLock<Mutex<Shared>> = OnceLock::new();
    SHARED.get_or_init(|| Mutex::new(Shared::default()))
}

/// Is an IME bridge running? (Set by the stepper, at `start()` / `close()`.)
pub(crate) fn is_active() -> bool {
    shared().lock().map(|shared| shared.active).unwrap_or(false)
}

/// Tell whether an IME bridge is running, dropping any pending state.
pub(crate) fn set_active(active: bool) {
    if let Ok(mut shared) = shared().lock() {
        shared.active = active;
        if !active {
            shared.field = None;
            shared.field_context = None;
            shared.replacement = None;
        }
    }
}

/// Publish the state of the focused field (called by [`Ui::input`]).
///
/// [`Ui::input`]: crate::ui::Ui::input
pub(crate) fn publish_field(text: &str, caret: i32, context: TextContext) {
    if let Ok(mut shared) = shared().lock() {
        if !shared.active {
            return;
        }
        shared.field = Some(FieldState { text: text.to_owned(), caret });
        shared.field_context = Some(context);
    }
}

/// Kind of text the focused field holds ([`TextContext`]), if any.
pub(crate) fn field_context() -> Option<TextContext> {
    shared().lock().ok().and_then(|shared| shared.field_context)
}

/// Latest published state of the focused field (read by the bridge).
pub(crate) fn field() -> Option<FieldState> {
    shared().lock().ok().and_then(|shared| shared.field.clone())
}

/// Publish what the IME produced, for the field to apply.
pub(crate) fn propose(state: FieldState) {
    if let Ok(mut shared) = shared().lock() {
        shared.replacement = Some(state);
    }
}

/// Is a replacement waiting to be applied by [`Ui::input`]?
///
/// [`Ui::input`]: crate::ui::Ui::input
pub(crate) fn has_replacement() -> bool {
    shared().lock().map(|shared| shared.replacement.is_some()).unwrap_or(false)
}

/// Take the pending replacement (called by [`Ui::input`]).
///
/// [`Ui::input`]: crate::ui::Ui::input
pub(crate) fn take_replacement() -> Option<FieldState> {
    shared().lock().ok()?.replacement.take()
}

/// Drop a replacement the field never applied (it lost the focus, or it is not
/// drawn by `Ui::input`), so the bridge can mirror again.
pub(crate) fn clear_replacement() {
    if let Ok(mut shared) = shared().lock() {
        shared.replacement = None;
    }
}
