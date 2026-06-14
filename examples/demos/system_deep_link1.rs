use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Log, Text, TextStyle},
    tools::os_api::{SystemAction, system_deep_link},
    ui::Ui,
    util::named_colors::GREEN,
};

/// Demo to test the system_deep_link function with various Meta Quest actions
#[derive(IStepper)]
pub struct SystemDeepLink1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub window_pose: Pose,
    pub window_width: f32,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for SystemDeepLink1 {}

impl Default for SystemDeepLink1 {
    fn default() -> Self {
        Self {
            id: "SystemDeepLinkDemo".to_string(),
            sk_info: None,
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::Z))),
            window_width: 50.0 * CM,
            text_style: Text::make_style(Font::default(), 0.3, GREEN),
        }
    }
}

impl SystemDeepLink1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window("System Deep Link Demo")
            .pose(&mut self.window_pose)
            .size(Vec2::new(self.window_width, 0.0))
            .begin();

        // Browser Actions Row
        Ui::label("Open Browser:").use_padding(true).draw();
        if Ui::button("Google").press() {
            let result = system_deep_link(SystemAction::Browser { url: "https://www.google.com".to_string() });
            Log::info(format!("Open Google - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Youtube").press() {
            let result = system_deep_link(SystemAction::Browser { url: "https://www.youtube.com".to_string() });
            Log::info(format!("Open Youtube - Result: {:?}", result));
        }
        Ui::same_line();

        if Ui::button("StereoKit Website").press() {
            let result = system_deep_link(SystemAction::Browser { url: "https://stereokit.net".to_string() });
            Log::info(format!("Open StereoKit Website - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("StereoKit-rust GitHub").press() {
            let result =
                system_deep_link(SystemAction::Browser { url: "https://github.com/mvvvv/StereoKit-rust".to_string() });
            Log::info(format!("Open StereoKit-rust doc - Result: {:?}", result));
        }

        Ui::hseparator();

        // Store section
        Ui::label("Open Store:").use_padding(true).draw();
        if Ui::button("Front Page").press() {
            let result = system_deep_link(SystemAction::Store { app_id: None });
            Log::info(format!("Open Store Front Page - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("A great game").press() {
            let result = system_deep_link(SystemAction::Store { app_id: Some("3749621795127676".to_string()) });
            Log::info(format!("Open Specific App - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Another great game").press() {
            let result = system_deep_link(SystemAction::Store { app_id: Some("4015163475201433".to_string()) });
            Log::info(format!("Open Specific App - Result: {:?}", result));
        }
        Ui::hseparator();

        // Settings section
        Ui::label("Open Settings:").use_padding(true).draw();
        if Ui::button("General").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/system".to_string()) });
            Log::info(format!("Open General Settings - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Controllers").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/controllers".to_string()) });
            Log::info(format!("Open Controllers Settings - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Movement Tracking").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/hands".to_string()) });
            Log::info(format!("Open Movement Tracking Settings - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Bluetooth").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/bluetooth".to_string()) });
            Log::info(format!("Open Bluetooth Settings - Result: {:?}", result));
        }
        Ui::same_line();
        // Third row of settings
        if Ui::button("WiFi").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/wifi".to_string()) });
            Log::info(format!("Open WiFi Settings - Result: {:?}", result));
        }

        if Ui::button("Display").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/device".to_string()) });
            Log::info(format!("Open Display & brightness Settings - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Guardian").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/guardian".to_string()) });
            Log::info(format!("Open Guardian Settings - Result: {:?}", result));
        }
        Ui::same_line();
        // Fourth row of settings
        if Ui::button("Accounts").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/accounts".to_string()) });
            Log::info(format!("Open Accounts Settings - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Notifications").press() {
            let result = system_deep_link(SystemAction::Settings { setting: Some("/notifications".to_string()) });
            Log::info(format!("Open Notifications Settings - Result: {:?}", result));
        }

        if Ui::button("App Settings").press() {
            let result = system_deep_link(SystemAction::Settings {
                setting: Some("/applications?package=com.stereokit.rust_binding.demos".to_string()),
            });
            Log::info(format!("Open VR Shell App Settings - Result: {:?}", result));
        }

        Ui::hseparator();

        // File Manager section
        Ui::label("Open File Manager:").use_padding(true).draw();
        if Ui::button("Recents Tab").press() {
            let result = system_deep_link(SystemAction::FileManager { path: None });
            Log::info(format!("Open File Manager (Recents) - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Media Tab").press() {
            let result = system_deep_link(SystemAction::FileManager { path: Some("/media/".to_string()) });
            Log::info(format!("Open Media Tab - Result: {:?}", result));
        }
        Ui::same_line();
        if Ui::button("Downloads Tab").press() {
            let result = system_deep_link(SystemAction::FileManager { path: Some("/downloads/".to_string()) });
            Log::info(format!("Open Downloads Tab - Result: {:?}", result));
        }

        Ui::hseparator();

        // Bug Report section
        Ui::label("System Actions:").use_padding(true).draw();
        if Ui::button("Open Bug Reporter").press() {
            let result = system_deep_link(SystemAction::BugReport);
            Log::info(format!("Open Bug Reporter - Result: {:?}", result));
        }

        Ui::window_end();

        // Affichage du titre principal
        Text::add_at(
            token,
            "SystemDeepLink1",
            self.transform,
            Some(self.text_style),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }
}
