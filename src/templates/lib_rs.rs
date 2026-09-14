use stereokit_rust::{
    framework::SkClosures,
    sk::{DepthMode, OriginMode, SkSettings},
    system::{BackendOpenXR, BackendVulkan, BackendVulkanRequest, LogLevel},
    ui::Ui,
};

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

/// All the SkSettings of the app, grouped in one single place.
pub fn sk_settings() -> SkSettings {
    // Initialize StereoKit with default settings
    let mut settings = SkSettings::default();
    settings
        .app_name("Basic Template App")
        .origin(OriginMode::Local)
        .render_multisample(4)
        .render_scaling(1.5)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    // The OpenXR extensions and the Vulkan requests must be set up before StereoKit initialization
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");
    BackendVulkan::request(&BackendVulkanRequest::new(Some("sk_template_request")));

    settings
}

#[unsafe(no_mangle)]
#[cfg(target_os = "android")]
pub fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::sk::Sk;

    // All the SkSettings AND the BackendOpenXR / BackendVulkan parameterizations are grouped in sk_settings()
    let mut settings = sk_settings();
    settings.android_app(app);

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    APP_ONCE.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });

    // Main loop
    launch(settings, false);

    Sk::shutdown();
}

/// Main function for All!
pub fn launch(mut settings: SkSettings, _is_testing: bool) {
    let sk = settings.init().unwrap();

    // Create a grabbable window with a button to exit the application
    let mut window_pose = Ui::popup_pose([0.0, -0.4, 0.0]);
    // Main loop
    SkClosures::new(sk, |sk, _token| {
        // Exit button
        Ui::window("Hello world!").pose(&mut window_pose).begin();
        if Ui::button("Exit").press() {
            sk.quit(None)
        }
        Ui::window_end();
    })
    .run();
}
