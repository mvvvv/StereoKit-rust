use std::{process, sync::Mutex, thread};

use stereokit_rust::{
    event_loop::{SkClosures, StepperAction, StepperId},
    framework::{HandMenuAction, HandMenuRadial, HandRadial, HandRadialLayer},
    material::{Cull, Material},
    maths::{units::*, Matrix, Pose, Quat, Vec2, Vec3, Vec4},
    mesh::Mesh,
    model::Model,
    shader::Shader,
    sk::{AppFocus, DisplayMode, Sk},
    sprite::Sprite,
    system::{
        Backend, BackendOpenXR, BackendXRType, BtnState, Input, Key, Lines, Log, LogLevel, Projection, Renderer, Text,
    },
    tex::{SHCubemap, Tex, TexSample},
    tools::{
        fly_over::FlyOver,
        log_window::{LogItem, LogWindow},
        notif::HudNotification,
        os_api::{get_display_refresh_rate, set_display_refresh_rate},
        passthrough_fb_ext::{PassthroughFbExt, PASSTHROUGH_FLIP},
        screenshoot::ScreenshotViewer,
        virtual_kbd_meta::VirtualKbdMETA,
    },
    ui::{Ui, UiBtnLayout},
    util::{
        named_colors::{BLACK, BLUE, LIGHT_BLUE, LIGHT_CYAN, RED, WHITE, YELLOW},
        Color128, Gradient, ShLight, SphericalHarmonics, Time,
    },
};
use winit::event_loop::EventLoop;

/// Somewhere to copy the log
static LOG_LOG: Mutex<Vec<LogItem>> = Mutex::new(vec![]);

