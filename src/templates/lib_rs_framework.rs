pub mod c_stepper;
use std::sync::Mutex;

use c_stepper::CStepper;
use stereokit_rust::{
    framework::{SkClosures, StepperAction},
    maths::{Pose, Quat, Vec2, Vec3, units::*},
    render::Renderer,
    sk::{DisplayBlend, Sk, SkInfo},
    sprite::Sprite,
    system::{Log, LogItem, LogLevel},
    tex::SHCubemap,
    tools::log_window::{LogWindow, SHOW_LOG_WINDOW, basic_log_fmt},
    ui::{Ui, UiBtnLayout},
    util::{
        Color128, Device, Gradient,
        named_colors::{BLUE, LIGHT_BLUE, LIGHT_CYAN, WHITE},
    },
};

/// Somewhere to copy the log
static LOG_LOG: Mutex<Vec<LogItem>> = Mutex::new(vec![]);

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
/// The main function for android app
fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::sk::{DepthMode, OriginMode, SkSettings};
    let mut settings = SkSettings::default();
    settings
        .app_name("Template App")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(2.0)
        .depth_mode(DepthMode::Stencil)
        .log_filter(LogLevel::Diagnostic);

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    APP_ONCE.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });
    let sk = settings.init(app).unwrap();

    _main(sk);
}

pub fn _main(sk: Sk) {
    let is_testing = false;
    Log::diag("Launch my_vr_program");
    launch(sk, is_testing);
    Sk::shutdown();
}

/// The main function for all platforms
pub fn launch(mut sk: Sk, _is_testing: bool) {
    Log::diag(
        "======================================================================================================== !!",
    );
    Renderer::scaling(1.0);
    Renderer::multisample(4);

    // Sending formated log to our mutex for the log window.
    let fn_mut = |level: LogLevel, log_text: &str| {
        let items = LOG_LOG.lock().unwrap();
        basic_log_fmt(level, log_text, 120, items);
    };
    Log::subscribe(fn_mut);
    let mut log_window = LogWindow::new(&LOG_LOG);
    log_window.window_pose = Pose::new(Vec3::new(-0.7, 2.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
    let mut show_log = false;
    log_window.enabled = false;
    sk.send_event(StepperAction::add("LogWindow", log_window));
    // Open or close the log window
    let send_event_show_log = SkInfo::get_message_closure(Some(sk.get_sk_info_clone()), "main", SHOW_LOG_WINDOW);

    // we will have a window to trigger some actions
    let mut window_demo_pose = Pose::new(Vec3::new(-0.7, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
    let demo_win_width = 50.0 * CM;

    // we create a sky dome to be able to switch from the default sky dome
    let mut gradient_sky = Gradient::new(None);
    gradient_sky
        .add(Color128::BLACK, 0.0)
        .add(BLUE, 0.3)
        .add(LIGHT_BLUE, 0.5)
        .add(LIGHT_CYAN, 0.8)
        .add(WHITE, 1.0);
    let cube0 = SHCubemap::gen_cubemap_gradient(gradient_sky, Vec3::Y, 1024);

    // save the default cubemap.
    let cube_default = SHCubemap::get_rendered_sky();
    cube0.render_as_sky();
    let mut sky = 1;

    // launch AStepper a basic stepper
    sk.send_event(StepperAction::add_default::<CStepper>("CStepper"));

    let mut passthrough = true;
    let mut passthough_blend_enabled = false;
    if Device::valid_blend(DisplayBlend::AnyTransparent) {
        passthough_blend_enabled = true;
        if passthrough {
            Device::display_blend(DisplayBlend::AnyTransparent);
            Log::diag("Passthrough Activated at start !!");
        } else {
            Log::diag("Passthrough Deactived at start !!");
        }
    } else {
        Log::diag("No Passthrough !!")
    }
    Log::diag(
        "======================================================================================================== !!",
    );
    dummy_function();
    let radio_on = Sprite::radio_on();
    let radio_off = Sprite::radio_off();
    SkClosures::run_app(
        sk,
        |sk, _token| {
            Ui::window("Template").pose(&mut window_demo_pose).size(Vec2::new(demo_win_width, 0.0)).begin();
            if Ui::radio("Blue light", sky == 1)
                .images(&radio_off, &radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                cube0.render_as_sky();
                sky = 1;
            }
            Ui::same_line();
            if Ui::radio("Default light", sky == 2)
                .images(&radio_off, &radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                cube_default.render_as_sky();
                sky = 2;
            }
            Ui::same_line();
            if passthough_blend_enabled {
                if let Some(new_value) = Ui::toggle("Passthrough MR", &mut passthrough).interact() {
                    if new_value {
                        Log::diag("Activate passthrough");
                        Device::display_blend(DisplayBlend::AnyTransparent);
                    } else {
                        Log::diag("Deactivate passthrough");
                        Device::display_blend(DisplayBlend::Opaque);
                    }
                }
                Ui::same_line();
            }
            Ui::next_line();
            Ui::hspace(0.11);
            if let Some(new_value) = Ui::toggle("Show Log", &mut show_log, None) {
                send_event_show_log(new_value.to_string());
            }
            Ui::next_line();
            Ui::hseparator();
            if Ui::button("Exit").size(Vec2::new(0.10, 0.10)).press() {
                sk.quit(None);
            }
            //Ui::image(&power_button, Vec2::new(0.1, 0.1));

            Ui::window_end();
        },
        |sk| Log::info(format!("QuitReason is {:?}", sk.get_quit_reason())),
    );
}

/// You can add examples to your documentation using `test_init_sk!`, `test_screenshot!` or `test_steps!` macros,
/// the same way they are used in the stereokit-rust documentation. Add the directory screenshots to your project to get
/// the default screenshot.
///  ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Quat}, util::{named_colors,Color32},
///                      mesh::Mesh, material::Material};
///
/// // Create Meshes
/// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
/// let material_cube = Material::pbr().copy();
/// let cube_transform = Matrix::r([40.0, 50.0, 20.0]);
///
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     cube.draw(&material_cube, cube_transform, None, None);
/// );
/// ```
fn dummy_function() {}
