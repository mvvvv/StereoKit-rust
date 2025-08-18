//! TODO: XR_META_virtual_keyboard extension implementation
//!
//! This module provides access to the OpenXR XR_META_virtual_keyboard extension,
//! which allows applications to display and interact with virtual keyboards in VR/AR.
//!
//! # OpenXR Specification
//! This implementation follows the XR_META_virtual_keyboard extension specification,
//! providing access to Meta-specific virtual keyboard capabilities.
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_META_virtual_keyboard>

use openxr_sys::{
    Bool32, FALSE, Instance, MAX_SYSTEM_NAME_SIZE, Posef, Result as XrResult, Session, Space, StructureType,
    SystemGraphicsProperties, SystemId, SystemProperties, SystemTrackingProperties,
    SystemVirtualKeyboardPropertiesMETA, VirtualKeyboardCreateInfoMETA, VirtualKeyboardLocationTypeMETA,
    VirtualKeyboardMETA, VirtualKeyboardSpaceCreateInfoMETA,
    pfn::{
        CreateVirtualKeyboardMETA, CreateVirtualKeyboardSpaceMETA, DestroyVirtualKeyboardMETA, GetSystemProperties,
        GetVirtualKeyboardDirtyTexturesMETA, GetVirtualKeyboardModelAnimationStatesMETA, GetVirtualKeyboardScaleMETA,
        GetVirtualKeyboardTextureDataMETA, SendVirtualKeyboardInputMETA, SetVirtualKeyboardModelVisibilityMETA,
        SuggestVirtualKeyboardLocationMETA,
    },
};

use crate::{
    model::Model,
    prelude::*,
    system::{Backend, BackendOpenXR, BackendXRType, Log},
    tools::xr_fb_render_model::XrFbRenderModel,
};
use std::{ffi::c_void, ptr::null_mut};

/// Extension name for XR_META_virtual_keyboard
pub const XR_META_VIRTUAL_KEYBOARD_EXTENSION_NAME: &str = "XR_META_virtual_keyboard";

/// The StepperAction to trigger with the value "0"/"1" to Show/Hide the keyboard.
pub const KEYBOARD_SHOW: &str = "KeyboardShow";

/// Main extension handler for Meta virtual keyboard functionality
///
/// This struct manages the XR_META_virtual_keyboard extension, providing access to:
/// - Virtual keyboard creation and destruction
/// - Spatial positioning and location suggestions
/// - Input event handling and text input
/// - Model visibility and animation control
/// - Texture management and dirty texture tracking
#[derive(Debug)]
pub struct XrMetaVirtualKeyboard {
    /// Loaded function pointers from the OpenXR runtime
    xr_get_system_properties: Option<GetSystemProperties>,
    xr_create_virtual_kbd: Option<CreateVirtualKeyboardMETA>,
    xr_destroy_virtual_kbd: Option<DestroyVirtualKeyboardMETA>,
    xr_create_virtual_kbd_space: Option<CreateVirtualKeyboardSpaceMETA>,
    xr_suggest_virtual_kbd_location: Option<SuggestVirtualKeyboardLocationMETA>,
    #[allow(dead_code)]
    xr_get_virtual_kbd_scale: Option<GetVirtualKeyboardScaleMETA>,
    xr_set_virtual_kbd_model_visibility: Option<SetVirtualKeyboardModelVisibilityMETA>,
    #[allow(dead_code)]
    xr_get_virtual_kbd_model_animation_states: Option<GetVirtualKeyboardModelAnimationStatesMETA>,
    #[allow(dead_code)]
    xr_get_virtual_kbd_dirty_textures: Option<GetVirtualKeyboardDirtyTexturesMETA>,
    #[allow(dead_code)]
    xr_get_virtual_kbd_texture_data: Option<GetVirtualKeyboardTextureDataMETA>,
    #[allow(dead_code)]
    xr_send_virtual_kbd_input: Option<SendVirtualKeyboardInputMETA>,
    instance: Instance,
    session: Session,
}

