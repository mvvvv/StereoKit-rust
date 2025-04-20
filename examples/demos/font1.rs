use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec3},
    prelude::*,
    system::{Text, TextStyle},
    ui::Ui,
    util::named_colors::{BLUE, GREEN, RED},
};
/// The basic Stepper. we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
#[derive(IStepper)]
pub struct Font1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    emoji_style: Option<TextStyle>,
    text_style: TextStyle,
    window_pose: Pose,
    pub transform: Matrix,
    pub text: String,
    title_style: Option<TextStyle>,
}

unsafe impl Send for Font1 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for Font1 {
    fn default() -> Self {
        // Load font assets
        let emoji_font = if cfg!(windows) {
            Font::from_file("C:\\Windows\\Fonts\\Seguiemj.ttf")
                .unwrap_or(Font::from_file("fonts/Noto_Emoji/NotoEmoji-VariableFont_wght.ttf").unwrap_or_default())
        } else {
            Font::from_file("fonts/Noto_Emoji/NotoEmoji-VariableFont_wght.ttf").unwrap_or_default()
        };
        let text_font = if cfg!(windows) {
            Font::from_file("C:\\Windows\\Fonts\\Arial.ttf").unwrap_or_default()
        } else {
            Font::from_file("fonts/Inter/Inter-VariableFont_opsz_wght.ttf").unwrap_or_default()
        };

        let emoji_style = Some(Text::make_style(emoji_font, 0.35, BLUE));
        let text_style = Text::make_style(text_font, 0.025, GREEN);
        let window_pose = Pose::new(Vec3::new(-0.05, 1.5, -0.45), Some(Quat::from_angles(0.0, 160.0, 0.0)));
        Self {
            id: "Font1".to_string(),
            sk_info: None,

            emoji_style,
            text_style,
            window_pose,
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Font1".to_owned(),
            title_style: None,
        }
    }
}

/// All the code here run in the main thread
impl Font1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.title_style = Some(Text::make_style(Font::default(), 0.3, RED));
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Text::add_at(
            token,
            "😋 Emojis🤪\n\n  🧐",
            self.transform,
            self.emoji_style,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        Ui::window_begin("Default Font", &mut self.window_pose, None, None, None);
        Ui::push_text_style(self.text_style);
        Ui::text("text font", None, None, None, Some(0.14), None, None);
        Ui::pop_text_style();
        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, self.title_style, None, None, None, None, None, None);
    }
}
