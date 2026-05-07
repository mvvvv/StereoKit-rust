use crate::{
    maths::{Bool32T, Bounds, Pose, Vec3},
    system::BtnState,
    ui::IdHashT,
    util::Color32,
};

/// Should this interactor behave like a single point in space interacting with elements? Or should it behave more like
/// an intangible line? Hit detection is still capsule shaped, but behavior may change a little to reflect the primary
/// position of the point interactor. This can also be thought of as direct interaction vs indirect interaction.
/// <https://stereokit.net/Pages/StereoKit/InteractorType.html>
///
/// see also [`Interactor`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum InteractorType {
    /// The interactor represents a physical point in space, such as a fingertip or the point of a pencil. Points do not
    /// use directionality for their interactions, nor do they take into account the distance of an element along the
    /// 'ray' of the capsule.
    Point = 0,
    /// The interactor represents a less tangible line or ray of interaction, such as a laser pointer or eye gaze. Lines
    /// will occasionally consider the directionality of the interactor to discard backpressing certain elements, and
    /// use distance along the line for occluding elements that are behind other elements.
    Line = 1,
}

/// This describes how an interactor activates elements. Does it use the
/// physical position of the interactor, or the activation state?
/// <https://stereokit.net/Pages/StereoKit/InteractorActivation.html>
///
/// see also [`Interactor`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum InteractorActivation {
    /// This interactor uses its `active` state to determine element activation.
    State = 0,
    /// This interactor uses its motion position to determine the element activation.
    Position = 1,
}

bitflags::bitflags! {
    /// A bit-flag mask for interaction event types. This allows or informs what type of events an interactor can perform,
    /// or an element can respond to.
    /// <https://stereokit.net/Pages/StereoKit/InteractorEvent.html>
    ///
    /// see also [`Interactor`]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct InteractorEvent: u32 {
        /// Poke events represent direct physical interaction with elements via a single point. This might be like a
        /// fingertip pressing a button, or a pencil tip on a page of a paper.
        const Poke = 1 << 1;
        /// Grip events represent the gripping gesture of the hand. This can also map to something like the grip button on
        /// a controller. This is generally for larger objects where humans have a tendency to make full fisted grasping
        /// motions, like with door handles or sword hilts.
        const Grip = 1 << 2;
        /// Pinch events represent the pinching gesture of the hand, where the index finger tip and thumb tip come
        /// together. This can also map to something like the trigger button of a controller. This is generally for
        /// smaller objects where humans tend to grasp more delicately with just their fingertips, like with a pencil
        /// or switches.
        const Pinch = 1 << 3;
    }
}

/// Options for what type of interactors StereoKit provides by default.
/// <https://stereokit.net/Pages/StereoKit/DefaultInteractors.html>
///
/// see also [`Interactor`] [`Interaction`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DefaultInteractors {
    /// Use the XR backend's default interactor mode. This is 'all' for XR, 'mouse' for simulator and window, and
    /// 'none' for offscreen.
    Default = 0,
    /// Don't provide any interactors at all. This means you either don't want interaction, or are providing your own
    /// custom interactors.
    None = 1,
    /// Auto-switch between hands and controllers based on the current input source. This provides aim, pinch, and poke
    /// interactors for hands, and aim rays for controllers.
    All = 2,
    /// Always use the hand interactors, using simulated hands when articulated hand tracking is not available.
    Hands = 3,
    /// Always use the controller interactors.
    Controllers = 4,
    /// Always use the mouse interactor.
    Mouse = 5,
}

