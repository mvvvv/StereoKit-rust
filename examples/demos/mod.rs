use stereokit_rust::prelude::*;

pub mod a_stepper;
pub mod anchor1;
pub mod anim1;
pub mod asset1;
pub mod b_stepper;
pub mod biplane1;
pub mod c_stepper;
pub mod hand_menu_radial1;
pub mod math1;
pub mod program;
pub mod render_list1;
pub mod shaders1;
pub mod sprite1;
pub mod tex1;
pub mod text1;
pub mod text2;
pub mod threads1;
pub mod threads2;
pub mod ui1;

use self::{
    a_stepper::AStepper, anchor1::Anchor1, anim1::Anim1, asset1::Asset1, b_stepper::BStepper, biplane1::Biplane1,
    c_stepper::CStepper, math1::Math1, render_list1::RenderList1, shaders1::Shader1, sprite1::Sprite1, tex1::Tex1,
    text1::Text1, text2::Text2, threads1::Threads1, threads2::Threads2, ui1::Ui1,
};

pub struct Test {
    pub name: String,
    pub launcher: Box<dyn (Fn(&mut Sk) -> StepperId) + 'static>,
}

impl Test {
    pub fn new<T: Fn(&mut Sk) -> StepperId + 'static>(name: impl AsRef<str>, launcher: T) -> Self {
        Self { name: name.as_ref().to_string(), launcher: Box::new(launcher) }
    }

    pub fn get_tests() -> Box<[Test]> {
        let tests = [
            Test::new("Test A", |sk| {
                sk.push_action(StepperAction::add_default::<AStepper>("Test A"));
                "Test A".to_string()
            }),
            Test::new("Test B", |sk| {
                sk.push_action(StepperAction::add_default::<BStepper>("Test B"));
                "Test B".to_string()
            }),
            Test::new("Test C", |sk| {
                sk.push_action(StepperAction::add_default::<CStepper>("Test C"));
                "Test C".to_string()
            }),
            Test::new("Threads1", |sk| {
                sk.push_action(StepperAction::add_default::<Threads1>("Threads1"));
                "Threads1".to_string()
            }),
            Test::new("Threads2", |sk| {
                sk.push_action(StepperAction::add_default::<Threads2>("Threads2"));
                "Threads2".to_string()
            }),
            Test::new("Anchor1", |sk| {
                sk.push_action(StepperAction::add_default::<Anchor1>("Anchor1"));
                "Anchor1".to_string()
            }),
            Test::new("Text1", |sk| {
                sk.push_action(StepperAction::add_default::<Text1>("Text1"));
                "Text1".to_string()
            }),
            Test::new("Text2", |sk| {
                sk.push_action(StepperAction::add_default::<Text2>("Text2"));
                "Text2".to_string()
            }),
            Test::new("Sprite1", |sk| {
                sk.push_action(StepperAction::add_default::<Sprite1>("Sprite1"));
                "Sprite1".to_string()
            }),
            Test::new("Tex1", |sk| {
                sk.push_action(StepperAction::add_default::<Tex1>("Tex1"));
                "Tex1".to_string()
            }),
            Test::new("Ui1", |sk| {
                sk.push_action(StepperAction::add_default::<Ui1>("Ui1"));
                "Ui1".to_string()
            }),
            Test::new("Anim1", |sk| {
                sk.push_action(StepperAction::add_default::<Anim1>("Anim1"));
                "Anim1".to_string()
            }),
            Test::new("Shader1", |sk| {
                sk.push_action(StepperAction::add_default::<Shader1>("Shader1"));
                "Shader1".to_string()
            }),
            Test::new("Math1", |sk| {
                sk.push_action(StepperAction::add_default::<Math1>("Math1"));
                "Math1".to_string()
            }),
            Test::new("Asset1", |sk| {
                sk.push_action(StepperAction::add_default::<Asset1>("Asset1"));
                "Asset1".to_string()
            }),
            Test::new("RenderList1", |sk| {
                sk.push_action(StepperAction::add_default::<RenderList1>("RenderList1"));
                "RenderList1".to_string()
            }),
            Test::new("Biplane1", |sk| {
                sk.push_action(StepperAction::add_default::<Biplane1>("Biplane1"));
                "Biplane1".to_string()
            }),
        ];
        Box::new(tests)
    }
}
