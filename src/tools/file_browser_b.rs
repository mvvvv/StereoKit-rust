use crate::{
    font::Font,
    framework::{Appearence, ISTEPPER_REMOVED},
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Align, Assets, Hierarchy, Text, TextFit},
    tex::{Tex, TexFormat, TexType},
    ui::{Ui, UiBtnLayout, UiCut, UiPad, UiSliderData, UiVisual, UiWin},
    util::{Color32, Color128, PickerMode, named_colors},
};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const FILE_BROWSER_B_OPEN: &str = "File_Browser_B_open";
pub const FILE_BROWSER_B_SAVE: &str = "File_Browser_B_save";
pub const FILE_BROWSER_B_SELECT_DIR: &str = "File_Browser_B_select_dir";
pub const FILE_BROWSER_B_DELETE: &str = "File_Browser_B_delete";

/// How to sort file entries in [`FileBrowserB`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort alphabetically by name (case-insensitive).
    Name,
    /// Sort by file size.
    Size,
    /// Sort by last modification time (most recent handling depends on ascending flag).
    Modified,
    /// Sort by type: directories first, then files.
    Type,
}

/// A single entry in a directory, enriched with metadata. Computed once at scan time so the draw loop doesn't have
/// to re-read every visible subdirectory each frame.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The file or directory name (final path component).
    pub name: std::ffi::OsString,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// The resolved target of a symbolic link, if this entry is a symlink and the target can be read
    /// (`fs::read_link(path).ok()`). `None` for non-symlinks or unreadable links.
    pub symlink_name: Option<String>,
    /// Whether the entry's metadata could not be read, i.e. `fs::metadata(entry.path()).is_err()`. This is typically
    /// true for broken symbolic links or otherwise unreadable entries.
    pub is_broken: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Number of entries inside this directory (0 for files).
    pub num_entries: usize,
    /// Last modification time, if available.
    pub modified: Option<SystemTime>,
}

impl FileEntry {
    /// Convenience accessor returning the name as a borrowed string slice when possible.
    pub fn name_str(&self) -> std::borrow::Cow<'_, str> {
        self.name.to_string_lossy()
    }

    /// True if the entry name starts with `.` (hidden on Unix-like systems / Android).
    pub fn is_hidden(&self) -> bool {
        self.name_str().starts_with('.')
    }
}

/// A full-featured file browser to open existing files, choose a save path, select a directory or delete files and
/// directories on PC and Android. Should be launched by another stepper set in [`FileBrowserB::caller`].
///
/// ### Fields that should be changed before initialization:
/// * `picker_mode` - What the file browser is for, also driving the window title ("Opening a file", "Creating a file",
///   "Selecting a directory", "Deleting a file" or "Deleting a directory"):
///   - [`PickerMode::Open`] (Default) confirms opening an existing file ([`FILE_BROWSER_B_OPEN`] event),
///   - [`PickerMode::Save`] enters or selects the name of the destination file ([`FILE_BROWSER_B_SAVE`] event),
///   - [`PickerMode::SelectDirectory`] confirms the browsed directory ([`FILE_BROWSER_B_SELECT_DIR`] event),
///   - [`PickerMode::DeleteFile`] / [`PickerMode::DeleteDirectory`] notify the file / directory to delete to the caller
///   ([`FILE_BROWSER_B_DELETE`] event) the browser never deletes anything itself so you have to do it in the caller.
/// * `caller` - The id of the stepper that launched the browser and is waiting for a [`FILE_BROWSER_B_OPEN`],
///   [`FILE_BROWSER_B_SAVE`], [`FILE_BROWSER_B_SELECT_DIR`], [`FILE_BROWSER_B_DELETE`] event.
/// * `dir` - The directory to show. see [`crate::tools::os_api::BrowseLocation`] You can browse outside of this
///   directory unless `root_dir` is set, in which case navigation is clamped to it.
///
/// ### Fields that can be changed before initialization:
/// * `root_dir` - When non-empty, the user cannot navigate above this directory.
/// * `exts` - The file extensions to filter (e.g. `[".png".into(), ".jpg".into()]`).
/// * `window_pose` - The pose where to show the browser window.
/// * `appearence.window_size` - The size of the browser window. Default is `Vec2{x: 0.6, y: 0.8}`.
/// * `max_visible_rows` - Maximum number of file rows shown before scrolling kicks in. 0 means auto (computed from the
///   available list height). In grid mode this is a number of grid rows.
/// * `close_on_select` - If true, the browser closes when the user confirms the selection in the
///   Open / SelectDirectory / delete panels. It is forced to false at start in Save mode.
/// * `file_name_to_save` - Pre-filled name in Save mode.
/// * `appearence.button_tint` - Tint used for directory buttons.
/// * `appearence.input_tint` - Tint used for the input fields.
/// * `appearence.error_tint` - Tint used to signal error entries in the list (dead symlinks, unreadable metadata).
/// * `show_hidden` - Whether hidden files (leading dot) are visible at start.
/// * `grid_view` - Whether to show files in a grid (true) or list (false) at start. Default is false (list).
/// * `appearence.title_style`, `appearence.list_style`, `appearence.label_style`,`appearence.small_style` - The four
///   text styles of the browser, from the biggest (header + breadcrumbs) to the smallest (annotations + status line),
///   giving its UI some relief. Their `layout_height` is multiplied by `appearence.ui_scale` while drawing, so fonts
///   follow the scale handle too.
/// * `preview` - An optional implementor of the [`Previewer`] trait, called every frame while the button of a directory
///   or a file of the list is focused see [`Ui::get_last_element_focused`]. Default is `None` (no preview). See
///   [`BasicPreviewer`] for a ready-to-use previewer drawing a small info panel (name, kind, size, date, path) beside
///   the focused button.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec2, sk::SkInfo, ui::Ui,
///                      tools::file_browser_b::{FileBrowserB, FILE_BROWSER_B_OPEN}};
///
/// let id = "main_b".to_string();
/// const BROWSER_SUFFIX: &str = "_file_browser_b";
/// let mut file_browser = FileBrowserB::default();
/// let sk_info = Some(sk.get_sk_info_clone());
///
/// file_browser.dir = std::path::PathBuf::from("/");
/// file_browser.caller = id.clone();
/// file_browser.window_pose = Ui::popup_pose([0.0, 0.24, 1.25]);
/// file_browser.appearence.window_size = Vec2{x: 0.52, y: 0.52};
/// file_browser.grid_view = true;
/// SkInfo::send_event(&sk_info, StepperAction::add(id.clone() + BROWSER_SUFFIX, file_browser));
///
/// filename_scr = "screenshots/file_browser_b.jpeg"; width_scr = 800; height_scr = 800;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     for event in token.get_event_report() {
///         if let StepperAction::Event(stepper_id, key, value) = event {
///             if stepper_id == &id && key.eq(FILE_BROWSER_B_OPEN) {
///                 println!("Selected file: {}", value);
///             }
///         }
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/file_browser_b.jpeg" alt="screenshot" width="200" />
///
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec2, sk::SkInfo, ui::Ui, util::PickerMode,
///                      tools::file_browser_b::{FileBrowserB, FILE_BROWSER_B_SAVE}};
///
/// let id = "main_b_save".to_string();
/// const BROWSER_SUFFIX: &str = "_file_browser_b_save";
/// let mut file_browser = FileBrowserB::default();
/// let sk_info = Some(sk.get_sk_info_clone());
///
/// file_browser.dir = std::path::PathBuf::from("/");
/// file_browser.picker_mode = PickerMode::Save;
/// file_browser.caller = id.clone();
/// file_browser.window_pose = Ui::popup_pose([0.0, 0.24, 1.25]);
/// file_browser.appearence.window_size = Vec2{x: 0.52, y: 0.52};
/// file_browser.exts = vec![".rs".into()];
/// file_browser.file_name_to_save = "main_tests.rs".into();
/// SkInfo::send_event(&sk_info, StepperAction::add(id.clone() + BROWSER_SUFFIX, file_browser));
///
/// filename_scr = "screenshots/file_save_b.jpeg"; width_scr = 800; height_scr = 800;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     for event in token.get_event_report() {
///         if let StepperAction::Event(stepper_id, key, value) = event {
///             if stepper_id == &id && key.eq(FILE_BROWSER_B_SAVE) {
///                 println!("Save file: {}", value);
///             }
///         }
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/file_save_b.jpeg" alt="screenshot" width="200" />
#[derive(IStepper)]
pub struct FileBrowserB {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub picker_mode: PickerMode,
    pub dir: PathBuf,
    pub root_dir: PathBuf,
    pub exts: Vec<String>,

    /// The look of the browser window: size, scaling, text styles and tints, see [`Appearence`].
    pub appearence: Appearence,
    pub window_pose: Pose,

    pub caller: StepperId,
    pub file_name_to_save: String,
    pub show_hidden: bool,
    pub grid_view: bool,
    /// Maximum number of visible rows before scrolling. 0 means auto (computed from the list height).
    pub max_visible_rows: u32,
    /// Elapsed time (seconds) between automatic directory refreshes. Default is 1 seconds.
    pub auto_refresh_interval: f32,
    /// Optional previewer, an implementor of the [`Previewer`] trait.
    pub preview: Option<Box<dyn Previewer>>,