unsafe extern "C" {
    // Interactor functions
    pub fn interactor_create(
        shape_type: InteractorType,
        events: InteractorEvent,
        activation_type: InteractorActivation,
        input_source_id: i32,
        capsule_radius: f32,
        secondary_motion_dimensions: i32,
    ) -> i32;
    pub fn interactor_destroy(interactor: i32);
    pub fn interactor_update(
        interactor: i32,
        capsule_start: Vec3,
        capsule_end: Vec3,
        motion: Pose,
        motion_anchor: Vec3,
        secondary_motion: Vec3,
        active: BtnState,
        tracked: BtnState,
    );
    pub fn interactor_set_min_distance(interactor: i32, min_distance: f32);
    pub fn interactor_get_min_distance(interactor: i32) -> f32;
    pub fn interactor_get_capsule_start(interactor: i32) -> Vec3;
    pub fn interactor_get_capsule_end(interactor: i32) -> Vec3;
    pub fn interactor_set_radius(interactor: i32, radius: f32);
    pub fn interactor_get_radius(interactor: i32) -> f32;
    pub fn interactor_get_tracked(interactor: i32) -> BtnState;
    pub fn interactor_get_focused(interactor: i32) -> IdHashT;
    pub fn interactor_get_active(interactor: i32) -> IdHashT;
    pub fn interactor_get_focus_bounds(
        interactor: i32,
        out_pose_world: *mut Pose,
        out_bounds_local: *mut Bounds,
        out_at_local: *mut Vec3,
    ) -> Bool32T;
    pub fn interactor_get_motion(interactor: i32) -> Pose;
    pub fn interactor_count() -> i32;
    pub fn interactor_get(index: i32) -> i32;

    // Interaction system functions
    pub fn interaction_set_default_interactors(default_interactors: DefaultInteractors);
    pub fn interaction_get_default_interactors() -> DefaultInteractors;
    pub fn interaction_set_default_draw(draw_interactors: Bool32T);
    pub fn interaction_get_default_draw() -> Bool32T;
}

/// Interactors are essentially capsules that allow interaction with StereoKit's interaction primitives used by the UI
/// system. While StereoKit does provide a number of interactors by default, you can replace StereoKit's defaults, add
/// additional interactors, or generally just customize your interactions!
/// <https://stereokit.net/Pages/StereoKit/Interactor.html>
///
/// see also [`InteractorType`] [`InteractorEvent`] [`InteractorActivation`] [`DefaultInteractors`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{system::{Interactor, InteractorType, InteractorEvent, InteractorActivation, BtnState},
///                      maths::{Vec3, Pose}};
///
/// let interactor = Interactor::create(
///     InteractorType::Point,
///     InteractorEvent::Poke,
///     InteractorActivation::State,
///     0,
///     0.01,
///     0
/// );
///
/// interactor.update(
///     Vec3::new(1.0, 2.0, 3.0),
///     Vec3::new(4.0, 5.0, 6.0),
///     Pose::IDENTITY,
///     Vec3::ZERO,
///     Vec3::ZERO,
///     BtnState::Active,
///     BtnState::Active
/// );
///
/// let radius = interactor.get_radius();
/// let start = interactor.get_start();
/// let end = interactor.get_end();
/// let tracked = interactor.get_tracked();
/// let focused = interactor.get_focused();
/// let active = interactor.get_active();
/// let motion = interactor.get_motion();
///
///
/// assert_eq!(radius,  0.01);
/// assert_eq!(start,   Vec3::new(1.0, 2.0, 3.0));
/// assert_eq!(end,     Vec3::new(4.0, 5.0, 6.0));
/// assert_eq!(tracked, BtnState::Active);
/// assert_eq!(active,  0u64);
/// assert_eq!(focused, 0u64);
/// assert_eq!(motion,  Pose::IDENTITY);
///
/// # sk::Sk::shutdown();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interactor {
    inst: i32,
}

/// Interactors, unlike Assets, don't destroy themselves! You must explicitly Destroy an Interactor if you're
/// finished with it, otherwise it will continue to interact with StereoKit's interactors. This function immediately
/// removes the interactor from the interactor list.
/// <https://stereokit.net/Pages/StereoKit/Interactor/Destroy.html>
///
/// see also [`interactor_destroy`]
impl Drop for Interactor {
    fn drop(&mut self) {
        unsafe {
            interactor_destroy(self.inst);
        }
    }
}

impl Interactor {
    /// Create a new custom Interactor.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Create.html>
    /// * `shape_type` - A line, or a point? These interactors behave slightly differently with respect to distance
    ///   checks and directionality. See `InteractorType` for more details.
    /// * `events` - What type of interaction events should this interactor fire? Interaction elements use this bitflag
    ///   as a filter to avoid interacting with certain interactors.
    /// * `activation_type` - How does this interactor activate elements?
    /// * `input_source_id` - An identifier that uniquely indicates a shared source for inputs. This will deactivate
    ///   other interactors with a shared source if one is already active. For example, 3 interactors for poke, pinch,
    ///   and aim on a hand would all come from a single hand, and if one is actively interacting, then the whole hand
    ///   source is considered busy.
    /// * `capsule_radius` - The radius of the interactor's capsule, in meters.
    /// * `secondary_motion_dimensions` - How many axes of secondary motion can this interactor provide? This should be 0-3.
    ///
    /// Returns the Interactor that was just created.
    /// see also [`interactor_create`]
    pub fn create(
        shape_type: InteractorType,
        events: InteractorEvent,
        activation_type: InteractorActivation,
        input_source_id: i32,
        capsule_radius: f32,
        secondary_motion_dimensions: i32,
    ) -> Self {
        let inst = unsafe {
            interactor_create(
                shape_type,
                events,
                activation_type,
                input_source_id,
                capsule_radius,
                secondary_motion_dimensions,
            )
        };
        Self { inst }
    }

