//! The C ABI shared between the `main_hot_reloading` hot-reload host and the plugin library of the project under development.
//!
//! Both sides are compiled from the *same* version of `stereokit_rust` (same rustc, same features), and both link the
//! same shared StereoKitC library (`libStereoKitC.so` on Linux, `StereoKitC.dll` on Windows, `libStereoKitC.dylib`
//! on macOS, see the `skc-shared`
//! feature), so the whole process drives ONE StereoKit engine and ONE XR session. But no Rust type ever crosses the
//! host<->plugin boundary: only opaque pointers and \[`repr(C)`\] structures.
//!
//! The plugin exports the `sk_run_sk_*` symbols described below; the host resolves them with `libloading` right after
//! loading it, checks [`crate::plugin_abi::SK_RUN_SK_ABI_VERSION`] (and the crate version guard), then drives the
//! plugin each frame:
//!
//! ```text
//! host: load(plugin) -> version OK? -> settings -> views_count/view_info -> begin(sk_info)
//!       each frame:                    step(sk_info, token)
//!       before unload:                 end()
//! ```
//!
//! `begin` and `step` receive an **opaque pointer to the `Rc<RefCell<SkInfo>>` of the host session**
//!
//! The settings of the host session are the ones of the project itself: the plugin exposes them through
//! [`crate::plugin_abi::SkSettingsFn`] (`sk_run_sk_settings` -> the `sk_settings()` function of the project), and the
//! host reads them before initializing StereoKit.

use crate::sk::SkSettings;
use std::ffi::c_char;

/// Version of the hot reload plugin ABI. The host refuses to load a plugin whose version differs.
///
/// * 1 - first version: `sk_run_sk_begin` received a pointer to the host `Sk`.
/// * 2 - `sk_run_sk_begin` receives a pointer to the `Rc<RefCell<SkInfo>>` of the host session.
pub const SK_RUN_SK_ABI_VERSION: u32 = 2;

/// Number of bytes (including the nul terminator) of a view name.
pub const SK_RUN_SK_NAME_MAX: usize = 64;

/// Description of one view (a "Test") exposed by the plugin.
///
/// `name` is a nul-terminated UTF-8 string; `has_screenshot` is non-zero when a screenshot file for this view already
/// exists under `screenshots/`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginViewInfo {
    /// Nul-terminated view name.
    pub name: [c_char; SK_RUN_SK_NAME_MAX],
    /// Non-zero when a screenshot file exists for this view.
    pub has_screenshot: u8,
    /// Reserved, keeps the structure padding explicit.
    pub _pad: [u8; 7],
}

impl PluginViewInfo {
    /// Builds a `PluginViewInfo` from a Rust string, truncating if needed.
    pub fn new(name: &str, has_screenshot: bool) -> Self {
        let mut buffer = [0 as c_char; SK_RUN_SK_NAME_MAX];
        for (dst, src) in buffer.iter_mut().zip(name.as_bytes().iter()) {
            *dst = *src as c_char;
        }
        // Ensure nul-termination even when the name has been truncated.
        buffer[SK_RUN_SK_NAME_MAX - 1] = 0;
        Self { name: buffer, has_screenshot: u8::from(has_screenshot), _pad: [0; 7] }
    }

    /// The view name as a Rust `String` (lossy for non-UTF-8 bytes).
    pub fn name(&self) -> String {
        let bytes: Vec<u8> = self.name.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// Signature of the `sk_run_sk_settings` plugin function: fills the out-parameter with the [`SkSettings`] of the
/// project (its `sk_settings()` function), so the host initializes its session exactly like the project does.
///
/// Returns 0 on success, 1 for a null pointer.
pub type SkSettingsFn = unsafe extern "C" fn(settings: *mut SkSettings) -> u32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_info_round_trip() {
        let info = PluginViewInfo::new("Tex1", true);
        assert_eq!(info.name(), "Tex1");
        assert_eq!(info.has_screenshot, 1);

        // Truncation keeps the struct nul-terminated and valid.
        let long = "x".repeat(200);
        let info = PluginViewInfo::new(&long, false);
        assert_eq!(info.name().len(), SK_RUN_SK_NAME_MAX - 1);
        assert_eq!(info.has_screenshot, 0);
    }
}
