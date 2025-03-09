pub mod demos;

#[cfg(target_os = "android")]
//use android_activity::AndroidApp;
use winit::platform::android::activity::AndroidApp;

#[cfg(feature = "event-loop")]
use demos::program::launch;
#[cfg(feature = "event-loop")]
use stereokit_rust::{
    framework::StepperAction,
    sk::Sk,
    sk::{OriginMode, SkSettings},
    system::BackendOpenXR,
    system::Log,
    system::LogLevel,
};
#[cfg(feature = "event-loop")]
use winit::event_loop::EventLoop;

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
#[cfg(feature = "event-loop")]
pub fn android_main(app: AndroidApp) {
    use stereokit_rust::sk::DepthMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(2.0)
        .depth_mode(DepthMode::Stencil)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
    );

    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_passthrough");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    let (sk, event_loop) = settings.init_with_event_loop(app).unwrap();

    _main(sk, event_loop);
}

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
#[cfg(feature = "event-loop")]
fn main() {
    use stereokit_rust::sk::AppMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Stage)
        .log_filter(LogLevel::Diagnostic)
        .no_flatscreen_fallback(true)
        .mode(AppMode::Simulator);

    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_passthrough");
    let (sk, event_loop) = settings.init_with_event_loop().unwrap();
    _main(sk, event_loop);
}

#[cfg(feature = "event-loop")]
pub fn _main(sk: Sk, event_loop: EventLoop<StepperAction>) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(sk, event_loop, is_testing, start_test);
    Sk::shutdown();
}

/// Fake main for no-event-loop asked by cargo test --features no-event-loop
#[allow(dead_code)]
#[cfg(feature = "no-event-loop")]
fn main() {}
