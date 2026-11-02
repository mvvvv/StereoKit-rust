//! Android soft keyboard stepper: bridges the Android IME to StereoKit's
//! keyboard event queue through a hidden 1x1 px `EditText` (Quest/Meta devices).
//!
//! When [`Platform::is_keyboard_visible`] turns true, StereoKit's virtual
//! keyboard is dismissed and the Android IME is asked to show instead. The
//! hidden `EditText` lives until the text field loses focus, the app goes to the
//! background, or the stepper is removed.
//!
//! Register it once at startup on Android:
//! ```ignore
//! sk.send_event(StepperAction::add(ANDROID_SOFT_KBD_ID, AndroidSoftKbd::default()));
//! ```
//!
//! The [`ControlPanel`](crate::tools::control_panel::ControlPanel) does it by default on Android (its "Android
//! Keyboard" toggle, see `ControlPanel::set_android_keyboard`), so an app that draws the panel has nothing to do.

use android_activity::AndroidApp;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    prelude::*,
    sk::{AppFocus, sk_app_focus},
    system::TextContext,
    tools::ime_bridge::{self, FieldState},
    ui::Ui,
    util::Platform,
};

/// Stepper ID for [`AndroidSoftKbd`].
pub const ANDROID_SOFT_KBD_ID: &str = "Tool_AndroidSoftKbdID";

/// Frames a replacement may wait for the field to apply it before it is
/// dropped (the field lost the focus, or it is not drawn by `Ui::input`).
const IME_PENDING_FRAMES: u32 = 60;

/// Frames a composing text may stay unchanged before it is published anyway
/// (~1 s at 90 Hz): a keyboard that never ends its composition must not lock
/// the field.
const IME_COMPOSING_FRAMES: u32 = 90;

/// Frames between two `showSoftInput` requests (~0.11 s at 90 Hz). The first
/// attempt of a cycle may only re-focus the view, hence the short interval.
const IME_SHOW_RETRY_FRAMES: u32 = 10;

/// Frames before giving up on `showSoftInput` (~1.5 s at 90 Hz).
const IME_SHOW_GIVE_UP_FRAMES: u32 = 120;

/// Frames without StereoKit UI keyboard focus before the session is closed
/// (~0.15 s at 90 Hz). Closing on focus loss is what makes the IME come back
/// on the next visit of the text field.
const IME_UNFOCUSED_FRAMES: u32 = 10;

/// `InputMethodManager.SHOW_FORCED`: show the IME even if Android thinks it is
/// already shown.
const IME_SHOW_FLAGS: i32 = 2;

/// `InputMethodManager.SHOW_IMPLICIT`, used for the decor view request.
const IME_SHOW_IMPLICIT: i32 = 1;

/// `InputType.TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE`.
///
/// The suggestion strip is *not* disabled (`TYPE_TEXT_FLAG_NO_SUGGESTIONS`):
/// advanced IME features (voice input, translation, autocorrect) live there,
/// and the text exchange below handles the composing text they produce.
const EDIT_TEXT_INPUT_TYPE: i32 = 0x1 | 0x0002_0000;

/// `EditorInfo.IME_FLAG_NO_EXTRACT_UI | IME_FLAG_NO_FULLSCREEN |
/// IME_FLAG_NO_ENTER_ACTION`, with `IME_ACTION_NONE` in the action bits.
///
/// `IME_FLAG_NO_ENTER_ACTION` is what tells the IME to show a plain Enter
/// (newline) key instead of an action button (Send, Done…). Without it, some
/// keyboards — Gboard in spatial mode, the Android XR one — show no Enter key at
/// all for a multi-line editor.
const EDIT_TEXT_IME_OPTIONS: i32 = 0x1 | 0x4000_0000 | 0x1000_0000 | 0x0200_0000;

/// A mirror write queued on the Java main thread: the state the hidden view
/// holds when the write is decided (`from`) and the state it is being given
/// (`to`). The view keeps `from` until the Java thread runs, and the poll must
/// not take that for an IME edit.
#[derive(Clone)]
struct MirrorWrite {
    from: FieldState,
    to: FieldState,
}

/// State of the hidden `EditText`, as read each frame.
struct ImeSample {
    /// Text and caret of the hidden view.
    state: FieldState,
    /// The IME is composing: its text is provisional (autocorrect, dictation or
    /// translation preview).
    composing: bool,
}

/// `EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING`: keep what is typed out of the
/// IME's learning dictionary (used for password fields).
const IME_FLAG_NO_PERSONALIZED_LEARNING: i32 = 0x0100_0000;

#[derive(Default)]
struct ShowOutcome {
    /// `InputMethodManager.showSoftInput` accepted the request for our view.
    shown: bool,
    /// `InputMethodManager.showSoftInput` accepted the request for the decor
    /// view. On a `NativeActivity` this is the view IMM serves, and the IME then
    /// sends its key events to the focused view, i.e. our hidden `EditText`.
    shown_root: bool,
    /// `WindowInsetsController.show(ime())` was called (API 30+). Unlike
    /// `showSoftInput` it reports no result, so it is never treated as a success.
    insets: bool,
    /// The view is the served view of `InputMethodManager`.
    served: bool,
    /// The view holds the keyboard focus after the repair attempt.
    has_focus: bool,
    /// The view's window has the focus (a VR shell may hold it).
    window_focus: bool,
}

