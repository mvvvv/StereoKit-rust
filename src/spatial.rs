use crate::{
    maths::{Bool32T, Pose, Vec2, Vec3},
    mesh::{Mesh, MeshT},
    system::BtnState,
};
use std::{
    ffi::{CStr, CString, c_char},
    fmt,
    mem::MaybeUninit,
    ptr::{null, null_mut},
};

/// StereoKit ffi type.
pub type SpatialEntityT = u64;

/// A 128 bit universally unique identifier, with bytes in the standard RFC 4122 order, so they read the same as the
/// usual xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx text form. All zeros is the nil UUID, which never identifies anything.
/// Native `sk_uuid_t`, spatial entities hand these out via [`SpatialEntity::try_get_uuid`] once they're persisted.
/// ### Examples
/// ```
/// use stereokit_rust::spatial::Uuid;
///
/// let id = Uuid::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
/// assert!(!id.is_nil());
/// assert_eq!(format!("{id}"), "01020304-0506-0708-090a-0b0c0d0e0f10");
///
/// // All zeros is the nil UUID, which never identifies anything.
/// assert!(Uuid::default().is_nil());
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Uuid {
    bytes: [u8; 16],
}

impl Uuid {
    /// Creates a UUID from its 16 bytes, in standard RFC 4122 order.
    pub fn from_bytes(bytes: [u8; 16]) -> Uuid {
        Uuid { bytes }
    }

    /// The UUID's 16 bytes, in standard RFC 4122 order.
    pub fn bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// Is this the nil UUID? All zeros, which never identifies anything.
    pub fn is_nil(&self) -> bool {
        self.bytes == [0; 16]
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, byte) in self.bytes.iter().enumerate() {
            if matches!(i, 4 | 6 | 8 | 10) {
                write!(f, "-")?;
            }
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

bitflags::bitflags! {
    /// A spatial capability is a unit of scene understanding functionality that a device may provide, such as plane
    /// tracking, or QR code tracking. Check what the device supports with [`Spatial::capabilities`], request what you
    /// need, and StereoKit will maintain a list of the spatial entities the system discovers.
    ///
    /// The top 4 bits of this flag are reserved for vendor and experimental capabilities.
    /// <https://stereokit.net/Pages/StereoKit/SpatialCapability.html>
    ///
    /// see also [`Spatial`] [`SpatialEntity`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::spatial::SpatialCapability;
    ///
    /// let caps = SpatialCapability::Anchor | SpatialCapability::PlaneTracking;
    /// assert!(caps.contains(SpatialCapability::Anchor));
    /// assert!(!caps.contains(SpatialCapability::QrCode));
    /// assert!(SpatialCapability::None.is_empty());
    /// ```
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct SpatialCapability : u32 {
        /// No spatial capabilities.
        const None = 0;
        /// Spatial anchors, poses the system keeps as stable as it can relative to the physical world. This allows
        /// creating new anchor entities via [`SpatialEntity::create_anchor`].
        const Anchor = 1 << 0;
        /// Detection and tracking of flat surfaces in the environment, like floors, walls, and tables.
        const PlaneTracking = 1 << 1;
        /// Detection and tracking of QR codes, including their decoded data.
        const QrCode = 1 << 2;
        /// Detection and tracking of Micro QR codes, including their decoded data.
        const MicroQr = 1 << 3;
        /// Detection and tracking of ArUco fiducial markers.
        const Aruco = 1 << 4;
        /// Detection and tracking of AprilTag fiducial markers.
        const AprilTag = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Spatial entities are composed of components, where each component is a chunk of data or behavior the entity
    /// provides. This flag describes a set of components, and each component has a matching accessor on the entity.
    ///
    /// The top 4 bits of this flag are reserved for vendor and experimental components.
    /// <https://stereokit.net/Pages/StereoKit/SpatialComponent.html>
    ///
    /// see also [`SpatialEntity`] [`Spatial::components_for`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::spatial::SpatialComponent;
    ///
    /// let comps = SpatialComponent::Bounds2D | SpatialComponent::Label;
    /// assert!(comps.contains(SpatialComponent::Bounds2D));
    /// assert!(!comps.contains(SpatialComponent::Mesh));
    /// ```
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct SpatialComponent : u32 {
        /// No components.
        const None = 0;
        /// A center pose and XY size describing a 2D rectangle, such as the extents of a detected plane, or the shape
        /// of a marker. The pose faces out of the surface, so Forward (-Z) is the surface normal, matching how quads
        /// and text face in StereoKit.
        const Bounds2D = 1 << 0;
        /// A center pose and XYZ size describing an oriented bounding volume. When the entity has a front, like a
        /// screen or table top, Forward (-Z) is the direction it faces.
        const Bounds3D = 1 << 1;
        /// A reference to a parent spatial entity this entity is attached to.
        const Parent = 1 << 2;
        /// A 3D triangle mesh representing the entity's shape.
        const Mesh = 1 << 3;
        /// A pose the system actively keeps stable relative to the physical world.
        const Anchor = 1 << 4;
        /// A durable identity that allows the entity to be recognized across sessions and reboots. When
        /// [`Spatial::components_for`] lists this for a capability, the app can persist that capability's entities.
        const Persistence = 1 << 5;
        /// The general orientation category of a detected plane, see [`PlaneAlign`].
        const PlaneAlignment = 1 << 6;
        /// A 2D triangle mesh of the entity's surface, on the XY plane of its bounds2d pose.
        const Mesh2D = 1 << 7;
        /// A 2D boundary polygon outlining the entity's surface. In C, the vertex pointer from
        /// [`spatial_entity_get_polygon`] is only valid until the next frame, so copy it to keep it longer.
        const Polygon = 1 << 8;
        /// A semantic category for the entity, like floor or table, see [`SpatialLabel`].
        const Label = 1 << 9;
        /// Marker information: the marker's type, numeric id, and any decoded data. In C, the text and data pointers
        /// are only valid until the next frame, so copy them to keep them longer.
        const Marker = 1 << 10;
    }
}

/// Whether the things you've asked of a spatial entity have gone through, like creating an anchor or persisting it.
/// Failure states are negative and healthy ones positive, so `status < 0` catches every failure. This is separate from
/// tracking, so an entity can be tracked and usable while a persist is still [`SpatialStatus::Pending`]. Entities the
/// system discovers on its own, like planes, are [`SpatialStatus::Ready`] unless you ask something of them.
/// <https://stereokit.net/Pages/StereoKit/SpatialStatus.html>
///
/// see also [`SpatialEntity::get_status`]
/// ### Examples
/// ```
/// use stereokit_rust::spatial::SpatialStatus;
///
/// // Failure states are negative and healthy ones positive.
/// assert!((SpatialStatus::Failed as i32) < 0);
/// assert!((SpatialStatus::Partial as i32) < 0);
/// assert!((SpatialStatus::Ready as i32) > 0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum SpatialStatus {
    /// The entity couldn't be created, or storage doesn't have the id it was looked up by. It shows up in the removed
    /// list with this status for its final frame.
    Failed = -2,
    /// The entity exists, but at least one thing you asked of it failed, like a persist. Check its components to see
    /// what's missing, and making a new request clears this.
    Partial = -1,
    /// Not a valid entity.
    None = 0,
    /// Something you asked of this entity is still in progress, like creation, loading it by persist id, or a persist
    /// that's waiting on the system.
    Pending = 1,
    /// Nothing you've asked of this entity is still in progress, and nothing failed.
    Ready = 2,
}

/// The general orientation of a detected plane.
/// <https://stereokit.net/Pages/StereoKit/PlaneAlign.html>
///
/// see also [`SpatialEntity::try_get_plane_align`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum PlaneAlign {
    /// Alignment is not known.
    None = 0,
    /// A horizontal surface facing up, like a floor or table top.
    HorizontalUp = 1,
    /// A horizontal surface facing down, like a ceiling.
    HorizontalDown = 2,
    /// A vertical surface, like a wall.
    Vertical = 3,
    /// A surface at some other arbitrary angle, like a ramp.
    Arbitrary = 4,
}

/// A semantic category the system has assigned to a spatial entity, such as a floor plane or a tracked keyboard. All
/// label sources share this one list, so a category can come from plane tracking on one device and object tracking on
/// another. Any given entity only uses a subset of these, and if the system reports a category StereoKit doesn't know
/// yet, it arrives as [`SpatialLabel::Uncategorized`]. New values are only ever appended.
/// <https://stereokit.net/Pages/StereoKit/SpatialLabel.html>
///
/// see also [`SpatialEntity::try_get_label`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum SpatialLabel {
    /// No label information available.
    None = 0,
    /// The system recognizes this entity, but it doesn't fit any of the categories it knows.
    Uncategorized = 1,
    /// A floor.
    Floor = 2,
    /// A wall.
    Wall = 3,
    /// A ceiling.
    Ceiling = 4,
    /// A table, or table-like surface.
    Table = 5,
}

/// The type of a detected marker.
/// <https://stereokit.net/Pages/StereoKit/MarkerType.html>
///
/// see also [`SpatialEntity::try_get_marker`] [`Spatial::set_marker_size`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MarkerType {
    /// Not a marker.
    None = 0,
    /// A QR code, data is typically a decoded string.
    QrCode = 1,
    /// A Micro QR code, data is typically a decoded string.
    MicroQr = 2,
    /// An ArUco fiducial marker, identified by its numeric id.
    Aruco = 3,
    /// An AprilTag fiducial marker, identified by its numeric id.
    AprilTag = 4,
}

