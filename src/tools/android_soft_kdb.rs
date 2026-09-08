//! Android soft keyboard stepper — manages the hidden `EditText` IME bridge
//! and drains Android input events each frame on Quest/Meta devices.
//!
//! The stepper watches [`crate::util::Platform::is_keyboard_visible`] every
//! frame.  When StereoKit's virtual keyboard appears it dismisses it and opens
//! the Android IME instead; when it disappears the IME is closed.
//!
//! Register this stepper once at startup on Android:
//! ```ignore
//! sk.send_event(StepperAction::add(ANDROID_SOFT_KBD_ID, AndroidSoftKbd::default()));
//! ```

use android_activity::AndroidApp;

use std::sync::{Arc, Mutex};

use crate::prelude::*;

/// Stepper ID for [`AndroidSoftKbd`].
pub const ANDROID_SOFT_KBD_ID: &str = "Tool_AndroidSoftKbdID";

/// Stepper that manages the hidden Android `EditText` IME bridge and polls
/// input events every frame.
///
/// Each frame it checks [`crate::util::Platform::is_keyboard_visible`]: when
/// the SK keyboard becomes visible it hides it and shows the Android IME;
/// when it disappears the IME is dismissed.
#[derive(IStepper)]
pub struct AndroidSoftKbd {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    /// Clone of the `AndroidApp` stored at `start()` time.
    android_app: Option<AndroidApp>,

    /// Raw JNI global ref to the hidden `EditText`.
    /// `0` = not created, `1` = creation in progress, `>1` = valid jobject.
    edit_text: Arc<Mutex<usize>>,

    /// Whether a reset of the hidden EditText to "  " (cursor 1) has been
    /// scheduled but not yet confirmed.  While `true`, `poll_input_events`
    /// skips processing until the EditText reports the expected state.
    reset_pending: bool,

    /// Whether we have requested the Android IME to be shown.
    /// Stays `true` until the stepper shuts down or we explicitly close it.
    ime_open: bool,
}

unsafe impl Send for AndroidSoftKbd {}