/// Stepper bridging the Android IME to StereoKit's keyboard event queue.
///
/// Checked every frame against [`Platform::is_keyboard_visible`]: the SK
/// keyboard is replaced by the Android IME, and each new request simply asks
/// the IME again.
#[derive(IStepper)]
pub struct AndroidSoftKbd {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    /// `AndroidApp` clone, stored at `start()` time.
    android_app: Option<AndroidApp>,

    /// Raw JNI global ref to the hidden `EditText`:
    /// `0` = absent, `1` = creation in progress, `>1` = valid jobject.
    edit_text: Arc<Mutex<usize>>,

    /// Last state of the hidden `EditText` that was exchanged with the field:
    /// the reference both sides converge to (`EditText` → field, or field →
    /// `EditText`).
    ime_state: Option<FieldState>,

    /// Frames a replacement has been waiting for the field to apply it.
    pending_frames: u32,

    /// Last composing state seen, and for how many frames it has not changed.
    composing_state: Option<FieldState>,
    composing_frames: u32,

    /// Kind of text the hidden `EditText` is configured for.
    input_context: Option<TextContext>,

    /// State written into the hidden `EditText` but not applied yet by the Java
    /// main thread, and for how many frames it has been pending.
    mirror_in_flight: Option<MirrorWrite>,
    mirror_frames: u32,

    /// The hidden `EditText` could not be read on the previous frame: only the
    /// transitions are logged, so a failing read doesn't flood the log.
    read_error: bool,

    /// The IME was asked to show but nothing proves it is displayed yet.
    show_pending: bool,

    /// Frames spent in the current `show_pending` cycle.
    show_frames: u32,

    /// Frames left before repeating the `showSoftInput` request.
    show_cooldown: u32,

    /// `showSoftInput` requests sent and accepted in the current cycle. A
    /// refused request is the only failure signal `InputMethodManager` gives.
    show_requests: Arc<AtomicU32>,
    show_accepted: Arc<AtomicU32>,

    /// A session is running: the `EditText` is alive and the IME was asked.
    ime_open: bool,

    /// The IME produced input since the current cycle started: it is displayed.
    /// Cleared by [`Self::arm_show_ime`], otherwise a new visit of the text
    /// field would reuse the previous proof and never ask again.
    ime_seen_input: bool,

    /// [`Ui::has_keyboard_focus`] was observed: losing it means the user left
    /// the text field.
    ime_had_sk_focus: bool,

    /// Consecutive frames without StereoKit UI keyboard focus.
    ime_unfocused_frames: u32,

    /// The IME could not be opened: keep StereoKit's keyboard until it closes.
    ime_unavailable: bool,

    /// Set by `close()`: enables the `IStepper::shutdown` path of the derive.
    shutdown_completed: bool,
}

unsafe impl Send for AndroidSoftKbd {}

impl Default for AndroidSoftKbd {
    fn default() -> Self {
        Self {
            id: ANDROID_SOFT_KBD_ID.to_string(),
            sk_info: None,

            android_app: None,
            edit_text: Arc::new(Mutex::new(0)),
            ime_state: None,
            pending_frames: 0,
            composing_state: None,
            composing_frames: 0,
            input_context: None,
            mirror_in_flight: None,
            mirror_frames: 0,
            read_error: false,
            show_pending: false,
            show_frames: 0,
            show_cooldown: 0,
            show_requests: Arc::new(AtomicU32::new(0)),
            show_accepted: Arc::new(AtomicU32::new(0)),
            ime_open: false,
            ime_seen_input: false,
            ime_had_sk_focus: false,
            ime_unfocused_frames: 0,
            ime_unavailable: false,
            shutdown_completed: false,
        }
    }
}

/// Methods called by the `IStepper` derive macro; all run on the main thread.
impl AndroidSoftKbd {
    /// Called from `IStepper::initialize`.
    fn start(&mut self) -> bool {
        if self.id != ANDROID_SOFT_KBD_ID {
            Log::err(format!("AndroidSoftKbd wrong Unique ID, expected {}, got {}", ANDROID_SOFT_KBD_ID, self.id));
            return false;
        }
        let sk_i = self.sk_info.as_ref().expect("sk_info should be Some").borrow();
        self.android_app = Some(sk_i.get_android_app().clone());
        // `Ui::input` starts exchanging its text with the hidden `EditText`.
        ime_bridge::set_active(true);
        true
    }

