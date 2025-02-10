#![cfg(test)]

#[cfg(feature = "event-loop")]
fn main() {
    use stereokit_rust::{framework::*, material::Material};

    stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
                                     // Open or close the log window (done by IStepper LogWindow waiting for SHOW_LOG_WINDOW event)
    let mut swap_value = true;

    // nice icon
    let mut menu_ico = Material::pbr_clip().copy_for_tex("icons/hamburger.png", true, None).unwrap_or_default();
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
    SkInfo::send_message(&Some(sk.get_sk_info_clone()), StepperAction::add(id.clone(), hand_menu_stepper));
    number_of_steps = 10;
    test_steps!(// !!!! Get a proper main loop !!!!
        if iter == 1 {
            SkInfo::send_message(&Some(sk.get_sk_info_clone()),
                StepperAction::event(id.clone(), HAND_MENU_RADIAL_FOCUS, &true.to_string()));
        }
        if iter == 8 {
            SkInfo::send_message(&Some(sk.get_sk_info_clone()),
                StepperAction::remove(id.clone()));
        }
    );
}

#[cfg(feature = "no-event-loop")]
fn main() {}