use super::Test;
pub fn launch(mut sk: Sk, event_loop: EventLoop<StepperAction>, is_testing: bool, start_test: String) {
    Log::diag(
        "======================================================================================================================== !!",
    );

    Renderer::scaling(1.5);
    Renderer::multisample(4);

    let mut window_demo_pose = Pose::new(Vec3::new(-0.7, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
    let window_demo_show = false;

    let demo_win_width = 55.0 * CM;

    let mut last_focus = AppFocus::Hidden;

    let run_seconds = 0.0f32;
    // let mut run_frames = 2;
    // let mut test_index = 0;
    let mut active_scene: Option<StepperId> = None;
    let mut next_scene: Option<&Test> = None;
    let mut scene_frame = 0;
    let mut scene_time = 0.0f32;
    //--------------------------------------------------------------------

    let fn_mut = |level: LogLevel, log_text: &str| {
        let mut items = LOG_LOG.lock().unwrap();

        for line_text in log_text.lines() {
            let subs = line_text.as_bytes().chunks(120);
            for (pos, sub_line) in subs.enumerate() {
                if let Ok(mut sub_string) = String::from_utf8(sub_line.to_vec()) {
                    if pos > 0 {
                        sub_string.insert_str(0, "‣‣‣‣");
                    }
                    if let Some(item) = items.last_mut() {
                        if item.text == sub_string {
                            item.count += 1;
                            continue;
                        }
                    }

                    items.push(LogItem { level, text: sub_string.to_owned(), count: 1 });
                };
            }
        }
    };
    Log::subscribe(fn_mut);
    // need a way to do that properly Log::unsubscribe(fn_mut);

    let mut log_window = LogWindow::new(&LOG_LOG);
    log_window.pose = Pose::new(Vec3::new(-0.7, 2.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));

    log_window.show(true);

    //---- Sky domes and floor
    let mut gradient_sky = Gradient::new(None);
    gradient_sky
        .add(Color128::BLACK, 0.0)
        .add(BLUE, 0.4)
        .add(LIGHT_BLUE, 0.8)
        .add(LIGHT_CYAN, 0.9)
        .add(WHITE, 1.0);
    let cube0 = SHCubemap::gen_cubemap_gradient(gradient_sky, Vec3::Y, 1024);

    let mut gradient = Gradient::new(None);
    gradient
        .add(RED, 0.01)
        .add(YELLOW, 0.1)
        .add(LIGHT_CYAN, 0.3)
        .add(LIGHT_BLUE, 0.4)
        .add(BLUE, 0.5)
        .add(BLACK, 0.7);
    let cube1 = SHCubemap::gen_cubemap_gradient(&gradient, Vec3::NEG_Z, 1);

    let lights: [ShLight; 1] = [ShLight::new(Vec3::ONE, WHITE); 1];
    let sh = SphericalHarmonics::from_lights(&lights);
    let cube2 = SHCubemap::gen_cubemap_sh(sh, 15, 5.0, 0.02);

    let cube3 =
        SHCubemap::from_cubemap_equirectangular("hdri/sky_dawn.hdr", true, 0).unwrap_or(SHCubemap::get_rendered_sky());

    //save the default cubemap.
    let cube_default = SHCubemap::get_rendered_sky();

    let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr())).unwrap();
    let tile = Material::find("mobiles.gltf/mat/Calcaire blanc").unwrap_or_default();
    Log::diag(format!("{:?}", mobile.get_id()));
    for iter in mobile.get_nodes().visuals() {
        Log::diag(format!("{:?}", iter.get_mesh().unwrap().get_id()));
    }

    let mut clean_tile = Material::pbr().copy();
    Log::diag("calcaire_blanc params:");
    for param in tile.get_all_param_info() {
        match param.get_name() {
            "metal" => {
                let metal_tex = param.get_texture().unwrap();
                metal_tex.sample_mode(TexSample::Anisotropic).anisotropy(6);
                clean_tile.metal_tex(metal_tex);
                &mut clean_tile
            }
            "diffuse" => clean_tile.diffuse_tex(param.get_texture().unwrap()),
            "normal" => clean_tile.normal_tex(param.get_texture().unwrap()),
            "occlusion" => clean_tile.occlusion_tex(param.get_texture().unwrap()),
            _ => &mut clean_tile,
        };
        Log::diag(format!(" --- {} :{}", param.get_name(), param.to_string().unwrap_or("no value".to_string())));
    }
    clean_tile
        .id("clean_tile")
        .tex_transform(Vec4::new(0.0, 0.0, 3.0, 3.0))
        .roughness_amount(0.7)
        .color_tint(BLACK)
        .queue_offset(11);

    let floor_model =
        Model::from_mesh(Mesh::generate_plane(Vec2::new(40.0, 40.0), Vec3::UP, Vec3::FORWARD, None, true), clean_tile);
    let floor_tr = Matrix::tr(&Vec3::new(0.0, 0.0, 0.0), &Quat::IDENTITY);

    let tex_particule = Tex::gen_particle(128, 128, 0.9, None);
    let exit_button =
        match Sprite::from_tex(Tex::from_file("textures/exit.jpeg", true, None).unwrap_or_default(), None, None) {
            Ok(sprite) => sprite,
            Err(_) => Sprite::from_tex(&tex_particule, None, None).unwrap(),
        };

    // Open or close the log window
    let event_loop_proxy = sk.get_event_loop_proxy().unwrap();
    let send_event_show_log = move || {
        let _ = &event_loop_proxy.send_event(StepperAction::event("main".to_string(), "ShowLogWindow", "1"));
    };

    let event_loop_proxy = sk.get_event_loop_proxy().clone().unwrap();
    let send_event_show_screenshot = move || {
        let _ = &event_loop_proxy.send_event(StepperAction::event("main".to_string(), "ShowScreenshotWindow", "1"));
    };

    let mut menu_ico = Material::pbr_clip().copy();
    let tex = Tex::from_file("icons/hamburger.png", true, None).unwrap_or_default();
    menu_ico.diffuse_tex(tex).clip_cutoff(0.1);

    let mut screenshot_ico = Material::pbr_clip().copy();
    let tex = Tex::from_file("icons/screenshot.png", true, None).unwrap_or_default();
    screenshot_ico.diffuse_tex(tex).clip_cutoff(0.1);

    let mut log_ico = Material::pbr_clip().copy();
    let tex = Tex::from_file("icons/log_viewer.png", true, None).unwrap_or_default();
    log_ico.diffuse_tex(tex).clip_cutoff(0.1);

    //---Load hand menu
    let hand_menu_stepper = HandMenuRadial::new(HandRadialLayer::new(
        "root",
        None,
        Some(0.0),
        vec![
            HandRadial::layer(
                "\nSkydome",
                Some(menu_ico),
                None,
                vec![
                    HandRadial::item(
                        "Day",
                        None,
                        move || {
                            cube0.render_as_sky();
                        },
                        HandMenuAction::Unchecked(1),
                    ),
                    HandRadial::item(
                        "Sunset",
                        None,
                        move || {
                            cube1.render_as_sky();
                        },
                        HandMenuAction::Unchecked(1),
                    ),
                    HandRadial::item(
                        "Black\nlight",
                        None,
                        move || {
                            cube2.render_as_sky();
                        },
                        HandMenuAction::Unchecked(1),
                    ),
                    HandRadial::item(
                        "HDRI\ndawn",
                        None,
                        move || {
                            cube3.render_as_sky();
                        },
                        HandMenuAction::Unchecked(1),
                    ),
                    HandRadial::item(
                        "Default",
                        None,
                        move || {
                            cube_default.render_as_sky();
                        },
                        HandMenuAction::Checked(1),
                    ),
                    HandRadial::item("Back", None, || {}, HandMenuAction::Back),
                    HandRadial::item("Close", None, || {}, HandMenuAction::Close),
                ],
            ),
            HandRadial::item(
                "Screenshot",
                Some(screenshot_ico),
                move || {
                    send_event_show_screenshot();
                },
                HandMenuAction::Unchecked(2),
            ),
            HandRadial::item(
                "Log",
                Some(log_ico),
                move || {
                    send_event_show_log();
                },
                HandMenuAction::Checked(3),
            ),
            HandRadial::item("Close", None, || {}, HandMenuAction::Close),
        ],
    ));

    let mut notif = HudNotification::default();
    if Backend::xr_type() == BackendXRType::Simulator {
        notif.text = "Press [F1] key to open the hand menu".into();
    } else {
        notif.text = "Press menu button to open the hand menu".into();
    }
    sk.push_action(StepperAction::add("HudNotif1", notif));

    sk.push_action(StepperAction::add("HandMenuStepper", hand_menu_stepper));
    sk.push_action(StepperAction::add("LogWindow", log_window));
    sk.push_action(StepperAction::add_default::<ScreenshotViewer>("Screenshoot"));
    sk.push_action(StepperAction::add_default::<FlyOver>("FlyOver"));
    let mut passthrough = false;
    let passthrough_enabled = BackendOpenXR::ext_enabled("XR_FB_passthrough");
    if passthrough_enabled {
        sk.push_action(StepperAction::add_default::<PassthroughFbExt>("PassthroughFbExt"));
        Log::diag("Passthrough Disabled !!")
    } else {
        Log::diag("No Passthrough !!")
    }
    let virtual_kbd_enabled = BackendOpenXR::ext_enabled("XR_META_virtual_keyboard");
    if virtual_kbd_enabled {
        sk.push_action(StepperAction::add_default::<VirtualKbdMETA>("VirtualKbdMETA"));
        Log::diag("XR_META_virtual_keyboard Ready !!")
    } else {
        Log::diag("No XR_META_virtual_keyboard !!")
    }
    let refresh_rate_editable = BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate");
    let next_refresh_rate_image = Sprite::arrow_right();
    let refresh_rates = [60.0, 72.0, 90.0, 120.0];
    let mut current_refresh_rate = get_display_refresh_rate().unwrap_or(0.0);

    let mut viewport_scaling = Renderer::get_viewport_scaling();
    // let mut multisample = Renderer::get_multisample() as f32;
    let mut fps = 72.0;

    let tests = Test::get_tests();

    if !start_test.is_empty() {
        for test in tests.iter() {
            if test.name.eq(&start_test) {
                Log::info(format!("Starting first scene: {}", &test.name.to_string()));
                next_scene = Some(test);
            }
        }
    }

    let ui_text_style = Ui::get_text_style();
    ui_text_style.get_material().face_cull(Cull::Back);

    Log::diag(
        "===================================================================================================================== !!",
    );
    Log::diag(format!("Thread id : {:?} / {:?} ", thread::current().name(), thread::current().id()));
    Log::diag(format!("Process id : {:?} / {:?} ", thread::current().name(), process::id()));

    SkClosures::run_app(
        sk,
        event_loop,
        |sk, token| {
            if last_focus != sk.get_app_focus() {
                last_focus = sk.get_app_focus();
                Log::info(format!("App focus changed to : {:?}", last_focus));
            }

            if is_testing && run_seconds != 0.0 {
                Time::set_time(Time::get_total() + 1.0 / 90.0, 1.0 / 90.0)
            }

            if let Some(next_s) = &next_scene {
                match &active_scene {
                    Some(active_stepper) => {
                        sk.push_action(StepperAction::remove(active_stepper.clone()));
                        active_scene = None;
                        // As we can relaunch the same IStepper, we have to be sure the previous is closed so we leave
                        // this frame here to execute the StepperAction::remove before launching next IStepper.
                        // So 2 frames without any IStepper.
                        return;
                    }
                    None => {}
                }
                // if is_testing {
                //     Time::set_time(0.0, 0.0);
                //     Input::hand_visible(Handed::Max, false);
                //     Input::hand_clear_override(Handed::Left);
                //     Input::hand_clear_override(Handed::Right);
                //     Assets::block_for_priority(i32::MAX);
                // }
                let next_launcher = (next_s.launcher)(sk);
                active_scene = Some(next_launcher);
                scene_time = Time::get_totalf();
                next_scene = None;
            }
            scene_frame += 1;

            // Playing with projection in simulator mode
            if sk.get_active_display_mode() == DisplayMode::Flatscreen && Input::key(Key::P) == BtnState::JustActive {
                if Renderer::get_projection() == Projection::Perspective {
                    Renderer::projection(Projection::Orthographic);
                } else {
                    Renderer::projection(Projection::Perspective);
                }
            }

            // draw a floor if needed
            //let transform = if World::has_bounds() { World::get_bounds_pose().to_matrix(None) } else { floor_tr };
            if !passthrough {
                floor_model.draw(token, floor_tr, None, None);
            }
            Lines::add_axis(token, Pose::IDENTITY, Some(0.5), None);

            if !window_demo_show {
                Ui::window_begin("Demos", &mut window_demo_pose, Some(Vec2::new(demo_win_width, 0.0)), None, None);
                let mut start = 0usize;
                let mut curr_width_total = 0.0;
                let ui_settings = Ui::get_settings();
                let style = Ui::get_text_style();
                let mut i = 0;
                for test in tests.iter() {
                    i += 1;
                    let width = Text::size(&test.name, Some(style), None).x + ui_settings.padding * 2.0;
                    if curr_width_total + width + ui_settings.gutter > demo_win_width {
                        let inflate =
                            (demo_win_width - (curr_width_total - ui_settings.gutter + 0.0001)) / ((i - start) as f32);
                        for t in start..i {
                            let test_in_line = &tests[t];
                            let curr_width = Text::size(&test_in_line.name, Some(style), None).x
                                + ui_settings.padding * 2.0
                                + inflate;
                            if Ui::button(&test_in_line.name, Some(Vec2::new(curr_width, 0.0))) {
                                Log::info(format!("Starting scene: {}", &test_in_line.name.to_string()));
                                next_scene = Some(test_in_line);
                            }
                            Ui::same_line();
                        }
                        start = i;
                    }
                    if start == i {
                        curr_width_total = ui_settings.margin * 2.0;
                    }
                    curr_width_total += width + ui_settings.gutter;
                }
                for t in start..tests.len() {
                    let test = tests.get(t).unwrap();
                    let curr_width = Text::size(&test.name, Some(style), None).x + ui_settings.padding * 2.0;

                    if Ui::button(&test.name, Some(Vec2::new(curr_width, 0.0))) {
                        next_scene = Some(test);
                    }
                    Ui::same_line();
                }
                Ui::next_line();
                Ui::hseparator();
                if Ui::button_img(
                    "Exit",
                    &exit_button,
                    Some(UiBtnLayout::CenterNoText),
                    Some(Vec2::new(0.10, 0.10)),
                    None,
                ) {
                    Log::diag(format!(
                        "Closure Thread id : {:?} / {:?} ",
                        thread::current().name(),
                        thread::current().id()
                    ));
                    Log::diag(format!("Closure Process id : {:?} / {:?} ", thread::current().name(), process::id()));
                    sk.quit(None);
                }
                Ui::same_line();
                Ui::panel_begin(None);
                if passthrough_enabled {
                    if let Some(new_value) = Ui::toggle("Passthrough MR", passthrough, None) {
                        passthrough = new_value;
                        let mut string_value = "0";
                        if new_value {
                            Log::diag("Activate passthrough");
                            string_value = "1";
                        } else {
                            Log::diag("Deactivate passthrough");
                        }
                        sk.push_action(StepperAction::event("main".into(), PASSTHROUGH_FLIP, string_value))
                    }
                    Ui::same_line();
                }

                fps = ((1.0 / Time::get_step()) + fps) / 2.0;
                Ui::label(format!("FPS: {:.0}", fps), None, true);
                Ui::same_line();

                if refresh_rate_editable
                    && Ui::button_img(
                        format!("Up to {:?} FPS", current_refresh_rate as u32),
                        &next_refresh_rate_image,
                        None,
                        None,
                        None,
                    )
                {
                    let mut restart = true;
                    for i in refresh_rates {
                        if i > current_refresh_rate {
                            current_refresh_rate = i;
                            restart = false;
                            break;
                        }
                    }
                    if restart {
                        current_refresh_rate = refresh_rates[0]
                    }
                    if !set_display_refresh_rate(current_refresh_rate) {
                        current_refresh_rate = 0.0;
                    }
                }

                Ui::next_line();
                Ui::label("Viewport scaling:", None, true);
                Ui::same_line();
                Ui::label(format!("{:.2}", viewport_scaling), None, true);
                Ui::same_line();
                if let Some(new_value) =
                    Ui::hslider("scaling", &mut viewport_scaling, 0.1, 2.0, Some(0.05), None, None, None)
                {
                    Renderer::viewport_scaling(new_value);
                    viewport_scaling = new_value;
                }

                // Ui::label("MSAA:", None, true);
                // Ui::same_line();
                // Ui::label(format!("{:.0}", multisample), None, true);
                // Ui::same_line();
                // if let Some(new_value) = Ui::hslider("msaa", &mut multisample, 0.1, 8.0, Some(1.0), None, None, None) {
                //     Renderer::multisample(new_value as i32);
                //     multisample = new_value;
                // }

                Ui::panel_end();

                Ui::window_end();
            }
        },
        |sk| Log::info(format!("QuitReason is {:?}", sk.get_quit_reason())),
    );
}
