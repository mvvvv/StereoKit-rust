//! TODO: XR_ANDROID_depth_texture extension implementation (AndroidXR)
//!
//! This module provides access to the OpenXR XR_ANDROID_depth_texture extension,
//! which allows Android applications to access depth and confidence texture data
//! from the device's depth sensors.
//!
//! The extension provides both raw and smooth depth data, along with confidence
//! values for each pixel, enabling advanced computer vision and depth-aware
//! rendering applications.
//!
//! # Features
//! - **Multiple Resolution Support**: Quarter, half, and full resolution depth textures
//! - **Raw and Processed Data**: Access to both raw sensor data and smoothed depth information
//! - **Confidence Values**: Per-pixel confidence metrics for depth accuracy assessment
//! - **Swapchain Management**: Efficient depth texture rendering through OpenXR swapchains
//! - **System Capability Detection**: Runtime checking for depth tracking support
//!
//!
//! # OpenXR Specification
//! This implementation follows the XR_ANDROID_depth_texture extension specification
//! version 1, providing access to Android-specific depth sensor capabilities.
//! <https://developer.android.com/develop/xr/openxr/extensions/XR_ANDROID_depth_texture>

use crate::system::{Backend, BackendOpenXR, BackendXRType, Log};
use openxr_sys::{
    Bool32, Instance, MAX_SYSTEM_NAME_SIZE, Result, Session, StructureType, Swapchain, SystemGraphicsProperties,
    SystemId, SystemProperties, SystemTrackingProperties, pfn::GetSystemProperties,
};
use std::os::raw::{c_uint, c_ulong, c_void};
use std::ptr::null_mut;

/// Extension name for XR_FB_render_model
pub const XR_ANDROID_DEPTH_TEXTURE_EXTENSION_NAME: &str = "XR_ANDROID_depth_texture";

/// Type definitions for XR_ANDROID_depth_texture (openxr_sys compatible)
pub type SurfaceOriginAndroid = c_uint;
pub type ViewConfigurationType = c_uint;
pub type SessionState = c_uint;
pub type SpaceLocationFlags = c_ulong;
pub type ReferenceSpaceType = c_uint;
pub type ActionType = c_uint;
pub type DepthResolutionAndroid = c_uint;
pub type DepthSwapchainCreateFlagsAndroid = c_ulong;

/// Surface origin constants for Android depth textures
pub const SURFACE_ORIGIN_TOP_LEFT_ANDROID: SurfaceOriginAndroid = 0;
pub const SURFACE_ORIGIN_BOTTOM_LEFT_ANDROID: SurfaceOriginAndroid = 1;

/// Depth swapchain creation flags for different types of depth data
/// These flags control which depth and confidence images are generated
pub const XR_DEPTH_SWAPCHAIN_CREATE_SMOOTH_DEPTH_IMAGE_BIT_ANDROID: DepthSwapchainCreateFlagsAndroid = 0x00000001;
pub const XR_DEPTH_SWAPCHAIN_CREATE_SMOOTH_CONFIDENCE_IMAGE_BIT_ANDROID: DepthSwapchainCreateFlagsAndroid = 0x00000002;
pub const XR_DEPTH_SWAPCHAIN_CREATE_RAW_DEPTH_IMAGE_BIT_ANDROID: DepthSwapchainCreateFlagsAndroid = 0x00000004;
pub const XR_DEPTH_SWAPCHAIN_CREATE_RAW_CONFIDENCE_IMAGE_BIT_ANDROID: DepthSwapchainCreateFlagsAndroid = 0x00000008;

/// Depth resolution enumeration values
/// These define the available depth texture resolutions relative to the main display
pub const XR_DEPTH_RESOLUTION_QUARTER_ANDROID: DepthResolutionAndroid = 1;
pub const XR_DEPTH_RESOLUTION_HALF_ANDROID: DepthResolutionAndroid = 2;
pub const XR_DEPTH_RESOLUTION_FULL_ANDROID: DepthResolutionAndroid = 3;

/// Structures for the depth texture extension
/// Information about depth resolution capabilities
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DepthResolutionInfoAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    pub width: u32,
    pub height: u32,
}

/// Surface information for depth texture rendering
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DepthSurfaceInfoAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    pub depth_surface: *mut c_void,
}

/// Creation parameters for depth textures
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DepthTextureCreateInfoAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    pub resolution: DepthResolutionInfoAndroid,
    pub surface_origin: SurfaceOriginAndroid,
}