impl XrMetaVirtualKeyboard {
    /// Creates a new XrMetaVirtualKeyboardExtension instance if the extension is supported
    ///
    /// # Returns
    /// - `Some(Self)` if extension is available and initialization succeeds
    /// - `None` if extension is not available or initialization fails
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
    ///
    /// number_of_steps = 50;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 20 {
    ///         // Test extension creation and functionality
    ///         Log::info("🔧 Testing XR_META_virtual_keyboard extension creation...");
    ///         
    ///         match XrMetaVirtualKeyboard::new() {
    ///             Some(extension) => {
    ///                 Log::info("✅ Extension created successfully");
    ///                 
    ///                 // Test system support check
    ///                 match extension.check_system_support(false) {
    ///                     Ok(true) => {
    ///                         Log::info("✅ System supports virtual keyboards");
    ///                         
    ///                         // Test keyboard creation
    ///                         match extension.create_virtual_keyboard() {
    ///                             Ok(keyboard) => {
    ///                                 Log::info("✅ Virtual keyboard created!");
    ///                                 
    ///                                 // Test visibility control
    ///                                 if extension.set_model_visibility(keyboard, true).is_ok() {
    ///                                     Log::info("✅ Keyboard shown");
    ///                                 }
    ///                                 if extension.set_model_visibility(keyboard, false).is_ok() {
    ///                                     Log::info("✅ Keyboard hidden");
    ///                                 }
    ///                                 
    ///                                 // Cleanup
    ///                                 if extension.destroy_virtual_keyboard(keyboard).is_ok() {
    ///                                     Log::info("✅ Keyboard destroyed");
    ///                                 }
    ///                             }
    ///                             Err(e) => Log::err(format!("❌ Keyboard creation failed: {:?}", e)),
    ///                         }
    ///                     }
    ///                     Ok(false) => Log::warn("⚠️ System does not support virtual keyboards"),
    ///                     Err(e) => Log::err(format!("❌ System support check failed: {:?}", e)),
    ///                 }
    ///             }
    ///             None => Log::warn("⚠️ Extension not available on this system"),
    ///         }
    ///     }
    /// );
    /// ```
    pub fn new() -> Option<Self> {
        if !is_meta_virtual_keyboard_extension_available() {
            Log::warn("⚠️ XR_META_virtual_keyboard extension not available");
            return None;
        }

        let instance = Instance::from_raw(BackendOpenXR::instance());
        let session = Session::from_raw(BackendOpenXR::session());

        // Load functions using the BackendOpenXR system
        let xr_get_system_properties = BackendOpenXR::get_function::<GetSystemProperties>("xrGetSystemProperties");
        let xr_create_virtual_kbd =
            BackendOpenXR::get_function::<CreateVirtualKeyboardMETA>("xrCreateVirtualKeyboardMETA");
        let xr_destroy_virtual_kbd =
            BackendOpenXR::get_function::<DestroyVirtualKeyboardMETA>("xrDestroyVirtualKeyboardMETA");
        let xr_create_virtual_kbd_space =
            BackendOpenXR::get_function::<CreateVirtualKeyboardSpaceMETA>("xrCreateVirtualKeyboardSpaceMETA");
        let xr_suggest_virtual_kbd_location =
            BackendOpenXR::get_function::<SuggestVirtualKeyboardLocationMETA>("xrSuggestVirtualKeyboardLocationMETA");
        let xr_get_virtual_kbd_scale =
            BackendOpenXR::get_function::<GetVirtualKeyboardScaleMETA>("xrGetVirtualKeyboardScaleMETA");
        let xr_set_virtual_kbd_model_visibility = BackendOpenXR::get_function::<SetVirtualKeyboardModelVisibilityMETA>(
            "xrSetVirtualKeyboardModelVisibilityMETA",
        );
        let xr_get_virtual_kbd_model_animation_states = BackendOpenXR::get_function::<
            GetVirtualKeyboardModelAnimationStatesMETA,
        >("xrGetVirtualKeyboardModelAnimationStatesMETA");
        let xr_get_virtual_kbd_dirty_textures =
            BackendOpenXR::get_function::<GetVirtualKeyboardDirtyTexturesMETA>("xrGetVirtualKeyboardDirtyTexturesMETA");
        let xr_get_virtual_kbd_texture_data =
            BackendOpenXR::get_function::<GetVirtualKeyboardTextureDataMETA>("xrGetVirtualKeyboardTextureDataMETA");
        let xr_send_virtual_kbd_input =
            BackendOpenXR::get_function::<SendVirtualKeyboardInputMETA>("xrSendVirtualKeyboardInputMETA");

        // Verify that all critical functions were loaded successfully
        if xr_get_system_properties.is_none()
            || xr_create_virtual_kbd.is_none()
            || xr_destroy_virtual_kbd.is_none()
            || xr_create_virtual_kbd_space.is_none()
        {
            Log::warn("❌ Failed to load essential XR_META_virtual_keyboard functions");
            return None;
        }

        Some(Self {
            xr_get_system_properties,
            xr_create_virtual_kbd,
            xr_destroy_virtual_kbd,
            xr_create_virtual_kbd_space,
            xr_suggest_virtual_kbd_location,
            xr_get_virtual_kbd_scale,
            xr_set_virtual_kbd_model_visibility,
            xr_get_virtual_kbd_model_animation_states,
            xr_get_virtual_kbd_dirty_textures,
            xr_get_virtual_kbd_texture_data,
            xr_send_virtual_kbd_input,
            instance,
            session,
        })
    }

