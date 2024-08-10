use std::{cell::RefCell, rc::Rc, sync::Mutex};

use crate::{
    event_loop::{IStepper, StepperAction, StepperId},
    maths::{units::CM, Pose, Quat, Vec2, Vec3},
    sk::{MainThreadToken, SkInfo},
    system::{Assets, Log, Renderer},
    tex::{Tex, TexFormat, TexType},
    ui::Ui,
    util::{Color128, PickerMode, Platform},
};

use crate::sprite::Sprite;

/// Somewhere to store the selected filename
static FILE_NAME: Mutex<String> = Mutex::new(String::new());

pub const SHOW_SCREENSHOT_WINDOW: &str = "ShowScreenshotWindow";

pub struct ScreenshotViewer {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,
    pub pose: Pose,
    tex: Tex,
    screen: Option<Sprite>,
}

unsafe impl Send for ScreenshotViewer {}

impl Default for ScreenshotViewer {
    fn default() -> Self {
        let mut tex = Tex::gen_color(Color128::WHITE, 800, 600, TexType::Image, TexFormat::RGBA32);
        //let mut tex = Tex::render_target(800, 600, None, Some(TexFormat::RGBA32), Some(TexFormat::Depth32)).unwrap();
        tex.id("ScreenshotTex");
        Self {
            id: "ScreenshotStepper".to_string(),
            sk_info: None,
            enabled: false,
            pose: Pose::new(Vec3::new(-0.7, 1.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            tex,
            screen: None,
        }
    }
}

impl IStepper for ScreenshotViewer {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);

        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        for e in token.get_event_report().iter() {
            if let StepperAction::Event(_, key, value) = e {
                if key.eq(SHOW_SCREENSHOT_WINDOW) {
                    self.enabled = value.parse().unwrap_or(false)
                }
            }
        }
        self.draw(token)
    }
}

impl ScreenshotViewer {
    pub fn show(&mut self, value: bool) {
        self.enabled = value;
    }

    fn draw(&mut self, token: &MainThreadToken) {
        if !self.enabled {
            return;
        };

        Ui::window_begin("Screenshot", &mut self.pose, Some(Vec2::new(42.0, 36.5) * CM), None, None);
        if let Some(sprite) = &self.screen {
            Ui::image(sprite, Vec2::new(0.4, 0.3));
        } else {
            Ui::vspace(30.0 * CM);
            let mut name = FILE_NAME.lock().unwrap();
            if !name.is_empty() {
                if let Ok(sprite) = Sprite::from_file(name.to_string(), None, None) {
                    self.screen = Some(sprite);
                } else {
                    Log::err(format!("ScreenshotViewer : file {} is not valid", name))
                }
                name.clear();
            }
        }
        Ui::hseparator();
        if Ui::button("Open", None) {
            self.screen = None;
            if !Platform::get_file_picker_visible() {
                Platform::file_picker_sz(
                    PickerMode::Open,
                    move |ok, file_name| {
                        let mut name = FILE_NAME.lock().unwrap();
                        name.clear();
                        if ok {
                            Log::diag(format!("Open screenshot {}", file_name));
                            name.push_str(file_name);
                        }
                    },
                    &Assets::TEXTURE_FORMATS,
                )
            }
        }
        Ui::same_line();
        if Ui::button("Take Screenshot", None) {
            let mut camera_at = self.pose;
            camera_at.orientation = Quat::look_dir(camera_at.get_forward() * -1.0);

            Renderer::screenshot_capture(
                token,
                |dots, width, height| {
                    Log::info(format!("data lenght {} -> size {}/{}", dots.len(), width, height));
                    let tex = Tex::find("ScreenshotTex").ok();
                    match tex {
                        Some(mut tex) => tex.set_colors32(width, height, dots),
                        None => todo!(),
                    };
                },
                camera_at,
                800,
                600,
                Some(90.0),
                Some(TexFormat::RGBA32),
            );

            self.screen = Sprite::from_tex(&self.tex, None, None).ok();
        }
        Ui::same_line();
        if Ui::button("Save", None) && !Platform::get_file_picker_visible() {
            Platform::file_picker_sz(
                PickerMode::Save,
                move |ok, file_name| {
                    if ok {
                        Log::err(format!("Save screenshot in {} not implemented yet", file_name));
                        let _tex = Tex::find("ScreenshotTex").ok();
                        // to continue ...
                    }
                },
                &Assets::TEXTURE_FORMATS,
            )
        }

        Ui::window_end();
    }
}