/// Predefined ArUco marker dictionaries. A dictionary describes the grid size of the markers, and how many unique
/// marker ids it contains. The tracker can only detect markers from the dictionary it's configured for.
/// <https://stereokit.net/Pages/StereoKit/ArucoDict.html>
///
/// see also [`Spatial::set_aruco_dictionary`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum ArucoDict {
    /// Let StereoKit pick, currently 4x4, 50 ids.
    Default = 0,
    /// 4x4 grid, 50 unique ids.
    Dict4x4_50 = 1,
    /// 4x4 grid, 100 unique ids.
    Dict4x4_100 = 2,
    /// 4x4 grid, 250 unique ids.
    Dict4x4_250 = 3,
    /// 4x4 grid, 1000 unique ids.
    Dict4x4_1000 = 4,
    /// 5x5 grid, 50 unique ids.
    Dict5x5_50 = 5,
    /// 5x5 grid, 100 unique ids.
    Dict5x5_100 = 6,
    /// 5x5 grid, 250 unique ids.
    Dict5x5_250 = 7,
    /// 5x5 grid, 1000 unique ids.
    Dict5x5_1000 = 8,
    /// 6x6 grid, 50 unique ids.
    Dict6x6_50 = 9,
    /// 6x6 grid, 100 unique ids.
    Dict6x6_100 = 10,
    /// 6x6 grid, 250 unique ids.
    Dict6x6_250 = 11,
    /// 6x6 grid, 1000 unique ids.
    Dict6x6_1000 = 12,
    /// 7x7 grid, 50 unique ids.
    Dict7x7_50 = 13,
    /// 7x7 grid, 100 unique ids.
    Dict7x7_100 = 14,
    /// 7x7 grid, 250 unique ids.
    Dict7x7_250 = 15,
    /// 7x7 grid, 1000 unique ids.
    Dict7x7_1000 = 16,
}

/// Predefined AprilTag marker dictionaries. The name describes the tag family: grid bits, and the minimum hamming
/// distance between ids.
/// <https://stereokit.net/Pages/StereoKit/AprilTagDict.html>
///
/// see also [`Spatial::set_april_tag_dictionary`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum AprilTagDict {
    /// Let StereoKit pick, currently 36h11.
    Default = 0,
    /// 4x4 bits, hamming distance 5, 30 ids.
    Tag16h5 = 1,
    /// 5x5 bits, hamming distance 9, 35 ids.
    Tag25h9 = 2,
    /// 6x6 bits, hamming distance 10, 2320 ids.
    Tag36h10 = 3,
    /// 6x6 bits, hamming distance 11, 587 ids. The most common choice.
    Tag36h11 = 4,
}
/// Spatial is where you choose which kinds of scene understanding the device should run, like plane tracking, QR codes,
/// or anchors, and how they're configured. Each of these is a [`SpatialCapability`], and the things they discover show
/// up as [`SpatialEntity`] objects.
///
/// Check [`Spatial::capabilities`] to see what the current device supports, then [`Spatial::request`] what you need.
/// Capabilities start up asynchronously, and may need a permission first, so [`Spatial::running`] tells you which ones
/// have actually started. Marker settings like [`Spatial::set_marker_size`] and [`Spatial::get_aruco_dictionary`] are
/// best set before requesting, since changing them restarts that capability's tracking.
/// <https://stereokit.net/Pages/StereoKit/Spatial.html>
///
/// see also [`SpatialEntity`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::spatial::{MarkerType, Spatial, SpatialCapability};
///
/// // Check what the device supports, then request what you need!
/// if Spatial::is_supported(SpatialCapability::PlaneTracking) {
///     Spatial::request(SpatialCapability::PlaneTracking);
/// }
/// // Marker settings are best set before requesting, since changing them
/// // restarts that capability's tracking.
/// Spatial::set_marker_size(MarkerType::Aruco, 0.16);
/// Spatial::request(SpatialCapability::Aruco);
/// # test_steps!();
/// # sk::Sk::shutdown();
/// ```
pub struct Spatial;

unsafe extern "C" {
    pub fn spatial_capabilities() -> SpatialCapability;
    pub fn spatial_capability_components(capability: SpatialCapability) -> SpatialComponent;
    pub fn spatial_request(capabilities: SpatialCapability);
    pub fn spatial_disable(capabilities: SpatialCapability);
    pub fn spatial_get_requested() -> SpatialCapability;
    pub fn spatial_get_running() -> SpatialCapability;

    pub fn spatial_set_marker_size(marker_type: MarkerType, size_meters: f32);
    pub fn spatial_get_marker_size(marker_type: MarkerType) -> f32;
    pub fn spatial_set_marker_stationary(marker_type: MarkerType, stationary: Bool32T);
    pub fn spatial_get_marker_stationary(marker_type: MarkerType) -> Bool32T;
    pub fn spatial_set_aruco_dictionary(dictionary: ArucoDict);
    pub fn spatial_get_aruco_dictionary() -> ArucoDict;
    pub fn spatial_set_april_tag_dictionary(dictionary: AprilTagDict);
    pub fn spatial_get_april_tag_dictionary() -> AprilTagDict;

    pub fn spatial_entity_get_count(with_components: SpatialComponent) -> i32;
    pub fn spatial_entity_get_index(with_components: SpatialComponent, index: i32) -> SpatialEntityT;
    pub fn spatial_entity_get_new_count(with_components: SpatialComponent) -> i32;
    pub fn spatial_entity_get_new_index(with_components: SpatialComponent, index: i32) -> SpatialEntityT;
    pub fn spatial_entity_get_removed_count(with_components: SpatialComponent) -> i32;
    pub fn spatial_entity_get_removed_index(with_components: SpatialComponent, index: i32) -> SpatialEntityT;
    pub fn spatial_entity_find_anchor(name_utf8: *const c_char) -> SpatialEntityT;
    pub fn spatial_entity_find_anchor_uuid(uuid: Uuid) -> SpatialEntityT;
    pub fn spatial_entity_is_valid(entity: SpatialEntityT) -> Bool32T;
    pub fn spatial_entity_get_tracked(entity: SpatialEntityT) -> BtnState;
    pub fn spatial_entity_get_status(entity: SpatialEntityT) -> SpatialStatus;
    pub fn spatial_entity_get_components(entity: SpatialEntityT) -> SpatialComponent;
    pub fn spatial_entity_get_changed(entity: SpatialEntityT) -> SpatialComponent;
    pub fn spatial_entity_get_parent(entity: SpatialEntityT) -> SpatialEntityT;

    pub fn spatial_entity_get_pose(entity: SpatialEntityT) -> Pose;
    pub fn spatial_entity_get_bounds2d(entity: SpatialEntityT, out_center: *mut Pose, out_size: *mut Vec2) -> Bool32T;
    pub fn spatial_entity_get_bounds3d(entity: SpatialEntityT, out_center: *mut Pose, out_size: *mut Vec3) -> Bool32T;
    pub fn spatial_entity_get_plane_align(entity: SpatialEntityT, out_alignment: *mut PlaneAlign) -> Bool32T;
    pub fn spatial_entity_get_label(entity: SpatialEntityT, out_label: *mut SpatialLabel) -> Bool32T;
    pub fn spatial_entity_get_mesh(entity: SpatialEntityT, mesh: MeshT, out_origin: *mut Pose) -> Bool32T;
    pub fn spatial_entity_get_mesh2d(entity: SpatialEntityT, mesh: MeshT, out_origin: *mut Pose) -> Bool32T;
    // Hands back a pointer to registry-owned vertices, not a caller array
    pub fn spatial_entity_get_polygon(
        entity: SpatialEntityT,
        out_origin: *mut Pose,
        out_verts: *mut *const Vec2,
        out_count: *mut i32,
    ) -> Bool32T;
    pub fn spatial_entity_get_marker(
        entity: SpatialEntityT,
        out_type: *mut MarkerType,
        out_marker_id: *mut u32,
    ) -> Bool32T;
    pub fn spatial_entity_get_marker_text(entity: SpatialEntityT) -> *const c_char;
    pub fn spatial_entity_get_marker_data(entity: SpatialEntityT, out_size: *mut i32) -> *const u8;

    pub fn spatial_entity_create_anchor(
        pose: Pose,
        opt_name_utf8: *const c_char,
        parent: SpatialEntityT,
    ) -> SpatialEntityT;
    pub fn spatial_entity_destroy(entity: SpatialEntityT) -> Bool32T;

    pub fn spatial_entity_get_uuid(entity: SpatialEntityT, out_uuid: *mut Uuid) -> Bool32T;
    pub fn spatial_entity_get_name(entity: SpatialEntityT) -> *const c_char;
    pub fn spatial_entity_persist(entity: SpatialEntityT) -> Bool32T;
    pub fn spatial_entity_unpersist(entity: SpatialEntityT) -> Bool32T;
    pub fn spatial_entity_unpersist_uuid(uuid: Uuid) -> Bool32T;
}

