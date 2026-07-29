use stereokit_rust::{
    material::Material,
    maths::{Matrix, Quat, Vec2, Vec3},
    mesh::{Mesh, MeshData, VertComponent, VertFmt, VertSemantic, Vertex, VertexLayout},
    prelude::*,
    system::{OcclusionCaps, World},
    util::{
        Color32,
        named_colors::{BLUE, GREEN, RED},
    },
};

#[derive(Default, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
struct CustomVertex {
    pos: Vec3,
    col: Color32,
}

impl VertexLayout for CustomVertex {
    const COMPONENTS: &'static [VertComponent] = &[
        VertComponent::new(VertSemantic::Position, VertFmt::F32, 3, 0),
        VertComponent::new(VertSemantic::Color, VertFmt::U8Normalized, 4, 0),
    ];
}

/// IStepper implementation for Shader1
#[derive(IStepper)]
pub struct Shaders2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub transform_mesh: Matrix,
    pub transform_plane: Matrix,

    material_red: Material,
    material_green: Material,
    mesh1: Mesh,
    mesh2: Mesh,
}

unsafe impl Send for Shaders2 {}

impl Default for Shaders2 {
    fn default() -> Self {
        //------ Materials
        let mut material_green =
            Material::from_file("shaders/vert_custom.hlsl.sks", "green".into()).unwrap_or_default().copy();
        material_green.id("green_material").color_tint(GREEN);

        let mut material_red =
            Material::from_file("shaders/vert_custom.hlsl.sks", "red".into()).unwrap_or_default().copy();
        material_red.id("red_material").color_tint(RED);

        //---- Transform Matrices.
        let transform_mesh1 = Matrix::t_r_s(
            (Vec3::NEG_Z * 1.0) + Vec3::X + Vec3::Y * 1.4,
            Quat::from_angles(90.0, 0.0, 0.0),
            Vec3::ONE * 0.3,
        );

        let transform_plane =
            Matrix::t_r_s(Vec3::new(0.2, 1.4, -1.0), Quat::from_angles(90.0, 0.0, 0.0), Vec3::ONE * 0.3);

        //----- Meshes
        // 1 - regularly formatted
        let vertices = [
            Vertex { pos: Vec3::X, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 1.0 }, col: BLUE },
            Vertex { pos: Vec3::NEG_X, norm: Vec3::Y, uv: Vec2 { x: 0.0, y: 1.0 }, col: RED },
            Vertex { pos: Vec3::Z, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 0.50 }, col: GREEN },
        ];
        let indices = [0, 1, 2, 2, 1, 0];

        let mut mesh1 = Mesh::new();
        mesh1.id("mesh1").keep_data(true).set_data(&vertices, &indices, None, None);

        // 2 - using set_data_fmt
        let custom_vertices = [
            CustomVertex { pos: Vec3::X, col: BLUE },
            CustomVertex { pos: Vec3::NEG_X, col: RED },
            CustomVertex { pos: Vec3::Z, col: GREEN },
        ];
        let custom_indices = [0, 1, 2, 2, 1, 0];
        let mut mesh2 = Mesh::new();
        mesh2
            .id("mesh2")
            .keep_data(true)
            .set_data_fmt(&custom_vertices, &custom_indices, Some(MeshData::None), None);

        Self {
            id: "Shaders2".to_string(),
            sk_info: None,
            shutdown_completed: false,

            transform_mesh: transform_mesh1,
            transform_plane,
            material_red,
            material_green,
            mesh1,
            mesh2,
        }
    }
}

impl Shaders2 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        World::occlusion(stereokit_rust::system::OcclusionCaps::Mesh);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        self.mesh1.draw(&self.material_red, self.transform_mesh, None, None);

        self.mesh2.draw(&self.material_green, self.transform_plane, None, None);
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            World::occlusion(OcclusionCaps::None);
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }
}
