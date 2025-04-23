use crate::{
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    tools::os_api::PathEntry,
    ui::{Ui, UiBtnLayout, UiVisual, UiWin},
    util::{Color128, PickerMode},
};
use std::path::PathBuf;

use super::os_api::get_files;

pub const FILE_BROWSER_OPEN: &str = "File_Browser_open";
pub const FILE_BROWSER_SAVE: &str = "File_Browser_save";

/// A basic file browser to open existing file on PC and Android. Must be launched by an other stepper which has to be
/// set in caller.
/// ### Fields that can be changed before initialization:
/// * `picker_mode` - What the file browser is for. Default is PickerMode::Open.
/// * `caller` - The id of the stepper that launched the file browser and is waiting for a FILE_BROWSER_OPEN message.
/// * `dir` - The directory to show. You can't browse outside of this directory.
/// * `exts` - The file extensions to filter.
/// * `window_pose` - The pose where to show the file browser window.
/// * `window_size` - The size of the file browser window. Default is Vec2{x: 0.5, y: 0.0}.
/// * `close_on_select` - If true, the file browser will close when a file is selected. Default is true.
/// * `file_name_to_save` - The name of the file to save. Default is an empty string.
/// * `dir_tint` - The tint to differenciate directories from files. Default is same as UiVisual::Separator.
/// * `input_tint` - The tint of the input fields. Default is RED.to_gamma().
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec2, Vec3}, sk::SkInfo, ui::Ui, tools::os_api::get_external_path,
///                      tools::file_browser::{FileBrowser, FILE_BROWSER_OPEN}, };
///
/// let id = "main".to_string();
/// const BROWSER_SUFFIX: &str = "_file_browser";
/// let mut file_browser = FileBrowser::default();
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// if cfg!(target_os = "android") {
///     if let Some(img_dir) = get_external_path(&sk_info) {
///         file_browser.dir = img_dir;
///     }
/// }
/// if !file_browser.dir.exists() {
///     file_browser.dir = std::env::current_dir().unwrap_or_default().join("tests");
/// }
/// file_browser.caller = id.clone();
/// file_browser.window_pose = Ui::popup_pose([-0.02, 0.04, 1.40]);
/// file_browser.window_size = Vec2{x: 0.16, y: 0.0};
/// SkInfo::send_event(&sk_info, StepperAction::add(id.clone() + BROWSER_SUFFIX, file_browser));
///
/// filename_scr = "screenshots/file_browser.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     for event in token.get_event_report() {
///         if let StepperAction::Event(stepper_id, key, value) = event{
///             if stepper_id == &id && key.eq(FILE_BROWSER_OPEN) {
///                println!("Selected file: {}", value);
///             }   
///         }
///     }
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/file_browser.jpeg" alt="screenshot" width="200">
///
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec2, Vec3}, sk::SkInfo, ui::Ui, tools::os_api::get_external_path,
///                      tools::file_browser::{FileBrowser, FILE_BROWSER_SAVE}, util::PickerMode, };
///
/// let id = "main".to_string();
/// const BROWSER_SUFFIX: &str = "_file_to_save";
/// let mut file_browser = FileBrowser::default();
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// if cfg!(target_os = "android") {
///     if let Some(img_dir) = get_external_path(&sk_info) {
///         file_browser.dir = img_dir;
///     }
/// }
/// if !file_browser.dir.exists() {
///     file_browser.dir = std::env::current_dir().unwrap_or_default().join("tests");
/// }
/// file_browser.picker_mode = PickerMode::Save;
/// file_browser.caller = id.clone();
/// file_browser.window_pose = Ui::popup_pose([-0.02, 0.09, 1.37]);
/// file_browser.window_size = Vec2{x: 0.25, y: 0.0};
/// file_browser.file_name_to_save = "main.rs".into();
/// SkInfo::send_event(&sk_info, StepperAction::add(id.clone() + BROWSER_SUFFIX, file_browser));
///
/// filename_scr = "screenshots/file_save.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     for event in token.get_event_report() {
///         if let StepperAction::Event(stepper_id, key, value) = event{
///             if stepper_id == &id && key.eq(FILE_BROWSER_SAVE) {
///                println!("Save file: {}", value);
///             }   
///         }
///     }
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/file_save.jpeg" alt="screenshot" width="200">
#[derive(IStepper)]
pub struct FileBrowser {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub picker_mode: PickerMode,
    pub dir: PathBuf,
    files_of_dir: Vec<PathEntry>,
    pub exts: Vec<String>,
    pub window_pose: Pose,
    pub window_size: Vec2,
    pub close_on_select: bool,
    pub caller: StepperId,
    pub dir_buttons_tint: Color128,
    pub input_tint: Color128,
    pub file_name_to_save: String,
    start_dir: PathBuf,
    replace_existing_file: bool,
    file_selected: u32,
    radio_off: Sprite,
    radio_on: Sprite,
    close: Sprite,
}

unsafe impl Send for FileBrowser {}

impl Default for FileBrowser {
    fn default() -> Self {
        let yellow = Color128::new(1.0, 0.0, 0.0, 1.0).to_gamma();
        Self {
            id: "FileBrowser".to_string(),
            sk_info: None,

            picker_mode: PickerMode::Open,
            files_of_dir: vec![],
            dir: PathBuf::new(),
            exts: vec![],
            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            window_size: Vec2::new(0.5, 0.0),
            close_on_select: true,
            caller: "".into(),
            dir_buttons_tint: Ui::get_element_color(UiVisual::Separator, 0.0),
            input_tint: yellow,
            start_dir: PathBuf::new(),
            file_name_to_save: String::with_capacity(255),
            replace_existing_file: false,
            file_selected: 0,
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            close: Sprite::close(),
        }
    }
}

