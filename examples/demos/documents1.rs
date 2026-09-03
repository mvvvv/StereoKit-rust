use std::path::PathBuf;

use stereokit_rust::{
    font::Font,
    framework::Appearence,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Align, Log, Text, TextBuilder, TextFit, TextStyle},
    tools::{
        file_browser_b::{
            BasicPreviewer, FILE_BROWSER_B_DELETE_MULTI, FILE_BROWSER_B_OPEN_MULTI, FILE_BROWSER_B_SAVE,
            FILE_BROWSER_B_SELECT_DIR, FileBrowserB, available_locales, locale, set_locale,
        },
        os_api::BrowseLocation,
    },
    ui::{Ui, UiBtnLayout, UiScroll, UiSettings, UiWin},
    util::{PickerMode, named_colors::GREEN},
};

/// Demo showing how to use [`FileBrowserB`] in all its modes.
///
/// It provides one button per [`PickerMode`]: open a file, open multiple files, save a file, select a directory,
/// delete a file, delete multiple files, delete a directory. The starting storage location is chosen with radio
/// buttons, the extension filter is entered in a text field (e.g. `.txt, .md, .rs`) and the language of the browser
/// texts is cycled through the [`available_locales`] with a right arrow button. The events received from the
/// browser are concatenated in a multi-line log, most recent first, displayed in a scrollable text area and logged:
/// the demo never deletes anything itself, it only shows the [`FileBrowserB`] events.
///
/// The demo panel uses [`Appearence`] to resize/zoom the demo window.
#[derive(IStepper)]
pub struct Documents1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,
    /// Size, scaling, text styles and tints of the demo window, same as [`FileBrowserB`].
    pub appearence: Appearence,
    locales: Vec<String>,

    /// The storage location the next launched browser will start in.
    pub location: BrowseLocation,
    /// The resolved path of `location`, displayed under the radios.
    pub location_path: Option<std::path::PathBuf>,
    /// The file extensions filter for the next launched browser, entered as `.txt, .md, .rs`.
    /// An empty input means no filter (all files).
    pub exts_input: String,
    /// Counter added to the browser id so we can have multiple browsers open at once, each with
    /// their own state.
    pub browser_counter: u32,

    /// Log of the events received from the launched browsers, most recent first, one entry per event formatted by
    /// [`Documents1::remember_event`] from [`FileBrowserB::get_selected_paths`]: `File_Browser_B_save: /path/to/file`
    /// on one line for the single-path events, the event name on its own line then the paths below it, indented by
    /// [`Documents1::EVENT_INDENT`], for the multi-file ones. Capped at [`Documents1::MAX_EVENT_LINES`] lines.
    events_received: String,
    /// Scroll state (in and out, updated by the scrollbars) of the scrollable [`Ui::text`] showing
    /// [`Documents1::events_received`].
    events_scroll: Vec2,

    radio_off: Sprite,
    radio_on: Sprite,
    right_arrow: Sprite,

    pub title: String,
    pub title_style: TextStyle,
    pub title_transform: Matrix,
}

unsafe impl Send for Documents1 {}

