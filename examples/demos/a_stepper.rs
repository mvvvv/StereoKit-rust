use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    sk::{IStepper, StepperAction, StepperId},
    system::{Renderer, Text, TextStyle},
    util::named_colors::RED,
};
use winit::event_loop::EventLoopProxy;

pub struct AStepper {
    id: StepperId,
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    pub transform: Matrix,
    round_cube: Mesh,
    text: String,
    text_style: TextStyle,
}

impl Default for AStepper {
    fn default() -> Self {
        Self {
            id: "AStepper".to_string(),
            event_loop_proxy: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            round_cube: Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.2, Some(16)),
            text: "Stepper A".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl IStepper for AStepper {
    fn initialize(&mut self, id: StepperId, event_loop_proxy: EventLoopProxy<StepperAction>) -> bool {
        self.id = id;
        self.event_loop_proxy = Some(event_loop_proxy);
        true
    }

    fn step(&mut self, _event_report: &[StepperAction]) {
        self.draw()
    }
}

impl AStepper {
    fn draw(&mut self) {
        Renderer::add_mesh(&self.round_cube, Material::pbr(), self.transform, Some(RED.into()), None);
        Text::add_at(&self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