impl Spatial {
    /// The spatial capabilities the current device supports! This is [`SpatialCapability::None`] until an XR session
    /// with spatial entity support has initialized.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/Capabilities.html>
    ///
    /// see also [`spatial_capabilities`] [`Spatial::is_supported`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// let caps = Spatial::capabilities();
    /// println!("Spatial capabilities: {caps:?}");
    /// // On simulator and PC OpenXR:
    /// assert_eq!(caps, SpatialCapability::None);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn capabilities() -> SpatialCapability {
        unsafe { spatial_capabilities() }
    }

    /// The capabilities that have been requested, via [`Spatial::request`] or by StereoKit systems like
    /// [`Anchor`](crate::anchor::Anchor), minus any you've turned off with [`Spatial::disable`].
    /// <https://stereokit.net/Pages/StereoKit/Spatial/Requested.html>
    ///
    /// see also [`spatial_get_requested`] [`Spatial::is_requested`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// Spatial::request(SpatialCapability::QrCode);
    /// assert_eq!(Spatial::requested() & SpatialCapability::QrCode, SpatialCapability::QrCode);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn requested() -> SpatialCapability {
        unsafe { spatial_get_requested() }
    }

    /// The capabilities that have started up and are providing entities. This is a subset of
    /// [`Spatial::requested`], since capabilities take a little time to start after being requested.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/Running.html>
    ///
    /// see also [`spatial_get_running`] [`Spatial::is_running`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::Spatial;
    ///
    /// let running = Spatial::running();
    /// println!("Running spatial capabilities: {running:?}");
    /// // Running is always a subset of what's been requested.
    /// assert!((running & !Spatial::requested()).is_empty());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn running() -> SpatialCapability {
        unsafe { spatial_get_running() }
    }

    /// The components the device can provide on entities discovered by the given capability.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/ComponentsFor.html>
    /// * `capability` - A single capability to look up.
    ///
    /// Returns all components the device supports for that capability.
    ///
    /// see also [`spatial_capability_components`] [`Spatial::is_component_supported`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability, SpatialComponent};
    ///
    /// let components = Spatial::components_for(SpatialCapability::Anchor);
    /// println!("Anchor components: {components:?}");
    /// // Anchors always at least provide an anchor component, when the device supports them.
    /// assert!(components.is_empty() || components.contains(SpatialComponent::Anchor));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn components_for(capability: SpatialCapability) -> SpatialComponent {
        unsafe { spatial_capability_components(capability) }
    }

    /// Does the current device support all of these capabilities? This is false until an XR session with spatial entity
    /// support has initialized.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/IsSupported.html>
    /// * `capabilities` - One or more capabilities to check.
    ///
    /// Returns true if every capability given is supported.
    ///
    /// see also [`Spatial::capabilities`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// let supported = Spatial::is_supported(SpatialCapability::PlaneTracking);
    /// println!("Plane tracking supported: {supported}");
    /// assert!(!Spatial::is_supported(SpatialCapability::None));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_supported(capabilities: SpatialCapability) -> bool {
        !capabilities.is_empty() && (Spatial::capabilities() & capabilities) == capabilities
    }

    /// Can this capability provide all of these components on this device? Handy for optional data like labels, which
    /// some devices don't provide.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/IsSupported.html>
    /// * `capability` - A single capability to look up.
    /// * `components` - One or more components to check.
    ///
    /// Returns true if the capability can provide every component given.
    ///
    /// see also [`Spatial::components_for`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability, SpatialComponent};
    ///
    /// if Spatial::is_supported(SpatialCapability::PlaneTracking) {
    ///     let labels = Spatial::is_component_supported(SpatialCapability::PlaneTracking,
    ///                               SpatialComponent::Label);
    ///     println!("Planes come with labels: {labels}");
    /// }
    /// assert!(!Spatial::is_component_supported(SpatialCapability::PlaneTracking,
    ///                       SpatialComponent::None));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_component_supported(capability: SpatialCapability, components: SpatialComponent) -> bool {
        !components.is_empty() && (Spatial::components_for(capability) & components) == components
    }

    /// Have all of these capabilities been requested, and not turned off with [`Spatial::disable`]? This doesn't mean
    /// they've started yet, see [`Spatial::is_running`] for that.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/IsRequested.html>
    /// * `capabilities` - One or more capabilities to check.
    ///
    /// Returns true if every capability given is requested.
    ///
    /// see also [`Spatial::requested`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// Spatial::request(SpatialCapability::QrCode | SpatialCapability::Aruco);
    /// assert!(Spatial::is_requested(SpatialCapability::QrCode));
    /// assert!(Spatial::is_requested(SpatialCapability::QrCode | SpatialCapability::Aruco));
    /// assert!(!Spatial::is_requested(SpatialCapability::AprilTag));
    /// assert!(!Spatial::is_requested(SpatialCapability::None));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_requested(capabilities: SpatialCapability) -> bool {
        !capabilities.is_empty() && (Spatial::requested() & capabilities) == capabilities
    }

    /// Have all of these capabilities started up, and are they providing entities? Capabilities take a little time to
    /// start after being requested, and may wait on a permission first.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/IsRunning.html>
    /// * `capabilities` - One or more capabilities to check.
    ///
    /// Returns true if every capability given is running.
    ///
    /// see also [`Spatial::running`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// Spatial::request(SpatialCapability::QrCode);
    /// let running = Spatial::is_running(SpatialCapability::QrCode);
    /// println!("QR codes running: {running}");
    /// assert!(!Spatial::is_running(SpatialCapability::None));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_running(capabilities: SpatialCapability) -> bool {
        !capabilities.is_empty() && (Spatial::running() & capabilities) == capabilities
    }

    /// Request tracking for these capabilities, additively! This takes effect asynchronously, and entities will appear
    /// in the entity list as the system starts up and discovers them. Requesting or disabling one capability never
    /// disturbs entities belonging to another. If a capability needs a permission, this requests it automatically as a
    /// fallback, but requesting it yourself in advance via
    /// [`Permission::request`](crate::permission::Permission::request) gives you control over when the user is asked,
    /// and lets you handle a denial. You can call this before [`SkSettings::init`](crate::sk::SkSettings::init), and it
    /// takes effect once StereoKit starts.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/Request.html>
    /// * `capabilities` - One or more capabilities to request. Unsupported capabilities never start running.
    ///
    /// see also [`spatial_request`] [`Spatial::disable`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// // Requesting is additive, and asynchronous.
    /// Spatial::request(SpatialCapability::QrCode);
    /// Spatial::request(SpatialCapability::Aruco);
    /// assert!(Spatial::is_requested(SpatialCapability::QrCode | SpatialCapability::Aruco));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn request(capabilities: SpatialCapability) {
        unsafe { spatial_request(capabilities) }
    }

    /// Stop tracking these capabilities. Their entities leave the entity list, and any SpatialEntity identifiers you
    /// still hold stop resolving. Persisted entities are the exception, they wait with [`SpatialStatus::Pending`], and
    /// come back if the capability is requested again. This also overrides StereoKit's own use of a capability. The
    /// [`Anchor`](crate::anchor::Anchor) system turns on [`SpatialCapability::Anchor`] the first time you use it, and
    /// calling this before [`SkSettings::init`](crate::sk::SkSettings::init) keeps [`Anchor`](crate::anchor::Anchor)
    /// from using spatial entities at all.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/Disable.html>
    /// * `capabilities` - One or more capabilities to disable.
    ///
    /// see also [`spatial_disable`] [`Spatial::request`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{Spatial, SpatialCapability};
    ///
    /// Spatial::request(SpatialCapability::QrCode);
    /// assert!(Spatial::is_requested(SpatialCapability::QrCode));
    /// Spatial::disable(SpatialCapability::QrCode);
    /// assert!(!Spatial::is_requested(SpatialCapability::QrCode));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn disable(capabilities: SpatialCapability) {
        unsafe { spatial_disable(capabilities) }
    }

    /// Tell the system how big your printed markers of this type are. This matters most for ArUco and AprilTags, where
    /// a known size can help a runtime judge marker distance, but it's only a hint, and runtimes may ignore it.
    /// Changing this while that marker type is being tracked restarts its tracking, so set it before
    /// [`Spatial::request`] when you can.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/SetMarkerSize.html>
    /// * `marker_type` - The marker type this size applies to.
    /// * `size_meters` - The edge length of the marker's square, in meters. Use 0 if the sizes are mixed or unknown.
    ///
    /// see also [`spatial_set_marker_size`] [`Spatial::get_marker_size`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{MarkerType, Spatial};
    ///
    /// Spatial::set_marker_size(MarkerType::Aruco, 0.2);
    /// assert_eq!(Spatial::get_marker_size(MarkerType::Aruco), 0.2);
    /// // 0 means the sizes are mixed or unknown.
    /// assert_eq!(Spatial::get_marker_size(MarkerType::AprilTag), 0.0);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_marker_size(marker_type: MarkerType, size_meters: f32) {
        unsafe { spatial_set_marker_size(marker_type, size_meters) }
    }

    /// The physical marker size set via [`Spatial::set_marker_size`], 0 if unknown.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/GetMarkerSize.html>
    /// * `marker_type` - The marker type to look up.
    ///
    /// Returns the marker's edge length in meters.
    ///
    /// see also [`spatial_get_marker_size`]
    /// see example in [`Spatial::set_marker_size`]
    pub fn get_marker_size(marker_type: MarkerType) -> f32 {
        unsafe { spatial_get_marker_size(marker_type) }
    }

    /// Tell the system whether markers of this type stay put, like a code taped to a wall, rather than being carried
    /// around. Stationary markers can have their pose refined over time instead of re-detected every frame. Changing
    /// this while that marker type is being tracked restarts its tracking, so set it before [`Spatial::request`] when
    /// you can. Not all devices use this, treat it as a hint.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/SetMarkerStationary.html>
    /// * `marker_type` - The marker type this applies to.
    /// * `stationary` - True if these markers never move.
    ///
    /// see also [`spatial_set_marker_stationary`] [`Spatial::get_marker_stationary`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{MarkerType, Spatial};
    ///
    /// assert!(!Spatial::get_marker_stationary(MarkerType::QrCode));
    /// Spatial::set_marker_stationary(MarkerType::QrCode, true);
    /// assert!(Spatial::get_marker_stationary(MarkerType::QrCode));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_marker_stationary(marker_type: MarkerType, stationary: bool) {
        unsafe { spatial_set_marker_stationary(marker_type, stationary as Bool32T) }
    }

    /// Whether this marker type was marked as stationary via [`Spatial::set_marker_stationary`].
    /// <https://stereokit.net/Pages/StereoKit/Spatial/GetMarkerStationary.html>
    /// * `marker_type` - The marker type to look up.
    ///
    /// Returns true if these markers are expected to stay put.
    ///
    /// see also [`spatial_get_marker_stationary`]
    /// see example in [`Spatial::set_marker_stationary`]
    pub fn get_marker_stationary(marker_type: MarkerType) -> bool {
        unsafe { spatial_get_marker_stationary(marker_type) != 0 }
    }

    /// Which family of ArUco markers [`SpatialCapability::Aruco`] looks for. Markers from other dictionaries aren't
    /// detected, so this must match the markers you printed! [`ArucoDict::Default`] lets StereoKit pick. Changing it
    /// while ArUco markers are being tracked restarts that tracking.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/ArucoDictionary.html>
    /// * `dictionary` - The marker dictionary to look for.
    ///
    /// see also [`spatial_set_aruco_dictionary`] [`Spatial::get_aruco_dictionary`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{ArucoDict, Spatial};
    ///
    /// assert_eq!(Spatial::get_aruco_dictionary(), ArucoDict::Default);
    /// Spatial::set_aruco_dictionary(ArucoDict::Dict5x5_100);
    /// assert_eq!(Spatial::get_aruco_dictionary(), ArucoDict::Dict5x5_100);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_aruco_dictionary(dictionary: ArucoDict) {
        unsafe { spatial_set_aruco_dictionary(dictionary) }
    }

    /// Which family of ArUco markers [`SpatialCapability::Aruco`] looks for, see [`Spatial::set_aruco_dictionary`].
    /// <https://stereokit.net/Pages/StereoKit/Spatial/ArucoDictionary.html>
    ///
    /// Returns the current ArUco marker dictionary.
    ///
    /// see also [`spatial_get_aruco_dictionary`]
    /// see example in [`Spatial::set_aruco_dictionary`]
    pub fn get_aruco_dictionary() -> ArucoDict {
        unsafe { spatial_get_aruco_dictionary() }
    }

    /// Which family of AprilTag markers [`SpatialCapability::AprilTag`] looks for. Markers from other dictionaries
    /// aren't detected, so this must match the markers you printed! [`AprilTagDict::Default`] lets StereoKit pick.
    /// Changing it while AprilTags are being tracked restarts that tracking.
    /// <https://stereokit.net/Pages/StereoKit/Spatial/AprilTagDictionary.html>
    /// * `dictionary` - The marker dictionary to look for.
    ///
    /// see also [`spatial_set_april_tag_dictionary`] [`Spatial::get_april_tag_dictionary`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{AprilTagDict, Spatial};
    ///
    /// assert_eq!(Spatial::get_april_tag_dictionary(), AprilTagDict::Default);
    /// Spatial::set_april_tag_dictionary(AprilTagDict::Tag36h10);
    /// assert_eq!(Spatial::get_april_tag_dictionary(), AprilTagDict::Tag36h10);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_april_tag_dictionary(dictionary: AprilTagDict) {
        unsafe { spatial_set_april_tag_dictionary(dictionary) }
    }

    /// Which family of AprilTag markers [`SpatialCapability::AprilTag`] looks for, see
    /// [`Spatial::set_april_tag_dictionary`].
    /// <https://stereokit.net/Pages/StereoKit/Spatial/AprilTagDictionary.html>
    ///
    /// Returns the current AprilTag marker dictionary.
    ///
    /// see also [`spatial_get_april_tag_dictionary`]
    /// see example in [`Spatial::set_april_tag_dictionary`]
    pub fn get_april_tag_dictionary() -> AprilTagDict {
        unsafe { spatial_get_april_tag_dictionary() }
    }
}

