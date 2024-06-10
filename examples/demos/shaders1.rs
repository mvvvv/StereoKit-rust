use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    event_loop::{IStepper, StepperId},
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec2, Vec3, Vec4},
    mesh::{Mesh, Vertex},
    shader::Shader,
    sk::{MainThreadToken, SkInfo},
    system::{Text, TextStyle},
    tex::Tex,
    util::{
        named_colors::{BLUE, GREEN, RED, WHITE},
        Time,
    },
};

pub struct Shader1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform_mesh: Matrix,
    pub transform_plane: Matrix,
    material_red: Material,
    material_green: Material,
    mesh: Mesh,
    plane: Mesh,
    pub transform_text: Matrix,
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

        let mut material_green = Material::copy(&blinker_material);
        material_green
            .id("green_material")
            .tex_transform(Vec4::new(0.0, 0.0, 2.0, 2.0))
            .color_tint(GREEN)
            .time(10.0);

        //---- Transform Matrices.
        let transform_mesh = Matrix::trs(
            &((Vec3::NEG_Z * 1.0) + Vec3::X + Vec3::Y * 1.4),
            &Quat::from_angles(90.0, 0.0, 0.0),
            &(Vec3::ONE * 0.3),
        );

        let transform_plane =
            Matrix::tr(&((Vec3::NEG_Z * 1.0) + Vec3::X * 0.2 + Vec3::Y * 1.2), &Quat::from_angles(90.0, 0.0, 0.0));

        let transform_text = Matrix::tr(&(Vec3::ONE * -0.2), &Quat::from_angles(0.0, 180.0, 0.0));

        //----- Meshes
        let vertices = [
            Vertex { pos: Vec3::X, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 1.0 }, col: BLUE },
            Vertex { pos: Vec3::NEG_X, norm: Vec3::Y, uv: Vec2 { x: 0.0, y: 1.0 }, col: RED },
            Vertex { pos: Vec3::Z, norm: Vec3::Y, uv: Vec2 { x: 1.0, y: 0.50 }, col: GREEN },
        ];
        let indices = [0, 1, 2, 2, 1, 0];

        let mut mesh = Mesh::new();
        mesh.id("mesh1").keep_data(true).set_data(&vertices, &indices, None);

        let mut plane = Mesh::generate_plane_up(Vec2::new(0.5, 0.5), None, true);
        plane.id("plane1");

        Self {
            id: "Shader1".to_string(),
            sk_info: None,
            transform_mesh,
            transform_plane,
            material_red: blinker_material,
            material_green,
            mesh,
            plane,
            transform_text,
            text: "Shader1".to_owned(),
            text_style,
            fps: 0.0,
        }
    }
}

impl IStepper for Shader1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl Shader1 {
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