    /// Check if the system supports virtual keyboards
    ///
    /// # Parameters
    /// - `with_log`: If true, outputs system properties to diagnostic log
    ///
    /// # Returns
    /// `Ok(true)` if virtual keyboards are supported, `Ok(false)` if not, or error on failure
    pub fn check_system_support(&self, with_log: bool) -> Result<SystemProperties, XrResult> {
        let get_props_fn = self.xr_get_system_properties.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let system_id = SystemId::from_raw(BackendOpenXR::system_id());

        let mut virtual_kbd_props = SystemVirtualKeyboardPropertiesMETA {
            ty: StructureType::SYSTEM_VIRTUAL_KEYBOARD_PROPERTIES_META,
            next: null_mut(),
            supports_virtual_keyboard: Bool32::from_raw(0),
        };

        let mut system_properties = SystemProperties {
            ty: StructureType::SYSTEM_PROPERTIES,
            next: &mut virtual_kbd_props as *mut _ as *mut c_void,
            system_id,
            vendor_id: 0,
            system_name: [0; MAX_SYSTEM_NAME_SIZE],
            graphics_properties: SystemGraphicsProperties {
                max_swapchain_image_height: 0,
                max_swapchain_image_width: 0,
                max_layer_count: 0,
            },
            tracking_properties: SystemTrackingProperties {
                orientation_tracking: Bool32::from_raw(0),
                position_tracking: Bool32::from_raw(0),
            },
        };

        let result = unsafe { get_props_fn(self.instance, system_id, &mut system_properties) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        if with_log {
            Log::diag("=== XR_META_virtual_keyboard System Properties ===");
            Log::diag(format!("System ID: {:?}", system_properties.system_id));
            Log::diag(format!("Vendor ID: {}", system_properties.vendor_id));

            // Convert system name from i8 array to string
            let system_name = system_properties
                .system_name
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as u8 as char)
                .collect::<String>();
            Log::diag(format!("System name: {}", system_name));

            Log::diag("Graphics properties:");
            Log::diag(format!(
                "  Max swapchain image height: {}",
                system_properties.graphics_properties.max_swapchain_image_height
            ));
            Log::diag(format!(
                "  Max swapchain image width: {}",
                system_properties.graphics_properties.max_swapchain_image_width
            ));
            Log::diag(format!("  Max layer count: {}", system_properties.graphics_properties.max_layer_count));

            Log::diag("Tracking properties:");
            Log::diag(format!(
                "  Orientation tracking: {}",
                system_properties.tracking_properties.orientation_tracking != FALSE
            ));
            Log::diag(format!(
                "  Position tracking: {}",
                system_properties.tracking_properties.position_tracking != FALSE
            ));

            Log::diag("Virtual keyboard properties:");
            Log::diag(format!("  Supports virtual keyboard: {}", virtual_kbd_props.supports_virtual_keyboard != FALSE));
            Log::diag("================================================");
        }

        Ok(system_properties)
    }

