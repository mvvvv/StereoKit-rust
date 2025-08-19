#![cfg(feature = "event-loop")]

use hand_menu_radial0::HandMenuRadial0;
use input1::Input1;
use stereokit_rust::{prelude::*, system::BackendOpenXR};

pub mod a_stepper;
pub mod anchor1;
pub mod anim1;
pub mod asset1;
pub mod b_stepper;
pub mod biplane1;
pub mod c_stepper;
pub mod font1;
pub mod hand_menu_radial0;
pub mod hand_menu_radial1;
pub mod input1;
pub mod interactor1;
pub mod layers1;
pub mod math1;
pub mod program;
pub mod render_list1;
pub mod shaders1;
pub mod shadows1;
pub mod sprite1;
pub mod tex1;
pub mod text1;
pub mod text2;
pub mod threads1;
pub mod threads2;
pub mod ui1;

use self::{
    a_stepper::AStepper, anchor1::Anchor1, anim1::Anim1, asset1::Asset1, b_stepper::BStepper, biplane1::Biplane1,
    c_stepper::CStepper, font1::Font1, interactor1::Interactor1, layers1::Layers1, math1::Math1,
    render_list1::RenderList1, shaders1::Shader1, shadows1::Shadows1, sprite1::Sprite1, tex1::Tex1, text1::Text1,
    text2::Text2, threads1::Threads1, threads2::Threads2, ui1::Ui1,
};

pub struct Test {
    pub name: String,
    pub launcher: Box<dyn (Fn(&mut Sk) -> StepperId) + 'static>,
}

impl Test {
    pub fn new<T: Fn(&mut Sk) -> StepperId + 'static>(name: impl AsRef<str>, launcher: T) -> Self {
        Self { name: name.as_ref().to_string(), launcher: Box::new(launcher) }
    }

    pub fn get_tests() -> Box<[Test]> {
        let tests = [
            Test::new("Test A", |sk| {
                sk.send_event(StepperAction::add_default::<AStepper>("Test A"));
                "Test A".to_string()
            }),
            Test::new("Test B", |sk| {
                sk.send_event(StepperAction::add_default::<BStepper>("Test B"));
                "Test B".to_string()
            }),
            Test::new("Test C", |sk| {
                sk.send_event(StepperAction::add_default::<CStepper>("Test C"));
                "Test C".to_string()
            }),
            Test::new("HandMenuRadial0", |sk| {
                sk.send_event(StepperAction::add_default::<HandMenuRadial0>("HandMenuRadial0"));
                "HandMenuRadial0".to_string()
            }),
            Test::new("Threads1", |sk| {
                sk.send_event(StepperAction::add_default::<Threads1>("Threads1"));
                "Threads1".to_string()
            }),
            Test::new("Threads2", |sk| {
                sk.send_event(StepperAction::add_default::<Threads2>("Threads2"));
                "Threads2".to_string()
            }),
            Test::new("Anchor1", |sk| {
                sk.send_event(StepperAction::add_default::<Anchor1>("Anchor1"));
                "Anchor1".to_string()
            }),
            Test::new("Text1", |sk| {
                sk.send_event(StepperAction::add_default::<Text1>("Text1"));
                "Text1".to_string()
            }),
            Test::new("Font1", |sk| {
                sk.send_event(StepperAction::add_default::<Font1>("Font1"));
                "Font1".to_string()
            }),
            Test::new("Text2", |sk| {
                sk.send_event(StepperAction::add_default::<Text2>("Text2"));
                "Text2".to_string()
            }),
            Test::new("Sprite1", |sk| {
                sk.send_event(StepperAction::add_default::<Sprite1>("Sprite1"));
                "Sprite1".to_string()
            }),
            Test::new("Tex1", |sk| {
                sk.send_event(StepperAction::add_default::<Tex1>("Tex1"));
                "Tex1".to_string()
            }),
            Test::new("Ui1", |sk| {
                sk.send_event(StepperAction::add_default::<Ui1>("Ui1"));
                "Ui1".to_string()
            }),
            Test::new("Input1", |sk| {
                sk.send_event(StepperAction::add_default::<Input1>("Input1"));
                "Input1".to_string()
            }),
            Test::new("Interactor1", |sk| {
                sk.send_event(StepperAction::add_default::<Interactor1>("Interactor1"));
                "Interactor1".to_string()
            }),
            Test::new("Anim1", |sk| {
                sk.send_event(StepperAction::add_default::<Anim1>("Anim1"));
                "Anim1".to_string()
            }),
            Test::new("Shader1", |sk| {
                sk.send_event(StepperAction::add_default::<Shader1>("Shader1"));
                "Shader1".to_string()
            }),
            Test::new("Math1", |sk| {
                sk.send_event(StepperAction::add_default::<Math1>("Math1"));
                "Math1".to_string()
            }),
            Test::new("Asset1", |sk| {
                sk.send_event(StepperAction::add_default::<Asset1>("Asset1"));
                "Asset1".to_string()
            }),
            Test::new("RenderList1", |sk| {
                sk.send_event(StepperAction::add_default::<RenderList1>("RenderList1"));
                "RenderList1".to_string()
            }),
            Test::new("Biplane1", |sk| {
                sk.send_event(StepperAction::add_default::<Biplane1>("Biplane1"));
                "Biplane1".to_string()
            }),
            Test::new("Layers1", |sk| {
                sk.send_event(StepperAction::add_default::<Layers1>("Layers1"));
                "Layers1".to_string()
            }),
            Test::new("Shadows1", |sk| {
                sk.send_event(StepperAction::add_default::<Shadows1>("Shadows1"));
                "Shadows1".to_string()
            }),
        ];
        Box::new(tests)
    }
}

pub fn load_all_extensions() {
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
    BackendOpenXR::request_ext("XR_KHR_binding_modification");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_color_scale_bias");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cube");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_equirect2");
    BackendOpenXR::request_ext("XR_KHR_extended_struct_name_lengths");
    BackendOpenXR::request_ext("XR_KHR_generic_controller");
    BackendOpenXR::request_ext("XR_KHR_loader_init");
    BackendOpenXR::request_ext("XR_KHR_locate_spaces");
    BackendOpenXR::request_ext("XR_KHR_maintenance1");
    BackendOpenXR::request_ext("XR_KHR_opengl_enable");
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