impl Default for Documents1 {
    fn default() -> Self {
        Self {
            id: "Documents1".to_string(),
            sk_info: None,

            window_pose: Pose::new(Vec3::new(0.0, 1.5, -1.0), Some(Quat::look_dir(Vec3::Z))),
            appearence: Appearence::default(),
            locales: available_locales(),

            location: BrowseLocation::External,
            location_path: None,
            exts_input: String::new(),
            browser_counter: 0,

            events_received: String::new(),
            events_scroll: Vec2::ZERO,

            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            right_arrow: Sprite::arrow_right(),

            title: "Documents1".into(),
            title_style: Text::make_style(Font::default(), 0.3, GREEN),
            title_transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Documents1 {
    const BROWSER_OPEN_SUFFIX: &'static str = "_docs_open";
    const BROWSER_OPEN_MULTI_SUFFIX: &'static str = "_docs_open_multi";
    const BROWSER_SAVE_SUFFIX: &'static str = "_docs_save";
    const BROWSER_SELECT_DIR_SUFFIX: &'static str = "_docs_select_dir";
    const BROWSER_DELETE_SUFFIX: &'static str = "_docs_delete";
    /// Indentation of the paths of a multi-file event in [`Documents1::events_received`]: StereoKit's text has no
    /// tab-stop handling, so the alignment tab is a few spaces that line the paths up below the event name.
    const EVENT_INDENT: &str = "    ";
    /// Maximum number of event lines kept in [`Documents1::events_received`], so the log stays bounded and readable.
    /// The multi-file events span several lines (paths indented under the event name) and the scrollbar handles the
    /// overflow, so this is only a memory bound, not a display one.
    const MAX_EVENT_LINES: usize = 50;

    /// Logs the event, then formats its paths (see [`FileBrowserB::get_selected_paths`]) and prepends them to
    /// [`Documents1::events_received`] (most recent first): `event_name: path` on ONE line for a single path, the
    /// `event_name` alone on the first line then one path per line below it, indented by [`Documents1::EVENT_INDENT`],
    /// for the multi-file events. Also scrolls the log back to its newest lines, and trims it to
    /// [`Documents1::MAX_EVENT_LINES`] lines.
    fn remember_event(&mut self, event_name: &str, value: &str) {
        Log::info(format!("Documents1 received {event_name} event: {value}"));
        let paths = FileBrowserB::get_selected_paths(value);
        let event_lines: Vec<String> = match paths.as_slice() {
            [path] => vec![format!("{event_name}: {path}")],
            _ => std::iter::once(format!("{event_name}:"))
                .chain(paths.into_iter().map(|path| format!("{}{path}", Self::EVENT_INDENT)))
                .collect(),
        };
        let mut all: Vec<String> =
            event_lines.into_iter().chain(self.events_received.lines().map(str::to_string)).collect();
        all.truncate(Self::MAX_EVENT_LINES);
        self.events_received = all.join("\n");
        // The newest lines were just added on top of the log: reset the scroll of the events text to show them.
        self.events_scroll = Vec2::ZERO;
    }

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.location_path = self.location.get_path(&self.sk_info);

        // size of the window
        self.appearence.window_size = Vec2 { x: 0.80, y: 0.60 };
        self.appearence.min_window_size = Vec2 { x: 0.45, y: 0.60 };
        // A nice handle
        self.appearence.handle_sprite = Sprite::from_file("icons/zoom.png", None, None).ok();

        self.appearence.keep_window_ratio = true;
        self.appearence.start();
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        if id != &self.id {
            return;
        }
        match key {
            // Every browser event carries path(s) in its value, several separated by `\n` in the multi-file modes:
            // remember_event formats the paths of FileBrowserB::get_selected_paths for the log.
            FILE_BROWSER_B_OPEN_MULTI
            | FILE_BROWSER_B_SAVE
            | FILE_BROWSER_B_SELECT_DIR
            | FILE_BROWSER_B_DELETE_MULTI => {
                self.remember_event(key, value);
            }
            _ => {}
        }
    }

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        // The demo window uses the `Appearence` management
        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());

        Ui::window("Documents1 - FileBrowserB Demo")
            .pose(&mut self.window_pose)
            .size(self.appearence.scaled_window_size())
            .window_type(UiWin::Normal)
            .begin();

        self.draw_mode_buttons();
        Ui::hseparator();
        self.draw_browser_settings();
        Ui::hseparator();
        self.draw_events_received();

        Ui::window_end();
        Ui::settings(prev_settings);

        // Grab-able knob beside the window: resizes / scales it, exactly like on FileBrowserB.
        self.appearence.scale_handle(&self.window_pose, "documents1_scale_handle");

        // Title floating behind the window
        TextBuilder::new(&self.title).transform(self.title_transform).style(self.title_style).add();
    }

    /// One button per [`PickerMode`], laid out four per line.
    fn draw_mode_buttons(&mut self) {
        const MODES: [(&str, PickerMode); 7] = [
            ("Open a file", PickerMode::Open),
            ("Open files", PickerMode::OpenMulti),
            ("Save a file", PickerMode::Save),
            ("Select a directory", PickerMode::SelectDirectory),
            ("Delete a file", PickerMode::DeleteFile),
            ("Delete files", PickerMode::DeleteFileMulti),
            ("Delete a directory", PickerMode::DeleteDirectory),
        ];

        Ui::push_text_style(self.appearence.title_style);
        Ui::label("Launch a file browser:").draw();
        Ui::pop_text_style();

        Ui::push_text_style(self.appearence.label_style);
        for (label, mode) in MODES.iter() {
            Ui::same_line();
            if Ui::button(label).press() {
                self.spawn_browser(*mode);
            }
        }
        Ui::pop_text_style();
    }

    /// The settings shared by every launched browser: the starting location, the extension filter and the language
    /// of the browser texts.
    fn draw_browser_settings(&mut self) {
        Ui::push_text_style(self.appearence.title_style);
        Ui::label("Browser location:").draw();
        Ui::pop_text_style();

        Ui::push_text_style(self.appearence.label_style);
        self.draw_location_chooser();
        self.draw_exts_input();
        self.draw_locale_chooser();
        Ui::pop_text_style();
    }

    /// The internal / external / documents... location chooser, shown as radio buttons with the resolved root path of
    /// the selected location displayed under the radios.
    fn draw_location_chooser(&mut self) {
        const LOCATIONS: [BrowseLocation; 7] = [
            BrowseLocation::Internal,
            BrowseLocation::External,
            BrowseLocation::Documents,
            BrowseLocation::Pictures,
            BrowseLocation::Movies,
            BrowseLocation::Music,
            BrowseLocation::Downloads,
        ];

        let mut new_location: Option<BrowseLocation> = None;
        for location in LOCATIONS.iter() {
            Ui::same_line();
            if Ui::radio(location.as_str(), self.location == *location)
                .size(self.appearence.scale_size(Vec2::new(0.09, 0.03)))
                .images(&self.radio_off, &self.radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                new_location = Some(*location);
            }
        }
        if let Some(location) = new_location {
            self.location = location;
            self.location_path = self.location.get_path(&self.sk_info);
            Log::diag(format!("Documents1 location set to {:?}: {:?}", self.location, self.location_path));
        }

        // Show where the selected location resolves to on this platform.
        Ui::push_text_style(self.appearence.small_style);
        let path_text = self
            .location_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unavailable".to_string());
        Ui::label(format!("Absolute path: {path_text}")).draw();
        Ui::pop_text_style();
    }

