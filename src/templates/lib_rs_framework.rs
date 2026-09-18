pub mod c_stepper;

/// Hot-reload plugin entry points for the hot reloading viewer. The exported `sk_run_sk_*` symbols are inert
/// unless the library is dlopen'ed by the host: they don't interfere with the normal (Android) launch.
#[cfg(all(feature = "skc-shared", not(target_os = "android")))]
pub mod plugin_shim;

use c_stepper::CStepper;
use std::{cell::RefCell, rc::Rc, sync::Mutex};
use stereokit_rust::{
    framework::SkClosures,
    maths::{Pose, Quat, Vec2, Vec3, units::*},
    prelude::*,
    render::Renderer,
    sk::{DepthMode, DisplayBlend, MainThreadToken, OriginMode, SkSettings},
    sprite::Sprite,
    system::{BackendOpenXR, BackendVulkan, BackendVulkanRequest, LogItem, LogLevel},
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

#[unsafe(no_mangle)]
#[allow(dead_code)]
#[cfg(target_os = "android")]
/// The main function for android app
fn android_main(app: AndroidApp) {
    use std::sync::OnceLock;
    use stereokit_rust::sk::Sk;

    // All the SkSettings AND the BackendOpenXR / BackendVulkan parameterizations are grouped in sk_settings()
    let mut settings = sk_settings();
    settings.android_app(app);

    static APP_ONCE: OnceLock<()> = OnceLock::new();
    APP_ONCE.get_or_init(|| {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Debug).with_tag("STKit-rs"),
        );
    });

    // main loop
    launch(settings, false);

    Sk::shutdown();
}

/// All the SkSettings of the app, grouped in one single place, at the same level as `launch`.
pub fn sk_settings() -> SkSettings {
    let mut settings = SkSettings::default();
    settings
        .app_name("Template App")
        .origin(OriginMode::Floor)
        .render_multisample(4)
        .render_scaling(2.0)
        .depth_mode(DepthMode::D32)
        .omit_empty_frames(true)
        .log_filter(LogLevel::Diagnostic);

    // The OpenXR extensions and the Vulkan requests must be set up before StereoKit initialization
    BackendOpenXR::request_ext("XR_FB_display_refresh_rate");
    BackendOpenXR::request_ext("XR_FB_render_model");
    BackendOpenXR::request_ext("XR_META_virtual_keyboard");
    BackendOpenXR::request_ext("XR_KHR_composition_layer_cylinder");
    BackendVulkan::request(&BackendVulkanRequest::new(Some("sk_template_request")));

    settings
}

/// The main function for all platforms
pub fn launch(mut settings: SkSettings, _is_testing: bool) {
    // Sending formated log to our mutex for the log window.
    let fn_mut = |level: LogLevel, log_text: &str| {
        let items = LOG_LOG.lock().unwrap();
        basic_log_fmt(level, log_text, items);
    };
    Log::subscribe(fn_mut);

    let mut sk = settings.init().unwrap();
    Log::diag(
        "======================================================================================================== !!",
    );
    Renderer::scaling(1.0);
    Renderer::multisample(4);

    let mut log_window = LogWindow::new(&LOG_LOG);
    log_window.window_pose = Pose::new(Vec3::new(-0.7, 2.0, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))));
    log_window.enabled = false;
    sk.send_event(StepperAction::add("LogWindow", log_window));

    // The "Template" window.
    sk.send_event(StepperAction::add_default::<MainStepper>("main"));

    // launch CStepper, a basic stepper
    sk.send_event(StepperAction::add_default::<CStepper>("CStepper"));

    dummy_function();

    // The app main loop: the UI and the scene are driven by the steppers above.
    SkClosures::run_app(sk, |_sk, _token| {}, |sk| Log::info(format!("QuitReason is {:?}", sk.get_quit_reason())));
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

/// The "Template" window of the app, as an IStepper.
#[derive(IStepper)]
pub struct MainStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,
    pub demo_win_width: f32,

    show_log: bool,
    sky: u8,
    passthrough: bool,
    passthrough_blend_enabled: bool,

    sky_gradient: Option<SHCubemap>,
    sky_default: Option<SHCubemap>,
    radio_on: Option<Sprite>,
    radio_off: Option<Sprite>,
}

unsafe impl Send for MainStepper {}

/// This code may be called in some threads, so no StereoKit code
impl Default for MainStepper {
    fn default() -> Self {
        Self {
            id: "main".to_string(),
            sk_info: None,
            window_pose: Pose::new(Vec3::new(-0.7, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 50.0 * CM,
            show_log: false,
            sky: 1,
            passthrough: true,
            passthrough_blend_enabled: false,
            sky_gradient: None,
            sky_default: None,
            radio_on: None,
            radio_off: None,
        }
    }
}

/// All the code here runs in the main thread
impl MainStepper {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // we create a sky dome to be able to switch from the default sky dome
        let mut gradient_sky = Gradient::new(None);
        gradient_sky
            .add(Color128::BLACK, 0.0)
            .add(BLUE, 0.3)
            .add(LIGHT_BLUE, 0.5)
            .add(LIGHT_CYAN, 0.8)
            .add(WHITE, 1.0);
        let sky_gradient = SHCubemap::gen_cubemap_gradient(gradient_sky, Vec3::Y, 1024);
        // save the default cubemap, then display the gradient one.
        let sky_default = SHCubemap::get_rendered_sky();
        sky_gradient.render_as_sky();
        self.sky_gradient = Some(sky_gradient);
        self.sky_default = Some(sky_default);

        self.radio_on = Some(Sprite::radio_on());
        self.radio_off = Some(Sprite::radio_off());

        if Device::valid_blend(DisplayBlend::AnyTransparent) {
            self.passthrough_blend_enabled = true;
            if self.passthrough {
                Device::display_blend(DisplayBlend::AnyTransparent);
                Log::diag("Passthrough Activated at start !!");
            } else {
                Log::diag("Passthrough Deactived at start !!");
            }
        } else {
            Log::diag("No Passthrough !!")
        }
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step, after check_event here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("Template").pose(&mut self.window_pose).size(Vec2::new(self.demo_win_width, 0.0)).begin();
        if let (Some(radio_on), Some(radio_off), Some(sky_gradient), Some(sky_default)) = (
            self.radio_on.as_ref(),
            self.radio_off.as_ref(),
            self.sky_gradient.as_ref(),
            self.sky_default.as_ref(),
        ) {
            if Ui::radio("Blue light", self.sky == 1)
                .images(radio_off, radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                sky_gradient.render_as_sky();
                self.sky = 1;
            }
            Ui::same_line();
            if Ui::radio("Default light", self.sky == 2)
                .images(radio_off, radio_on)
                .image_layout(UiBtnLayout::Left)
                .press()
            {
                sky_default.render_as_sky();
                self.sky = 2;
            }
        }
        Ui::same_line();
        if self.passthrough_blend_enabled {
            if let Some(new_value) = Ui::toggle("Passthrough MR", &mut self.passthrough).interact() {
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
        if let Some(new_value) = Ui::toggle("Show Log", &mut self.show_log).interact() {
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event("main", SHOW_LOG_WINDOW, if new_value { "true" } else { "false" }),
            );
        }
        Ui::next_line();
        Ui::hseparator();
        if Ui::button("Exit").size(Vec2::new(0.10, 0.10)).press() {
            SkInfo::send_event(&self.sk_info, StepperAction::quit("main", "Exit button pressed"));
        }

        Ui::window_end();
    }
}