/// Handle to a depth texture resource
#[repr(C)]
#[derive(Debug)]
pub struct DepthTextureAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    pub texture: *mut c_void,
}

/// Creation parameters for depth swapchains
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DepthSwapchainCreateInfoAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    pub resolution: DepthResolutionAndroid,
    pub create_flags: DepthSwapchainCreateFlagsAndroid,
}

/// Individual depth swapchain image containing multiple texture types
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DepthSwapchainImageAndroid {
    pub ty: StructureType,
    pub next: *const c_void,
    /// Raw depth image data directly from sensors
    pub raw_depth_image: *mut c_void,
    /// Confidence values for raw depth data
    pub raw_depth_confidence_image: *mut c_void,
    /// Processed smooth depth image
    pub smooth_depth_image: *mut c_void,
    /// Confidence values for smooth depth data
    pub smooth_depth_confidence_image: *mut c_void,
}

/// System properties structure for depth tracking capabilities
#[repr(C)]
#[derive(Debug, Clone)]
pub struct XrSystemDepthTrackingPropertiesANDROID {
    pub ty: StructureType,
    pub next: *mut c_void,
    /// Boolean indicating if the system supports depth tracking
    pub supports_depth_tracking: Bool32,
}

impl Default for XrSystemDepthTrackingPropertiesANDROID {
    fn default() -> Self {
        Self {
            ty: xr_type_system_depth_tracking_properties_android(),
            next: std::ptr::null_mut(),
            supports_depth_tracking: Bool32::from_raw(0),
        }
    }
}

/// Function pointer types for extension functions
type PfnEnumerateDepthResolutionsAndroid = unsafe extern "C" fn(
    session: Session,
    capacity_input: u32,
    count_output: *mut u32,
    resolutions: *mut DepthResolutionAndroid,
) -> Result;

type PfnCreateDepthSwapchainAndroid = unsafe extern "C" fn(
    session: Session,
    create_info: *const DepthSwapchainCreateInfoAndroid,
    swapchain: *mut Swapchain,
) -> Result;

type PfnDestroyDepthSwapchainAndroid = unsafe extern "C" fn(session: Session, swapchain: Swapchain) -> Result;

type PfnEnumerateDepthSwapchainImagesAndroid = unsafe extern "C" fn(
    swapchain: Swapchain,
    capacity_input: u32,
    count_output: *mut u32,
    images: *mut DepthSwapchainImageAndroid,
) -> Result;

type PfnAcquireDepthSwapchainImageAndroid =
    unsafe extern "C" fn(session: Session, swapchain: Swapchain, index: *mut u32) -> Result;

/// Structure type functions (using raw values from OpenXR specification)
pub fn xr_type_depth_resolution_info_android() -> StructureType {
    StructureType::from_raw(1000343000)
}
pub fn xr_type_depth_surface_info_android() -> StructureType {
    StructureType::from_raw(1000343001)
}
pub fn xr_type_depth_texture_create_info_android() -> StructureType {
    StructureType::from_raw(1000343002)
}
pub fn xr_type_depth_texture_android() -> StructureType {
    StructureType::from_raw(1000343003)
}
pub fn xr_type_depth_swapchain_create_info_android() -> StructureType {
    StructureType::from_raw(1000343004)
}
pub fn xr_type_depth_swapchain_image_android() -> StructureType {
    StructureType::from_raw(1000343005)
}
pub fn xr_type_system_depth_tracking_properties_android() -> StructureType {
    StructureType::from_raw(1000343006)
}

/// Raw structure type constants for direct OpenXR API usage
pub const XR_TYPE_DEPTH_RESOLUTION_INFO_ANDROID_RAW: u32 = 1000343000;
pub const XR_TYPE_DEPTH_SURFACE_INFO_ANDROID_RAW: u32 = 1000343001;
pub const XR_TYPE_DEPTH_TEXTURE_CREATE_INFO_ANDROID_RAW: u32 = 1000343002;
pub const XR_TYPE_DEPTH_TEXTURE_ANDROID_RAW: u32 = 1000343003;
pub const XR_TYPE_DEPTH_SWAPCHAIN_CREATE_INFO_ANDROID_RAW: u32 = 1000343004;
pub const XR_TYPE_DEPTH_SWAPCHAIN_IMAGE_ANDROID_RAW: u32 = 1000343005;
pub const XR_TYPE_SYSTEM_DEPTH_TRACKING_PROPERTIES_ANDROID_RAW: u32 = 1000343006;

