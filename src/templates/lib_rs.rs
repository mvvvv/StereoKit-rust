use stereokit_rust::{framework::SkClosures, prelude::*, ui::Ui};

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
pub fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::{
        sk::{DepthMode, OriginMode, SkSettings},
        system::LogLevel,
    };
    // Initialize StereoKit with default settings
    let mut settings = SkSettings::default();
    settings
        .app_name("Basic Template App")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    APP_ONCE.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });
    let sk = settings.init(app).unwrap();

    _main(sk);
}

/// Main function for All!
pub fn _main(sk: Sk) {
    // Create a grabbable window with a button to exit the application
    let mut window_pose = Ui::popup_pose([0.0, -0.4, 0.0]);
    // Main loop
    SkClosures::new(sk, |sk, _token| {
        // Exit button
        Ui::window_begin("Hello world!", &mut window_pose, None, None, None);
        if Ui::button("Exit", None) {
            sk.quit(None)
        }
        Ui::window_end();
    })
    .run();
}
