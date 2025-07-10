#![cfg(test)]
use stereokit_rust::{
    material::Material,
    mesh::Mesh,
    prelude::*,
    system::{BtnState, Input, InputSource, Pointer},
};

#[cfg(feature = "event-loop")]
fn main() {
    stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    use stereokit_rust::{
        framework::HAND_MENU_RADIAL_FOCUS,
        maths::Matrix,
        system::{BtnState, Input, InputSource},
    };

    {
        let (pointer, input_cb) = input_subscribe();
        let id = hand_menu_radial0(Some(sk.get_sk_info_clone()));
        let (circle, material_circle) = material1();

        number_of_steps = 10;
        test_steps!( // !!!! Get a proper main loop !!!!

            // testing input
            assert_eq!(pointer.state, BtnState::Inactive);
            assert_eq!(pointer.tracked, BtnState::Inactive);
            if iter == 0 {
                Input::subscribe(InputSource::CanPress, BtnState::JustActive, Some(input_cb));
                Input::fire_event(InputSource::CanPress, BtnState::JustActive, &pointer);
            } else if iter == 1 {
                 Input::fire_event(pointer.source, BtnState::JustActive, &pointer);
            } else if iter == 8 {
                Input::unsubscribe(InputSource::CanPress, BtnState::JustInactive, Some(input_cb));
            }

            // testing hand_menu_radial0
            if iter == 1 {
                SkInfo::send_event(&Some(sk.get_sk_info_clone()),
                StepperAction::event(id.as_str(), HAND_MENU_RADIAL_FOCUS, &true.to_string()));
            }
            if iter == 8 {
                SkInfo::send_event(&Some(sk.get_sk_info_clone()),
                StepperAction::remove(id.clone()));
            }

            // testing material1
            circle.draw(token, &material_circle,  Matrix::IDENTITY, None, None);
        );
    }
    //Sk::shutdown();
}

#[cfg(feature = "no-event-loop")]
fn main() {
    stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    use stereokit_rust::{
        maths::Matrix,
        system::{BtnState, Input, InputSource},
    };

    {
        let (pointer, input_cb) = input_subscribe();
        let (circle, material_circle) = material1();

        number_of_steps = 10;
        test_steps!( // !!!! Get a proper main loop !!!!

            // testing input
            assert_eq!(pointer.state, BtnState::Inactive);
            assert_eq!(pointer.tracked, BtnState::Inactive);
            if iter == 0 {
                Input::subscribe(InputSource::CanPress, BtnState::JustActive, Some(input_cb));
                Input::fire_event(InputSource::CanPress, BtnState::JustActive, &pointer);
            } else if iter == 1 {
                 Input::fire_event(pointer.source, BtnState::JustActive, &pointer);
            } else if iter == 8 {
                Input::unsubscribe(InputSource::CanPress, BtnState::JustInactive, Some(input_cb));
            }


            // testing material1
            circle.draw(token, &material_circle,  Matrix::IDENTITY, None, None);
        );
    }
    //Sk::shutdown();
}

pub fn input_subscribe() -> (Pointer, unsafe extern "C" fn(InputSource, BtnState, *const Pointer)) {
    let pointer = Input::pointer(0, None);

    #[unsafe(no_mangle)]
    unsafe extern "C" fn input_cb(source: InputSource, input_event: BtnState, in_pointer: *const Pointer) {
        let in_pointer = unsafe { *in_pointer };
        Log::diag(format!("Event {:?}\n  from: {:?}\n   for: {:?}", input_event, source, in_pointer.source));
        //assert_eq!(in_pointer.source, InputSource::Hand | InputSource::HandLeft | InputSource::CanPress);
        assert_eq!(in_pointer.source, InputSource::None);
        assert_eq!(input_event, BtnState::JustActive);
    }
    (pointer, input_cb)
}

pub fn material1() -> (Mesh, Material) {
    use stereokit_rust::{material::Material, maths::Vec3, mesh::Mesh};

    // Create Mesh and its material
    let circle = Mesh::generate_circle(1.0, Vec3::NEG_Z, Vec3::X, None, true);
    let material_circle =
        Material::from_file("shaders/blinker.hlsl.sks", Some("my_material_circle")).unwrap_or_default();
    (circle, material_circle)
}

#[cfg(feature = "event-loop")]
fn hand_menu_radial0(sk_info: Option<Rc<RefCell<SkInfo>>>) -> String {
    // Open or close the log window (done by IStepper LogWindow waiting for SHOW_LOG_WINDOW event)

    use stereokit_rust::framework::{HandMenuAction, HandMenuRadial, HandRadial, HandRadialLayer};
    let mut swap_value = true;

    // nice icon
    let mut menu_ico = Material::pbr_clip().tex_file_copy("icons/hamburger.png", true, None).unwrap_or_default();
    menu_ico.clip_cutoff(0.1);

    //---Load hand menu
    let hand_menu_stepper = HandMenuRadial::new(HandRadialLayer::new(
        "root",
        None,
        Some(100.0),
        vec![
            HandRadial::layer(
                "Todo!",
                Some(menu_ico),
                None,
                vec![
                    HandRadial::item("Back", None, || {}, HandMenuAction::Back),
                    HandRadial::item("Close", None, || {}, HandMenuAction::Close),
                ],
            ),
            HandRadial::item(
                "Swap",
                None,
                move || {
                    swap_value = !swap_value;
                },
                HandMenuAction::Checked(1),
            ),
            HandRadial::item("Close", None, || {}, HandMenuAction::Close),
        ],
    ));
    let id = HandMenuRadial::build_id("1");
    SkInfo::send_event(&sk_info, StepperAction::add(id.clone(), hand_menu_stepper));
    id
}
