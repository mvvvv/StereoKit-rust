use std::{process, sync::Mutex, thread};
use stereokit_rust::{
    font::Font,
    framework::{Appearence, ISTEPPER_REMOVED, SkClosures},
    maths::{Pose, Quat, Vec2, Vec3, units::*},
    model::Model,
    prelude::*,
    render::{Projection, Renderer},
    shader::Shader,
    sk::{DepthMode, OriginMode, SkSettings},
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{
        Backend, BackendOpenXR, BackendVulkan, BackendVulkanRequest, BackendXRType, Input, Key, Lines, LogItem,
        LogLevel, Text,
    },
    tex::Tex,
    tools::{
        control_panel::ControlPanel,
        fly_over::{FLY_OVER_ID, FlyOver},
        log_window::{LogWindow, basic_log_fmt},
        notif::HudNotification,
        screenshot::ScreenshotViewer,
        xr_meta_virtual_keyboard::{
            XR_META_VIRTUAL_KEYBOARD_EXTENSION_NAME, XrMetaVirtualKeyboardStepper,
            is_meta_virtual_keyboard_extension_available,
        },
    },
    ui::{Ui, UiBtnLayout},
    util::{Device, Time},
};

/// Somewhere to copy the log
static LOG_LOG: Mutex<Vec<LogItem>> = Mutex::new(vec![]);

use super::{
    Test,
    hand_menu_radial1::{HAND_MENU_RADIAL1_ID, HandMenuRadial1, SHOW_FLOOR},
};
/// The SkSettings of the demos, grouped in one single place, at the same level as `launch`: the settings AND the
/// BackendOpenXR / BackendVulkan parameterizations, which must be done before StereoKit initialization. This is
/// the function the hot-reload viewer reads through the plugin ABI (`sk_run_sk_settings`) to initialize its session
/// exactly like the demos. The launch-dependent settings (`mode`, `fullscreen`, `android_app`...) can still be
/// adjusted by the callers on the returned value, before calling `launch`.
pub fn sk_settings() -> SkSettings {
    let mut settings = SkSettings::default();
    settings
        .app_name("rust Demos")
        .origin(OriginMode::Floor)
        .render_multisample(4) // aka the default aka 0
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    // Platform specific settings
    #[cfg(target_os = "android")]
    settings.render_scaling(1.5);
    #[cfg(not(target_os = "android"))]
    settings.default_font_family("Noto Sans, SimSun");

    // The OpenXR extensions must be requested before SK.Initialize
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");

    #[cfg(target_os = "android")]
    BackendOpenXR::request_ext("XR_META_simultaneous_hands_and_controllers");

    // Required by the Layers1 demo for cylinder composition layers.
    #[cfg(target_os = "android")]
    BackendOpenXR::request_ext("XR_KHR_android_surface_swapchain");

    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");

    // Solving SteamVR linux with Steam link for Quest 2
    #[cfg(not(target_os = "android"))]
    BackendOpenXR::exclude_ext("XR_EXT_hand_tracking");

    // The Vulkan requests must be registered before SK.Initialize too
    BackendVulkan::request(&BackendVulkanRequest::new(Some("sk_test_request")));

    settings
}