impl FileBrowser {
    /// Called from IStepper::initialize here you can abort the initialization by returning false.
    fn start(&mut self) -> bool {
        self.files_of_dir = get_files(&self.sk_info, self.dir.clone(), &self.exts, true);

        if self.caller.is_empty() {
            Log::err(
                "FileBrowser must be called by an other stepper (FileBrowser::caller) it will notify of the selected file ",
            );
            return false;
        }
        if self.picker_mode == PickerMode::Save && self.close_on_select {
            //Log::warn("FileBrowser::close_on_select true is ignored when saving a file");
            self.close_on_select = false;
        }

        Log::diag(format!("Browsing directory {:?}", self.dir));
        self.start_dir = self.dir.clone();

        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        let mut dir_selected = None;

        // The window to select existing file
        let mut window_text2 = String::with_capacity(2048);
        let window_text = if self.exts.is_empty() {
            format!("{:?}", self.dir)
        } else {
            format!("{:?} with type {:?}", self.dir, self.exts)
        };
        window_text2.push_str(&window_text);

        Ui::window_begin(&window_text, &mut self.window_pose, Some(self.window_size), Some(UiWin::Normal), None);
        if Ui::button_img_at(
            "a",
            &self.close,
            None,
            Vec3::new(self.window_size.x / 2.0 + 0.04, 0.03, 0.01),
            Vec2::new(0.03, 0.03),
            None,
        ) {
            self.close_me();
        }

        if self.picker_mode == PickerMode::Save {
            Ui::push_tint(self.input_tint);
            Ui::label("File name: ", None, false);
            Ui::same_line();
            Ui::input("filename_to_save", &mut self.file_name_to_save, None, None);
            let file = self.dir.join(&self.file_name_to_save);

            let mut ok_to_save = false;
            for ext in &self.exts {
                if self.file_name_to_save.ends_with(ext) {
                    ok_to_save = true;
                    break;
                }
            }

            if file.exists() && !self.file_name_to_save.is_empty() {
                Ui::toggle("Replace existing file", &mut self.replace_existing_file, None);
            } else {
                self.replace_existing_file = false;
            }
            ok_to_save = ok_to_save && (!file.exists() || file.exists() && self.replace_existing_file);
            Ui::push_enabled(ok_to_save, None);
            Ui::same_line();
            if Ui::button("Save", None) {
                // Be sure we can save the file
                SkInfo::send_event(
                    &self.sk_info,
                    StepperAction::event(
                        self.caller.as_str(),
                        FILE_BROWSER_SAVE,
                        file.to_str().unwrap_or("problemo!!"),
                    ),
                );
                self.close_me();
            }
            Ui::pop_enabled();
            Ui::pop_tint();
            Ui::next_line();
        }

        let mut i = 0;
        for file_name in &self.files_of_dir {
            i += 1;

            if let PathEntry::File(name) = file_name {
                let file_name_str = name.to_str().unwrap_or("OsString error!!");
                Ui::same_line();
                if Ui::radio_img(
                    file_name_str,
                    self.file_selected == i,
                    &self.radio_off,
                    &self.radio_on,
                    UiBtnLayout::Left,
                    None,
                ) {
                    self.file_selected = i;

                    if self.picker_mode == PickerMode::Save {
                        self.file_name_to_save = file_name_str.to_string();
                        self.replace_existing_file = false;
                    } else {
                        let file = self.dir.join(file_name_str);
                        SkInfo::send_event(
                            &self.sk_info,
                            StepperAction::event(
                                self.caller.as_str(),
                                FILE_BROWSER_OPEN,
                                file.to_str().unwrap_or("problemo!!"),
                            ),
                        );
                        if self.close_on_select {
                            self.close_me()
                        }
                    }
                }
            }
        }
        Ui::next_line();
        Ui::push_tint(self.dir_buttons_tint);
        if let Some(sub_dir_name) = self.dir.to_str() {
            if !sub_dir_name.is_empty() {
                Ui::push_enabled(self.dir != self.start_dir, None);
                //---back button
                if Ui::button("..", None) {
                    self.dir.pop();
                    dir_selected = Some(get_files(&self.sk_info, self.dir.clone(), &self.exts, true));
                }
                Ui::pop_enabled();
            }
        }
        let cur_dir = self.dir.clone();
        // we add the dir at the end
        let mut sub_dir: String = String::with_capacity(2048);
        sub_dir += cur_dir.to_string_lossy().to_string().as_str();
        if !sub_dir.is_empty() {
            sub_dir.insert(sub_dir.len(), '/');
        }
        for file_name in &self.files_of_dir {
            if let PathEntry::Dir(name) = file_name {
                let dir_name = name.to_str().unwrap_or("OsString error!!");
                Ui::same_line();
                if Ui::button(dir_name, None) {
                    self.dir.push(dir_name);
                    dir_selected = Some(get_files(&self.sk_info, self.dir.clone(), &self.exts, true));
                }
            }
        }
        Ui::pop_tint();

        if let Some(new_value) = dir_selected {
            self.files_of_dir = new_value;
            self.file_selected = 0;
        }
        Ui::window_end();
    }

    fn close_me(&self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
    }
}