    /// Update the interactor with data for the current frame! This should be called as soon as possible at the start
    /// of the frame before any UI is done, otherwise the UI will not properly react.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Update.html>
    /// * `capsule_start` - World space location of the collision capsule's start. For Line interactors, this should be
    ///   the 'origin' of the capsule's orientation.
    /// * `capsule_end` - World space location of the collision capsule's end. For Line interactors, this should be in
    ///   the direction the Start/origin is facing.
    /// * `motion` - This pose is the source of translation and rotation motion caused by the interactor. In most cases
    ///   it will be the same as your capsuleStart with the orientation of your interactor, but in some instance may be
    ///   something else!
    /// * `motion_anchor` - Some motion, like that of amplified motion, needs some anchor point with which to determine
    ///   the amplification from. This might be a shoulder, or a head, or some other point that the interactor will
    ///   push from / pull towards.
    /// * `secondary_motion` - This is motion that comes from somewhere other than the interactor itself! This can be
    ///   something like an analog stick on a controller, or the scroll wheel of a mouse.
    /// * `active` - The activation state of the Interactor.
    /// * `tracked` - The tracking state of the Interactor.
    ///
    /// see also [`interactor_update`]
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        capsule_start: impl Into<Vec3>,
        capsule_end: impl Into<Vec3>,
        motion: impl Into<Pose>,
        motion_anchor: impl Into<Vec3>,
        secondary_motion: impl Into<Vec3>,
        active: BtnState,
        tracked: BtnState,
    ) {
        unsafe {
            interactor_update(
                self.inst,
                capsule_start.into(),
                capsule_end.into(),
                motion.into(),
                motion_anchor.into(),
                secondary_motion.into(),
                active,
                tracked,
            );
        }
    }

    /// The distance at which a ray starts being interactive. For pointing rays, you may not want them to interact
    /// right at their start, or you may want the start to move depending on how outstretched the hand is! This allows
    /// you to change that start location without affecting the movement caused by the ray, and still capturing
    /// occlusion from blocking elements too close to the start. By default, this is a large negative value.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/MinDistance.html>
    ///
    /// see also [`interactor_get_min_distance`] [`Interactor::min_distance`]
    pub fn get_min_distance(&self) -> f32 {
        unsafe { interactor_get_min_distance(self.inst) }
    }

    /// Set the distance at which a ray starts being interactive.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/MinDistance.html>
    ///
    /// see also [`interactor_set_min_distance`] [`Interactor::get_min_distance`]
    pub fn min_distance(&self, min_distance: f32) {
        unsafe { interactor_set_min_distance(self.inst, min_distance) }
    }

    /// The world space radius of the interactor capsule, in meters.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Radius.html>
    ///
    /// see also [`interactor_get_radius`] [`Interactor::radius`]
    pub fn get_radius(&self) -> f32 {
        unsafe { interactor_get_radius(self.inst) }
    }

    /// Set the world space radius of the interactor capsule, in meters.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Radius.html>
    ///
    /// see also [`interactor_set_radius`] [`Interactor::get_radius`]
    pub fn radius(&self, radius: f32) {
        unsafe { interactor_set_radius(self.inst, radius) }
    }

    /// The world space start of the interactor capsule. Some interactions can be directional, especially for `Line`
    /// type interactors, so if you think of the interactor as an "oriented" capsule, this would be the origin which
    /// points towards the capsule `End`.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Start.html>
    ///
    /// see also [`interactor_get_capsule_start`] [`Interactor::get_end`]
    pub fn get_start(&self) -> Vec3 {
        unsafe { interactor_get_capsule_start(self.inst) }
    }

    /// The world space end of the interactor capsule. Some interactions can be directional, especially for `Line`
    /// type interactors, so if you think of the interactor as an "oriented" capsule, this would be the end which the
    /// `Start`/origin points towards.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/End.html>
    ///
    /// see also [`interactor_get_capsule_end`] [`Interactor::get_start`]
    pub fn get_end(&self) -> Vec3 {
        unsafe { interactor_get_capsule_end(self.inst) }
    }

    /// The tracking state of this interactor.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Tracked.html>
    ///
    /// see also [`interactor_get_tracked`] [`Interactor::get_focused`] [`Interactor::get_active`]
    pub fn get_tracked(&self) -> BtnState {
        unsafe { interactor_get_tracked(self.inst) }
    }

    /// The id of the interaction element that is currently focused, this will be `IdHashT::NONE` if this interactor
    /// has nothing focused.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Focused.html>
    ///
    /// see also [`interactor_get_focused`] [`Interactor::get_active`]
    pub fn get_focused(&self) -> IdHashT {
        unsafe { interactor_get_focused(self.inst) }
    }

    /// The id of the interaction element that is currently active, this will be `IdHashT::NONE` if this interactor
    /// has nothing active. This will always be the same id as `Focused` when not `None`.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Active.html>
    ///
    /// see also [`interactor_get_active`] [`Interactor::get_focused`]
    pub fn get_active(&self) -> IdHashT {
        unsafe { interactor_get_active(self.inst) }
    }

    /// This pose is the source of translation and rotation motion caused by the interactor. In most cases it will be
    /// the same as your Start with the orientation of your interactor, but in some instance may be something else!
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Motion.html>
    ///
    /// see also [`interactor_get_motion`] [`Interactor::update`]
    pub fn get_motion(&self) -> Pose {
        unsafe { interactor_get_motion(self.inst) }
    }

    /// If this interactor has an element focused, this will output information about the location of that element, as
    /// well as the interactor's intersection point with that element.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/TryGetFocusBounds.html>
    /// * `pose_world` - The world space Pose of the element's hierarchy space. This is typically the Pose of the
    ///   Window/Handle/Surface the element belongs to.
    /// * `bounds_local` - The bounds of the UI element relative to the Pose. Note that the `center` should always be
    ///   accounted for here!
    /// * `at_local` - The intersection point relative to the Bounds, NOT relative to the Pose!
    ///
    /// Returns `Some((pose_world, bounds_local, at_local))` if bounds data is available, `None` otherwise.
    ///
    /// see also [`interactor_get_focus_bounds`] [`Interactor::get_focused`]
    pub fn try_get_focus_bounds(&self) -> Option<(Pose, Bounds, Vec3)> {
        let mut pose_world = Pose::IDENTITY;
        let mut bounds_local = Bounds::default();
        let mut at_local = Vec3::ZERO;

        let result =
            unsafe { interactor_get_focus_bounds(self.inst, &mut pose_world, &mut bounds_local, &mut at_local) };

        if result != 0 { Some((pose_world, bounds_local, at_local)) } else { None }
    }

    /// The number of interactors currently in the system. Can be used with `get`.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Count.html>
    ///
    /// see also [`interactor_count`] [`Interactor::get`]
    pub fn count() -> i32 {
        unsafe { interactor_count() }
    }

    /// Returns the `Interactor` at the given index. Should be used with `count`.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Get.html>
    /// * `index` - The index.
    ///
    /// Returns an Interactor.
    /// see also [`interactor_get`] [`Interactor::count`]
    pub fn get(index: i32) -> Option<Self> {
        if index < 0 || index >= Self::count() {
            None
        } else {
            let inst = unsafe { interactor_get(index) };
            Some(Self { inst })
        }
    }
}

