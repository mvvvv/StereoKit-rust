//! XR_FB_render_model extension implementation
//!
//! This module provides access to the OpenXR XR_FB_render_model extension,
//! which allows applications to retrieve render models for controllers and other devices.

use std::ffi::{CString, c_char};
use std::ptr;

use openxr_sys::{
    Instance, Path, RenderModelBufferFB, RenderModelFlagsFB, RenderModelKeyFB, RenderModelLoadInfoFB,
    RenderModelPathInfoFB, RenderModelPropertiesFB, Result as XrResult, Session, StructureType,
    pfn::{EnumerateRenderModelPathsFB, GetRenderModelPropertiesFB, LoadRenderModelFB, PathToString, StringToPath},
};

use crate::maths::units::CM;
use crate::maths::{Matrix, Quat, Vec3};
use crate::{
    model::Model,
    prelude::*,
    system::{Backend, BackendOpenXR, BackendXRType, Handed, Input, Log},
    util::Time,
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
    xr_enumerate_render_model_paths: Option<EnumerateRenderModelPathsFB>,
    xr_get_render_model_properties: Option<GetRenderModelPropertiesFB>,
    xr_load_render_model: Option<LoadRenderModelFB>,
    xr_string_to_path: Option<StringToPath>,
    xr_path_to_string: Option<PathToString>,
    instance: Instance,
    session: Session,
    // Cached controller models to avoid reloading
    left_controller_data: Option<Model>,
    right_controller_data: Option<Model>,
}