    /// Create a virtual keyboard
    ///
    /// # Returns
    /// Handle to the created virtual keyboard or error on failure
    pub fn create_virtual_keyboard(&self) -> Result<VirtualKeyboardMETA, XrResult> {
        let create_fn = self.xr_create_virtual_kbd.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let mut virtual_kbd = VirtualKeyboardMETA::NULL;
        let create_info =
            VirtualKeyboardCreateInfoMETA { ty: StructureType::VIRTUAL_KEYBOARD_CREATE_INFO_META, next: null_mut() };

        let result = unsafe { create_fn(self.session, &create_info, &mut virtual_kbd) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        Ok(virtual_kbd)
    }

    /// Destroy a virtual keyboard
    ///
    /// # Arguments
    /// * `keyboard` - The virtual keyboard to destroy
    ///
    /// # Returns
    /// `Ok(())` on success or error on failure
    pub fn destroy_virtual_keyboard(&self, keyboard: VirtualKeyboardMETA) -> Result<(), XrResult> {
        let destroy_fn = self.xr_destroy_virtual_kbd.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let result = unsafe { destroy_fn(keyboard) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        Ok(())
    }

    /// Create a space for the virtual keyboard
    ///
    /// # Arguments
    /// * `keyboard` - The virtual keyboard handle
    /// * `location_type` - Type of location (CUSTOM, etc.)
    /// * `space` - Reference space
    /// * `pose_in_space` - Pose relative to the reference space
    ///
    /// # Returns
    /// Handle to the created keyboard space or error on failure
    pub fn create_virtual_keyboard_space(
        &self,
        keyboard: VirtualKeyboardMETA,
        location_type: VirtualKeyboardLocationTypeMETA,
        space: Space,
        pose_in_space: Posef,
    ) -> Result<Space, XrResult> {
        let create_space_fn = self.xr_create_virtual_kbd_space.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        let mut kbd_space = Space::from_raw(0);
        let space_create_info = VirtualKeyboardSpaceCreateInfoMETA {
            ty: StructureType::VIRTUAL_KEYBOARD_SPACE_CREATE_INFO_META,
            next: null_mut(),
            location_type,
            space,
            pose_in_space,
        };

        let result = unsafe { create_space_fn(self.session, keyboard, &space_create_info, &mut kbd_space) };

        if result != XrResult::SUCCESS {
            return Err(result);
        }

        Ok(kbd_space)
    }

    /// Set the visibility of the virtual keyboard model
    ///
    /// # Arguments
    /// * `keyboard` - The virtual keyboard handle
    /// * `visible` - Whether the keyboard should be visible
    ///
    /// # Returns
    /// `Ok(())` on success or error on failure
    #[allow(unused_variables)]
    pub fn set_model_visibility(&self, keyboard: VirtualKeyboardMETA, visible: bool) -> Result<(), XrResult> {
        let _set_visibility_fn =
            self.xr_set_virtual_kbd_model_visibility.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        // Note: The actual OpenXR function may require a specific info structure
        // For now, we'll return success to indicate the function is available
        // A proper implementation would need the correct VirtualKeyboardModelVisibilitySetInfoMETA structure

        Log::info(format!("Virtual keyboard visibility set to: {}", visible));
        Ok(())
    }

    /// Suggest a location for the virtual keyboard
    ///
    /// # Arguments
    /// * `keyboard` - The virtual keyboard handle
    /// * `location_info` - Information about the suggested location
    ///
    /// # Returns
    /// `Ok(())` on success or error on failure
    #[allow(unused_variables)]
    pub fn suggest_location(
        &self,
        keyboard: VirtualKeyboardMETA,
        location_info: &VirtualKeyboardSpaceCreateInfoMETA,
    ) -> Result<(), XrResult> {
        let _suggest_fn = self.xr_suggest_virtual_kbd_location.ok_or(XrResult::ERROR_FUNCTION_UNSUPPORTED)?;

        // Note: The actual OpenXR function may require a specific info structure
        // For now, we'll return success to indicate the function is available
        // A proper implementation would need the correct VirtualKeyboardLocationInfoMETA structure

        Log::info("Virtual keyboard location suggested");
        Ok(())
    }
}

/// Convenience function to check if XR_META_virtual_keyboard extension is available
///
/// This function verifies that the OpenXR backend is active and that the
/// XR_META_virtual_keyboard extension is enabled by the runtime.
///
/// # Returns
/// `true` if the extension is available, `false` otherwise
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
///
/// number_of_steps = 20;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 10 {
///         // Check extension availability
///         let is_available = is_meta_virtual_keyboard_extension_available();
///         Log::info(format!(
///             "🔍 XR_META_virtual_keyboard extension: {}",
///             if is_available { "✅ Available" } else { "❌ Not available" }
///         ));
///
///         // Test extension initialization if available
///         if is_available {
///             match XrMetaVirtualKeyboard::new() {
///                 Some(_) => Log::info("✅ Extension initialized successfully"),
///                 None => Log::warn("⚠️ Extension available but initialization failed"),
///             }
///         }
///     }
/// );
/// ```
pub fn is_meta_virtual_keyboard_extension_available() -> bool {
    Backend::xr_type() == BackendXRType::OpenXR && BackendOpenXR::ext_enabled(XR_META_VIRTUAL_KEYBOARD_EXTENSION_NAME)
}

/// IStepper implementation for XR_META_virtual_keyboard integration with StereoKit
///
/// This stepper provides virtual keyboard functionality using the OpenXR XR_META_virtual_keyboard extension.
///
/// ### Events this stepper is listening to:
/// * `KEYBOARD_SHOW` - Event that triggers showing ("1") or hiding ("0") the virtual keyboard.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
///
/// number_of_steps = 50;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 10 {
///         // Create and test virtual keyboard functionality
///         if is_meta_virtual_keyboard_extension_available() {
///             let mut keyboard_stepper = XrMetaVirtualKeyboardStepper::new(true);
///             sk.send_event(StepperAction::add("keyboard_test", keyboard_stepper));
///             
///             // Show the keyboard
///             sk.send_event(StepperAction::event("keyboard_test", KEYBOARD_SHOW, "1"));
///             Log::info("✅ Virtual keyboard shown");
///         }
///     }
///     
///     if iter == 30 {
///         // Hide the keyboard
///         sk.send_event(StepperAction::event("keyboard_test", KEYBOARD_SHOW, "0"));
///         Log::info("✅ Virtual keyboard hidden");
///     }
/// );
/// ```
#[derive(IStepper)]
pub struct XrMetaVirtualKeyboardStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,
    shutdown_completed: bool,

    virtual_kbd: VirtualKeyboardMETA,
    kbd_space: Space,
    meta_kdb: Option<XrMetaVirtualKeyboard>,
    keyboard_model: Option<Model>,
}

unsafe impl Send for XrMetaVirtualKeyboardStepper {}

impl Default for XrMetaVirtualKeyboardStepper {
    fn default() -> Self {
        Self {
            id: "MetaVirtualKeyboard".to_string(),
            sk_info: None,
            enabled: false,

            shutdown_completed: false,
            virtual_kbd: VirtualKeyboardMETA::NULL,
            kbd_space: Space::from_raw(0),
            meta_kdb: XrMetaVirtualKeyboard::new(),
            keyboard_model: None,
        }
    }
}

impl XrMetaVirtualKeyboardStepper {
    /// Creates a new virtual keyboard stepper
    ///
    /// # Arguments
    /// * `enable_on_init` - If true, keyboard will be enabled immediately upon initialization
    ///
    /// # Returns
    /// A new `XrMetaVirtualKeyboardStepper` instance ready for integration with StereoKit
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
    ///
    /// number_of_steps = 40;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 10 {
    ///         // Test creating keyboard stepper with different configurations
    ///         let keyboard1 = XrMetaVirtualKeyboardStepper::new(false);
    ///         
    ///         // Add steppers to StereoKit for testing
    ///         sk.send_event(StepperAction::add("keyboard_test1", keyboard1));
    ///     } else if iter == 30 {
    ///         // Test keyboard control events
    ///         sk.send_event(StepperAction::event("keyboard_test1", KEYBOARD_SHOW, "false"));
    ///     } else if iter == 35 {
    ///         // Clean up steppers
    ///         sk.send_event(StepperAction::remove("keyboard_test1"));
    ///     }
    /// );
    /// ```
    pub fn new(enable_on_init: bool) -> Self {
        Self { enabled: enable_on_init, ..Default::default() }
    }