    /// Called from `IStepper::step` to handle events — nothing to do here.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from `IStepper::step` every frame.
    fn draw(&mut self, _token: &MainThreadToken) {
        // Clone to avoid holding a borrow on self while calling methods.
        let app = match self.android_app.clone() {
            Some(a) => a,
            None => return,
        };

        self.poll_input_events(&app);

        let kb_visible = Platform::is_keyboard_visible();
        let focus = unsafe { sk_app_focus() };

        if self.ime_open {
            // The user left the text field: release the IME and the `EditText`
            // so the next visit starts from a clean session.
            if Ui::has_keyboard_focus() {
                self.ime_had_sk_focus = true;
                self.ime_unfocused_frames = 0;
            } else if self.ime_had_sk_focus {
                self.ime_unfocused_frames += 1;
                if self.ime_unfocused_frames > IME_UNFOCUSED_FRAMES {
                    Log::diag("AndroidSoftKbd: text field lost focus, closing the IME");
                    self.close_session(&app);
                    return;
                }
            }
            // The app asked for a keyboard again: hide SK's, ask the IME.
            if kb_visible {
                Platform::keyboard_show(false, TextContext::Text);
                self.arm_show_ime();
            }
            // The system IME takes the focus when displayed, and it is
            // displayed once it produced input. `InputMethodManager.isActive`
            // is not a display signal (true as soon as the view is focused).
            let requested = self.show_requests.load(Ordering::Acquire) > 0;
            if self.show_pending && (self.ime_seen_input || (requested && focus != AppFocus::Active)) {
                self.show_pending = false;
                self.show_frames = 0;
                self.show_cooldown = 0;
                Log::info("AndroidSoftKbd: the Android IME is up");
            }
            if focus == AppFocus::Hidden {
                // Window and IME connection are gone.
                self.close_session(&app);
            }
        } else if kb_visible {
            // After a failed attempt, StereoKit's keyboard keeps this input.
            if !self.ime_unavailable {
                Platform::keyboard_show(false, TextContext::Text);
                self.open_session(&app);
            }
        } else {
            // No input requested: allow a new IME attempt next time.
            self.ime_unavailable = false;
        }

        if self.show_pending {
            self.pump_show_ime(&app);
        }
    }

    // ---------------------------------------------------------------------
    // IME session
    // ---------------------------------------------------------------------

    /// Create the hidden `EditText` (once) and ask for the IME.
    ///
    /// The view is **not** recreated per session: removing the view that IMM
    /// serves leaves `InputMethodManager` in a state where it refuses every
    /// later request ("is not served"), even for a brand new view. It is only
    /// destroyed when it lost its window (app resume) or with the stepper.
    fn open_session(&mut self, app: &AndroidApp) {
        if self.edit_text_attached() == Some(false) {
            self.destroy_edit_text(app);
        }
        self.create_edit_text(app);
        self.ime_open = true;
        self.ime_seen_input = false;
        self.ime_had_sk_focus = false;
        self.ime_unfocused_frames = 0;
        // A new `EditText` starts empty: the first exchange mirrors the field
        // content into it (never the other way round).
        self.ime_state = None;
        self.pending_frames = 0;
        self.composing_state = None;
        self.composing_frames = 0;
        self.input_context = None;
        self.mirror_in_flight = None;
        self.mirror_frames = 0;
        self.arm_show_ime();
    }

    /// Hide the IME and release the session state, keeping the `EditText` alive.
    fn close_session(&mut self, app: &AndroidApp) {
        self.hide_ime(app);
        self.ime_open = false;
        self.ime_seen_input = false;
        self.ime_had_sk_focus = false;
        self.ime_unfocused_frames = 0;
        self.show_pending = false;
        self.show_frames = 0;
        self.show_cooldown = 0;
        self.ime_state = None;
        self.pending_frames = 0;
        self.composing_state = None;
        self.composing_frames = 0;
        self.input_context = None;
        self.mirror_in_flight = None;
        self.mirror_frames = 0;
        // A replacement the field never applied must not land in a later
        // session.
        ime_bridge::clear_replacement();
    }

