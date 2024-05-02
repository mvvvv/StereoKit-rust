use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    sk::{IStepper, MainThreadToken, SkInfo, StepperClosures, StepperId},
    system::{Log, Renderer, Text},
    util::{named_colors::RED, Time},
};
/// The basic Stepper. This stepper is used for Thread1 demo, we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
pub struct BStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub text: String,
    closures: StepperClosures<'static>,
}

unsafe impl Send for BStepper {}

/// This code may be called in some threads, so no StereoKit code
impl Default for BStepper {
    fn default() -> Self {
        Self {
            id: "BStepper".to_string(),
            sk_info: None,
            text: "Stepper B".to_owned(),
            closures: StepperClosures::new(),
        }
    }
}

/// All the code here run in the main thread
impl IStepper for BStepper {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);

        let mut transform = Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0));
        let mut round_cube = Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.005, Some(16));
        round_cube.id("round_cube BStepper");
        let text_style = Some(Text::make_style(Font::default(), 0.3, RED));
        let text = self.text.clone();

        self.closures.set(
            move |token| {
                transform *= Matrix::t(Vec3::Z * 0.2 * Time::get_stepf());
                Renderer::add_mesh(token, &round_cube, Material::pbr(), transform, Some(RED.into()), None);
                Text::add_at(token, &text, transform, text_style, None, None, None, None, None, None);
                // (1) You can't do that here: self.text = "youpi".into();
            },
            || Log::diag("Closing Stepper B !!!"),
        );
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.closures.step(token);
        // (2) Add here The code about fields that are shared with shutdown >>>
        {}
        //<<<
    }

    fn shutdown(&mut self) {
        self.closures.shutdown();
        // (3) Add here The code about fields that are shared with step >>>
        {}
        //<<<
    }
}