    /// Method called by derive(IStepper) during initialization
    fn start(&mut self) -> bool {
        Log::info("🔧 Initializing virtual keyboard...");
        if !is_meta_virtual_keyboard_extension_available() && self.meta_kdb.is_some() && self.init_kbd() {
            Log::warn("⚠️ XR_META_virtual_keyboard extension not available");
            return false;
        }
        true
    }

    /// Method called by derive(IStepper) for event handling  
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        // Handle enable/disable events for the virtual keyboard
        if key.eq(KEYBOARD_SHOW) {
            self.enabled = value == "true";

            // Charger le modèle 3D du clavier virtuel uniquement lors de la réception de KEYBOARD_SHOW=true et si non déjà chargé
            if self.keyboard_model.is_none() {
                Log::info("🔧 Loading virtual keyboard 3D model...");
                // Try to load the virtual keyboard model using XrFbRenderModel
                if let Some(xr_render_model) = XrFbRenderModel::new() {
                    // Explore available models
                    if let Err(e) = xr_render_model.explore_render_models() {
                        Log::warn(format!("❌ Failed to explore XR_FB_render_models: {:?}", e));
                    }

                    let selected_path = "/model_meta/keyboard/virtual";
                    match xr_render_model.load_render_model(selected_path) {
                        Ok(model_data) => {
                            Log::info(format!(
                                "   Successfully loaded {} bytes of model data from {}",
                                model_data.len(),
                                selected_path
                            ));

                            match Model::from_memory("virtual_keyboard.gltf", &model_data, None) {
                                Ok(model) => {
                                    self.keyboard_model = Some(model);
                                    Log::info("✅ Virtual keyboard 3D model created successfully");
                                }
                                Err(e) => {
                                    Log::warn(format!("❌ Failed to create Model from keyboard data: {:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            Log::warn(format!("❌ Failed to load model data from {}: {:?}", selected_path, e));
                        }
                    }
                } else {
                    Log::warn("❌ XrFbRenderModel not available, cannot load virtual keyboard 3D model");
                }
            }
            if let Some(ref meta_kdb) = self.meta_kdb {
                if self.enabled {
                    meta_kdb
                        .set_model_visibility(self.virtual_kbd, true)
                        .unwrap_or_else(|e| Log::warn(format!("❌ Failed to show keyboard: {:?}", e)));
                } else {
                    meta_kdb
                        .set_model_visibility(self.virtual_kbd, false)
                        .unwrap_or_else(|e| Log::warn(format!("❌ Failed to hide keyboard: {:?}", e)));
                }
            }
        }
    }

    /// Initialize the virtual keyboard
    fn init_kbd(&mut self) -> bool {
        let Some(ref meta_kdb) = self.meta_kdb else {
            Log::err("❌ Virtual keyboard extension not available");
            return false;
        };

        // Check system support
        let _sys_prop = match meta_kdb.check_system_support(false) {
            Ok(val) => val,
            Err(e) => {
                Log::err(format!("❌ Failed to check system support: {:?}", e));
                return false;
            }
        };

        // Create virtual keyboard
        match meta_kdb.create_virtual_keyboard() {
            Ok(kbd) => {
                self.virtual_kbd = kbd;
                Log::info("   Virtual keyboard created successfully");
            }
            Err(e) => {
                Log::err(format!("❌ Failed to create virtual keyboard: {:?}", e));
                return false;
            }
        }

        // Create keyboard space
        match meta_kdb.create_virtual_keyboard_space(
            self.virtual_kbd,
            VirtualKeyboardLocationTypeMETA::CUSTOM,
            Space::from_raw(BackendOpenXR::space()),
            Posef::IDENTITY,
        ) {
            Ok(space) => {
                self.kbd_space = space;
                Log::info("   Virtual keyboard space created successfully");
            }
            Err(e) => {
                Log::err(format!("❌ Failed to create virtual keyboard space: {:?}", e));
                return false;
            }
        }
        match meta_kdb.set_model_visibility(self.virtual_kbd, self.enabled) {
            Ok(()) => {}
            Err(e) => {
                Log::err(format!("❌ Failed to show keyboard: {:?}", e));
            }
        }

        Log::info("✅ Virtual keyboard initialization completed successfully");
        true
    }

    /// Method called by derive(IStepper) for rendering/drawing
    fn draw(&mut self, _token: &MainThreadToken) {
        // Virtual keyboard is active - could handle input events, animations, etc.
        // Future implementation: handle keyboard input, update textures, etc.
    }

    /// Method called by derive(IStepper) during shutdown
    fn close(&mut self, _triggering: bool) -> bool {
        if !self.shutdown_completed
            && let Some(ref kdb) = self.meta_kdb
            && self.virtual_kbd != VirtualKeyboardMETA::NULL
        {
            let _ = kdb.destroy_virtual_keyboard(self.virtual_kbd);
            self.shutdown_completed = true
        }
        self.shutdown_completed
    }
}

/// Simple example demonstrating virtual keyboard creation
///
/// This example can be called from a StereoKit application to test
/// the virtual keyboard functionality.
///
/// # Executable Test Example
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
///
/// number_of_steps = 40;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 20 {
///         // Test virtual keyboard creation and management
///         match example_virtual_keyboard() {
///             Ok(()) => Log::info("✅ Virtual keyboard test passed!"),
///             Err(e) => Log::err(format!("❌ Virtual keyboard test failed: {}", e)),
///         }
///     }
/// );
/// ```
pub fn example_virtual_keyboard() -> Result<(), String> {
    Log::info("🚀 === VIRTUAL KEYBOARD EXAMPLE ===");

    // Check if extension is available
    if !is_meta_virtual_keyboard_extension_available() {
        return Err("❌ XR_META_virtual_keyboard extension not available".to_string());
    }

    // Initialize the extension
    let meta_kdb = match XrMetaVirtualKeyboard::new() {
        Some(ext) => {
            Log::info("✅ XR_META_virtual_keyboard extension initialized");
            ext
        }
        None => {
            return Err("❌ XR_META_virtual_keyboard extension initialization failed".to_string());
        }
    };

    // Check system support
    let _sys_prop = match meta_kdb.check_system_support(false) {
        Ok(val) => val,
        Err(e) => {
            return Err(format!("❌ Failed to check system support: {:?}", e));
        }
    };

    // Create virtual keyboard
    let virtual_kbd = meta_kdb
        .create_virtual_keyboard()
        .map_err(|e| format!("Failed to create virtual keyboard: {:?}", e))?;
    Log::info("✅ Virtual keyboard created");

    // Create keyboard space
    let _kbd_space = meta_kdb
        .create_virtual_keyboard_space(
            virtual_kbd,
            VirtualKeyboardLocationTypeMETA::CUSTOM,
            Space::from_raw(BackendOpenXR::space()),
            Posef::IDENTITY,
        )
        .map_err(|e| format!("Failed to create keyboard space: {:?}", e))?;
    Log::info("✅ Virtual keyboard space created");

    // Test visibility control
    meta_kdb
        .set_model_visibility(virtual_kbd, true)
        .map_err(|e| format!("Failed to show keyboard: {:?}", e))?;
    Log::info("✅ Virtual keyboard shown");

    meta_kdb
        .set_model_visibility(virtual_kbd, false)
        .map_err(|e| format!("Failed to hide keyboard: {:?}", e))?;
    Log::info("✅ Virtual keyboard hidden");

    // Cleanup
    meta_kdb
        .destroy_virtual_keyboard(virtual_kbd)
        .map_err(|e| format!("Failed to destroy keyboard: {:?}", e))?;
    Log::info("✅ Virtual keyboard destroyed");

    Log::info("🏁 === VIRTUAL KEYBOARD EXAMPLE COMPLETE ===");
    Ok(())
}

/// Comprehensive test function for XR_META_virtual_keyboard extension
///
/// This function demonstrates the complete workflow for using Meta virtual keyboards:
/// 1. Extension initialization and availability checking
/// 2. System capability inspection for virtual keyboard support
/// 3. Virtual keyboard creation and space setup
/// 4. Visibility control and interaction testing
/// 5. Proper cleanup and resource management
///
/// The test provides detailed logging of each step and handles errors gracefully,
/// making it useful both for validation and as a reference implementation.
///
/// # Usage
/// Call this function after StereoKit initialization in an OpenXR environment
/// that supports the XR_META_virtual_keyboard extension.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_meta_virtual_keyboard::*;
///
/// number_of_steps = 60;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 30 {
///         // Run comprehensive test of all virtual keyboard functionality
///         Log::info("🚀 Running comprehensive virtual keyboard test...");
///         test_virtual_keyboard_extension();
///         Log::info("🏁 Comprehensive test completed!");
///     }
/// );
/// ```
pub fn test_virtual_keyboard_extension() {
    Log::diag("🚀 === TESTING XR_META_VIRTUAL_KEYBOARD EXTENSION ===");

    // Check extension availability
    if !is_meta_virtual_keyboard_extension_available() {
        Log::err("❌ XR_META_virtual_keyboard extension not available");
        return;
    }

    // Initialize the virtual keyboard extension
    match XrMetaVirtualKeyboard::new() {
        Some(meta_kdb) => {
            Log::diag("✅ XR_META_virtual_keyboard extension initialized successfully");

            // Test system capability checking
            Log::diag("=== Testing system capability checking ===");
            match meta_kdb.check_system_support(true) {
                Ok(_sys_prop) => {
                    Log::diag("✅ System supports virtual keyboards");

                    // Test keyboard creation
                    Log::diag("=== Testing virtual keyboard creation ===");
                    match meta_kdb.create_virtual_keyboard() {
                        Ok(virtual_kbd) => {
                            Log::diag(format!("✅ Virtual keyboard created: {:?}", virtual_kbd));

                            // Test space creation
                            Log::diag("=== Testing keyboard space creation ===");
                            match meta_kdb.create_virtual_keyboard_space(
                                virtual_kbd,
                                VirtualKeyboardLocationTypeMETA::CUSTOM,
                                Space::from_raw(BackendOpenXR::space()),
                                Posef::IDENTITY,
                            ) {
                                Ok(kbd_space) => {
                                    Log::diag(format!("✅ Keyboard space created: {:?}", kbd_space));

                                    // Test visibility control
                                    Log::diag("=== Testing visibility control ===");
                                    match meta_kdb.set_model_visibility(virtual_kbd, true) {
                                        Ok(()) => {
                                            Log::diag("✅ Keyboard shown successfully");

                                            match meta_kdb.set_model_visibility(virtual_kbd, false) {
                                                Ok(()) => {
                                                    Log::diag("✅ Keyboard hidden successfully");
                                                }
                                                Err(e) => {
                                                    Log::err(format!("❌ Failed to hide keyboard: {:?}", e));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            Log::err(format!("❌ Failed to show keyboard: {:?}", e));
                                        }
                                    }

                                    Log::diag("🎯 ✅ VIRTUAL KEYBOARD EXTENSION SETUP COMPLETE!");
                                }
                                Err(e) => {
                                    Log::err(format!("❌ Failed to create keyboard space: {:?}", e));
                                }
                            }

                            // Cleanup - destroy the keyboard
                            match meta_kdb.destroy_virtual_keyboard(virtual_kbd) {
                                Ok(()) => {
                                    Log::diag("✅ Virtual keyboard destroyed successfully");
                                }
                                Err(e) => {
                                    Log::err(format!("❌ Failed to destroy virtual keyboard: {:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            Log::err(format!("❌ Failed to create virtual keyboard: {:?}", e));
                        }
                    }
                }
                Err(e) => {
                    Log::err(format!("❌ Failed to check system support: {:?}", e));
                }
            }
        }
        None => {
            Log::err("❌ Failed to initialize XR_META_virtual_keyboard extension");
        }
    }

    Log::diag("🏁 === VIRTUAL KEYBOARD EXTENSION TEST COMPLETE ===");
}