    entries: Vec<FileEntry>,
    filtered_indices: Vec<usize>,
    /// History of visited directories for the "back" button. The current dir is NOT included.
    history: Vec<PathBuf>,
    /// Confirmation toggle to replace existing file.
    replace_existing_file: bool,
    /// Confirmation toggle of the delete modes: the "Delete" button stays disabled until the user checks it.
    confirm_delete: bool,
    /// Name of the entry selected in the list: the file to open (Open mode), the file/directory to delete
    /// (DeleteFile/DeleteDirectory modes). Also used to highlight the selected row.
    file_selected_name: String,
    sort_by: SortBy,
    sort_ascending: bool,
    scroll: f32,
    search: String,
    new_folder_name: String,
    show_new_folder: bool,
    needs_refresh: bool,
    last_auto_refresh: Option<SystemTime>,
    status: String,

    radio_off: Sprite,
    radio_on: Sprite,
    close: Sprite,
    arrow_up: Sprite,
    arrow_down: Sprite,
    back: Sprite,
    grid: Sprite,
    list: Sprite,
}

unsafe impl Send for FileBrowserB {}

impl Default for FileBrowserB {
    fn default() -> Self {
        Self {
            id: "FileBrowserB".to_string(),
            sk_info: None,

            picker_mode: PickerMode::Open,
            dir: PathBuf::new(),
            root_dir: PathBuf::new(),
            exts: vec![],

            appearence: Appearence::default(),
            window_pose: Ui::popup_pose([0.15, 0.05, 0.10]),

            caller: "".into(),
            file_name_to_save: String::with_capacity(255),
            show_hidden: false,
            grid_view: false,
            max_visible_rows: 0,
            auto_refresh_interval: 1.0,
            preview: None,

            entries: vec![],
            filtered_indices: vec![],
            history: vec![],
            replace_existing_file: false,
            confirm_delete: false,
            file_selected_name: String::with_capacity(255),
            sort_by: SortBy::Type,
            sort_ascending: true,
            scroll: 0.0,
            search: String::with_capacity(255),
            new_folder_name: String::with_capacity(255),
            show_new_folder: false,
            needs_refresh: true,
            last_auto_refresh: None,
            status: String::with_capacity(128),

            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            close: Sprite::close(),
            arrow_up: Sprite::arrow_up(),
            arrow_down: Sprite::arrow_down(),
            back: Sprite::arrow_left(),
            grid: Sprite::grid(),
            list: Sprite::list(),
        }
    }
}

impl FileBrowserB {
    /// Called from `IStepper::initialize`. Returns false to abort the initialization.
    fn start(&mut self) -> bool {
        if self.caller.is_empty() {
            Log::err(
                "FileBrowserB must be called by another stepper (FileBrowserB::caller); \
                 it will notify the selected file to it.",
            );
            return false;
        }

        // Capture the current (possibly user-tweaked) font sizes as the base heights the draw
        // loop multiplies by `ui_scale`, so scaling never compounds over the frames.
        self.appearence.start();

        // We ajust the preview appearence if any
        if let Some(preview) = self.preview.as_deref_mut() {
            preview.set_ui_scale(self.appearence.ui_scale);
        }

        self.refresh();
        Log::diag(format!("FileBrowserB browsing {:?}", self.dir));
        true
    }

    /// Called from `IStepper::step` to check incoming events.
    ///
    /// Listens for the death of the caller: when the stepper that launched this browser is removed, it emits an
    /// [`ISTEPPER_REMOVED`] event and nobody is left waiting for the selection anymore, so the browser closes itself.
    fn check_event(&mut self, id: &StepperId, key: &str, _value: &str) {
        if id == &self.caller && key.eq(ISTEPPER_REMOVED) {
            self.close_me();
        }
    }

    /// Called from `IStepper::step` after `check_event`; draws the UI.
    fn draw(&mut self, _token: &MainThreadToken) {
        // Automatic periodic refresh: re-read the directory if enough time has elapsed.
        if self.auto_refresh_interval > 0.0 {
            let now = SystemTime::now();
            let due = match self.last_auto_refresh {
                Some(last) => {
                    now.duration_since(last).map(|d| d.as_secs_f32() >= self.auto_refresh_interval).unwrap_or(true)
                }
                None => true,
            };
            if due {
                self.needs_refresh = true;
                self.last_auto_refresh = Some(now);
            }
        }

        if self.needs_refresh {
            self.refresh();
            self.needs_refresh = false;
            self.last_auto_refresh = Some(SystemTime::now());
        }

        // Keep the filtered view in sync with search / sort / hidden prefs every frame.
        self.recompute_filtered();

        // The window title starts with the action the browser was launched for, followed by the
        // extension filter (except for the directory modes, where files are not even listed).
        let action_text = match self.picker_mode {
            PickerMode::Open => "Opening a file",
            PickerMode::Save => "Creating a file",
            PickerMode::SelectDirectory => "Selecting a directory",
            PickerMode::DeleteFile => "Deleting a file",
            PickerMode::DeleteDirectory => "Deleting a directory",
        };
        let header_text = match self.picker_mode {
            PickerMode::SelectDirectory | PickerMode::DeleteDirectory => action_text.to_string(),
            _ if self.exts.is_empty() => format!("{action_text} - All file types"),
            _ => format!("{action_text} - Only ({})", self.exts.join(",")),
        };

        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.ui_settings_scaled());

        Ui::push_id(&self.id);
        Ui::window(header_text)
            .pose(&mut self.window_pose)
            .size(self.appearence.window_size * self.appearence.ui_scale)
            .window_type(UiWin::Normal)
            .begin();

        let line = Ui::get_line_height();
        let btn = Vec2::new(line * 1.4, line * 1.4) * self.appearence.ui_scale;

        self.draw_toolbar(btn);

        // The search and sort bars use the secondary label style.
        Ui::push_text_style(self.appearence.label_style);
        self.draw_search_bar();
        self.draw_sort_bar();
        Ui::pop_text_style();

        // The list area: cut a Top section for the scrollable file list.
        // Reserve the status line height so it stays at the bottom.
        let status_h = match self.picker_mode {
            // save name / delete confirm toggle + status line + separator
            PickerMode::Save | PickerMode::DeleteFile | PickerMode::DeleteDirectory => line * 5.5,
            // open/select-dir panel + status line + separator
            PickerMode::Open | PickerMode::SelectDirectory => line * 4.2,
        };
        let remaining_after_bars = Ui::get_layout_remaining();
        let list_h = (remaining_after_bars.y - status_h).max(line * 3.0);

        // Reserve the list area in the flow layout, then push a sub-layout at that position.
        let list_bounds = Ui::layout_reserve(Vec2::new(remaining_after_bars.x, list_h), false, 0.0);
        // layout_push expects the top-left corner of the new layout region.
        // Bounds.center is the center; top-left = (center.x - w/2, center.y + h/2) in SK coords (Y down).
        let list_start = Vec3::new(
            list_bounds.center.x + list_bounds.dimensions.x / 2.0,
            list_bounds.center.y + list_bounds.dimensions.y / 2.0,
            list_bounds.center.z,
        );
        Ui::layout_push(list_start, Vec2::new(remaining_after_bars.x, list_h), false);

        // The file list uses its own style, between the title and the labels.
        Ui::push_text_style(self.appearence.list_style);
        self.draw_list(line, list_h, self.picker_mode);
        Ui::pop_text_style();

        Ui::layout_pop();

        Ui::hseparator();

        // The Open/Save/Select/Delete confirmation row lives in the main window
        // flow, OUTSIDE the list sub-layout, so the vertical slider only spans the file list itself
        // and not the input/button row.
        match self.picker_mode {
            PickerMode::Open => {
                Ui::push_text_style(self.appearence.label_style);
                self.draw_open_panel();
                Ui::pop_text_style();
                Ui::hseparator();
            }
            PickerMode::Save => {
                Ui::push_text_style(self.appearence.label_style);
                self.draw_save_panel(line);
                Ui::pop_text_style();
                Ui::hseparator();
            }
            PickerMode::SelectDirectory => {
                Ui::push_text_style(self.appearence.label_style);
                self.draw_select_dir_panel();
                Ui::pop_text_style();
                Ui::hseparator();
            }
            PickerMode::DeleteFile => {
                Ui::push_text_style(self.appearence.label_style);
                self.draw_delete_file_panel(line);
                Ui::pop_text_style();
                Ui::hseparator();
            }
            PickerMode::DeleteDirectory => {
                Ui::push_text_style(self.appearence.label_style);
                self.draw_delete_dir_panel(line);
                Ui::pop_text_style();
                Ui::hseparator();
            }
        }

        self.draw_status_line();
        Ui::window_end();
        Ui::pop_id();

        // Restore the caller's UiSettings exactly as they were before this window was drawn.
        Ui::settings(prev_settings);