/// Controls for the interaction system, and the interactors that StereoKit provides by default.
/// <https://stereokit.net/Pages/StereoKit/Interaction.html>
///
/// see also [`DefaultInteractors`] [`Interactor`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::system::{Interaction, DefaultInteractors};
///
/// // Set the default interactors
/// Interaction::set_default_interactors(DefaultInteractors::Default);
///
/// // Check what interactors are currently set
/// let current_interactors = Interaction::get_default_interactors();
/// assert_eq!(current_interactors, DefaultInteractors::Default);
///
/// // Disable the default drawing of interactor indicators
/// Interaction::set_default_draw(false);
///
/// // Check if default drawing is enabled
/// assert_eq!(Interaction::get_default_draw(), false);
/// # sk::Sk::shutdown();
/// ```
pub struct Interaction;

impl Interaction {
    /// This allows you to control what kind of interactors StereoKit will provide for you.
    /// This also allows you to entirely disable StereoKit's interactors so you can just use custom ones!
    /// <https://stereokit.net/Pages/StereoKit/Interaction/DefaultInteractors.html>
    ///
    /// see also [`interaction_get_default_interactors`] [`Interaction::set_default_interactors`]
    pub fn get_default_interactors() -> DefaultInteractors {
        unsafe { interaction_get_default_interactors() }
    }

