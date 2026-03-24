pub mod demos;

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(not(feature = "no-event-loop"))]
use demos::program::launch;
#[cfg(not(feature = "no-event-loop"))]
use stereokit_rust::{
    sk::Sk,
    sk::{OriginMode, SkSettings},
    system::BackendOpenXR,
    system::Log,
    system::LogLevel,
};

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
#[cfg(not(feature = "no-event-loop"))]
pub fn android_main(app: AndroidApp) {
    use stereokit_rust::sk::DepthMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
    );

    //stereokit_rust::tools::load_all_extensions();
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");
    //BackendOpenXR::exclude_ext("XR_META_detached_controllers"); // uncomment if you don't want to see detached controllers
    BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");

    let sk = settings.init(app).unwrap();

    _main(sk);
}

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
#[cfg(not(feature = "no-event-loop"))]
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

#[cfg(not(feature = "no-event-loop"))]
pub fn _main(sk: Sk) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(sk, is_testing, start_test);
    Sk::shutdown();
}

/// Fake main for no-event-loop asked by cargo test --features no-event-loop
#[allow(dead_code)]
#[cfg(feature = "no-event-loop")]
fn main() {}
