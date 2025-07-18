use std::{
    env::{current_dir, set_current_dir},
    fs::File,
    io::{Read, Write},
    sync::Mutex,
};

use stereokit_macros::IStepper;

use crate::{
    maths::{Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::Renderer,
    tex::{Tex, TexFormat},
    ui::Ui,
    util::{PickerMode, Platform},
};

use crate::sprite::Sprite;

use super::{
    file_browser::{FILE_BROWSER_OPEN, FILE_BROWSER_SAVE, FileBrowser},
    os_api::get_external_path,
};

/// Somewhere to store the selected filename
static FILE_NAME: Mutex<String> = Mutex::new(String::new());

pub const SHOW_SCREENSHOT_WINDOW: &str = "Tool_ShowScreenshotWindow";
pub const SCREENSHOT_FORMATS: [&str; 2] = [".raw", ".rgba"];
pub const CAPTURE_TEXTURE_ID: &str = "Uniq_ScreenshotTexture";
const BROWSER_SUFFIX: &str = "_file_browser";

/// A simple screenshot viewer to take / save / display screenshots.
/// ### Fields that can be changed before initialization:
/// * `picture_size` - The size of the picture to take. Default is Vec2::new(800.0, 600.0).
/// * `field_of_view` - The field of view of the camera. Default is 90.0.
/// * `windows_pose` - The initial pose of the window.
/// * `window_size` - The size of the window. Default is Vec2::new(42.0, 37.0) * CM.
/// * `enabled` - If the screenshot viewer is enabled at start. Default is `true`
///
/// ### Events this stepper is listening to:
/// * `SHOW_SCREENSHOT_WINDOW` - Event that triggers when the window is visible ("true") or hidden ("false").
/// * `FILE_BROWSER_OPEN` - Event that triggers when a file as been selected with the file browser. You can use this
///   event too if you want to load a screenshot.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec3, sk::SkInfo, ui::Ui,
///                      tools::{file_browser::FILE_BROWSER_OPEN,
///                              screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}};
///
/// let sk_info  = Some(sk.get_sk_info_clone());
///
/// let mut screenshot_viewer = ScreenshotViewer::default();
/// screenshot_viewer.window_pose = Ui::popup_pose([0.0, 0.15, 1.3]);
/// sk.send_event(StepperAction::add("ScrViewer", screenshot_viewer));
///
/// let screenshot_path = std::env::current_dir().unwrap().join("assets/textures/screenshot.raw");
/// assert!(screenshot_path.exists());
/// let scr_file = screenshot_path.to_str().expect("String should be valid");
///
/// number_of_steps = 4;
/// filename_scr = "screenshots/screenshot_viewer.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     if iter == 0 {
///        sk.send_event(StepperAction::event( "main", SHOW_SCREENSHOT_WINDOW,"false",));
///     } else if iter == 1 {
///        sk.send_event(StepperAction::event( "main", SHOW_SCREENSHOT_WINDOW,"true",));
///        // The image is not visible at the next step, but at the step after.
///        sk.send_event(StepperAction::event( "ScrViewer", FILE_BROWSER_OPEN, scr_file));
///     }
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screenshot_viewer.jpeg" alt="screenshot" width="200">
#[derive(IStepper)]
pub struct ScreenshotViewer {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub enabled: bool,
    shutdown_completed: bool,

    pub picture_size: Vec2,
    pub field_of_view: f32,
    pub window_pose: Pose,
    pub window_size: Vec2,
    tex: Tex,
    screen: Option<Sprite>,
}

unsafe impl Send for ScreenshotViewer {}

impl Default for ScreenshotViewer {
    fn default() -> Self {
        let picture_size = Vec2::new(800.0, 600.0);
        let tex = Tex::default();

        Self {
            id: "ScreenshotStepper".to_string(),
            sk_info: None,
            enabled: false,
            shutdown_completed: false,

            picture_size,
            field_of_view: 90.0,
            window_pose: Pose::new(Vec3::new(-0.7, 1.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            window_size: Vec2::new(42.0, 37.0) * CM,
            tex,
            screen: None,
        }
    }
}

impl ScreenshotViewer {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // self.tex = Tex::gen_color(
        //     Color128::WHITE,
        //     self.picture_size.x as i32,
        //     self.picture_size.y as i32,
        //     TexType::Rendertarget,
        //     TexFormat::RGBA32,
        // );
        self.tex = Tex::render_target(
            self.picture_size.x as usize,
            self.picture_size.y as usize,
            None,
            Some(TexFormat::RGBA32),
            Some(TexFormat::Depth32),
        )
        .unwrap_or_default();
        self.tex.id(CAPTURE_TEXTURE_ID);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        if key.eq(SHOW_SCREENSHOT_WINDOW) {
            self.enabled = value.parse().unwrap_or(false);
            if !self.enabled {
                self.close_file_browser()
            }
        } else if id == &self.id {
            if key.eq(FILE_BROWSER_OPEN) {
                let mut file_name = FILE_NAME.lock().unwrap();
                file_name.clear();
                file_name.push_str(value);
                self.screen = None;
            } else if key.eq(FILE_BROWSER_SAVE) {
                save_screenshot(value);
            }
        }
    }

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        if !self.enabled {
            return;
        };

        Ui::window_begin("Screenshot", &mut self.window_pose, Some(self.window_size), None, None);
        if let Some(sprite) = &self.screen {
            Ui::image(sprite, Vec2::new(0.4, 0.3));
        } else {
            Ui::vspace(30.0 * CM);
            let mut file_name_lock = FILE_NAME.lock().unwrap();
            let file_name = file_name_lock.to_string();
            if !file_name.is_empty() {
                if let Ok(mut file) = File::open(&file_name) {
                    if let Ok(mut tex) = Tex::find(CAPTURE_TEXTURE_ID) {
                        let mut buf = [0u8; 12];
                        if file.read_exact(&mut buf).is_ok() {
                            // Vive le format RGBA !!! https://github.com/bzotto/rgba_bitmap
                            let rgba_tag = format!("{:?}", &buf[0..4]);
                            let mut four_u8 = [0u8; 4];
                            four_u8.copy_from_slice(&buf[4..8]);
                            let width = u32::from_be_bytes(four_u8) as usize;
                            four_u8.copy_from_slice(&buf[8..12]);
                            let height = u32::from_be_bytes(four_u8) as usize;
                            Log::diag(format!("RGBA file {} with size is {}x{}", &file_name, width, height));
                            if rgba_tag != "RGBA" {
                                let mut data = vec![];
                                match file.read_to_end(&mut data) {
                                    Ok(mut _size) => {
                                        let data_slice = data.as_slice();
                                        tex.set_colors_u8(width, height, data_slice, 4);
                                        self.screen = Sprite::from_tex(&self.tex, None, None).ok();
                                    }
                                    Err(err) => {
                                        Log::warn(format!("Screenshoot Error when reading file {file_name} : {err:?}"))
                                    }
                                }
                            } else {
                                Log::warn(format!("File is not an RGBA {file_name}"));
                            }
                        } else {
                            Log::warn(format!("Screenshoot Error unable to read rgba file infos {}", &file_name));
                        }
                    } else {
                        Log::warn(format!("Screenshoot Error unable to get texture ScreenshotTex {}", &file_name));
                    }
                } else {
                    Log::err(format!("ScreenshotViewer : file {} is not valid", &file_name))
                }
                file_name_lock.clear();
            }
        }
        Ui::hseparator();
        if Ui::button("Open", None) {
            if true {
                let mut file_browser = FileBrowser::default();

                if cfg!(target_os = "android") {
                    if let Some(img_dir) = get_external_path(&self.sk_info) {
                        file_browser.dir = img_dir;
                    }
                }
                if !file_browser.dir.exists() {
                    file_browser.dir = current_dir().unwrap_or_default();
                }
                file_browser.caller = self.id.clone();
                file_browser.window_pose = Ui::popup_pose(Vec3::ZERO);
                self.close_file_browser();
                SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone() + BROWSER_SUFFIX, file_browser));
            } else if !Platform::get_file_picker_visible() {
                Platform::file_picker_sz(
                    PickerMode::Open,
                    move |ok, file_name| {
                        let mut name = FILE_NAME.lock().unwrap();
                        name.clear();
                        if ok {
                            Log::diag(format!("Open screenshot {file_name}"));
                            name.push_str(file_name);
                            Platform::file_picker_close();
                        } else {
                            // großen tricherie
                            name.push_str("aaa.raw");
                        }
                    },
                    &SCREENSHOT_FORMATS,
                )
            }
        }
        Ui::same_line();
        if Ui::button("Take Screenshot", None) {
            let mut camera_at = self.window_pose;
            camera_at.orientation = Quat::look_dir(camera_at.get_forward() * -1.0);
            let width_i = self.picture_size.x as i32;
            let height_i = self.picture_size.y as i32;

            Renderer::screenshot_capture(
                token,
                move |dots, width, height| {
                    Log::info(format!("data length {} -> size {}/{}", dots.len(), width, height));
                    let tex = Tex::find(CAPTURE_TEXTURE_ID).ok();
                    match tex {
                        Some(mut tex) => tex.set_colors32(width, height, dots),
                        None => todo!(),
                    };
                },
                camera_at,
                width_i,
                height_i,
                Some(self.field_of_view),
                Some(TexFormat::RGBA32),
            );

            self.screen = Sprite::from_tex(&self.tex, None, None).ok();
        }
        Ui::same_line();
        Ui::push_enabled(self.screen.is_some(), None);
        if Ui::button("Save", None) && !Platform::get_file_picker_visible() {
            if cfg!(target_os = "android") {
                if let Some(img_dir) = get_external_path(&self.sk_info) {
                    if let Err(err) = set_current_dir(&img_dir) {
                        Log::err(format!("Unable to move current_dir to {img_dir:?} : {err:?}"))
                    }
                }
            }
            if true {
                let mut file_browser = FileBrowser::default();

                if cfg!(target_os = "android") {
                    if let Some(img_dir) = get_external_path(&self.sk_info) {
                        file_browser.dir = img_dir;
                    }
                }
                if !file_browser.dir.exists() {
                    file_browser.dir = current_dir().unwrap_or_default();
                }
                file_browser.picker_mode = PickerMode::Save;
                file_browser.caller = self.id.clone();
                file_browser.window_pose = Ui::popup_pose(Vec3::ZERO);
                file_browser.file_name_to_save = "scr_.rgba".into();
                file_browser.exts = vec![".rgba".into(), ".raw".into()];
                self.close_file_browser();
                SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone() + BROWSER_SUFFIX, file_browser));
            } else {
                Platform::file_picker_sz(
                    PickerMode::Save,
                    move |ok, file_name| {
                        if ok {
                            save_screenshot(file_name);
                        }
                    },
                    &SCREENSHOT_FORMATS,
                )
            }
        }
        Ui::pop_enabled();
        Ui::window_end();
    }

    fn close_file_browser(&mut self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone() + BROWSER_SUFFIX));
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            self.close_file_browser();
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }
}