        // Scale handle: a small grab-able knob in world space, anchored to the window in its
        // local space so it follows the window when it moves. Dragging it along the window local
        // X resizes the width, along Y the height, and along Z (towards the user) the whole scale.
        // While grabbed, the live scale is forwarded to the child windows: the preview panel text
        // styles follow the browser scale.
        if let Some(scale) = self.appearence.scale_handle(&self.window_pose, "fb_scale_handle")
            && let Some(preview) = self.preview.as_deref_mut()
        {
            preview.set_ui_scale(scale);
        }
    }

    fn close_me(&self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
    }

    // ----------------------------------------------------------------------- UI sections

    fn draw_toolbar(&mut self, btn: Vec2) {
        // Close button.
        let size_meter = if self.show_new_folder && self.new_folder_allowed() { btn.y * 3.6 } else { btn.y * 2.4 };
        Ui::layout_push_cut(UiCut::Top, size_meter, false);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), Some(UiPad::Outside));

        // The toolbar buttons and inputs use the secondary label style; the header and the
        // breadcrumbs below use the bigger title style, for some relief between the parts.
        Ui::push_text_style(self.appearence.label_style);
        if Ui::button("fb_close").image(&self.close).image_layout(UiBtnLayout::CenterNoText).size(btn).press() {
            self.close_me();
        }

        // Up
        Ui::same_line();
        Ui::hspace(0.01);
        Ui::push_enabled(self.can_go_up(), None);
        if Ui::button("fb_up").image(&self.arrow_up).image_layout(UiBtnLayout::CenterNoText).size(btn).press() {
            self.go_up();
        }
        Ui::pop_enabled();

        // Back (navigate to previous dir in history)
        Ui::same_line();
        Ui::push_enabled(!self.history.is_empty(), None);
        if Ui::button("fb_back").image(&self.back).image_layout(UiBtnLayout::CenterNoText).size(btn).press()
            && let Some(prev) = self.history.pop()
        {
            self.dir = prev;
            self.scroll = 0.0;
            self.file_selected_name.clear();
            self.needs_refresh = true;
        }
        Ui::pop_enabled();

        // Grid/List view toggle: shows the grid sprite when off, list sprite when on
        Ui::same_line();
        Ui::hspace(0.01);
        Ui::toggle("fb_view_mode", &mut self.grid_view)
            .images(&self.list, &self.grid)
            .image_layout(UiBtnLayout::CenterNoText)
            .size(btn)
            .interact();

        // New folder (Save and SelectDirectory modes)
        if self.new_folder_allowed() {
            Ui::same_line();
            let at = Ui::get_layout_at();
            if Ui::button("New folder").at(at, Vec2::new(btn.x * 4.0, btn.y)).press() {
                self.show_new_folder = !self.show_new_folder;
                if self.show_new_folder {
                    self.new_folder_name.clear();
                }
            }
        }
        Ui::pop_text_style();

        Ui::next_line();

        if self.show_new_folder && self.new_folder_allowed() {
            Ui::push_text_style(self.appearence.label_style);
            self.draw_new_folder_input();
            Ui::pop_text_style();
        }

        Ui::push_text_style(self.appearence.title_style);
        self.draw_breadcrumbs(btn);
        Ui::pop_text_style();
        Ui::layout_pop();
    }

    fn draw_breadcrumbs(&mut self, btn: Vec2) {
        // we reduce the size of the buttons:
        let btn = btn * 0.9;

        // Build path components from filesystem root to current dir.
        let components: Vec<PathBuf> =
            self.dir.ancestors().collect::<Vec<_>>().into_iter().rev().map(PathBuf::from).collect();

        // If root_dir is set, only show breadcrumbs from root_dir onwards.
        // If root_dir is empty (default), show the full path.
        let start_idx = if self.root_dir.as_os_str().is_empty() {
            0
        } else {
            components.iter().position(|c| *c == self.root_dir).unwrap_or(0)
        };

        // (label, target path) pairs to display, from root to current dir.
        let labels: Vec<(String, PathBuf)> = components
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= start_idx)
            .map(|(_, comp)| {
                let label = comp
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| comp.to_string_lossy().to_string());
                (label, comp.clone())
            })
            .filter(|(label, _)| !label.is_empty())
            .collect();
        if labels.is_empty() {
            return;
        }

        // Reserve one full-width row in the flow layout (layout_reserve also ends the line),
        // then place the buttons with `.at()` (button_at): unlike flow buttons, these can simply
        // overflow past the window's edge when the path is too long, which is fine in VR — the
        // window floats in space, the user can lean/move to read it.
        let gutter = self.appearence.ui_settings_scaled().gutter;
        // Breadcrumb buttons are smaller than regular buttons.
        let at = Ui::get_layout_at();
        let row_w = Ui::get_layout_remaining().x;
        Ui::layout_reserve(Vec2::new(row_w, btn.y), true, 0.0);

        // Estimated width of a breadcrumb button
        let btn_w = |label: &str| label.chars().count() as f32 * btn.x * 0.15 + btn.y;

        // Deferred navigation target, applied after the borrow of the labels.
        let mut navigate: Option<PathBuf> = None;

        Ui::push_tint(self.appearence.button_tint);
        Ui::push_text_style(self.appearence.small_style);
        // The UI layout advances leftward: x starts at the row's top-left corner and decreases,
        // unbounded on the right — the path may overflow the window.
        let mut x = at.x;
        for (label, target) in &labels {
            let w = btn_w(label);
            if Ui::button(label).at([x, at.y, at.z], [w, btn.y]).press() {
                navigate = Some(target.clone());
            }
            x -= w + gutter / 3.0;
        }
        Ui::pop_tint();
        Ui::pop_text_style();

        if let Some(target) = navigate {
            self.navigate_to(target);
        }
    }

    fn draw_new_folder_input(&mut self) {
        Ui::push_tint(self.appearence.input_tint);
        Ui::label("New folder:").draw();
        Ui::same_line();
        Ui::input("fb_new_folder_name", &mut self.new_folder_name)
            .size(Vec2::new(0.2 * self.appearence.ui_scale, 0.0))
            .edit();
        Ui::same_line();
        Ui::push_enabled(!self.new_folder_name.trim().is_empty(), None);
        if Ui::button("create").press() {
            let new_path = self.dir.join(self.new_folder_name.trim());
            match std::fs::create_dir_all(&new_path) {
                Ok(_) => {
                    self.show_new_folder = false;
                    self.needs_refresh = true;
                    Log::diag(format!("Created directory {:?}", new_path));
                }
                Err(e) => {
                    self.status = format!("Cannot create folder: {e}");
                    Log::warn(format!("FileBrowserB cannot create folder {new_path:?}: {e}"));
                }
            }
        }
        Ui::pop_enabled();
        Ui::same_line();
        if Ui::button_round("cancel", &self.close, 0.03 * self.appearence.ui_scale).press() {
            self.show_new_folder = false;
        }
        Ui::pop_tint();
        Ui::next_line();
    }

    fn draw_search_bar(&mut self) {
        Ui::push_tint(self.appearence.input_tint);
        Ui::label("search:").draw();
        Ui::same_line();
        Ui::input("fb_search", &mut self.search).size(Vec2::new(0.2 * self.appearence.ui_scale, 0.0)).edit();
        if !self.search.is_empty() {
            Ui::same_line();
            if Ui::button_round("clear", &self.close, 0.03 * self.appearence.ui_scale).press() {
                self.search.clear();
            }
        }
        Ui::pop_tint();
        Ui::next_line();
    }

    fn draw_sort_bar(&mut self) {
        // Hidden toggle
        if Ui::toggle("hidden", &mut self.show_hidden).interact().is_some() {
            self.needs_refresh = true;
        }
        Ui::same_line();

        let sort_btn = |browser: &mut Self, label: &str, mode: SortBy| {
            let active = browser.sort_by == mode;
            // Show the sort indicator sprite only on the active column.
            let indicator = if active {
                if browser.sort_ascending { Some(&browser.arrow_up) } else { Some(&browser.arrow_down) }
            } else {
                None
            };

            let mut builder = Ui::button(label);
            if let Some(spr) = indicator {
                builder = builder.image(spr).image_layout(UiBtnLayout::Right);
            }
            if builder.press() {
                if active {
                    browser.sort_ascending = !browser.sort_ascending;
                } else {
                    browser.sort_by = mode;
                    browser.sort_ascending = true;
                }
                browser.recompute_filtered();
            }
            Ui::same_line();
        };

        sort_btn(self, "name", SortBy::Name);
        sort_btn(self, "size", SortBy::Size);
        sort_btn(self, "date", SortBy::Modified);
        sort_btn(self, "type", SortBy::Type);

        Ui::next_line();
    }

    /// Shared rendering of the scrollable file list (vertical slider + grid/list view), used by all the picker modes.
    /// Only the selection target and the click behaviour differ, which are all driven by `mode`.
    ///
    /// Symlinks show their target path (`-> target`) and entries whose metadata could not be read are marked with an
    /// error, both drawn as a small multi-line [`Ui::text`] (lines joined with `\n`) slightly below the entry name —
    /// in grid view, the lines break after `/` or `\` separators (too long names are hard-split), so the
    /// `TextFit::Exact` adjustment is proportional (one uniform scale for the whole note) — and tinted with
    /// [`Appearence::error_tint`] for broken entries.
    fn draw_list(&mut self, line: f32, list_h: f32, mode: PickerMode) {
        // If the directory to display doesn't exist
        if !self.dir.is_dir() {
            let at = Ui::get_layout_at();
            let size = Ui::get_layout_remaining();
            Ui::push_tint(self.appearence.error_tint);
            Ui::text(format!("Error: directory does not exist!\n{:?}", self.dir))
                .at([at.x, at.y, at.z - 0.01], [size.x, size.y])
                .text_align(Align::TopLeft)
                .fit(TextFit::Squeeze)
                .draw();
            Ui::pop_tint();
            return;
        }

        let mut row_clicked: Option<usize> = None;
        let mut dir_clicked: Option<usize> = None;

        let total = self.filtered_indices.len();

        // Compute the scrollable list area dimensions. The authoritative height is the `list_h`
        // reserved by `draw` for this list; the pushed layout may report a slightly different
        // remaining (rounding, panel padding...), so take the smallest of both to guarantee the
        // drawn buttons never overflow the reserved list area.
        let list_area = Ui::get_layout_remaining();
        let usable_h = list_h.min(list_area.y);
        let slider_w = line * 0.7;

        // Effective height of ONE row, so the drawn buttons always fit in the reserved list
        // height:
        let row_h = if self.grid_view { line * 2.0 } else { line };

        // In list mode each row holds 1 entry. In grid mode, the column count adapts to the window
        // width: fill it with as many columns as possible.
        const GRID_MIN_CELL_CHARS: f32 = 5.0;
        let settings = self.appearence.ui_settings_scaled();
        let columns = if self.grid_view {
            (((list_area.x - slider_w).max(0.0) + settings.gutter) / (line * GRID_MIN_CELL_CHARS + settings.padding))
                .floor()
                .max(1.0) as usize
        } else {
            1usize
        };
        // Dynamic row count based on the available height and the current view mode.
        let visible_rows = self.visible_rows_count(usable_h, row_h);
        let total_rows = total.div_ceil(columns);

        // Cut a right portion for the scrollbar.
        Ui::layout_push_cut(UiCut::Right, slider_w, false);
        let max_scroll = (total_rows as f32 - visible_rows as f32).max(0.0);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        if self.scroll < 0.0 {
            self.scroll = 0.0;
        }
        if total_rows > visible_rows {
            self.draw_scrollbar(slider_w, list_area.y, max_scroll, visible_rows, total_rows);
        }
        Ui::layout_pop();

        // Remaining area for the list content.
        let content_w = Ui::get_layout_remaining().x;

        if self.grid_view {
            // ----- GRID MODE (matrix of `columns` x `visible_rows`) -----
            let grid_w = (content_w + settings.gutter) / columns as f32 - settings.padding;
            let grid_h = line * 2.0;
            let grid_size = Vec2::new(grid_w, grid_h);

            // Scroll is expressed in grid rows: each skipped row == `columns` entries.
            let start_row = self.scroll as usize;
            for row in 0..visible_rows {
                // Belt and braces: never draw a row that would not fit the reserved list area.
                if Ui::get_layout_remaining().y < row_h {
                    break;
                }
                let mut drew_any = false;
                for col in 0..columns {
                    let idx = (start_row + row) * columns + col;
                    if idx >= total {
                        break;
                    }
                    if drew_any {
                        Ui::same_line();
                    }
                    drew_any = true;
                    let entry_idx = self.filtered_indices[idx];
                    let entry = &self.entries[entry_idx];
                    let name = entry.name_str().to_string();
                    // Grid cells are narrow, so wrap long names into multiple lines to avoid horizontal overflow.
                    const NAME_MAX_CHARS: usize = 25;
                    let display_name = if name.chars().count() > NAME_MAX_CHARS {
                        Self::wrap_chars_lines(&name, NAME_MAX_CHARS).join("\n")
                    } else {
                        name.clone()
                    };
                    // Top-left corner of the cell, to anchor the symlink/error note below the name.
                    let at = Ui::get_layout_at();

                    let entry_annotation = Self::entry_annotation(entry);
                    // Copies for `run_preview_if_focused`: it takes `&mut self`, which cannot
                    // coexist with the still-live borrows of `self` used by the note below.
                    let (dir_tint, error_tint, is_broken) =
                        (self.appearence.button_tint, self.appearence.error_tint, entry.is_broken);

                    // Directories are navigation buttons, except in DeleteDirectory mode where they
                    // are the deletion targets and become selectable radios.
                    if entry.is_dir && mode != PickerMode::DeleteDirectory {
                        // Broken directories are tinted with `error_tint` instead of `dir_tint`.
                        Ui::push_tint(self.appearence.button_tint);
                        if Ui::button(&display_name)
                            .size(grid_size)
                            .text_align(if entry_annotation.is_some() { Align::TopLeft } else { Align::Center })
                            .press()
                        {
                            dir_clicked = Some(entry_idx);
                        }
                        Ui::pop_tint();
                    } else {
                        let selected = self.selected_name(mode) == name.as_str();
                        if Ui::radio(&display_name, selected)
                            .size(grid_size)
                            .images(&self.radio_off, &self.radio_on)
                            .image_layout(UiBtnLayout::Left)
                            .text_align(if entry_annotation.is_some() { Align::TopLeft } else { Align::Center })
                            .press()
                        {
                            row_clicked = Some(entry_idx);
                        }
                    }

                    // Previewer callback while this entry's button (dir or file) is focused. Called
                    // right after the button/radio, before any other element steals
                    // `Ui::get_last_element_focused` / `Ui::get_layout_last`.
                    self.run_preview_if_focused(&name);

                    // Symlink target / error note, slightly below the name in the cell, as a
                    // SINGLE text with `\n` separated lines: TextFit::Exact then applies ONE
                    // uniform scale to the whole block, so the adjustment is proportional.
                    if let Some(note) = entry_annotation {
                        let max_chars = 35;
                        let note = Self::wrap_chars(&note, max_chars);

                        Ui::push_tint(if is_broken { error_tint } else { dir_tint });
                        Ui::push_text_style(self.appearence.small_style);
                        Ui::text(note)
                            .at([at.x - 0.04, at.y - line * 0.8, at.z - 0.01], [grid_w * 0.7, line * 1.1])
                            .fit(TextFit::Exact)
                            .draw();
                        Ui::pop_text_style();
                        Ui::pop_tint();
                    }
                }
                Ui::next_line();
            }
        } else {
            // ----- LIST MODE (3 aligned columns: name | size | date) -----
            let gutter = self.appearence.ui_settings_scaled().gutter;
            let name_w = content_w * 0.55;
            let size_w = content_w * 0.20;
            let date_w = content_w - name_w - size_w - gutter * 2.0;

            let start_row = self.scroll as usize;
            for visible_i in 0..visible_rows {
                // Belt and braces: never draw a row that would not fit the reserved list area.
                if Ui::get_layout_remaining().y < row_h {
                    break;
                }
                let idx = start_row + visible_i;
                if idx >= total {
                    break;
                }
                let entry_idx = self.filtered_indices[idx];
                let entry = &self.entries[entry_idx];
                let name = entry.name_str().to_string();
                // Top-left corner of the row, to anchor the symlink/error note below the name.
                let at = Ui::get_layout_at();
                let (size_text, date_text) = if entry.is_broken {
                    // Broken entry (dead symlink / unreadable metadata): signal the error instead of
                    // size and date, the whole row is tinted with `error_tint` below.
                    ("error!".to_string(), "-".to_string())
                } else if entry.is_dir {
                    (format!("{} item(s)", entry.num_entries), format_date(entry.modified))
                } else {
                    (format_size(entry.size), format_date(entry.modified))
                };

                let entry_annotation = Self::entry_annotation(entry);
                // Copies for `run_preview_if_focused`: it takes `&mut self`, which cannot coexist
                // with the still-live borrows of `self` used by the note below.
                let (dir_tint, error_tint, is_broken) =
                    (self.appearence.button_tint, self.appearence.error_tint, entry.is_broken);

                // Column 1: name (interactive). Directories are navigation buttons, except in
                // DeleteDirectory mode where they are the deletion targets and become selectable
                // radios (see the grid branch above for the rationale).
                if entry.is_dir && mode != PickerMode::DeleteDirectory {
                    Ui::push_tint(self.appearence.button_tint);
                    if Ui::button(&name)
                        .size(Vec2::new(name_w, 0.0))
                        .text_align(if entry_annotation.is_some() { Align::TopCenter } else { Align::Center })
                        .press()
                    {
                        dir_clicked = Some(entry_idx);
                    }
                    Ui::pop_tint();
                } else {
                    let selected = self.selected_name(mode) == name.as_str();
                    if Ui::radio(&name, selected)
                        .size(Vec2::new(name_w, 0.0))
                        .images(&self.radio_off, &self.radio_on)
                        .image_layout(UiBtnLayout::Left)
                        .text_align(if entry_annotation.is_some() { Align::TopCenter } else { Align::Center })
                        .press()
                    {
                        row_clicked = Some(entry_idx);
                    }
                }

                // Previewer callback while this entry's button (dir or file) is focused. Called
                // right after the button/radio, before the size/date labels of the row steal
                // `Ui::get_layout_last`.
                self.run_preview_if_focused(&name);

                // Column 2: size / item count (non-interactive label)
                Ui::same_line();
                Ui::label(size_text).size(Vec2::new(size_w, 0.0)).use_padding(false).draw();

                // Column 3: date (non-interactive label)
                Ui::same_line();
                Ui::label(date_text).size(Vec2::new(date_w, 0.0)).use_padding(false).draw();

                // Symlink target / error note, slightly below the name in the row. The list row
                // is a single line tall, so the note is drawn as-is (Squeeze only scales down).
                if let Some(note) = entry_annotation {
                    Ui::push_tint(if is_broken { error_tint } else { dir_tint });
                    Ui::push_text_style(self.appearence.small_style);
                    Ui::text(note)
                        .at([at.x - 0.01, at.y - line * 0.8, at.z - 0.01], [name_w, line * 0.3])
                        .fit(TextFit::Squeeze)
                        .draw();
                    Ui::pop_text_style();
                    Ui::pop_tint();
                }
            }
        }

        // Handle deferred actions so we don't borrow self during the draw loop.
        if let Some(i) = dir_clicked {
            let target = self.dir.join(&self.entries[i].name);
            self.navigate_to(target);
        }
        if let Some(i) = row_clicked {
            let name = self.entries[i].name_str().to_string();
            match mode {
                PickerMode::Save => {
                    self.file_name_to_save = name;
                    self.replace_existing_file = false;
                }
                // The other modes only select the entry (the file to open, or the file /
                // directory to delete); confirmation happens in the bottom panel of the mode.
                _ => {
                    self.file_selected_name = name;
                    self.confirm_delete = false;
                }
            }
        }
    }

    /// The vertical scrollbar of the file list. Instead of a plain [`Ui::vslider`],  which allows a custom rendering.
    fn draw_scrollbar(&mut self, width: f32, height: f32, max_scroll: f32, visible_rows: usize, total_rows: usize) {
        let bar_bounds =
            Ui::layout_reserve(Vec2::new(width, height), false, self.appearence.ui_settings_scaled().depth);
        let tlb = bar_bounds.tlb();

        // Thumb size: same width ratio as StereoKit's vslider push button (`size_min * 0.55` for a
        // vertical slider), height proportional to the visible fraction of the list, with a floor
        // so it never gets too small to grab on very large directories.
        let thumb_w = width * 0.55;
        let thumb_h = (height * visible_rows as f32 / total_rows as f32).max(thumb_w);
        let thumb_size = Vec2::new(thumb_w, thumb_h);

        let mut value = Vec2::new(0.0, self.scroll);
        let mut slider = UiSliderData::default();
        let id = Ui::stack_hash("fb_scroll");
        Ui::slider_behavior(
            tlb,
            Vec2::new(width, height),
            id,
            &mut value,
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, max_scroll),
            thumb_size,
            thumb_size
                + Vec2::new(self.appearence.ui_settings_scaled().padding, self.appearence.ui_settings_scaled().padding)
                    * 2.0,
            None, // UiConfirm::Push, the vslider default
            &mut slider,
        );

        // Keep the scroll expressed in whole rows, like the previous `.step(1.0)` vslider.
        let prev_row = self.scroll.round();
        self.scroll = value.y.round().clamp(0.0, max_scroll);

        let focus = Ui::get_anim_focus(id, slider.focus_state, slider.active_state);
        // `button_center` is the center of the thumb, `draw_element` expects its top-left corner.
        let thumb_at = Vec3::new(slider.button_center.x + thumb_w / 2.0, slider.button_center.y + thumb_h / 2.0, tlb.z);

        // Track: full-height thin inactive line behind the thumb.
        Ui::draw_element(
            UiVisual::SliderLine,
            None,
            tlb,
            Vec3::new(width, height, self.appearence.ui_settings_scaled().depth * 0.1),
            focus,
        );
        // Thumb: SliderLine with a height proportional to the visible part of the directory.
        Ui::draw_element(
            UiVisual::SliderPush,
            None,
            thumb_at,
            Vec3::new(thumb_w, thumb_h, self.appearence.ui_settings_scaled().depth),
            focus,
        );

        // Same sound feedback as StereoKit's sliders: activation on/off, then a tick per row.
        if slider.active_state.is_just_active() {
            Ui::play_sound_on_off(UiVisual::SliderPush, id, thumb_at);
        }
        if slider.active_state.is_active() && prev_row != self.scroll {
            Ui::play_sound_on(UiVisual::SliderPush, thumb_at);
        }
    }

    /// Annotation drawn slightly below the entry name in the list and grid views: symlink entries show their target
    /// path (`-> target`) and entries whose metadata could not be read (`is_broken`) get an explicit error marker.
    /// Returns `None` for plain entries.
    ///
    /// The grid view splits it into several lines of equal length with [`FileBrowserB::wrap_chars`] so the
    /// `TextFit::Exact` scaling stays proportional (same glyph size) on every line.
    fn entry_annotation(entry: &FileEntry) -> Option<String> {
        match (&entry.symlink_name, entry.is_broken) {
            (Some(target), true) => Some(format!("-> {target} (broken link)")),
            (Some(target), false) => Some(format!("-> {target}")),
            (None, true) => Some("[error] unreadable entry".to_string()),
            (None, false) => None,
        }
    }

    /// Wraps `text` into lines of at most `max_chars` columns, preferring to break lines AFTER `/` or `\` path
    /// separators so paths stay readable. Only a segment without any separator that is longer than `max_chars` (a too
    /// long file name) gets hard-split. The lines are joined with `\n`, so a single [`Ui::text`] call draws the whole
    /// note with one uniform `TextFit::Exact` scale — the adjustment stays proportional.
    ///
    /// The result is also padded with blank lines up to a minimum of 3 lines, so the scale of a short annotation note
    /// matches the one of a longer note. Use [`FileBrowserB::wrap_chars_lines`] for the raw wrapped lines WITHOUT that
    /// padding (e.g. for button texts, where a trailing blank line would shift the text upward).
    fn wrap_chars(text: &str, max_chars: usize) -> String {
        let mut lines = Self::wrap_chars_lines(text, max_chars);
        if lines.len() == 1 {
            lines.push(String::from(" "));
            lines.push(String::from(" "));
        } else if lines.len() == 2 {
            lines.push(String::from(" "));
        }
        lines.join("\n")
    }

    /// The core of [`FileBrowserB::wrap_chars`]: wraps `text` into lines of at most `max_chars` columns, preferring to
    /// break lines AFTER `/` or `\` path separators so paths stay readable, and hard-splitting only a separator-less
    /// segment longer than a full line (a too long file name). Returns the raw lines, WITHOUT the vertical padding
    /// `wrap_chars` adds for the annotation notes.
    fn wrap_chars_lines(text: &str, max_chars: usize) -> Vec<String> {
        // Split into separator-terminated segments, hard-splitting any segment longer than a
        // full line (a too long name with no separator to break on).
        let mut chunks: Vec<String> = Vec::new();
        for seg in text.split_inclusive(|c: char| ['/', '\\', '_', ' '].contains(&c)) {
            let mut chars = seg.chars();
            loop {
                let chunk: String = chars.by_ref().take(max_chars).collect();
                if chunk.is_empty() {
                    break;
                }
                chunks.push(chunk);
            }
        }

        // Greedily pack the chunks into lines of at most `max_chars`, breaking after separators.
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();
        for chunk in chunks {
            if !current.is_empty() && current.chars().count() + chunk.chars().count() > max_chars {
                lines.push(std::mem::take(&mut current));
            }
            current.push_str(&chunk);
        }
        if !current.is_empty() {
            lines.push(current);
        }
        lines
    }

    /// Name used to highlight the currently selected entry: the file selected for opening /
    /// deletion (Open and delete modes), or the pre-filled/typed save name in Save mode.
    fn selected_name(&self, mode: PickerMode) -> &str {
        match mode {
            PickerMode::Save => &self.file_name_to_save,
            _ => &self.file_selected_name,
        }
    }

    /// Runs [`FileBrowserB::preview`] when the directory or file button just drawn is focused.
    ///
    /// Must be called right after the `Ui::button`/`Ui::radio` call of a list entry, while it is still the
    /// "last element": [`Ui::get_last_element_focused`] then gives the focus state of that very button, and
    /// [`Ui::get_layout_last`] its layout bounds. The bounds center (window-local layout coordinates) is converted
    /// into a world-space `bouton_pose` through the current UI hierarchy, exactly like StereoKit's `ui_popup_pose`
    /// does when it attaches a popup to the focused element.
    fn run_preview_if_focused(&mut self, name: &str) {
        let Some(previewer) = &mut self.preview else { return };
        if !Ui::get_last_element_focused().is_active() {
            return;
        }
        let file_path = self.dir.join(name);
        let bounds = Ui::get_layout_last();
        let button_pose = Pose {
            position: Hierarchy::to_world_point(bounds.center),
            orientation: Hierarchy::to_world_rotation(Quat::IDENTITY),
        };
        previewer.preview(file_path, self.window_pose, button_pose);
    }

    /// The Open-mode confirmation row (file name + Open button).
    /// Drawn in the main window flow, below the scrollable file list, so the list's vertical slider does not span this
    /// row. Selecting a file in the list only fills the input here; the user confirms opening it by pressing "Open".
    fn draw_open_panel(&mut self) {
        // Validate the extension and that the file actually exists in the current directory: Open
        // mode requires an existing file.
        let mut ext_ok = self.exts.is_empty();
        for ext in &self.exts {
            if self.file_selected_name.to_lowercase().ends_with(&ext.to_lowercase()) {
                ext_ok = true;
                break;
            }
        }
        let name_ok = !self.file_selected_name.trim().is_empty();
        let file = self.dir.join(&self.file_selected_name);
        let ok_to_open = name_ok && ext_ok && file.is_file();

        Ui::push_tint(self.appearence.input_tint);
        Ui::push_enabled(ok_to_open, None);
        if Ui::button("Open").press() {
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(self.caller.as_str(), FILE_BROWSER_B_OPEN, file.to_str().unwrap_or("path_error")),
            );
            self.close_me();
        }
        Ui::pop_enabled();
        Ui::same_line();
        Ui::label(&self.file_selected_name).draw();

        Ui::pop_tint();
        Ui::next_line();
    }

    /// The Save-mode input row (file name + Save button + "replace existing file" toggle).
    /// Drawn in the main window flow, below the scrollable file list, so the list's vertical slider does not span this
    /// row.
    fn draw_save_panel(&mut self, line: f32) {
        Ui::push_tint(self.appearence.input_tint);

        let file = self.dir.join(&self.file_name_to_save);

        // Validate extension.
        let mut ext_ok = self.exts.is_empty();
        for ext in &self.exts {
            if self.file_name_to_save.to_lowercase().ends_with(&ext.to_lowercase()) {
                ext_ok = true;
                break;
            }
        }
        let name_ok = !self.file_name_to_save.trim().is_empty();

        let ok_to_save = name_ok && ext_ok && (!file.exists() || self.replace_existing_file);
        Ui::push_enabled(ok_to_save, None);
        Ui::same_line();
        if Ui::button("Save").press() {
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(self.caller.as_str(), FILE_BROWSER_B_SAVE, file.to_str().unwrap_or("path_error")),
            );
            self.close_me();
        }
        Ui::pop_enabled();
        Ui::same_line();

        Ui::label("file name:").draw();
        Ui::same_line();
        Ui::input("fb_filename", &mut self.file_name_to_save).size(Vec2::new(0.0, 0.0)).edit();

        if file.exists() && name_ok {
            Ui::toggle("replace existing file", &mut self.replace_existing_file).interact();
        } else {
            Ui::vspace(line * 1.34);
            self.replace_existing_file = false;
        }

        Ui::pop_tint();
        Ui::next_line();
    }

    /// The SelectDirectory-mode confirmation row (current directory + Select button): navigate to the directory to
    /// select (files are hidden in this mode), then confirm. Drawn in the main window flow, below the scrollable file
    /// list, so the list's vertical slider does not span this row.
    fn draw_select_dir_panel(&mut self) {
        let dir_ok = self.dir.is_dir();
        Ui::push_tint(self.appearence.input_tint);
        Ui::push_enabled(dir_ok, None);
        if Ui::button("Select").press() {
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(
                    self.caller.as_str(),
                    FILE_BROWSER_B_SELECT_DIR,
                    self.dir.to_str().unwrap_or("path_error"),
                ),
            );
            self.close_me();
        }
        Ui::pop_enabled();
        Ui::same_line();
        Ui::label("directory:").draw();
        Ui::same_line();
        Ui::label(self.dir.to_string_lossy().to_string()).draw();
        Ui::pop_tint();
        Ui::next_line();
    }

    /// The DeleteFile-mode confirmation row (selected file + Delete button + confirm toggle). Drawn in the main window
    /// flow, below the scrollable file list, so the list's vertical slider does not span this row. The "Delete" button
    /// stays disabled until the "confirm deletion" toggle is checked, mirroring the "replace existing file" safety of
    /// Save mode. Pressing it only notifies the caller (see [`FileBrowserB::send_delete_event`]): the browser never
    /// deletes anything itself.
    fn draw_delete_file_panel(&mut self, line: f32) {
        let name_ok = !self.file_selected_name.trim().is_empty();
        let file = self.dir.join(&self.file_selected_name);
        let is_target = name_ok && file.is_file();
        let ok_to_delete = is_target && self.confirm_delete;

        Ui::push_tint(self.appearence.input_tint);
        Ui::push_enabled(ok_to_delete, None);
        if Ui::button("Delete").press() {
            self.send_delete_event();
        }
        Ui::pop_enabled();
        Ui::same_line();
        Ui::label("file to delete:").draw();
        Ui::same_line();
        Ui::label(&self.file_selected_name).draw();

        if is_target {
            Ui::toggle("confirm deletion", &mut self.confirm_delete).interact();
        } else {
            Ui::vspace(line * 1.34);
            self.confirm_delete = false;
        }

        Ui::pop_tint();
        Ui::next_line();
    }

    /// The DeleteDirectory-mode confirmation row (selected directory + Delete button + confirm toggle). Same layout as
    /// [`FileBrowserB::draw_delete_file_panel`], but the selected entry is one of the directories of the list (drawn
    /// as radios in this mode). The toggle reads "confirm recursive deletion" as a hint that deleting a directory
    /// generally wipes its whole content; the actual deletion is the caller's business
    /// (see [`FileBrowserB::send_delete_event`]).
    fn draw_delete_dir_panel(&mut self, line: f32) {
        let name_ok = !self.file_selected_name.trim().is_empty();
        let dir_target = self.dir.join(&self.file_selected_name);
        let is_target = name_ok && dir_target.is_dir();
        let ok_to_delete = is_target && self.confirm_delete;

        Ui::push_tint(self.appearence.input_tint);
        Ui::push_enabled(ok_to_delete, None);
        if Ui::button("Delete").press() {
            self.send_delete_event();
        }
        Ui::pop_enabled();
        Ui::same_line();
        Ui::label("directory to delete:").draw();
        Ui::same_line();
        Ui::label(&self.file_selected_name).draw();

        if is_target {
            Ui::toggle("confirm recursive deletion", &mut self.confirm_delete).interact();
        } else {
            Ui::vspace(line * 1.34);
            self.confirm_delete = false;
        }

        Ui::pop_tint();
        Ui::next_line();
    }

    /// Notifies the caller that the user wants to delete the entry whose name is in `file_selected_name`: a
    /// [`FILE_BROWSER_B_DELETE`] event carrying the full path of the entry. Like the other modes, the browser never
    /// touches the filesystem (except to create a directoru) — it only sends the event, and the caller is free to
    /// delete the entry (`fs::remove_file` / `fs::remove_dir_all`) — or not — when it receives it.
    fn send_delete_event(&mut self) {
        let path = self.dir.join(&self.file_selected_name);
        Log::diag(format!("FileBrowserB requesting deletion of {path:?}"));
        SkInfo::send_event(
            &self.sk_info,
            StepperAction::event(self.caller.as_str(), FILE_BROWSER_B_DELETE, path.to_str().unwrap_or("path_error")),
        );
        self.file_selected_name.clear();
        self.confirm_delete = false;
        self.close_me();
    }

    /// The status line at the bottom of the file list, showing the number of files and folders,
    fn draw_status_line(&mut self) {
        let n_files = self.filtered_indices.iter().filter(|i| !self.entries[**i].is_dir).count();
        let n_dirs = self.filtered_indices.len() - n_files;
        let total_size: u64 = self.filtered_indices.iter().map(|i| self.entries[*i].size).sum();
        self.status = format!(
            "{} folder(s), {} file(s){}  —  {:?}",
            n_dirs,
            n_files,
            if total_size > 0 { format!(" ({})", format_size(total_size)) } else { String::new() },
            self.sort_by,
        );
        // The status line closes the window with the smallest text of the browser.
        Ui::push_text_style(self.appearence.small_style);
        Ui::label(&self.status).size(Vec2::new(Ui::get_layout_remaining().x, 0.0)).draw();
        Ui::pop_text_style();
    }

    // ----------------------------------------------------------------------- helpers

    /// Number of rows that fit in `available_h`, clamped to [`FileBrowserB::max_visible_rows`]
    /// when it is non-zero.
    ///
    /// `row_h` is the effective height of one row, computed by the caller so it matches what the
    /// buttons actually reserve: `line * 2.0` for the explicit grid cells, or the current text
    /// style's line height for the auto-height list buttons (see `draw_list`).
    fn visible_rows_count(&self, available_h: f32, row_h: f32) -> usize {
        let gutter = self.appearence.ui_settings_scaled().gutter;
        let mut rows =
            if row_h <= 0.0 { 1 } else { ((available_h + gutter) / (row_h + gutter)).floor().max(1.0) as usize };
        if self.max_visible_rows > 0 {
            rows = rows.min(self.max_visible_rows as usize);
        }
        rows.max(1)
    }

    /// Whether the current mode offers the "New folder" toolbar button (and its input row):
    /// creating a destination folder makes sense when saving a file, and when selecting a
    /// directory to write into.
    fn new_folder_allowed(&self) -> bool {
        matches!(self.picker_mode, PickerMode::Save | PickerMode::SelectDirectory)
    }

    fn can_go_up(&self) -> bool {
        match self.dir.parent() {
            Some(parent) => {
                if self.root_dir.as_os_str().is_empty() {
                    !parent.as_os_str().is_empty()
                } else {
                    self.dir != self.root_dir && !parent.as_os_str().is_empty()
                }
            }
            None => false,
        }
    }

    fn go_up(&mut self) {
        if self.can_go_up()
            && let Some(parent) = self.dir.parent()
        {
            let parent = parent.to_path_buf();
            // Clamp to root if a root is set.
            let new_dir = if !self.root_dir.as_os_str().is_empty() && !parent.starts_with(&self.root_dir) {
                self.root_dir.clone()
            } else {
                parent
            };
            if new_dir != self.dir {
                self.history.push(self.dir.clone());
                self.dir = new_dir;
                self.needs_refresh = true;
            }
        }
    }

    fn navigate_to(&mut self, target: PathBuf) {
        // Only navigate within the root if a root is set.
        if !self.root_dir.as_os_str().is_empty() && !target.starts_with(&self.root_dir) {
            Log::diag(format!("FileBrowserB: refusing to navigate outside root to {:?}", target));
            return;
        }
        if target.is_dir() {
            // Only record history if the target is actually different from the current dir.
            if target != self.dir {
                self.history.push(self.dir.clone());
                self.dir = target;
            }
            self.scroll = 0.0;
            self.file_selected_name.clear();
            self.needs_refresh = true;
        } else {
            Log::diag(format!("FileBrowserB: target is not a directory: {:?}", target));
        }
    }

    fn refresh(&mut self) {
        self.entries = read_directory(&self.dir, &self.exts, self.show_hidden);
        self.recompute_filtered();
    }

    fn recompute_filtered(&mut self) {
        // The directory modes (SelectDirectory / DeleteDirectory) only work on directories: hide
        // the files from the list. The extension filter is irrelevant there, which is also why the
        // window title of these modes does not show it.
        let dirs_only = matches!(self.picker_mode, PickerMode::SelectDirectory | PickerMode::DeleteDirectory);
        // Build indices, then apply search + sort.
        let search_lc = self.search.trim().to_lowercase();
        let mut indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                (!dirs_only || e.is_dir) && (search_lc.is_empty() || e.name_str().to_lowercase().contains(&search_lc))
            })
            .map(|(i, _)| i)
            .collect();

        let asc = self.sort_ascending;
        let by = self.sort_by;
        indices.sort_by(|&a, &b| {
            let ea = &self.entries[a];
            let eb = &self.entries[b];
            let ord = match by {
                SortBy::Name => ea.name_str().to_lowercase().cmp(&eb.name_str().to_lowercase()),
                SortBy::Size => ea.size.cmp(&eb.size),
                SortBy::Modified => ea.modified.cmp(&eb.modified),
                SortBy::Type => match (ea.is_dir, eb.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => ea.name_str().to_lowercase().cmp(&eb.name_str().to_lowercase()),
                },
            };
            if asc { ord } else { ord.reverse() }
        });

        self.filtered_indices = indices;
    }
}

