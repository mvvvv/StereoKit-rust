use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    sprite::Sprite,
    system::{Log, Text, TextBuilder, TextStyle},
    tools::{
        file_browser_b::{BasicPreviewer, FILE_BROWSER_B_OPEN, FILE_BROWSER_B_SAVE, FileBrowserB},
        os_api::BrowseLocation,
    },
    ui::{Ui, UiBtnLayout, UiWin},
    util::{PickerMode, named_colors::GREEN},
};

/// Demo showing how to use [`FileBrowserB`] to open and save files.
///
/// It provides two buttons: one to open a file (Open mode) and one to save a file (Save mode).
/// The starting storage location is chosen among internal / external / documents with radio buttons, and the extension
/// filter is entered in a text field (e.g. `.txt, .md, .rs`). When a file is selected/saved, the path is displayed in
/// the window and logged.
#[derive(IStepper)]
pub struct Documents1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,
    pub window_width: f32,

    /// The storage location the next launched browser will start in.
    pub location: BrowseLocation,
    pub location_path: Option<std::path::PathBuf>, //We want to print the resolved path
    /// counter to add to the browser id so we can have multiple browsers open at once, each with their own state.
    pub browser_counter: u32,

    /// The file extensions filter for the next launched browser, entered as `.txt, .md, .rs`.
    /// An empty input means no filter (all files).
    pub exts_input: String,

    radio_off: Sprite,
    radio_on: Sprite,

    // Info echoed back to the user
    opened_file: String,
    saved_file: String,

    pub title: String,
    pub title_style: TextStyle,
    pub title_transform: Matrix,
}

unsafe impl Send for Documents1 {}

impl Default for Documents1 {
    fn default() -> Self {
        let location = BrowseLocation::External;
        Self {
            id: "Documents1".to_string(),
            sk_info: None,

            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.0), Some(Quat::look_dir(Vec3::Z))),
            window_width: 80.0 * CM,

            location,
            location_path: None,
            browser_counter: 0,

            exts_input: "".into(),
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            opened_file: String::new(),
            saved_file: String::new(),

            title: "Documents1".into(),
            title_style: Text::make_style(Font::default(), 0.3, GREEN),
            title_transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Documents1 {
    const BROWSER_OPEN_SUFFIX: &'static str = "_docs_open";
    const BROWSER_SAVE_SUFFIX: &'static str = "_docs_save";

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.location_path = self.location.get_path(&self.sk_info);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        match key {
            k if k == FILE_BROWSER_B_OPEN => {
                if id == &self.id {
                    self.opened_file = value.to_string();
                    Log::info(format!("Documents1 received OPEN event: {value}"));
                }
            }
            k if k == FILE_BROWSER_B_SAVE => {
                if id == &self.id {
                    self.saved_file = value.to_string();
                    Log::info(format!("Documents1 received SAVE event: {value}"));
                }
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

        // ---- Location chooser: internal / external / documents
        self.draw_location_chooser();

        // ---- Extensions filter input (e.g. ".txt, .md, .rs")
        self.draw_exts_input();

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
        TextBuilder::new(&self.title).transform(self.title_transform).style(self.title_style).add();
    }

    /// The internal / external / documents location chooser, shown as radio buttons.
    ///
    /// Android: app `internal_data_path` / app `external_data_path` / shared Documents folder.
    /// PC (juxtaposition): current dir / assets dir / user Documents folder.
    /// The resolved root path of the selected location is displayed under the radios.
    fn draw_location_chooser(&mut self) {
        Ui::next_line();
        let mut new_location: Option<BrowseLocation> = None;
        for location in [
            BrowseLocation::Internal,
            BrowseLocation::External,
            BrowseLocation::Documents,
            BrowseLocation::Pictures,
            BrowseLocation::Movies,
            BrowseLocation::Music,
            BrowseLocation::Downloads,
        ] {
            Ui::same_line();
            if Ui::radio(location.as_str(), self.location == location)
                .size([0.09, 0.03])
                .images(&self.radio_off, &self.radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                new_location = Some(location);
            }
        }
        if let Some(location) = new_location {
            self.location = location;
            self.location_path = self.location.get_path(&self.sk_info);
            Log::diag(format!("Documents1 location set to {:?}: {:?}", self.location, self.location_path));
        }

        // Show where the selected location resolves to on this platform.
        let path_text = self
            .location_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unavailable".to_string());
        Ui::label(format!("Absolute path: {path_text}")).draw();
    }

    /// Input field for the file extensions filter, entered as `.txt, .md, .rs`.
    /// An empty input means no filter (all files). The effective filter parsed from
    /// the input is displayed under the field.
    fn draw_exts_input(&mut self) {
        Ui::label("File Extensions (ie: .txt, .md, .rs):").draw();
        Ui::same_line();
        Ui::input("documents1_exts", &mut self.exts_input).size(Vec2::new(0.17, 0.0)).edit();

        let exts = parse_exts(&self.exts_input);
        let filter_text = if exts.is_empty() { "no filter (all files)".to_string() } else { exts.join(" ") };
        Ui::label(format!("filter: {filter_text}")).draw();
    }

    fn spawn_browser(&mut self, mode: PickerMode) {
        self.browser_counter = (self.browser_counter + 1) % 1000; // keep it small for the id
        let suffix = match mode {
            PickerMode::Open => Self::BROWSER_OPEN_SUFFIX,
            PickerMode::Save => Self::BROWSER_SAVE_SUFFIX,
        };
        let suffix = format!("{}_{}", suffix, self.browser_counter);

        let mut file_browser = FileBrowserB::default();

        // Start browsing from the location chosen with the demo radio buttons.
        file_browser.location = self.location;

        // Extension filter from the demo input field, entered as `.txt, .md, .rs`.
        // An empty input means no filter (all files).
        file_browser.exts = parse_exts(&self.exts_input);

        // The caller id MUST match this stepper id so we receive the events back.
        file_browser.caller = self.id.clone();

        // Tweak the window so it pops up comfortably next to the demo panel.
        file_browser.window_pose = Ui::popup_pose([(self.browser_counter as f32) / 100.0, 0.05, 0.10]);

        // Set the default previewer: `BasicPreview`, boxed as a `Preview` trait object. The browser
        // calls it every frame with the focused entry's path, the window pose and the focused
        // button pose, drawing a small info panel beside the button.
        file_browser.preview = Some(Box::new(BasicPreviewer::default()));

        if mode == PickerMode::Save {
            file_browser.picker_mode = PickerMode::Save;
            file_browser.file_name_to_save = "demo_output.txt".into();
        }

        // Enable the grid view by default for a nicer demo.
        file_browser.grid_view = false;

        SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone() + &suffix, file_browser));
    }
}

/// Parse an extensions filter entered as `.txt, .md, .rs` into a `Vec<String>`.
///
/// Entries are split on `,`, trimmed and lowercased; a missing leading `.` is added back.
/// An empty input (or only separators/whitespace) gives an empty vector, meaning no filter.
fn parse_exts(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|ext| ext.trim().to_lowercase())
        .filter(|ext| !ext.is_empty())
        .map(|ext| if ext.starts_with('.') { ext } else { format!(".{ext}") })
        .collect()
}
