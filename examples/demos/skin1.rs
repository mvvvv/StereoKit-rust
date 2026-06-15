use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec2, Vec4},
    mesh::{Mesh, Vertex},
    prelude::*,
    system::{Text, TextBuilder, TextStyle},
    util::{
        self, Time,
        named_colors::{CYAN, ORANGE, WHITE, YELLOW},
    },
};

// Half-height of the ribbon mesh (the joint is at this height).
const JOINT_Y: f32 = 0.20;
// Total height of the ribbon.
const MESH_H: f32 = 0.40;
// Half-width.
const MESH_W: f32 = 0.06;
// Depth.
const MESH_D: f32 = 0.015;
// Maximum bend angle for the upper bone (degrees).
const MAX_ANGLE: f32 = 38.0;

/// Builds the source ribbon mesh and attaches skin data.
///
/// Topology: 2 columns × 5 rows = 10 front + 10 back vertices.
/// The mesh is flat, facing +Z (front) and –Z (back) so it's visible from both sides.
///
/// Bone assignment:
/// * bone 0 – lower half  (rows 0–2,  y ≤ JOINT_Y) – resting = Identity
/// * bone 1 – upper half  (rows 2–4,  y ≥ JOINT_Y) – resting = Matrix::t([0, JOINT_Y, 0])
///
/// Row 2 (the joint row) blends 50/50 between the two bones.
fn build_skin_mesh() -> Mesh {
    // ------------------------------------------------------------------
    // 1.  Geometry
    // ------------------------------------------------------------------
    let rows: usize = 5;
    let cols: usize = 2;
    let n = rows * cols; // 10 front + 10 back

    let xs = [-MESH_W, MESH_W];
    let ys: Vec<f32> = (0..rows).map(|r| r as f32 * MESH_H / (rows as f32 - 1.0)).collect();

    let front_norm = [0.0_f32, 0.0, 1.0];
    let back_norm = [0.0_f32, 0.0, -1.0];

    let mut verts: Vec<Vertex> = Vec::with_capacity(n * 2);

    // Colour gradient from blue (bottom) to yellow (top).
    let lerp_col = |t: f32| -> util::Color32 {
        let r = (60.0 + (255.0 - 60.0) * t) as u8;
        let g = (160.0 + (220.0 - 160.0) * t) as u8;
        let b = (255.0 + (60.0 - 255.0) * t) as u8;
        util::Color32::rgba(r, g, b, 255)
    };

    // Front face
    for &y in &ys {
        let t = y / MESH_H;
        let u0 = 0.0_f32;
        let u1 = 1.0_f32;
        let v = t;
        let col = lerp_col(t);
        verts.push(Vertex::new([xs[0], y, MESH_D * 0.5], front_norm, Some(Vec2::new(u0, v)), Some(col)));
        verts.push(Vertex::new([xs[1], y, MESH_D * 0.5], front_norm, Some(Vec2::new(u1, v)), Some(col)));
    }
    // Back face (same positions, flipped normals, flipped U)
    for &y in &ys {
        let t = y / MESH_H;
        let col = lerp_col(t);
        verts.push(Vertex::new([xs[1], y, -MESH_D * 0.5], back_norm, Some(Vec2::new(0.0, t)), Some(col)));
        verts.push(Vertex::new([xs[0], y, -MESH_D * 0.5], back_norm, Some(Vec2::new(1.0, t)), Some(col)));
    }

    // Indices — front and back quads
    let mut inds: Vec<u32> = Vec::new();
    for face in 0..2_u32 {
        let base = face * (n as u32);
        for row in 0..(rows as u32 - 1) {
            let tl = base + row * cols as u32;
            let tr = tl + 1;
            let bl = tl + cols as u32;
            let br = bl + 1;
            inds.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    // Side strips joining front and back edges.
    //
    // Front layout: row r → vi = r*2 (left, x=−w) and r*2+1 (right, x=+w),  z=+d
    // Back layout:  row r → vi = n+r*2 (x=+w, z=−d) and n+r*2+1 (x=−w, z=−d)
    //
    // Left side  (x = −MESH_W): front col=0  ↔  back col=1
    // Right side (x = +MESH_W): front col=1  ↔  back col=0
    for row in 0..(rows as u32 - 1) {
        // left side
        let fl0 = row * 2;
        let fl1 = (row + 1) * 2;
        let bl0 = n as u32 + row * 2 + 1;
        let bl1 = n as u32 + (row + 1) * 2 + 1;
        inds.extend_from_slice(&[fl0, fl1, bl1, fl0, bl1, bl0]);

        // right side
        let fr0 = row * 2 + 1;
        let fr1 = (row + 1) * 2 + 1;
        let br0 = n as u32 + row * 2;
        let br1 = n as u32 + (row + 1) * 2;
        inds.extend_from_slice(&[fr0, br0, br1, fr0, br1, fr1]);
    }

    let mut mesh = Mesh::new();
    mesh.set_data(&verts, &inds, None, None);

    // ------------------------------------------------------------------
    // 2.  Skin data
    // ------------------------------------------------------------------
    // bone_ids: 4 bone-id shorts per vertex (only the first two matter here)
    // bone_weights: 1 Vec4 per vertex (components sum to ~1)
    let total_verts = verts.len();
    let mut bone_ids: Vec<u16> = vec![0u16; total_verts * 4];
    let mut bone_weights: Vec<Vec4> = vec![Vec4::new(1.0, 0.0, 0.0, 0.0); total_verts];

    // Helper: assign weights for a given vertex index based on its y position.
    let assign = |vi: usize, y: f32, bone_ids: &mut Vec<u16>, bone_weights: &mut Vec<Vec4>| {
        if y <= JOINT_Y - 0.001 {
            // Fully bone 0
            bone_ids[vi * 4] = 0;
            bone_ids[vi * 4 + 1] = 1;
            bone_weights[vi] = Vec4::new(1.0, 0.0, 0.0, 0.0);
        } else if y >= JOINT_Y + 0.001 {
            // Fully bone 1
            bone_ids[vi * 4] = 1;
            bone_ids[vi * 4 + 1] = 0;
            bone_weights[vi] = Vec4::new(1.0, 0.0, 0.0, 0.0);
        } else {
            // Joint row — 50/50 blend
            bone_ids[vi * 4] = 0;
            bone_ids[vi * 4 + 1] = 1;
            bone_weights[vi] = Vec4::new(0.5, 0.5, 0.0, 0.0);
        }
    };

    // Front face vertices (indices 0..n)
    for (row, &y) in ys.iter().enumerate() {
        for col in 0..cols {
            let vi = row * cols + col;
            assign(vi, y, &mut bone_ids, &mut bone_weights);
        }
    }
    // Back face vertices (indices n..2n) — same y positions
    for (row, &y) in ys.iter().enumerate() {
        for col in 0..cols {
            let vi = n + row * cols + col;
            assign(vi, y, &mut bone_ids, &mut bone_weights);
        }
    }

    // Resting transforms: bone 0 at origin, bone 1 pivot at joint height.
    let resting = [Matrix::IDENTITY, Matrix::t([0.0, JOINT_Y, 0.0])];

    mesh.set_skin(&bone_ids, &bone_weights, &resting);
    mesh
}

/// Skin1 — CPU Mesh Skinning demo
///
/// Shows three independent deforming ribbons, each a [`Mesh::copy`] of the same skinned source. They oscillate at
/// different phases, driven each frame by [`Mesh::update_skin`].
#[derive(IStepper)]
pub struct Skin1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    /// Three independent deforming copies, each with its own vertex buffer.
    copies: [Mesh; 3],
    text_copies: [TextBuilder; 3],
    material: Material,

    pub transform: Matrix,
    pub text: String,
    text_style: TextStyle,
}

unsafe impl Send for Skin1 {}

impl Default for Skin1 {
    fn default() -> Self {
        let src = build_skin_mesh();

        // Each copy gets its own independent vertex buffer — UpdateSkin mutates
        // the target in place, so sharing a buffer would corrupt one copy.
        let copies = [src.copy(), src.copy(), src.copy()];

        let mut material = Material::unlit().copy();
        material.face_cull(stereokit_rust::material::Cull::None);

        Self {
            id: "Skin1".to_string(),
            sk_info: None,

            copies,
            text_copies: [TextBuilder::new("Copy A"), TextBuilder::new("Copy B"), TextBuilder::new("Copy C")],
            material,

            transform: Matrix::t_r([0.0, 0.0, -0.5], Quat::Y_180),
            text: "Skin1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.04, WHITE),
        }
    }
}

impl Skin1 {
    fn start(&mut self) -> bool {
        for (i, color) in [CYAN, YELLOW, ORANGE].iter().enumerate() {
            let style = Text::make_style(Font::default(), 0.025, *color);
            self.text_copies[i].update_style(style);
        }
        true
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, token: &MainThreadToken) {
        let t = Time::get_totalf();

        // Three copies side by side, each with a phase-shifted oscillation.
        let offsets_x = [-0.22_f32, 0.0, 0.22];
        let phases = [0.0_f32, std::f32::consts::FRAC_PI_3 * 2.0, std::f32::consts::FRAC_PI_3 * 4.0];
        let speed = 1.8_f32;

        for (i, mesh) in self.copies.iter_mut().enumerate() {
            let angle = MAX_ANGLE * (t * speed + phases[i]).sin();
            let bend_rot = Quat::from_angles(0.0, 0.0, angle);

            // bone 0: stays at origin (lower half, fixed)
            // bone 1: rotates around the joint (upper half, bends)
            let palette = [Matrix::IDENTITY, Matrix::t_r([0.0, JOINT_Y, 0.0], bend_rot)];
            mesh.update_skin(&palette);

            // Position each ribbon in a row
            let world = Matrix::t([offsets_x[i], 1.5, -0.55]);
            mesh.draw(token, &self.material, world, None, None);

            // Small label under each ribbon
            self.text_copies[i]
                .update_transform(Matrix::t_r([offsets_x[i], 1.5 - MESH_H * 0.15, -0.55], Quat::Y_180))
                .add();
        }

        // Title
        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }
}