    /// Set what kind of interactors StereoKit will provide for you.
    /// <https://stereokit.net/Pages/StereoKit/Interaction/DefaultInteractors.html>
    ///
    /// see also [`interaction_set_default_interactors`] [`Interaction::get_default_interactors`]
    pub fn set_default_interactors(default_interactors: DefaultInteractors) {
        unsafe { interaction_set_default_interactors(default_interactors) }
    }

    /// By default, StereoKit will draw indicators for some of the default interactors, such as the far interaction /
    /// aiming rays. This doesn't affect custom interactors. Setting this to false will prevent StereoKit from drawing
    /// any of these indicators.
    /// <https://stereokit.net/Pages/StereoKit/Interaction/DefaultDraw.html>
    ///
    /// see also [`interaction_get_default_draw`] [`Interaction::set_default_draw`]
    pub fn get_default_draw() -> bool {
        unsafe { interaction_get_default_draw() != 0 }
    }

    /// Set whether StereoKit should draw indicators for the default interactors.
    /// <https://stereokit.net/Pages/StereoKit/Interaction/DefaultDraw.html>
    ///
    /// see also [`interaction_set_default_draw`] [`Interaction::get_default_draw`]
    pub fn set_default_draw(draw_interactors: bool) {
        unsafe { interaction_set_default_draw(if draw_interactors { 1 } else { 0 }) }
    }
}

/// A controller-based interactor that mirrors `interact_mode_controllers*()` from the C++ StereoKit code.
///
/// This creates a `Line` interactor with `Poke | Pinch` events and `State`-based activation,
/// driven by a controller's aim pose and trigger. It is intended for a single hand.
///
/// see also [`Interactor`] [`crate::system::Controller`] [`crate::system::Input::controller`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::interactor::InteractorController;
/// use stereokit_rust::system::Handed;
///
/// let mut ctrl_interactor = InteractorController::new(Handed::Right);
///
/// assert_eq!(ctrl_interactor.hand(), Handed::Right);
///
/// test_steps!( // !!!! Get a proper main loop !!!!
///     ctrl_interactor.step();
///     ctrl_interactor.draw_ray(token);
/// );
///
/// let interactor = ctrl_interactor.interactor();
/// assert_eq!(interactor.get_radius(), 0.005);
///
/// # sk::Sk::shutdown();
/// ```
pub struct InteractorController {
    interactor: Interactor,
    hand: crate::system::Handed,
    trigger_last: f32,
    ray_visible: f32,
    ray_active: f32,
}

impl InteractorController {
    /// Create a new controller interactor for the given hand.
    ///
    /// Mirrors `interact_mode_controllers_start()`:
    /// creates a `Line` interactor with `Poke | Pinch` events, `State` activation,
    /// capsule radius 0.005 m, and 2 secondary motion dimensions (stick X/Y).
    pub fn new(hand: crate::system::Handed) -> Self {
        let interactor = Interactor::create(
            InteractorType::Line,
            InteractorEvent::Poke | InteractorEvent::Pinch,
            InteractorActivation::State,
            -1,
            0.005,
            2,
        );
        Self { interactor, hand, trigger_last: 0.0, ray_visible: 0.0, ray_active: 0.0 }
    }

