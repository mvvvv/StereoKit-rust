use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Log, Text, TextBuilder, TextStyle},
    tools::file_browser_b::{FILE_BROWSER_B_OPEN, FILE_BROWSER_B_SAVE, FileBrowserB},
    ui::{Ui, UiWin},
    util::{PickerMode, named_colors::GREEN},
};

/// Demo showing how to use [`FileBrowserB`] to open and save files.
///
/// It provides two buttons: one to open a file (Open mode) and one to save a file (Save mode).
/// When a file is selected/saved, the path is displayed in the window and logged.
#[derive(IStepper)]
pub struct Documents1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,
    pub window_width: f32,
    pub title_style: TextStyle,
    pub title_transform: Matrix,

    // Info echoed back to the user
    opened_file: String,
    saved_file: String,
}

unsafe impl Send for Documents1 {}

impl Default for Documents1 {
    fn default() -> Self {
        Self {
            id: "Documents1".to_string(),
            sk_info: None,
            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.0), Some(Quat::look_dir(Vec3::Z))),
            window_width: 40.0 * CM,
            title_style: Text::make_style(Font::default(), 0.3, GREEN),
            title_transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            opened_file: String::new(),
            saved_file: String::new(),
        }
    }
}

impl Documents1 {
    const BROWSER_OPEN_SUFFIX: &'static str = "_docs_open";
    const BROWSER_SAVE_SUFFIX: &'static str = "_docs_save";

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        match key {
            k if k == FILE_BROWSER_B_OPEN => {
                self.opened_file = value.to_string();
                Log::info(format!("Documents1 received OPEN event: {value}"));
            }
            k if k == FILE_BROWSER_B_SAVE => {
                self.saved_file = value.to_string();
                Log::info(format!("Documents1 received SAVE event: {value}"));
            }
            _ => {}
        }
    }

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("Documents1 - FileBrowserB Demo")
            .pose(&mut self.window_pose)
            .size(Vec2::new(self.window_width, 0.0))
            .window_type(UiWin::Normal)
            .begin();

        Ui::label("Launch a file browser:").draw();

        // ---- Open button
        if Ui::button("Open a file").press() {
            self.spawn_browser(PickerMode::Open);
        }

        Ui::same_line();

        // ---- Save button
        if Ui::button("Save a file").press() {
            self.spawn_browser(PickerMode::Save);
        }

        Ui::hseparator();

        // ---- Show selected file (Open mode)
        if self.opened_file.is_empty() {
            Ui::label("No file opened yet.").draw();
        } else {
            Ui::label(format!("Opened: {}", self.opened_file)).draw();
        }

        // ---- Show saved file (Save mode)
        if self.saved_file.is_empty() {
            Ui::label("No file saved yet.").draw();
        } else {
            Ui::label(format!("Saved: {}", self.saved_file)).draw();
        }

        Ui::window_end();

        // Title floating behind the window
        TextBuilder::new("Documents1").transform(self.title_transform).style(self.title_style).add();
    }

    fn spawn_browser(&self, mode: PickerMode) {
        let suffix = match mode {
            PickerMode::Open => Self::BROWSER_OPEN_SUFFIX,
            PickerMode::Save => Self::BROWSER_SAVE_SUFFIX,
        };

        let mut file_browser = FileBrowserB::default();

        // Browse the tests directory as a starting point.
        if !file_browser.dir.exists() {
            file_browser.dir = std::env::current_dir().unwrap_or_default().join("tests");
        }

        // The caller id MUST match this stepper id so we receive the events back.
        file_browser.caller = self.id.clone();

        // Tweak the window so it pops up comfortably next to the demo panel.
        file_browser.window_pose = Ui::popup_pose([0.15, 0.05, 0.10]);

        if mode == PickerMode::Save {
            file_browser.picker_mode = PickerMode::Save;
            file_browser.file_name_to_save = "demo_output.txt".into();
            // Pre-select a default extension for saving.
            file_browser.exts = vec![".txt".into(), ".md".into()];
        } else {
            // Filter on a couple of common text extensions for the demo.
            file_browser.exts = vec![".rs".into(), ".txt".into(), ".md".into()];
        }

        // Enable the grid view by default for a nicer demo.
        file_browser.grid_view = false;

        SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone() + suffix, file_browser));
    }
}
