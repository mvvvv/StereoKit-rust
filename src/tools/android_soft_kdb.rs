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

use android_activity::AndroidApp;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    prelude::*,
    sk::{AppFocus, sk_app_focus},
    system::{Input, Key, TextContext},
    ui::Ui,
    util::Platform,
};

/// Stepper ID for [`AndroidSoftKbd`].
pub const ANDROID_SOFT_KBD_ID: &str = "Tool_AndroidSoftKbdID";

/// Idle content of the hidden `EditText`: two spaces with the caret in the
/// middle, so Del/Left/Right always have a character on each side.
const IME_BASELINE: &str = "  ";

/// Caret position of [`IME_BASELINE`].
const IME_BASELINE_CARET: i32 = 1;

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

/// `InputType.TYPE_CLASS_TEXT | TYPE_TEXT_FLAG_MULTI_LINE |
/// TYPE_TEXT_FLAG_NO_SUGGESTIONS`. Without `NO_SUGGESTIONS` the IME keeps a
/// composing region that it rewrites while typing, which the text diff below
/// would see as replacements.
const EDIT_TEXT_INPUT_TYPE: i32 = 0x1 | 0x0002_0000 | 0x0008_0000;

/// `EditorInfo.IME_FLAG_NO_EXTRACT_UI | IME_FLAG_NO_FULLSCREEN |
/// IME_FLAG_NO_ENTER_ACTION`, with `IME_ACTION_NONE` in the action bits.
///
/// `IME_FLAG_NO_ENTER_ACTION` is what tells the IME to show a plain Enter
/// (newline) key instead of an action button (Send, Done…). Without it, some
/// keyboards — Gboard in spatial mode, the Android XR one — show no Enter key at
/// all for a multi-line editor.
const EDIT_TEXT_IME_OPTIONS: i32 = 0x1 | 0x4000_0000 | 0x1000_0000 | 0x0200_0000;