/// Extension function names for dynamic loading
const XR_ENUMERATE_DEPTH_RESOLUTIONS_ANDROID_NAME: &str = "xrEnumerateDepthResolutionsANDROID";
const XR_CREATE_DEPTH_SWAPCHAIN_ANDROID_NAME: &str = "xrCreateDepthSwapchainANDROID";
const XR_DESTROY_DEPTH_SWAPCHAIN_ANDROID_NAME: &str = "xrDestroyDepthSwapchainANDROID";
const XR_ENUMERATE_DEPTH_SWAPCHAIN_IMAGES_ANDROID_NAME: &str = "xrEnumerateDepthSwapchainImagesANDROID";
const XR_ACQUIRE_DEPTH_SWAPCHAIN_IMAGE_ANDROID_NAME: &str = "xrAcquireDepthSwapchainImagesANDROID";

/// Main extension handler for Android depth texture functionality
///
/// This struct manages the XR_ANDROID_depth_texture extension, providing access to:
/// - Depth and confidence texture data from device sensors
/// - Multiple resolution options (quarter, half, full)
/// - Both raw and processed depth information
/// - Swapchain management for efficient rendering
///
///
/// ###Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_android_depth_texture::*;
///
/// number_of_steps = 50;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 10 {
///         // Initialize extension and check system support
///         if let Some(depth_ext) = XrAndroidDepthTexture::new() {
///             // Check system support with logging
///             match depth_ext.check_system_support(true) {
///                 Ok(_system_props) => {
///                     stereokit_rust::system::Log::info("✅ System supports depth tracking");
///                     
///                     // Continue with depth texture operations...
///                     let session = openxr_sys::Session::from_raw(
///                         stereokit_rust::system::BackendOpenXR::session()
///                     );
///                     
///                     // Create depth swapchain with smooth and raw depth images
///                     let swapchain_info = create_depth_swapchain_info(
///                         XR_DEPTH_RESOLUTION_HALF_ANDROID,
///                         XR_DEPTH_SWAPCHAIN_CREATE_SMOOTH_DEPTH_IMAGE_BIT_ANDROID
///                             | XR_DEPTH_SWAPCHAIN_CREATE_RAW_DEPTH_IMAGE_BIT_ANDROID
///                     );
///                     
///                     match depth_ext.create_depth_swapchain(session, &swapchain_info) {
///                         Ok(swapchain) => {
///                             stereokit_rust::system::Log::info("✅ Depth swapchain created!");
///                             
///                             // Enumerate swapchain images
///                             if let Ok(images) = depth_ext.enumerate_depth_swapchain_images(swapchain) {
///                                 stereokit_rust::system::Log::info(
///                                     format!("Found {} swapchain images", images.len())
///                                 );
///                             }
///                             
///                             // Test image acquisition
///                             if let Ok(index) = depth_ext.acquire_depth_swapchain_image(session, swapchain) {
///                                 stereokit_rust::system::Log::info(
///                                     format!("Acquired image at index: {}", index)
///                                 );
///                             }
///                             
///                             // Cleanup
///                             let _ = depth_ext.destroy_depth_swapchain(swapchain);
///                             stereokit_rust::system::Log::info("✅ Swapchain test completed!");
///                         }
///                         Err(e) => stereokit_rust::system::Log::err(format!("Swapchain creation failed: {}", e)),
///                     }
///                 }
///                 Err(e) => stereokit_rust::system::Log::err(format!("System support check failed: {:?}", e)),
///             }
///         }
///     }
/// );
/// ```
///
///
/// #### Depth Resolution Enumeration Example  
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_android_depth_texture::*;
///
/// number_of_steps = 20;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 5 {
///         // Test depth resolution enumeration and analysis
///         if let Some(depth_ext) = XrAndroidDepthTexture::new() {
///             let session = openxr_sys::Session::from_raw(
///                 stereokit_rust::system::BackendOpenXR::session()
///             );
///             
///             match depth_ext.enumerate_depth_resolutions(session) {
///                 Ok(resolutions) => {
///                     stereokit_rust::system::Log::info(
///                         format!("✅ Found {} supported depth resolutions", resolutions.len())
///                     );
///                     
///                     for (i, resolution) in resolutions.iter().enumerate() {
///                         let (width, height) = get_resolution_dimensions(*resolution);
///                         let resolution_name = match *resolution {
///                             XR_DEPTH_RESOLUTION_QUARTER_ANDROID => "Quarter",
///                             XR_DEPTH_RESOLUTION_HALF_ANDROID => "Half",
///                             XR_DEPTH_RESOLUTION_FULL_ANDROID => "Full",
///                             _ => "Unknown",
///                         };
///                         
///                         stereokit_rust::system::Log::info(format!(
///                             "  Resolution {}: {} ({}x{}) - enum: {}",
///                             i, resolution_name, width, height, resolution
///                         ));
///                     }
///                     
///                     stereokit_rust::system::Log::info("✅ Resolution enumeration completed!");
///                 }
///                 Err(e) => stereokit_rust::system::Log::err(
///                     format!("Resolution enumeration failed: {}", e)
///                 ),
///             }
///         }
///     }
/// );
/// ```
#[derive(Debug)]
pub struct XrAndroidDepthTexture {
    /// Loaded function pointers from the OpenXR runtime
    xr_get_system_properties: Option<GetSystemProperties>,
    enumerate_depth_resolutions: Option<PfnEnumerateDepthResolutionsAndroid>,
    create_depth_swapchain: Option<PfnCreateDepthSwapchainAndroid>,
    destroy_depth_swapchain: Option<PfnDestroyDepthSwapchainAndroid>,
    enumerate_depth_swapchain_images: Option<PfnEnumerateDepthSwapchainImagesAndroid>,
    acquire_depth_swapchain_image: Option<PfnAcquireDepthSwapchainImageAndroid>,
    instance: Instance,
}

