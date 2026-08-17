use crate::{
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    ui::{Ui, UiBtnLayout, UiCut, UiVisual, UiWin},
    util::{Color128, PickerMode},
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
    /// File size in bytes (0 for directories).
    pub size: u64,
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
///
/// Compared to [`crate::tools::file_browser::FileBrowser`], this version adds:
/// - breadcrumbs navigation (click any path component),
/// - a search/filter text field,
/// - column sorting (name / size / date / type, ascending or descending),
/// - a toggle to show/hide hidden files,
/// - per-entry metadata display (formatted size),
/// - a scrollable list with vertical slider,
/// - home / up / refresh toolbar buttons,
/// - directory creation in `Save` mode,
/// - a status line summarizing the current view.
///
/// Must be launched by another stepper set in [`FileBrowserB::caller`].
///
/// ### Fields that can be changed before initialization:
/// * `picker_mode` - What the file browser is for. Default is [`PickerMode::Open`].
/// * `caller` - The id of the stepper that launched the browser and is waiting for a
///   `FILE_BROWSER_B_OPEN` / `FILE_BROWSER_B_SAVE` message.
/// * `dir` - The directory to show. You can browse outside of this directory unless `root_dir`
///   is set, in which case navigation is clamped to it.
/// * `root_dir` - When non-empty, the user cannot navigate above this directory. Defaults to the
///   value of `dir` at `start`.
/// * `exts` - The file extensions to filter (e.g. `[".png".into(), ".jpg".into()]`).
/// * `window_pose` - The pose where to show the browser window.
/// * `window_size` - The size of the browser window. Default is `Vec2{x: 0.5, y: 0.0}`.
/// * `max_visible_rows` - Maximum number of file rows shown before scrolling kicks in. 0 means auto
///   (computed from the available list height). In grid mode this is a number of grid rows.
/// * `close_on_select` - If true (Open mode only), the browser closes when a file is selected.
/// * `file_name_to_save` - Pre-filled name in Save mode.
/// * `dir_tint` - Tint used for directory buttons.
/// * `input_tint` - Tint used for the input fields.
/// * `show_hidden` - Whether hidden files (leading dot) are visible at start.
/// * `grid_view` - Whether to show files in a grid (true) or list (false) at start. Default is false (list).
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
    pub exts: Vec<String>,
    pub window_pose: Pose,
    pub window_size: Vec2,
    pub close_on_select: bool,
    pub caller: StepperId,
    pub dir_tint: Color128,
    pub input_tint: Color128,
    pub file_name_to_save: String,
    pub show_hidden: bool,
    pub grid_view: bool,
    /// Maximum number of visible rows before scrolling. 0 means auto (computed from the list height).
    pub max_visible_rows: u32,
    /// Elapsed time (seconds) between automatic directory refreshes. 0 disables auto-refresh.
    /// Default is 1 seconds.
    pub auto_refresh_interval: f32,

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
        let yellow = Color128::new(1.0, 0.0, 0.0, 1.0).to_gamma();
        Self {
            id: "FileBrowserB".to_string(),
            sk_info: None,

            picker_mode: PickerMode::Open,
            dir: PathBuf::new(),
            root_dir: PathBuf::new(),
            exts: vec![],
            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            window_size: Vec2::new(0.6, 0.8),
            close_on_select: true,
            caller: "".into(),
            dir_tint: Ui::get_element_color(UiVisual::Separator, 0.0),
            input_tint: yellow,
            file_name_to_save: String::with_capacity(255),
            show_hidden: false,
            grid_view: false,
            max_visible_rows: 0,

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
            auto_refresh_interval: 1.0,
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

        // In Save mode, the "file name" input row + Save button belong to the main window flow,
        // ABOVE the sort bar and the scrollable list area. This keeps them out of the list sub-layout
        // so the vertical slider only spans the file list itself, not the input/button row.
        if self.picker_mode == PickerMode::Save {
            Ui::hseparator();
            self.draw_save_panel();
            Ui::hseparator();
        }

        self.draw_search_bar();
        self.draw_sort_bar();

        // The list area: cut a Top section for the scrollable file list.
        // Reserve the status line height so it stays at the bottom.
        let status_h = line * 1.5; // status line + separator
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
            PickerMode::Save => self.draw_save_list(),
        }

        Ui::layout_pop();

        self.draw_status_line();
        Ui::window_end();
    }

    fn close_me(&self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
    }

    // ----------------------------------------------------------------------- UI sections

    fn draw_toolbar(&mut self, line: f32, btn: Vec2) {
        // Close button.
        Ui::same_line();
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

        self.draw_breadcrumbs();
    }

    fn draw_breadcrumbs(&mut self) {
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

        Ui::push_tint(self.dir_tint);
        for (i, comp) in components.iter().enumerate() {
            if i < start_idx {
                continue;
            }
            if i > start_idx {
                Ui::same_line();
                // Ui::label("/").use_padding(false).draw();
                // Ui::same_line();
            }
            let label = comp
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| comp.to_string_lossy().to_string());
            if label.is_empty() {
                continue;
            }
            if Ui::button(&label).press() {
                let target = comp.clone();
                self.navigate_to(target);
            }
        }
        Ui::pop_tint();
        Ui::next_line();
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
        Ui::toggle("hidden", &mut self.show_hidden).interact();
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

        // Cut a right portion for the vslider.
        Ui::layout_push_cut(UiCut::Right, slider_w, false);
        let max_scroll = (total_rows as f32 - visible_rows as f32).max(0.0);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        if self.scroll < 0.0 {
            self.scroll = 0.0;
        }
        if total_rows > visible_rows
            && let Some(pos) = Ui::vslider("fb_scroll_open", &mut self.scroll, 0.0, max_scroll).step(1.0).interact()
        {
            self.scroll = pos.clamp(0.0, max_scroll);
        }
        Ui::layout_pop();

        // Remaining area for the list content.
        let content_w = Ui::get_layout_remaining().x;

        if self.grid_view {
            // ----- GRID MODE (matrix of `columns` x `visible_rows`) -----
            let gutter = Ui::get_settings().gutter;
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

                    if entry.is_dir {
                        Ui::push_tint(self.dir_tint);
                        if Ui::button(&name).size(grid_size).press() {
                            dir_clicked = Some(entry_idx);
                        }
                        Ui::pop_tint();
                    } else {
                        let selected = self.file_selected_name == name;
                        if Ui::radio(&name, selected)
                            .images(&self.radio_off, &self.radio_on)
                            .image_layout(UiBtnLayout::Left)
                            .size(grid_size)
                            .press()
                        {
                            row_clicked = Some(entry_idx);
                        }
                    }
                }
                Ui::next_line();
            }
        } else {
            // ----- LIST MODE (3 aligned columns: name | size | date) -----
            let gutter = Ui::get_settings().gutter;
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
                let (size_text, date_text) = if entry.is_dir {
                    let count = std::fs::read_dir(self.dir.join(&entry.name)).map(|rd| rd.count()).unwrap_or(0);
                    (format!("{} item(s)", count), format_date(entry.modified))
                } else {
                    (format_size(entry.size), format_date(entry.modified))
                };

                // Column 1: name (interactive)
                if entry.is_dir {
                    Ui::push_tint(self.dir_tint);
                    if Ui::button(&name).size(Vec2::new(name_w, 0.0)).press() {
                        dir_clicked = Some(entry_idx);
                    }
                    Ui::pop_tint();
                } else {
                    let selected = self.file_selected_name == name;
                    if Ui::radio(&name, selected)
                        .images(&self.radio_off, &self.radio_on)
                        .image_layout(UiBtnLayout::Left)
                        .size(Vec2::new(name_w, 0.0))
                        .press()
                    {
                        row_clicked = Some(entry_idx);
                    }
                }

                // Column 2: size / item count (non-interactive label)
                Ui::same_line();
                Ui::label(size_text).size(Vec2::new(size_w, 0.0)).use_padding(false).draw();

                // Column 3: date (non-interactive label)
                Ui::same_line();
                Ui::label(date_text).size(Vec2::new(date_w, 0.0)).use_padding(false).draw();
            }
        }

        let _ = list_area;

        // Handle deferred actions so we don't borrow self during the draw loop.
        if let Some(i) = dir_clicked {
            let target = self.dir.join(&self.entries[i].name);
            self.navigate_to(target);
        }
        if let Some(i) = row_clicked {
            let entry = &self.entries[i];
            let name = entry.name_str().to_string();
            self.file_selected_name = name.clone();
            let file = self.dir.join(&name);
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(self.caller.as_str(), FILE_BROWSER_B_OPEN, file.to_str().unwrap_or("path_error")),
            );
            if self.close_on_select {
                self.close_me();
            }
        }
    }

    /// The Save-mode input row (file name + Save button + "replace existing file" toggle).
    /// Drawn in the main window flow, ABOVE the scrollable file list, so the list's vertical slider
    /// does not span this row.
    fn draw_save_panel(&mut self) {
        Ui::push_tint(self.input_tint);
        Ui::label("file name:").draw();
        Ui::same_line();
        Ui::input("fb_filename", &mut self.file_name_to_save).size(Vec2::new(0.0, 0.0)).edit();

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

        if file.exists() && name_ok {
            Ui::toggle("replace existing file", &mut self.replace_existing_file).interact();
        } else {
            self.replace_existing_file = false;
        }

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
        Ui::pop_tint();
        Ui::next_line();
    }

    fn draw_save_list(&mut self) {
        let mut clicked: Option<usize> = None;
        let mut dir_clicked: Option<usize> = None;

        let line = Ui::get_line_height();
        let total = self.filtered_indices.len();
        let slider_w = line * 0.7;

        // In list mode each row holds 1 entry; in grid mode each row holds `columns` entries.
        let columns = if self.grid_view { 3usize } else { 1usize };
        let list_area = Ui::get_layout_remaining();
        let visible_rows = self.visible_rows_count(list_area.y, line);
        let total_rows = total.div_ceil(columns);

        // Cut a right portion for the vslider.
        Ui::layout_push_cut(UiCut::Right, slider_w, false);
        let max_scroll = (total_rows as f32 - visible_rows as f32).max(0.0);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
        if self.scroll < 0.0 {
            self.scroll = 0.0;
        }
        if total_rows > visible_rows
            && let Some(pos) = Ui::vslider("fb_scroll_save", &mut self.scroll, 0.0, max_scroll).step(1.0).interact()
        {
            self.scroll = pos.clamp(0.0, max_scroll);
        }
        Ui::layout_pop();

        let content_w = Ui::get_layout_remaining().x;

        if self.grid_view {
            // ----- GRID MODE (matrix of `columns` x `visible_rows`) -----
            let gutter = Ui::get_settings().gutter;
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

                    if entry.is_dir {
                        Ui::push_tint(self.dir_tint);
                        if Ui::button(&name).size(grid_size).press() {
                            dir_clicked = Some(entry_idx);
                        }
                        Ui::pop_tint();
                    } else {
                        let selected = self.file_name_to_save == name;
                        if Ui::radio(&name, selected)
                            .images(&self.radio_off, &self.radio_on)
                            .image_layout(UiBtnLayout::Left)
                            .size(grid_size)
                            .press()
                        {
                            clicked = Some(entry_idx);
                        }
                    }
                }
                Ui::next_line();
            }
        } else {
            // ----- LIST MODE (3 aligned columns: name | size | date) -----
            let gutter = Ui::get_settings().gutter;
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
                let (size_text, date_text) = if entry.is_dir {
                    let count = std::fs::read_dir(self.dir.join(&entry.name)).map(|rd| rd.count()).unwrap_or(0);
                    (format!("{} item(s)", count), format_date(entry.modified))
                } else {
                    (format_size(entry.size), format_date(entry.modified))
                };

                // Column 1: name (interactive)
                if entry.is_dir {
                    Ui::push_tint(self.dir_tint);
                    if Ui::button(&name).size(Vec2::new(name_w, 0.0)).press() {
                        dir_clicked = Some(entry_idx);
                    }
                    Ui::pop_tint();
                } else {
                    let selected = self.file_name_to_save == name;
                    if Ui::radio(&name, selected)
                        .images(&self.radio_off, &self.radio_on)
                        .image_layout(UiBtnLayout::Left)
                        .size(Vec2::new(name_w, 0.0))
                        .press()
                    {
                        clicked = Some(entry_idx);
                    }
                }

                // Column 2: size (non-interactive label)
                Ui::same_line();
                Ui::label(size_text).size(Vec2::new(size_w, 0.0)).use_padding(false).draw();

                // Column 3: date (non-interactive label)
                Ui::same_line();
                Ui::label(date_text).size(Vec2::new(date_w, 0.0)).use_padding(false).draw();
            }
        }

        if let Some(i) = dir_clicked {
            let target = self.dir.join(&self.entries[i].name);
            self.navigate_to(target);
        }
        if let Some(i) = clicked {
            let name = self.entries[i].name_str().to_string();
            self.file_name_to_save = name;
            self.replace_existing_file = false;
        }
    }

    fn draw_status_line(&mut self) {
        let n_files = self.filtered_indices.iter().filter(|i| !self.entries[**i].is_dir).count();
        let n_dirs = self.filtered_indices.len() - n_files;
        let total_size: u64 = self.filtered_indices.iter().map(|i| self.entries[*i].size).sum();
        self.status = format!(
            "{} folder(s), {} file(s){}  —  {:?}",
            n_dirs,
            n_files,
            if total_size > 0 { format!(" (≈ {})", format_size(total_size)) } else { String::new() },
            self.sort_by,
        );
        Ui::label(&self.status).size(Vec2::new(Ui::get_layout_remaining().x, 0.0)).draw();
    }

    // ----------------------------------------------------------------------- helpers

    /// Number of rows that fit in `available_h` according to the current view mode (list or grid),
    /// clamped to [`FileBrowserB::max_visible_rows`] when it is non-zero.
    ///
    /// - In list mode each row is one `line` tall.
    /// - In grid mode each row is `line * 2.0` tall.
    fn visible_rows_count(&self, available_h: f32, line: f32) -> usize {
        let row_h = if self.grid_view { line * 2.0 } else { line };
        let gutter = Ui::get_settings().gutter;
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
        self.scroll = 0.0;
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

        if !is_dir && !is_file {
            // Skip symlinks/special files unless explicitly wanted.
            continue;
        }

        if is_file && !exts_lc.is_empty() {
            let ok = path.extension().map(|e| exts_lc.contains(&e.to_string_lossy().to_lowercase())).unwrap_or(false);
            if !ok {
                continue;
            }
        }

        let (size, modified) = entry
            .metadata()
            .map(|m| {
                let size = if is_dir { 0 } else { m.len() };
                let modified = m.modified().ok();
                (size, modified)
            })
            .unwrap_or((0, None));

        entries.push(FileEntry { name: file_name, is_dir, size, modified });
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
