use crate::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    prelude::*,
    system::{Text, TextStyle},
    util::{Color32, named_colors},
};

/// Title is a basic Stepper to show a big title in the scene.
/// ### Fields that can be changed before initialization:
/// * `transform` - The transform of the text. Default is [0.0, 1.0, -0.5] * Y_180°
/// * `text` - The text to display. Default is "Title".
/// * `text_style` - The style of the text. Default is a white text with a size of 0.5 and the default font.
///
/// ### Events this stepper is listening to:
/// None, This stepper does not listen to any event.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{tools::title::Title, util::named_colors, maths::Matrix};
///
/// let mut title = Title::new("My Title", Some(named_colors::RED), None, None);
/// title.transform = Matrix::tr(&([-0.2, 0.0, -0.3].into()), &([0.0, 160.0, 0.0].into()));
/// sk.send_event(StepperAction::add("Title", title));
///
/// filename_scr = "screenshots/title.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/title.jpeg" alt="screenshot" width="200">
#[derive(IStepper, Clone)]
pub struct Title {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub transform: Matrix,
    pub text: String,
    pub text_style: Option<TextStyle>,
}

unsafe impl Send for Title {}

impl Default for Title {
    /// This code may be called in some threads, so no StereoKit code.
    /// It's better to use [Title::new] instead as you need to set the text.
    fn default() -> Self {
        Self {
            id: "Title".to_string(),
            sk_info: None,

            transform: Matrix::tr(&((Vec3::NEG_Z * 0.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Title".to_owned(),
            text_style: None,
        }
    }
}

/// All the code here run in the main thread
impl Title {
    /// Create a new Title stepper
    /// * `text` - The text to display
    /// * `color` - The color of the text. Default is WHITE
    /// * `font_size` - The size of the font. Default is 0.5
    /// * `font` - The font to use. Default is the default font
    pub fn new(text: &str, color: Option<Color32>, font_size: Option<f32>, font: Option<Font>) -> Self {
        let mut title = Self { text: text.into(), ..Default::default() };
        let font = font.unwrap_or_default();
        let font_size = font_size.unwrap_or(0.5);
        let color = color.unwrap_or(named_colors::WHITE);
        title.text_style = Some(Text::make_style(font, font_size, color));
        title
    }

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {
        // if key == "Title" {
        //     self.enabled = value.parse().unwrap_or(false)
        // }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }
}