/// A SpatialEntity is something the device has discovered or is tracking in the user's physical environment: a wall, a
/// table, a QR code, or an anchor the app has placed. This is StereoKit's surface for OpenXR's spatial entity
/// extensions.
///
/// Entities are composed of components, which are chunks of data like a bounding rectangle, a semantic label, or a
/// mesh. Which components an entity has depends on the capability that discovered it and what the device supports, so
/// data is accessed through the `try_get_*` methods, and entities can be filtered by the components you need via
/// [`SpatialEntity::with`].
///
/// Request the capabilities you're interested in with [`Spatial::request`], and StereoKit will keep an up-to-date list
/// of entities that you can poll each frame. Component data reflects the entity's last known state, so check
/// [`SpatialEntity::get_tracked`] to know if it's currently live.
///
/// SpatialEntity is a lightweight identifier, not a reference. The device owns these entities and controls their
/// lifetimes. Identifiers are never reused within a session, so a stale one simply stops resolving once its entity is
/// gone. [`SpatialEntity::is_valid`] becomes false, and accessors return no data.
/// <https://stereokit.net/Pages/StereoKit/SpatialEntity.html>
///
/// see also [`Spatial`] [`SpatialEntityIter`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::spatial::{Spatial, SpatialCapability, SpatialEntity};
///
/// // Ask for what you need, entities show up as the system discovers them.
/// Spatial::request(SpatialCapability::PlaneTracking);
/// for entity in SpatialEntity::all() {
///     println!("Entity {entity:?}, status {:?}", entity.get_status());
/// }
/// # test_steps!();
/// # sk::Sk::shutdown();
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct SpatialEntity(pub SpatialEntityT);

