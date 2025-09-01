//! XR_FB_render_model extension implementation (HorizonOS)
//!
//! This module provides access to the OpenXR XR_FB_render_model extension,
//! which allows applications to retrieve render models for controllers and other devices.
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_FB_render_model>

use std::ffi::{CString, c_char};
use std::ptr;

use openxr_sys::{RenderModelCapabilitiesRequestFB, 
    Instance, Path, RenderModelBufferFB, RenderModelFlagsFB, RenderModelKeyFB, RenderModelLoadInfoFB,
    RenderModelPathInfoFB, RenderModelPropertiesFB, Result as XrResult, Session, StructureType,
    pfn::{EnumerateRenderModelPathsFB, GetRenderModelPropertiesFB, LoadRenderModelFB, PathToString, StringToPath},
};

use crate::maths::units::CM;
use crate::maths::{Matrix, Quat, Vec3};
use crate::model::AnimMode;
use crate::{
    model::Model, 
    prelude::*,
    system::{Backend, BackendOpenXR, BackendXRType, Handed, Input, Log},
};


/// Extension name for XR_FB_render_model
pub const XR_FB_RENDER_MODEL_EXTENSION_NAME: &str = "XR_FB_render_model";

/// Render model properties (simplified for easier use)
#[derive(Debug, Clone)]
pub struct RenderModelProperties {
    pub vendor_id: u32,  
    pub model_name: String,
    pub model_version: u32,
    pub flags: u64,
}

/// Main struct for XR_FB_render_model extension
pub struct XrFbRenderModel {
    with_log: bool,
    xr_enumerate_render_model_paths: Option<EnumerateRenderModelPathsFB>,
    xr_get_render_model_properties: Option<GetRenderModelPropertiesFB>,
    xr_load_render_model: Option<LoadRenderModelFB>,
    xr_string_to_path: Option<StringToPath>,
    xr_path_to_string: Option<PathToString>,
    instance: Instance,
    session: Session,

    // Cached controller models
    left_controller_data: Option<Model>,
    right_controller_data: Option<Model>,
}

impl XrFbRenderModel {
    /// Creates a new XrFbRenderModel instance if the extension is supported
    pub fn new(with_log: bool) -> Option<Self> {
        if Backend::xr_type() != BackendXRType::OpenXR || !BackendOpenXR::ext_enabled(XR_FB_RENDER_MODEL_EXTENSION_NAME)
        {
            return None;
        }

        let instance = Instance::from_raw(BackendOpenXR::instance());
        let session = Session::from_raw(BackendOpenXR::session());

        let xr_enumerate_render_model_paths =
            BackendOpenXR::get_function::<EnumerateRenderModelPathsFB>("xrEnumerateRenderModelPathsFB");
        let xr_get_render_model_properties =
            BackendOpenXR::get_function::<GetRenderModelPropertiesFB>("xrGetRenderModelPropertiesFB");
        let xr_load_render_model = BackendOpenXR::get_function::<LoadRenderModelFB>("xrLoadRenderModelFB");
        let xr_string_to_path = BackendOpenXR::get_function::<StringToPath>("xrStringToPath");
        let xr_path_to_string = BackendOpenXR::get_function::<PathToString>("xrPathToString");

        if xr_enumerate_render_model_paths.is_none()
            || xr_get_render_model_properties.is_none()
            || xr_load_render_model.is_none()
            || xr_string_to_path.is_none()
            || xr_path_to_string.is_none()
        {
            Log::warn("❌ Failed to load all XR_FB_render_model functions");
            return None;
        }

        Some(Self {
            with_log,
            xr_enumerate_render_model_paths,
            xr_get_render_model_properties,
            xr_load_render_model,
            xr_string_to_path,
            xr_path_to_string,
            instance,
            session,
            left_controller_data: None,
            right_controller_data: None,
        })
    }

