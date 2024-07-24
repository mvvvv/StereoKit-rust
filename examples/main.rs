pub mod demos;

use stereokit_rust::event_loop::StepperAction;
#[cfg(target_os = "android")]
//use android_activity::AndroidApp;
use winit::platform::android::activity::AndroidApp;

use demos::program::launch;
use stereokit_rust::sk::Sk;
use stereokit_rust::system::Log;
use stereokit_rust::{
    sk::{OriginMode, SkSettings},
    system::BackendOpenXR,
    system::LogLevel,
};
use winit::event_loop::EventLoop;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use stereokit_rust::sk::DepthMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("stereokit-rust")
        .assets_folder("assets")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(2.0)
        .depth_mode(DepthMode::Stencil)
        .log_filter(LogLevel::Diagnostic);

    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Debug));

    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_passthrough");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    let (sk, event_loop) = settings.init_with_event_loop(app).unwrap();

    _main(sk, event_loop);
}

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::sk::AppMode;

    let mut settings = SkSettings::default();
    settings
        .app_name("stereokit-rust")
        .assets_folder("assets")
        .origin(OriginMode::Stage)
        .log_filter(LogLevel::Diagnostic)
        .no_flatscreen_fallback(true)
        .mode(AppMode::Simulator);

    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_passthrough");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    let (sk, event_loop) = settings.init_with_event_loop().unwrap();

    _main(sk, event_loop);
}

pub fn _main(sk: Sk, event_loop: EventLoop<StepperAction>) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::warn("Go go go !!!");
    launch(sk, event_loop, is_testing, start_test);
    Sk::shutdown();
}
