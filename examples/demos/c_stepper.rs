use std::{cell::RefCell, rc::Rc};

use stereokit_macros::IStepper;
use stereokit_rust::{
    event_loop::{IStepper, StepperAction, StepperId},
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    sk::{MainThreadToken, SkInfo},
    system::{Renderer, Text, TextStyle},
    util::{named_colors::RED, Time},
};
/// The basic Stepper. we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
#[derive(IStepper)]
pub struct CStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    initialize_completed: bool,
    enabled: bool,
    shutdown_completed: bool,

    pub transform: Matrix,
    round_cube: Option<Mesh>,
    pub text: String,
    text_style: Option<TextStyle>,
}

unsafe impl Send for CStepper {}

/// This code may be called in some threads, so no StereoKit code
impl Default for CStepper {
    fn default() -> Self {
        Self {
            id: "CStepper".to_string(),
            sk_info: None,
            initialize_completed: false,
            enabled: true,
            shutdown_completed: false,

            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            round_cube: None,
            text: "Stepper C".to_owned(),
            text_style: None,
        }
    }
}

/// All the code here run in the main thread
impl CStepper {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.round_cube = Some(Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.2, Some(16)));
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));
        self.initialize_completed = true;
        true
    }

    /// Called from IStepper::initialize_done(waiting for true response)
    fn start_completed(&self) -> bool {
        self.initialize_completed
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {
        // if key == "CStepper" {
        //     self.enabled = value.parse().unwrap_or(false)
        // }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        self.transform *= Matrix::r(Quat::from_angles(0.0, 10.0 * Time::get_stepf(), 0.0));
        if let Some(round_cube) = &self.round_cube {
            Renderer::add_mesh(token, round_cube, Material::pbr(), self.transform, Some(RED.into()), None);
        }
        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            self.shutdown_completed = true;
            true
        } else {
            self.shutdown_completed
        }
    }
}