    fn path_to_string(&self, path: Path) -> Option<String> {
        let path_to_string_fn = self.xr_path_to_string?;

        let mut buffer_count_output = 0u32;
        let result = unsafe { path_to_string_fn(self.instance, path, 0, &mut buffer_count_output, ptr::null_mut()) };

        if result != XrResult::SUCCESS || buffer_count_output == 0 {
            return None;
        }

        let mut buffer = vec![0u8; buffer_count_output as usize];
        let result = unsafe {
            path_to_string_fn(
                self.instance,
                path,
                buffer_count_output,
                &mut buffer_count_output,
                buffer.as_mut_ptr() as *mut c_char,
            )
        };

        if result == XrResult::SUCCESS {
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            String::from_utf8(buffer).ok()
        } else {
            None
        }
    }

    /// Enumerates available render model paths
    pub fn enumerate_render_model_paths(&self) -> Result<Vec<String>, XrResult> {
        let enumerate_fn = self.xr_enumerate_render_model_paths.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let mut path_count = 0u32;
        let result = unsafe { enumerate_fn(self.session, 0, &mut path_count, ptr::null_mut()) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        if path_count == 0 {
            return Ok(Vec::new());
        }

        let mut path_infos = vec![
            RenderModelPathInfoFB {
                ty: StructureType::RENDER_MODEL_PATH_INFO_FB,
                next: ptr::null_mut(),
                path: Path::from_raw(0),
            };
            path_count as usize
        ];

        let result = unsafe { enumerate_fn(self.session, path_count, &mut path_count, path_infos.as_mut_ptr()) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        let mut paths = Vec::new();
        for path_info in path_infos {
            if let Some(path_string) = self.path_to_string(path_info.path) {
                paths.push(path_string);
            }
        }

        Ok(paths)
    }

    /// Gets render model properties for a given model path
    pub fn get_render_model_properties(&self, model_path: &str) -> Result<RenderModelProperties, XrResult> {
        let get_properties_fn = self.xr_get_render_model_properties.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;
        let string_to_path_fn = self.xr_string_to_path.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let c_string = CString::new(model_path).map_err(|_| XrResult::ERROR_VALIDATION_FAILURE)?;
        let mut path = Path::from_raw(0);
        let result = unsafe { string_to_path_fn(self.instance, c_string.as_ptr(), &mut path) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        let mut properties = RenderModelPropertiesFB {
            ty: StructureType::RENDER_MODEL_PROPERTIES_FB,
            next: ptr::null_mut(),
            vendor_id: 0,
            model_name: [0; 64],
            model_key: RenderModelKeyFB::from_raw(0),
            model_version: 0,
            flags: RenderModelFlagsFB::from_raw(0),
        };

        let mut cap_req = RenderModelCapabilitiesRequestFB {
            ty: StructureType::RENDER_MODEL_CAPABILITIES_REQUEST_FB,
            next: ptr::null_mut(),
            flags: RenderModelFlagsFB::SUPPORTS_GLTF_2_0_SUBSET_2,

        };

        properties.next = &mut cap_req as *mut _ as *mut _;

        let result = unsafe { get_properties_fn(self.session, path, &mut properties) };

        if result != XrResult::SUCCESS //
            // && result != XrResult::RENDER_MODEL_UNAVAILABLE_FB //
            // && result != XrResult::SESSION_LOSS_PENDING //
        {
            return Err(result);
        }

        let model_name = unsafe {
            let name_ptr = properties.model_name.as_ptr() as *const c_char;
            let c_str = std::ffi::CStr::from_ptr(name_ptr);
            c_str.to_string_lossy().into_owned()
        };

        Ok(RenderModelProperties {
            vendor_id: properties.vendor_id,
            model_name,
            model_version: properties.model_version,
            flags: properties.flags.into_raw(),
        })
    }


    /// Loads render model data for a given model path
    pub fn load_render_model(&self, model_path: &str) -> Result<Vec<u8>, XrResult> {
        let load_fn = self.xr_load_render_model.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;
        let get_properties_fn = self.xr_get_render_model_properties.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;
        let string_to_path_fn = self.xr_string_to_path.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let c_string = CString::new(model_path).map_err(|_| XrResult::ERROR_VALIDATION_FAILURE)?;
        let mut path = Path::from_raw(0);
        let result = unsafe { string_to_path_fn(self.instance, c_string.as_ptr(), &mut path) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        let mut properties = RenderModelPropertiesFB {
            ty: StructureType::RENDER_MODEL_PROPERTIES_FB,
            next: ptr::null_mut(),
            vendor_id: 0,
            model_name: [0; 64],
            model_key: RenderModelKeyFB::from_raw(0),
            model_version: 0,
            flags: RenderModelFlagsFB::from_raw(0),
        };

        let mut cap_req = RenderModelCapabilitiesRequestFB {
            ty: StructureType::RENDER_MODEL_CAPABILITIES_REQUEST_FB,
            next: ptr::null_mut(),
            flags: RenderModelFlagsFB::SUPPORTS_GLTF_2_0_SUBSET_2,

        };

        properties.next = &mut cap_req as *mut _ as *mut _;

        let result = unsafe { get_properties_fn(self.session, path, &mut properties) };
        if result != XrResult::SUCCESS && result != XrResult::RENDER_MODEL_UNAVAILABLE_FB && result != XrResult::SESSION_LOSS_PENDING {
            return Err(result);
        }

        let model_key = properties.model_key;

        let load_info =
            RenderModelLoadInfoFB { ty: StructureType::RENDER_MODEL_LOAD_INFO_FB, next: ptr::null_mut(), model_key };

        let mut buffer = RenderModelBufferFB {
            ty: StructureType::RENDER_MODEL_BUFFER_FB,
            next: ptr::null_mut(),
            buffer_capacity_input: 0,
            buffer_count_output: 0,
            buffer: ptr::null_mut(),
        };

        let result = unsafe { load_fn(self.session, &load_info, &mut buffer) };
        if result != XrResult::SUCCESS {
            return Err(result);
        }

        let buffer_size = buffer.buffer_count_output;
        if buffer_size == 0 {
            return Ok(Vec::new());
        }

        let mut data_buffer = vec![0u8; buffer_size as usize];
        buffer.buffer_capacity_input = buffer_size;
        buffer.buffer = data_buffer.as_mut_ptr();

        let result = unsafe { load_fn(self.session, &load_info, &mut buffer) };
        if result != XrResult::SUCCESS {
            return Err(result);
        }

        data_buffer.truncate(buffer.buffer_count_output as usize);

        Ok(data_buffer)
    }

    /// Get cached controller model using specified path and hand, loading it if necessary
    pub fn get_controller_model(
        &mut self,
        handed: Handed,
        model_path: &str,
    ) -> Result<&Model, Box<dyn std::error::Error>> {
        let needs_loading = match handed {
            Handed::Left => self.left_controller_data.is_none(),
            Handed::Right => self.right_controller_data.is_none(),
            Handed::Max => return Err("Invalid handed value: Max is not a valid controller".into()),
        };

        if needs_loading {
            let data = self.load_render_model(model_path)?;
            let model = Model::from_memory(format!("{model_path}.gltf"), &data, None)?;

            if let Some(mut n) = model.get_nodes().get_root_node() {
                let new_rotation = Quat::from_angles(0.0, 0.0, 0.0);
                let transf = Matrix::t_r(Vec3::new(0.0, 0.0 * CM, 0.0 * CM), new_rotation);
                n.local_transform(transf);
            }

            match handed {
                Handed::Left => self.left_controller_data = Some(model),
                Handed::Right => self.right_controller_data = Some(model),
                Handed::Max => unreachable!(),
            }
        }

        match handed {
            Handed::Left => Ok(self.left_controller_data.as_ref().unwrap()),
            Handed::Right => Ok(self.right_controller_data.as_ref().unwrap()),
            Handed::Max => unreachable!(),
        }
    }

    /// Loads and configures controller models for both hands using specified paths
    pub fn setup_controller_models(&mut self, left_path: &str, right_path: &str, with_animation: bool) -> Result<(), XrResult> {
        // Load and set right controller model using specified path
        let with_log = self.with_log;
        if let Ok(right_model) = self.get_controller_model(Handed::Right, right_path) {
            Input::set_controller_model(Handed::Right, Some(right_model));
            if with_log {
                Log::info(format!("   Right controller model loaded and configured from path: {}", right_path));
            }

            // Launch animation 0 in Loop mode if with_animation is true
            if with_animation {
                right_model.get_anims().play_anim_idx(0, AnimMode::Loop);
                if right_model.get_anims().get_count() > 1 { 
                    Log::warn("⚠️ Right controller model has more than one animation, only the first will be played in loop"); 
                }
                if with_log {
                    Log::info("✅ Right controller animation started");
                }
            }
        } else {
            Log::warn(format!("❌ Failed to load right controller model from path: {}", right_path));
            return Err(XrResult::ERROR_RUNTIME_FAILURE);
        }

        // Load and set left controller model using specified path
        if let Ok(left_model) = self.get_controller_model(Handed::Left, left_path) {
            Input::set_controller_model(Handed::Left, Some(left_model));
            if with_log {
                Log::info(format!("   Left controller model loaded and configured from path: {}", left_path));
            }

            // Launch animation 0 in Loop mode if with_animation is true
            if with_animation {
                left_model.get_anims().play_anim_idx(0, AnimMode::Loop);
                if with_log {
                    Log::info("✅ Left controller animation started");
                }
            }
        } else {
            Log::warn(format!("❌ Failed to load left controller model from path: {}", left_path));
            return Err(XrResult::ERROR_RUNTIME_FAILURE);
        }

        Ok(())
    }

    /// Disables controller models by setting them to None
    pub fn disable_controller_models(&mut self) {
        use crate::system::{Handed, Input};

        Input::set_controller_model(Handed::Right, None);
        Input::set_controller_model(Handed::Left, None);
        self.left_controller_data = None;
        self.right_controller_data = None;
    }

    /// Explores and logs information about all available render models
    pub fn explore_render_models(&self) -> Result<(), XrResult> {
        if let Ok(paths) = self.enumerate_render_model_paths() {
            for path in paths {
                Log::diag(format!("   Render model: <{}>", path));
                match self.get_render_model_properties(&path) {
                    Ok(properties) => {
                        Log::diag(format!("     Model: {:?}", properties.model_name));
                        Log::diag(format!("     Vendor ID: {}", properties.vendor_id));
                        Log::diag(format!("     Model version: {}", properties.model_version));
                        Log::diag(format!("     Model flags: 0x{:?}", properties.flags));
                    }
                    Err(e) => {
                        Log::diag(format!("     No properties for model: {}: {:?}", path, e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Sets the animation time for a specific controller
    ///
    /// # Arguments
    /// * `handed` - Which controller to modify (Left or Right)
    /// * `time` - The animation time in seconds
    #[allow(unused, dead_code)]
    pub fn set_controller_anim_time(&mut self, handed: Handed, time: f32) {
        match handed {
            Handed::Left => {
                if let Some(ref left_model) = self.left_controller_data {
                    left_model.get_anims().anim_time(time);
                } 
            }
            Handed::Right => {
                if let Some(ref right_model) = self.right_controller_data {
                    right_model.get_anims().anim_time(time);
                } 
            }
            Handed::Max => {}
        }
    }
}

/// Convenience function to check if XR_FB_render_model extension is available
pub fn is_fb_render_model_extension_available() -> bool {
    Backend::xr_type() == BackendXRType::OpenXR && BackendOpenXR::ext_enabled(XR_FB_RENDER_MODEL_EXTENSION_NAME)
}

/// Event key for enabling/disabling controller drawing
pub const DRAW_CONTROLLER: &str = "draw_controller";

const LEFT_SHIFT: f32 = 0.04; // Left hand animation timing offset for synchronization

/// IStepper implementation for XR_FB_render_model integration with StereoKit
///
/// This stepper provides controller model rendering and animations using the OpenXR XR_FB_render_model extension.
/// You can configure the model paths for left and right controllers using the public properties or setter methods.
///
/// ### Events this stepper is listening to:
/// * `DRAW_CONTROLLER` - Event that triggers when controller rendering is enabled ("true") or disabled ("false").
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{
///     tools::xr_fb_render_model::{XrFbRenderModelStepper, is_fb_render_model_extension_available, DRAW_CONTROLLER},
///     system::{Input, Handed},
///     prelude::*,
/// };
///
/// // Check if the extension is available before using the stepper
/// if is_fb_render_model_extension_available() {
///     let mut stepper = XrFbRenderModelStepper::default();
///     
///     // Optional: customize controller model paths
///     stepper.left_controller_model_path = "/model_fb/controller/left".to_string();
///     stepper.right_controller_model_path = "/model_fb/controller/right".to_string();
///     
///     // Add the stepper to StereoKit
///     sk.send_event(StepperAction::add_default::<XrFbRenderModelStepper>("animate_controller"));
///     
///     // Enable controller rendering
///     sk.send_event(StepperAction::event("animate_controller", DRAW_CONTROLLER, "true"));
///     
///     filename_scr = "screenshots/xr_fb_render_model.jpeg"; fov_scr = 45.0;
///     test_steps!( // !!!! Get a proper main loop !!!!
///         // The stepper will automatically render controllers with animations
///         // based on input state (trigger, grip, etc.)
///         if iter == number_of_steps / 2 {
///             // Disable controller rendering halfway through
///             sk.send_event(StepperAction::event("main", DRAW_CONTROLLER, "false"));
///         }
///     );
/// }
/// ```
///
/// # Animation System
/// The stepper maps controller inputs to specific animation time codes:
/// - **Stick directions**: 8 cardinal points (1.18-1.64 range)
/// - **Trigger pressure**: Variable animation (0.6-0.66 range)
/// - **Grip pressure**: Variable animation (0.82-0.88 range)
/// - **Button combinations**: Discrete animations (0.18, 0.32, 0.46, 0.98)
///
/// When multiple inputs are active, the step rotation system cycles through
/// available animations using the `animation_time_code` property.
#[derive(IStepper)]
pub struct XrFbRenderModelStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,
    shutdown_completed: bool,

    /// Path to the left controller's render model in the OpenXR runtime
    /// Default: "/model_fb/controller/left" (Meta Quest controllers)
    pub left_controller_model_path: String,
    
    /// Path to the right controller's render model in the OpenXR runtime  
    /// Default: "/model_fb/controller/right" (Meta Quest controllers)
    pub right_controller_model_path: String,

    xr_render_model: Option<XrFbRenderModel>,
    is_enabled: bool,

    /// Animation time code for manual control and step rotation system
    /// Used in animation_analyser for development and in set_animation for step cycling
    pub animation_time_code: f32,
    
    /// Controls whether animations are executed in the draw method
    /// When false, controllers will be rendered but remain static
    pub with_animation: bool,
}

impl Default for XrFbRenderModelStepper {
    fn default() -> Self {
        Self {
            id: "XrFbRenderModelStepper".to_string(),
            sk_info: None,
            enabled: true,
            shutdown_completed: false,

            xr_render_model: None,
            is_enabled: false,
            animation_time_code: 0.0,
            with_animation: true,

            // Default model paths for Meta Quest controllers
            left_controller_model_path: "/model_fb/controller/left".to_string(),
            right_controller_model_path: "/model_fb/controller/right".to_string(),
        }
    }
}

unsafe impl Send for XrFbRenderModelStepper {}

impl XrFbRenderModelStepper {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // Initialize XR render model system
        //Log::info("🔧 Initializing XR_FB_render_model...");
        if !is_fb_render_model_extension_available() {
            Log::err("⚠️ XR_FB_render_model extension not available");
            return false; 
        }
        match XrFbRenderModel::new(false) {
            Some(xr_model) => {
                // Explore available models
                // if let Err(e) = xr_model.explore_render_models() {
                //     Log::warn(format!("❌ Failed to explore XR_FB_render_models: {:?}", e));
                // }
                self.xr_render_model = Some(xr_model);
                true
            }
            None => {
                Log::err("❌ XR_FB_render_model extension not available");
                false 
            }
        }
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key == DRAW_CONTROLLER {
            match value {
                "true" => {
                    if !self.is_enabled {
                        self.is_enabled = true;

                        // Load controller models if available
                        if let Some(ref mut xr_model) = self.xr_render_model {
                            // Setup controller models for both hands using configured paths
                            if let Err(e) = xr_model.setup_controller_models(
                                &self.left_controller_model_path,
                                &self.right_controller_model_path,
                                self.with_animation,
                            ) {
                                Log::warn(format!("❌ DRAW_CONTROLLER Failed to setup controller models: {:?}", e));
                            } else {
                                Log::info("DRAW_CONTROLLER `true`: Controller models setup completed");
                            }
                        } else {
                            Log::warn("❌ DRAW_CONTROLLER `true` error: XR_FB_render_model not initialized");
                        }
                    }
                }
                _=> {
                    self.is_enabled = false;

                    // Disable controller models and animations
                    if let Some(ref mut xr_model) = self.xr_render_model {
                        xr_model.disable_controller_models();
                        Log::info("DRAW_CONTROLLER `false`: Controller drawing disabled");
                    } else {
                        Log::warn("❌ DRAW_CONTROLLER `false` error: XR_FB_render_model not initialized");
                    }
                }

            }
        }
    }

    /// Set animation based on controller button states with step rotation system
    ///
    /// This function maps controller input states to specific animation times:
    /// - Stick directions (8 cardinal points): 1.18-1.64 range
    /// - Trigger pressure: 0.6-0.66 range (variable based on pressure)  
    /// - Grip pressure: 0.82-0.88 range (variable based on pressure)
    /// - Button combinations: 0.18, 0.32, 0.46, 0.98
    /// 
    /// Uses a step rotation system via animation_time_code to cycle through
    /// multiple simultaneous animations when several inputs are active.
    ///
    /// # Arguments
    /// * `handed` - Which controller to animate (Left or Right)
    /// * `controller` - Reference to the controller input state
    /// * `shift` - Time shift to apply (LEFT_SHIFT for left hand, 0.0 for right)
    fn set_animation(&mut self, handed: Handed, controller: &crate::system::Controller, shift: f32) {
        let mut animation_times = vec![];

        if handed == Handed::Left {
            self.animation_time_code = (self.animation_time_code.max(0.0) + 1.0) % 4.0;
        }

        if let Some(ref mut xr_render_model) = self.xr_render_model {
            // Check stick direction first (8 cardinal points)
            let stick_threshold = 0.25; // Threshold for stick activation
            if controller.stick.magnitude() > stick_threshold {
                let x = controller.stick.x;
                let y = controller.stick.y;

                // Map to 8 cardinal directions based on x/y dominance
                let animation_time = 
                    // Horizontal directions dominate
                    if x > 0.3 {
                        // right side
                        if y > 0.3 {
                            1.58  // Right-up direction
                        } else if y < -0.3 {
                            1.64  // Right-down direction
                        } else {
                            1.38  // Pure right direction
                        } 
                    } else if x < -0.3 {
                        // left side
                        if y > 0.3 {
                            1.52  // Left-up direction
                        } else if y < -0.3 {
                            1.46  // Left-down direction
                        } else {
                            1.32  // Pure left direction
                        }
                    } else {
                        // Center (vertical movements)
                        if y > 0.3 {
                            1.18  // Pure up direction
                        } else if y < -0.3 {
                            1.26  // Pure down direction
                        } else {
                            -1.0  // Center position (no movement)
                        }
                    };
                if animation_time > 0.0 { animation_times.push(animation_time); }
            }
            // Button-based animations with different combinations
            if controller.trigger > 0.1 {
                let animation_time = 0.6 + 0.06 * controller.trigger; // Variable trigger animation
                animation_times.push(animation_time);
            }
            if controller.grip > 0.1 {
                let animation_time = 0.82 + 0.06 * controller.grip; // Variable grip animation
                animation_times.push(animation_time);
            }

            // Discrete button animations
            let mut animation_time= -1.0;
            if controller.is_x1_pressed() && controller.is_x2_pressed() {
                animation_time = 0.46; // Both X/Y or A/B buttons pressed
            } else if controller.is_x1_pressed() {
                animation_time = 0.18; // Single X or A button pressed
            } else if controller.is_x2_pressed() {
                animation_time = 0.32; // Single Y or B button pressed
            } else if controller.is_stick_clicked() {
                animation_time = -1.0; // Stick click (no animation found)
            } else if Input::get_controller_menu_button().is_active() {
                animation_time = 0.98; // System/menu button pressed
            }

            if animation_time > 0.0 { animation_times.push(animation_time); }

            if animation_times.is_empty() {
                // No active inputs detected, use default idle animation
                xr_render_model.set_controller_anim_time(handed, 4.4);
            } else {
                // Multiple animations available - use step rotation to cycle through them
                let step_sel = self.animation_time_code % animation_times.len() as f32;
                for (i, animation_time) in animation_times.into_iter().enumerate() {
                    if i as f32 == step_sel{
                        xr_render_model.set_controller_anim_time(handed, animation_time + shift);
                        break;
                    }
                }
            }
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        if !self.is_enabled {
            return;
        }

        // Animation analysis helper (disabled by default)
        if false {
            self.animation_analyser(_token);
        } else if self.with_animation {
            // Execute controller animations based on current input states
            if self.xr_render_model.is_some() {
                // Apply animations to both controllers independently
                let left_controller = Input::controller(Handed::Left);
                self.set_animation(Handed::Left, &left_controller, LEFT_SHIFT);

                let right_controller = Input::controller(Handed::Right);
                self.set_animation(Handed::Right, &right_controller, 0.0);
            }
        }
    }

    /// Animation analysis helper for finding correct animation time codes
    ///
    /// This is a development tool that displays the current animation time code
    /// in 3D space and allows manual advancement through animations using joystick clicks.
    /// Useful for discovering and documenting animation time codes for different poses.
    ///
    /// # Arguments
    /// * `_token` - Main thread token for safe UI operations
    fn animation_analyser(&mut self, _token: &MainThreadToken) {
        // Advance animation time on joystick button press
        if Input::controller(Handed::Right).stick_click.is_just_active()
            || Input::controller(Handed::Left).stick_click.is_just_active()
        {
            self.animation_time_code += 0.02;

            // Reset to 0 when reaching 6 seconds maximum
            if self.animation_time_code >= 6.0 {
                self.animation_time_code = 0.0;
            }

            Log::diag(format!("Animation time code: {:.1}s", self.animation_time_code));
        }

        // Display current animation time code in 3D space
        use crate::maths::{Matrix, Quat, Vec3};
        use crate::system::Text;

        let text_content = format!("Animation Time: {:.2}s\nPress joystick to advance", self.animation_time_code);
        let position = Vec3::new(0.0, 1.5, -0.8);
        let rotation = Quat::from_angles(0.0, 180.0, 0.0); // Rotated toward Z-axis
        let transform = Matrix::t_r(position, rotation);

        Text::add_at(_token, &text_content, transform, None, None, None, None, None, None, None);

        // Apply current time code to both controllers for analysis
        if let Some(ref mut xr_render_model) = self.xr_render_model {
            xr_render_model.set_controller_anim_time(Handed::Left, self.animation_time_code);
            xr_render_model.set_controller_anim_time(Handed::Right, self.animation_time_code);
        }
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Disable controller models
            if let Some(ref mut xr_model) = self.xr_render_model {
                xr_model.disable_controller_models();
            }

            self.xr_render_model = None;
            self.shutdown_completed = true;
            true
        } else {
            self.shutdown_completed
        }
    }
}
