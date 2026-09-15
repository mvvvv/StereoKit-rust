//! Hot-reload plugin entry points for the `cargo-run_sk` dev viewer.
//!
//! `src/plugin_shim.rs` turns this crate into a **plugin** loadable by the `cargo-run_sk` host:
//!
//! ```text
//! cargo install stereokit-rust --version <stereokit-rust version> --features skc-shared --bin cargo-run_sk
//! cargo build --features skc-shared
//! cargo run_sk --lib target/debug/lib<your_crate>.so   (Windows: <your_crate>.dll, macOS: lib<your_crate>.dylib)
//! ```
//!
//! The host keeps a single StereoKit session alive (Simulator or OpenXR) and reloads this library each time
//! `cargo build --features skc-shared` produces a new one: no session restart, no headset re-pairing. Your views (the
//! steppers declared in [`views`]) appear in the viewer's selector window.
//!
//! Safety: the boundary with the host is 100% C (opaque pointers + the `#[repr(C)]` types of
//! `stereokit_rust::plugin_abi`). Never mix a host and a plugin built from different versions of stereokit-rust: the
//! host checks the versions before calling `begin`.

use std::{cell::RefCell, ffi::c_void, rc::Rc, sync::Mutex};

use crate::MainStepper;
use crate::c_stepper::CStepper;
use stereokit_rust::{
    framework::{StepperAction, StepperId, Steppers},
    plugin_abi::{PluginViewInfo, SK_RUN_SK_ABI_VERSION},
    sk::{MainThreadToken, SkInfo, SkSettings},
    system::Log,
};

/// The `stereokit-rust` version this plugin was compiled against. The host compares it with its own: a mismatch means
/// the two sides may have different struct layouts, so the load is rejected.
/// You have to update this version with the same value of StereoKit-rust version from Cargo.toml
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_crate_version() -> *const std::ffi::c_char {
    concat!("${SK_VERSION}", "\0").as_ptr() as *const std::ffi::c_char
}

/// Declare here the views (your steppers) exposed to the `cargo-run_sk` viewer.
/// Each view is a name + a factory producing the `StepperAction::add_default` of the stepper to run. Add one line per
/// view.
fn views() -> Vec<(&'static str, Box<dyn Fn() -> StepperAction + Send>)> {
    vec![
        ("MainStepper", Box::new(|| StepperAction::add_default::<MainStepper>("MainStepper"))),
        ("CStepper", Box::new(|| StepperAction::add_default::<CStepper>("CStepper"))),
        // Add your own views here, for example:
        // ("My view", Box::new(|| StepperAction::add_default::<MyStepper>("My view"))),
    ]
}

/// Fills the `settings` out-parameter with the settings of this project (its `sk_settings()` function, defined in
/// `src/lib.rs`), so the host session of the `cargo-run_sk` viewer is initialized exactly like your app. Returns 0
/// on success, 1 for a null pointer.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_settings(settings: *mut SkSettings) -> u32 {
    // SAFETY: `settings` points to a `SkSettings` owned by the host.
    let Some(settings) = (unsafe { settings.as_mut() }) else {
        return 1;
    };
    *settings = crate::sk_settings();
    0
}

/// Everything the plugin needs between `begin` and `end`.
struct PluginState {
    /// The `SkInfo` of the host session: a clone of the `Rc` handed by the host, shared with its session. Only used
    /// on the main thread (see the SAFETY note on `unsafe impl Send`).
    #[allow(dead_code)]
    sk_info: Rc<RefCell<SkInfo>>,
    /// The plugin-side steppers: where the views actually run.
    steppers: Steppers,
    /// Id of the currently running view stepper, if any.
    active_id: Option<StepperId>,
}

// SAFETY: every `sk_run_sk_*` entry point is called by the host on its main thread only, so the state (including the
// shared `SkInfo`) is never actually shared across threads. The `Mutex` is just a convenient process-wide cell.
unsafe impl Send for PluginState {}

static PLUGIN: Mutex<Option<PluginState>> = Mutex::new(None);

