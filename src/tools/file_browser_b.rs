use crate::{
    font::Font,
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Align, Assets, Hierarchy, Text, TextFit, TextStyle},
    tex::{Tex, TexFormat, TexType},
    tools::os_api::BrowseLocation,
    ui::{Ui, UiBtnLayout, UiCut, UiPad, UiSettings, UiSliderData, UiVisual, UiWin},
    util::{Color128, PickerMode, named_colors},
};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const FILE_BROWSER_B_OPEN: &str = "File_Browser_B_open";
pub const FILE_BROWSER_B_SAVE: &str = "File_Browser_B_save";

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

/// A single entry in a directory, enriched with metadata.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The file or directory name (final path component).
    pub name: std::ffi::OsString,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// The resolved target of a symbolic link, if this entry is a symlink and the target can be
    /// read (`fs::read_link(path).ok()`). `None` for non-symlinks or unreadable links.
    pub symlink_name: Option<String>,
    /// Whether the entry's metadata could not be read, i.e. `fs::metadata(entry.path()).is_err()`.
    /// This is typically true for broken symbolic links or otherwise unreadable entries.
    pub is_broken: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Number of entries inside this directory (0 for files), computed once at scan time so the
    /// draw loop doesn't have to re-read every visible subdirectory each frame.
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