// --------------------------------------------------------------------------- free functions

/// Reads a directory into a sorted-ish (unsorted) list of [`FileEntry`], applying extension filter and hidden-file
/// filter. Directories are always included regardless of extension.
pub fn read_directory(dir: &Path, exts: &[String], show_hidden: bool) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    let exts_lc: Vec<String> = exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();

    if !dir.exists() || !dir.is_dir() {
        return entries;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            Log::diag(format!("FileBrowserB: cannot read {dir:?}: {e}"));
            return entries;
        }
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        // Always skip "." and ".." — they refer to the directory itself and its parent,
        // not to actual contents. These may be returned by read_dir on some platforms.
        if name_str == "." || name_str == ".." {
            continue;
        }

        if !show_hidden && name_str.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let is_file = path.is_file();
        // `fs::metadata` follows symlinks; it returns an error for broken links or other entries
        // whose metadata can't be read. Such entries are marked `is_broken` rather than skipped.
        let is_broken = std::fs::metadata(&path).is_err();

        if !is_dir && !is_file && !is_broken {
            // Skip regular special files/symlinks unless they are broken (broken ones are kept
            // so they can be surfaced and marked in the UI).
            continue;
        }

        if is_file && !exts_lc.is_empty() {
            let ok = path.extension().map(|e| exts_lc.contains(&e.to_string_lossy().to_lowercase())).unwrap_or(false);
            if !ok {
                continue;
            }
        }

        let (size, modified, symlink_name) = entry
            .metadata()
            .map(|m| {
                let size = if is_dir { 0 } else { m.len() };
                let modified = m.modified().ok();
                let symlink_name = if m.is_symlink() {
                    std::fs::read_link(&path).ok().map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };
                (size, modified, symlink_name)
            })
            .unwrap_or((0, None, None));

        // Number of entries inside a subdirectory, computed once at scan time so the draw loop
        // doesn't have to re-read every visible directory each frame.
        let num_entries = if is_dir { std::fs::read_dir(&path).map(|rd| rd.count()).unwrap_or(0) } else { 0 };

        entries.push(FileEntry { name: file_name, is_dir, is_broken, symlink_name, size, modified, num_entries });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name_str().to_lowercase().cmp(&b.name_str().to_lowercase()),
    });

    entries
}