/// ABI version guard: the host refuses plugins that don't match.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_version() -> u32 {
    /// Number of views exposed by this plugin.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_views_count() -> u32 {
        views().len() as u32
    }

    /// Fills the `info` out-parameter with the name and screenshot flag of view `index`. Returns 0 on success, 1 for a
    /// null `info`, 2 for an out-of-bounds `index`.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_view_info(index: u32, info: *mut PluginViewInfo) -> u32 {
        // SAFETY: `info` points to a `PluginViewInfo` owned by the host.
        let Some(info) = (unsafe { info.as_mut() }) else {
            return 1;
        };
        let views = views();
        let Some((name, _)) = views.get(index as usize) else {
            return 2;
        };
        let screenshot =
            std::path::Path::new(&format!("screenshots/{}.jpeg", name.to_lowercase().replace(' ', "_"))).is_file();
        *info = PluginViewInfo::new(name, screenshot);
        0
    }

    /// Called by the host right after a successful load or reload. Stores the `SkInfo` of the host session and
    /// prepares the plugin-side steppers. Returns 0 on success, 1 for a null pointer, 2 if already begun.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_begin(sk_info: *mut c_void) -> u32 {
        if sk_info.is_null() {
            return 1;
        }
        let mut plugin = PLUGIN.lock().unwrap();
        if plugin.is_some() {
            Log::err("run_sk plugin: begin() called twice without end()");
            return 2;
        }
        // SAFETY: the host passes a pointer to its live `Rc<RefCell<SkInfo>>`, built from the same crate version and
        // features as this plugin (checked by the host), and calls us on its main thread. We only clone that `Rc`
        // here: the plugin then owns a share of the host `SkInfo`, and the pointer itself is never used again.
        let Some(sk_info) = (unsafe { sk_info.cast::<Rc<RefCell<SkInfo>>>().as_ref() }).cloned() else {
            return 1;
        };
        let count = views().len();
        *plugin = Some(PluginState { sk_info: sk_info.clone(), steppers: Steppers::new(sk_info), active_id: None });
        Log::info(format!("run_sk plugin: begin, {count} views"));
        0
    }

    /// Selects (and swaps) the active view. The previously active view, if any, is properly removed first. Returns 0
    /// on success, 1 when the plugin is not begun, 2 for an out-of-bounds index.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_select(index: u32) -> u32 {
        let mut plugin = PLUGIN.lock().unwrap();
        let Some(state) = plugin.as_mut() else {
            return 1;
        };
        if let Some(active_id) = state.active_id.take() {
            state.steppers.send_event(StepperAction::remove(active_id));
        }
        let views = views();
        let Some((name, add_action)) = views.get(index as usize) else {
            return 2;
        };
        state.steppers.send_event(add_action());
        state.active_id = Some((*name).to_string());
        Log::diag(format!("run_sk plugin: view {name} selected"));
        0
    }

    /// Called by the host every frame, right after its own step callback. Drives the plugin-side steppers (pre-app
    /// then post-app, like `SkClosures` does). Returns 0 on success, 1 for a null token, 2 when the plugin is not
    /// begun.
    /// * `_sk_info` - The `Rc<RefCell<SkInfo>>` pointer of the host session: reserved, the plugin uses the clone
    ///   stored by `begin`.
    /// * `token` - The `MainThreadToken` of the frame.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_step(_sk_info: *mut c_void, token: *mut c_void) -> u32 {
        if token.is_null() {
            return 1;
        }
        // SAFETY: the host passes a pointer to its `MainThreadToken`, laid out
        // identically on both sides (same crate version), main thread only.
        let token = unsafe { &mut *token.cast::<MainThreadToken>() };
        let mut plugin = PLUGIN.lock().unwrap();
        let Some(state) = plugin.as_mut() else {
            return 2;
        };
        if !state.steppers.step(token) {
            // A view asked to quit: close the plugin views but keep the host
            // session alive (the dev viewer stays usable).
            Log::info("run_sk plugin: a view asked to quit, closing all views");
            state.steppers.shutdown();
            state.active_id = None;
        }
        state.steppers.step_post_app(token);
        0
    }

    /// Called by the host before unloading (`dlclose`/`FreeLibrary`) the plugin, and at application shutdown. Shuts
    /// the plugin-side steppers down.
    #[unsafe(no_mangle)]
    pub extern "C" fn sk_run_sk_end() -> u32 {
        let mut plugin = PLUGIN.lock().unwrap();
        if let Some(mut state) = plugin.take() {
            state.steppers.shutdown();
            state.active_id = None;
            Log::info("run_sk plugin: end");
        }
        0
    }

    SK_RUN_SK_ABI_VERSION
}
