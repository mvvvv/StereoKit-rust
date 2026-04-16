//! XR_META_detached_controllers extension implementation
//!
//! This module provides access to the OpenXR XR_META_detached_controllers extension,
//! which allows applications to track and render controllers that are detached from the
//! user's hands (e.g., controllers placed on a surface while using hand tracking).
//!
//! When simultaneous hand and controller tracking is active (XR_META_simultaneous_hands_and_controllers),
//! the runtime may report that a controller is "detached" — meaning the user's hand is tracked via
//! articulated hand tracking, while the physical controller is somewhere else (on a desk, in a pocket, etc.).
//!
//! The stepper draws controller models at the appropriate poses:
//! - When a controller is attached (held by the user, `HandSource::Simulated`): draws the controller
//!   model at the regular controller pose from `Input::controller()`.
//! - When a controller is detached (articulated hand active, detached profile detected): draws the
//!   controller model at the detached controller pose.
//! - When hand tracking is active with no controller: does not draw a controller.
//!
//! Detection of the detached state is done by querying the OpenXR interaction profiles for the
//! `/user/detached_controller_meta/left` and `/user/detached_controller_meta/right` top-level
//! paths, mirroring the logic of `oxri_update_profiles()` in the C++ StereoKit code.
//!
//! ## Detached controller pose
//!
//! Retrieving the pose of a detached controller requires OpenXR action bindings on the
//! `/user/detached_controller_meta/*` top-level paths. This is implemented in the C++ PR
//! via `oxri_register_profile()` which adds `grip/pose` actions for `detached_pose_l` and
//! `detached_pose_r`. Once that PR is merged into the StereoKit submodule, the C function
//! `input_controller_detached(hand)` becomes available and provides the detached pose.
//!
//! Until then, the stepper uses `Input::controller()` as a best-effort fallback when a
//! detached controller is detected. This works when the runtime still provides controller
//! tracking data on the standard `/user/hand/*` paths (e.g. with
//! `XR_META_simultaneous_hands_and_controllers`).
//!
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_META_detached_controllers>
//!
//! See also the StereoKit C++ implementation:
//! <https://github.com/StereoKit/StereoKit/pull/1272>

use std::ffi::CString;

use openxr_sys::{
    Handle, Instance, InteractionProfileState, Path, Result as XrResult, Session,
    pfn::{GetCurrentInteractionProfile, StringToPath},
};

use crate::{
    interactor::{InteractorController, InteractorHand},
    maths::Pose,
    prelude::*,
    system::{Backend, BackendOpenXR, BackendXRType, Handed, Input, Interaction, Log},
};

/// Extension name constant for XR_META_detached_controllers
pub const XR_META_DETACHED_CONTROLLERS_EXTENSION_NAME: &str = "XR_META_detached_controllers";

/// Stepper ID used for the [`XrMetaDetachedControllersStepper`] managed by
/// [`resume_simultaneous_hands_and_controllers`](super::xr_meta_simultaneous_hands_controllers::resume_simultaneous_hands_and_controllers)
/// and [`pause_simultaneous_hands_and_controllers`](super::xr_meta_simultaneous_hands_controllers::pause_simultaneous_hands_and_controllers).
pub const META_DETACHED_CTRLRS_ID: &str = "Tool_MetaDetachedCtrlrsID";

/// Check if the XR_META_detached_controllers extension is available in the current runtime.
///
/// Returns true if the OpenXR backend is active and the extension is enabled.
///
/// see also [`XrMetaDetachedControllersStepper`]
/// ### Examples
/// ```
/// use stereokit_rust::system::BackendOpenXR;
/// BackendOpenXR::request_ext("XR_META_detached_controllers");
///
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::tools::xr_meta_detached_controllers::is_meta_detached_controllers_available;
///
/// let available = is_meta_detached_controllers_available();
/// // On PC/simulator this will be false, on Meta Quest it may be true
/// assert_eq!(available, false);
/// # sk::Sk::shutdown();
/// ```
pub fn is_meta_detached_controllers_available() -> bool {
    Backend::xr_type() == BackendXRType::OpenXR
        && BackendOpenXR::ext_enabled(XR_META_DETACHED_CONTROLLERS_EXTENSION_NAME)
}

/// Holds the state needed for querying OpenXR interaction profiles to detect detached controllers.
///
/// This mirrors the logic of `oxri_update_profiles()` in the C++ StereoKit code:
/// for each detached controller top-level path, we call `xrGetCurrentInteractionProfile`
/// to check whether the runtime has activated a profile for that path.
struct DetachedProfileState {
    xr_get_current_interaction_profile: GetCurrentInteractionProfile,
    session: Session,
    detached_path_left: Path,
    detached_path_right: Path,
    left_detached: bool,
    right_detached: bool,
}

