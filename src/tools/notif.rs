use crate::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    prelude::*,
    shader::Shader,
    system::{Text, TextStyle},
    util::{named_colors::BLACK, Time},
};

/// A simple notification to display a text for a given duration in second.
#[derive(IStepper)]
pub struct HudNotification {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub text: String,
    pub duration: f32,
    pub position: Vec3,
    transform_text: Matrix,
    pub text_style: TextStyle,
}

unsafe impl Send for HudNotification {}

impl Default for HudNotification {
    fn default() -> Self {
        let hud_text_shader = Shader::from_file("shaders/hud_text.hlsl.sks").unwrap();
        let text_style = Text::make_style_with_shader(Font::default(), 0.03, hud_text_shader, BLACK);
        let position = Vec3::new(0.0, -0.2, -0.2);
        let transform_text = Matrix::IDENTITY;
        let text = "???".into();

        Self {
            id: "HudNotification".to_string(),
            sk_info: None,

            text,
            duration: 5.0,
            position,
            transform_text,
            text_style,
        }
    }
}

impl HudNotification {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.transform_text = Matrix::tr(&self.position, &Quat::from_angles(0.0, 180.0, 0.0));
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Text::add_at(token, &self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);

        self.duration -= Time::get_stepf();
        if self.duration < 0.0 {
            SkInfo::send_message(&self.sk_info, StepperAction::Remove(self.id.clone()));
        }
    }
}
