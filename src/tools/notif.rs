use std::{cell::RefCell, rc::Rc};

use crate::{
    event_loop::{IStepper, StepperAction, StepperId},
    font::Font,
    maths::{Matrix, Quat, Vec3},
    shader::Shader,
    sk::{MainThreadToken, SkInfo},
    system::{Text, TextStyle},
    util::{named_colors::BLACK, Time},
};

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

impl IStepper for HudNotification {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        self.transform_text = Matrix::tr(&self.position, &Quat::from_angles(0.0, 180.0, 0.0));
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl HudNotification {
    fn draw(&mut self, token: &MainThreadToken) {
        Text::add_at(token, &self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);

        self.duration -= Time::get_stepf();
        if self.duration < 0.0 {
            let rc_sk = self.sk_info.as_ref().unwrap();
            let sk = rc_sk.as_ref();
            let event_loop_proxy = sk.borrow().get_event_loop_proxy().unwrap();
            let _ = event_loop_proxy.send_event(StepperAction::Remove(self.id.clone()));
        }
    }
}