/// Outcome of one IME request.
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

    /// A Java-main-thread callback resetting the `EditText` is in flight.
    reset_in_flight: Arc<AtomicBool>,

    /// The last reset was applied: the buffer is back to [`IME_BASELINE`],
    /// whatever `last_seen` still says.
    reset_applied: Arc<AtomicBool>,

    /// Last `(text, caret)` already turned into StereoKit events. It is the
    /// reference for the next diff, so a rewritten character (composing text,
    /// accent, autocorrect) is a replacement and not a second insertion.
    last_seen: Option<(String, i32)>,

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
            reset_in_flight: Arc::new(AtomicBool::new(false)),
            reset_applied: Arc::new(AtomicBool::new(false)),
            last_seen: None,
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
        self.last_seen = None;
        self.reset_in_flight.store(false, Ordering::Release);
        self.reset_applied.store(false, Ordering::Release);
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
        self.last_seen = None;
        self.reset_in_flight.store(false, Ordering::Release);
        self.reset_applied.store(false, Ordering::Release);
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

    /// Read the hidden `EditText` text and caret. `None` if the view is absent.
    fn poll_edit_text(&self) -> Option<(String, i32)> {
        use jni::{jni_sig, jni_str, objects::JString};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return None;
        }
        let app = self.android_app.as_ref()?;

        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr() as _) };
        vm.attach_current_thread(|env| -> jni::errors::Result<Option<(String, i32)>> {
            let edit_text = local_ref(env, raw)?;

            let editable =
                env.call_method(&edit_text, jni_str!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?.l()?;
            let text_obj =
                env.call_method(&editable, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
            let jstr = unsafe { JString::from_raw(env, text_obj.into_raw() as jni::sys::jstring) };
            let text = jstr.to_string();

            let caret = env.call_method(&edit_text, jni_str!("getSelectionStart"), jni_sig!("()I"), &[])?.i()?;

            Ok(Some((text, caret)))
        })
        .unwrap_or(None)
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

                let two_spaces = env.new_string(IME_BASELINE)?;
                env.call_method(
                    &edit_text,
                    jni_str!("setText"),
                    jni_sig!("(Ljava/lang/CharSequence;)V"),
                    &[JValue::Object(&two_spaces)],
                )?;
                env.call_method(
                    &edit_text,
                    jni_str!("setSelection"),
                    jni_sig!("(II)V"),
                    &[IME_BASELINE_CARET.into(), IME_BASELINE_CARET.into()],
                )?;

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

    /// Inject what the IME wrote into the hidden `EditText`.
    ///
    /// The buffer is compared to the last injected state (`last_seen`): caret
    /// move → Left/Right, removal → `Key::Backspace`, insertion →
    /// `Input::text_inject`, replacement → Backspace + insert.
    ///
    /// Deletions must go through `Input::key_inject_press`: StereoKit's text
    /// queue only carries insertable characters, so `"\x08"`/`"\x7f"` are
    /// dropped with a warning. Each state is handled once, and the reset posted
    /// afterwards only applies when the content is unchanged, so a slow Java
    /// thread can neither duplicate an event nor wipe new characters.
    fn poll_input_events(&mut self, app: &AndroidApp) {
        let (text, caret) = match self.poll_edit_text() {
            Some(s) => s,
            None => return,
        };

        // The Java main thread applied a reset while we were reading the
        // `EditText`: the text and the caret below may come from two different
        // states. Skip this frame, the next read will be consistent.
        if self.reset_applied.swap(false, Ordering::AcqRel) {
            self.last_seen = None;
            return;
        }

        let at_baseline = text == IME_BASELINE && caret == IME_BASELINE_CARET;

        if at_baseline {
            // The `EditText` is back to its idle state: nothing to inject.
            self.last_seen = None;
            return;
        }

        if self.last_seen.as_ref().is_some_and(|(seen, seen_caret)| *seen_caret == caret && seen == &text) {
            // Already handled, the reset is still on its way.
            self.schedule_edit_text_reset(app, text);
            return;
        }

        let (previous, previous_caret) =
            self.last_seen.clone().unwrap_or_else(|| (IME_BASELINE.to_string(), IME_BASELINE_CARET));

        Self::inject_ime_change(&previous, previous_caret, &text, caret);
        // The IME produced something: it is displayed, stop asking for it.
        self.ime_seen_input = true;
        self.schedule_edit_text_reset(app, text.clone());
        self.last_seen = Some((text, caret));
    }

    /// Compare the new hidden `EditText` state to the previous one and inject
    /// the matching StereoKit events.
    ///
    /// `previous` mirrors what the `EditText` contained when the events already
    /// sent to StereoKit were consumed: a character rewritten by the IME
    /// (composing text, accent, autocorrect) is a replacement, not a second
    /// insertion.
    fn inject_ime_change(previous: &str, previous_caret: i32, text: &str, caret: i32) {
        let previous: Vec<char> = previous.chars().collect();
        let current: Vec<char> = text.chars().collect();

        // Common prefix and suffix, in characters: everything between them was
        // removed and/or inserted by the IME.
        let max_prefix = previous.len().min(current.len());
        let mut prefix = 0;
        while prefix < max_prefix && previous[prefix] == current[prefix] {
            prefix += 1;
        }
        let max_suffix = max_prefix - prefix;
        let mut suffix = 0;
        while suffix < max_suffix && previous[previous.len() - 1 - suffix] == current[current.len() - 1 - suffix] {
            suffix += 1;
        }

        let removed = previous.len() - prefix - suffix;
        let inserted: String = current[prefix..current.len() - suffix].iter().collect();

        if removed == 0 && inserted.is_empty() {
            // Only the caret moved: arrow key(s).
            let delta = caret - previous_caret;
            let key = if delta < 0 { Key::Left } else { Key::Right };
            for _ in 0..delta.unsigned_abs() {
                Input::key_inject_press(key);
                Input::key_inject_release(key);
            }
            return;
        }

        // Backspace and not Del: system keyboards only provide a Backspace, and
        // both baseline characters are identical, so a forward deletion could
        // not be told apart from a Backspace anyway.
        for _ in 0..removed {
            Input::key_inject_press(Key::Backspace);
            Input::key_inject_release(Key::Backspace);
        }
        if !inserted.is_empty() {
            // Line breaks are forwarded as text: `input_text_inject` turns `\r`
            // and `\r\n` into `\n`. The Return key is a separate key of the
            // StereoKit keyboard layout, this path only carries the character.
            Input::text_inject(&inserted);
        }
    }

    /// Post a Java-main-thread callback resetting the hidden `EditText` back to
    /// [`IME_BASELINE`].
    ///
    /// The content is only overwritten when it still is `expected`: anything the
    /// IME produced in the meantime is left for the next `poll_input_events`
    /// instead of being wiped.
    fn schedule_edit_text_reset(&mut self, app: &AndroidApp, expected: String) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let edit_text_raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if edit_text_raw <= 1 {
            return;
        }
        // One reset in flight is enough: the next one is only needed once the
        // content changed again.
        if self.reset_in_flight.swap(true, Ordering::AcqRel) {
            return;
        }

        let app2 = app.clone();
        let reset_in_flight = Arc::clone(&self.reset_in_flight);
        let reset_applied = Arc::clone(&self.reset_applied);
        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<bool> {
                let edit_text = local_ref(env, edit_text_raw)?;

                let editable = env
                    .call_method(&edit_text, jni_str!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?
                    .l()?;
                let text_obj =
                    env.call_method(&editable, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
                let jstr = unsafe { jni::objects::JString::from_raw(env, text_obj.into_raw() as jni::sys::jstring) };
                if jstr.to_string() != expected {
                    // The IME typed more while this reset was queued: keep it.
                    return Ok(false);
                }

                let two_spaces = env.new_string(IME_BASELINE)?;
                env.call_method(
                    &edit_text,
                    jni_str!("setText"),
                    jni_sig!("(Ljava/lang/CharSequence;)V"),
                    &[JValue::Object(&two_spaces)],
                )?;
                env.call_method(
                    &edit_text,
                    jni_str!("setSelection"),
                    jni_sig!("(II)V"),
                    &[IME_BASELINE_CARET.into(), IME_BASELINE_CARET.into()],
                )?;

                Ok(true)
            });
            match result {
                Ok(true) => reset_applied.store(true, Ordering::Release),
                Ok(false) => {}
                Err(e) => Log::err(format!("AndroidSoftKbd: schedule_edit_text_reset JNI error: {e:?}")),
            }
            reset_in_flight.store(false, Ordering::Release);
        }));
    }
}

/// Borrow a raw JNI global reference as a local reference of the current frame.
fn local_ref<'local>(env: &mut jni::Env<'local>, raw: usize) -> jni::errors::Result<jni::objects::JObject<'local>> {
    let object = unsafe { jni::objects::JObject::from_raw(env, raw as jni::sys::jobject) };
    env.new_local_ref(&object)
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