impl XrAndroidDepthTexture {
    /// Create and initialize a new AndroidDepthTextureExtension instance
    ///
    /// This method combines creation and initialization, checking extension availability
    /// and loading all necessary OpenXR functions for depth texture operations.
    ///
    /// # Returns
    /// - `Some(Self)` if extension is available and initialization succeeds
    /// - `None` if extension is not available or initialization fails
    pub fn new() -> Option<Self> {
        // Check if the extension is available first
        if !is_android_depth_texture_extension_available() {
            Log::warn("XR_ANDROID_depth_texture extension not available");
            return None;
        }

        // Load functions using the generic system
        let xr_get_system_properties = BackendOpenXR::get_function::<GetSystemProperties>("xrGetSystemProperties");
        let enumerate_depth_resolutions = BackendOpenXR::get_function::<PfnEnumerateDepthResolutionsAndroid>(
            XR_ENUMERATE_DEPTH_RESOLUTIONS_ANDROID_NAME,
        );

        let create_depth_swapchain =
            BackendOpenXR::get_function::<PfnCreateDepthSwapchainAndroid>(XR_CREATE_DEPTH_SWAPCHAIN_ANDROID_NAME);

        let destroy_depth_swapchain =
            BackendOpenXR::get_function::<PfnDestroyDepthSwapchainAndroid>(XR_DESTROY_DEPTH_SWAPCHAIN_ANDROID_NAME);

        let enumerate_depth_swapchain_images = BackendOpenXR::get_function::<PfnEnumerateDepthSwapchainImagesAndroid>(
            XR_ENUMERATE_DEPTH_SWAPCHAIN_IMAGES_ANDROID_NAME,
        );

        let acquire_depth_swapchain_image = BackendOpenXR::get_function::<PfnAcquireDepthSwapchainImageAndroid>(
            XR_ACQUIRE_DEPTH_SWAPCHAIN_IMAGE_ANDROID_NAME,
        );

        let instance = Instance::from_raw(BackendOpenXR::instance());

        // Verify that all critical functions were loaded successfully
        if xr_get_system_properties.is_none()
            || enumerate_depth_resolutions.is_none()
            || create_depth_swapchain.is_none()
            || destroy_depth_swapchain.is_none()
            || enumerate_depth_swapchain_images.is_none()
            || acquire_depth_swapchain_image.is_none()
        {
            Log::warn("Failed to load all XR_ANDROID_depth_texture functions");
            return None;
        }

        Log::info("XR_ANDROID_depth_texture extension initialized successfully");

        Some(Self {
            xr_get_system_properties,
            enumerate_depth_resolutions,
            create_depth_swapchain,
            destroy_depth_swapchain,
            enumerate_depth_swapchain_images,
            acquire_depth_swapchain_image,
            instance,
        })
    }

