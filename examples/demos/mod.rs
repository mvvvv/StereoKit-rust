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
#[cfg(feature = "file-browser")]
pub mod documents1;
pub mod font1;
pub mod hand_menu_radial0;
pub mod hand_menu_radial1;
pub mod haptic1;
pub mod input1;
pub mod interactor1;
pub mod layers1;
pub mod locale1;
pub mod math1;
pub mod mixer1;
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
pub mod subpass1;
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
    locale1::Locale1, math1::Math1, mixer1::Mixer1, permission1::Permission1, render_list1::RenderList1,
    screen1::Screen1, shaders1::Shaders1, shaders2::Shaders2, shadows1::Shadows1, skin1::Skin1, sprite1::Sprite1,
    stereo1::Stereo1, subpass1::Subpass1, tex1::Tex1, tex2::Tex2, text1::Text1, text2::Text2, threads1::Threads1,
    threads2::Threads2, ui1::Ui1, ui2::Ui2, ui3::Ui3,
};

pub struct Test {
    pub name: String,
    pub launcher: Box<dyn (Fn(&mut Sk) -> StepperId) + 'static>,
}

impl Test {
    pub fn new<T: Fn(&mut Sk) -> StepperId + 'static>(name: impl AsRef<str>, launcher: T) -> Self {
        Self { name: name.as_ref().to_string(), launcher: Box::new(launcher) }
    }

    /// Helper générique pour créer un Test en ne spécifiant le nom qu'une seule fois.
    pub fn from_stepper<S: IStepper + Default + Send + 'static>(name: &str) -> Self {
        let name_str = name.to_string();
        Self::new(name, move |sk| {
            sk.send_event(StepperAction::add_default::<S>(&name_str));
            name_str.clone()
        })
    }

    pub fn get_tests() -> Box<[Test]> {
        let tests = [
            Test::from_stepper::<AStepper>("Test A"),
            Test::from_stepper::<BStepper>("Test B"),
            Test::from_stepper::<CStepper>("Test C"),
            Test::from_stepper::<HandMenuRadial0>("HandMenuRadial0"),
            Test::from_stepper::<Threads1>("Threads1"),
            Test::from_stepper::<Threads2>("Threads2"),
            Test::from_stepper::<Anchor1>("Anchor1"),
            Test::from_stepper::<Text1>("Text1"),
            Test::from_stepper::<Text2>("Text2"),
            Test::from_stepper::<Locale1>("Locale1"),
            Test::from_stepper::<Font1>("Font1"),
            Test::from_stepper::<Sprite1>("Sprite1"),
            Test::from_stepper::<Tex1>("Tex1"),
            Test::from_stepper::<Tex2>("Tex2"),
            Test::from_stepper::<Stereo1>("Stereo1"),
            Test::from_stepper::<Screen1>("Screen1"),
            Test::from_stepper::<Ui1>("Ui1"),
            Test::from_stepper::<Ui2>("Ui2"),
            Test::from_stepper::<Ui3>("Ui3"),
            Test::from_stepper::<Input1>("Input1"),
            Test::from_stepper::<Haptic1>("Haptic1"),
            Test::from_stepper::<Interactor1>("Interactor1"),
            Test::from_stepper::<Anim1>("Anim1"),
            Test::from_stepper::<Shaders1>("Shaders1"),
            Test::from_stepper::<Shaders2>("Shaders2"),
            Test::from_stepper::<Subpass1>("Subpass1"),
            Test::from_stepper::<Compute1>("Compute1"),
            Test::from_stepper::<Math1>("Math1"),
            Test::from_stepper::<Mixer1>("Mixer1"),
            Test::from_stepper::<Permission1>("Permission1"),
            Test::from_stepper::<Asset1>("Asset1"),
            Test::from_stepper::<RenderList1>("RenderList1"),
            Test::from_stepper::<Biplane1>("Biplane1"),
            Test::from_stepper::<Layers1>("Layers1"),
            Test::from_stepper::<Shadows1>("Shadows1"),
            Test::from_stepper::<Skin1>("Skin1"),
            #[cfg(feature = "file-browser")]
            Test::from_stepper::<documents1::Documents1>("Documents1"),
            #[cfg(target_os = "android")]
            Test::from_stepper::<browser1::Browser1>("Browser1"),
            #[cfg(target_os = "android")]
            Test::from_stepper::<system_deep_link1::SystemDeepLink1>("SystemDeepLink"),
        ];
        Box::new(tests)
    }
}
