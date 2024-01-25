use std::sync::Mutex;

use crate::{
    maths::{units::CM, Pose, Quat, Vec2, Vec3},
    sk::{IStepper, StepperAction, StepperId},
    system::{Assets, Log, Renderer},
    tex::{Tex, TexFormat, TexType},
    ui::Ui,
    util::{Color128, PickerMode, Platform},
};
use winit::event_loop::EventLoopProxy;

use crate::sprite::Sprite;

/// Somewhere to store the selected filename
static FILE_NAME: Mutex<String> = Mutex::new(String::new());

pub struct ScreenshotViewer {
    id: StepperId,
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    enabled: bool,
    pub pose: Pose,
    tex: Tex,
    screen: Option<Sprite>,
}

impl Default for ScreenshotViewer {
    fn default() -> Self {
        let mut tex = Tex::gen_color(Color128::WHITE, 800, 600, TexType::Image, TexFormat::RGBA32Linear);
        tex.id("ScreenshotTex");
        Self {
            id: "ScreenshotStepper".to_string(),
            event_loop_proxy: None,
            enabled: false,
            pose: Pose::new(Vec3::new(-0.7, 1.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            tex,
            screen: None,
        }
    }
}

impl ScreenshotViewer {
    pub fn show(&mut self, value: bool) {
        self.enabled = value;
    }

    fn draw(&mut self) {
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
                Some(TexFormat::RGBA32Linear),
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

impl IStepper for ScreenshotViewer {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn initialize(&mut self, id: StepperId, event_loop_proxy: EventLoopProxy<StepperAction>) -> bool {
        self.id = id;
        self.event_loop_proxy = Some(event_loop_proxy);

        true
    }

    fn step(&mut self, event_report: &[StepperAction]) {
        for e in event_report.iter() {
            if let StepperAction::Event(_, key, _) = e {
                if key.eq("ShowScreenshotWindow") {
                    self.enabled = !self.enabled
                }
            }
        }
        self.draw()
    }
}
