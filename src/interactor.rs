use crate::{
    maths::{Bool32T, Bounds, Pose, Vec3},
    system::BtnState,
    ui::IdHashT,
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

/// TODO: is this redundant with interactor_type_? This describes how an interactor activates elements. Does it use the
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
/// see also [`Interactor`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DefaultInteractors {
    /// StereoKit's default interactors, this provides an aim ray for a mouse, aim rays for controllers, and aim, pinch,
    /// and poke interactors for hands.
    Default = 0,
    /// Don't provide any interactors at all. This means you either don't want interaction, or are providing your own
    /// custom interactors.
    None = 1,
}

#[link(name = "StereoKitC")]
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
///     Vec3::new(0.0, 0.0, 0.0),
///     Vec3::new(0.0, 0.0, 0.1),
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
/// interactor.destroy();
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Interactor {
    inst: i32,
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

    /// Destroy this interactor and free its resources.
    /// <https://stereokit.net/Pages/StereoKit/Interactor/Destroy.html>
    ///
    /// see also [`interactor_destroy`]
    pub fn destroy(&self) {
        unsafe {
            interactor_destroy(self.inst);
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
    pub fn get(index: i32) -> Self {
        let inst = unsafe { interactor_get(index) };
        Self { inst }
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
///
/// // Disable the default drawing of interactor indicators
/// Interaction::set_default_draw(false);
///
/// // Check if default drawing is enabled
/// let draw_enabled = Interaction::get_default_draw();
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
