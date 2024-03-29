pub mod demos;

#[cfg(target_os = "android")]
//use winit::platform::android::activity::AndroidApp;
use android_activity::AndroidApp;
use demos::program::launch;
use stereokit_rust::sk::{Sk, StepperAction};
use stereokit_rust::system::Log;
use stereokit_rust::{
    sk::{OriginMode, SkSettings},
    system::LogLevel,
};
use winit::event_loop::EventLoop;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    let mut settings = SkSettings::default();
    settings
        .app_name("stereokit-rust")
        .assets_folder("assets")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(2.0)
        .log_filter(LogLevel::Diagnostic);

    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Debug));

    let (sk, event_loop) = settings.init(app).unwrap();

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

    let (sk, event_loop) = settings.init().unwrap();

    _main(sk, event_loop);
}

pub fn _main(sk: Sk, event_loop: EventLoop<StepperAction>) {
    let is_testing = false;
    let start_test = "".to_string();
    Log::err("Go go go !!!");
    launch(sk, event_loop, is_testing, start_test);
}
