use crate::system::BackendOpenXR;

pub mod build_tools;
pub mod os_api;
pub mod xr_android_depth_texture;
pub mod xr_comp_layers;

#[cfg(feature = "event-loop")]
pub mod xr_fb_render_model;

#[cfg(feature = "event-loop")]
pub mod file_browser;

#[cfg(feature = "event-loop")]
pub mod fly_over;

#[cfg(feature = "event-loop")]
pub mod log_window;

#[cfg(feature = "event-loop")]
pub mod notif;

#[cfg(feature = "event-loop")]
pub mod passthrough_fb_ext;

#[cfg(feature = "event-loop")]
pub mod screenshot;

#[cfg(feature = "event-loop")]
pub mod xr_meta_virtual_keyboard;

#[cfg(feature = "event-loop")]
pub mod title;

/// All extensions encountered so far :
/// AndroidX + HorizonOS + ALVR Linux + WiVRn/Monado Simulator
pub fn load_all_extensions() {
    // Android extensions
    BackendOpenXR::request_ext("XR_ANDROID_depth_texture");
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
    BackendOpenXR::request_ext("XR_ANDROID_scene_meshing");
    BackendOpenXR::request_ext("XR_ANDROID_trackables");
    BackendOpenXR::request_ext("XR_ANDROID_trackables_marker");
    BackendOpenXR::request_ext("XR_ANDROID_trackables_object");
    BackendOpenXR::request_ext("XR_ANDROID_trackables_qr_code");
    BackendOpenXR::request_ext("XR_ANDROID_unbounded_reference_space");
    // EXT extensions
    BackendOpenXR::request_ext("XR_EXT_active_action_set_priority");
    BackendOpenXR::request_ext("XR_EXT_composition_layer_inverted_alpha");
    BackendOpenXR::request_ext("XR_EXT_debug_utils");
    BackendOpenXR::request_ext("XR_EXT_dpad_binding");
    BackendOpenXR::request_ext("XR_EXT_frame_composition_report");
    BackendOpenXR::request_ext("XR_EXT_frame_synthesis");
    BackendOpenXR::request_ext("XR_EXT_future");
    BackendOpenXR::request_ext("XR_EXT_hand_interaction");
    BackendOpenXR::request_ext("XR_EXT_hand_joints_motion_range");
    BackendOpenXR::request_ext("XR_EXT_hand_tracking");
    BackendOpenXR::request_ext("XR_EXT_palm_pose");
    BackendOpenXR::request_ext("XR_EXT_performance_settings");
    BackendOpenXR::request_ext("XR_EXT_samsung_odyssey_controller");
    BackendOpenXR::request_ext("XR_EXT_spatial_anchor");
    BackendOpenXR::request_ext("XR_EXT_spatial_entity");
    BackendOpenXR::request_ext("XR_EXT_spatial_marker_tracking");
    BackendOpenXR::request_ext("XR_EXT_spatial_persistence");
    BackendOpenXR::request_ext("XR_EXT_spatial_persistence_operations");
    BackendOpenXR::request_ext("XR_EXT_spatial_plane_tracking");
    BackendOpenXR::request_ext("XR_EXT_user_presence");
    BackendOpenXR::request_ext("XR_EXT_uuid");
    BackendOpenXR::request_ext("XR_EXTX_overlay");

    // FB extensions
    BackendOpenXR::request_ext("XR_FB_body_tracking");
    BackendOpenXR::request_ext("XR_FB_color_space");
    BackendOpenXR::request_ext("XR_FB_common_events");
    BackendOpenXR::request_ext("XR_FB_composition_layer_alpha_blend");
    BackendOpenXR::request_ext("XR_FB_composition_layer_depth_test");
    BackendOpenXR::request_ext("XR_FB_composition_layer_image_layout");
    BackendOpenXR::request_ext("XR_FB_composition_layer_secure_content");
    BackendOpenXR::request_ext("XR_FB_composition_layer_settings");
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_face_tracking");
    BackendOpenXR::request_ext("XR_FB_face_tracking2");
    BackendOpenXR::request_ext("XR_FB_foveation");
    BackendOpenXR::request_ext("XR_FB_foveation_configuration");
    BackendOpenXR::request_ext("XR_FB_foveation_vulkan");
    BackendOpenXR::request_ext("XR_FB_hand_tracking_aim");
    BackendOpenXR::request_ext("XR_FB_hand_tracking_capsules");
    BackendOpenXR::request_ext("XR_FB_hand_tracking_mesh");
    BackendOpenXR::request_ext("XR_FB_haptic_amplitude_envelope");
    BackendOpenXR::request_ext("XR_FB_haptic_pcm");
    BackendOpenXR::request_ext("XR_FB_passthrough");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_FB_scene");
    BackendOpenXR::request_ext("XR_FB_scene_capture");
    BackendOpenXR::request_ext("XR_FB_space_warp");
    BackendOpenXR::request_ext("XR_FB_spatial_entity");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_container");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_query");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_sharing");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_storage");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_storage_batch");
    BackendOpenXR::request_ext("XR_FB_spatial_entity_user");
    BackendOpenXR::request_ext("XR_FB_swapchain_update_state");
    BackendOpenXR::request_ext("XR_FB_swapchain_update_state_opengl_es");
    BackendOpenXR::request_ext("XR_FB_swapchain_update_state_vulkan");
    BackendOpenXR::request_ext("XR_FB_touch_controller_pro");
    BackendOpenXR::request_ext("XR_FB_touch_controller_proximity");
    BackendOpenXR::request_ext("XR_FB_triangle_mesh");

    // HTC extensions
    BackendOpenXR::request_ext("XR_HTC_facial_tracking");
    BackendOpenXR::request_ext("XR_HTC_vive_cosmos_controller_interaction");
    BackendOpenXR::request_ext("XR_HTC_vive_focus3_controller_interaction");
    BackendOpenXR::request_ext("XR_HTC_vive_wrist_tracker_interaction");

    BackendOpenXR::request_ext("XR_HTCX_vive_tracker_interaction");

    // KHR extensions
    BackendOpenXR::request_ext("XR_KHR_D3D12_enable\n");
    BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");
    BackendOpenXR::request_ext("XR_KHR_binding_modification");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_color_scale_bias");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cube");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_equirect2");
    BackendOpenXR::request_ext("XR_KHR_extended_struct_name_lengths");
    BackendOpenXR::request_ext("XR_KHR_generic_controller");
    BackendOpenXR::request_ext("XR_KHR_loader_init");
    BackendOpenXR::request_ext("XR_KHR_loader_init_android");
    BackendOpenXR::request_ext("XR_KHR_locate_spaces");
    BackendOpenXR::request_ext("XR_KHR_maintenance1");
    BackendOpenXR::request_ext("XR_KHR_opengl_enable");
    BackendOpenXR::request_ext("XR_KHR_opengl_es_enable");
    BackendOpenXR::request_ext("XR_KHR_swapchain_usage_input_attachment_bit");
    BackendOpenXR::request_ext("XR_KHR_visibility_mask");
    BackendOpenXR::request_ext("XR_KHR_vulkan_enable");
    BackendOpenXR::request_ext("XR_KHR_vulkan_enable2");
    BackendOpenXR::request_ext("XR_KHR_vulkan_swapchain_format_list");

    // Logitech extensions
    BackendOpenXR::request_ext("XR_LOGITECH_mx_ink_stylus_interaction");

    // META extensions
    BackendOpenXR::request_ext("XR_META_automatic_layer_filter");
    BackendOpenXR::request_ext("XR_META_body_tracking_calibration");
    BackendOpenXR::request_ext("XR_META_body_tracking_fidelity");
    BackendOpenXR::request_ext("XR_META_body_tracking_full_body");
    BackendOpenXR::request_ext("XR_META_boundary_visibility");
    BackendOpenXR::request_ext("XR_META_colocation_discovery");
    BackendOpenXR::request_ext("XR_META_detached_controllers");
    BackendOpenXR::request_ext("XR_META_face_tracking_visemes");
    BackendOpenXR::request_ext("XR_META_feature_fidelity");
    BackendOpenXR::request_ext("XR_META_foveation_eye_tracked");
    BackendOpenXR::request_ext("XR_META_hand_tracking_microgestures");
    BackendOpenXR::request_ext("XR_META_headset_id");
    BackendOpenXR::request_ext("XR_META_passthrough_color_lut");
    BackendOpenXR::request_ext("XR_META_passthrough_layer_resumed_event");
    BackendOpenXR::request_ext("XR_META_passthrough_preferences");
    BackendOpenXR::request_ext("XR_META_performance_metrics");
    BackendOpenXR::request_ext("XR_META_recommended_layer_resolution");
    BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");
    BackendOpenXR::request_ext("XR_META_spatial_entity_discovery");
    BackendOpenXR::request_ext("XR_META_spatial_entity_group_sharing");
    BackendOpenXR::request_ext("XR_META_spatial_entity_mesh");
    BackendOpenXR::request_ext("XR_META_spatial_entity_persistence");
    BackendOpenXR::request_ext("XR_META_spatial_entity_sharing");
    BackendOpenXR::request_ext("XR_META_touch_controller_plus");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_META_vulkan_swapchain_create_info");

    // Magic Leap extensions
    BackendOpenXR::request_ext("XR_ML_ml2_controller_interaction");

    // Monado and MNDX extensions
    BackendOpenXR::request_ext("XR_MND_headless");
    BackendOpenXR::request_ext("XR_MND_swapchain_usage_input_attachment_bit");
    BackendOpenXR::request_ext("XR_MNDX_ball_on_a_stick_controller");
    BackendOpenXR::request_ext("XR_MNDX_egl_enable");
    BackendOpenXR::request_ext("XR_MNDX_force_feedback_curl");
    BackendOpenXR::request_ext("XR_MNDX_hydra");
    BackendOpenXR::request_ext("XR_MNDX_oculus_remote");
    BackendOpenXR::request_ext("XR_MNDX_system_buttons");
    BackendOpenXR::request_ext("XR_MNDX_xdev_space");

    // Oculus extensions
    BackendOpenXR::request_ext("XR_OCULUS_common_reference_spaces");

    // OPPO extensions
    BackendOpenXR::request_ext("XR_OPPO_controller_interaction");

    // Valve extensions
    BackendOpenXR::request_ext("XR_VALVE_analog_threshold");
}
