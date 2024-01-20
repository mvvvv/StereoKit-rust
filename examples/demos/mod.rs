use stereokit_rust::{
    maths::{Matrix, Vec3},
    sk::{Sk, StepperAction, StepperId},
};

pub mod program;
pub mod a_stepper;
pub mod sprite1;
pub mod tex1;
pub mod anim1;

use a_stepper::AStepper;

use self::{sprite1::Sprite1, tex1::Tex1, anim1::Anim1};

pub struct Test {
    pub name: String,
    pub launcher: Box<dyn (Fn(&mut Sk) -> StepperId) + 'static>,
}

impl Test {
    pub fn new<T: Fn(&mut Sk) -> StepperId + 'static>(name: String, launcher: T) -> Self {
        Self { name, launcher: Box::new(launcher) }
    }

    pub fn get_tests() -> Box<[Test]> {
        let tests = [
            Test::new("Test A".to_string(), |sk| {
                sk.push_action(StepperAction::add_default::<AStepper>("Test A"));
                "Test A".to_string()
            }),
            Test::new("Test B".to_string(), |sk| {
                let mut a = AStepper::default();
                a.transform = Matrix::t(Vec3::NEG_Z + Vec3::Y);
                sk.push_action(StepperAction::add("Test B".to_string(), a));
                "Test B".to_string()
            }),
            Test::new("Sprites".to_string(), |sk| {
                sk.push_action(StepperAction::add_default::<Sprite1>("Sprites"));
                "Sprites".to_string()
            }),
            Test::new("Textures".to_string(), |sk| {
                sk.push_action(StepperAction::add_default::<Tex1>("Textures"));
                "Textures".to_string()
            }),
            Test::new("Animation".to_string(), |sk| {
                sk.push_action(StepperAction::add_default::<Anim1>("Animation"));
                "Animation".to_string()
            }),
        ];
        Box::new(tests)
    }
}
