// SPDX-License-Identifier: MIT
//! A port of StereoKit's `DemoSpatialEntity`, showcasing the `Spatial`/`SpatialEntity` APIs: scene understanding
//! capabilities like plane tracking and QR/ArUco/AprilTag markers, per-entity components (bounds, polygons, meshes,
//! labels), and anchor creation/persistence.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    spatial::{
        ArUcoDict, MarkerType, PlaneAlign, Spatial, SpatialCapability, SpatialComponent, SpatialEntity, SpatialLabel,
        SpatialStatus, Uuid,
    },
    system::{Input, Lines, Pivot, Text, TextBuilder, TextStyle},
    ui::Ui,
    util::{
        Color32,
        named_colors::{BLUE, GRAY, GREEN, RED, WHITE, YELLOW},
    },
};

/// The spatial capabilities this demo can toggle, with their display names.
const CAP_NAMES: [(SpatialCapability, &str); 6] = [
    (SpatialCapability::Anchor, "Anchors"),
    (SpatialCapability::PlaneTracking, "Plane Tracking"),
    (SpatialCapability::QrCode, "QR Codes"),
    (SpatialCapability::MicroQr, "Micro QR Codes"),
    (SpatialCapability::Aruco, "ArUco Markers"),
    (SpatialCapability::AprilTag, "AprilTags"),
];

/// All the ArUco dictionaries, cycled through by the dictionary button.
const ARUCO_DICTS: [(ArUcoDict, &str); 17] = [
    (ArUcoDict::Default, "Default"),
    (ArUcoDict::Dict4x4_50, "Dict4x4_50"),
    (ArUcoDict::Dict4x4_100, "Dict4x4_100"),
    (ArUcoDict::Dict4x4_250, "Dict4x4_250"),
    (ArUcoDict::Dict4x4_1000, "Dict4x4_1000"),
    (ArUcoDict::Dict5x5_50, "Dict5x5_50"),
    (ArUcoDict::Dict5x5_100, "Dict5x5_100"),
    (ArUcoDict::Dict5x5_250, "Dict5x5_250"),
    (ArUcoDict::Dict5x5_1000, "Dict5x5_1000"),
    (ArUcoDict::Dict6x6_50, "Dict6x6_50"),
    (ArUcoDict::Dict6x6_100, "Dict6x6_100"),
    (ArUcoDict::Dict6x6_250, "Dict6x6_250"),
    (ArUcoDict::Dict6x6_1000, "Dict6x6_1000"),
    (ArUcoDict::Dict7x7_50, "Dict7x7_50"),
    (ArUcoDict::Dict7x7_100, "Dict7x7_100"),
    (ArUcoDict::Dict7x7_250, "Dict7x7_250"),
    (ArUcoDict::Dict7x7_1000, "Dict7x7_1000"),
];

/// Uuids only mean something if you keep them! This file remembers every anchor the app has persisted, so they can
/// be restored or cleaned up in a later session.
const SAVED_IDS_FILE: &str = "spatial_anchor_ids.txt";

/// Per-entity cache for anything expensive to fetch or build, only refreshed when the entity's `Changed` flags say
/// the data is new.
#[derive(Default)]
struct EntityVisual {
    mesh: Option<Mesh>,
    mesh_origin: Pose,
    polygon: Vec<Vec2>,
    polygon_origin: Pose,
    name: String,
    anchor_label: String,
    uuid: Uuid,
}

/// Demo of the spatial entity APIs: capabilities like plane tracking and markers, per-entity components, and
/// anchor creation/persistence.
#[derive(IStepper)]
pub struct Spatial1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    window_pose: Pose,
    mesh_mat: Material,
    show_meshes: bool,
    persist_new: bool,
    aruco_dict: ArUcoDict,
    marker_size: f32,
    prev_requested: SpatialCapability,
    visuals: HashMap<SpatialEntity, EntityVisual>,
    saved_ids: HashSet<Uuid>,
    shutdown_completed: bool,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Spatial1 {}