impl DetachedProfileState {
    /// Initialize the profile detection state.
    ///
    /// Converts the detached controller top-level path strings to XrPath values
    /// and stores function pointers for querying interaction profiles.
    ///
    /// Returns `None` if the required OpenXR functions are not available or if the
    /// path strings cannot be converted (e.g., the extension is not enabled by the runtime).
    fn new() -> Option<Self> {
        let xr_string_to_path = BackendOpenXR::get_function::<StringToPath>("xrStringToPath")?;
        let xr_get_current_interaction_profile =
            BackendOpenXR::get_function::<GetCurrentInteractionProfile>("xrGetCurrentInteractionProfile")?;

        let instance = Instance::from_raw(BackendOpenXR::instance());
        let session = Session::from_raw(BackendOpenXR::session());

        let left_str = CString::new("/user/detached_controller_meta/left").ok()?;
        let right_str = CString::new("/user/detached_controller_meta/right").ok()?;

        let mut path_left = Path::NULL;
        let mut path_right = Path::NULL;

        unsafe {
            if xr_string_to_path(instance, left_str.as_ptr(), &mut path_left) != XrResult::SUCCESS {
                Log::diag("Failed to convert /user/detached_controller_meta/left to XrPath");
                return None;
            }
            if xr_string_to_path(instance, right_str.as_ptr(), &mut path_right) != XrResult::SUCCESS {
                Log::diag("Failed to convert /user/detached_controller_meta/right to XrPath");
                return None;
            }
        }

        Some(Self {
            xr_get_current_interaction_profile,
            session,
            detached_path_left: path_left,
            detached_path_right: path_right,
            left_detached: false,
            right_detached: false,
        })
    }

    /// Query the OpenXR runtime to update the detached state for both hands.
    ///
    /// This mirrors the relevant part of `oxri_update_profiles()`:
    /// for each `/user/detached_controller_meta/*` top-level path, call
    /// `xrGetCurrentInteractionProfile` and check whether a non-null profile is active.
    fn update_profiles(&mut self) {
        self.left_detached = self.check_profile_active(self.detached_path_left);
        self.right_detached = self.check_profile_active(self.detached_path_right);
    }

    /// Check whether the given top-level path has an active interaction profile.
    fn check_profile_active(&self, path: Path) -> bool {
        let mut profile_state = InteractionProfileState {
            ty: InteractionProfileState::TYPE,
            next: std::ptr::null_mut(),
            interaction_profile: Path::NULL,
        };

        let result = unsafe { (self.xr_get_current_interaction_profile)(self.session, path, &mut profile_state) };

        result == XrResult::SUCCESS && profile_state.interaction_profile != Path::NULL
    }

    /// Returns whether the given hand has a detached controller.
    fn is_detached(&self, hand: Handed) -> bool {
        match hand {
            Handed::Left => self.left_detached,
            Handed::Right => self.right_detached,
            _ => false,
        }
    }
}

/// IStepper implementation for drawing controllers (detached or attached).
///
/// This stepper manages the rendering of controller models based on the current hand/controller
/// tracking state. It supports both the standard case (controllers held in hand) and the
/// detached case (controllers tracked separately from hand tracking via
/// `XR_META_detached_controllers`).
///
/// Detection of the detached state is performed each frame by querying OpenXR interaction
/// profiles for `/user/detached_controller_meta/left|right`.
///
/// ### Rendering logic per hand:
/// 1. **Articulated hand + detached controller**: The hand mesh is drawn by StereoKit natively.
///    This stepper draws the controller model at the detached controller pose.
/// 2. **Controller only (simulated hand)**: This stepper draws the controller model at the
///    regular controller pose.
/// 3. **Articulated hand, no controller**: Nothing extra to draw (hand mesh is native).
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{
///     tools::xr_meta_detached_controllers::{XrMetaDetachedControllersStepper, META_DETACHED_CTRLRS_ID},
///     prelude::*,
/// };
///
/// // Add the stepper to StereoKit
/// sk.send_event(StepperAction::add_default::<XrMetaDetachedControllersStepper>(
///     META_DETACHED_CTRLRS_ID,
/// ));
///
/// test_steps!( // !!!! Get a proper main loop !!!!
///     // The stepper automatically draws controllers based on tracking state
/// );
/// # sk::Sk::shutdown();
/// ```
#[derive(IStepper)]
pub struct XrMetaDetachedControllersStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,
    shutdown_completed: bool,

    /// OpenXR profile detection state. We'll be initialized at start()
    profile_state: Option<DetachedProfileState>,

    /// was the controller detached in the previous frame? Used to detect changes in state and avoid unnecessary updates.
    was_detached: [bool; 2],

    /// Interactors for left and right. Created on start, stepped each frame.
    controller_interactors: [Option<InteractorController>; 2],
    hand_interactors: [Option<InteractorHand>; 2],
}