impl Default for AndroidSoftKbd {
    fn default() -> Self {
        Self {
            id: ANDROID_SOFT_KBD_ID.to_string(),
            sk_info: None,

            android_app: None,
            edit_text: Arc::new(Mutex::new(0)),
            reset_pending: false,
            ime_open: false,
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
        // Log::info("AndroidSoftKbd: initialized");
        true
    }

    /// Called from `IStepper::step` to handle events — nothing to do here.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from `IStepper::step` every frame.
    fn draw(&mut self, _token: &MainThreadToken) {
        use crate::system::TextContext;
        use crate::util::Platform;

        // Clone to avoid holding a borrow on self while calling methods.
        let app = match self.android_app.clone() {
            Some(a) => a,
            None => return,
        };

        self.poll_input_events(&app);

        // Bridge SK keyboard visibility → Android IME.
        let kb_visible = Platform::is_keyboard_visible();
        if kb_visible && !self.ime_open {
            // SK keyboard just appeared: dismiss it and open the Android IME.
            Platform::keyboard_show(false, TextContext::Text);
            self.show_ime_kdb(&app, true);
            self.ime_open = true;
        } else if self.ime_open {
            // SK keyboard reappeared while IME is open (e.g. SK re-showed it):
            // suppress it again without touching the IME.
            if kb_visible {
                Platform::keyboard_show(false, TextContext::Text);
            }
            if unsafe { crate::sk::sk_app_focus() } == crate::sk::AppFocus::Active {
                Log::diag("AndroidSoftKbd: app is Active → IME was closed externally");
                *self.edit_text.lock().expect("Failed to lock edit_text mutex") = 0;
                Platform::keyboard_show(false, TextContext::Text);
                self.ime_open = false;
            }
        }
    }

    /// Read the current text content of the hidden `EditText`.
    /// Returns `None` if the view is not ready yet.
    fn poll_edit_text(&self) -> Option<(String, i32, i32)> {
        use crate::system::BackendAndroid;
        use jni::{jni_sig, jni_str, objects::JString};

        let raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if raw <= 1 {
            return None;
        }

        let vm = unsafe { jni::JavaVM::from_raw(BackendAndroid::java_vm() as _) };
        vm.attach_current_thread(|env| -> jni::errors::Result<Option<(String, i32, i32)>> {
            let edit_text_raw = unsafe { jni::objects::JObject::from_raw(env, raw as jni::sys::jobject) };
            let edit_text = env.new_local_ref(&edit_text_raw)?;
            std::mem::forget(edit_text_raw);

            let editable =
                env.call_method(&edit_text, jni_str!("getText"), jni_sig!("()Landroid/text/Editable;"), &[])?.l()?;
            let text_obj =
                env.call_method(&editable, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
            let jstr = unsafe { JString::from_raw(env, text_obj.into_raw() as jni::sys::jstring) };
            let text = jstr.to_string();

            let start = env.call_method(&edit_text, jni_str!("getSelectionStart"), jni_sig!("()I"), &[])?.i()?;
            let end = env.call_method(&edit_text, jni_str!("getSelectionEnd"), jni_sig!("()I"), &[])?.i()?;

            Ok(Some((text, start, end)))
        })
        .unwrap_or(None)
    }

    /// Lazily create the hidden `EditText` on the Java UI thread (one-time).
    ///
    /// Also sets `FLAG_LOCAL_FOCUS_MODE` on the Activity window so that
    /// `ViewRootImpl` ignores window-focus changes from the window manager.
    /// Without this flag the VR runtime's focus steal (when the keyboard opens)
    /// causes VRI to drop `ACTION_DOWN`/`ACTION_MULTIPLE` key events and makes
    /// `InputMethodManager` disconnect the `InputConnection`.
    ///
    /// The creation is asynchronous (`run_on_java_main_thread`).  On the first
    /// frame the `EditText` does not exist yet; `poll_edit_text` returns `None`
    /// until the UI-thread work completes.
    fn show_ime_kdb(&mut self, app: &AndroidApp, show_keyboard_after: bool) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let state = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if state == 1 {
            // Creation already in progress, skip.
            return;
        }

        // Save the raw pointer of any existing EditText so the callback can remove it.
        let old_raw = if state > 1 { state } else { 0 };
        *self.edit_text.lock().expect("Failed to lock edit_text mutex") = 1;

        let app2 = app.clone();
        let edit_text_ref = Arc::clone(&self.edit_text);
        Log::info("AndroidSoftKbd: scheduling EditText creation on Java main thread");
        app.run_on_java_main_thread(Box::new(move || {
            // Log::diag("AndroidSoftKbd: EditText callback running on Java main thread");
            let jvm = unsafe { jni::JavaVM::from_raw(app2.vm_as_ptr() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = unsafe { jni::objects::JObject::from_raw(env, app2.activity_as_ptr() as _) };

                // ---- FLAG_LOCAL_FOCUS_MODE (0x10000000) ----
                // Makes ViewRootImpl ignore window-focus changes from the WM.
                // Without this, VRI drops non-terminal key events (ACTION_DOWN,
                // ACTION_MULTIPLE) and IMM disconnects the InputConnection when
                // the Quest VR runtime steals focus for the soft keyboard overlay.
                let jni_window =
                    env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
                env.call_method(&jni_window, jni_str!("addFlags"), jni_sig!("(I)V"), &[0x10000000i32.into()])?;
                // Log::diag("AndroidSoftKbd: FLAG_LOCAL_FOCUS_MODE set");

                // ---- get DecorView (shared by removal and addView below) ----
                let decor_view = env
                    .call_method(&jni_window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?
                    .l()?;

                // ---- remove previous EditText if present ----
                if old_raw > 1 {
                    let old_edit_text_raw =
                        unsafe { jni::objects::JObject::from_raw(env, old_raw as jni::sys::jobject) };
                    let old_edit_text = env.new_local_ref(&old_edit_text_raw)?;
                    std::mem::forget(old_edit_text_raw);
                    let _ = env.call_method(
                        &decor_view,
                        jni_str!("removeView"),
                        jni_sig!("(Landroid/view/View;)V"),
                        &[JValue::Object(&old_edit_text)],
                    );
                    // Release the JNI global reference.
                    let _drop_old =
                        unsafe { env.global_from_raw::<jni::objects::JObject>(old_raw as jni::sys::jobject) };
                    Log::diag("AndroidSoftKbd: removed previous EditText");
                }

                // ---- create EditText(activity) ----
                let edit_text = env.new_object(
                    jni_str!("android/widget/EditText"),
                    jni_sig!("(Landroid/content/Context;)V"),
                    &[JValue::Object(&activity)],
                )?;

                // VISIBLE so it CAN receive focus (INVISIBLE/GONE cannot).
                // Alpha 0 makes it transparent — invisible to the user.
                env.call_method(&edit_text, jni_str!("setVisibility"), jni_sig!("(I)V"), &[0i32.into()])?;
                env.call_method(&edit_text, jni_str!("setAlpha"), jni_sig!("(F)V"), &[0.0f32.into()])?;
                env.call_method(&edit_text, jni_str!("setFocusable"), jni_sig!("(Z)V"), &[true.into()])?;
                env.call_method(&edit_text, jni_str!("setFocusableInTouchMode"), jni_sig!("(Z)V"), &[true.into()])?;
                // InputType.TYPE_CLASS_TEXT = 1
                let input_type_multi = 1i32 | 0x00020000i32;
                env.call_method(&edit_text, jni_str!("setInputType"), jni_sig!("(I)V"), &[input_type_multi.into()])?;

                // ---- add to DecorView ----
                env.call_method(
                    &decor_view,
                    jni_str!("addView"),
                    jni_sig!("(Landroid/view/View;)V"),
                    &[JValue::Object(&edit_text)],
                )?;

                // ---- give it focus ----
                let _got_focus = env.call_method(&edit_text, jni_str!("requestFocus"), jni_sig!("()Z"), &[])?.z()?;
                // Log::diag(format!("AndroidSoftKbd: requestFocus returned {got_focus}"));

                // ---- initialise with "  " (two spaces) and cursor at position 1 ----
                let two_spaces = env.new_string("  ")?;
                env.call_method(
                    &edit_text,
                    jni_str!("setText"),
                    jni_sig!("(Ljava/lang/CharSequence;)V"),
                    &[JValue::Object(&*two_spaces)],
                )?;
                env.call_method(&edit_text, jni_str!("setSelection"), jni_sig!("(II)V"), &[1i32.into(), 1i32.into()])?;

                // ---- store a global reference ----
                let global = env.new_global_ref(&edit_text)?;
                let raw = global.as_raw() as usize;
                std::mem::forget(global);
                *edit_text_ref.lock().expect("Failed to lock edit_text mutex") = raw;
                // Log::diag(format!("AndroidSoftKbd: stored EditText global ref 0x{raw:x}"));

                // ---- show soft keyboard in the same callback (avoids race) ----
                if show_keyboard_after {
                    let class_ctxt = env.find_class(jni_str!("android/content/Context"))?;
                    let ims = env.get_static_field(
                        class_ctxt,
                        jni_str!("INPUT_METHOD_SERVICE"),
                        jni_sig!("Ljava/lang/String;"),
                    )?;
                    let im_manager = env
                        .call_method(
                            &activity,
                            jni_str!("getSystemService"),
                            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                            &[ims.borrow()],
                        )?
                        .l()?;
                    env.call_method(
                        &im_manager,
                        jni_str!("restartInput"),
                        jni_sig!("(Landroid/view/View;)V"),
                        &[JValue::Object(&edit_text)],
                    )?;
                    let show_result = env
                        .call_method(
                            im_manager,
                            jni_str!("showSoftInput"),
                            jni_sig!("(Landroid/view/View;I)Z"),
                            &[JValue::Object(&edit_text), 2i32.into()],
                        )?
                        .z()?;
                    Log::diag(format!("AndroidSoftKbd: showSoftInput result={show_result}"));
                }

                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: JNI failed: {e:?}"));
                *edit_text_ref.lock().expect("Failed to lock edit_text mutex") = 0;
            }
        }));
    }

    /// Drain IME changes from the hidden `EditText` and forward them to SK.
    ///
    /// The `EditText` always contains two spaces `"  "` with the cursor at
    /// position 1.  This guarantees the Del, Left, and Right keys are always
    /// enabled (one char to the left and one to the right of the cursor).
    ///
    /// Each frame we read the current text / cursor state and compare it to
    /// the expected baseline:
    ///
    /// | Observed state | Injected event |
    /// |---|---|
    /// | `"  "` cursor 0 | `Key::Left` |
    /// | `"  "` cursor 2 | `Key::Right` |
    /// | 1-char text, cursor 0 | `text_inject('\x08')` (Backspace) |
    /// | 1-char text, cursor 1 | `text_inject('\x7f')` (forward Del) |
    /// | >2-char text | `text_inject` for each inserted char |
    ///
    /// After detecting a change we reset the `EditText` back to `"  "` cursor 1
    /// via the Java main thread and set `reset_pending = true`.  We skip
    /// processing while a reset is in flight to avoid double-injection.
    fn poll_input_events(&mut self, app: &AndroidApp) {
        use crate::system::{Input, Key};
        // use android_activity::InputStatus;
        // use android_activity::input::{InputEvent, KeyAction, Keycode};

        // --- IME / InputConnection text-diff path ---
        let (text, start, _end) = match self.poll_edit_text() {
            Some(s) => s,
            None => return,
        };

        // Waiting for our async reset to propagate: skip until confirmed.
        if self.reset_pending {
            if text == "  " && start == 1 {
                // Log::diag("AndroidSoftKbd: reset confirmed");
                self.reset_pending = false;
            }
            return;
        }

        // Nothing changed from the baseline.
        if text == "  " && start == 1 {
            return;
        }

        let char_count = text.chars().count() as i32;

        if text.as_str() == "  " {
            // Text is unchanged but the cursor moved — arrow key.
            if start == 0 {
                // Log::diag("AndroidSoftKbd: inject Left");
                Input::key_inject_press(Key::Left);
                Input::key_inject_release(Key::Left);
            } else if start == 2 {
                // Log::diag("AndroidSoftKbd: inject Right");
                Input::key_inject_press(Key::Right);
                Input::key_inject_release(Key::Right);
            }
        } else if char_count < 2 {
            // A character was deleted.
            if start == 0 {
                // Log::diag("AndroidSoftKbd: inject Backspace (IME path)");
                Input::text_inject("\x08");
            } else {
                // Log::diag("AndroidSoftKbd: inject Del (IME path)");
                Input::text_inject("\x7f");
            }
        } else if char_count > 2 {
            // Characters were inserted at position 1 by the IME.
            //
            // `start` is the caret position as reported by Java in UTF-16
            // code units, *not* a byte index into the UTF-8 `text`.  For BMP
            // text it equals the character (code point) position, so extract
            // the inserted run by characters — byte-slicing here panics on
            // multi-byte chars, e.g. `&text[1..2]` when a lone 'õ' (bytes
            // 1..3) was inserted.
            let insert_len = (start as usize).saturating_sub(1);
            let inject_str: String = text.chars().skip(1).take(insert_len).collect();
            if !inject_str.is_empty() {
                // Log::diag(format!("AndroidSoftKbd: inject {} char(s)", insert_len));
                Input::text_inject(inject_str);
            }
        } else {
            // 2 chars but not "  " → likely an accented char inserted at position 0 by a long-press.
            Log::diag(format!(
                "AndroidSoftKbd: unexpected text \"{}\" cursor {}, injecting char at pos 0",
                text, start
            ));
            if start == 1 {
                // we delete the non accented char and inject the accented one instead.
                Input::text_inject("\x08");
                let first_char: String = text.chars().take(1).collect();
                Input::text_inject(first_char);
            }
        }

        // Reset the EditText back to "  " cursor 1 on the Java main thread.
        self.schedule_edit_text_reset(app);
        self.reset_pending = true;
    }

    /// Post a Java-main-thread callback that sets the hidden `EditText` content
    /// to `"  "` (two spaces) with the cursor at position 1.
    fn schedule_edit_text_reset(&self, app: &AndroidApp) {
        use jni::{jni_sig, jni_str, objects::JValue};

        let edit_text_raw = *self.edit_text.lock().expect("Failed to lock edit_text mutex");
        if edit_text_raw <= 1 {
            return;
        }

        app.run_on_java_main_thread(Box::new(move || {
            let jvm = unsafe { jni::JavaVM::from_raw(crate::system::BackendAndroid::java_vm() as _) };
            let result = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let edit_text_raw = unsafe { jni::objects::JObject::from_raw(env, edit_text_raw as jni::sys::jobject) };
                let edit_text = env.new_local_ref(&edit_text_raw)?;
                std::mem::forget(edit_text_raw);

                let two_spaces = env.new_string("  ")?;
                env.call_method(
                    &edit_text,
                    jni_str!("setText"),
                    jni_sig!("(Ljava/lang/CharSequence;)V"),
                    &[JValue::Object(&*two_spaces)],
                )?;
                env.call_method(&edit_text, jni_str!("setSelection"), jni_sig!("(II)V"), &[1i32.into(), 1i32.into()])?;

                Ok(())
            });
            if let Err(e) = result {
                Log::err(format!("AndroidSoftKbd: schedule_edit_text_reset JNI error: {e:?}"));
            }
        }));
    }
}
