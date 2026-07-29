use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Rect, Vec2, Vec3},
    mesh::Mesh,
    model::Model,
    prelude::*,
    render::{RenderBuilder, RenderClear, RenderList, RenderListRefs, Renderer},
    system::{Assets, Text, TextBuilder, TextStyle},
    tex::{Tex, TexFormat},
    ui::Ui,
    util::{
        Color128, Time,
        named_colors::{BLUE_VIOLET, RED},
    },
};

/// The RenderList1 stepper
#[derive(IStepper)]
pub struct RenderList1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub window_pose: Pose,
    primary: RenderList,
    list: RenderList,
    render_mat: Material,
    render_tex_a: Tex,
    render_tex_b: Tex,
    flip: u8,
    quad: Mesh,
    old_clear_color: Color128,
    camera_pos: Vec3,
    perspective: Matrix,
    clear_primary: bool,
    enable_fx: bool,
    fx1: Material,
    fx2: Material,

    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

impl Default for RenderList1 {
    fn default() -> Self {
        let quad = Mesh::screen_quad();
        let mut list = RenderList::new_with(RenderListRefs::Tracked);
        list.id("PlaneList");

        //let render_tex = Tex::gen_color(BLUE_VIOLET, 128, 128, TexType::Rendertarget, TexFormat::Rgba32Srgb);
        let render_tex_a =
            Tex::render_target(128, 128, None, TexFormat::Rgba32Srgb, TexFormat::Depth16).unwrap_or_default();
        let render_tex_b =
            Tex::render_target(128, 128, None, TexFormat::Rgba32Srgb, TexFormat::Depth16).unwrap_or_default();
        let mut render_mat = Material::pbr().copy();
        let model = Model::from_file("plane.glb", None, None).unwrap_or_default();

        let transform_model = Matrix::r(Quat::from_angles(90.0, 90.0, 45.0));
        let material_quad = Material::pbr();

        Assets::block_for_priority(i32::MAX);
        list.add_model(model.copy(), transform_model, Color128::WHITE, None);
        list.add_mesh(&quad, &material_quad, Matrix::IDENTITY, BLUE_VIOLET, None);

        Assets::block_for_priority(i32::MAX);
        let camera_pos = Vec3::new(-2.0, 1.0, -10.9);

        render_mat.face_cull(stereokit_rust::material::Cull::None);

        let perspective = Matrix::perspective(90.0, 1.0, 0.1, 50.0);

        let fx1 = Material::from_file("shaders/subpass/night_vision.hlsl.sks", None).unwrap_or_default();
        let fx2 = Material::from_file("shaders/subpass/dynamic_cloud.hlsl.sks", None).unwrap_or_default();
        Self {
            id: "RenderList1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            primary: RenderList::primary(),
            list,
            clear_primary: false,
            enable_fx: true,
            render_mat,
            render_tex_a,
            render_tex_b,
            flip: 0,
            quad,
            old_clear_color: Color128::BLACK_TRANSPARENT,
            camera_pos,
            perspective,
            fx1,
            fx2,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            text: "RenderList1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

unsafe impl Send for RenderList1 {}

impl RenderList1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.old_clear_color = Renderer::get_clear_color();
        Renderer::clear_color(Color128::hsv(0.4, 0.3, 0.5, 1.0));
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        if self.clear_primary {
            self.primary.clear();
        }

        let fx = if self.enable_fx { vec![&self.fx1, &self.fx2] } else { vec![] };
        let render = RenderBuilder::new()
            .camera(Matrix::look_at(self.camera_pos, Vec3::ZERO, Some(Vec3::new(1.0, Time::get_totalf().sin(), 1.0))))
            .projection(self.perspective)
            .clear(RenderClear::All)
            .viewport(Rect::new(0.0, 0.0, 1.0, 1.0))
            .post_process(fx);

        let (read_tex, write_tex) = if self.flip == 1 {
            self.flip = 2;
            (&self.render_tex_a, &self.render_tex_b)
        } else if self.flip == 2 {
            self.flip = 1;
            (&self.render_tex_b, &self.render_tex_a)
        } else {
            // We are here for the first step only to render render_tex_a for next step.
            self.flip = 1;
            render.draw_now(&self.list, &self.render_tex_a, Color128::WHITE);
            return;
        };

        self.render_mat.diffuse_tex(read_tex);
        render.draw_now(&self.list, write_tex, Color128::new(0.4, 0.3, 0.2, 0.5));

        Ui::window("Render Lists").pose(&mut self.window_pose).size(Vec2::new(0.23, 0.35)).begin();
        Ui::label(format!("Render items: {}/{}", self.primary.get_count(), self.primary.get_prev_count()))
            .use_padding(true)
            .draw();
        if let Some(value) = Ui::toggle("Clear", &mut self.clear_primary).interact() {
            if value {
                self.perspective = Matrix::perspective_focal(Vec2::ONE * 2048.0, 1500.0, 0.01, 1010.0)
            } else {
                self.perspective = Matrix::perspective(90.0, 1.0, 0.01, 1010.0)
            }
        };
        Ui::same_line();
        Ui::toggle("Fx", &mut self.enable_fx).interact();
        Ui::label("Offscreen List:").use_padding(true).draw();
        let b = Ui::layout_reserve(Vec2::new(0.1, 0.1), false, 0.0);
        self.quad.draw(
            &self.render_mat,
            Matrix::t_s(b.center + Vec3::new(-0.05, -0.05, -0.004), b.dimensions.xy1()),
            None,
            None,
        );
        Ui::window_end();

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            Renderer::clear_color(self.old_clear_color);
            self.shutdown_completed = true;
            true
        } else {
            self.shutdown_completed
        }
    }
}