    /// Check if the system supports depth tracking
    ///
    /// # Parameters
    /// - `with_log`: If true, outputs system properties to diagnostic log
    ///
    /// # Returns
    /// `Ok(SystemProperties)` with depth tracking properties if supported, or error on failure
    pub fn check_system_support(&self, with_log: bool) -> std::result::Result<SystemProperties, openxr_sys::Result> {
        let get_props_fn = self.xr_get_system_properties.ok_or(openxr_sys::Result::ERROR_FUNCTION_UNSUPPORTED)?;

        let system_id = SystemId::from_raw(BackendOpenXR::system_id());

        let mut depth_tracking_props = XrSystemDepthTrackingPropertiesANDROID {
            ty: xr_type_system_depth_tracking_properties_android(),
            next: null_mut(),
            supports_depth_tracking: Bool32::from_raw(0),
        };

        let mut system_properties = SystemProperties {
            ty: StructureType::SYSTEM_PROPERTIES,
            next: &mut depth_tracking_props as *mut _ as *mut c_void,
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

        if result != openxr_sys::Result::SUCCESS {
            return Err(result);
        }

        if with_log {
            Log::diag("=== XR_ANDROID_depth_texture System Properties ===");
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
                system_properties.tracking_properties.orientation_tracking.into_raw() != 0
            ));
            Log::diag(format!(
                "  Position tracking: {}",
                system_properties.tracking_properties.position_tracking.into_raw() != 0
            ));

            Log::diag("Depth tracking properties:");
            Log::diag(format!(
                "  Supports depth tracking: {}",
                depth_tracking_props.supports_depth_tracking.into_raw() != 0
            ));
            Log::diag("================================================");
        }

