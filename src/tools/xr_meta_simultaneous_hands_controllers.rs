//! XR_META_simultaneous_hands_and_controllers extension implementation
//!
//! This module provides access to the OpenXR XR_META_simultaneous_hands_and_controllers extension,
//! which allows applications to track both hands and controllers simultaneously.
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_META_simultaneous_hands_and_controllers>

use openxr_sys::pfn::GetSystemProperties;
use openxr_sys::{
    Bool32, Instance, Result, Session, StructureType, SystemGraphicsProperties, SystemId, SystemProperties,
    SystemTrackingProperties,
};

use crate::system::{Backend, BackendOpenXR, BackendXRType, Log};

/// Check if simultaneous hands and controllers tracking is supported by the runtime.
/// * with_log - whether to log diagnostic information
///
/// Returns true if the XR_META_simultaneous_hands_and_controllers extension is enabled.
/// see also [`resume_simultaneous_hands_and_controllers`] [`pause_simultaneous_hands_and_controllers`]
pub fn is_simultaneous_hands_and_controllers_supported(with_log: bool) -> bool {
    if Backend::xr_type() != BackendXRType::OpenXR
        || !BackendOpenXR::ext_enabled("XR_META_simultaneous_hands_and_controllers")
    {
        if with_log {
            Log::info("XR_META_simultaneous_hands_and_controllers extension is not available.");
        }
        return false;
    }

    if let Some(get_sys_props) = BackendOpenXR::get_function::<GetSystemProperties>("xrGetSystemProperties") {
        let system_id = SystemId::from_raw(BackendOpenXR::system_id());

        // Create the properties structure for simultaneous hands and controllers using openxr_sys types
        #[repr(C)]
        struct SimultaneousHandsAndControllersProperties {
            structure_type: StructureType,
            next: *mut std::ffi::c_void,
            supports_simultaneous_hands_and_controllers: Bool32,
        }

        let mut simultaneous_props = SimultaneousHandsAndControllersProperties {
            structure_type: StructureType::from_raw(1000532001), // XR_TYPE_SYSTEM_SIMULTANEOUS_HANDS_AND_CONTROLLERS_PROPERTIES_META
            next: std::ptr::null_mut(),
            supports_simultaneous_hands_and_controllers: Bool32::from_raw(0),
        };

        let mut system_properties = SystemProperties {
            ty: StructureType::SYSTEM_PROPERTIES,
            next: &mut simultaneous_props as *mut _ as *mut std::ffi::c_void,
            system_id,
            vendor_id: 0,
            system_name: [0; openxr_sys::MAX_SYSTEM_NAME_SIZE],
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

        match unsafe { get_sys_props(Instance::from_raw(BackendOpenXR::instance()), system_id, &mut system_properties) }
        {
            Result::SUCCESS => {
                let supported = simultaneous_props.supports_simultaneous_hands_and_controllers.into_raw() != 0;
                if with_log {
                    if supported {
                        Log::info("✅  Simultaneous hands and controllers tracking is available.");
                    } else {
                        Log::info("❌  Simultaneous hands and controllers tracking is not available.");
                    }
                }
                supported
            }
            otherwise => {
                if with_log {
                    Log::err(format!("❌ xrGetSystemProperties failed: {otherwise:?}"));
                }
                false
            }
        }
    } else {
        if with_log {
            Log::err("❌ xrGetSystemProperties binding function error!");
        }
        false
    }
}

/// Resume (enable) simultaneous hands and controllers tracking if supported.
/// * `with_log` - If true, will log the operation status
///
/// Returns true if the operation was successful or if already enabled.
/// see also [`is_simultaneous_hands_and_controllers_supported`] [`pause_simultaneous_hands_and_controllers`]
/// ### Examples
/// ```
/// use stereokit_rust::system::BackendOpenXR;
/// BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");
///
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::tools::xr_meta_simultaneous_hands_controllers::{resume_simultaneous_hands_and_controllers,
///                                     is_simultaneous_hands_and_controllers_supported};
///
/// if is_simultaneous_hands_and_controllers_supported(true) {
///     let success = resume_simultaneous_hands_and_controllers(true);
///     assert_eq!(success, true);
/// }
/// ```
pub fn resume_simultaneous_hands_and_controllers(with_log: bool) -> bool {
    if !is_simultaneous_hands_and_controllers_supported(with_log) {
        if with_log {
            Log::info("❌ XR_META_simultaneous_hands_and_controllers extension is not available.");
        }
        return false;
    }

    // Try to get the function pointer using a generic function type
    // Since the specific types might not be available in openxr_sys yet, we'll use raw function pointers
    type XrResumeSimultaneousHandsAndControllersTrackingMETA =
        unsafe extern "system" fn(session: Session, resume_info: *const std::ffi::c_void) -> Result;

    if let Some(resume_fn) = BackendOpenXR::get_function::<XrResumeSimultaneousHandsAndControllersTrackingMETA>(
        "xrResumeSimultaneousHandsAndControllersTrackingMETA",
    ) {
        // Create a basic structure for the resume info using openxr_sys types
        #[repr(C)]
        struct ResumeInfo {
            structure_type: StructureType,
            next: *const std::ffi::c_void,
        }

        let resume_info = ResumeInfo {
            structure_type: StructureType::from_raw(1000532002), // XR_TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_RESUME_INFO_META
            next: std::ptr::null(),
        };

        match unsafe {
            resume_fn(Session::from_raw(BackendOpenXR::session()), &resume_info as *const _ as *const std::ffi::c_void)
        } {
            Result::SUCCESS => {
                if with_log {
                    Log::info("✅ Simultaneous hands and controllers tracking resumed successfully.");
                }
                true
            }
            otherwise => {
                if with_log {
                    Log::err(format!("❌  xrResumeSimultaneousHandsAndControllersTrackingMETA failed: {otherwise:?}"));
                }
                false
            }
        }
    } else {
        if with_log {
            Log::err("❌  xrResumeSimultaneousHandsAndControllersTrackingMETA binding function error!");
        }
        false
    }
}

/// Pause (disable) simultaneous hands and controllers tracking if supported.
/// * `with_log` - If true, will log the operation status
///
/// Returns true if the operation was successful or if already paused.
/// see also [`resume_simultaneous_hands_and_controllers`] [`is_simultaneous_hands_and_controllers_supported`]
/// ### Examples
/// ```
/// use stereokit_rust::system::BackendOpenXR;
/// BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");
///
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::tools::xr_meta_simultaneous_hands_controllers::{pause_simultaneous_hands_and_controllers,
///                                     is_simultaneous_hands_and_controllers_supported};
///
/// if is_simultaneous_hands_and_controllers_supported(true) {
///     let success = pause_simultaneous_hands_and_controllers(true);
///     assert_eq!(success, true);
/// }
/// ```
pub fn pause_simultaneous_hands_and_controllers(with_log: bool) -> bool {
    if !is_simultaneous_hands_and_controllers_supported(with_log) {
        if with_log {
            Log::info("❌ XR_META_simultaneous_hands_and_controllers extension is not available.");
        }
        return false;
    }

    // Try to get the function pointer using a generic function type
    // Since the specific types might not be available in openxr_sys yet, we'll use raw function pointers
    type XrPauseSimultaneousHandsAndControllersTrackingMETA =
        unsafe extern "system" fn(session: Session, pause_info: *const std::ffi::c_void) -> Result;

    if let Some(pause_fn) = BackendOpenXR::get_function::<XrPauseSimultaneousHandsAndControllersTrackingMETA>(
        "xrPauseSimultaneousHandsAndControllersTrackingMETA",
    ) {
        // Create a basic structure for the pause info using openxr_sys types
        #[repr(C)]
        struct PauseInfo {
            structure_type: StructureType,
            next: *const std::ffi::c_void,
        }

        let pause_info = PauseInfo {
            structure_type: StructureType::from_raw(1000532003), // XR_TYPE_SIMULTANEOUS_HANDS_AND_CONTROLLERS_TRACKING_PAUSE_INFO_META
            next: std::ptr::null(),
        };

        match unsafe {
            pause_fn(Session::from_raw(BackendOpenXR::session()), &pause_info as *const _ as *const std::ffi::c_void)
        } {
            Result::SUCCESS => {
                if with_log {
                    Log::info("✅ Simultaneous hands and controllers tracking paused successfully.");
                }
                true
            }
            otherwise => {
                if with_log {
                    Log::err(format!("❌ xrPauseSimultaneousHandsAndControllersTrackingMETA failed: {otherwise:?}"));
                }
                false
            }
        }
    } else {
        if with_log {
            Log::err("❌ xrPauseSimultaneousHandsAndControllersTrackingMETA binding function error!");
        }
        false
    }
}
