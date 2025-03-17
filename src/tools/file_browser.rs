use crate::{
    maths::{Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    tools::os_api::PathEntry,
    ui::{Ui, UiBtnLayout, UiWin},
};
use std::path::PathBuf;

use super::os_api::get_files;

pub const FILE_BROWSER_OPEN: &str = "File_Browser_open";

/// A basic file browser to open existing file on PC and Android. Must be launched by an other stepper which has to be
/// set in caller.

#[derive(IStepper)]
pub struct FileBrowser {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub dir: PathBuf,
    files_of_dir: Vec<PathEntry>,
    pub exts: Vec<String>,
    pub window_pose: Pose,
    pub window_size: Vec2,
    pub close_on_select: bool,
    pub caller: StepperId,
    file_selected: u32,
    radio_off: Sprite,
    radio_on: Sprite,
    close: Sprite,
}

unsafe impl Send for FileBrowser {}

impl Default for FileBrowser {
    fn default() -> Self {
        Self {
            id: "FileBrowser".to_string(),
            sk_info: None,

            files_of_dir: vec![],
            dir: PathBuf::new(),
            exts: vec![],
            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            window_size: Vec2::new(0.5, 0.0),
            close_on_select: true,
            caller: "".into(),
            file_selected: 0,
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            close: Sprite::close(),
        }
    }
}

impl FileBrowser {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.files_of_dir = get_files(&self.sk_info, self.dir.clone(), &self.exts, true);

        if self.caller.is_empty() {
            Log::err(
                "FileBrowser must be called by an other stepper (FileBrowser::caller) it will notify of the selected file ",
            );
            return false;
        }

        Log::diag(format!("Browsing directory {:?}", self.dir));

        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        let mut dir_selected = None;

        // The window to select existing file
        let window_text = if self.exts.is_empty() {
            format!("{:?}", self.dir)
        } else {
            format!("{:?} with type {:?}", self.dir, self.exts)
        };
        Ui::window_begin(&window_text, &mut self.window_pose, Some(self.window_size), Some(UiWin::Normal), None);
        Ui::same_line();
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
        Ui::next_line();
        if let Some(sub_dir_name) = self.dir.to_str() {
            if !sub_dir_name.is_empty() {
                //---back button
                if Ui::button("..", None) {
                    self.dir.pop();
                    dir_selected = Some(get_files(&self.sk_info, self.dir.clone(), &self.exts, true));
                }
            }
        }
        let cur_dir = self.dir.clone();
        // we add the dir at the end
        let mut sub_dir: String = cur_dir.to_string_lossy().to_string();
        if !sub_dir.is_empty() {
            sub_dir += "/";
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