impl SpatialEntity {
    /// Does this identifier currently resolve to an entity? This becomes false once the entity permanently leaves the
    /// entity list, and a [`SpatialEntity::default`] is never valid.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Valid.html>
    ///
    /// see also [`spatial_entity_is_valid`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{Spatial, SpatialCapability, SpatialEntity}};
    ///
    /// // A default SpatialEntity is never valid.
    /// assert!(!SpatialEntity::default().is_valid());
    /// // And without device support for anchors, this is an invalid entity.
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// assert_eq!(anchor.is_valid(), Spatial::is_supported(SpatialCapability::Anchor));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_valid(&self) -> bool {
        self.0 != 0 && unsafe { spatial_entity_is_valid(self.0) != 0 }
    }

    /// Is the system tracking this entity right now? While active, [`SpatialEntity::get_pose`] and all component data
    /// are live. While inactive, they're last known and may be stale, or empty if the entity hasn't been tracked yet.
    /// For requests still in progress, like a new anchor, check [`SpatialEntity::get_status`] instead. Entities that are
    /// permanently lost leave the entity list, except persisted ones, which keep their identifier with
    /// [`SpatialStatus::Pending`] while storage loads them again.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Tracked.html>
    ///
    /// see also [`spatial_entity_get_tracked`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     // Until the system takes over, a new anchor sits at its pose with tracking inactive.
    ///     println!("Tracked: {:?}", anchor.get_tracked());
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_tracked(&self) -> BtnState {
        unsafe { spatial_entity_get_tracked(self.0) }
    }

    /// Where this entity is in the world, from its anchor, or else the center of its 2D or 3D bounds. This is live
    /// while [`SpatialEntity::get_tracked`] is active, and last known otherwise. It's [`Pose::default`] until the entity
    /// has any data, like a [`SpatialEntity::find_anchor`] that hasn't loaded yet, or something first seen while
    /// untracked. For a specific meaning, use the matching `try_get_*`.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Pose.html>
    ///
    /// see also [`spatial_entity_get_pose`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// // It's Pose::default() until the entity has any data.
    /// assert_eq!(anchor.get_pose(), Pose::default());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_pose(&self) -> Pose {
        unsafe { spatial_entity_get_pose(self.0) }
    }

    /// The set of components this entity has valid data for. Each component has a matching `try_get_*` accessor.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Components.html>
    ///
    /// see also [`spatial_entity_get_components`] [`SpatialComponent`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{SpatialComponent, SpatialEntity}};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     // A new anchor always has the Anchor component.
    ///     assert!(anchor.get_components().contains(SpatialComponent::Anchor));
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_components(&self) -> SpatialComponent {
        unsafe { spatial_entity_get_components(self.0) }
    }

    /// Whether the things you've asked of this entity have gone through, like creating, loading, or persisting it.
    /// [`SpatialStatus::Ready`] means all done, [`SpatialStatus::Pending`] means something's still in progress, and
    /// negative values are failures: [`SpatialStatus::Partial`] if a request like [`SpatialEntity::persist`] failed,
    /// [`SpatialStatus::Failed`] if the entity couldn't be created or found at all. This is separate from
    /// [`SpatialEntity::get_tracked`], so a Pending anchor can still be tracked and usable.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Status.html>
    ///
    /// see also [`spatial_entity_get_status`] [`SpatialStatus`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{SpatialEntity, SpatialStatus}};
    ///
    /// assert_eq!(SpatialEntity::default().get_status(), SpatialStatus::None);
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     println!("Status: {:?}", anchor.get_status());
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_status(&self) -> SpatialStatus {
        unsafe { spatial_entity_get_status(self.0) }
    }

    /// Components whose data meaningfully changed this frame! Continuously updating poses are only flagged when they
    /// first arrive, while mesh/polygon/marker data is flagged whenever the system provides new data. Handy for
    /// skipping expensive work like mesh extraction when nothing changed. These flags reset every frame, so check them
    /// each frame or you may miss an update.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Changed.html>
    ///
    /// see also [`spatial_entity_get_changed`] [`SpatialEntity::has_changed`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// // These flags reset every frame, so check them each frame.
    /// for entity in SpatialEntity::all() {
    ///     let _changed = entity.get_changed();
    ///     if entity.has_changed(SpatialComponent::Mesh) {
    ///         println!("New mesh data!");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_changed(&self) -> SpatialComponent {
        unsafe { spatial_entity_get_changed(self.0) }
    }

    /// Does this entity have data for all of these components?
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Has.html>
    /// * `components` - One or more components to check.
    ///
    /// Returns true if every component given is present.
    ///
    /// see also [`SpatialEntity::get_components`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{SpatialComponent, SpatialEntity}};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     assert!(anchor.has(SpatialComponent::Anchor));
    /// }
    /// assert!(!SpatialEntity::default().has(SpatialComponent::None));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn has(&self, components: SpatialComponent) -> bool {
        !components.is_empty() && (self.get_components() & components) == components
    }

    /// Did any of these components change this frame? Like [`SpatialEntity::get_changed`], this resets every frame, so
    /// check it each frame.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/HasChanged.html>
    /// * `components` - One or more components to check.
    ///
    /// Returns true if any component given changed this frame.
    ///
    /// see also [`SpatialEntity::get_changed`]
    /// see example in [`SpatialEntity::get_changed`]
    pub fn has_changed(&self, components: SpatialComponent) -> bool {
        !(self.get_changed() & components).is_empty()
    }

    /// The entity this entity is attached to. This is an invalid entity if there's no parent, so check
    /// [`SpatialEntity::is_valid`].
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Parent.html>
    ///
    /// see also [`spatial_entity_get_parent`] [`SpatialComponent::Parent`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// // This is an invalid entity if there's no parent, so check is_valid().
    /// assert!(!anchor.get_parent().is_valid());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_parent(&self) -> SpatialEntity {
        SpatialEntity(unsafe { spatial_entity_get_parent(self.0) })
    }

    /// The 2D rectangular bounds of this entity, such as the extents of a detected plane, or the shape of a marker. The
    /// pose faces out of the surface, so its Forward is the surface normal, the same way quads and text face in
    /// StereoKit. A floor's pose faces up, and a wall's pose faces into the room. The center is usually the same as
    /// [`SpatialEntity::get_pose`], but can differ on entities with several pose components, like a table with both a
    /// top and a volume.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetBounds2D.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Bounds2D`], or else the pose at the center of the
    /// rectangle, and the rectangle's total size in meters, along the center pose's X and Y axes.
    ///
    /// see also [`spatial_entity_get_bounds2d`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Bounds2D) {
    ///     if let Some((center, size)) = entity.try_get_bounds2d() {
    ///         println!("Rectangle at {:?}, size {size:?}", center.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_bounds2d(&self) -> Option<(Pose, Vec2)> {
        let mut center = MaybeUninit::<Pose>::uninit();
        let mut size = MaybeUninit::<Vec2>::uninit();
        if unsafe { spatial_entity_get_bounds2d(self.0, center.as_mut_ptr(), size.as_mut_ptr()) } != 0 {
            Some(unsafe { (center.assume_init(), size.assume_init()) })
        } else {
            None
        }
    }

    /// The oriented 3D bounding volume of this entity. When the entity has a front, like a screen or table top, the
    /// center pose's Forward is the direction it faces. The center is usually the same as [`SpatialEntity::get_pose`],
    /// but can differ on entities with several pose components, like a table with both a top and a volume.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetBounds3D.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Bounds3D`], or else the pose at the center of the volume,
    /// and the volume's total size in meters, along the center pose's axes.
    ///
    /// see also [`spatial_entity_get_bounds3d`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Bounds3D) {
    ///     if let Some((center, size)) = entity.try_get_bounds3d() {
    ///         println!("Volume at {:?}, size {size:?}", center.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_bounds3d(&self) -> Option<(Pose, Vec3)> {
        let mut center = MaybeUninit::<Pose>::uninit();
        let mut size = MaybeUninit::<Vec3>::uninit();
        if unsafe { spatial_entity_get_bounds3d(self.0, center.as_mut_ptr(), size.as_mut_ptr()) } != 0 {
            Some(unsafe { (center.assume_init(), size.assume_init()) })
        } else {
            None
        }
    }

    /// The general orientation of a detected plane, like horizontal or vertical. Plane tracking always provides this,
    /// so it's a reliable fallback on devices that don't provide labels.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetPlaneAlign.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::PlaneAlignment`], or else the plane's general
    /// orientation, [`PlaneAlign::None`] if unavailable.
    ///
    /// see also [`spatial_entity_get_plane_align`] [`PlaneAlign`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::PlaneAlignment) {
    ///     if let Some(alignment) = entity.try_get_plane_align() {
    ///         println!("Plane alignment: {alignment:?}");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_plane_align(&self) -> Option<PlaneAlign> {
        let mut alignment = MaybeUninit::<PlaneAlign>::uninit();
        if unsafe { spatial_entity_get_plane_align(self.0, alignment.as_mut_ptr()) } != 0 {
            Some(unsafe { alignment.assume_init() })
        } else {
            None
        }
    }

    /// A semantic category for this entity, like floor or table. Not all devices provide labels, so check
    /// [`Spatial::components_for`] for the capability you're using. For planes, [`SpatialEntity::try_get_plane_align`]
    /// makes a good fallback.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetLabel.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Label`], or else the entity's semantic category,
    /// [`SpatialLabel::None`] if unavailable.
    ///
    /// see also [`spatial_entity_get_label`] [`SpatialLabel`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Label) {
    ///     if let Some(label) = entity.try_get_label() {
    ///         println!("Label: {label:?}");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_label(&self) -> Option<SpatialLabel> {
        let mut label = MaybeUninit::<SpatialLabel>::uninit();
        if unsafe { spatial_entity_get_label(self.0, label.as_mut_ptr()) } != 0 {
            Some(unsafe { label.assume_init() })
        } else {
            None
        }
    }

    /// The entity's 3D mesh! Mesh vertices are relative to the origin pose, which the system keeps aligned with the
    /// physical world, so draw the mesh at origin each frame. Filling the Mesh is the expensive path, so pass None to
    /// fetch just the current origin, and refill only when [`SpatialEntity::get_changed`] flags the
    /// [`SpatialComponent::Mesh`] component.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetMesh.html>
    /// * `mesh` - A valid [`Mesh`] to fill with the entity's geometry, or None to only retrieve the origin pose.
    ///
    /// Returns None if this entity has no [`SpatialComponent::Mesh`], or else the pose the mesh's vertices are relative
    /// to.
    ///
    /// see also [`spatial_entity_get_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, mesh::Mesh, spatial::{SpatialComponent, SpatialEntity}};
    ///
    /// let mesh = Mesh::generate_cube(Vec3::ONE, None);
    /// for entity in SpatialEntity::with(SpatialComponent::Mesh) {
    ///     // Pass None to fetch just the origin without filling a mesh.
    ///     if let Some(origin) = entity.try_get_mesh(Some(&mesh)) {
    ///         println!("Mesh origin at {:?}", origin.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_mesh(&self, mesh: Option<&Mesh>) -> Option<Pose> {
        let mut origin = MaybeUninit::<Pose>::uninit();
        let mesh_t = mesh.map_or(null_mut(), |mesh| mesh.0.as_ptr());
        if unsafe { spatial_entity_get_mesh(self.0, mesh_t, origin.as_mut_ptr()) } != 0 {
            Some(unsafe { origin.assume_init() })
        } else {
            None
        }
    }

    /// The entity's 2D surface mesh, on the XY plane of the origin pose. Works just like [`SpatialEntity::try_get_mesh`],
    /// so pass None to fetch just the origin, and refill when [`SpatialEntity::get_changed`] flags the
    /// [`SpatialComponent::Mesh2D`] component.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetMesh2D.html>
    /// * `mesh` - A valid [`Mesh`] to fill with the entity's geometry, or None to only retrieve the origin pose.
    ///
    /// Returns None if this entity has no [`SpatialComponent::Mesh2D`], or else the pose the mesh's vertices are
    /// relative to.
    ///
    /// see also [`spatial_entity_get_mesh2d`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, mesh::Mesh, spatial::{SpatialComponent, SpatialEntity}};
    ///
    /// let mesh = Mesh::generate_cube(Vec3::ONE, None);
    /// for entity in SpatialEntity::with(SpatialComponent::Mesh2D) {
    ///     if let Some(origin) = entity.try_get_mesh2d(Some(&mesh)) {
    ///         println!("2D mesh origin at {:?}", origin.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_mesh2d(&self, mesh: Option<&Mesh>) -> Option<Pose> {
        let mut origin = MaybeUninit::<Pose>::uninit();
        let mesh_t = mesh.map_or(null_mut(), |mesh| mesh.0.as_ptr());
        if unsafe { spatial_entity_get_mesh2d(self.0, mesh_t, origin.as_mut_ptr()) } != 0 {
            Some(unsafe { origin.assume_init() })
        } else {
            None
        }
    }

    /// The boundary polygon outlining the entity's surface, on the XY plane of the origin pose. This allocates a new
    /// array each call, so for every frame use, prefer [`SpatialEntity::try_get_polygon_into`] which reuses one.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetPolygon.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Polygon`], or else the pose the polygon's points are
    /// relative to, and a copy of the boundary points, in meters on the origin's XY plane.
    ///
    /// see also [`spatial_entity_get_polygon`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Polygon) {
    ///     if let Some((origin, polygon)) = entity.try_get_polygon() {
    ///         println!("{} points at {:?}", polygon.len(), origin.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_polygon(&self) -> Option<(Pose, Vec<Vec2>)> {
        let mut origin = MaybeUninit::<Pose>::uninit();
        let mut verts = null();
        let mut count = 0;
        if unsafe { spatial_entity_get_polygon(self.0, origin.as_mut_ptr(), &mut verts, &mut count) } == 0 {
            return None;
        }
        let count = count.max(0) as usize;
        let mut polygon = Vec::with_capacity(count);
        if count > 0 && !verts.is_null() {
            polygon.extend_from_slice(unsafe { std::slice::from_raw_parts(verts, count) });
        }
        Some((unsafe { origin.assume_init() }, polygon))
    }

    /// The boundary polygon outlining the entity's surface, copied into a vector you keep, so calling this every frame
    /// doesn't allocate. The vector only grows when it's too small, so use its length as the point count rather than its
    /// capacity.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetPolygon.html>
    /// * `polygon` - A vector to copy the boundary points into, in meters on the origin's XY plane. It's resized to the
    ///   polygon's point count as needed.
    ///
    /// Returns None if this entity has no [`SpatialComponent::Polygon`], or else the pose the polygon's points are
    /// relative to.
    ///
    /// see also [`spatial_entity_get_polygon`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// let mut polygon = Vec::new();
    /// for entity in SpatialEntity::with(SpatialComponent::Polygon) {
    ///     // The Vec is only reallocated when it's too small, so this doesn't allocate every frame.
    ///     if let Some(origin) = entity.try_get_polygon_into(&mut polygon) {
    ///         println!("{} points at {:?}", polygon.len(), origin.position);
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_polygon_into(&self, polygon: &mut Vec<Vec2>) -> Option<Pose> {
        let mut origin = MaybeUninit::<Pose>::uninit();
        let mut verts = null();
        let mut count = 0;
        if unsafe { spatial_entity_get_polygon(self.0, origin.as_mut_ptr(), &mut verts, &mut count) } == 0 {
            return None;
        }
        let count = count.max(0) as usize;
        polygon.clear();
        if count > 0 && !verts.is_null() {
            polygon.extend_from_slice(unsafe { std::slice::from_raw_parts(verts, count) });
        }
        Some(unsafe { origin.assume_init() })
    }

    /// Marker information, for entities discovered by a marker tracking capability like QR codes or ArUco markers. The
    /// marker's pose and physical size come from [`SpatialEntity::try_get_bounds2d`].
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetMarker.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Marker`], or else what kind of marker this is, and the
    /// marker's numeric id, for marker dictionaries like ArUco and AprilTag. 0 for QR codes.
    ///
    /// see also [`spatial_entity_get_marker`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Marker) {
    ///     if let Some((marker_type, marker_id)) = entity.try_get_marker() {
    ///         println!("{marker_type:?} marker, id {marker_id}");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_marker(&self) -> Option<(MarkerType, u32)> {
        let mut marker_type = MaybeUninit::<MarkerType>::uninit();
        let mut marker_id = 0;
        if unsafe { spatial_entity_get_marker(self.0, marker_type.as_mut_ptr(), &mut marker_id) } != 0 {
            Some(unsafe { (marker_type.assume_init(), marker_id) })
        } else {
            None
        }
    }

    /// The marker's decoded string data, for QR family markers that contain text. None if unavailable. Each read
    /// creates a new string, so cache it, and read again when [`SpatialEntity::get_changed`] flags the
    /// [`SpatialComponent::Marker`] component.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/MarkerText.html>
    ///
    /// see also [`spatial_entity_get_marker_text`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// for entity in SpatialEntity::with(SpatialComponent::Marker) {
    ///     if let Some(text) = entity.get_marker_text() {
    ///         println!("QR says: {text}");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_marker_text(&self) -> Option<String> {
        let text = unsafe { spatial_entity_get_marker_text(self.0) };
        if text.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned())
        }
    }

    /// The marker's raw decoded bytes, for markers with binary data. None if unavailable. Each read allocates a new
    /// array, see [`SpatialEntity::try_get_marker_data`] to reuse one.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/MarkerData.html>
    ///
    /// see also [`spatial_entity_get_marker_data`]
    /// see example in [`SpatialEntity::try_get_marker_data`]
    pub fn get_marker_data(&self) -> Option<Vec<u8>> {
        let mut data = Vec::new();
        self.try_get_marker_data(&mut data)?;
        Some(data)
    }

    /// The marker's raw decoded bytes, copied into a vector you keep, so repeated reads don't allocate. The vector only
    /// grows when it's too small, so repeated reads of the same marker won't allocate at all.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetMarkerData.html>
    /// * `data` - A vector to copy the bytes into. It's resized to the marker's data size as needed.
    ///
    /// Returns None if the marker has no binary data, or else the number of bytes copied into `data`.
    ///
    /// see also [`spatial_entity_get_marker_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// let mut data = Vec::new();
    /// for entity in SpatialEntity::with(SpatialComponent::Marker) {
    ///     if let Some(size) = entity.try_get_marker_data(&mut data) {
    ///         println!("{size} bytes of marker data");
    ///     }
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_marker_data(&self, data: &mut Vec<u8>) -> Option<usize> {
        let mut size = 0;
        let source = unsafe { spatial_entity_get_marker_data(self.0, &mut size) };
        if source.is_null() {
            return None;
        }
        let size = size.max(0) as usize;
        data.clear();
        if size > 0 {
            data.extend_from_slice(unsafe { std::slice::from_raw_parts(source, size) });
        }
        Some(size)
    }

    /// Removes an app-created entity like an anchor from the system entirely. It's unpersisted if persisted, the system
    /// stops tracking it, and it leaves the entity list at the end of the frame. This SpatialEntity stops resolving once
    /// that happens. This also works on entities that are still [`SpatialStatus::Pending`], including ones from
    /// [`SpatialEntity::find_anchor`] that haven't loaded yet.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Destroy.html>
    ///
    /// Returns false if this entity can't be destroyed! Only app-created entities like anchors can be, entities the
    /// system discovered on its own, like planes, cannot.
    ///
    /// see also [`spatial_entity_destroy`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     anchor.destroy();
    /// }
    /// // Only app-created entities like anchors can be destroyed.
    /// assert!(!SpatialEntity::default().destroy());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn destroy(&self) -> bool {
        unsafe { spatial_entity_destroy(self.0) != 0 }
    }

    /// A durable identifier for this entity that stays the same across sessions and device reboots! Entities only have
    /// one once they're persisted, either by the system itself, or by a call to [`SpatialEntity::persist`]. Store it,
    /// and pass it to [`SpatialEntity::find_anchor_uuid`] to get the same physical entity back in a later session.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetGuid.html>
    ///
    /// Returns None if this entity has no [`SpatialComponent::Persistence`], or else the entity's persistent
    /// identifier.
    ///
    /// see also [`spatial_entity_get_uuid`] [`Uuid`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), Some("uuid_anchor"), None);
    /// // Entities only have a Uuid once they're persisted.
    /// if let Some(uuid) = anchor.try_get_uuid() {
    ///     println!("Stored as {uuid}");
    /// }
    /// # if anchor.is_valid() { anchor.destroy(); }
    /// # stereokit_rust::anchor::Anchor::clear_store(); // !!!! Clear the store to avoid side effects for other tests !!!!
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_uuid(&self) -> Option<Uuid> {
        let mut uuid = MaybeUninit::<Uuid>::uninit();
        if unsafe { spatial_entity_get_uuid(self.0, uuid.as_mut_ptr()) } != 0 {
            Some(unsafe { uuid.assume_init() })
        } else {
            None
        }
    }

    /// The name this anchor was given by [`SpatialEntity::create_anchor`], or by the
    /// [`Anchor`](crate::anchor::Anchor) class. Persisted anchors get their name back when they load in a later
    /// session, however they were found, so this is a good way to tell restored anchors apart.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/TryGetName.html>
    ///
    /// Returns None if this entity has no name, or else the anchor's name.
    ///
    /// see also [`spatial_entity_get_name`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), Some("named_anchor"), None);
    /// if let Some(name) = anchor.try_get_name() {
    ///     assert_eq!(name, "named_anchor");
    /// }
    /// assert!(SpatialEntity::default().try_get_name().is_none());
    /// # if anchor.is_valid() { anchor.destroy(); }
    /// # stereokit_rust::anchor::Anchor::clear_store(); // !!!! Clear the store to avoid side effects for other tests !!!!
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn try_get_name(&self) -> Option<String> {
        let name = unsafe { spatial_entity_get_name(self.0) };
        if name.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned())
        }
    }

    /// Ask the system to persist this entity, giving it a durable identity that survives across sessions! This is
    /// asynchronous, and safe to call right away since it waits until the entity is tracking and persistence has started
    /// up. On success, [`SpatialEntity::try_get_uuid`] succeeds and [`SpatialEntity::get_changed`] flags the
    /// [`SpatialComponent::Persistence`] component. On failure, [`SpatialEntity::get_status`] becomes
    /// [`SpatialStatus::Partial`] and the entity stays without a Uuid. If an [`SpatialEntity::unpersist`] is still in
    /// flight, this persists again once it lands.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Persist.html>
    ///
    /// Returns false if this can't work at all, like an invalid entity, or one from a capability without persistence
    /// support. Check [`Spatial::components_for`] for [`SpatialComponent::Persistence`] to see which capabilities
    /// support it.
    ///
    /// see also [`spatial_entity_persist`] [`SpatialEntity::unpersist`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// // False if this can't work at all, like an invalid entity.
    /// assert!(!SpatialEntity::default().persist());
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     let requested = anchor.persist();
    ///     println!("Persist requested: {requested}");
    ///     anchor.destroy();
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn persist(&self) -> bool {
        unsafe { spatial_entity_persist(self.0) != 0 }
    }

    /// Remove this entity from persistent storage. Its name, if it has one, is released right away, and it loses its
    /// Uuid once the asynchronous operation completes. [`SpatialEntity::get_status`] is [`SpatialStatus::Pending`] until
    /// then, or [`SpatialStatus::Partial`] if it fails. If a [`SpatialEntity::persist`] is still in flight, this waits
    /// for it to land and then undoes it. An entity from [`SpatialEntity::find_anchor`] that hasn't loaded yet stops
    /// loading, and once the unpersist lands it leaves the entity list, since there's nothing left to load.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Unpersist.html>
    ///
    /// Returns false if this can't work at all, like an invalid entity.
    ///
    /// see also [`spatial_entity_unpersist`] [`SpatialEntity::persist`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::SpatialEntity};
    ///
    /// // False if this can't work at all, like an invalid entity.
    /// assert!(!SpatialEntity::default().unpersist());
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// if anchor.is_valid() {
    ///     anchor.persist();
    ///     // Asynchronous, so this can land after the persist does.
    ///     assert!(anchor.unpersist());
    /// }
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn unpersist(&self) -> bool {
        unsafe { spatial_entity_unpersist(self.0) != 0 }
    }

    /// Remove something from persistent storage using just its Uuid, no live entity needed! This is how you clean up
    /// something stored in an earlier session that hasn't been rediscovered, or never will be. If persistence is still
    /// starting up, this waits for it.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Unpersist.html>
    /// * `uuid` - A Uuid from [`SpatialEntity::try_get_uuid`], saved in an earlier session.
    ///
    /// Returns false if this can't work at all, like the nil Uuid, or a system without persistence support.
    ///
    /// see also [`spatial_entity_unpersist_uuid`] [`SpatialEntity::unpersist`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialEntity, Uuid};
    ///
    /// // Nothing to clean up for the nil Uuid, and this can't work at all here.
    /// assert!(!SpatialEntity::unpersist_uuid(Uuid::default()));
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn unpersist_uuid(uuid: Uuid) -> bool {
        unsafe { spatial_entity_unpersist_uuid(uuid) != 0 }
    }

    /// Gets the anchor you created with this name, in this session or an earlier one! This is the easy way to restore
    /// anchors across app restarts. Call this once and hold onto the result. Storage loads asynchronously, so the
    /// anchor starts out with [`SpatialStatus::Pending`] and no data, then fills in when it loads. Nothing loads while
    /// [`SpatialCapability::Anchor`] isn't [`Spatial::requested`].
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/FindAnchor.html>
    /// * `name` - The name given to [`SpatialEntity::create_anchor`].
    ///
    /// Returns the anchor, already loaded or still loading. This is invalid if no anchor has this name, so check
    /// [`SpatialEntity::is_valid`].
    ///
    /// see also [`spatial_entity_find_anchor`] [`SpatialEntity::find_anchor_uuid`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{Spatial, SpatialCapability, SpatialEntity}};
    ///
    /// // Place an anchor named "table", then get it back by name!
    /// Spatial::request(SpatialCapability::Anchor);
    /// let anchor = SpatialEntity::create_anchor(Pose::default(), Some("table"), None);
    /// let found = SpatialEntity::find_anchor("table");
    /// assert_eq!(found.is_valid(), anchor.is_valid());
    /// # if found.is_valid() { found.destroy(); }
    /// # stereokit_rust::anchor::Anchor::clear_store(); // !!!! Clear the store to avoid side effects for other tests !!!!
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find_anchor<S: AsRef<str>>(name: S) -> SpatialEntity {
        match CString::new(name.as_ref()) {
            Ok(name) => SpatialEntity(unsafe { spatial_entity_find_anchor(name.as_ptr()) }),
            Err(_) => SpatialEntity::default(),
        }
    }

    /// Gets the anchor with this Uuid, for apps that keep track of anchors by Uuid themselves. Works like
    /// [`SpatialEntity::find_anchor`], and if storage doesn't have this Uuid, [`SpatialEntity::get_status`] becomes
    /// [`SpatialStatus::Failed`] and the anchor shows up in [`SpatialEntity::removed`].
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/FindAnchor.html>
    /// * `uuid` - A Uuid from [`SpatialEntity::try_get_uuid`], saved in an earlier session.
    ///
    /// Returns the anchor, already loaded or still loading. This is invalid for the nil Uuid, or a system without
    /// persistence support.
    ///
    /// see also [`spatial_entity_find_anchor_uuid`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialEntity, Uuid};
    ///
    /// // Invalid for the nil Uuid, which never identifies anything.
    /// assert!(!SpatialEntity::find_anchor_uuid(Uuid::default()).is_valid());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find_anchor_uuid(uuid: Uuid) -> SpatialEntity {
        SpatialEntity(unsafe { spatial_entity_find_anchor_uuid(uuid) })
    }

    /// Create a spatial anchor entity at the given pose, a point the system will keep aligned with the physical world
    /// as tracking improves or drifts. You can call this any time after requesting [`SpatialCapability::Anchor`], even
    /// while it's still starting up! Until the system takes over, the anchor sits at this pose with
    /// [`SpatialEntity::get_tracked`] inactive and [`SpatialStatus::Pending`]. If the system can't create it, the anchor
    /// shows up in [`SpatialEntity::removed`] with [`SpatialStatus::Failed`].
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/CreateAnchor.html>
    /// * `pose` - A world space pose for the new anchor.
    /// * `name` - Persists the anchor under this name, so [`SpatialEntity::find_anchor`] can get it back in a later
    ///   session. Any anchor that already has this name is destroyed, so placing "table" again moves it. Leave this
    ///   None for an anchor that only lasts this session, or call [`SpatialEntity::persist`] later to keep it by Uuid
    ///   instead.
    /// * `parent` - An optional entity to attach the anchor to, so it follows that entity as it moves. Few runtimes
    ///   support this yet, and creation fails on those that don't.
    ///
    /// Returns the new anchor entity. This is an invalid entity if this system doesn't support anchors, or can't
    /// persist them when given a name, so check [`SpatialEntity::is_valid`].
    ///
    /// see also [`spatial_entity_create_anchor`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Pose, spatial::{Spatial, SpatialCapability, SpatialComponent, SpatialEntity}};
    ///
    /// // You can create anchors any time after requesting the capability!
    /// Spatial::request(SpatialCapability::Anchor);
    ///
    /// // A session-only anchor at the world origin...
    /// let temp = SpatialEntity::create_anchor(Pose::default(), None, None);
    /// // ...and one that persists across sessions under the name "table".
    /// let named = SpatialEntity::create_anchor(Pose::default(), Some("table"), None);
    ///
    /// // Without device support for anchors, these are invalid entities.
    /// assert_eq!(temp.is_valid(), Spatial::is_supported(SpatialCapability::Anchor));
    /// assert_eq!(named.is_valid(),
    ///            Spatial::is_supported(SpatialCapability::Anchor)
    ///                && Spatial::is_component_supported(SpatialCapability::Anchor, SpatialComponent::Persistence));
    /// # if named.is_valid() { named.destroy(); }
    /// # stereokit_rust::anchor::Anchor::clear_store(); // !!!! Clear the store to avoid side effects for other tests !!!!
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn create_anchor(pose: Pose, name: Option<&str>, parent: Option<SpatialEntity>) -> SpatialEntity {
        let name = match name {
            Some(name) => match CString::new(name) {
                Ok(name) => Some(name),
                Err(_) => return SpatialEntity::default(),
            },
            None => None,
        };
        let name_ptr = name.as_ref().map_or(null(), |name| name.as_ptr());
        SpatialEntity(unsafe { spatial_entity_create_anchor(pose, name_ptr, parent.unwrap_or_default().0) })
    }

    /// An iterator over every spatial entity StereoKit currently knows about. This list is maintained for you, entities
    /// appear as the system discovers them, and leave when the system permanently stops tracking them. An entity in
    /// [`SpatialEntity::removed`] is still in this list for its final frame.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/All.html>
    ///
    /// see also [`spatial_entity_get_count`] [`spatial_entity_get_index`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::{SpatialComponent, SpatialEntity};
    ///
    /// // Every spatial entity StereoKit currently knows about.
    /// for entity in SpatialEntity::all() {
    ///     println!("Entity: {entity:?}");
    /// }
    /// // Or just ones with data for all the components you need.
    /// let with_bounds = SpatialEntity::with(SpatialComponent::Bounds2D).get_count();
    /// // Entities that appeared for the first time this frame, seen here exactly once.
    /// let brand_new = SpatialEntity::new_entities().get_count();
    /// let new_with   = SpatialEntity::new_with(SpatialComponent::Bounds2D).get_count();
    /// // And ones leaving the entity list this frame, the place to clean up per-entity caches.
    /// let removed      = SpatialEntity::removed().get_count();
    /// let removed_with = SpatialEntity::removed_with(SpatialComponent::Bounds2D).get_count();
    /// println!("{with_bounds} {brand_new} {new_with} {removed} {removed_with}");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn all() -> SpatialEntityIter {
        SpatialEntityIter::all()
    }

    /// An iterator over the spatial entities that have data for all the given components.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/With.html>
    /// * `components` - Components each entity must have.
    ///
    /// Returns an iterator over matching entities.
    ///
    /// see also [`SpatialEntity::all`]
    /// see example in [`SpatialEntity::all`]
    pub fn with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter::with(components)
    }

    /// An iterator over the spatial entities that appeared for the first time this frame. Entities you create yourself,
    /// like with [`SpatialEntity::create_anchor`] or [`SpatialEntity::find_anchor`], show up here on the following
    /// frame, so every entity is seen here exactly once.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/New.html>
    ///
    /// see also [`spatial_entity_get_new_count`] [`spatial_entity_get_new_index`]
    /// see example in [`SpatialEntity::all`]
    pub fn new_entities() -> SpatialEntityIter {
        SpatialEntityIter::new_entities()
    }

    /// An iterator over the spatial entities that appeared for the first time this frame, and have data for all the
    /// given components.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/NewWith.html>
    /// * `components` - Components each entity must have.
    ///
    /// Returns an iterator over matching new entities.
    ///
    /// see also [`SpatialEntity::new_entities`]
    /// see example in [`SpatialEntity::all`]
    pub fn new_with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter::new_with(components)
    }

    /// An iterator over the spatial entities leaving the entity list this frame, because they were lost by the system,
    /// destroyed, or [`SpatialStatus::Failed`]. Lost persisted entities don't leave, they wait to load again. These are
    /// still [`SpatialEntity::is_valid`] with readable data for this one frame, which makes this the place to clean up
    /// anything you've cached per-entity!
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/Removed.html>
    ///
    /// see also [`spatial_entity_get_removed_count`] [`spatial_entity_get_removed_index`]
    /// see example in [`SpatialEntity::all`]
    pub fn removed() -> SpatialEntityIter {
        SpatialEntityIter::removed()
    }

    /// An iterator over the spatial entities leaving the entity list this frame that have data for all the given
    /// components.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntity/RemovedWith.html>
    /// * `components` - Components each entity must have.
    ///
    /// Returns an iterator over matching removed entities.
    ///
    /// see also [`SpatialEntity::removed`]
    /// see example in [`SpatialEntity::all`]
    pub fn removed_with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter::removed_with(components)
    }
}