impl Default for XrMetaDetachedControllersStepper {
    fn default() -> Self {
        Self {
            id: "XrMetaDetachedControllersStepper".to_string(),
            sk_info: None,
            enabled: true,
            shutdown_completed: false,

            profile_state: None,
            was_detached: [false; 2],
            controller_interactors: [None, None],
            hand_interactors: [None, None],
        }
    }
}

unsafe impl Send for XrMetaDetachedControllersStepper {}

impl XrMetaDetachedControllersStepper {
    /// Called from IStepper::initialize — sets up OpenXR profile detection.
    fn start(&mut self) -> bool {
        if !is_meta_detached_controllers_available() {
            Log::warn("XR_META_detached_controllers extension is not available");
            return false;
        }

        if self.id != META_DETACHED_CTRLRS_ID {
            Log::err(format!(
                "XrMetaDetachedControllersStepper: Wrong Unique ID, expected {}, got {}",
                META_DETACHED_CTRLRS_ID, self.id
            ));
            return false;
        }

        match DetachedProfileState::new() {
            Some(state) => {
                self.profile_state = Some(state);
            }
            None => {
                Log::err(
                    "XR_META_detached_controllers: could not initialize profile detection \
                     (xrStringToPath or xrGetCurrentInteractionProfile unavailable).",
                );
                return false;
            }
        }

        // Create controller interactors for both hands
        self.controller_interactors =
            [Some(InteractorController::new(Handed::Left)), Some(InteractorController::new(Handed::Right))];
        self.hand_interactors = [Some(InteractorHand::new(Handed::Left)), Some(InteractorHand::new(Handed::Right))];

        // Disable far interac since we provide our own ray-based interactors
        // Ui::enable_far_interact(false);
        Interaction::set_default_draw(false);
        true
    }

    /// No events to handle — controllers are always drawn.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Draw a controller model for the given hand at the given pose.
    ///
    /// Uses the controller model assigned by the user or SK's default model.
    fn draw_controller_at_pose(token: &MainThreadToken, hand: Handed, pose: Pose) {
        let model = Input::get_controller_model(hand);
        model.draw(token, pose, None, None);
    }

    /// Draw controllers for a single hand based on current tracking state.
    ///
    fn draw_hand_controller(&mut self, token: &MainThreadToken, hand: Handed) {
        let controller = Input::controller(hand);
        let hand_idx = hand as usize;

        let is_detached = self.profile_state.as_ref().is_some_and(|state| state.is_detached(hand));
        if is_detached {
            // Controller is put down. Input::controller() now tracks the hand,
            // not the physical controller. Draw at the last pose from when it
            // was still held.
            let pose = Input::get_controller_detached(hand);
            Self::draw_controller_at_pose(token, hand, pose);
            if let Some(ref mut hand_interactor) = self.hand_interactors[hand_idx] {
                hand_interactor.step();
                hand_interactor.draw_ray(token);
            }
            self.was_detached[hand_idx] = true;
        } else {
            // Controller is held in hand — update saved pose and draw.
            if controller.is_tracked() {
                Self::draw_controller_at_pose(token, hand, controller.pose);
                // Controller-based hand simulation — step the controller interactor
                // so it drives UI interaction via its aim ray, then draw the model.
                if let Some(ref mut ctrl_interactor) = self.controller_interactors[hand_idx] {
                    ctrl_interactor.step();
                    ctrl_interactor.draw_ray(token);
                    if self.was_detached[hand_idx] {}
                }
            } else {
                if let Some(ref mut hand_interactor) = self.hand_interactors[hand_idx] {
                    hand_interactor.step();
                    hand_interactor.draw_ray(token);
                }
            }
            self.was_detached[hand_idx] = false;
        }
    }

    /// Called from IStepper::step — queries profiles and draws controllers each frame.
    fn draw(&mut self, token: &MainThreadToken) {
        // Update the detached profile state by querying the OpenXR interaction profiles,
        // mirroring oxri_update_profiles() from the C++ code.
        if let Some(ref mut state) = self.profile_state {
            state.update_profiles();
        }

        self.draw_hand_controller(token, Handed::Left);
        self.draw_hand_controller(token, Handed::Right);
    }

    /// Clean up controller interactors on shutdown.
    fn close(&mut self, triggering: bool) -> bool {
        if !triggering {
            return self.shutdown_completed;
        }
        //Ui::enable_far_interact(true);
        Interaction::set_default_draw(true);
        self.shutdown_completed = true;
        self.shutdown_completed
    }
}
