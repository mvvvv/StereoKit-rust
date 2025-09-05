use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Log, Text, TextStyle},
    tools::os_api::launch_browser_android,
    ui::Ui,
    util::named_colors::GREEN,
};

/// Demo to test the launch_browser_android function
#[derive(IStepper)]
pub struct Browser1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,
    pub window_width: f32,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Browser1 {}

impl Default for Browser1 {
    fn default() -> Self {
        Self {
            id: "Browser1".to_string(),
            sk_info: None,
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::Z))),
            window_width: 40.0 * CM,
            text_style: Text::make_style(Font::default(), 0.3, GREEN),
        }
    }
}

impl Browser1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin(
            "Browser Test Demo",
            &mut self.window_pose,
            Some(Vec2::new(self.window_width, 0.0)),
            None,
            None,
        );

        // Buttons to test different URLs
        if Ui::button("Open StereoKit", None) {
            let url = "https://stereokit.net";
            let result = launch_browser_android(url);
            Log::info(format!("Open StereoKit ({}) - Result: {}", url, result));
        }
        Ui::same_line();
        if Ui::button("Open StereoKit-rust", None) {
            let url = "https://docs.rs/stereokit-rust/latest/stereokit_rust/";
            let result = launch_browser_android(url);
            Log::info(format!("Open StereoKit-rust ({}) - Result: {}", url, result));
        }
        Ui::same_line();
        if Ui::button("Open GitHub", None) {
            let url = "https://github.com";
            let result = launch_browser_android(url);
            Log::info(format!("Open GitHub ({}) - Result: {}", url, result));
        }

        if Ui::button("Open Google", None) {
            let url = "https://www.google.com";
            let result = launch_browser_android(url);
            Log::info(format!("Open Google ({}) - Result: {}", url, result));
        }
        Ui::same_line();
        if Ui::button("Open YouTube", None) {
            let url = "https://www.youtube.com";
            let result = launch_browser_android(url);
            Log::info(format!("Open YouTube ({}) - Result: {}", url, result));
        }
        Ui::same_line();
        if Ui::button("Test Meta Quest Store", None) {
            let url = "https://www.oculus.com/experiences/quest/";
            let result = launch_browser_android(url);
            Log::info(format!("Test Meta Quest Store ({}) - Result: {}", url, result));
        }

        Ui::window_end();

        // Affichage du titre principal
        Text::add_at(token, "Browser1", self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
