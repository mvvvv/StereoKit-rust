use crate::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    prelude::*,
    system::{Input, Text, TextStyle},
    util::{Time, named_colors::BLACK},
};

/// A simple notification to display a text for a given duration in second.
/// ### Fields that can be changed before initialization:
/// * `text` - The text to display. Default is "???".
/// * `duration` - The duration in seconds to display the text, or None for infinite display. Default is Some(5.0).
/// * `position` - The position offset from the head in local space. Z (abs value) controls distance forward, X controls horizontal offset, Y controls vertical offset. Default is Vec3::new(0.0, -0.06, -0.3) (30cm forward).
/// * `text_style` - The style of the text. Default is a black text with a size of 0.018 and the default unlit shader.
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
    pub duration: Option<f32>,
    pub position: Vec3,
    transform_text: Matrix,
    pub text_style: TextStyle,
}

unsafe impl Send for HudNotification {}

impl Default for HudNotification {
    fn default() -> Self {
        let text_style = Text::make_style(Font::default(), 0.018, BLACK);
        let position = Vec3::new(0.0, -0.06, -0.3);
        let transform_text = Matrix::IDENTITY;
        let text = "???".into();

        Self {
            id: "HudNotification".to_string(),
            sk_info: None,

            text,
            duration: Some(5.0),
            position,
            transform_text,
            text_style,
        }
    }
}

impl HudNotification {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    ///
    /// If you set the duration field to None, you can directly call this method without adding the stepper to the event
    /// system.
    pub fn draw(&mut self, token: &MainThreadToken) {
        // Calculate position in front of head
        let head_pose = Input::get_head();
        let head_forward = head_pose.get_forward();
        let head_right = head_pose.orientation.mul_vec3(Vec3::X);
        let head_up = head_pose.orientation.mul_vec3(Vec3::Y);

        // Position the notification in front of the head based on its forward direction
        // Apply offsets in the head's local space
        let notif_position = head_pose.position
            + head_forward * self.position.z.abs()
            + head_right * self.position.x
            + head_up * self.position.y;

        // Orient the text to face the head
        let notif_orientation = Quat::look_at(notif_position, head_pose.position, None);
        self.transform_text = Matrix::t_r(notif_position, notif_orientation);

        Text::add_at(token, &self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);

        if let Some(ref mut duration) = self.duration {
            *duration -= Time::get_stepf();
            if *duration < 0.0 {
                SkInfo::send_event(&self.sk_info, StepperAction::Remove(self.id.clone()));
            }
        }
    }
}
