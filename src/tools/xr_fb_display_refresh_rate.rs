//! XR_FB_display_refresh_rate extension implementation
//!
//! This module provides access to display refresh rate control for OpenXR applications.
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_FB_display_refresh_rate>

use openxr_sys::pfn::{EnumerateDisplayRefreshRatesFB, GetDisplayRefreshRateFB, RequestDisplayRefreshRateFB};
use openxr_sys::{Handle, Result, Session};

use crate::system::{BackendOpenXR, Log};

pub const USUAL_FPS_SUSPECTS: [i32; 12] = [30, 60, 72, 80, 90, 100, 110, 120, 144, 165, 240, 360];

/// Return and maybe Log all the display refresh rates available.
/// * `with_log` - If true, log the refresh rates available
///
/// ### Examples
/// ```
/// use stereokit_rust::system::BackendOpenXR;
/// BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
///
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::tools::xr_fb_display_refresh_rate::{get_all_display_refresh_rates,
///                                     get_display_refresh_rate,
///                                     set_display_refresh_rate};
///
/// let refresh_rate_editable = BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate");
/// if refresh_rate_editable {
///     let rates = get_all_display_refresh_rates(true);
///     assert!(!rates.is_empty());
///     let rate = get_display_refresh_rate().unwrap_or(0.0);
///     assert!(rate >= 20.0);
///     assert!(set_display_refresh_rate(rate, true));
///     let rate2 = get_display_refresh_rate().unwrap_or(0.0);
///     assert_eq!(rate, rate2);
/// } else {
///     let rates = get_all_display_refresh_rates(true);
///     // assert!(rates.len(), 5); // with 5 value 0.0
///     let rate = get_display_refresh_rate();
///     assert_eq!(rate , None);
/// }
/// ```
pub fn get_all_display_refresh_rates(with_log: bool) -> Vec<f32> {
    let mut array = [0.0; 40];
    let mut count = 5u32;
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        if let Some(rate_display) =
            BackendOpenXR::get_function::<EnumerateDisplayRefreshRatesFB>("xrEnumerateDisplayRefreshRatesFB")
        {
            match unsafe {
                rate_display(Session::from_raw(BackendOpenXR::session()), 0, &mut count, array.as_mut_ptr())
            } {
                Result::SUCCESS => {
                    if count > 40 {
                        count = 40
                    }
                    match unsafe {
                        rate_display(Session::from_raw(BackendOpenXR::session()), count, &mut count, array.as_mut_ptr())
                    } {
                        Result::SUCCESS => {
                            if with_log {
                                Log::info(format!("✅ There are {count} display rate:"));
                                for (i, iter) in array.iter().enumerate() {
                                    if i >= count as usize {
                                        break;
                                    }
                                    Log::info(format!("   {iter:?} "));
                                }
                            }
                        }
                        otherwise => {
                            Log::err(format!("❌ xrEnumerateDisplayRefreshRatesFB failed: {otherwise}"));
                        }
                    }
                }
                otherwise => {
                    Log::err(format!("❌ xrEnumerateDisplayRefreshRatesFB failed: {otherwise}"));
                }
            }
        } else {
            Log::err("❌ xrEnumerateDisplayRefreshRatesFB binding function error !");
        }
    }
    array[0..(count as usize)].into()
}

/// Get the display rates available from the given list. See [`USUAL_FPS_SUSPECTS`])
/// * `fps_to_get` - The list of fps to test.
/// * `with_log` - If true, will log the available rates.
///
/// see also [`get_all_display_refresh_rates`]
pub fn get_display_refresh_rates(fps_to_get: &[i32], with_log: bool) -> Vec<f32> {
    let default_refresh_rate = get_display_refresh_rate();
    let mut available_rates = vec![];
    for rate in fps_to_get {
        if set_display_refresh_rate(*rate as f32, false) {
            available_rates.push(*rate as f32);
        }
    }
    if let Some(rate) = default_refresh_rate {
        set_display_refresh_rate(rate, with_log);
    }
    if with_log {
        Log::info(format!("✅ There are {} display rate from the given selection:", available_rates.len()));
        for iter in &available_rates {
            Log::info(format!("   {iter:?} "));
        }
    }

    available_rates
}

/// Get the current display rate if possible.
///
/// see also [`set_display_refresh_rate`]
/// see example in [`get_all_display_refresh_rates`]
pub fn get_display_refresh_rate() -> Option<f32> {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        if let Some(get_default_rate) =
            BackendOpenXR::get_function::<GetDisplayRefreshRateFB>("xrGetDisplayRefreshRateFB")
        {
            let mut default_rate = 0.0;
            match unsafe { get_default_rate(Session::from_raw(BackendOpenXR::session()), &mut default_rate) } {
                Result::SUCCESS => Some(default_rate),
                otherwise => {
                    Log::err(format!("❌ xrGetDisplayRefreshRateFB failed: {otherwise}"));
                    None
                }
            }
        } else {
            Log::err("❌ xrRequestDisplayRefreshRateFB binding function error !");
            None
        }
    } else {
        None
    }
}

/// Set the current display rate if possible.
/// Possible values on Quest and WiVRn are 60 - 80 - 72 - 80 - 90 - 120
/// returns true if the given value was accepted
/// * `rate` - the rate to set
/// * `with_log` - if true, will log the error if the rate was not accepted.
///
/// see example in [`get_all_display_refresh_rates`]
pub fn set_display_refresh_rate(rate: f32, with_log: bool) -> bool {
    if BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate") {
        //>>>>>>>>>>> Set the value
        if let Some(set_new_rate) =
            BackendOpenXR::get_function::<RequestDisplayRefreshRateFB>("xrRequestDisplayRefreshRateFB")
        {
            match unsafe { set_new_rate(Session::from_raw(BackendOpenXR::session()), rate) } {
                Result::SUCCESS => true,
                otherwise => {
                    if with_log {
                        Log::err(format!("❌ xrRequestDisplayRefreshRateFB failed: {otherwise}"));
                    }
                    false
                }
            }
        } else {
            Log::err("❌ xrRequestDisplayRefreshRateFB binding function error !");
            false
        }
    } else {
        false
    }
}