    /// Update the interactor with the current frame's controller data.
    ///
    /// Mirrors `interact_mode_controllers_step()`:
    /// - capsule start/end: aim ray extending 100 m forward
    /// - motion: aim pose
    /// - motion anchor: head position offset by -0.12 m on Y
    /// - secondary motion: stick * 0.02
    /// - active: trigger > 0.5 transitions
    /// - tracked: controller tracked state
    pub fn step(&mut self) {
        let ctrl = crate::system::Input::controller(self.hand);
        let head = crate::system::Input::get_head();

        // aim ray: start at aim position, end 100 m in the aim direction
        let capsule_start = ctrl.aim.position;
        let capsule_end = ctrl.aim.position + ctrl.aim.orientation * Vec3::FORWARD * 100.0;

        let motion = ctrl.aim;
        let motion_anchor = head.position + Vec3::new(0.0, -0.12, 0.0);
        let secondary_motion = Vec3::new(ctrl.stick.x, ctrl.stick.y, 0.0) * 0.02;

        let active = BtnState::make(self.trigger_last > 0.5, ctrl.trigger > 0.5);
        let tracked = ctrl.tracked;

        self.interactor
            .update(capsule_start, capsule_end, motion, motion_anchor, secondary_motion, active, tracked);
        self.trigger_last = ctrl.trigger;
    }

    /// Draw the controller aim ray indicator, mirroring `interactor_show_ray()` from the C++ code.
    ///
    /// Draws a smooth curved line from the interactor's aim origin outward. The ray animates
    /// its visibility and thickness based on whether the interactor has a focused or active element.
    ///
    /// * `token` - The main thread token for drawing.
    pub fn draw_ray(&mut self, token: &crate::sk::MainThreadToken) {
        use crate::{
            maths::lerp,
            system::{LinePoint, Lines},
            util::Time,
        };

        let interactor = &self.interactor;
        if !interactor.get_tracked().is_active() {
            return;
        }

        let _focused = interactor.get_focused() != 0;
        let active_elem = interactor.get_active() != 0;
        let dt = 16.0 * Time::get_step_unscaledf();

        // Animate visibility: visible when focused (or always for controllers — hide_inactive=false)
        self.ray_visible = lerp(self.ray_visible, 1.0_f32, dt);
        let visibility = self.ray_visible;
        if visibility < 0.001 {
            return;
        }

        // Animate active state
        self.ray_active = lerp(self.ray_active, if active_elem { 1.0 } else { 0.0 }, dt);
        let active_amt = self.ray_active;

        let motion_pos = interactor.get_motion().position;
        let capsule_end = interactor.get_end();
        let uncentered_dir = (capsule_end - motion_pos).get_normalized();

        // If focused, adjust ray toward the focused element
        let (mut length, centered_dir) =
            if let Some((pose_world, bounds_local, at_local)) = interactor.try_get_focus_bounds() {
                let pt = pose_world.to_matrix(None).transform_point(bounds_local.center + at_local);
                let l = (pt - motion_pos).magnitude();
                let d = (pt - motion_pos).get_normalized();
                (l, d)
            } else {
                (0.35_f32, uncentered_dir)
            };
        length = lerp(0.35, length, visibility);
        length = length.max(0.0);

        let alpha = 0.35 + active_amt * 0.65;

        const CT: usize = 20;
        const RAY_SNAP: f32 = 1.0;
        let mut pts = [LinePoint { pt: Vec3::ZERO, thickness: 0.0, color: Color32::WHITE }; CT];
        for (i, pt) in pts.iter_mut().enumerate() {
            let pct = i as f32 / (CT - 1) as f32;
            let blend = pct * pct * pct * RAY_SNAP;
            let d = pct * length;

            let pct_i = 1.0 - pct;
            let curve = lerp(
                (pct_i * pct_i * std::f32::consts::PI).sin(),
                ((pct * pct * std::f32::consts::PI).sin() * 1.5).min(1.0),
                active_amt,
            );
            let width = (0.002 + curve * 0.003) * visibility;
            let pos = motion_pos + Vec3::lerp(uncentered_dir * d, centered_dir * d, blend);
            let a = (curve * alpha * 255.0) as u8;
            *pt = LinePoint { pt: pos, thickness: width, color: Color32::rgba(255, 255, 255, a) };
        }
        Lines::add_list(token, &pts);
    }

    /// Returns a reference to the underlying [`Interactor`].
    pub fn interactor(&self) -> &Interactor {
        &self.interactor
    }

    /// Returns the hand this controller interactor is bound to.
    pub fn hand(&self) -> crate::system::Handed {
        self.hand
    }
}