    /// Hide the IME of the hidden `EditText` and give the window its normal
    /// focus behaviour back.
    fn hide_ime(&self, app: &AndroidApp) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return;
        }
        let app2 = app.clone();
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = activity_ref(env, &app2)?;
                let edit_text = local_ref(env, raw)?;

                let window_token = env
                    .call_method(&edit_text, jni_str!("getWindowToken"), jni_sig!("()Landroid/os/IBinder;"), &[])?
                    .l()?;
                if !window_token.is_null() {
                    let im_manager = input_method_manager(env, &activity)?;
                    env.call_method(
                        &im_manager,
                        jni_str!("hideSoftInputFromWindow"),
                        jni_sig!("(Landroid/os/IBinder;I)Z"),
                        &[JValue::Object(&window_token), 0i32.into()],
                    )?;
                }
                env.call_method(&edit_text, jni_str!("clearFocus"), jni_sig!("()V"), &[])?;
                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: hide IME JNI error: {e:?}"));
            }
        }));
    }

    /// Start (or restart) a cycle of `showSoftInput` requests.
    fn arm_show_ime(&mut self) {
        self.show_pending = true;
        self.show_frames = 0;
        self.show_cooldown = 0;
        self.show_requests.store(0, Ordering::Release);
        self.show_accepted.store(0, Ordering::Release);
        // The previous cycle's proof does not apply to this one.
        self.ime_seen_input = false;
    }

    /// Ask for the IME once the `EditText` is attached to the window.
    ///
    /// The first attempt of a cycle may only re-focus the view (see
    /// [`Self::request_show_ime`]), so the request is repeated every
    /// [`IME_SHOW_RETRY_FRAMES`] until it is accepted or the budget is exhausted.
    fn pump_show_ime(&mut self, app: &AndroidApp) {
        self.show_frames += 1;

        if self.edit_text_attached().is_some_and(|attached| attached) {
            if self.show_cooldown == 0 {
                self.request_show_ime(app, self.show_frames <= 1);
                self.show_cooldown = IME_SHOW_RETRY_FRAMES;
            } else {
                self.show_cooldown -= 1;
            }
        }

        if self.show_frames > IME_SHOW_GIVE_UP_FRAMES {
            self.show_pending = false;
            self.show_frames = 0;
            self.show_cooldown = 0;
            if self.show_requests.load(Ordering::Acquire) == 0 || self.show_accepted.load(Ordering::Acquire) == 0 {
                // Every request was refused: the IME can not serve our view.
                Log::err("AndroidSoftKbd: could not open the Android IME, falling back to the StereoKit keyboard");
                self.close_session(app);
                self.ime_unavailable = true;
                Platform::keyboard_show(true, TextContext::Text);
            } else {
                Log::warn("AndroidSoftKbd: the IME accepted the request but no input was seen");
            }
        }
    }

    /// Post an IME request for the hidden `EditText`.
    ///
    /// `restart` resets the input connection (first request of a cycle).
    ///
    /// Three paths are tried, because `showSoftInput` only works for the view
    /// `InputMethodManager` *serves* — on a `NativeActivity` that is the decor
    /// view, so our child is often refused:
    /// 1. `showSoftInput(hidden EditText)`: gives an `InputConnection`, the best
    ///    path when IMM targets our view.
    /// 2. `showSoftInput(root view)`: always served, the IME shows and sends its
    ///    key events to the focused view (our `EditText`).
    /// 3. `WindowInsetsController.show(ime)` (API 30+): window-manager side, no
    ///    result to report, tried once per cycle.
    fn request_show_ime(&self, app: &AndroidApp, restart: bool) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return;
        }
        let app2 = app.clone();
        let show_requests = Arc::clone(&self.show_requests);
        let show_accepted = Arc::clone(&self.show_accepted);
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<ShowOutcome> {
                let activity = activity_ref(env, &app2)?;
                let edit_text = local_ref(env, raw)?;
                let im_manager = input_method_manager(env, &activity)?;

                let served = env
                    .call_method(
                        &im_manager,
                        jni_str!("isActive"),
                        jni_sig!("(Landroid/view/View;)Z"),
                        &[JValue::Object(&edit_text)],
                    )?
                    .z()?;
                if !served {
                    // Focus events reach IMM through its own handler, so this
                    // pass repairs the focus and the request is for the next one.
                    env.call_method(&edit_text, jni_str!("clearFocus"), jni_sig!("()V"), &[])?;
                    env.call_method(&edit_text, jni_str!("requestFocus"), jni_sig!("()Z"), &[])?;
                }

                if restart {
                    env.call_method(
                        &im_manager,
                        jni_str!("restartInput"),
                        jni_sig!("(Landroid/view/View;)V"),
                        &[JValue::Object(&edit_text)],
                    )?;
                }
                let shown = env
                    .call_method(
                        &im_manager,
                        jni_str!("showSoftInput"),
                        jni_sig!("(Landroid/view/View;I)Z"),
                        &[JValue::Object(&edit_text), IME_SHOW_FLAGS.into()],
                    )?
                    .z()?;
                let root = env
                    .call_method(&edit_text, jni_str!("getRootView"), jni_sig!("()Landroid/view/View;"), &[])?
                    .l()?;
                // On a `NativeActivity` IMM serves the decor view: asking for it
                // still brings the keyboard up, and without an `InputConnection`
                // the IME then sends key events to the focused view, i.e. our
                // hidden `EditText`.
                let shown_root = if shown {
                    false
                } else {
                    env.call_method(
                        &im_manager,
                        jni_str!("showSoftInput"),
                        jni_sig!("(Landroid/view/View;I)Z"),
                        &[JValue::Object(&root), IME_SHOW_IMPLICIT.into()],
                    )?
                    .z()?
                };
                // The window-manager path has no result: last resort, once per
                // cycle, and never treated as a success.
                let insets = if shown || shown_root || !restart {
                    false
                } else {
                    match show_ime_via_insets(env, &root) {
                        Ok(attempted) => attempted,
                        Err(_) => {
                            // Never leave a pending Java exception behind.
                            env.exception_clear();
                            false
                        }
                    }
                };
                let has_focus = env.call_method(&edit_text, jni_str!("hasFocus"), jni_sig!("()Z"), &[])?.z()?;
                let window_focus =
                    env.call_method(&edit_text, jni_str!("hasWindowFocus"), jni_sig!("()Z"), &[])?.z()?;

                Ok(ShowOutcome { shown, shown_root, insets, served, has_focus, window_focus })
            });
            match result {
                Ok(outcome) => {
                    show_requests.fetch_add(1, Ordering::AcqRel);
                    if outcome.shown || outcome.shown_root {
                        show_accepted.fetch_add(1, Ordering::AcqRel);
                    } else {
                        // Only the refusals are worth a line: they carry the state
                        // needed to understand why the IME does not come up.
                        Log::diag(format!(
                            "AndroidSoftKbd: ime request refused (served={}, focus={}, windowFocus={}, decor={}, insets={})",
                            outcome.served, outcome.has_focus, outcome.window_focus, outcome.shown_root, outcome.insets
                        ));
                    }
                }
                Err(e) => Log::err(format!("AndroidSoftKbd: showSoftInput JNI error: {e:?}")),
            }
        }));
    }

    /// Is the hidden `EditText` attached to the window? `None` when absent.
    ///
    /// Attachment is the prerequisite for the view to take the focus, hence to
    /// become the served view of `InputMethodManager`.
    fn edit_text_attached(&self) -> Option<bool> {
        use jni::{jni_sig, jni_str};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return None;
        }
        let app = self.android_app.as_ref()?;

        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as _) };
        vm.attach_current_thread(|env| -> jni::errors::Result<bool> {
            let edit_text = local_ref(env, raw)?;
            env.call_method(&edit_text, jni_str!("isAttachedToWindow"), jni_sig!("()Z"), &[])?.z()
        })
        .ok()
    }

    /// Read the hidden `EditText` state: its text, its caret and whether the IME
    /// is composing (in which case the text is provisional). `None` if the view
    /// is absent or unreadable.
    ///
    /// The text is decoded by [`read_editable_text`], which tolerates an unpaired
    /// UTF-16 surrogate (a Java-to-Rust conversion would fail on it and leave the
    /// bridge blind for good).
    fn poll_edit_text(&self) -> Option<ImeSample> {
        use jni::{jni_sig, jni_str, objects::JValue};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return None;
        }
        let app = self.android_app.as_ref()?;

        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as _) };
        vm.attach_current_thread(|env| -> jni::errors::Result<ImeSample> {
            // The references created below (view, text, char array, class…) live
            // in a local frame popped at the end: this runs on every frame, and
            // an attached thread does not free them by itself.
            env.with_local_frame(24, |env| {
                let edit_text = local_ref(env, raw)?;

                let editable = env
                    .call_method(&edit_text, jni_str!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?
                    .l()?;
                // Never fails on the encoding: see `read_editable_text`.
                let text = read_editable_text(env, &editable)?;

                let caret_utf16 =
                    env.call_method(&edit_text, jni_str!("getSelectionStart"), jni_sig!("()I"), &[])?.i()?;
                // Java counts UTF-16 units, StereoKit counts characters.
                let caret = utf16_to_chars(&text, caret_utf16);

                // `BaseInputConnection.getComposingSpanStart/End(Editable)`: a
                // non-negative range means the IME is composing (autocorrect,
                // dictation, translation preview), so the text is not final yet.
                let class_bic = env.find_class(jni_str!("android/view/inputmethod/BaseInputConnection"))?;
                let composing_start = env
                    .call_static_method(
                        &class_bic,
                        jni_str!("getComposingSpanStart"),
                        jni_sig!("(Landroid/text/Spannable;)I"),
                        &[JValue::Object(&editable)],
                    )?
                    .i()?;
                let composing_end = env
                    .call_static_method(
                        &class_bic,
                        jni_str!("getComposingSpanEnd"),
                        jni_sig!("(Landroid/text/Spannable;)I"),
                        &[JValue::Object(&editable)],
                    )?
                    .i()?;

                Ok(ImeSample {
                    state: FieldState { text, caret },
                    composing: composing_start >= 0 && composing_end >= 0,
                })
            })
        })
        .ok()
    }

    /// Create the hidden `EditText` on the Java UI thread (asynchronous).
    ///
    /// The IME is never asked here: `pump_show_ime` does it once the view is
    /// attached.
    fn create_edit_text(&mut self, app: &AndroidApp) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let state = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if state != 0 {
            // Already created, or being created.
            return;
        }
        *self.edit_text.lock().expect("Failed to lock edit_text mutex") = 1;

        let app2 = app.clone();
        let edit_text_ref = Arc::clone(&self.edit_text);
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = activity_ref(env, &app2)?;

                let jni_window =
                    env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
                let decor_view = env
                    .call_method(&jni_window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?
                    .l()?;

                // ---- create EditText(activity) ----
                let edit_text = env.new_object(
                    jni_str!("android/widget/EditText"),
                    jni_sig!("(Landroid/content/Context;)V"),
                    &[JValue::Object(&activity)],
                )?;

                // VISIBLE so it CAN receive focus (INVISIBLE/GONE cannot), with
                // alpha 0 so that it stays invisible to the user.
                env.call_method(&edit_text, jni_str!("setVisibility"), jni_sig!("(I)V"), &[0i32.into()])?;
                env.call_method(&edit_text, jni_str!("setAlpha"), jni_sig!("(F)V"), &[0.0f32.into()])?;
                env.call_method(&edit_text, jni_str!("setFocusable"), jni_sig!("(Z)V"), &[true.into()])?;
                env.call_method(&edit_text, jni_str!("setFocusableInTouchMode"), jni_sig!("(Z)V"), &[true.into()])?;
                env.call_method(
                    &edit_text,
                    jni_str!("setInputType"),
                    jni_sig!("(I)V"),
                    &[EDIT_TEXT_INPUT_TYPE.into()],
                )?;
                env.call_method(
                    &edit_text,
                    jni_str!("setImeOptions"),
                    jni_sig!("(I)V"),
                    &[EDIT_TEXT_IME_OPTIONS.into()],
                )?;

                // Explicit LayoutParams: without them the view is never
                // measured and stays at 0x0.
                let layout_params = env.new_object(
                    jni_str!("android/view/ViewGroup$LayoutParams"),
                    jni_sig!("(II)V"),
                    &[1i32.into(), 1i32.into()],
                )?;
                env.call_method(
                    &decor_view,
                    jni_str!("addView"),
                    jni_sig!("(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"),
                    &[JValue::Object(&edit_text), JValue::Object(&layout_params)],
                )?;
                let _ = env.call_method(&edit_text, jni_str!("requestFocus"), jni_sig!("()Z"), &[])?;

                // The view starts empty: the first exchange of the session
                // mirrors the focused field text into it.
                //
                // `into_raw` transfers the ownership: the reference is released
                // when the view is destroyed (`destroy_edit_text`).
                let raw = env.new_global_ref(&edit_text)?.into_raw() as usize;
                *edit_text_ref.lock().expect("Failed to lock edit_text mutex") = raw;

                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: JNI failed: {e:?}"));
                *edit_text_ref.lock().expect("Failed to lock edit_text mutex") = 0;
            }
        }));
    }

    /// Called from `IStepper::shutdown` when the stepper is removed: close the
    /// session and release the hidden `EditText`.
    fn close(&mut self, _triggering: bool) -> bool {
        // `Ui::input` stops exchanging text with the hidden `EditText`.
        ime_bridge::set_active(false);
        let has_view = *self.edit_text.lock().expect("Failed to lock edit_text mutex") > 1;
        if let Some(app) = self.android_app.clone().filter(|_| self.ime_open || has_view) {
            self.close_session(&app);
        }
        self.shutdown_completed = true;
        self.shutdown_completed
    }

    /// Remove the hidden `EditText` from the window and release its JNI global
    /// reference.
    ///
    /// Only used when the view can not be kept: the app went to the background
    /// (the window and the view are gone) or the stepper is removed. Removing
    /// the view IMM serves makes it refuse every later request, so a session
    /// normally keeps its `EditText`.
    fn destroy_edit_text(&mut self, app: &AndroidApp) {
        use jni::{jni_sig, jni_str, objects::JValue};

        self.hide_ime(app);
        let raw = std::mem::replace(&mut *self.edit_text.lock().expect("Failed to lock edit_text mutex"), 0);
        if raw <= 1 {
            return;
        }
        let app2 = app.clone();
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = activity_ref(env, &app2)?;
                let edit_text = local_ref(env, raw)?;

                // Release the global reference first: the local reference above
                // is enough for the calls below, so an exception on `removeView`
                // can not leak it.
                let _drop_global = unsafe { env.global_from_raw::<jni::objects::JObject>(raw as jni::sys::jobject) };

                let jni_window =
                    env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
                let decor_view = env
                    .call_method(&jni_window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?
                    .l()?;
                env.call_method(
                    &decor_view,
                    jni_str!("removeView"),
                    jni_sig!("(Landroid/view/View;)V"),
                    &[JValue::Object(&edit_text)],
                )?;

                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: EditText removal JNI error: {e:?}"));
            }
        }));
    }

    /// Exchange the hidden `EditText` state with the focused `Ui::input`.
    ///
    /// The IME edits the *real* text of the field (mirrored into the hidden
    /// view), so an IME change is published as a whole `(text, caret)`
    /// replacement that [`Ui::input`] applies: translation, dictation,
    /// autocorrect and caret moves all come through as they are, with no per
    /// character injection and no size limit.
    ///
    /// While the IME composes, its text is provisional and nothing is
    /// published. The other way round, when StereoKit edited the text (its own
    /// keyboard, or the application assigning the buffer), the field state is
    /// written back into the hidden `EditText`.
    ///
    /// Writing into the `EditText` happens on the Java main thread, so the view
    /// keeps the old text for a frame or two: [`Self::mirror_in_flight`] marks
    /// those states, otherwise the old text would be taken for an IME edit and
    /// published back to the field (which is how a backspace could be undone).
    ///
    /// [`Ui::input`]: crate::ui::Ui::input
    fn poll_input_events(&mut self, app: &AndroidApp) {
        // Without a focused `Ui::input` there is nothing to mirror, and the
        // published field state would be stale.
        if !Ui::has_keyboard_focus() {
            return;
        }
        let Some(ime) = self.poll_edit_text() else {
            if !self.read_error {
                self.read_error = true;
                Log::diag("AndroidSoftKbd: the hidden EditText is unreadable, retrying every frame");
            }
            // The hidden view can stay unreadable for a moment (a JNI error, a
            // view being recreated): a mirror write waiting for it must not
            // block the bridge forever.
            if self.mirror_in_flight.is_some() {
                if self.mirror_frames < IME_PENDING_FRAMES {
                    self.mirror_frames += 1;
                } else {
                    Log::diag("AndroidSoftKbd: the EditText stayed unreadable, dropping the pending mirror write");
                    self.mirror_in_flight = None;
                    self.mirror_frames = 0;
                }
            }
            return;
        };

        if self.read_error {
            self.read_error = false;
            Log::diag("AndroidSoftKbd: the hidden EditText is readable again");
        }

        // Provisional text (autocorrect, dictation, translation preview): wait
        // for the commit. A keyboard that never ends its composition would lock
        // the field, so a text that stopped moving is published anyway.
        if ime.composing {
            let stable = self.composing_state.as_ref() == Some(&ime.state);
            self.composing_frames = if stable { self.composing_frames + 1 } else { 0 };
            self.composing_state = Some(ime.state.clone());
            if self.composing_frames < IME_COMPOSING_FRAMES {
                return;
            }
        } else {
            self.composing_state = None;
            self.composing_frames = 0;
        }

        // Keep the keyboard type in sync with the kind of text the field holds
        // (password masking, numeric or URI keyboards…).
        if let Some(context) = ime_bridge::field_context() {
            self.apply_context(app, context);
        }

        // A mirror write is on its way to the Java thread: the view may still
        // hold the old text, which must not be taken for an IME edit.
        if let Some(write) = self.mirror_in_flight.clone() {
            if ime.state == write.to {
                // Applied.
                self.mirror_in_flight = None;
                self.mirror_frames = 0;
                self.ime_state = Some(ime.state);
                return;
            }
            if ime.state == write.from && self.mirror_frames < IME_PENDING_FRAMES {
                // Not applied yet: the view still holds the previous state.
                self.mirror_frames += 1;
                return;
            }
            // The IME typed meanwhile, or the write never landed: forget it.
            self.mirror_in_flight = None;
            self.mirror_frames = 0;
        }

        let Some(previous) = self.ime_state.as_ref() else {
            // First exchange of the session: the field content wins, otherwise
            // the empty `EditText` would wipe it.
            if let Some(field) = ime_bridge::field() {
                self.write_mirror(app, &field, true, &ime.state);
                self.ime_state = Some(field);
            }
            return;
        };

        // The IME moved: hand the whole state over to the field.
        if ime.state != *previous {
            ime_bridge::propose(ime.state.clone());
            self.ime_seen_input = true;
            self.ime_state = Some(ime.state);
            self.pending_frames = 0;
            return;
        }

        // A replacement still waiting for the field means it holds the old
        // text: writing that back now would undo the IME edit.
        if ime_bridge::has_replacement() {
            self.pending_frames += 1;
            if self.pending_frames > IME_PENDING_FRAMES {
                // Keep `ime_state`: the next IME edit proposes the whole text
                // again, so the two sides resynchronize by themselves.
                Log::diag("AndroidSoftKbd: the text field did not take the IME text, dropping it");
                ime_bridge::clear_replacement();
                self.pending_frames = 0;
            }
            return;
        }
        self.pending_frames = 0;

        // The field moved on its own: mirror it into the hidden `EditText`.
        let mirror = {
            let Some(previous) = self.ime_state.as_ref() else { return };
            ime_bridge::field()
                .filter(|field| *field != *previous)
                .map(|field| (field.text != previous.text, previous.clone(), field))
        };
        if let Some((set_text, from, field)) = mirror {
            self.write_mirror(app, &field, set_text, &from);
            self.ime_state = Some(field);
        }
    }

    /// Configure the hidden `EditText` for the kind of text the field holds:
    /// password masking, numeric or URI keyboards, plain text otherwise.
    fn apply_context(&mut self, app: &AndroidApp, context: TextContext) {
        use jni::{jni_sig, jni_str};

        if self.input_context == Some(context) {
            return;
        }
        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return;
        }
        self.input_context = Some(context);

        let app2 = app.clone();
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let edit_text = local_ref(env, raw)?;
                env.call_method(
                    &edit_text,
                    jni_str!("setInputType"),
                    jni_sig!("(I)V"),
                    &[input_type_of(context).into()],
                )?;
                env.call_method(
                    &edit_text,
                    jni_str!("setImeOptions"),
                    jni_sig!("(I)V"),
                    &[ime_options_of(context).into()],
                )?;
                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: EditText keyboard type JNI error: {e:?}"));
            }
        }));
    }

    /// Write a field state into the hidden `EditText` (on the Java main thread),
    /// so the IME sees the text and the caret StereoKit has.
    fn write_mirror(&mut self, app: &AndroidApp, field: &FieldState, set_text: bool, from: &FieldState) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return;
        }
        let app2 = app.clone();
        let field = field.clone();
        let expected = from.text.clone();
        self.mirror_in_flight = Some(MirrorWrite { from: from.clone(), to: field.clone() });
        self.mirror_frames = 0;
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let edit_text = local_ref(env, raw)?;

                // Conditional write: the IME may have typed since this write was
                // queued, and overwriting would lose its input.
                if set_text {
                    let editable = env
                        .call_method(&edit_text, jni_str!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?
                        .l()?;
                    if read_editable_text(env, &editable)? != expected {
                        return Ok(());
                    }
                    let text = env.new_string(&field.text)?;
                    env.call_method(
                        &edit_text,
                        jni_str!("setText"),
                        jni_sig!("(Ljava/lang/CharSequence;)V"),
                        &[JValue::Object(&text)],
                    )?;
                }
                let caret = chars_to_utf16(&field.text, field.caret);
                env.call_method(
                    &edit_text,
                    jni_str!("setSelection"),
                    jni_sig!("(II)V"),
                    &[caret.into(), caret.into()],
                )?;
                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: EditText mirror JNI error: {e:?}"));
            }
        }));
    }
}