fn save_screenshot(file_name: &str) {
    let mut name = file_name.to_string();
    if !file_name.ends_with(".rgba") && !file_name.ends_with(".raw") {
        name += ".raw";
    }

    if let Ok(tex) = Tex::find(CAPTURE_TEXTURE_ID) {
        if let Some((width, height, size)) = tex.get_data_infos(0) {
            Log::diag(format!("size is {}", size * 4));
            let data = vec![0u8; size * 4];
            let data_slice = data.as_slice();
            if tex.get_color_data_u8(data_slice, 4, 0) {
                match File::create(&name) {
                    // Vive le format RGBA !!! https://github.com/bzotto/rgba_bitmap
                    Ok(mut file) => {
                        if let Err(err) = file.write_fmt(format_args!("RGBA")) {
                            Log::warn(format!("Screenshoot Error when writing RGBA {} : {:?}", &name, err));
                        }
                        if let Err(err) = file.write(&width.to_be_bytes()[4..]) {
                            Log::warn(format!("Screenshoot Error when writing width {} : {:?}", &name, err));
                        }
                        if let Err(err) = file.write(&height.to_be_bytes()[4..]) {
                            Log::warn(format!("Screenshoot Error when writing height {} : {:?}", &name, err));
                        }
                        if let Err(err) = file.write_all(data_slice) {
                            Log::warn(format!("Screenshoot Error when writing raw image {} : {:?}", &name, err));
                        }
                    }
                    Err(err) => Log::warn(format!("Screenshoot Error when creating file {name} : {err:?}")),
                }
            } else {
                Log::warn(format!("Screenshoot Error when getting texture data {file_name}"));
            }
        } else {
            Log::warn(format!("Screenshoot Error unable to get texture infos {file_name}"));
        }
    } else {
        Log::warn(format!("Screenshoot Error unable to get texture ScreenshotTex {file_name}"));
    }
}
