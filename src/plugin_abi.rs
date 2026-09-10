//! The C ABI shared between the `cargo-run_sk` hot-reload host and the plugin `.so` of the project under development.
//!
//! Both sides are compiled from the *same* version of `stereokit_rust` (same rustc, same features), and both link the
//! same shared `libStereoKitC.so` (see the `skc-shared` feature), so the whole process drives ONE StereoKit
//! engine and ONE XR session. But no Rust type ever crosses the host<->plugin boundary: only opaque pointers and
//! \[`repr(C)`\] structures.
//!
//! The plugin exports the `sk_run_sk_*` symbols described below; the host resolves them with `libloading` right after
//! `dlopen`, checks [`crate::plugin_abi::SK_RUN_SK_ABI_VERSION`] (and the crate version guard), then drives the plugin each frame:
//!
//! ```text
//! host: dlopen(plugin) -> version OK? -> views_count/view_info -> begin(sk)
//!       each frame:                     step(sk, token)
//!       before dlclose:                 end()
//! ```

use std::ffi::c_char;

/// Version of the `cargo-run_sk` plugin ABI. The host refuses to load a plugin whose version differs.
pub const SK_RUN_SK_ABI_VERSION: u32 = 1;

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
