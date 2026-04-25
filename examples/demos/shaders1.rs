use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, Vec4},
    mesh::{Mesh, Vertex},
    prelude::*,
    system::{OcclusionCaps, Text, World},
    tex::Tex,
    tools::notif::HudNotification,
    ui::{Ui, UiMove, UiWin},
    util::{
        Time,
        named_colors::{BLUE, GREEN, LIGHT_BLUE, RED, WHITE},
    },
};

/// IStepper implementation for Shader1
#[derive(IStepper)]
pub struct Shaders1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub transform_mesh: Matrix,
    pub transform_plane: Matrix,
    pub transform_water2: Matrix,
    pub transform_brick: Matrix,
    pub pose_progress: Pose,
    material_red: Material,
    material_green: Material,
    water2: Material,
    brick: Material,
    mesh: Mesh,
    plane: Mesh,
    pub transform_text: Matrix,
    notif: HudNotification,
    fps: f64,
}

unsafe impl Send for Shaders1 {}

impl Default for Shaders1 {
    fn default() -> Self {
        let mut notif = HudNotification::default();
        notif.duration = None;
        notif.position = Vec3::new(-0.03, -0.03, -0.3);
        notif.text_style = Text::make_style(Font::default(), 0.01, GREEN);

        //------ Materials
        let mut blinker_material =
            Material::from_file("shaders/blinker.hlsl.sks", Some("red_material")).unwrap_or_default();
        blinker_material
            .diffuse_tex(Tex::from_file("textures/open_gltf.jpeg", true, None).unwrap_or_default())
            .tex_transform(Vec4::new(0.0, 0.0, 4.0, 4.0))
            .color_tint(WHITE);

        let mut material_green = blinker_material.copy();
        material_green
            .id("green_material")
            .tex_transform(Vec4::new(0.0, 0.0, 4.0, 4.0))
            .color_tint(GREEN)
            .time(10.0);

        // fresh water
        let bump_tex = Tex::from_file("textures/water/bump_large.ktx2", true, None).unwrap();

        let mut water2 =
            Material::from_file("shaders/water_pbr2.hlsl.sks", "water_pbr2_s".into()).unwrap_or_default().copy();
        water2
            .normal_tex(&bump_tex)
            .tex_transform(Vec4::new(0.0, 0.0, 2.0, 2.0))
            .roughness_amount(0.4)
            .metallic_amount(0.6)
            .color_tint(LIGHT_BLUE)
            .time(5.0);

        // brick
        let mut brick = Material::from_file("shaders/brick_pbr.hlsl.sks", "brick".into()).unwrap_or_default().copy();
        brick
            .tex_transform(Vec4::new(0.0, 0.0, 0.04, 0.04))
            .get_all_param_info()
            .set_bool("use_occlusion", true);

        //---- Transform Matrices.
        let transform_mesh = Matrix::t_r_s(
            (Vec3::NEG_Z * 1.0) + Vec3::X + Vec3::Y * 1.4,
            Quat::from_angles(90.0, 0.0, 0.0),
            Vec3::ONE * 0.3,
        );

        let transform_plane = Matrix::t_r(Vec3::new(0.2, 1.2, -1.0), Quat::from_angles(90.0, 0.0, 0.0));
        let pose_progress = Pose::new(Vec3::new(0.1, 1.5, -1.0), Some(Quat::from_angles(0.0, 180.0, 0.0)));

        let transform_water2 =
            Matrix::t_r((Vec3::NEG_Z * 1.0) + Vec3::X * 0.2 + Vec3::Y * 0.2, Quat::from_angles(0.0, 180.0, 0.0));

        let transform_brick =
            Matrix::t_r((Vec3::NEG_Z * 1.0) + Vec3::X * 1.5 + Vec3::Y * 0.2, Quat::from_angles(0.0, 180.0, 0.0));

        let transform_text = Matrix::t_r(Vec3::ONE * -0.2, Quat::from_angles(0.0, 180.0, 0.0));

        //----- Meshes
        let vertices = [
            Vertex { pos: Vec3::X, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 1.0 }, col: BLUE },
            Vertex { pos: Vec3::NEG_X, norm: Vec3::Y, uv: Vec2 { x: 0.0, y: 1.0 }, col: RED },
            Vertex { pos: Vec3::Z, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 0.50 }, col: GREEN },
        ];
        let indices = [0, 1, 2, 2, 1, 0];

        let mut mesh = Mesh::new();
        mesh.id("mesh1").keep_data(true).set_data(&vertices, &indices, None, None);

        let mut plane = Mesh::generate_plane_up(Vec2::new(0.5, 0.5), None, true);
        plane.id("plane1");

        Self {
            id: "Shader1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            transform_mesh,
            transform_plane,
            transform_water2,
            transform_brick,
            pose_progress,
            material_red: blinker_material,
            material_green,
            water2,
            brick,
            mesh,
            plane,
            transform_text,
            notif,
            fps: 0.0,
        }
    }
}

impl Shaders1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        World::occlusion(stereokit_rust::system::OcclusionCaps::Mesh);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI
    fn draw(&mut self, token: &MainThreadToken) {
        self.mesh.draw(token, &self.material_red, self.transform_mesh, None, None);

        let total_scale = (Time::get_totalf() % 360.0).to_radians().sin().abs() * 2.0;
        let tex_transform = Vec4::new(0.0, 0.0, total_scale, total_scale);
        let mut param_info = self.material_green.get_all_param_info();
        param_info
            .set_vector4("tex_trans", tex_transform)
            //.set_int("do_not_exist", &[1, 3, 5, 6])
            .set_float("time", total_scale);
        self.plane.draw(token, &self.material_green, self.transform_plane, None, None);

        Ui::window_begin(
            "progress",
            &mut self.pose_progress,
            Some(Vec2::new(0.41, 0.1)),
            Some(UiWin::Empty),
            Some(UiMove::None),
        );
        //Ui::progress_bar_at(total_scale / 2.0, Vec3::new(0.0, 0.0, 0.0), Vec2::new(0.4, 0.1), UiDir::Horizontal, false);
        Ui::hprogress_bar(total_scale / 2.0, 0.54, false);
        Ui::vprogress_bar(total_scale / 2.0, 0.50, false);
        Ui::window_end();
        self.mesh.draw(token, &self.water2, self.transform_water2, None, None);
        self.plane.draw(token, &self.brick, self.transform_brick, None, None);

        self.fps = ((1.0 / Time::get_step()) + self.fps) / 2.0;

        self.notif.text = format!("Shader1\nFPS: {:.0}", self.fps);
        self.notif.draw(token);
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            World::occlusion(OcclusionCaps::None);
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }
}