/// An iterator over SpatialEntities, as provided by [`SpatialEntity::all`], [`SpatialEntity::with`],
/// [`SpatialEntity::new_entities`], and [`SpatialEntity::removed`]. Iteration is allocation free.
/// <https://stereokit.net/Pages/StereoKit/SpatialEntityCollection.html>
///
/// see also [`SpatialEntity`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::spatial::{SpatialEntity, SpatialEntityIter};
///
/// for entity in SpatialEntity::all() {
///     println!("Entity: {entity:?}");
/// }
/// // Same thing, straight from the iterator.
/// for entity in SpatialEntityIter::all() {
///     println!("Entity: {entity:?}");
/// }
/// assert_eq!(SpatialEntityIter::all().get_count(), SpatialEntity::all().get_count());
/// # test_steps!();
/// # sk::Sk::shutdown();
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SpatialEntityIter {
    filter: SpatialComponent,
    list: SpatialEntityList,
    index: i32,
}

/// Which entity list a [`SpatialEntityIter`] walks over.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SpatialEntityList {
    All,
    New,
    Removed,
}

impl Iterator for SpatialEntityIter {
    type Item = SpatialEntity;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let id = unsafe {
            match self.list {
                SpatialEntityList::All => spatial_entity_get_index(self.filter, self.index),
                SpatialEntityList::New => spatial_entity_get_new_index(self.filter, self.index),
                SpatialEntityList::Removed => spatial_entity_get_removed_index(self.filter, self.index),
            }
        };
        if id == 0 { None } else { Some(SpatialEntity(id)) }
    }
}

