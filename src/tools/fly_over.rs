use std::{cell::RefCell, rc::Rc};

use crate::{
    event_loop::{IStepper, StepperId},
    maths::{Matrix, Quat, Vec2, Vec3},
    sk::{MainThreadToken, SkInfo},
    system::{Handed, Input, Renderer},
    util::Time,
};

pub struct FlyOver {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub move_speed: f32,
    pub rotate_speed: f32,
}

unsafe impl Send for FlyOver {}

impl Default for FlyOver {
    fn default() -> Self {
        Self { id: "FlyOver".to_string(), sk_info: None, move_speed: 2.0, rotate_speed: 90.0 }
    }
}

impl IStepper for FlyOver {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl FlyOver {
    fn draw(&mut self, _token: &MainThreadToken) {
        //----- move
        let camera_pose = Renderer::get_camera_root().get_pose();
        let move_ctrler = Input::controller(Handed::Left);
        let mut move_v = -move_ctrler.stick.x0y();
        let mut speed_accelerator = self.move_speed;
        let mut apply = false;
        let mut shift = camera_pose.position;
        let mut rotate = camera_pose.orientation;
        if move_v != Vec3::ZERO {
            let head_rotate = Input::get_head().get_forward();
            move_v.y = head_rotate.y;

            if move_ctrler.is_stick_clicked() {
                speed_accelerator *= 3.0;
            }

            shift +=
                camera_pose.orientation * move_v * Time::get_step_unscaledf() * self.move_speed * speed_accelerator;
            apply = true
        }

        //----- rotate
        let rotate_stick = Input::controller(Handed::Right).stick;
        let rotate_val = Vec2::dot(rotate_stick, Vec2::X);

        if rotate_val != 0.0 {
            let delta_rotate = Quat::from_angles(0.0, rotate_val * self.rotate_speed * Time::get_step_unscaledf(), 0.0);
            rotate *= delta_rotate;
            apply = true;
        }

        if apply {
            Renderer::camera_root(Matrix::tr(&shift, &rotate));
        }
    }
}