impl XrFbRenderModel {
    /// Creates a new XrFbRenderModel instance if the extension is supported
    pub fn new() -> Option<Self> {
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

        Some(Self {
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

        let result = unsafe { get_properties_fn(self.session, path, &mut properties) };

        if result != XrResult::SUCCESS {
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

        let mut properties_struct = RenderModelPropertiesFB {
            ty: StructureType::RENDER_MODEL_PROPERTIES_FB,
            next: ptr::null_mut(),
            vendor_id: 0,
            model_name: [0; 64],
            model_key: RenderModelKeyFB::from_raw(0),
            model_version: 0,
            flags: RenderModelFlagsFB::from_raw(0),
        };

        let result = unsafe { get_properties_fn(self.session, path, &mut properties_struct) };
        if result != XrResult::SUCCESS {
            return Err(result);
        }

        let model_key = properties_struct.model_key;

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

    /// Get cached left controller model using specified path, loading it if necessary
    pub fn get_left_controller_model(&mut self, model_path: &str) -> Result<&Model, Box<dyn std::error::Error>> {
        if self.left_controller_data.is_none() {
            let data = self.load_render_model(model_path)?;
            let left_model = Model::from_memory(format!("{model_path}.gltf"), &data, None)?;

            if let Some(mut n) = left_model.get_nodes().get_root_node() {
                let new_rotation = Quat::from_angles(0.0, 0.0, 0.0);
                let transf = Matrix::t_r(Vec3::new(0.0, 0.0 * CM, 0.0 * CM), new_rotation);
                n.local_transform(transf);
            }
            self.left_controller_data = Some(left_model);
        }
        Ok(self.left_controller_data.as_ref().unwrap())
    }

    /// Get cached right controller model using specified path, loading it if necessary
    pub fn get_right_controller_model(&mut self, model_path: &str) -> Result<&Model, Box<dyn std::error::Error>> {
        if self.right_controller_data.is_none() {
            let data = self.load_render_model(model_path)?;
            let right_model = Model::from_memory(format!("{model_path}.gltf"), &data, None)?;
            if let Some(mut n) = right_model.get_nodes().get_root_node() {
                let new_rotation = Quat::from_angles(0.0, 0.0, 0.0);
                let transf = Matrix::t_r(Vec3::new(0.0, 0.0 * CM, 0.0 * CM), new_rotation);
                n.local_transform(transf);
            }
            self.right_controller_data = Some(right_model);
        }
        Ok(self.right_controller_data.as_ref().unwrap())
    }

    /// Loads and configures controller models for both hands using specified paths
    pub fn setup_controller_models(&mut self, left_path: &str, right_path: &str) -> Result<(), XrResult> {
        // Load and set right controller model using specified path
        if let Ok(right_model) = self.get_right_controller_model(right_path) {
            Input::set_controller_model(Handed::Right, Some(right_model));
            Log::info(format!("Right controller model loaded and configured from path: {}", right_path));
        } else {
            Log::warn(format!("Failed to load right controller model from path: {}", right_path));
            return Err(XrResult::ERROR_RUNTIME_FAILURE);
        }

        // Load and set left controller model using specified path
        if let Ok(left_model) = self.get_left_controller_model(left_path) {
            Input::set_controller_model(Handed::Left, Some(left_model));
            Log::info(format!("Left controller model loaded and configured from path: {}", left_path));
        } else {
            Log::warn(format!("Failed to load left controller model from path: {}", left_path));
            return Err(XrResult::ERROR_RUNTIME_FAILURE);
        }

        Ok(())
    }

    /// Initialize controller models for animation using specified paths
    pub fn init_controller_animations(
        &mut self,
        left_path: &str,
        right_path: &str,
    ) -> Result<(Option<ControllerAnim>, Option<ControllerAnim>), Box<dyn std::error::Error>> {
        let mut left_controller = None;
        let mut right_controller = None;

        // Create right controller for animation using specified path
        if let Ok(_right_model) = self.get_right_controller_model(right_path) {
            right_controller = Some(ControllerAnim { current_animation: "Idle".to_string(), animation_time: 0.0 });
        }

        // Create left controller for animation using specified path
        if let Ok(_left_model) = self.get_left_controller_model(left_path) {
            left_controller = Some(ControllerAnim { current_animation: "Idle".to_string(), animation_time: 0.0 });
        }

        Ok((left_controller, right_controller))
    }

    /// Disables controller models by setting them to None
    pub fn disable_controller_models() {
        use crate::system::{Handed, Input};

        Input::set_controller_model(Handed::Right, None);
        Input::set_controller_model(Handed::Left, None);
        Log::info("Controller models disabled");
    }

    /// Explores and logs information about all available render models
    pub fn explore_render_models(&self) -> Result<(), XrResult> {
        if let Ok(paths) = self.enumerate_render_model_paths() {
            for path in paths {
                Log::diag(format!("Available render model: {}", path));

                if let Ok(properties) = self.get_render_model_properties(&path) {
                    Log::diag(format!("--Model: {}", properties.model_name));
                    Log::diag(format!("    Vendor ID: {}", properties.vendor_id));
                    Log::diag(format!("    Model version: {}", properties.model_version));
                    Log::diag(format!("    Model flags: 0x{:x}", properties.flags));
                } else {
                    Log::diag(format!("No connected device for model: {}", path));
                }
            }
        }
        Ok(())
    }
}

/// Convenience function to check if XR_FB_render_model extension is available
pub fn is_render_model_extension_available() -> bool {
    Backend::xr_type() == BackendXRType::OpenXR && BackendOpenXR::ext_enabled(XR_FB_RENDER_MODEL_EXTENSION_NAME)
}

/// Event key for enabling/disabling controller drawing
pub const DRAW_CONTROLLER: &str = "draw_controller";

/// Controller model with animation state
#[derive(Debug)]
pub struct ControllerAnim {
    pub current_animation: String,
    pub animation_time: f32,
}

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
///     tools::xr_fb_render_model::{XrFbRenderModelStepper, is_render_model_extension_available, DRAW_CONTROLLER},
///     system::{Input, Handed},
///     prelude::*,
/// };
///
/// // Check if the extension is available before using the stepper
/// if is_render_model_extension_available() {
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
#[derive(IStepper)]
pub struct XrFbRenderModelStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,
    shutdown_completed: bool,

    xr_render_model: Option<XrFbRenderModel>,
    is_enabled: bool,
    left_controller: Option<ControllerAnim>,
    right_controller: Option<ControllerAnim>,
    animation_time: f32,

    // Model paths for controllers
    pub left_controller_model_path: String,
    pub right_controller_model_path: String,
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
            left_controller: None,
            right_controller: None,
            animation_time: 0.0,
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
        match XrFbRenderModel::new() {
            Some(xr_model) => {
                Log::info("XR_FB_render_model extension initialized");

                // Explore available models
                if let Err(e) = xr_model.explore_render_models() {
                    Log::warn(format!("Failed to explore XR_FB_render_models: {:?}", e));
                }

                self.xr_render_model = Some(xr_model);
                true
            }
            None => {
                Log::err("XR_FB_render_model extension not available");
                false // Still succeed even if extension is not available
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
                        Log::diag("Controller drawing enabled via event");

                        // Load controller models if available
                        if let Some(ref mut xr_model) = self.xr_render_model {
                            // Setup controller models for both hands using configured paths
                            if let Err(e) = xr_model.setup_controller_models(
                                &self.left_controller_model_path,
                                &self.right_controller_model_path,
                            ) {
                                Log::warn(format!("Failed to setup controller models: {:?}", e));
                            } else {
                                Log::info("Controller models setup completed");

                                // Initialize animation controllers using configured paths
                                match xr_model.init_controller_animations(
                                    &self.left_controller_model_path,
                                    &self.right_controller_model_path,
                                ) {
                                    Ok((left, right)) => {
                                        self.left_controller = left;
                                        self.right_controller = right;
                                        Log::info("Controller animations initialized with configured paths");
                                    }
                                    Err(e) => {
                                        Log::warn(format!("Failed to initialize controller animations: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
                "false" => {
                    self.is_enabled = false;
                    Log::diag("Controller drawing disabled via event");

                    // Disable controller models and animations
                    if self.xr_render_model.is_some() {
                        XrFbRenderModel::disable_controller_models();
                        self.left_controller = None;
                        self.right_controller = None;
                    }
                }
                _ => {
                    Log::warn(format!("Unknown DRAW_CONTROLLER value: {}", value));
                }
            }
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        if !self.is_enabled {
            return;
        }

        // Update global animation time
        self.animation_time += Time::get_step_unscaledf();

        // Draw and animate right controller
        let right_controller = Input::controller(Handed::Right);
        if right_controller.tracked.is_active()
            && let Some(ref mut controller) = self.right_controller
        {
            // Determine animation based on controller state
            let new_animation = if right_controller.trigger > 0.5 {
                "ButtonPress"
            } else if right_controller.grip > 0.5 {
                "Grip"
            } else {
                "Idle"
            };

            // Update animation if it changed
            if controller.current_animation != new_animation {
                controller.current_animation = new_animation.to_string();
                controller.animation_time = 0.0;
            } else {
                controller.animation_time += Time::get_step_unscaledf();
            }
        }

        // Draw and animate left controller
        let left_controller = Input::controller(Handed::Left);
        if left_controller.tracked.is_active()
            && let Some(ref mut controller) = self.left_controller
        {
            // Determine animation based on controller state
            let new_animation = if left_controller.trigger > 0.5 {
                "ButtonPress"
            } else if left_controller.grip > 0.5 {
                "Grip"
            } else {
                "Idle"
            };

            // Update animation if it changed
            if controller.current_animation != new_animation {
                controller.current_animation = new_animation.to_string();
                controller.animation_time = 0.0;
            } else {
                controller.animation_time += Time::get_step_unscaledf();
            }
        }
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Disable controller models
            if self.xr_render_model.is_some() {
                XrFbRenderModel::disable_controller_models();
            }

            self.xr_render_model = None;
            self.left_controller = None;
            self.right_controller = None;
            self.shutdown_completed = true;
            Log::info("XR FB render model stepper closed");
            true
        } else {
            self.shutdown_completed
        }
    }
}