impl SpatialEntityIter {
    /// Get the number of entities in this iterator's list. Unlike [`Iterator::count()`] does not consume the iterator.
    /// <https://stereokit.net/Pages/StereoKit/SpatialEntityCollection/Count.html>
    ///
    /// see also [`spatial_entity_get_count`] [`spatial_entity_get_new_count`] [`spatial_entity_get_removed_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::spatial::SpatialEntity;
    ///
    /// let entities = SpatialEntity::all();
    /// assert_eq!(entities.get_count(), entities.count() as i32);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe {
            match self.list {
                SpatialEntityList::All => spatial_entity_get_count(self.filter),
                SpatialEntityList::New => spatial_entity_get_new_count(self.filter),
                SpatialEntityList::Removed => spatial_entity_get_removed_count(self.filter),
            }
        }
    }

    /// An iterator over every spatial entity StereoKit currently knows about, see [`SpatialEntity::all`].
    pub fn all() -> SpatialEntityIter {
        SpatialEntityIter { filter: SpatialComponent::None, list: SpatialEntityList::All, index: -1 }
    }

    /// An iterator over the spatial entities with data for all the given components, see [`SpatialEntity::with`].
    /// * `components` - Components each entity must have.
    pub fn with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter { filter: components, list: SpatialEntityList::All, index: -1 }
    }

    /// An iterator over the spatial entities that appeared for the first time this frame, see
    /// [`SpatialEntity::new_entities`].
    pub fn new_entities() -> SpatialEntityIter {
        SpatialEntityIter { filter: SpatialComponent::None, list: SpatialEntityList::New, index: -1 }
    }

    /// An iterator over the new spatial entities with data for all the given components, see
    /// [`SpatialEntity::new_with`].
    /// * `components` - Components each entity must have.
    pub fn new_with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter { filter: components, list: SpatialEntityList::New, index: -1 }
    }

    /// An iterator over the spatial entities leaving the entity list this frame, see [`SpatialEntity::removed`].
    pub fn removed() -> SpatialEntityIter {
        SpatialEntityIter { filter: SpatialComponent::None, list: SpatialEntityList::Removed, index: -1 }
    }

    /// An iterator over the removed spatial entities with data for all the given components, see
    /// [`SpatialEntity::removed_with`].
    /// * `components` - Components each entity must have.
    pub fn removed_with(components: SpatialComponent) -> SpatialEntityIter {
        SpatialEntityIter { filter: components, list: SpatialEntityList::Removed, index: -1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_enum_values() {
        // Check that enum values match the expected constants
        assert_eq!(SpatialCapability::None.bits(), 0);
        assert_eq!(SpatialCapability::Anchor.bits(), 1);
        assert_eq!(SpatialCapability::PlaneTracking.bits(), 2);
        assert_eq!(SpatialCapability::QrCode.bits(), 4);
        assert_eq!(SpatialCapability::MicroQr.bits(), 8);
        assert_eq!(SpatialCapability::Aruco.bits(), 16);
        assert_eq!(SpatialCapability::AprilTag.bits(), 32);

        assert_eq!(SpatialComponent::None.bits(), 0);
        assert_eq!(SpatialComponent::Bounds2D.bits(), 1);
        assert_eq!(SpatialComponent::Bounds3D.bits(), 2);
        assert_eq!(SpatialComponent::Parent.bits(), 4);
        assert_eq!(SpatialComponent::Mesh.bits(), 8);
        assert_eq!(SpatialComponent::Anchor.bits(), 16);
        assert_eq!(SpatialComponent::Persistence.bits(), 32);
        assert_eq!(SpatialComponent::PlaneAlignment.bits(), 64);
        assert_eq!(SpatialComponent::Mesh2D.bits(), 128);
        assert_eq!(SpatialComponent::Polygon.bits(), 256);
        assert_eq!(SpatialComponent::Label.bits(), 512);
        assert_eq!(SpatialComponent::Marker.bits(), 1024);

        assert_eq!(SpatialStatus::Failed as i32, -2);
        assert_eq!(SpatialStatus::Partial as i32, -1);
        assert_eq!(SpatialStatus::None as i32, 0);
        assert_eq!(SpatialStatus::Pending as i32, 1);
        assert_eq!(SpatialStatus::Ready as i32, 2);

        assert_eq!(PlaneAlign::None as u32, 0);
        assert_eq!(PlaneAlign::HorizontalUp as u32, 1);
        assert_eq!(PlaneAlign::HorizontalDown as u32, 2);
        assert_eq!(PlaneAlign::Vertical as u32, 3);
        assert_eq!(PlaneAlign::Arbitrary as u32, 4);

        assert_eq!(SpatialLabel::None as u32, 0);
        assert_eq!(SpatialLabel::Uncategorized as u32, 1);
        assert_eq!(SpatialLabel::Floor as u32, 2);
        assert_eq!(SpatialLabel::Wall as u32, 3);
        assert_eq!(SpatialLabel::Ceiling as u32, 4);
        assert_eq!(SpatialLabel::Table as u32, 5);

        assert_eq!(MarkerType::None as u32, 0);
        assert_eq!(MarkerType::QrCode as u32, 1);
        assert_eq!(MarkerType::MicroQr as u32, 2);
        assert_eq!(MarkerType::Aruco as u32, 3);
        assert_eq!(MarkerType::AprilTag as u32, 4);

        assert_eq!(ArucoDict::Default as u32, 0);
        assert_eq!(ArucoDict::Dict4x4_50 as u32, 1);
        assert_eq!(ArucoDict::Dict7x7_1000 as u32, 16);

        assert_eq!(AprilTagDict::Default as u32, 0);
        assert_eq!(AprilTagDict::Tag16h5 as u32, 1);
        assert_eq!(AprilTagDict::Tag25h9 as u32, 2);
        assert_eq!(AprilTagDict::Tag36h10 as u32, 3);
        assert_eq!(AprilTagDict::Tag36h11 as u32, 4);
    }

    #[test]
    fn test_uuid() {
        let nil = Uuid::default();
        assert!(nil.is_nil());
        assert_eq!(format!("{nil}"), "00000000-0000-0000-0000-000000000000");

        let id = Uuid::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        assert!(!id.is_nil());
        assert_eq!(id.bytes(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        assert_eq!(format!("{id}"), "01020304-0506-0708-090a-0b0c0d0e0f10");
    }

    #[test]
    fn test_spatial_entity_default() {
        // A default SpatialEntity is never valid, and is a lightweight identifier we can copy around.
        let entity = SpatialEntity::default();
        let copy = entity;
        assert_eq!(entity, copy);
        assert!(!entity.is_valid());
    }
}
