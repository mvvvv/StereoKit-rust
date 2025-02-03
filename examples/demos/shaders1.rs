use stereokit_rust::{
    font::Font,
    material::{Cull, Material},
    maths::{Matrix, Pose, Quat, Vec2, Vec3, Vec4},
    mesh::{Mesh, Vertex},
    prelude::*,
    shader::Shader,
    system::{Text, TextStyle},
    tex::Tex,
    ui::{Ui, UiMove, UiWin},
    util::{
        named_colors::{BLUE, GREEN, LIGHT_BLUE, RED, WHITE},
        Time,
    },
};

/// IStepper implementation for Shader1
#[derive(IStepper)]
pub struct Shader1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform_mesh: Matrix,
    pub transform_plane: Matrix,
    pub pose_progress: Pose,
    material_red: Material,
    material_green: Material,
    water2: Material,
    mesh: Mesh,
    plane: Mesh,
    pub transform_text: Matrix,
    pub transform_water2: Matrix,
    text: String,
    text_style: TextStyle,
    fps: f64,
}

unsafe impl Send for Shader1 {}

impl Default for Shader1 {
    fn default() -> Self {
        //------ Materials
        let hud_text_shader = Shader::from_file("shaders/hud_text.hlsl.sks").unwrap();
        let text_style = Text::make_style_with_shader(Font::default(), 0.03, hud_text_shader, RED);

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
            .face_cull(Cull::Back)
            .color_tint(LIGHT_BLUE)
            .time(5.0);

        //---- Transform Matrices.
        let transform_mesh = Matrix::trs(
            &((Vec3::NEG_Z * 1.0) + Vec3::X + Vec3::Y * 1.4),
            &Quat::from_angles(90.0, 0.0, 0.0),
            &(Vec3::ONE * 0.3),
        );

        let transform_plane = Matrix::tr(&(Vec3::new(0.2, 1.2, -1.0)), &Quat::from_angles(90.0, 0.0, 0.0));
        let pose_progress = Pose::new(Vec3::new(0.1, 1.5, -1.0), Some(Quat::from_angles(0.0, 180.0, 0.0)));

        let transform_water2 =
            Matrix::tr(&((Vec3::NEG_Z * 1.0) + Vec3::X * 0.2 + Vec3::Y * 0.2), &Quat::from_angles(0.0, 180.0, 0.0));

        let transform_text = Matrix::tr(&(Vec3::ONE * -0.2), &Quat::from_angles(0.0, 180.0, 0.0));

        //----- Meshes
        let vertices = [
            Vertex { pos: Vec3::X, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 1.0 }, col: BLUE },
            Vertex { pos: Vec3::NEG_X, norm: Vec3::Y, uv: Vec2 { x: 0.0, y: 1.0 }, col: RED },
            Vertex { pos: Vec3::Z, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 0.50 }, col: GREEN },
        ];
        let indices = [0, 1, 2, 2, 1, 0];

        let mut mesh = Mesh::new();
        mesh.id("mesh1").keep_data(true).set_data(&vertices, &indices, true);

        let mut plane = Mesh::generate_plane_up(Vec2::new(0.5, 0.5), None, true);
        plane.id("plane1");

        Self {
            id: "Shader1".to_string(),
            sk_info: None,
            transform_mesh,
            transform_plane,
            pose_progress,
            transform_water2,
            material_red: blinker_material,
            material_green,
            water2,
            mesh,
            plane,
            transform_text,
            text: "Shader1".to_owned(),
            text_style,
            fps: 0.0,
        }
    }
}

impl Shader1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
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
            .set_vec4("tex_trans", tex_transform)
            .set_int("do_not_exist", &[1, 3, 5, 6])
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

        self.fps = ((1.0 / Time::get_step()) + self.fps) / 2.0;

        Text::add_at(
            token,
            format!("{}\n{:?} FPS", &self.text, self.fps as i16),
            self.transform_text,
            Some(self.text_style),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }
}