/// A full-featured file browser to open existing files or choose a save path on PC and Android.
/// Should be launched by another stepper set in [`FileBrowserB::caller`].
///
/// Compared to [`crate::tools::file_browser::FileBrowser`], this version adds:
/// - breadcrumbs navigation (click any path component; a too long path may overflow past the
///   window edge, which is fine in VR),
/// - a search/filter text field,
/// - column sorting (name / size / date / type, ascending or descending),
/// - a toggle to show/hide hidden files,
/// - per-entry metadata display (formatted size),
/// - symlink targets and entry errors drawn as a small multi-line text slightly below the entry
///   name (grid view lines break after `/` or `\`, `error_tint` for broken entries),
/// - a scrollable list with a vertical scrollbar whose thumb (`UiVisual::SliderLine`, driven by
///   `Ui::slider_behavior`) has a height proportional to the visible part of the directory,
/// - home / up / refresh toolbar buttons,
/// - directory creation in `Save` mode,
/// - a status line summarizing the current view.
///
///
/// ### Fields that should be changed before initialization:
/// * `picker_mode` - What the file browser is for. Default is [`PickerMode::Open`].
/// * `caller` - The id of the stepper that launched the browser and is waiting for a
///   `FILE_BROWSER_B_OPEN` / `FILE_BROWSER_B_SAVE` message.
/// * `dir` - The directory to show. If left empty, it is resolved at start from `location`.
///   You can browse outside of this directory unless `root_dir` is set, in which case navigation is
///   clamped to it.
/// ### Fields that can be changed before initialization:
/// * `root_dir` - When non-empty, the user cannot navigate above this directory. Defaults to the
///   value of `dir` at `start`.
/// * `location` - The storage location used to resolve `dir` when it is left empty, and the target of
///   the internal/external/documents switcher in the toolbar. On Android: the app `internal_data_path`,
///   the app `external_data_path` or the shared public folders. On PC: the app internal files dir
///   (`~/.local/share/<app>` on Linux, `%APPDATA%\<app>` on Windows), the app settings dir
///   (`~/.config/<app>` on Linux) or the user's public folders. Default is [`BrowseLocation::External`].
/// * `exts` - The file extensions to filter (e.g. `[".png".into(), ".jpg".into()]`).
/// * `window_pose` - The pose where to show the browser window.
/// * `window_size` - The size of the browser window. Default is `Vec2{x: 0.5, y: 0.0}`.
/// * `max_visible_rows` - Maximum number of file rows shown before scrolling kicks in. 0 means auto
///   (computed from the available list height). In grid mode this is a number of grid rows.
/// * `close_on_select` - If true (Open mode only), the browser closes when the user confirms opening
///   a file in the Open panel.
/// * `file_name_to_save` - Pre-filled name in Save mode.
/// * `dir_tint` - Tint used for directory buttons.
/// * `input_tint` - Tint used for the input fields.
/// * `error_tint` - Tint used to signal error entries in the list (dead symlinks, unreadable metadata).
/// * `show_hidden` - Whether hidden files (leading dot) are visible at start.
/// * `grid_view` - Whether to show files in a grid (true) or list (false) at start. Default is false (list).
/// * `preview` - An optional implementor of the [`Previewer`] trait, called every frame while the
///   button of a directory or a file of the list is focused (an interactor is in or near the
///   button, see [`Ui::get_last_element_focused`]) with:
///   - `file_path`: the full path of the focused entry,
///   - `window_pose`: the current pose of the browser window,
///   - `bouton_pose`: the world-space pose of the focused entry's button, so a preview (e.g. a 3D
///     model, a thumbnail...) can be drawn right next to it.
///
///   Unlike a plain closure, an implementor owns its state (`&mut self`), so it can keep caches
///   between frames.
///
///   Default is `None` (no preview). See [`BasicPreviewer`] for a ready-to-use previewer drawing a
///   small info panel (name, kind, size, date, path) beside the focused button.
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
/// file_browser.window_pose = Ui::popup_pose([-0.02, 0.25, 1.25]);
/// file_browser.window_size = Vec2{x: 0.5, y: 0.5};
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
/// file_browser.window_pose = Ui::popup_pose([-0.02, 0.25, 1.25]);
/// file_browser.window_size = Vec2{x: 0.5, y: 0.5};
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
    pub location: BrowseLocation,
    pub exts: Vec<String>,
    pub window_pose: Pose,
    pub window_size: Vec2,
    pub ui_settings: UiSettings,
    pub close_on_select: bool,
    pub caller: StepperId,
    pub dir_tint: Color128,
    pub input_tint: Color128,
    /// Tint used to signal error entries in the list: dead symlinks or entries whose metadata
    /// could not be read.
    pub error_tint: Color128,
    pub file_name_to_save: String,
    pub show_hidden: bool,
    pub grid_view: bool,
    /// Maximum number of visible rows before scrolling. 0 means auto (computed from the list height).
    pub max_visible_rows: u32,
    /// Elapsed time (seconds) between automatic directory refreshes. 0 disables auto-refresh.
    /// Default is 1 seconds.
    pub auto_refresh_interval: f32,
    /// Optional previewer, an implementor of the [`Previewer`] trait, called every frame while the
    /// button of a directory or a file of the list is focused
    pub preview: Option<Box<dyn Previewer>>,

    entries: Vec<FileEntry>,
    filtered_indices: Vec<usize>,
    /// History of visited directories for the "back" button. The current dir is NOT included.
    history: Vec<PathBuf>,
    replace_existing_file: bool,
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
            location: BrowseLocation::External,
            exts: vec![],
            window_pose: Ui::popup_pose([0.15, 0.05, 0.10]),
            window_size: Vec2::new(0.6, 0.8),
            ui_settings: Ui::get_settings(),
            close_on_select: true,
            caller: "".into(),
            dir_tint: named_colors::DARK_SLATE_GRAY.into(),
            input_tint: named_colors::SADDLE_BROWN.into(),
            error_tint: named_colors::RED.into(),
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
        if self.picker_mode == PickerMode::Save && self.close_on_select {
            self.close_on_select = false;
        }

        // If window_size.y is too small (auto), force a minimum height so the layout works.
        if self.window_size.y < 0.5 {
            self.window_size.y = 0.5;
        }

        // If `dir` was left empty, resolve the starting directory from `location`:
        // Android → internal_data_path / external_data_path / shared Documents,
        // PC → juxtaposed current dir / assets dir / user Documents.
        if self.dir.as_os_str().is_empty() {
            self.dir = self.location_root(self.location);
            Log::diag(format!("FileBrowserB starting at location {:?}: {:?}", self.location, self.dir));
        }

        // root_dir stays empty by default: the user can navigate the full filesystem
        // and breadcrumbs show the complete path. Set root_dir explicitly to clamp.
        self.history.clear();
        self.refresh();
        Log::diag(format!("FileBrowserB browsing {:?}", self.dir));
        true
    }

    /// Called from `IStepper::step` to check incoming events.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

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

        Ui::window(&self.id)
            .pose(&mut self.window_pose)
            .size(self.window_size)
            .window_type(UiWin::Body)
            .begin();

        let line = Ui::get_line_height();
        let btn = Vec2::new(line * 1.4, line * 1.4);

        self.draw_toolbar(line, btn);

        self.draw_search_bar();
        self.draw_sort_bar();

        // The list area: cut a Top section for the scrollable file list.
        // Reserve the status line height so it stays at the bottom.
        let status_h = if self.picker_mode == PickerMode::Save {
            line * 5.5 // save name + status line + separator
        } else {
            line * 4.2 // open panel + status line + separator
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

        match self.picker_mode {
            PickerMode::Open => self.draw_open_list(line),
            PickerMode::Save => self.draw_save_list(line),
        }

        Ui::layout_pop();

        Ui::hseparator();

        // The Open/Save confirmation row (file name + Open/Save button) lives in the main window
        // flow, OUTSIDE the list sub-layout, so the vertical slider only spans the file list itself
        // and not the input/button row.
        match self.picker_mode {
            PickerMode::Open => {
                self.draw_open_panel();
                Ui::hseparator();
            }
            PickerMode::Save => {
                self.draw_save_panel(line);
                Ui::hseparator();
            }
        }

        self.draw_status_line();
        Ui::window_end();
    }

    fn close_me(&self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
    }

    // ----------------------------------------------------------------------- UI sections

    fn draw_toolbar(&mut self, line: f32, btn: Vec2) {
        // Close button.
        let size_meter =
            if self.show_new_folder && self.picker_mode == PickerMode::Save { line * 6.0 } else { line * 3.0 };
        Ui::layout_push_cut(UiCut::Top, size_meter, false);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), Some(UiPad::Outside));
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

        // New folder (Save mode only)
        if self.picker_mode == PickerMode::Save {
            Ui::same_line();
            if Ui::button("New folder").size(Vec2::new(line * 4.0, line * 1.4)).press() {
                self.show_new_folder = !self.show_new_folder;
                if self.show_new_folder {
                    self.new_folder_name.clear();
                }
            }
        }

        let header_text = if self.exts.is_empty() {
            "All files".to_string()
        } else {
            format!("Only ({})", self.exts.join(","))
        };

        Ui::same_line();
        Ui::label(header_text).draw();

        Ui::next_line();

        if self.show_new_folder && self.picker_mode == PickerMode::Save {
            self.draw_new_folder_input();
        }

        self.draw_breadcrumbs(line);
        Ui::layout_pop();
    }

    fn draw_breadcrumbs(&mut self, line: f32) {
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
        let gutter = self.ui_settings.gutter;
        // Breadcrumb buttons are smaller than regular buttons.
        let btn_h = line * 1.0;
        let at = Ui::get_layout_at();
        let row_w = Ui::get_layout_remaining().x;
        Ui::layout_reserve(Vec2::new(row_w, btn_h), true, 0.0);

        // Estimated width of a breadcrumb button
        let btn_w = |label: &str| label.chars().count() as f32 * line * 0.15 + btn_h;

        // Deferred navigation target, applied after the borrow of the labels.
        let mut navigate: Option<PathBuf> = None;

        Ui::push_tint(self.dir_tint);
        // The UI layout advances leftward: x starts at the row's top-left corner and decreases,
        // unbounded on the right — the path may overflow the window.
        let mut x = at.x;
        for (label, target) in &labels {
            let w = btn_w(label);
            if Ui::button(label).at([x, at.y, at.z], [w, btn_h]).press() {
                navigate = Some(target.clone());
            }
            x -= w + gutter / 3.0;
        }
        Ui::pop_tint();

        if let Some(target) = navigate {
            self.navigate_to(target);
        }
    }

    fn draw_new_folder_input(&mut self) {
        Ui::push_tint(self.input_tint);
        Ui::label("new folder:").draw();
        Ui::same_line();
        Ui::input("fb_new_folder_name", &mut self.new_folder_name).size(Vec2::new(0.0, 0.0)).edit();
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
        Ui::pop_tint();
        Ui::next_line();
    }

    fn draw_search_bar(&mut self) {
        Ui::push_tint(self.input_tint);
        Ui::label("search:").draw();
        Ui::same_line();
        Ui::input("fb_search", &mut self.search).size(Vec2::new(0.0, 0.0)).edit();
        if !self.search.is_empty() {
            Ui::same_line();
            if Ui::button("clear").press() {
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

    fn draw_open_list(&mut self, line: f32) {
        self.draw_list(line, PickerMode::Open);
    }

    fn draw_save_list(&mut self, line: f32) {
        self.draw_list(line, PickerMode::Save);
    }

    /// Shared rendering of the scrollable file list (vertical slider + grid/list view), used by both
    /// Open and Save modes. Only the slider id, the selection target and the click behaviour differ,
    /// which are all driven by `mode`. Deferred actions are applied here so the draw loop never holds
    /// a borrow of `self`.
    ///
    /// Symlinks show their target path (`-> target`) and entries whose metadata could not be read
    /// are marked with an error, both drawn as a small multi-line [`Ui::text`] (lines joined with
    /// `\n`) slightly below the entry name — in grid view, the lines break after `/` or `\`
    /// separators (too long names are hard-split), so the `TextFit::Exact` adjustment is
    /// proportional (one uniform scale for the whole note) — and tinted with
    /// [`FileBrowserB::error_tint`] for broken entries.
    fn draw_list(&mut self, line: f32, mode: PickerMode) {
        // If the directory to display doesn't exist
        if !self.dir.is_dir() {
            let at = Ui::get_layout_at();
            let size = Ui::get_layout_remaining();
            Ui::push_tint(self.error_tint);
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

        // Compute the scrollable list area dimensions.
        let list_area = Ui::get_layout_remaining();
        let slider_w = line * 0.7;

        // In list mode each row holds 1 entry; in grid mode each row holds `columns` entries.
        let columns = if self.grid_view { 3usize } else { 1usize };
        // Dynamic row count based on the available height and the current view mode.
        let visible_rows = self.visible_rows_count(list_area.y, line);
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
            self.draw_scrollbar(slider_w, list_area.y, self.ui_settings, max_scroll, visible_rows, total_rows);
        }
        Ui::layout_pop();

        // Remaining area for the list content.
        let content_w = Ui::get_layout_remaining().x;

        if self.grid_view {
            // ----- GRID MODE (matrix of `columns` x `visible_rows`) -----
            let gutter = self.ui_settings.gutter;
            let grid_w = ((content_w + gutter) / columns as f32 - gutter).max(0.02);
            let grid_h = line * 2.0;
            let grid_size = Vec2::new(grid_w, grid_h);

            // Scroll is expressed in grid rows: each skipped row == `columns` entries.
            let start_row = self.scroll as usize;
            for row in 0..visible_rows {
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
                    let (dir_tint, error_tint, is_broken) = (self.dir_tint, self.error_tint, entry.is_broken);

                    if entry.is_dir {
                        // Broken directories are tinted with `error_tint` instead of `dir_tint`.
                        Ui::push_tint(self.dir_tint);
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
                        let max_chars = 40;
                        let note = Self::wrap_chars(&note, max_chars);

                        Ui::push_tint(if is_broken { error_tint } else { dir_tint });
                        Ui::text(note)
                            .at([at.x - 0.04, at.y - line * 0.8, at.z - 0.01], [grid_w * 0.7, line * 1.1])
                            .fit(TextFit::Exact)
                            .draw();
                        Ui::pop_tint();
                    }
                }
                Ui::next_line();
            }
        } else {
            // ----- LIST MODE (3 aligned columns: name | size | date) -----
            let gutter = self.ui_settings.gutter;
            let name_w = (content_w * 0.55).max(0.05);
            let size_w = (content_w * 0.20).max(0.03);
            let date_w = (content_w - name_w - size_w - gutter * 2.0).max(0.03);

            let start_row = self.scroll as usize;
            for visible_i in 0..visible_rows {
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
                let (dir_tint, error_tint, is_broken) = (self.dir_tint, self.error_tint, entry.is_broken);

                // Column 1: name (interactive)
                if entry.is_dir {
                    Ui::push_tint(self.dir_tint);
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
                    Ui::text(note)
                        .at([at.x - 0.01, at.y - line * 0.8, at.z - 0.01], [name_w, line * 0.2])
                        .fit(TextFit::Squeeze)
                        .draw();
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
                // Open mode only selects the file; confirmation happens in `draw_open_panel`.
                PickerMode::Open => self.file_selected_name = name,
                PickerMode::Save => {
                    self.file_name_to_save = name;
                    self.replace_existing_file = false;
                }
            }
        }
    }

    /// The vertical scrollbar of the file list. Instead of a plain [`Ui::vslider`],  which allows a custom rendering.
    fn draw_scrollbar(
        &mut self,
        width: f32,
        height: f32,
        ui_settings: UiSettings,
        max_scroll: f32,
        visible_rows: usize,
        total_rows: usize,
    ) {
        let bar_bounds = Ui::layout_reserve(Vec2::new(width, height), false, ui_settings.depth);
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
            thumb_size + Vec2::new(ui_settings.padding, ui_settings.padding) * 2.0,
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
        Ui::draw_element(UiVisual::SliderLine, None, tlb, Vec3::new(width, height, ui_settings.depth * 0.1), focus);
        // Thumb: SliderLine with a height proportional to the visible part of the directory.
        Ui::draw_element(UiVisual::SliderPush, None, thumb_at, Vec3::new(thumb_w, thumb_h, ui_settings.depth), focus);

        // Same sound feedback as StereoKit's sliders: activation on/off, then a tick per row.
        if slider.active_state.is_just_active() {
            Ui::play_sound_on_off(UiVisual::SliderPush, id, thumb_at);
        }
        if slider.active_state.is_active() && prev_row != self.scroll {
            Ui::play_sound_on(UiVisual::SliderPush, thumb_at);
        }
    }

    /// Annotation drawn slightly below the entry name in the list and grid views: symlink entries
    /// show their target path (`-> target`) and entries whose metadata could not be read
    /// (`is_broken`) get an explicit error marker. Returns `None` for plain entries.
    ///
    /// The grid view splits it into several lines of equal length with [`FileBrowserB::wrap_chars`]
    /// so the `TextFit::Exact` scaling stays proportional (same glyph size) on every line.
    fn entry_annotation(entry: &FileEntry) -> Option<String> {
        match (&entry.symlink_name, entry.is_broken) {
            (Some(target), true) => Some(format!("-> {target} (broken link)")),
            (Some(target), false) => Some(format!("-> {target}")),
            (None, true) => Some("[error] unreadable entry".to_string()),
            (None, false) => None,
        }
    }

    /// Wraps `text` into lines of at most `max_chars` columns, preferring to break lines AFTER
    /// `/` or `\` path separators so paths stay readable. Only a segment without any separator
    /// that is longer than `max_chars` (a too long file name) gets hard-split. The lines are
    /// joined with `\n`, so a single [`Ui::text`] call draws the whole note with one uniform
    /// `TextFit::Exact` scale — the adjustment stays proportional.
    ///
    /// The result is also padded with blank lines up to a minimum of 3 lines, so the scale of a
    /// short annotation note matches the one of a longer note. Use [`FileBrowserB::wrap_chars_lines`]
    /// for the raw wrapped lines WITHOUT that padding (e.g. for button texts, where a trailing
    /// blank line would shift the text upward).
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

    /// The core of [`FileBrowserB::wrap_chars`]: wraps `text` into lines of at most `max_chars`
    /// columns, preferring to break lines AFTER `/` or `\` path separators so paths stay readable,
    /// and hard-splitting only a separator-less segment longer than a full line (a too long file
    /// name). Returns the raw lines, WITHOUT the vertical padding `wrap_chars` adds for the
    /// annotation notes.
    fn wrap_chars_lines(text: &str, max_chars: usize) -> Vec<String> {
        // Split into separator-terminated segments, hard-splitting any segment longer than a
        // full line (a too long name with no separator to break on).
        let mut chunks: Vec<String> = Vec::new();
        for seg in text.split_inclusive(|c: char| ['/', '\\'].contains(&c)) {
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

    /// Name used to highlight the currently selected file: the file selected for opening in Open
    /// mode, the pre-filled/typed save name in Save mode.
    fn selected_name(&self, mode: PickerMode) -> &str {
        match mode {
            PickerMode::Open => &self.file_selected_name,
            PickerMode::Save => &self.file_name_to_save,
        }
    }

    /// Runs [`FileBrowserB::preview`] when the directory or file button just drawn is focused.
    ///
    /// Must be called right after the `Ui::button`/`Ui::radio` call of a list entry, while it is
    /// still the "last element": [`Ui::get_last_element_focused`] then gives the focus state of
    /// that very button, and [`Ui::get_layout_last`] its layout bounds. The bounds center
    /// (window-local layout coordinates) is converted into a world-space `bouton_pose` through the
    /// current UI hierarchy, exactly like StereoKit's `ui_popup_pose` does when it attaches a popup
    /// to the focused element.
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
    /// Drawn in the main window flow, below the scrollable file list, so the list's vertical slider
    /// does not span this row. Selecting a file in the list only fills the input here; the user
    /// confirms opening it by pressing "Open".
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

        Ui::push_tint(self.input_tint);
        Ui::push_enabled(ok_to_open, None);
        if Ui::button("Open").press() {
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(self.caller.as_str(), FILE_BROWSER_B_OPEN, file.to_str().unwrap_or("path_error")),
            );
            if self.close_on_select {
                self.close_me();
            }
        }
        Ui::pop_enabled();
        Ui::same_line();
        Ui::label(&self.file_selected_name).draw();

        Ui::pop_tint();
        Ui::next_line();
    }

    /// The Save-mode input row (file name + Save button + "replace existing file" toggle).
    /// Drawn in the main window flow, below the scrollable file list, so the list's vertical slider
    /// does not span this row.
    fn draw_save_panel(&mut self, line: f32) {
        Ui::push_tint(self.input_tint);

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
        Ui::label(&self.status).size(Vec2::new(Ui::get_layout_remaining().x, 0.0)).draw();
    }

    // ----------------------------------------------------------------------- helpers

    /// Resolve the root directory of a [`BrowseLocation`], creating app data directories when they
    /// don't exist yet (they may be missing on Android). Never fails: falls back to the current
    /// dir, then "/".
    fn location_root(&self, location: BrowseLocation) -> PathBuf {
        let root = location
            .get_path(&self.sk_info)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));

        if !root.is_dir() {
            Log::warn(format!("FileBrowserB: {root:?} directory does not exist!"));
        }
        root
    }

    /// Number of rows that fit in `available_h` according to the current view mode (list or grid),
    /// clamped to [`FileBrowserB::max_visible_rows`] when it is non-zero.
    ///
    /// - In list mode each row is one `line` tall.
    /// - In grid mode each row is `line * 2.0` tall.
    fn visible_rows_count(&self, available_h: f32, line: f32) -> usize {
        let row_h = if self.grid_view { line * 2.0 } else { line };
        let gutter = self.ui_settings.gutter;
        let mut rows =
            if row_h <= 0.0 { 1 } else { ((available_h + gutter) / (row_h + gutter)).floor().max(1.0) as usize };
        if self.max_visible_rows > 0 {
            rows = rows.min(self.max_visible_rows as usize);
        }
        rows.max(1)
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
        // Build indices, then apply search + sort.
        let search_lc = self.search.trim().to_lowercase();
        let mut indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| search_lc.is_empty() || e.name_str().to_lowercase().contains(&search_lc))
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

/// Reads a directory into a sorted-ish (unsorted) list of [`FileEntry`], applying extension filter
/// and hidden-file filter. Directories are always included regardless of extension.
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

/// The previewer of [`FileBrowserB`]: the implementor set in [`FileBrowserB::preview`] is called
/// every frame while the button of a directory or a file of the list is focused (an interactor is
/// in or near the button, see [`Ui::get_last_element_focused`]).
pub trait Previewer: Send {
    /// Draws the preview of the focused entry.
    ///
    /// - `file_path`: the full path of the focused entry,
    /// - `window_pose`: the current pose of the browser window,
    /// - `bouton_pose`: the world-space pose of the focused entry's button, so a preview (e.g. a
    ///   3D model, a thumbnail...) can be drawn right next to it.
    fn preview(&mut self, file_path: PathBuf, window_pose: Pose, bouton_pose: Pose);
}

/// A ready-to-use basic preview for [`FileBrowserB::preview`]: draws a small info panel in world
/// space beside the focused entry's button. Every piece of information is displayed directly as
/// its own [`Ui::label`], on its own line of the panel:
/// - the entry name, as a title above a separator,
/// - its kind (file + extension / directory + item count / symlink -> target / unreadable entry),
/// - its size for files ([`format_size`]),
/// - its last modification ([`format_date`]) and its read-only status,
/// - its full path, wrapped on ~30 chars (same separator-aware wrapping as the list annotations).
///
/// A small panel at the right of the text also shows a thumbnail: for image files (extensions in
/// [`Assets::TEXTURE_FORMATS`], like the asset1 demo) it is the [`Tex`] loaded from the file, for
/// any other entry an identicon-like pattern derived from its bytes. The file probing / pattern
/// generation runs in a worker thread, and every asset access happens on the main thread.
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
/// file_browser.window_size = Vec2{x: 0.5, y: 0.5};
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
    /// Style of the title line: the entry name, above the separator.
    pub title_style: TextStyle,
    /// Style of the information lines: kind, size, modification date and access.
    pub field_style: TextStyle,
    /// Style of the last line: the full path of the entry.
    pub path_style: TextStyle,
    /// Thumbnail sprite currently displayed in the panel at the right of the text.
    sprite: Sprite,
    /// Keeps the transparent tex when no preview is available
    black_image_tex: Tex,
    /// Path of the entry whose thumbnail is being produced: a worker thread is spawned only when
    /// the focused entry changes, not at every frame.
    thumb_path: PathBuf,
}