/// A hand-based interactor that mirrors `interact_mode_hands*()` from the C++ StereoKit code.
///
/// This creates three interactors per hand:
/// - **poke**: a `Point` interactor with `Poke` event and `Position` activation, driven by the
///   index fingertip.
/// - **pinch**: a `Point` interactor with `Pinch` event and `State` activation, driven by the
///   thumb/index pinch point.
/// - **far**: a `Line` interactor with `Poke | Pinch` events and `State` activation, driven by
///   the hand's aim ray. Only active when far interaction is enabled.
///
/// see also [`Interactor`] [`crate::system::Hand`] [`crate::system::Input::hand`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::interactor::InteractorHand;
/// use stereokit_rust::system::Handed;
///
/// let mut hand_interactor = InteractorHand::new(Handed::Right);
///
/// assert_eq!(hand_interactor.hand(), Handed::Right);
///
/// test_steps!( // !!!! Get a proper main loop !!!!
///     hand_interactor.step();
///     hand_interactor.draw_ray(token);
/// );
///
/// let poke  = hand_interactor.poke();
/// assert_eq!(poke.get_radius(), 0.007);
/// let pinch = hand_interactor.pinch();
/// assert_eq!(pinch.get_radius(), 0.007);
/// let far   = hand_interactor.far();
/// assert_eq!(far.get_radius(), 0.01);
///
/// # sk::Sk::shutdown();
/// ```
pub struct InteractorHand {
    poke: Interactor,
    pinch: Interactor,
    far: Interactor,
    hand: crate::system::Handed,
    ray_visible: f32,
    ray_active: f32,
    prev_active: bool,
}

impl InteractorHand {
    /// Create a new hand interactor for the given hand.
    ///
    /// Mirrors `interact_mode_hands_start()`:
    /// - poke: `Point`, `Poke`, `Position`, input_source_id = 1000 + hand index, radius 0, 0 secondary dims.
    /// - pinch: `Point`, `Pinch`, `State`, input_source_id = 1000 + hand index, radius 0, 0 secondary dims.
    /// - far: `Line`, `Poke | Pinch`, `State`, input_source_id = 1000 + hand index, radius 0.01 m, 2 secondary dims.
    pub fn new(hand: crate::system::Handed) -> Self {
        let src_id = 1000 + hand as i32;
        let poke = Interactor::create(
            InteractorType::Point,
            InteractorEvent::Poke,
            InteractorActivation::Position,
            src_id,
            0.0,
            0,
        );
        let pinch = Interactor::create(
            InteractorType::Point,
            InteractorEvent::Pinch,
            InteractorActivation::State,
            src_id,
            0.0,
            0,
        );
        let far = Interactor::create(
            InteractorType::Line,
            InteractorEvent::Poke | InteractorEvent::Pinch,
            InteractorActivation::State,
            src_id,
            0.01,
            2,
        );
        Self { poke, pinch, far, hand, ray_visible: 0.0, ray_active: 0.0, prev_active: false }
    }

    /// Update the hand interactors with the current frame's hand data.
    ///
    /// Mirrors `interact_mode_hands_step()`:
    /// - poke: index fingertip position, with `Position`-based activation.
    /// - pinch: thumb/index pinch with `State`-based activation.
    /// - far: hand aim ray, gated by `aim_ready` and UI far-interact setting.
    pub fn step(&mut self) {
        use crate::{
            maths::lerp,
            system::{FingerId, Input, JointId},
        };
        let hand = Input::hand(self.hand);
        let head = Input::get_head();
        let index_tip = hand.get(FingerId::Index, JointId::Tip);
        let thumb_tip = hand.get(FingerId::Thumb, JointId::Tip);

        // --- Poke ---
        self.poke.radius(index_tip.radius);
        let poke_start = if hand.tracked.is_just_active() { index_tip.position } else { self.poke.get_end() };
        self.poke.update(
            poke_start,
            index_tip.position,
            Pose { position: index_tip.position, orientation: hand.palm.orientation },
            index_tip.position,
            Vec3::ZERO,
            BtnState::Inactive,
            hand.tracked,
        );

        // --- Pinch ---
        self.pinch.radius(index_tip.radius);
        self.pinch.update(
            thumb_tip.position,
            index_tip.position,
            Pose { position: hand.pinch_pt, orientation: hand.palm.orientation },
            hand.pinch_pt,
            Vec3::ZERO,
            hand.pinch,
            hand.tracked,
        );

        // --- Far (always, even when far interaction is disabled) ---
        let head_anchor = head.position + Vec3::new(0.0, -0.12, 0.0);
        let hand_dist = (hand.palm.position - head_anchor).magnitude();
        let is_pinched = hand.pinch.is_active();
        let just_pinched = hand.pinch.is_just_active();
        let aim_ready = hand.aim_ready.is_active();
        let is_active = (self.prev_active && is_pinched) || (aim_ready && just_pinched);
        let far_pinch_state = BtnState::make(self.prev_active, is_active);
        self.prev_active = is_active;

        let min_dist = lerp(0.25, 0.1, ((hand_dist - 0.1) / 0.4).clamp(0.0, 1.0));
        self.far.min_distance(min_dist);
        self.far.update(
            hand.aim.position,
            hand.aim.position + hand.aim.orientation * Vec3::FORWARD * 100.0,
            hand.aim,
            head_anchor,
            Vec3::ZERO,
            far_pinch_state,
            hand.aim_ready,
        );
    }

