use std::{cell::RefCell, rc::Rc};

use stereokit_macros::IStepper;
use stereokit_rust::{
    event_loop::{IStepper, StepperAction, StepperId},
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Rect, Vec2, Vec3},
    mesh::Mesh,
    model::Model,
    render_list::RenderList,
    sk::{MainThreadToken, SkInfo},
    system::{Assets, RenderClear, Renderer, Text, TextStyle},
    tex::{Tex, TexFormat, TexType},
    ui::Ui,
    util::{
        named_colors::{BLUE_VIOLET, RED},
        Color128, Time,
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
    render_tex: Tex,
    old_clear_color: Color128,
    at: Vec3,
    quad: Mesh,
    perspective: Matrix,
    clear_primary: bool,
    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

impl Default for RenderList1 {
    fn default() -> Self {
        let quad = Mesh::screen_quad();
        let mut list = RenderList::new();
        list.id("PlaneList");
        let render_tex = Tex::gen_color(BLUE_VIOLET, 128, 128, TexType::Rendertarget, TexFormat::RGBA32);
        //let render_tex = Tex::render_target(128, 128, None, None, None).unwrap_or_default();
        let mut render_mat = Material::pbr().copy();
        let model = Model::from_file("plane.glb", None).unwrap();
        list.add_model(model, None, Matrix::r(Quat::from_angles(90.0, 90.0, 145.0)), Color128::WHITE, None);
        //list.add_mesh(&quad, &render_mat, Matrix::IDENTITY, BLUE_VIOLET, None);

        Assets::block_for_priority(i32::MAX);
        let at = Vec3::new(-2.0, 1.0, 1000.9);

        render_mat.diffuse_tex(&render_tex);
        render_mat.face_cull(stereokit_rust::material::Cull::None);

        let perspective = Matrix::perspective(90.0, 1.0, 0.01, 1010.0);
        Self {
            id: "RenderList1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            primary: RenderList::primary(),
            list,
            clear_primary: false,
            render_mat,
            render_tex,
            old_clear_color: Color128::BLACK_TRANSPARENT,
            at,
            quad,
            perspective,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
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
    fn draw(&mut self, token: &MainThreadToken) {
        if self.clear_primary {
            self.primary.clear()
        };

        self.list.draw_now(
            &self.render_tex,
            Matrix::look_at(self.at, Vec3::ZERO, Some(Vec3::new(1.0, Time::get_totalf().sin(), 1.0))),
            self.perspective,
            Some(Color128::new(0.4, 0.3, 0.2, 0.5)),
            Some(RenderClear::Color),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            None,
        );

        Ui::window_begin("Render Lists", &mut self.window_pose, Some(Vec2::new(0.23, 0.35)), None, None);
        Ui::label(format!("Render items: {}/{}", self.primary.get_count(), self.primary.get_prev_count()), None, true);
        if let Some(value) = Ui::toggle("Clear", self.clear_primary, None) {
            self.clear_primary = value;
            if value {
                self.perspective = Matrix::perspective_focal(Vec2::ONE * 2048.0, 100000.0, 0.01, 1010.0)
            } else {
                self.perspective = Matrix::perspective(90.0, 1.0, 0.01, 1010.0)
            }
        };
        Ui::label("Offscreen List:", None, true);
        let b = Ui::layout_reserve(Vec2::new(0.1, 0.1), false, 0.0);
        self.quad.draw(
            token,
            &self.render_mat,
            Matrix::ts(b.center + Vec3::new(-0.05, -0.05, -0.004), b.dimensions.xy1()),
            None,
            None,
        );
        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
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