        Ok(system_properties)
    }

    /// Wrapper methods used by lib.rs
    /// Enumerate available depth resolutions supported by the device
    ///
    /// # Arguments
    /// * `session` - The OpenXR session
    ///
    /// # Returns
    /// Vector of supported depth resolution enum values
    pub fn enumerate_depth_resolutions(
        &self,
        session: Session,
    ) -> std::result::Result<Vec<DepthResolutionAndroid>, String> {
        let enumerate_fn = self.enumerate_depth_resolutions.ok_or("enumerate_depth_resolutions function not loaded")?;

        // First call to get the count
        let mut count = 0u32;
        let result = unsafe { enumerate_fn(session, 0, &mut count, std::ptr::null_mut()) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to get depth resolutions count: {:?}", result));
        }

        if count == 0 {
            return Ok(vec![]);
        }

        // Second call to get the actual resolutions
        let mut resolutions = vec![0u32; count as usize];
        let result = unsafe { enumerate_fn(session, count, &mut count, resolutions.as_mut_ptr()) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to enumerate depth resolutions: {:?}", result));
        }

        // Resize to actual count returned
        resolutions.resize(count as usize, 0);
        Ok(resolutions)
    }

    /// Create a depth swapchain for rendering depth textures
    ///
    /// # Arguments
    /// * `session` - The OpenXR session
    /// * `create_info` - Configuration for the depth swapchain
    ///
    /// # Returns
    /// Handle to the created swapchain or error description
    pub fn create_depth_swapchain(
        &self,
        session: Session,
        create_info: &DepthSwapchainCreateInfoAndroid,
    ) -> std::result::Result<Swapchain, String> {
        let create_fn = self.create_depth_swapchain.ok_or("create_depth_swapchain function not loaded")?;

        let mut swapchain = Swapchain::from_raw(0);
        let result = unsafe { create_fn(session, create_info, &mut swapchain as *mut Swapchain) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to create depth swapchain: {:?}", result));
        }

        Ok(swapchain)
    }

    /// Enumerate the images in a depth swapchain
    ///
    /// # Arguments
    /// * `swapchain` - The depth swapchain to enumerate
    ///
    /// # Returns
    /// Vector of depth swapchain images with texture handles
    pub fn enumerate_depth_swapchain_images(
        &self,
        swapchain: Swapchain,
    ) -> std::result::Result<Vec<DepthSwapchainImageAndroid>, String> {
        let enumerate_fn = self
            .enumerate_depth_swapchain_images
            .ok_or("enumerate_depth_swapchain_images function not loaded")?;

        // First call to get the count
        let mut count = 0u32;
        let result = unsafe { enumerate_fn(swapchain, 0, &mut count, std::ptr::null_mut()) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to get depth swapchain images count: {:?}", result));
        }

        if count == 0 {
            return Ok(vec![]);
        }

        // Prepare vector with initialized structures
        let mut images = vec![
            DepthSwapchainImageAndroid {
                ty: xr_type_depth_swapchain_image_android(),
                next: std::ptr::null(),
                raw_depth_image: std::ptr::null_mut(),
                raw_depth_confidence_image: std::ptr::null_mut(),
                smooth_depth_image: std::ptr::null_mut(),
                smooth_depth_confidence_image: std::ptr::null_mut(),
            };
            count as usize
        ];

        // Second call to get the actual images
        let result = unsafe { enumerate_fn(swapchain, count, &mut count, images.as_mut_ptr()) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to enumerate depth swapchain images: {:?}", result));
        }

        // Resize to actual count returned
        images.resize(
            count as usize,
            DepthSwapchainImageAndroid {
                ty: xr_type_depth_swapchain_image_android(),
                next: std::ptr::null(),
                raw_depth_image: std::ptr::null_mut(),
                raw_depth_confidence_image: std::ptr::null_mut(),
                smooth_depth_image: std::ptr::null_mut(),
                smooth_depth_confidence_image: std::ptr::null_mut(),
            },
        );

        Ok(images)
    }

    /// Destroy a previously created depth swapchain
    ///
    /// # Arguments
    /// * `swapchain` - The depth swapchain to destroy
    ///
    /// # Returns
    /// `Ok(())` on success or error description on failure
    pub fn destroy_depth_swapchain(&self, swapchain: Swapchain) -> std::result::Result<(), String> {
        let destroy_fn = self.destroy_depth_swapchain.ok_or("destroy_depth_swapchain function not loaded")?;

        let session = Session::from_raw(BackendOpenXR::session());
        let result = unsafe { destroy_fn(session, swapchain) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to destroy depth swapchain: {:?}", result));
        }

        Ok(())
    }

    /// Acquire an image from a depth swapchain
    ///
    /// # Arguments
    /// * `session` - The OpenXR session
    /// * `swapchain` - The depth swapchain to acquire from
    ///
    /// # Returns
    /// Index of the acquired image or error description
    pub fn acquire_depth_swapchain_image(
        &self,
        session: Session,
        swapchain: Swapchain,
    ) -> std::result::Result<u32, String> {
        let acquire_fn =
            self.acquire_depth_swapchain_image.ok_or("acquire_depth_swapchain_image function not loaded")?;

        let mut image_index = 0u32;
        let result = unsafe { acquire_fn(session, swapchain, &mut image_index) };

        if result != openxr_sys::Result::SUCCESS {
            return Err(format!("Failed to acquire depth swapchain image: {:?}", result));
        }

        Ok(image_index)
    }
}

/// Helper function to create depth texture creation info
///
/// # Arguments
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
/// * `surface_origin` - Surface origin (top-left or bottom-left)
///
/// # Returns
/// Initialized `DepthTextureCreateInfoAndroid` structure
pub fn create_depth_texture_info(
    width: u32,
    height: u32,
    surface_origin: SurfaceOriginAndroid,
) -> DepthTextureCreateInfoAndroid {
    let resolution = DepthResolutionInfoAndroid {
        ty: xr_type_depth_resolution_info_android(),
        next: std::ptr::null(),
        width,
        height,
    };

    DepthTextureCreateInfoAndroid {
        ty: xr_type_depth_texture_create_info_android(),
        next: std::ptr::null(),
        resolution,
        surface_origin,
    }
}

impl Default for XrAndroidDepthTexture {
    fn default() -> Self {
        // This will panic if extension is not available, which is the intended behavior
        // for Default trait when the extension should always be available
        Self::new().expect("XR_ANDROID_depth_texture extension should be available")
    }
}

/// Convenience function to check if XR_FB_render_model extension is available
pub fn is_android_depth_texture_extension_available() -> bool {
    Backend::xr_type() == BackendXRType::OpenXR && BackendOpenXR::ext_enabled(XR_ANDROID_DEPTH_TEXTURE_EXTENSION_NAME)
}