/// Human-readable byte size, e.g. `1.2 KB`, `3.4 MB`.
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}

/// Human-readable modification date, e.g. `2025-12-31 14:30`.
/// Returns `"-"` if the modification time is unavailable.
pub fn format_date(modified: Option<SystemTime>) -> String {
    match modified {
        Some(time) => {
            // Simple duration-based formatting without external chrono dependency.
            let now = SystemTime::now();
            match now.duration_since(time) {
                Ok(elapsed) => {
                    let secs = elapsed.as_secs();
                    if secs < 60 {
                        "just now".to_string()
                    } else if secs < 3600 {
                        format!("{}m ago", secs / 60)
                    } else if secs < 86400 {
                        format!("{}h ago", secs / 3600)
                    } else if secs < 86400 * 30 {
                        format!("{}d ago", secs / 86400)
                    } else if secs < 86400 * 365 {
                        format!("{}mo ago", secs / (86400 * 30))
                    } else {
                        format!("{}y ago", secs / (86400 * 365))
                    }
                }
                Err(_) => "future".to_string(),
            }
        }
        None => "-".to_string(),
    }
}

/// The previewer of [`FileBrowserB`]: the implementor set in [`FileBrowserB::preview`] is called every frame while the
/// button of a directory or a file of the list is focused (an interactor is in or near the button,
/// see [`Ui::get_last_element_focused`]).
pub trait Previewer: Send {
    /// Draws the preview of the focused entry.
    ///
    /// - `file_path`: the full path of the focused entry,
    /// - `window_pose`: the current pose of the browser window,
    /// - `bouton_pose`: the world-space pose of the focused entry's button, so a preview (e.g. a 3D model, a
    ///   thumbnail...) can be drawn right next to it.
    fn preview(&mut self, file_path: PathBuf, window_pose: Pose, bouton_pose: Pose);

