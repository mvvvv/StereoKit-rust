use crate::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    prelude::*,
    shader::Shader,
    system::{Text, TextStyle},
    util::{Time, named_colors::BLACK},
};

/// A simple notification to display a text for a given duration in second.
/// ### Fields that can be changed before initialization:
/// * `text` - The text to display. Default is "???".
/// * `duration` - The duration in seconds to display the text. Default is 5.0.
/// * `position` - The position of the text. Default is Vec3::new(0.0, -0.2, -0.2).
/// * `text_style` - The style of the text. Default is a black text with a size of 0.03 and a shader from "shaders/hud_text.hlsl.sks".
///
/// ### Events this stepper is listening to:
/// None, This stepper does not listen to any event.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::tools::notif::HudNotification;
///
/// let mut hud_notif = HudNotification::default();
/// hud_notif.text = "Notification!".into();
/// hud_notif.text_style.layout_height(0.2);
/// sk.send_event(StepperAction::add("HudNotification1", hud_notif));
///
/// filename_scr = "screenshots/hud_notification.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/hud_notification.jpeg" alt="screenshot" width="200">
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
            SkInfo::send_event(&self.sk_info, StepperAction::Remove(self.id.clone()));
        }
    }
}
