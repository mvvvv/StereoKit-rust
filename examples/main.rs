#![cfg(not(feature = "no-event-loop"))]
pub mod demos;

/// Hot-reload plugin entry points for the `cargo-run_sk` dev viewer. The exported `sk_run_sk_*` symbols are inert
/// unless the library is dlopen'ed by the host: they don't interfere with the normal (Android) launch.
#[cfg(feature = "skc-shared")]
mod run_sk_plugin;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

use demos::program::launch;
use stereokit_rust::{
    sk::{OriginMode, Sk, SkSettings},
    system::{BackendOpenXR, Log, LogLevel},
};

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
pub fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::{
        sk::DepthMode,
        system::{BackendVulkan, BackendVulkanRequest},
    };

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4) // aka the default aka 0
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic)
        .android_app(app);

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
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");
    //BackendOpenXR::request_ext("XR_META_detached_controllers");
    // Required by the Layers1 demo for cylinder composition layers.
    BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");

    BackendVulkan::request(&BackendVulkanRequest::new(Some("sk_test_request")));

    _main(settings);
}

// Fake main that cannot be called as main.rs is a cdylib. That's why main_pc.rs exists.
// We keep it for information
#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::sk::AppMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Stage)
        .log_filter(LogLevel::Diagnostic)
        .no_flatscreen_fallback(true)
        .mode(AppMode::Simulator);

    //stereokit_rust::tools::load_all_extensions();
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    _main(settings);
}

pub fn _main(settings: SkSettings) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(settings, is_testing, start_test);
    Sk::shutdown();
}
