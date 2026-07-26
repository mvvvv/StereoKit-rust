#![cfg(not(feature = "no-event-loop"))]
pub mod demos;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

use demos::program::launch;
use stereokit_rust::{
    sk::Sk,
    sk::{OriginMode, SkSettings},
    system::{BackendOpenXR, Log, LogLevel}
};

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
pub fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::{sk::DepthMode, system::{BackendVulkan, BackendVulkanRequest}};

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4) // aka the default aka 0
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

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

    let sk = settings.init(app).unwrap();

    _main(sk);
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
    let sk = settings.init().unwrap();
    _main(sk);
}

pub fn _main(sk: Sk) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(sk, is_testing, start_test);
    Sk::shutdown();
}