/// Helper functions used by lib.rs
/// Get the pixel dimensions for a given depth resolution enum
///
/// # Arguments
/// * `resolution` - The depth resolution enum value
///
/// # Returns
/// Tuple of (width, height) in pixels
pub fn get_resolution_dimensions(resolution: DepthResolutionAndroid) -> (u32, u32) {
    match resolution {
        XR_DEPTH_RESOLUTION_QUARTER_ANDROID => (640, 480), // Quarter resolution
        XR_DEPTH_RESOLUTION_HALF_ANDROID => (1280, 960),   // Half resolution
        XR_DEPTH_RESOLUTION_FULL_ANDROID => (2560, 1920),  // Full resolution
        _ => (0, 0),                                       // Unknown resolution
    }
}

/// Create a depth swapchain creation info structure
///
/// # Arguments
/// * `resolution` - The desired depth resolution
/// * `create_flags` - Flags controlling which depth/confidence images to create
///
/// # Returns
/// Initialized `DepthSwapchainCreateInfoAndroid` structure
pub fn create_depth_swapchain_info(
    resolution: DepthResolutionAndroid,
    create_flags: DepthSwapchainCreateFlagsAndroid,
) -> DepthSwapchainCreateInfoAndroid {
    DepthSwapchainCreateInfoAndroid {
        ty: xr_type_depth_swapchain_create_info_android(),
        next: std::ptr::null(),
        resolution,
        create_flags,
    }
}