/// Borrow a raw JNI global reference as a local reference of the current frame.
fn local_ref<'local>(env: &mut jni::Env<'local>, raw: usize) -> jni::errors::Result<jni::objects::JObject<'local>> {
    let object = unsafe { jni::objects::JObject::from_raw(env, raw as jni::sys::jobject) };
    env.new_local_ref(&object)
}

/// Read the text of the hidden `EditText` as a Rust `String`, **without ever
/// failing on its encoding**.
///
/// The IME can leave an unpaired UTF-16 surrogate in the text (one half of an
/// emoji, a dictation inserting a code point in two steps, a caret moved inside
/// a pair…), and the Java-to-Rust string conversion rejects it: `JString` is
/// decoded as modified UTF-8, which has no representation for a lone surrogate,
/// so every read used to fail with a `DecodingError` — the mirror then stopped
/// for good and the failure was logged on every frame.
///
/// The text is read as UTF-16 code units (`String.getChars`, the very units
/// Android counts for the caret) and decoded with [`String::from_utf16_lossy`]:
/// a lone surrogate becomes `U+FFFD` and the mirror keeps working. The unit
/// count never changes, so the caret conversions stay valid.
fn read_editable_text(env: &mut jni::Env<'_>, editable: &jni::objects::JObject<'_>) -> jni::errors::Result<String> {
    use jni::{jni_sig, jni_str, objects::JValue};

    let text = env.call_method(editable, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
    let length = env.call_method(&text, jni_str!("length"), jni_sig!("()I"), &[])?.i()?;
    if length <= 0 {
        return Ok(String::new());
    }
    let units = env.new_char_array(length as usize)?;
    env.call_method(
        &text,
        jni_str!("getChars"),
        jni_sig!("(II[CI)V"),
        &[0i32.into(), length.into(), JValue::Object(&units), 0i32.into()],
    )?;
    let mut buffer = vec![0u16; length as usize];
    units.get_region(env, 0, &mut buffer)?;
    Ok(String::from_utf16_lossy(&buffer))
}

/// Android `InputType` of a StereoKit `TextContext`.
fn input_type_of(context: TextContext) -> i32 {
    let bits = context.bits();
    if bits == TextContext::Number.bits() {
        0x2 // TYPE_CLASS_NUMBER
    } else if bits == TextContext::Uri.bits() {
        0x1 | 0x10 // TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_URI
    } else if bits == TextContext::Password.bits() {
        0x1 | 0x80 // TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD
    } else {
        EDIT_TEXT_INPUT_TYPE
    }
}

/// `EditorInfo.imeOptions` of a StereoKit `TextContext`.
fn ime_options_of(context: TextContext) -> i32 {
    if context.bits() == TextContext::Password.bits() {
        // Keep the password out of the IME's learning dictionary.
        EDIT_TEXT_IME_OPTIONS | IME_FLAG_NO_PERSONALIZED_LEARNING
    } else {
        EDIT_TEXT_IME_OPTIONS
    }
}

/// Convert a Java caret offset (UTF-16 units) to a StereoKit caret (characters).
fn utf16_to_chars(text: &str, offset: i32) -> i32 {
    let offset = offset.max(0) as usize;
    let mut units = 0;
    for (index, character) in text.chars().enumerate() {
        if units >= offset {
            return index as i32;
        }
        units += character.len_utf16();
    }
    text.chars().count() as i32
}

/// Convert a StereoKit caret (characters) to a Java caret offset (UTF-16 units).
fn chars_to_utf16(text: &str, caret: i32) -> i32 {
    text.chars().take(caret.max(0) as usize).map(char::len_utf16).sum::<usize>() as i32
}

/// Local reference to the activity behind `app`.
fn activity_ref<'local>(
    env: &mut jni::Env<'local>,
    app: &AndroidApp,
) -> jni::errors::Result<jni::objects::JObject<'local>> {
    let activity = unsafe { jni::objects::JObject::from_raw(env, app.activity_as_ptr() as _) };
    env.new_local_ref(&activity)
}