    /// Propagates the browser `ui_scale` to this previewer, so a child window follows the scale grabbed on the browser
    /// scale handle (see [`Appearence::scale_handle`], whose result [`FileBrowserB`] forwards here in its `draw` every
    /// frame the handle is dragged).
    fn set_ui_scale(&mut self, _ui_scale: f32);
}

/// A ready-to-use basic preview for [`FileBrowserB::preview`]: draws a small info panel in world space beside the
/// focused entry's button. Every piece of information is displayed directly as its own [`Ui::label`], on its own line
/// of the panel:
/// - the entry name, as a title above a separator,
/// - its kind (file + extension / directory + item count / symlink -> target / unreadable entry),
/// - its size for files ([`format_size`]),
/// - its last modification ([`format_date`]) and its read-only status,
/// - its full path, wrapped on ~30 chars (same separator-aware wrapping as the list annotations).
///
/// A small panel at the right of the text also shows a thumbnail: for image files (extensions in
/// [`Assets::TEXTURE_FORMATS`], like the asset1 demo) it is the [`Tex`] loaded from the file, for any other entry an
/// identicon-like pattern derived from its bytes. The file probing / pattern generation runs in a worker thread, and
/// every asset access happens on the main thread.
///
/// The panel is placed relative to `bouton_pose`.
///
/// Assign it to [`FileBrowserB::preview`] boxed as a [`Previewer`] trait object:
/// ```ignore
/// file_browser.preview = Some(Box::new(BasicPreviewer::default()));
/// ```
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec2}, sk::SkInfo, ui::Ui,
///                      tools::file_browser_b::{BasicPreviewer, FileBrowserB, FILE_BROWSER_B_OPEN}};
///
/// let id = "main_b_preview".to_string();
/// const BROWSER_SUFFIX: &str = "_file_browser_b_preview";
/// let mut file_browser = FileBrowserB::default();
/// let sk_info = Some(sk.get_sk_info_clone());
///
/// file_browser.dir = std::path::PathBuf::from("/");
/// file_browser.caller = id.clone();
/// file_browser.window_pose = Ui::popup_pose([-0.02, 0.25, 1.25]);
/// file_browser.appearence.window_size = Vec2{x: 0.5, y: 0.5};
/// file_browser.preview = Some(Box::new(BasicPreviewer::default()));
/// SkInfo::send_event(&sk_info, StepperAction::add(id.clone() + BROWSER_SUFFIX, file_browser));
/// test_steps!( // !!!! Get a proper main loop !!!!
///
///     for event in token.get_event_report() {
///         if let StepperAction::Event(stepper_id, key, value) = event {
///             if stepper_id == &id && key.eq(FILE_BROWSER_B_OPEN) {
///                 println!("Selected file: {}", value);
///             }
///         }
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
pub struct BasicPreviewer {
    /// The look of the preview panel. Only its text styles are used: the panel is a pop-up of the browser window,
    /// drawn inside its layout with the browser's already-scaled [`crate::ui::UiSettings`], so it has no scale handle
    /// of its own (see [`Appearence::scale_handle`]) and its `window_size` / `ui_settings` / tints are unused.
    ///
    /// The style mapping is: the panel title uses [`Appearence::title_style`], the information lines
    /// [`Appearence::label_style`] (defaulted to a dimmer `LIGHT_GRAY`), and the full path [`Appearence::small_style`].
    pub appearence: Appearence,
    /// Thumbnail sprite currently displayed in the panel at the right of the text.
    sprite: Sprite,
    /// Keeps the transparent tex when no preview is available
    black_image_tex: Tex,
    /// Path of the entry whose thumbnail is being produced: a worker thread is spawned only when the focused entry
    /// changes, not at every frame.
    thumb_path: PathBuf,
}

