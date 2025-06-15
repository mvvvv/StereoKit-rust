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
    /*
        BackendOpenXR::request_ext("XR_ANDROID_anchor_sharing_export");
        BackendOpenXR::request_ext("XR_ANDROID_composition_layer_passthrough_mesh");
        BackendOpenXR::request_ext("XR_ANDROID_depth_texture");
        BackendOpenXR::request_ext("XR_ANDROID_device_anchor_persistence");
        BackendOpenXR::request_ext("XR_ANDROID_eye_tracking");
        BackendOpenXR::request_ext("XR_ANDROID_face_tracking");
        BackendOpenXR::request_ext("XR_ANDROID_hand_mesh");
        BackendOpenXR::request_ext("XR_ANDROID_light_estimation");
        BackendOpenXR::request_ext("XR_ANDROID_mouse_interaction");
        BackendOpenXR::request_ext("XR_ANDROID_passthrough_camera_state");
        BackendOpenXR::request_ext("XR_ANDROID_performance_metrics");
        BackendOpenXR::request_ext("XR_ANDROID_raycast");
        BackendOpenXR::request_ext("XR_ANDROID_recommended_resolution");
        BackendOpenXR::request_ext("XR_ANDROID_trackables");
        BackendOpenXR::request_ext("XR_ANDROID_trackables_marker");
        BackendOpenXR::request_ext("XR_ANDROID_trackables_object");
        BackendOpenXR::request_ext("XR_ANDROID_trackables_qr_code");
        BackendOpenXR::request_ext("XR_ANDROID_unbounded_reference_space");

        BackendOpenXR::request_ext("XR_ANDROIDSYS_anchor_sharing_import");
        BackendOpenXR::request_ext("XR_ANDROIDSYS_eye_tracking_calibration");
        BackendOpenXR::request_ext("XR_ANDROIDSYS_face_tracking_calibration");
        BackendOpenXR::request_ext("XR_ANDROIDSYS_ipd_calibration");
        BackendOpenXR::request_ext("XR_ANDROIDSYS_trackables_shoebox");

        BackendOpenXR::request_ext("XR_ANDROIDX_composition_layer_axis_aligned_distortion");
        BackendOpenXR::request_ext("XR_ANDROIDX_scene_meshing");

        BackendOpenXR::request_ext("XR_EXT_dpad_binding");
        BackendOpenXR::request_ext("XR_EXT_debug_utils");
        BackendOpenXR::request_ext("XR_EXT_future");
        BackendOpenXR::request_ext("XR_EXT_hand_interaction");
        BackendOpenXR::request_ext("XR_EXT_hand_tracking");
        BackendOpenXR::request_ext("XR_EXT_palm_pose");
        BackendOpenXR::request_ext("XR_EXT_performance_settings");
        BackendOpenXR::request_ext("XR_EXT_uuid");

        BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");
        BackendOpenXR::request_ext("XR_KHR_binding_modification");
        BackendOpenXR::request_ext("XR_KHR_composition_layer_color_scale_bias");
        BackendOpenXR::request_ext("XR_KHR_composition_layer_cube");
        BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");
        BackendOpenXR::request_ext("XR_KHR_composition_layer_equirect2");
        BackendOpenXR::request_ext("XR_KHR_loader_init");
        BackendOpenXR::request_ext("XR_KHR_loader_init_android");
        BackendOpenXR::request_ext("XR_KHR_locate_spaces");
        BackendOpenXR::request_ext("XR_KHR_maintenance1");
        BackendOpenXR::request_ext("XR_KHR_opengl_es_enable");
        BackendOpenXR::request_ext("XR_KHR_swapchain_usage_input_attachment_bit");
        BackendOpenXR::request_ext("XR_KHR_visibility_mask");
        BackendOpenXR::request_ext("XR_KHR_vulkan_enable");
        BackendOpenXR::request_ext("XR_KHR_vulkan_enable2");

        BackendOpenXR::request_ext("XR_FB_color_space");
        BackendOpenXR::request_ext("XR_FB_composition_layer_depth_test");
        BackendOpenXR::request_ext("XR_FB_composition_layer_image_layout");
        BackendOpenXR::request_ext("XR_FB_composition_layer_settings");
        BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
        BackendOpenXR::request_ext("XR_FB_render_model");
        BackendOpenXR::request_ext("XR_FB_foveation");
        BackendOpenXR::request_ext("XR_FB_foveation_configuration");
        BackendOpenXR::request_ext("XR_FB_foveation_vulkan");
        BackendOpenXR::request_ext("XR_FB_hand_tracking_aim");
        BackendOpenXR::request_ext("XR_FB_hand_tracking_mesh");
        BackendOpenXR::request_ext("XR_FB_space_warp");
        BackendOpenXR::request_ext("XR_FB_swapchain_update_state");

        BackendOpenXR::request_ext("XR_META_foveation_eye_tracked");
        BackendOpenXR::request_ext("XR_META_vulkan_swapchain_create_info");
        BackendOpenXR::request_ext("XR_META_virtual_keyboard");

        BackendOpenXR::request_ext("XR_MND_headless");
        BackendOpenXR::request_ext("XR_MND_headless");
        BackendOpenXR::request_ext("XR_MND_swapchain_usage_input_attachment_bit");

        BackendOpenXR::request_ext("XR_MNDX_ball_on_a_stick_controller");
        BackendOpenXR::request_ext("XR_MNDX_force_feedback_curl");
        BackendOpenXR::request_ext("XR_MNDX_hydra");
        BackendOpenXR::request_ext("XR_MNDX_system_buttons");
        BackendOpenXR::request_ext("XR_MNDX_xdev_space");
    */

    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");

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
