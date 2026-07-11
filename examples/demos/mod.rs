#![cfg(not(feature = "no-event-loop"))]
use hand_menu_radial0::HandMenuRadial0;
use haptic1::Haptic1;
use input1::Input1;
use stereokit_rust::prelude::*;

pub mod a_stepper;
pub mod anchor1;
pub mod anim1;
pub mod asset1;
pub mod b_stepper;
pub mod biplane1;
pub mod browser1;
pub mod c_stepper;
pub mod compute1;
pub mod font1;
pub mod hand_menu_radial0;
pub mod hand_menu_radial1;
pub mod haptic1;
pub mod input1;
pub mod interactor1;
pub mod layers1;
pub mod locale1;
pub mod math1;
pub mod permission1;
pub mod program;
pub mod render_list1;
pub mod screen1;
pub mod shaders1;
pub mod shaders2;
pub mod shadows1;
pub mod skin1;
pub mod sprite1;
pub mod stereo1;
pub mod system_deep_link1;
pub mod tex1;
pub mod tex2;
pub mod text1;
pub mod text2;
pub mod threads1;
pub mod threads2;
pub mod ui1;
pub mod ui2;
pub mod ui3;

use self::{
    a_stepper::AStepper, anchor1::Anchor1, anim1::Anim1, asset1::Asset1, b_stepper::BStepper, biplane1::Biplane1,
    c_stepper::CStepper, compute1::Compute1, font1::Font1, interactor1::Interactor1, layers1::Layers1,
    locale1::Locale1, math1::Math1, permission1::Permission1, render_list1::RenderList1, screen1::Screen1,
    shaders1::Shaders1, shaders2::Shaders2, shadows1::Shadows1, skin1::Skin1, sprite1::Sprite1, stereo1::Stereo1,
    tex1::Tex1, tex2::Tex2, text1::Text1, text2::Text2, threads1::Threads1, threads2::Threads2, ui1::Ui1, ui2::Ui2,
    ui3::Ui3,
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
                sk.send_event(StepperAction::add_default::<AStepper>("Test A"));
                "Test A".to_string()
            }),
            Test::new("Test B", |sk| {
                sk.send_event(StepperAction::add_default::<BStepper>("Test B"));
                "Test B".to_string()
            }),
            Test::new("Test C", |sk| {
                sk.send_event(StepperAction::add_default::<CStepper>("Test C"));
                "Test C".to_string()
            }),
            Test::new("HandMenuRadial0", |sk| {
                sk.send_event(StepperAction::add_default::<HandMenuRadial0>("HandMenuRadial0"));
                "HandMenuRadial0".to_string()
            }),
            Test::new("Threads1", |sk| {
                sk.send_event(StepperAction::add_default::<Threads1>("Threads1"));
                "Threads1".to_string()
            }),
            Test::new("Threads2", |sk| {
                sk.send_event(StepperAction::add_default::<Threads2>("Threads2"));
                "Threads2".to_string()
            }),
            Test::new("Anchor1", |sk| {
                sk.send_event(StepperAction::add_default::<Anchor1>("Anchor1"));
                "Anchor1".to_string()
            }),
            Test::new("Text1", |sk| {
                sk.send_event(StepperAction::add_default::<Text1>("Text1"));
                "Text1".to_string()
            }),
            Test::new("Text2", |sk| {
                sk.send_event(StepperAction::add_default::<Text2>("Text2"));
                "Text2".to_string()
            }),
            Test::new("Locale1", |sk| {
                sk.send_event(StepperAction::add_default::<Locale1>("Locale1"));
                "Locale1".to_string()
            }),
            Test::new("Font1", |sk| {
                sk.send_event(StepperAction::add_default::<Font1>("Font1"));
                "Font1".to_string()
            }),
            Test::new("Sprite1", |sk| {
                sk.send_event(StepperAction::add_default::<Sprite1>("Sprite1"));
                "Sprite1".to_string()
            }),
            Test::new("Tex1", |sk| {
                sk.send_event(StepperAction::add_default::<Tex1>("Tex1"));
                "Tex1".to_string()
            }),
            Test::new("Tex2", |sk| {
                sk.send_event(StepperAction::add_default::<Tex2>("Tex2"));
                "Tex2".to_string()
            }),
            Test::new("Stereo1", |sk| {
                sk.send_event(StepperAction::add_default::<Stereo1>("Stereo1"));
                "Stereo1".to_string()
            }),
            Test::new("Screen1", |sk| {
                sk.send_event(StepperAction::add_default::<Screen1>("Screen1"));
                "Screen1".to_string()
            }),
            Test::new("Ui1", |sk| {
                sk.send_event(StepperAction::add_default::<Ui1>("Ui1"));
                "Ui1".to_string()
            }),
            Test::new("Ui2", |sk| {
                sk.send_event(StepperAction::add_default::<Ui2>("Ui2"));
                "Ui2".to_string()
            }),
            Test::new("Ui3", |sk| {
                sk.send_event(StepperAction::add_default::<Ui3>("Ui3"));
                "Ui3".to_string()
            }),
            Test::new("Input1", |sk| {
                sk.send_event(StepperAction::add_default::<Input1>("Input1"));
                "Input1".to_string()
            }),
            Test::new("Haptic1", |sk| {
                sk.send_event(StepperAction::add_default::<Haptic1>("Haptic1"));
                "Haptic1".to_string()
            }),
            Test::new("Interactor1", |sk| {
                sk.send_event(StepperAction::add_default::<Interactor1>("Interactor1"));
                "Interactor1".to_string()
            }),
            Test::new("Anim1", |sk| {
                sk.send_event(StepperAction::add_default::<Anim1>("Anim1"));
                "Anim1".to_string()
            }),
            Test::new("Shaders1", |sk| {
                sk.send_event(StepperAction::add_default::<Shaders1>("Shaders1"));
                "Shaders1".to_string()
            }),
            Test::new("Shaders2", |sk| {
                sk.send_event(StepperAction::add_default::<Shaders2>("Shaders2"));
                "Shaders2".to_string()
            }),
            Test::new("Compute1", |sk| {
                sk.send_event(StepperAction::add_default::<Compute1>("Compute1"));
                "Compute1".to_string()
            }),
            Test::new("Math1", |sk| {
                sk.send_event(StepperAction::add_default::<Math1>("Math1"));
                "Math1".to_string()
            }),
            Test::new("Permission1", |sk| {
                sk.send_event(StepperAction::add_default::<Permission1>("Permission1"));
                "Permission1".to_string()
            }),
            Test::new("Asset1", |sk| {
                sk.send_event(StepperAction::add_default::<Asset1>("Asset1"));
                "Asset1".to_string()
            }),
            Test::new("RenderList1", |sk| {
                sk.send_event(StepperAction::add_default::<RenderList1>("RenderList1"));
                "RenderList1".to_string()
            }),
            Test::new("Biplane1", |sk| {
                sk.send_event(StepperAction::add_default::<Biplane1>("Biplane1"));
                "Biplane1".to_string()
            }),
            Test::new("Layers1", |sk| {
                sk.send_event(StepperAction::add_default::<Layers1>("Layers1"));
                "Layers1".to_string()
            }),
            Test::new("Shadows1", |sk| {
                sk.send_event(StepperAction::add_default::<Shadows1>("Shadows1"));
                "Shadows1".to_string()
            }),
            Test::new("Skin1", |sk| {
                sk.send_event(StepperAction::add_default::<Skin1>("Skin1"));
                "Skin1".to_string()
            }),
            #[cfg(target_os = "android")]
            Test::new("Browser1", |sk| {
                sk.send_event(StepperAction::add_default::<browser1::Browser1>("Browser1"));
                "Browser1".to_string()
            }),
            #[cfg(target_os = "android")]
            Test::new("SystemDeepLink", |sk| {
                sk.send_event(StepperAction::add_default::<system_deep_link1::SystemDeepLink1>("SystemDeepLink"));
                "SystemDeepLink".to_string()
            }),
        ];
        Box::new(tests)
    }
}