    /// Draw the hand aim ray indicator, mirroring `interactor_show_ray()` with `skip=0.07` and
    /// `hide_inactive=true`.
    ///
    /// The ray is only drawn when the far interactor has a focused element, and animates its
    /// visibility and thickness accordingly.
    ///
    /// * `token` - The main thread token for drawing.
    pub fn draw_ray(&mut self, token: &crate::sk::MainThreadToken) {
        use crate::{
            maths::lerp,
            system::{LinePoint, Lines},
            util::Time,
        };

        let interactor = &self.far;
        if !interactor.get_tracked().is_active() {
            return;
        }

        let focused = interactor.get_focused() != 0;
        let active_elem = interactor.get_active() != 0;
        let dt = 16.0 * Time::get_step_unscaledf();

        // hide_inactive=true: visible only when focused
        self.ray_visible = lerp(self.ray_visible, if focused { 1.0_f32 } else { 0.0_f32 }, dt);
        let visibility = self.ray_visible;
        if visibility < 0.001 {
            return;
        }

        self.ray_active = lerp(self.ray_active, if active_elem { 1.0 } else { 0.0 }, dt);
        let active_amt = self.ray_active;

        const SKIP: f32 = 0.07;
        let motion_pos = interactor.get_motion().position;
        let capsule_end = interactor.get_end();
        let uncentered_dir = (capsule_end - motion_pos).get_normalized();

        let (mut length, centered_dir) =
            if let Some((pose_world, bounds_local, at_local)) = interactor.try_get_focus_bounds() {
                let pt = pose_world.to_matrix(None).transform_point(bounds_local.center + at_local);
                let l = (pt - motion_pos).magnitude();
                let d = (pt - motion_pos).get_normalized();
                (l, d)
            } else {
                (0.35_f32, uncentered_dir)
            };
        length = lerp(0.35, length, visibility);
        length = (length - SKIP).max(0.0);

        // hide_inactive=true: alpha is multiplied by visibility
        let alpha = (0.35 + active_amt * 0.65) * visibility;

        const CT: usize = 20;
        const RAY_SNAP: f32 = 1.0;
        let mut pts = [LinePoint { pt: Vec3::ZERO, thickness: 0.0, color: Color32::WHITE }; CT];
        for (i, pt) in pts.iter_mut().enumerate() {
            let pct = i as f32 / (CT - 1) as f32;
            let blend = pct * pct * pct * RAY_SNAP;
            let d = SKIP + pct * length;

            let pct_i = 1.0 - pct;
            let curve = lerp(
                (pct_i * pct_i * std::f32::consts::PI).sin(),
                ((pct * pct * std::f32::consts::PI).sin() * 1.5).min(1.0),
                active_amt,
            );
            let width = (0.002 + curve * 0.003) * visibility;
            let pos = motion_pos + Vec3::lerp(uncentered_dir * d, centered_dir * d, blend);
            let a = (curve * alpha * 255.0) as u8;
            *pt = LinePoint { pt: pos, thickness: width, color: Color32::rgba(255, 255, 255, a) };
        }
        Lines::add_list(token, &pts);
    }

    /// Returns a reference to the poke [`Interactor`].
    pub fn poke(&self) -> &Interactor {
        &self.poke
    }

    /// Returns a reference to the pinch [`Interactor`].
    pub fn pinch(&self) -> &Interactor {
        &self.pinch
    }

    /// Returns a reference to the far [`Interactor`].
    pub fn far(&self) -> &Interactor {
        &self.far
    }

    /// Returns the hand this interactor is bound to.
    pub fn hand(&self) -> crate::system::Handed {
        self.hand
    }
}
