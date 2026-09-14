#![cfg(not(feature = "no-event-loop"))]
pub mod demos;

/// Hot-reload plugin entry points for the `cargo-run_sk` dev viewer. The exported `sk_run_sk_*` symbols are inert
/// unless the library is dlopen'ed by the host: they don't interfere with the normal (Android) launch.
#[cfg(feature = "skc-shared")]
mod run_sk_plugin;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

use demos::program::{launch, sk_settings};
use stereokit_rust::{
    sk::{Sk, SkSettings},
    system::Log,
};

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
pub fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;

    // All the SkSettings AND the BackendOpenXR / BackendVulkan parameterizations are grouped in sk_settings()
    let mut settings = sk_settings();
    settings.android_app(app);

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    if APP_ONCE.get().is_some() {
        Log::err("android_main called multiple times, ignoring subsequent calls");
        return;
    }
    APP_ONCE.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });
    //stereokit_rust::tools::load_all_extensions();
    _main(settings);
}

// Fake main that cannot be called as main.rs is a cdylib. That's why main_pc.rs exists.
// We keep it for information
#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::sk::AppMode;

    let mut settings = sk_settings();
    settings.no_flatscreen_fallback(true).mode(AppMode::Simulator);
    _main(settings);
}

pub fn _main(settings: SkSettings) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(settings, is_testing, start_test);
    Sk::shutdown();
}