    /// Input field for the file extensions filter, entered as `.txt, .md, .rs`. An empty input means no filter (all
    /// files). The effective filter parsed from the input is displayed under the field.
    fn draw_exts_input(&mut self) {
        Ui::push_tint(self.appearence.input_tint);
        Ui::label("File Extensions (ie: .txt, .md, .rs):").draw();
        Ui::same_line();
        Ui::input("documents1_exts", &mut self.exts_input)
            .size(self.appearence.scale_size(Vec2::new(0.17, 0.0)))
            .edit();
        Ui::pop_tint();

        Ui::push_text_style(self.appearence.small_style);
        let exts = parse_exts(&self.exts_input);
        let filter_text = if exts.is_empty() { "no filter (all files)".to_string() } else { exts.join(" ") };
        Ui::label(format!("filter: {filter_text}")).draw();
        Ui::pop_text_style();
    }

    /// The language of the [`FileBrowserB`] texts: a `right_arrow` button showing the current [`locale`], cycling
    /// through the [`available_locales`] on press, with the whole list displayed beside it, the current one
    /// tinted. The change is global: every already open browser re-translates its texts on the next frame.
    fn draw_locale_chooser(&mut self) {
        let current = locale();

        // The right arrow button shows the current locale and cycles to the next available one on press (back
        // to the first after the last), on the model of the display refresh rate button of the demos program.
        if Ui::button(format!("language: {current}")).image(&self.right_arrow).press() {
            let next = self
                .locales
                .iter()
                .position(|l| l == &current)
                .map(|index| self.locales[(index + 1) % self.locales.len()].clone())
                .or_else(|| self.locales.first().cloned());
            if let Some(next) = next {
                set_locale(&next);
                Log::diag(format!("Documents1 locale set to {next}"));
            }
        }
    }

    /// The last event received from the browser for each mode, with the smallest text of the window. The demo does NOT
    /// delete anything: the delete events are only displayed.
    fn draw_events_received(&mut self) {
        Ui::push_text_style(self.appearence.title_style);
        Ui::label("Last events received:").draw();
        Ui::pop_text_style();

        Ui::push_text_style(self.appearence.list_style);
        // The log area takes all the remaining space of the window layout, scrollbars included.
        let remaining = Ui::get_layout_remaining();
        let text = if self.events_received.is_empty() { "-" } else { &self.events_received };
        Ui::text(text)
            .scroll(&mut self.events_scroll, UiScroll::Both)
            .size(remaining)
            .text_align(Align::TopLeft)
            .fit(TextFit::Overflow)
            .draw();
        Ui::pop_text_style();
    }

    /// Launches a [`FileBrowserB`] stepper in the given mode, pre-filled with the demo settings.
    fn spawn_browser(&mut self, mode: PickerMode) {
        // We don't want to propagate scaled settings.
        Ui::settings(UiSettings::default());
        // Counter in the id so several browsers can be open at once, each with their own state.
        self.browser_counter = (self.browser_counter + 1) % 1000;
        let suffix = match mode {
            PickerMode::Open => Self::BROWSER_OPEN_SUFFIX,
            PickerMode::OpenMulti => Self::BROWSER_OPEN_MULTI_SUFFIX,
            PickerMode::Save => Self::BROWSER_SAVE_SUFFIX,
            PickerMode::SelectDirectory => Self::BROWSER_SELECT_DIR_SUFFIX,
            PickerMode::DeleteFile | PickerMode::DeleteFileMulti | PickerMode::DeleteDirectory => {
                Self::BROWSER_DELETE_SUFFIX
            }
        };
        let suffix = format!("{}_{}", suffix, self.browser_counter);

        let mut file_browser = FileBrowserB::default();

        // Start browsing from the location chosen with the demo radio buttons.
        file_browser.dir = BrowseLocation::Pictures
            .get_path(&self.sk_info)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));

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

        // The browser mode: Open (default), OpenMulti, Save, SelectDirectory, DeleteFile, DeleteFileMulti or
        // DeleteDirectory.
        file_browser.picker_mode = mode;

        // A nice handle
        file_browser.appearence.handle_sprite = Sprite::from_file("icons/zoom.png", None, None).ok();

        if mode == PickerMode::Save {
            file_browser.file_name_to_save = "demo_output.txt".into();
        }

        SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone() + &suffix, file_browser));
        Ui::settings(self.appearence.get_ui_settings_scaled());
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