pub fn launch(mut settings: SkSettings, is_testing: bool, start_test: String) {
    // Sending formated log to our mutex for the log window.
    let fn_mut = |level: LogLevel, log_text: &str| {
        let items = LOG_LOG.lock().unwrap();
        basic_log_fmt(level, log_text, items);
    };
    Log::subscribe(fn_mut);
    // need a way to do that properly Log::unsubscribe(fn_mut);

    // Initialize StereoKit
    let mut sk = settings.init().unwrap();

    Log::diag("==================================================================================== !!");

    let mut window_demo_pose = Pose::new(Vec3::new(-0.7, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));

    // The Demos window goes through `Appearence`: its width is resizable.
    let text_height = Ui::get_text_style().get_layout_height();
    let font = Font::default();
    let mut appearence_demos = Appearence::new(&font, text_height * 4.0 / 3.0);
    appearence_demos.window_size = Vec2::new(60.0 * CM, 0.0);
    appearence_demos.handle_sprite = Sprite::from_file("icons/SK.png", None, None).ok();
    appearence_demos.start();

    let mut hidden_time = std::time::SystemTime::now();
    let mut now = std::time::SystemTime::now();

    let mut active_scene: Option<StepperId> = None;
    let mut deleting_scene: Option<StepperId> = None;
    let mut next_scene: Option<&Test> = None;
    let mut scene_frame = 0;
    let mut scene_time = 0.0f32;

    // When in testing mode, run for a limited number of steps then screenshot.
    const TEST_NUMBER_OF_STEPS: u32 = 1000;
    let mut test_step = 0u32;

    // The runtime controls (passthrough, fullscreen, mouse, FPS, interactors, refresh rate, viewport scaling).
    let mut control_panel = ControlPanel::new(sk.get_sk_info_clone());
    // Ask the floor of the HandMenuRadial1 demo to follow the passthrough mode.
    control_panel.passthrough_event = Some(("main".into(), SHOW_FLOOR.into()));

    let mut log_window = LogWindow::new(&LOG_LOG);
    log_window.window_pose = Pose::new(Vec3::new(-0.7, 2.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
    log_window.appearence.handle_sprite = Sprite::from_file("icons/log_viewer.png", None, None).ok();

    let tex_particule = Tex::gen_particle(128, 128, 0.9, None);
    let exit_button =
        match Sprite::from_tex(Tex::from_file("textures/exit.jpeg", true, None).unwrap_or_default(), None, None) {
            Ok(sprite) => sprite,
            Err(_) => Sprite::from_tex(&tex_particule, None, None).unwrap(),
        };

    Log::diag(format!(
        "Runtime {} / Device {}",
        Device::get_runtime().unwrap_or("???"),
        Device::get_name().unwrap_or("???")
    ));
    let mut notif = HudNotification::default();
    notif.duration = Some(10.0);
    notif.position = Vec3::new(0.0, 0.0, -0.6);
    if Backend::xr_type() == BackendXRType::Simulator {
        notif.text = "Press [F1] key to open the hand menu".into();
        if control_panel.simulator_fullscreen {
            control_panel.notif_escape()
        }
    } else if cfg!(target_os = "android") || Device::get_runtime().unwrap_or_default().starts_with(" 'v") {
        notif.text = "Press menu button to open the hand menu".into();
    } else {
        notif.text = "Look at your wrist then grip when icons are\n aligned to open the hand menu".into();
    }
    sk.send_event(StepperAction::add("HudNotif1", notif));

    let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr()), None).unwrap_or_default();
    Log::diag(format!("{:?}", mobile.get_id()));
    for iter in mobile.get_nodes().visuals() {
        Log::diag(format!("{:?}", iter.get_mesh().unwrap().get_id()));
    }

    sk.send_event(StepperAction::add_default::<HandMenuRadial1>(HAND_MENU_RADIAL1_ID));
    sk.send_event(StepperAction::add("Tool_LogWindow", log_window));
    sk.send_event(StepperAction::add_default::<ScreenshotViewer>("Tool_Screenshoot"));
    sk.send_event(StepperAction::add_default::<FlyOver>(FLY_OVER_ID));

    let tests = Test::get_tests();

    if !start_test.is_empty() {
        for test in tests.iter() {
            if test.name.eq(&start_test) {
                Log::info(format!("Starting first scene: {}", test.name));
                next_scene = Some(test);
            }
        }
    }

    // Add virtual keyboard stepper if extension is available
    if is_meta_virtual_keyboard_extension_available() {
        Log::info("✅ XR_META_virtual_keyboard extension available");
        let keyboard_stepper = XrMetaVirtualKeyboardStepper::new(false);
        sk.send_event(StepperAction::add(XR_META_VIRTUAL_KEYBOARD_EXTENSION_NAME, keyboard_stepper));
        //sk.send_event(StepperAction::event(XR_META_VIRTUAL_KEYBOARD_EXTENSION_NAME, KEYBOARD_SHOW, "true"));
    } else {
        Log::diag("XR_META_virtual_keyboard extension not available");
    }

    let mut inst_play: Option<SoundInst> = None;

    // Checking BackendVulkan
    assert!(BackendVulkan::request_enabled("sk_test_request"));
    assert!(!BackendVulkan::get_function_ptr("vkCreateBuffer").is_null());
    assert!(BackendVulkan::get_function_ptr("vkNotARealVkFunc").is_null());

    Log::diag("=========================================================== !!");
    Log::diag(format!("Thread id : {:?} / {:?} ", thread::current().name(), thread::current().id()));
    Log::diag(format!("Process id : {:?} / {:?} ", thread::current().name(), process::id()));

    SkClosures::new(sk, |sk, token| {
        // In testing mode, take a screenshot at the last step then quit on the
        // next frame (see the early check at the top of the closure).
        if is_testing {
            test_step += 1;
            if test_step == TEST_NUMBER_OF_STEPS {
                let screenshot_path = format!("screenshots/Demos{start_test}.jpeg");
                Renderer::screenshot(
                    &screenshot_path,
                    90,
                    Pose::look_at(Vec3::new(2.0, 1.5, 1.5), Vec3::new(0.0, 1.0, 0.0)),
                    800,
                    600,
                    Some(80.0),
                );
                Log::info(format!("Test mode: screenshot saved to {screenshot_path}"));
            } else if test_step > TEST_NUMBER_OF_STEPS {
                sk.send_event(StepperAction::Quit("test".into(), "Test mode reached step limit".into()));
            }
        }

        control_panel.update();

        // In case we close the active_scene we have to free the choice to select an other one
        let mut launch_next = false;
        for event in token.get_event_report() {
            if let Some(active_stepper) = &active_scene
                && let StepperAction::Event(stepper_id, key, _value) = event
                && active_stepper == stepper_id
                && key == ISTEPPER_REMOVED
            {
                active_scene = None;
                launch_next = true;
            }
        }

        if let Some(next_s) = &next_scene {
            if deleting_scene.is_none() {
                if let Some(active_stepper) = &active_scene {
                    sk.send_event(StepperAction::remove(active_stepper.clone()));
                    deleting_scene = Some(active_stepper.clone());
                } else {
                    launch_next = true;
                }
            }

            if launch_next {
                deleting_scene = None;
                let next_launcher = (next_s.launcher)(sk);
                active_scene = Some(next_launcher);
                scene_time = Time::get_totalf();
                next_scene = None;
            }
        }
        scene_frame += 1;

        // Playing with projection in simulator mode
        if Backend::xr_type() == BackendXRType::Simulator
            && Input::key(Key::P).is_just_active()
            && !Ui::has_keyboard_focus()
        {
            if Renderer::get_projection() == Projection::Perspective {
                Renderer::projection(Projection::Orthographic);
            } else {
                Renderer::projection(Projection::Perspective);
            }
        }

        Lines::add_axis(Pose::IDENTITY, Some(0.5), None);

        // The Demos window width comes from `Appearence`.
        let demo_win_width = appearence_demos.scaled_window_size().x;
        let prev_settings = Ui::get_settings();
        Ui::settings(appearence_demos.get_ui_settings_scaled());

        Ui::window("Demos").pose(&mut window_demo_pose).size(Vec2::new(demo_win_width, 0.0)).begin();
        // The Appearence text styles are what zoom the window content.
        Ui::push_text_style(appearence_demos.label_style);
        Ui::push_enabled(deleting_scene.is_none(), None);
        let ui_settings = Ui::get_settings();
        let style = Ui::get_text_style();

        // The tests buttons, laid out in a grid.
        let content_width = demo_win_width - ui_settings.margin * 2.0;
        let widths: Vec<f32> = tests
            .iter()
            .map(|test| Text::size_layout(&test.name, Some(style), None).x + ui_settings.padding * 2.0)
            .collect();
        // Column count: the widest one whose cells (each sized by the widest button it holds) still fit the window width.
        let mut cells = vec![widths.iter().copied().fold(0.0, f32::max)];
        for columns in 1..=widths.len() {
            let mut candidate = vec![0.0f32; columns];
            for (index, width) in widths.iter().enumerate() {
                let cell = &mut candidate[index % columns];
                *cell = (*cell).max(*width);
            }
            if candidate.iter().sum::<f32>() + ui_settings.gutter * (columns - 1) as f32 > content_width {
                break;
            }
            cells = candidate;
        }
        // Share the leftover width over the cells, so the grid still spans the window width.
        let columns = cells.len();
        let extra = (content_width - (cells.iter().sum::<f32>() + ui_settings.gutter * (columns - 1) as f32 + 0.0001))
            / columns as f32;
        for cell in &mut cells {
            *cell += extra;
        }

        for (index, test) in tests.iter().enumerate() {
            if Ui::button(&test.name).size(Vec2::new(cells[index % columns], 0.0)).press() {
                Log::info(format!("Starting scene: {}", test.name));
                next_scene = Some(test);
            }
            Ui::same_line();
        }
        Ui::pop_enabled();
        Ui::next_line();
        Ui::hseparator();
        if Ui::button("Exit")
            .image(&exit_button)
            .image_layout(UiBtnLayout::CenterNoText)
            .size(appearence_demos.scale_size(Vec2::new(0.10, 0.10)))
            .press()
        {
            Log::diag(format!("Closure Thread id : {:?} / {:?} ", thread::current().name(), thread::current().id()));
            Log::diag(format!("Closure Process id : {:?} / {:?} ", thread::current().name(), process::id()));
            // sk.quit(None); // is too harsh we want to shutdown our steppers
            sk.send_event(StepperAction::Quit("main".into(), "Main program call quit".into()));
            if cfg!(target_os = "android") {
                let no = Sound::from_file("sounds/no.wav").unwrap();
                inst_play = Some(no.play(Vec3::ONE, None));
            }
        }
        Ui::same_line();
        control_panel.draw();

        Ui::pop_text_style();

        Ui::window_end();

        Ui::settings(prev_settings);

        // Grab-able knob anchored to the window.
        appearence_demos.scale_handle(&window_demo_pose, "demos_scale_handle");
    })
    // .on_window_event(|_sk| {
    //     // we hope to flood the log with external controllers soon ...
    //     Log::diag(format!("{event:?}"));
    // })
    .on_sleeping_step(|_sk, _token| {
        now = std::time::SystemTime::now();
        if let Ok(duration) = now.duration_since(hidden_time)
            && duration.as_secs() > 15
        {
            Log::info("HIDDEN STEP -------> Dreaming ");
            hidden_time = now;
        }
    })
    .shutdown(|sk| Log::info(format!("QuitReason is {:?}", sk.get_quit_reason())))
    .run();
}
