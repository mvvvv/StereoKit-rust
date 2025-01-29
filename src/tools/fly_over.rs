use std::{cell::RefCell, rc::Rc};

use stereokit_macros::IStepper;

use crate::{
    event_loop::{IStepper, StepperAction, StepperId},
    maths::{Matrix, Quat, Vec2, Vec3},
    sk::{AppMode, MainThreadToken, OriginMode, SkInfo},
    system::{Handed, Input, Key, Log, Renderer, World},
    util::Time,
};

pub const ENABLE_FLY_OVER: &str = "Tool_EnableFlyOver";

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
                move_v.x = 1.0;
            }
            if Input::key(Key::Left).is_just_active() {
                move_v.x = -1.0;
            }
        }
        let mut speed_accelerator = self.move_speed;
        if move_v != Vec3::ZERO {
            let camera_pose = camera_root.get_pose();
            let head_forward = head.get_forward();
            move_v.y = head_forward.y * self.reverse;
            let mut shift = camera_pose.position;

            if move_ctrler.is_stick_clicked() {
                speed_accelerator *= 3.0;
            }

            shift += head.orientation * move_v * Time::get_step_unscaledf() * speed_accelerator * self.reverse;
            camera_root = Matrix::tr(&shift, &camera_pose.orientation);
            Renderer::camera_root(camera_root);
        }

        //----- rotate

        let rotate_stick = Input::controller(Handed::Right).stick;
        let rotate_val = Vec2::dot(rotate_stick, Vec2::X);

        // Credit to Cazzola: https://discord.com/channels/805160376529715210/805160377130156124/1307293861680255067
        if rotate_val != 0.0 {
            let delta_rotate = Quat::from_angles(0.0, rotate_val * self.rotate_speed * Time::get_step_unscaledf(), 0.0);
            let camera_in_head_space = camera_root * Matrix::t(head.position).get_inverse();
            let rotated = camera_in_head_space * Matrix::r(delta_rotate);
            Renderer::camera_root(rotated * Matrix::t(head.position));
        }
    }
}
