use openxr_sys::{
    pfn::{
        CreatePassthroughFB, CreatePassthroughLayerFB, DestroyPassthroughFB, DestroyPassthroughLayerFB,
        PassthroughLayerPauseFB, PassthroughLayerResumeFB, PassthroughLayerSetStyleFB, PassthroughPauseFB,
        PassthroughStartFB,
    },
    CompositionLayerFlags, CompositionLayerPassthroughFB, PassthroughCreateInfoFB, PassthroughFB, PassthroughFlagsFB,
    PassthroughLayerCreateInfoFB, PassthroughLayerFB, PassthroughLayerPurposeFB, Result, Session, Space, StructureType,
};

use crate::{
    event_loop::{IStepper, StepperAction, StepperId},
    sk::{MainThreadToken, SkInfo},
    system::{Backend, BackendOpenXR, BackendXRType, Log, Renderer},
    util::Color128,
};
use std::{cell::RefCell, ptr::null_mut, rc::Rc};

/// The StepperAction to trigger with the value "0"/"1" to Deactivate/Activate the passthrough.
pub const PASSTHROUGH_FLIP: &str = "PassthroughFlip";

///
///
///  This is a rust copycat of https://github.com/StereoKit/StereoKit/blob/master/Examples/StereoKitTest/Tools/PassthroughFBExt.cs
///
///
/// Use PassthroughFbExt::new(true) instead of Default if you want to have it at start up.
///
///
/// ```ignore
/// // The folowing line must be added before initializing sk:
/// stereokit_rust::system::BackendOpenXR::request_ext("XR_FB_passthrough");
/// let (sk, event_loop) = settings.init_with_event_loop(app).unwrap();
///
/// // Launch the stepper as follow :
/// let passthrough = true;
/// let passthrough_enabled = stereokit_rust::system::BackendOpenXR::ext_enabled("XR_FB_passthrough");
/// if passthrough_enabled {
///    sk.push_action(StepperAction::add_default::<PassthroughFbExt>(
///        "PassthroughFbExt",
///    ));
///    if passthrough {
///        sk.push_action(StepperAction::event(
///            "main".into(),
///            PASSTHROUGH_FLIP,
///            "1",
///        ));
///        Log::diag("Passthrough Activated at start !!");
///    } else {
///        Log::diag("Passthrough Deactived at start !!");
///    }
/// } else {
///    Log::diag("No Passthrough !!")
/// }
///
///  // Activate/Deactivate the stepper as follow :
///  if passthrough_enabled && passthrough != new_passthrough_value {
///      passthrough = new_passthrough_value;
///          let mut string_value = "0";
///          if passthrough {
///              Log::diag("Activate passthrough");
///              string_value = "1";
///          } else {
///              Log::diag("Deactivate passthrough");
///          }
///          sk.push_action(StepperAction::event("main".into(), PASSTHROUGH_FLIP, string_value))
///      }
///  }
/// ```
pub struct PassthroughFbExt {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    ext_available: bool,
    enabled: bool,
    enable_on_init: bool,
    active_passtrough: PassthroughFB,
    active_layer: PassthroughLayerFB,
    old_color: Color128,
    old_sky: bool,
    xr_create_passthrough_fb: Option<CreatePassthroughFB>,
    xr_destroy_passthrough_fb: Option<DestroyPassthroughFB>,
    xr_passthrough_start_fb: Option<PassthroughStartFB>,
    xr_passthrough_pause_fb: Option<PassthroughPauseFB>,
    xr_create_passthrough_layer_fb: Option<CreatePassthroughLayerFB>,
    xr_destroy_passthrough_layer_fb: Option<DestroyPassthroughLayerFB>,
    xr_passthrough_layer_pause_fb: Option<PassthroughLayerPauseFB>,
    xr_passthrough_layer_resume_fb: Option<PassthroughLayerResumeFB>,
    xr_passthrough_layer_set_style_fb: Option<PassthroughLayerSetStyleFB>,
}

unsafe impl Send for PassthroughFbExt {}

impl Default for PassthroughFbExt {
    fn default() -> Self {
        Self {
            id: "PassthroughFbExt".to_string(),
            sk_info: None,
            ext_available: false,
            enabled: false,
            enable_on_init: false,
            active_passtrough: PassthroughFB::from_raw(0),
            active_layer: PassthroughLayerFB::from_raw(0),
            old_color: Color128::WHITE,
            old_sky: false,
            xr_create_passthrough_fb: BackendOpenXR::get_function::<CreatePassthroughFB>("xrCreatePassthroughFB"),
            xr_destroy_passthrough_fb: BackendOpenXR::get_function::<DestroyPassthroughFB>("xrDestroyPassthroughFB"),
            xr_passthrough_start_fb: BackendOpenXR::get_function::<PassthroughStartFB>("xrPassthroughStartFB"),
            xr_passthrough_pause_fb: BackendOpenXR::get_function::<PassthroughPauseFB>("xrPassthroughPauseFB"),
            xr_create_passthrough_layer_fb: BackendOpenXR::get_function::<CreatePassthroughLayerFB>(
                "xrCreatePassthroughLayerFB",
            ),
            xr_destroy_passthrough_layer_fb: BackendOpenXR::get_function::<DestroyPassthroughLayerFB>(
                "xrDestroyPassthroughLayerFB",
            ),
            xr_passthrough_layer_pause_fb: BackendOpenXR::get_function::<PassthroughLayerPauseFB>(
                "xrPassthroughLayerPauseFB",
            ),
            xr_passthrough_layer_resume_fb: BackendOpenXR::get_function::<PassthroughLayerResumeFB>(
                "xrPassthroughLayerResumeFB",
            ),
            xr_passthrough_layer_set_style_fb: BackendOpenXR::get_function::<PassthroughLayerSetStyleFB>(
                "xrPassthroughLayerSetStyleFB",
            ),
        }
    }
}