/// Ask the window manager to show the IME for the window of `root`
/// (`WindowInsetsController.show(WindowInsets.Type.ime())`, API 30+).
///
/// Unlike `InputMethodManager.showSoftInput`, this path does not depend on
/// IMM's client-side served view. Returns `false` on older Android or when the
/// view has no insets controller.
fn show_ime_via_insets(env: &mut jni::Env<'_>, root: &jni::objects::JObject<'_>) -> jni::errors::Result<bool> {
    use jni::{jni_sig, jni_str};

    let class_version = env.find_class(jni_str!("android/os/Build$VERSION"))?;
    let sdk = env.get_static_field(class_version, jni_str!("SDK_INT"), jni_sig!("I"))?.i()?;
    if sdk < 30 {
        return Ok(false);
    }

    let controller = env
        .call_method(
            root,
            jni_str!("getWindowInsetsController"),
            jni_sig!("()Landroid/view/WindowInsetsController;"),
            &[],
        )?
        .l()?;
    if controller.is_null() {
        return Ok(false);
    }
    // WindowInsets.Type.ime() == 8
    env.call_method(&controller, jni_str!("show"), jni_sig!("(I)V"), &[8i32.into()])?;
    Ok(true)
}

/// Fetch the `InputMethodManager` of the activity (`Context.INPUT_METHOD_SERVICE`).
fn input_method_manager<'local>(
    env: &mut jni::Env<'local>,
    activity: &jni::objects::JObject<'local>,
) -> jni::errors::Result<jni::objects::JObject<'local>> {
    use jni::{jni_sig, jni_str};

    let class_ctxt = env.find_class(jni_str!("android/content/Context"))?;
    let ims = env.get_static_field(class_ctxt, jni_str!("INPUT_METHOD_SERVICE"), jni_sig!("Ljava/lang/String;"))?;
    let im_manager = env
        .call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[ims.borrow()],
        )?
        .l()?;
    Ok(im_manager)
}