impl Default for Spatial1 {
    fn default() -> Self {
        let mut mesh_mat = Material::default_copy();
        mesh_mat.wireframe(true);
        Self {
            id: "Spatial1".to_string(),
            sk_info: None,

            window_pose: Pose::new(Vec3::NEG_Z * 0.5 + Vec3::Y * 1.5, Some(Quat::from_angles(0.0, 180.0, 0.0))),
            mesh_mat,
            show_meshes: true,
            persist_new: false,
            aruco_dict: ArUcoDict::Default,
            marker_size: 0.0,
            prev_requested: SpatialCapability::None,
            visuals: HashMap::new(),
            saved_ids: HashSet::new(),
            shutdown_completed: false,

            text: "Spatial1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Spatial1 {
    /// Called by derive macro during IStepper::initialize
    fn start(&mut self) -> bool {
        // Remember what was already requested, so shutdown can restore it for the other demos.
        self.prev_requested = Spatial::requested();
        // Ask for everything the device can do! Capabilities start up asynchronously.
        Spatial::request(Spatial::capabilities());

        // Restore the anchors we persisted in an earlier session. Storage loads asynchronously, so these start out
        // Pending and fill in as they're found.
        self.saved_ids = load_saved_ids();
        for id in &self.saved_ids {
            SpatialEntity::find_anchor_uuid(*id);
        }
        true
    }

    /// Called by derive macro when the stepper shuts down
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Disabling everything would also turn off the Anchor system for later demos.
            Spatial::disable(Spatial::capabilities() & !self.prev_requested);
            self.visuals.clear();
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }

    /// Called by derive macro during IStepper::step
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called by derive macro during IStepper::step
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("Spatial Entities").pose(&mut self.window_pose).begin();

        // Capability toggles, greyed out when the device doesn't support them. Requested and disabled capabilities
        // can be changed at any time, entities show up as the system starts up and discovers them.
        let supported = Spatial::capabilities();
        let requested = Spatial::requested();
        for (cap, name) in CAP_NAMES {
            Ui::push_enabled(supported.contains(cap), None);
            let mut on = requested.contains(cap);
            if Ui::toggle(name, &mut on).interact().is_some() {
                if on {
                    Spatial::request(cap);
                } else {
                    Spatial::disable(cap);
                }
            }
            Ui::pop_enabled();
        }

        // Pending entities are ones still being created or loaded, like a persisted anchor waiting for the system
        // to find it again.
        let (mut tracked, mut pending) = (0, 0);
        for entity in SpatialEntity::all() {
            if entity.get_tracked().is_active() {
                tracked += 1;
            }
            if entity.get_status() == SpatialStatus::Pending {
                pending += 1;
            }
        }
        Ui::hseparator();
        Ui::label(format!("{} entities, {tracked} tracked, {pending} pending", SpatialEntity::all().get_count()))
            .draw();
        Ui::label(format!(
            "{} new, {} removed this frame",
            SpatialEntity::new_entities().get_count(),
            SpatialEntity::removed().get_count()
        ))
        .draw();
        Ui::toggle("Show Meshes", &mut self.show_meshes).interact();

        // Marker configuration! Best set before requesting the marker capabilities, since changing it restarts
        // that tracking.
        Ui::hseparator();
        if Ui::hslider("marker_size", &mut self.marker_size, 0.0, 0.5).interact().is_some() {
            // The edge length of the printed markers, in meters. 0 means mixed or unknown sizes.
            Spatial::set_marker_size(MarkerType::QrCode, self.marker_size);
        }
        let dict_index = ARUCO_DICTS.iter().position(|(dict, _)| *dict == self.aruco_dict).unwrap_or(0);
        if Ui::button(format!("ArUco dict: {}", ARUCO_DICTS[dict_index].1)).press() {
            self.aruco_dict = ARUCO_DICTS[(dict_index + 1) % ARUCO_DICTS.len()].0;
            Spatial::set_aruco_dictionary(self.aruco_dict);
        }
        Ui::same_line();
        let mut stationary = Spatial::get_marker_stationary(MarkerType::QrCode);
        if Ui::toggle("QR stationary", &mut stationary).interact().is_some() {
            Spatial::set_marker_stationary(MarkerType::QrCode, stationary);
        }

        // Anchor creation + persistence.
        Ui::hseparator();
        Ui::push_enabled(Spatial::is_requested(SpatialCapability::Anchor), None);
        if Ui::button("Create Anchor").press() {
            let head = Input::get_head();
            let at = head.position + head.get_forward() * 0.5;
            let anchor =
                SpatialEntity::create_anchor(Pose::new(at, Some(Quat::look_at(at, head.position, None))), None, None);
            if self.persist_new {
                anchor.persist();
            }
        }
        Ui::same_line();
        Ui::toggle("Persist", &mut self.persist_new).interact();

        // Unpersist works from just the id, so this also cleans up anchors from earlier sessions that haven't
        // been found again.
        Ui::push_enabled(!self.saved_ids.is_empty(), None);
        if Ui::button(format!("Unpersist Saved ({})", self.saved_ids.len())).press() {
            for id in self.saved_ids.drain() {
                SpatialEntity::unpersist_uuid(id);
            }
            save_saved_ids(&self.saved_ids);
        }
        Ui::pop_enabled();
        Ui::pop_enabled();
        Ui::window_end();

        // Draw every entity, and clean up caches for the ones that left the list this frame.
        for entity in SpatialEntity::all() {
            self.draw_entity(entity);
        }
        for entity in SpatialEntity::removed() {
            // Persisted entities only leave once they're gone from storage too, like a destroyed anchor, so their
            // saved id goes with them. Lookups that were never drawn have no Visual, but still carry their Guid.
            let mut saved_changed = false;
            if let Some(id) = entity.try_get_uuid()
                && !id.is_nil()
                && self.saved_ids.remove(&id)
            {
                saved_changed = true;
            }
            if let Some(vis) = self.visuals.remove(&entity)
                && !vis.uuid.is_nil()
                && self.saved_ids.remove(&vis.uuid)
            {
                saved_changed = true;
            }
            if saved_changed {
                save_saved_ids(&self.saved_ids);
            }
        }

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    /// Refresh the per-entity cache for anything expensive to fetch or build, when the entity's `Changed` flags say
    /// the data is new.
    fn update_entity(&mut self, entity: SpatialEntity) {
        let changed = entity.get_changed();
        let vis = self.visuals.entry(entity).or_default();

        // Polygon points allocate a copy on fetch, so they're cached, and refreshed only when Changed says there's
        // new data.
        if entity.has(SpatialComponent::Polygon)
            && (vis.polygon.is_empty() || changed.contains(SpatialComponent::Polygon))
        {
            if let Some((origin, polygon)) = entity.try_get_polygon() {
                vis.polygon_origin = origin;
                vis.polygon = polygon;
            } else {
                vis.polygon.clear();
            }
        }

        // Anchors draw as an axis gizmo, with their persist id when stored.
        if entity.has(SpatialComponent::Anchor)
            && (vis.anchor_label.is_empty() || changed.contains(SpatialComponent::Persistence))
        {
            let uuid = entity.try_get_uuid().unwrap_or_default();
            vis.anchor_label = if uuid.is_nil() { "anchor".to_string() } else { format!("{uuid}")[..8].to_string() };
            // Named anchors are kept by StereoKit's name store, so the file skips them.
            if entity.try_get_name().is_some() {
                if !uuid.is_nil() && self.saved_ids.remove(&uuid) {
                    save_saved_ids(&self.saved_ids);
                }
            } else if vis.uuid != uuid {
                // Track the id whenever it lands, so "Unpersist Saved" can clean it up later, even in a later session.
                if !vis.uuid.is_nil() {
                    self.saved_ids.remove(&vis.uuid);
                }
                if !uuid.is_nil() {
                    self.saved_ids.insert(uuid);
                    save_saved_ids(&self.saved_ids);
                }
                vis.uuid = uuid;
            }
        }

        // A text label: decoded marker data, marker id, semantic label, or plane alignment for devices that don't
        // provide labels.
        if vis.name.is_empty()
            || changed.intersects(SpatialComponent::Marker | SpatialComponent::Label | SpatialComponent::PlaneAlignment)
        {
            vis.name = if let Some((_marker_type, marker_id)) = entity.try_get_marker() {
                entity.get_marker_text().unwrap_or_else(|| format!("Marker #{marker_id}"))
            } else if let Some(label) = entity.try_get_label().filter(|label| *label != SpatialLabel::None) {
                format!("{label:?}")
            } else if let Some(alignment) =
                entity.try_get_plane_align().filter(|alignment| *alignment != PlaneAlign::None)
            {
                format!("{alignment:?}")
            } else {
                String::new()
            };
        }

        // Wireframe entity meshes, refilled only when the data changes.
        if let Some(mesh) = vis.mesh.as_mut() {
            // Pass None to fetch just the origin, or the mesh to refill it with new data.
            let refill = changed.contains(SpatialComponent::Mesh).then_some(mesh.as_ref());
            if let Some(origin) = entity.try_get_mesh(refill) {
                vis.mesh_origin = origin;
            }
        } else if entity.has(SpatialComponent::Mesh) {
            let mesh = Mesh::new();
            if let Some(origin) = entity.try_get_mesh(Some(&mesh)) {
                vis.mesh_origin = origin;
                vis.mesh = Some(mesh);
            }
        }
    }

    /// Draw an entity's bounds, polygon, anchor gizmo, label and mesh.
    fn draw_entity(&mut self, entity: SpatialEntity) {
        self.update_entity(entity);

        let Some(vis) = self.visuals.get(&entity) else { return };
        let tracked = entity.get_tracked().is_active();
        let label = entity.try_get_label().unwrap_or(SpatialLabel::None);
        let color = if tracked { label_color(label) } else { GRAY };

        let mut rect = None;
        if let Some((center, size)) = entity.try_get_bounds2d() {
            draw_rect(center, size, color);
            Lines::add_axis(center, Some(0.05), None);
            rect = Some(center);
        }
        if let Some((center, size)) = entity.try_get_bounds3d() {
            draw_box(center, size, color);
        }

        // The boundary polygon outlining the entity's surface.
        if vis.polygon.len() > 1 {
            for i in 0..vis.polygon.len() {
                let a = vis.polygon_origin.position + vis.polygon_origin.orientation * vis.polygon[i].xy0();
                let b = vis.polygon_origin.position
                    + vis.polygon_origin.orientation * vis.polygon[(i + 1) % vis.polygon.len()].xy0();
                Lines::add(a, b, color, None, 0.003);
            }
        }

        // Anchors draw as an axis gizmo, with their persist id when stored.
        if entity.has(SpatialComponent::Anchor) {
            let anchor_pose = entity.get_pose();
            Lines::add_axis(anchor_pose, Some(0.1), None);
            TextBuilder::new(&vis.anchor_label)
                .transform(anchor_pose.to_matrix(None))
                .position(Pivot::TopCenter)
                .add();
        }

        // The entity's name, drawn over its 2D bounds when it has any.
        if !vis.name.is_empty()
            && let Some(center) = rect
        {
            TextBuilder::new(&vis.name).transform(center.to_matrix(None)).add();
        }

        // Wireframe entity meshes, drawn at their origin pose.
        if self.show_meshes
            && entity.has(SpatialComponent::Mesh)
            && let Some(mesh) = &vis.mesh
        {
            mesh.draw(&self.mesh_mat, vis.mesh_origin.to_matrix(None), None, None);
        }
    }
}

/// Color code for each semantic label, grey when the entity isn't tracked.
fn label_color(label: SpatialLabel) -> Color32 {
    match label {
        SpatialLabel::Floor => GREEN,
        SpatialLabel::Wall => BLUE,
        SpatialLabel::Ceiling => RED,
        SpatialLabel::Table => YELLOW,
        _ => WHITE,
    }
}

/// Outlines a 2D rectangle pose with lines.
fn draw_rect(pose: Pose, size: Vec2, color: Color32) {
    let right = pose.orientation * Vec3::new(size.x * 0.5, 0.0, 0.0);
    let up = pose.orientation * Vec3::new(0.0, size.y * 0.5, 0.0);
    let p = pose.position;
    Lines::add(p - right - up, p + right - up, color, None, 0.005);
    Lines::add(p + right - up, p + right + up, color, None, 0.005);
    Lines::add(p + right + up, p - right + up, color, None, 0.005);
    Lines::add(p - right + up, p - right - up, color, None, 0.005);
}

/// Outlines an oriented 3D box with lines.
fn draw_box(pose: Pose, size: Vec3, color: Color32) {
    let h = size * 0.5;
    let mut corners = [Vec3::ZERO; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        *corner = pose.position
            + pose.orientation
                * Vec3::new(
                    if (i & 1) != 0 { h.x } else { -h.x },
                    if (i & 2) != 0 { h.y } else { -h.y },
                    if (i & 4) != 0 { h.z } else { -h.z },
                );
    }
    const EDGES: [(usize, usize); 12] =
        [(0, 1), (1, 3), (3, 2), (2, 0), (4, 5), (5, 7), (7, 6), (6, 4), (0, 4), (1, 5), (2, 6), (3, 7)];
    for (a, b) in EDGES {
        Lines::add(corners[a], corners[b], color, None, 0.005);
    }
}

fn load_saved_ids() -> HashSet<Uuid> {
    std::fs::read_to_string(SAVED_IDS_FILE)
        .map(|content| content.lines().filter_map(parse_uuid).collect())
        .unwrap_or_default()
}

fn save_saved_ids(ids: &HashSet<Uuid>) {
    let content = ids.iter().map(|id| format!("{id}")).collect::<Vec<_>>().join("\n");
    let _ = std::fs::write(SAVED_IDS_FILE, content);
}

/// Parses a Uuid back from its `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` text form.
fn parse_uuid(text: &str) -> Option<Uuid> {
    let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(Uuid::from_bytes(bytes))
}
