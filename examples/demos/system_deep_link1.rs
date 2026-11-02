use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Log, Text, TextBuilder, TextStyle},
    tools::os_api::{SystemAction, app_package_name, is_horizon_os, system_deep_link},
    ui::Ui,
    util::named_colors::GREEN,
};

/// Demo of [`system_deep_link`], showing the links which work on the current platform.
#[derive(IStepper)]
pub struct SystemDeepLink1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub window_pose: Pose,
    pub window_width: f32,

    pub transform: Matrix,
    pub text: String,
    text_style: TextStyle,
    /// Meta Horizon OS, the only platform with the `systemux://` links. Detected at start.
    horizon_os: bool,
}

unsafe impl Send for SystemDeepLink1 {}

impl Default for SystemDeepLink1 {
    fn default() -> Self {
        Self {
            id: "SystemDeepLinkDemo".to_string(),
            sk_info: None,

            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::Z))),
            window_width: 50.0 * CM,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::Y_180),
            text: "SystemDeepLink1".to_string(),
            text_style: Text::make_style(Font::default(), 0.3, GREEN),
            horizon_os: false,
        }
    }
}

/// Draws a button which triggers a system deep link when pressed, and logs the result.
fn link_button(label: &str, action: impl FnOnce() -> SystemAction) {
    if Ui::button(label).press() {
        let result = system_deep_link(action());
        Log::info(format!("{label} - Result: {:?}", result));
    }
}

/// The settings page of this app, with its real package name.
fn app_settings() -> SystemAction {
    match app_package_name() {
        Some(package) => SystemAction::Settings { setting: Some(format!("/applications?package={package}")) },
        None => SystemAction::Settings { setting: None },
    }
}

impl SystemDeepLink1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.horizon_os = is_horizon_os();
        Log::info(format!("SystemDeepLink1: {}", self.platform()));
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("System Deep Link Demo")
            .pose(&mut self.window_pose)
            .size(Vec2::new(self.window_width, 0.0))
            .begin();

        Ui::label(self.platform()).use_padding(true).draw();

        if self.horizon_os {
            self.draw_horizon_os();
        } else if cfg!(target_os = "android") {
            self.draw_android();
        }

        Ui::window_end();

        // Title of this demo:
        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    /// The name of the platform, for the window and the logs.
    fn platform(&self) -> &'static str {
        if self.horizon_os {
            "Meta Horizon OS"
        } else if cfg!(target_os = "android") {
            "Android"
        } else {
            "Deep links need Android"
        }
    }

    /// The `systemux://` links documented by Meta for Horizon OS.
    fn draw_horizon_os(&mut self) {
        Ui::label("Browser:").use_padding(true).draw();
        link_button("Google", || SystemAction::Browser { url: "https://www.google.com".to_string() });
        Ui::same_line();
        link_button("Youtube", || SystemAction::Browser { url: "https://www.youtube.com".to_string() });
        Ui::same_line();
        link_button("StereoKit Website", || SystemAction::Browser { url: "https://stereokit.net".to_string() });
        Ui::same_line();
        link_button("StereoKit-rust GitHub", || SystemAction::Browser {
            url: "https://github.com/mvvvv/StereoKit-rust".to_string(),
        });

        Ui::hseparator();

        Ui::label("Store:").use_padding(true).draw();
        link_button("Front Page", || SystemAction::Store { app_id: None });
        Ui::same_line();
        link_button("A great game", || SystemAction::Store { app_id: Some("3749621795127676".to_string()) });
        Ui::same_line();
        link_button("Another great game", || SystemAction::Store { app_id: Some("4015163475201433".to_string()) });

        Ui::hseparator();

        Ui::label("Settings:").use_padding(true).draw();
        link_button("Main", || SystemAction::Settings { setting: None });
        Ui::same_line();
        link_button("Hand Tracking", || SystemAction::Settings { setting: Some("/hands".to_string()) });
        Ui::same_line();
        link_button("App Settings", app_settings);

        Ui::hseparator();

        Ui::label("Files:").use_padding(true).draw();
        link_button("Recents", || SystemAction::FileManager { path: None });
        Ui::same_line();
        link_button("Media", || SystemAction::FileManager { path: Some("/media/".to_string()) });
        Ui::same_line();
        link_button("Downloads", || SystemAction::FileManager { path: Some("/downloads/".to_string()) });

        Ui::hseparator();

        link_button("Bug Reporter", || SystemAction::BugReport);
    }

    /// The standard Android intents which [`system_deep_link`] uses outside Horizon OS.
    fn draw_android(&mut self) {
        Ui::label("Browser:").use_padding(true).draw();
        link_button("Google", || SystemAction::Browser { url: "https://www.google.com".to_string() });
        Ui::same_line();
        link_button("Youtube", || SystemAction::Browser { url: "https://www.youtube.com".to_string() });
        Ui::same_line();
        link_button("StereoKit Website", || SystemAction::Browser { url: "https://stereokit.net".to_string() });
        Ui::same_line();
        link_button("StereoKit-rust GitHub", || SystemAction::Browser {
            url: "https://github.com/mvvvv/StereoKit-rust".to_string(),
        });

        Ui::hseparator();

        Ui::label("Play Store:").use_padding(true).draw();
        link_button("Front Page", || SystemAction::Store { app_id: None });
        Ui::same_line();
        link_button("Youtube", || SystemAction::Store { app_id: Some("com.google.android.youtube".to_string()) });

        Ui::hseparator();

        Ui::label("Settings:").use_padding(true).draw();
        link_button("Main", || SystemAction::Settings { setting: None });
        Ui::same_line();
        link_button("WiFi", || SystemAction::Settings { setting: Some("/wifi".to_string()) });
        Ui::same_line();
        link_button("Bluetooth", || SystemAction::Settings { setting: Some("/bluetooth".to_string()) });
        Ui::same_line();
        link_button("Display", || SystemAction::Settings { setting: Some("/device".to_string()) });
        Ui::same_line();
        link_button("App Settings", app_settings);

        Ui::hseparator();

        Ui::label("Files:").use_padding(true).draw();
        link_button("Open", || SystemAction::FileManager { path: None });
    }
}
