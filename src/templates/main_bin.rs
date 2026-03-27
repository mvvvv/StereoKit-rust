#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
fn main() {
    use stereokit_rust::{
        sk::{DepthMode, OriginMode, Sk, SkSettings},
        system::LogLevel,
    };
    use vr_app::_main;
    // Initialize StereoKit with default settings
    let mut settings = SkSettings::default();
    settings
        .app_name("BasicTemplate App")
        .origin(OriginMode::Local)
        .render_multisample(4)
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    let sk = settings.init().expect("Should initialize StereoKit");
    _main(sk);
    Sk::shutdown();
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
//fake main fn for android as entry is lib.rs/android_main(...)
fn main() {}
