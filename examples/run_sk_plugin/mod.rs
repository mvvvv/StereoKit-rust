//! Plugin side of the `cargo-run_sk` hot-reload workflow.
//!
//! This module turns the `main` example (a `cdylib`) into a **plugin** loadable by the `cargo-run_sk` host:
//!
//! - the host keeps a single StereoKit session alive (Simulator or OpenXR),
//! - each time `cargo build --example main --features skc-shared` produces a new plugin library (`libmain.so` on
//!   Linux, `main.dll` on Windows, `libmain.dylib` on macOS), the host `end()`s the old plugin, unloads it and loads
//!   the new one, then calls `begin` again: the session survives,
//! - the views exposed here are the `Test`s of [`demos::Test::get_tests`], each one driven as an `IStepper` inside a
//!   **plugin-side** [`Steppers`] manager (never inside the host's one, so no vtable or `TypeId` ever crosses the
//!   host<->plugin boundary).
//!
//! Safety: the boundary with the host is 100% C (opaque pointers + the `#[repr(C)]` types of
//! [`stereokit_rust::plugin_abi`]). The `Sk` and [`MainThreadToken`] pointers are reinterpreted between two
//! compilations of the *same* crate version: never mix a host and a plugin built from different versions or feature
//! sets of `stereokit_rust` (the host checks both versions before calling `begin`).
use std::{ffi::c_void, sync::Mutex};

use stereokit_rust::{
    framework::{StepperAction, StepperId, Steppers},
    plugin_abi::{PluginViewInfo, SK_RUN_SK_ABI_VERSION},
    sk::{MainThreadToken, Sk},
    system::Log,
};

use crate::demos::Test;

/// Everything the plugin needs between `begin` and `end`.
struct PluginState {
    /// Pointer to the host `Sk`. Kept for future introspection, only valid on the main thread (see the SAFETY note on
    /// `unsafe impl Send`).
    #[allow(dead_code)]
    sk: *mut Sk,
    /// The plugin-side steppers: where the views actually run.
    steppers: Steppers,
    /// The views exposed to the host.
    tests: Box<[Test]>,
    /// Id of the currently running view stepper, if any.
    active_id: Option<StepperId>,
}

// SAFETY: every `sk_run_sk_*` entry point is called by the host on its main thread only, so the state (including the
// raw `Sk` pointer) is never actually shared across threads. The `Mutex` is just a convenient process-wide cell.
unsafe impl Send for PluginState {}

static PLUGIN: Mutex<Option<PluginState>> = Mutex::new(None);

/// ABI version guard: the host refuses plugins that don't match.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_version() -> u32 {
    SK_RUN_SK_ABI_VERSION
}

/// The `stereokit-rust` version this plugin was compiled against. The host compares it with its own: a mismatch means
/// the two sides may have different struct layouts, so the load is rejected.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_crate_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const std::ffi::c_char
}

/// Number of views exposed by this plugin.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_views_count() -> u32 {
    Test::get_tests().len() as u32
}

/// Fills the `info` out-parameter with the name and screenshot flag of view `index`. Returns 0 on success, 1 for a
/// null `info`, 2 for an out-of-bounds `index`.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_view_info(index: u32, info: *mut PluginViewInfo) -> u32 {
    // SAFETY: `info` points to a `PluginViewInfo` owned by the host.
    let Some(info) = (unsafe { info.as_mut() }) else {
        return 1;
    };
    let tests = Test::get_tests();
    let Some(test) = tests.get(index as usize) else {
        return 2;
    };
    *info = PluginViewInfo::new(&test.name, test.screenshot.is_some());
    0
}

/// Called by the host right after a successful load or reload. Stores the host `Sk` pointer and prepares the
/// plugin-side steppers.
/// Returns 0 on success, 1 for a null pointer, 2 if already begun.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_begin(sk: *mut c_void) -> u32 {
    if sk.is_null() {
        return 1;
    }
    let mut plugin = PLUGIN.lock().unwrap();
    if plugin.is_some() {
        Log::err("run_sk plugin: begin() called twice without end()");
        return 2;
    }
    // SAFETY: the host passes a pointer to its live `Sk`, built from the same
    // crate version and features as this plugin (checked by the host), and
    // calls us on its main thread.
    let sk = sk.cast::<Sk>();
    let sk_info = unsafe { (*sk).get_sk_info_clone() };
    let tests = Test::get_tests();
    let count = tests.len();
    *plugin = Some(PluginState { sk, steppers: Steppers::new(sk_info), tests, active_id: None });
    Log::info(format!("run_sk plugin: begin, {count} views"));
    0
}

/// Selects (and swaps) the active view. The previously active view, if any, is properly removed first. Returns 0 on
/// success, 1 when the plugin is not begun, 2 for an out-of-bounds index, 3 for a view without hot-reload launcher
/// (created with `Test::new` instead of `Test::from_stepper`).
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_select(index: u32) -> u32 {
    let mut plugin = PLUGIN.lock().unwrap();
    let Some(state) = plugin.as_mut() else {
        return 1;
    };
    if let Some(active_id) = state.active_id.take() {
        state.steppers.send_event(StepperAction::remove(active_id));
    }
    let Some(test) = state.tests.get(index as usize) else {
        return 2;
    };
    let Some(add_action) = &test.add_action else {
        Log::warn(format!("run_sk plugin: view {} has no hot-reload launcher", test.name));
        return 3;
    };
    state.steppers.send_event(add_action());
    state.active_id = Some(test.name.clone());
    Log::diag(format!("run_sk plugin: view {} selected", test.name));
    0
}

/// Called by the host every frame, right after its own step callback. Drives the plugin-side steppers (pre-app then
/// post-app, like `SkClosures` does). Returns 0 on success, 1 for a null token, 2 when the plugin is not begun.
#[unsafe(no_mangle)]
pub extern "C" fn sk_run_sk_step(_sk: *mut c_void, token: *mut c_void) -> u32 {
    if token.is_null() {
        return 1;
    }
    // SAFETY: the host passes a pointer to its `MainThreadToken`, laid out identically on both sides (same crate
    // version), main thread only.
    let token = unsafe { &mut *token.cast::<MainThreadToken>() };
    let mut plugin = PLUGIN.lock().unwrap();
    let Some(state) = plugin.as_mut() else {
        return 2;
    };
    if !state.steppers.step(token) {
        // A view asked to quit: close the plugin views but keep the host session alive (the dev viewer stays usable).
        Log::info("run_sk plugin: a view asked to quit, closing all views");
        state.steppers.shutdown();
        state.active_id = None;
    }
    state.steppers.step_post_app(token);
    0
}

/// Called by the host before unloading (`dlclose`) the plugin, and at application shutdown. Shuts the plugin-side
/// steppers down.
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