/// All the code here run in the main thread
impl IStepper for PassthroughFbExt {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);

        self.ext_available = Backend::xr_type() == BackendXRType::OpenXR
            && BackendOpenXR::ext_enabled("XR_FB_passthrough")
            && self.load_binding()
            && self.init_passthrough();

        self.ext_available
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn step(&mut self, token: &MainThreadToken) {
        // Here with enable/disable the passthrough
        for e in token.get_event_report().iter() {
            if let StepperAction::Event(_, key, value) = e {
                if key.eq(PASSTHROUGH_FLIP) {
                    if value == "0" {
                        self.enable(false)
                    } else {
                        self.enable(true)
                    }
                }
            }
        }
        if self.enabled() {
            let mut layer = CompositionLayerPassthroughFB {
                ty: StructureType::COMPOSITION_LAYER_PASSTHROUGH_FB,
                next: null_mut(),
                space: Space::from_raw(0),
                flags: CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
                layer_handle: self.active_layer,
            };
            BackendOpenXR::add_composition_layer(&mut layer, -1);
        }
    }

    fn shutdown(&mut self) {
        if self.enabled {
            self.enable(false);
            if self.ext_available {
                unsafe { self.xr_destroy_passthrough_layer_fb.unwrap()(self.active_layer) };
                unsafe { self.xr_destroy_passthrough_fb.unwrap()(self.active_passtrough) };
            }
        };
    }
}

impl PassthroughFbExt {
    /// Use this if you don't want passthrough at init.
    pub fn new(enabled: bool) -> Self {
        Self { enable_on_init: enabled, ..Default::default() }
    }

    pub fn enable(&mut self, value: bool) {
        if self.ext_available && self.enabled != value {
            if value {
                self.enabled = self.start_passthrough();
            } else {
                self.pause_passthrough();
                self.enabled = false;
            }
        }
    }

    fn init_passthrough(&mut self) -> bool {
        let flags = if self.enable_on_init {
            PassthroughFlagsFB::IS_RUNNING_AT_CREATION
        } else {
            PassthroughFlagsFB::EMPTY
        };

        match unsafe {
            self.xr_create_passthrough_fb.unwrap()(
                Session::from_raw(BackendOpenXR::session()),
                &PassthroughCreateInfoFB { ty: StructureType::PASSTHROUGH_CREATE_INFO_FB, next: null_mut(), flags },
                &mut self.active_passtrough,
            )
        } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrCreatePassthroughFB failed: {otherwise}"));
                return false;
            }
        }

        match unsafe {
            self.xr_create_passthrough_layer_fb.unwrap()(
                Session::from_raw(BackendOpenXR::session()),
                &PassthroughLayerCreateInfoFB {
                    ty: StructureType::PASSTHROUGH_LAYER_CREATE_INFO_FB,
                    next: null_mut(),
                    passthrough: self.active_passtrough,
                    flags,
                    purpose: PassthroughLayerPurposeFB::RECONSTRUCTION,
                },
                &mut self.active_layer,
            )
        } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrCreatePassthroughLayerFB failed: {otherwise}"));
                return false;
            }
        }
        self.enable(self.enable_on_init);
        if self.enabled {
            self.start_sky();
        }
        true
    }

    fn start_passthrough(&mut self) -> bool {
        match unsafe { self.xr_passthrough_start_fb.unwrap()(self.active_passtrough) } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrPassthroughStartFB failed: {otherwise}"));
                return false;
            }
        }

        match unsafe { self.xr_passthrough_layer_resume_fb.unwrap()(self.active_layer) } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrPassthroughLayerResumeFB failed: {otherwise}"));
                return false;
            }
        }

        self.start_sky();
        true
    }

    fn start_sky(&mut self) {
        self.old_color = Renderer::get_clear_color();
        self.old_sky = Renderer::get_enable_sky();
        Renderer::clear_color(Color128::BLACK_TRANSPARENT);
        Renderer::enable_sky(false);
    }

    fn pause_passthrough(&mut self) {
        match unsafe { self.xr_passthrough_layer_pause_fb.unwrap()(self.active_layer) } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrPassthroughLayerPauseFB failed: {otherwise}"));
                return;
            }
        }

        match unsafe { self.xr_passthrough_pause_fb.unwrap()(self.active_passtrough) } {
            Result::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrPassthroughPauseFB failed: {otherwise}"));
                return;
            }
        }
        Renderer::clear_color(self.old_color);
        Renderer::enable_sky(self.old_sky);
    }

    /// Check if all the binded functions are ready.
    fn load_binding(&mut self) -> bool {
        self.xr_create_passthrough_fb.is_some()
            && self.xr_destroy_passthrough_fb.is_some()
            && self.xr_passthrough_start_fb.is_some()
            && self.xr_passthrough_pause_fb.is_some()
            && self.xr_create_passthrough_layer_fb.is_some()
            && self.xr_destroy_passthrough_layer_fb.is_some()
            && self.xr_passthrough_layer_pause_fb.is_some()
            && self.xr_passthrough_layer_resume_fb.is_some()
            && self.xr_passthrough_layer_set_style_fb.is_some()
    }
}