/// Comprehensive test function for XR_ANDROID_depth_texture extension
///
/// This function demonstrates the complete workflow for using Android depth textures:
/// 1. Extension initialization and availability checking
/// 2. System capability inspection for depth tracking support  
/// 3. Depth resolution enumeration and selection
/// 4. Depth swapchain creation with multiple image types
/// 5. Swapchain image enumeration and inspection
/// 6. Proper cleanup and resource management
///
/// The test provides detailed logging of each step and handles errors gracefully,
/// making it useful both for validation and as a reference implementation.
///
/// # Usage
/// Call this function after StereoKit initialization in an OpenXR environment
/// that supports the XR_ANDROID_depth_texture extension.
///
/// ### Example
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::xr_android_depth_texture::*;
///
/// number_of_steps = 60;
/// test_steps!( // !!!! Get a proper main loop !!!!
///     if iter == 30 {
///         // Run comprehensive test of all extension functionality
///         stereokit_rust::system::Log::info("🚀 Running comprehensive depth texture test...");
///         test_depth_texture_extension();
///         stereokit_rust::system::Log::info("🏁 Comprehensive test completed!");
///     }
/// );
/// ```
pub fn test_depth_texture_extension() {
    Log::diag("🚀 === TESTING XR_ANDROID_DEPTH_TEXTURE EXTENSION ===");

    // Initialize the depth texture extension
    match XrAndroidDepthTexture::new() {
        Some(depth_ext) => {
            Log::diag("✅ XR_ANDROID_depth_texture extension initialized successfully");

            Log::diag("✅ XR_ANDROID_depth_texture swapchain API is available");

            // Get OpenXR handles from StereoKit for logging purposes
            let instance_raw = BackendOpenXR::instance();
            let system_id_raw = BackendOpenXR::system_id();
            Log::diag(format!("✅ OpenXR instance obtained: {:?}", instance_raw));
            Log::diag(format!("✅ OpenXR system ID obtained: {:?}", system_id_raw));

            // === INSPECT SYSTEM CAPABILITY ===
            Log::diag("=== Inspecting system depth tracking capabilities ===");

            // Test system support check using the new method
            match depth_ext.check_system_support(true) {
                Ok(sys_prop) => {
                    Log::diag("✅ System support check completed successfully");

                    // Get the depth tracking properties from the next pointer
                    let depth_tracking_properties =
                        unsafe { &*(sys_prop.next as *const XrSystemDepthTrackingPropertiesANDROID) };

                    if depth_tracking_properties.supports_depth_tracking.into_raw() != 0 {
                        Log::diag("🎯 ✅ DEPTH TRACKING IS SUPPORTED BY THE SYSTEM!");

                        // Get session from StereoKit and convert to proper type
                        let session_raw = BackendOpenXR::session();
                        let session = openxr_sys::Session::from_raw(session_raw);
                        Log::diag(format!("✅ OpenXR session obtained: {:?}", session_raw));

                        // === QUERY SUPPORTED DEPTH RESOLUTIONS ===
                        Log::diag("=== Querying supported depth resolutions ===");

                        match depth_ext.enumerate_depth_resolutions(session) {
                            Ok(resolutions) => {
                                Log::diag(format!("✅ Found {} supported depth resolutions", resolutions.len()));
                                for (i, res) in resolutions.iter().enumerate() {
                                    let (width, height) = get_resolution_dimensions(*res);
                                    Log::diag(format!(
                                        "  Resolution {}: {}x{} (enum value: {})",
                                        i, width, height, res
                                    ));
                                }

                                if !resolutions.is_empty() {
                                    let selected_resolution = resolutions[0];
                                    let (width, height) = get_resolution_dimensions(selected_resolution);
                                    Log::diag(format!(
                                        "🎯 Selected resolution: {}x{} (enum: {})",
                                        width, height, selected_resolution
                                    ));

                                    // === CREATE DEPTH SWAPCHAIN ===
                                    Log::diag("=== Creating depth swapchain ===");

                                    let swapchain_create_info = create_depth_swapchain_info(
                                        selected_resolution,
                                        XR_DEPTH_SWAPCHAIN_CREATE_SMOOTH_DEPTH_IMAGE_BIT_ANDROID
                                            | XR_DEPTH_SWAPCHAIN_CREATE_SMOOTH_CONFIDENCE_IMAGE_BIT_ANDROID
                                            | XR_DEPTH_SWAPCHAIN_CREATE_RAW_DEPTH_IMAGE_BIT_ANDROID
                                            | XR_DEPTH_SWAPCHAIN_CREATE_RAW_CONFIDENCE_IMAGE_BIT_ANDROID,
                                    );

                                    Log::diag(format!(
                                        "Swapchain create info - Resolution: {}, Flags: 0b{:08b}",
                                        swapchain_create_info.resolution, swapchain_create_info.create_flags
                                    ));

                                    match depth_ext.create_depth_swapchain(session, &swapchain_create_info) {
                                        Ok(depth_swapchain) => {
                                            Log::diag(format!(
                                                "✅ Depth swapchain created successfully: {:?}",
                                                depth_swapchain
                                            ));

                                            // === ENUMERATE DEPTH SWAPCHAIN IMAGES ===
                                            Log::diag("=== Enumerating depth swapchain images ===");

                                            match depth_ext.enumerate_depth_swapchain_images(depth_swapchain) {
                                                Ok(depth_images) => {
                                                    Log::diag(format!(
                                                        "✅ Enumerated {} depth swapchain images",
                                                        depth_images.len()
                                                    ));

                                                    for (i, image) in depth_images.iter().enumerate() {
                                                        Log::diag(format!(
                                                            "  Image {}: raw_depth={:p}, raw_conf={:p}, smooth_depth={:p}, smooth_conf={:p}",
                                                            i,
                                                            image.raw_depth_image,
                                                            image.raw_depth_confidence_image,
                                                            image.smooth_depth_image,
                                                            image.smooth_depth_confidence_image
                                                        ));
                                                    }

                                                    Log::diag(
                                                        "🎯 ✅ DEPTH TEXTURE EXTENSION SETUP COMPLETE AND READY FOR USE!",
                                                    );

                                                    // Cleanup - destroy the swapchain
                                                    match depth_ext.destroy_depth_swapchain(depth_swapchain) {
                                                        Ok(()) => {
                                                            Log::diag("✅ Depth swapchain destroyed successfully")
                                                        }
                                                        Err(e) => Log::err(format!(
                                                            "❌ Failed to destroy depth swapchain: {:?}",
                                                            e
                                                        )),
                                                    }
                                                }
                                                Err(e) => {
                                                    Log::err(format!(
                                                        "❌ Failed to enumerate depth swapchain images: {:?}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            Log::err(format!("❌ Failed to create depth swapchain: {:?}", e));
                                        }
                                    }
                                } else {
                                    Log::warn("⚠️ No depth resolutions available");
                                }
                            }
                            Err(e) => {
                                Log::err(format!("❌ Failed to enumerate depth resolutions: {:?}", e));
                            }
                        }
                    } else {
                        Log::warn("⚠️ Depth tracking is NOT supported by this system");
                    }
                }
                Err(e) => {
                    Log::err(format!("❌ System support check failed: {:?}", e));
                }
            }
        }
        None => {
            Log::err("❌ Failed to initialize XR_ANDROID_depth_texture extension");
        }
    }

    Log::diag("🏁 === DEPTH TEXTURE EXTENSION TEST COMPLETE ===");
}