unsafe impl Send for BasicPreviewer {}

const DIFFUSE_SIZE: usize = 128;

impl Default for BasicPreviewer {
    fn default() -> Self {
        let font = Font::default();
        let image_tex = Tex::gen_color(
            Color128::BLACK_TRANSPARENT,
            DIFFUSE_SIZE as i32,
            DIFFUSE_SIZE as i32,
            TexType::ImageNomips,
            TexFormat::Rgba32Srgb,
        );
        let sprite = Sprite::from_tex(&image_tex, None, None).unwrap_or_default();
        Self {
            // Bigger and brighter for the title...
            title_style: Text::make_style(&font, 0.012, named_colors::WHITE),
            // ...the UI default size, slightly dimmed, for the information lines...
            field_style: Text::make_style(&font, 0.009, named_colors::LIGHT_GRAY),
            // ...smaller and dimmer for the full path.
            path_style: Text::make_style(&font, 0.0075, named_colors::GRAY),
            black_image_tex: image_tex,
            sprite,
            thumb_path: PathBuf::new(),
        }
    }
}

impl Previewer for BasicPreviewer {
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
        Ui::layout_push(origin, [PANEL_W * 2.0, 0.0], false);
        Ui::panel_begin(Some(UiPad::Inside));

        // Right panel: the thumbnail `Tex` of the entry, whose diffuse is generated in a thread
        // (see above). The `Ui::image` sprite tracks the `Tex` contents, so the thumbnail pops in
        // as soon as the main thread has uploaded the worker-thread result.
        Ui::layout_push_cut(UiCut::Right, PANEL_W, false);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), Some(UiPad::Inside));
        Ui::image(&self.sprite, Vec2::ONE * PANEL_W);
        Ui::layout_pop();

        // Title line: bigger and brighter than the rest.
        Ui::push_text_style(self.title_style);
        Ui::label(&title).size([PANEL_W - 0.03, 0.0]).use_padding(false).draw();
        Ui::pop_text_style();

        Ui::hseparator();

        // Information lines.
        Ui::push_text_style(self.field_style);
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
        Ui::push_text_style(self.path_style);
        Ui::label(&path_line).use_padding(false).draw();
        Ui::pop_text_style();

        Ui::panel_end();
        Ui::layout_pop();
        //Ui::pop_surface();
    }
}
