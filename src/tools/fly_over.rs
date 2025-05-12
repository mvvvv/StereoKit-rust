use crate::{
    maths::{Matrix, Quat, Vec2, Vec3},
    prelude::*,
    sk::{AppMode, OriginMode},
    system::{Handed, Input, Key, Renderer, World},
    util::Time,
};

pub const ENABLE_FLY_OVER: &str = "Tool_EnableFlyOver";

/// FlyOver is a tool that allows you to fly around the scene using the controller sticks.
/// ### Fields that can be changed before initialization:
/// * `move_speed` - The speed at which the camera moves. Default is 2.0.
/// * `rotate_speed` - The speed at which the camera rotates. Default is 90.0°
/// * `enabled` - Whether the tool is enabled or not at start. Default is true.
///
/// ### Events this stepper is listening to:
/// * `ENABLE_FLY_OVER` - Event that triggers when the tool is enabled ("true") or disabled ("false").
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix},
///                      tools::fly_over::{FlyOver, ENABLE_FLY_OVER},
///                      system::Input, system::{Key, Pivot}, sprite::Sprite};
///
/// let sprite = Sprite::from_file("icons/fly_over.png", None, Some("MY_ID"))
///                          .expect("fly_over.png should be able to create sprite");
///
/// let mut fly_over = FlyOver::default();
/// sk.send_event(StepperAction::add_default::<FlyOver>("FlyOver"));
///
/// filename_scr = "screenshots/fly_over.jpeg"; fov_scr = 45.0;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     sprite.draw(token, Matrix::Y_180, Pivot::Center, None);
///     Input::key_inject_press(Key::Left);
///     if iter == number_of_steps  {
///        sk.send_event(StepperAction::event( "main", ENABLE_FLY_OVER, "false",));
///     }
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/fly_over.jpeg" alt="screenshot" width="200">
#[derive(IStepper)]
pub struct FlyOver {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub enabled: bool,

    pub move_speed: f32,
    pub rotate_speed: f32,
    reverse: f32,
}

unsafe impl Send for FlyOver {}

impl Default for FlyOver {
    fn default() -> Self {
        Self {
            id: "FlyOver".to_string(),
            sk_info: None,
            enabled: true,

            move_speed: 2.0,
            rotate_speed: 90.0,
            reverse: 1.0,
        }
    }
}

impl FlyOver {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        let sk_settings = SkInfo::settings_from(&self.sk_info);
        if sk_settings.mode != AppMode::Simulator {
            let origin_mode = World::get_origin_mode();
            Log::diag(format!("Fly_Over: OriginMode is {:?} ", origin_mode));
            if cfg!(target_os = "android") {
                if origin_mode == OriginMode::Stage {
                    Log::diag("Stage origin reversion");
                    self.reverse *= -1.0;
                }
            } else {
                Log::diag("--");
            }
        }
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(ENABLE_FLY_OVER) {
            self.enabled = value.parse().unwrap_or(false);
            Log::diag(format!("Fly_Over: enabled is {}", self.enabled));
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        //----- move
        let mut camera_root = Renderer::get_camera_root();
        let head = Input::get_head();

        let move_ctrler = Input::controller(Handed::Left);
        let mut move_v = -move_ctrler.stick.x0y();

        if cfg!(debug_assertions) {
            if Input::key(Key::Up).is_just_active() {
                move_v.z = -1.0;
            }
            if Input::key(Key::Down).is_just_active() {
                move_v.z = 1.0;
            }
            if Input::key(Key::Right).is_just_active() {
                move_v.x = -1.0;
            }
            if Input::key(Key::Left).is_just_active() {
                move_v.x = 1.0;
            }
        }
        let mut speed_accelerator = self.move_speed;
        if move_v != Vec3::ZERO {
            move_v *= Vec3 { x: -1.0, y: 1.0, z: 1.0 };
            let camera_pose = camera_root.get_pose();
            let head_forward = head.get_forward();
            move_v.y = head_forward.y * self.reverse;
            let mut shift = camera_pose.position;

            if move_ctrler.is_stick_clicked() {
                speed_accelerator *= 3.0;
            }

            shift += head.orientation * move_v * Time::get_step_unscaledf() * speed_accelerator * self.reverse;
            camera_root.update_t_r(&shift, &camera_pose.orientation);
            Renderer::camera_root(camera_root);
        }

        //----- rotate
        let rotate_stick = Input::controller(Handed::Right).stick;
        let rotate_val = Vec2::dot(rotate_stick, -Vec2::X);

        // Credit to Cazzola: https://discord.com/channels/805160376529715210/805160377130156124/1307293861680255067
        if rotate_val != 0.0 {
            let delta_rotate = Quat::from_angles(0.0, rotate_val * self.rotate_speed * Time::get_step_unscaledf(), 0.0);
            let camera_in_head_space = camera_root * Matrix::t(head.position).get_inverse();
            let rotated = camera_in_head_space * Matrix::r(delta_rotate);
            Renderer::camera_root(rotated * Matrix::t(head.position));
        }
    }
}