unsafe impl Send for BasicPreviewer {}

const DIFFUSE_SIZE: usize = 128;

/// Reads a raw RGBA bitmap file (see <https://github.com/bzotto/rgba_bitmap>): four bytes of `"RGBA"` magic, then the
/// width and the height as big-endian `u32`s, then the RGBA8888 pixel data. Returns the size and the pixels as
/// [`Color32`]s, ready for `Tex::set_colors32`.
pub fn read_rgba_bitmap(path: &Path) -> Result<(usize, usize, Vec<Color32>), std::io::Error> {
    use std::io::Read;

    let mut header = [0u8; 12];
    let mut file = std::fs::File::open(path)?;

    file.read_exact(&mut header)?;
    if &header[0..4] != b"RGBA" {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid magic"));
    }
    let width = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let height = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
    if width == 0 || height == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid dimensions"));
    }

    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    if data.len() != width * height * 4 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Pixel data size mismatch"));
    }
    // The length check above guarantees `data.len()` is a multiple of 4, so `as_chunks` leaves no remainder.
    let pixels = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| Color32 { r: px[0], g: px[1], b: px[2], a: px[3] })
        .collect();
    Ok((width, height, pixels))
}

impl Default for BasicPreviewer {
    fn default() -> Self {
        let font = Font::default();
        let black_image_tex = Tex::gen_color(
            Color128::BLACK_TRANSPARENT,
            DIFFUSE_SIZE as i32,
            DIFFUSE_SIZE as i32,
            TexType::ImageNomips,
            TexFormat::Rgba32Srgb,
        );
        let sprite = Sprite::from_tex(&black_image_tex, None, None).unwrap_or_default();
        // Same text style hierarchy as the browser: bigger and brighter for the title
        // (`title_style`), UI default size slightly dimmed for the information lines
        // (`label_style`), smaller and dimmer for the full path (`small_style`).
        let mut appearence = Appearence::default();
        appearence.label_style = Text::make_style(&font, 0.009, named_colors::LIGHT_GRAY);
        // Capture the base heights of the (possibly tweaked) styles above, so `set_ui_scale`
        // scalings never compound over the frames.
        appearence.start();
        Self { appearence, black_image_tex, sprite, thumb_path: PathBuf::new() }
    }
}

impl Previewer for BasicPreviewer {
    /// Scales the panel text styles with the browser scale: the base heights captured at `default` are re-multiplied
    /// by the new `ui_scale`, so repeated scalings never compound.
    fn set_ui_scale(&mut self, ui_scale: f32) {
        self.appearence.set_ui_scale(ui_scale);
    }

    /// Draws the info panel of `file_path` beside `bouton_pose` (see the [`Previewer`] trait).
    fn preview(&mut self, file_path: PathBuf, window_pose: Pose, bouton_pose: Pose) {
        // ------------------------------------------- produce the thumbnail
        if self.thumb_path != file_path {
            // Same extension dispatch as the asset1 demo: image files are handled by
            // StereoKit's own `from_file` (whose decode runs in a SK job thread)
            let ext = file_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
                .unwrap_or_default();
            if Assets::TEXTURE_FORMATS.contains(&ext.as_str()) {
                // Asset1 pattern: load the texture from file; StereoKit decodes it asynchronously
                // in its own job threads, and the sprite displays it as soon as it is ready.
                let image_tex = Tex::from_file(&file_path, true, None).unwrap_or_else(|err| {
                    Log::warn(format!("BasicPreviewer cannot load the texture {file_path:?}: {err}"));
                    Tex::default()
                });
                self.sprite = Sprite::from_tex(&image_tex, None, None).unwrap_or_default();
            } else if ext == ".rgba" || ext == ".raw" {
                // Raw RGBA bitmap (https://github.com/bzotto/rgba_bitmap): "RGBA" magic, then the width and height as
                // big-endian u32s, then the RGBA8888 pixels, uploaded into the display texture with its real size.
                match read_rgba_bitmap(&file_path) {
                    Ok((width, height, pixels)) => {
                        let mut image_tex = Tex::from_color32(&pixels, width, height, true).unwrap_or_default();
                        image_tex.id(file_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".into()));
                        Log::diag(format!(
                            "BasicPreviewer loaded RGBA bitmap {}: {file_path:?} ({width}x{height})",
                            image_tex.get_id()
                        ));
                        self.sprite = Sprite::from_tex(&image_tex, None, None).unwrap_or_default();
                    }
                    Err(error) => {
                        Log::warn(format!("BasicPreviewer cannot load the RGBA bitmap {file_path:?}: {error}"));
                        self.sprite = Sprite::from_tex(&self.black_image_tex, None, None).unwrap_or_default();
                    }
                }
            } else {
                self.sprite = Sprite::from_tex(&self.black_image_tex, None, None).unwrap_or_default();
            }
            self.thumb_path = file_path.clone();
        }

        // ------------------------------------------------------------------ gather entry information
        // Cheap metadata reads, re-checked every frame while the button stays focused, so the preview
        // always reflects the current state of the file on disk.
        let name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.to_string_lossy().to_string());

        // `metadata` follows symlinks: an Err means a broken symlink or an otherwise unreadable entry.
        let metadata = std::fs::metadata(&file_path).ok();
        let is_symlink = std::fs::symlink_metadata(&file_path).map(|m| m.is_symlink()).unwrap_or(false);
        let symlink_target = if is_symlink { std::fs::read_link(&file_path).ok() } else { None };

        // Each piece of information is computed as its own field, then displayed directly as its own
        // [`Ui::label`] below the entry name (the title).
        let kind: String;
        let mut size_line: Option<String> = None;
        match (&metadata, is_symlink) {
            (None, _) => kind = "kind: unreadable entry!".to_string(),
            (Some(m), true) => {
                let target = symlink_target.unwrap_or_default().to_string_lossy().to_string();
                kind = if m.is_dir() {
                    format!("kind: symlink -> {target} (directory)")
                } else {
                    format!("kind: symlink -> {target}")
                };
            }
            (Some(m), false) if m.is_dir() => {
                let items = std::fs::read_dir(&file_path).map(|rd| rd.count()).unwrap_or(0);
                kind = format!("kind: directory ({items} item(s))");
            }
            (Some(m), false) => {
                let ext =
                    file_path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_else(|| "-".into());
                kind = format!("kind: file ({ext})");
                size_line = Some(format!("size: {}", format_size(m.len())));
            }
        }
        let (modified_line, access_line) = match &metadata {
            Some(m) => (
                Some(format!("modified: {}", format_date(m.modified().ok()))),
                Some(format!("access: {}", if m.permissions().readonly() { "read-only" } else { "read/write" })),
            ),
            None => (None, None),
        };
        // Full path, wrapped on PATH_MAX_CHARS columns so it fits the panel width; the continuation
        // lines are indented to align under "path: ".
        const PATH_MAX_CHARS: usize = 30;
        let path_line = format!(
            "path: {}",
            FileBrowserB::wrap_chars_lines(&file_path.to_string_lossy(), PATH_MAX_CHARS).join("\n      ")
        );

        // ------------------------------------------------------------------------ draw the panel
        const PANEL_W: f32 = 0.22;
        const TITLE_MAX_CHARS: usize = 35;

        // Labels squeeze their text instead of wrapping it, so the possibly long fields are pre-wrapped
        // on ~N chars (same separator-aware wrapping as the list annotations).
        let title = FileBrowserB::wrap_chars_lines(&name, TITLE_MAX_CHARS).join("\n");
        let kind = FileBrowserB::wrap_chars_lines(&kind, PATH_MAX_CHARS).join("\n");

        // where to set the preview:
        let origin = [
            window_pose.position.x - bouton_pose.position.x - 0.1,
            bouton_pose.position.y - window_pose.position.y,
            -0.1,
        ];

        // The panel is its own UI surface, so it composes correctly with the browser window even though
        // this preview callback runs inside its layout. Fixed width, auto height (size y = 0): every
        // field below is displayed directly as its own [`Ui::label`], left auto-sized so the multi-line
        // pre-wrapped ones keep their natural height.
        //Ui::push_surface(Pose::new(origin, Some(bouton_pose.orientation)), Vec3::ZERO, Vec2::new(PANEL_W, 0.0));
        Ui::layout_push(origin, [PANEL_W * 2.0 * self.appearence.ui_scale, 0.0], false);
        Ui::panel_begin(Some(UiPad::Inside));

        // Right panel: the thumbnail `Tex` of the entry, whose diffuse is generated in a thread
        // (see above). The `Ui::image` sprite tracks the `Tex` contents, so the thumbnail pops in
        // as soon as the main thread has uploaded the worker-thread result.
        Ui::layout_push_cut(UiCut::Right, PANEL_W * self.appearence.ui_scale, false);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), Some(UiPad::Inside));
        Ui::image(&self.sprite, Vec2::ONE * PANEL_W * self.appearence.ui_scale);
        Ui::layout_pop();

        // Title line: bigger and brighter than the rest.
        Ui::push_text_style(self.appearence.title_style);
        Ui::label(&title).size([PANEL_W * self.appearence.ui_scale - 0.03, 0.0]).use_padding(false).draw();
        Ui::pop_text_style();

        Ui::hseparator();

        // Information lines.
        Ui::push_text_style(self.appearence.label_style);
        Ui::label(&kind).use_padding(false).draw();
        if let Some(text) = &size_line {
            Ui::label(text).use_padding(false).draw();
        }
        if let Some(text) = &modified_line {
            Ui::label(text).use_padding(false).draw();
        }
        if let Some(text) = &access_line {
            Ui::label(text).use_padding(false).draw();
        }
        Ui::pop_text_style();

        // Path line: smaller and dimmer.
        Ui::push_text_style(self.appearence.small_style);
        Ui::label(&path_line).use_padding(false).draw();
        Ui::pop_text_style();

        Ui::panel_end();
        Ui::layout_pop();
        //Ui::pop_surface();
    }
}
