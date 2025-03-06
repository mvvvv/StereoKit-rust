use stereokit_rust::{
    font::Font,
    framework::{HAND_MENU_RADIAL_FOCUS, HandMenuAction, HandMenuRadial, HandRadial, HandRadialLayer},
    material::Material,
    maths::{Matrix, Quat, Vec3},
    prelude::*,
    system::{Text, TextStyle},
    util::named_colors::RED,
};

const ID: &str = "demo_0";

/// The basic Stepper. we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
#[derive(IStepper)]
pub struct HandMenuRadial0 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    initialize_completed: bool,
    shutdown_completed: bool,

    value_selected: Rc<RefCell<String>>,
    show_value: Rc<RefCell<bool>>,

    pub transform: Matrix,
    pub text: String,
    text_style: Option<TextStyle>,
}

unsafe impl Send for HandMenuRadial0 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for HandMenuRadial0 {
    fn default() -> Self {
        Self {
            id: "HandMenuRadial0_demo".to_string(),
            sk_info: None,
            initialize_completed: false,
            shutdown_completed: false,

            value_selected: Rc::new(RefCell::new(String::from("One!"))),
            show_value: Rc::new(RefCell::new(true)),

            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "HandMenuRadial0".to_owned(),
            text_style: None,
        }
    }
}

/// All the code here run in the main thread
impl HandMenuRadial0 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));

        let show_value = self.show_value.clone();
        let value_selected1 = self.value_selected.clone();
        let value_selected2 = self.value_selected.clone();
        let value_selected3 = self.value_selected.clone();

        // nice icons
        let mut menu_ico = Material::pbr_clip().copy_for_tex("icons/hamburger.png", true, None).unwrap_or_default();
        menu_ico.clip_cutoff(0.1);

        let mut show_ico = Material::pbr_clip().copy_for_tex("icons/log_viewer.png", true, None).unwrap_or_default();
        show_ico.clip_cutoff(0.1);

        //---Load hand menu
        let hand_menu_radial = HandMenuRadial::new(HandRadialLayer::new(
            "root",
            None,
            Some(10.0),
            vec![
                HandRadial::layer(
                    "Select",
                    Some(menu_ico),
                    None,
                    vec![
                        HandRadial::item(
                            "1",
                            None,
                            move || *value_selected1.borrow_mut() = String::from("One!"),
                            if *self.value_selected.borrow() == "One!" {
                                HandMenuAction::Checked(1)
                            } else {
                                HandMenuAction::Unchecked(1)
                            },
                        ),
                        HandRadial::item(
                            "2",
                            None,
                            move || *value_selected2.borrow_mut() = String::from("Two!"),
                            if *self.value_selected.borrow() == "Two!" {
                                HandMenuAction::Checked(1)
                            } else {
                                HandMenuAction::Unchecked(1)
                            },
                        ),
                        HandRadial::item(
                            "3",
                            None,
                            move || *value_selected3.borrow_mut() = String::from("Three!"),
                            if *self.value_selected.borrow() == "Three!" {
                                HandMenuAction::Checked(1)
                            } else {
                                HandMenuAction::Unchecked(1)
                            },
                        ),
                        HandRadial::item("Back", None, || {}, HandMenuAction::Back),
                        HandRadial::item("Close", None, || {}, HandMenuAction::Close),
                    ],
                ),
                HandRadial::item(
                    "Show\nvalue",
                    Some(show_ico),
                    move || {
                        let prev_value = *show_value.borrow();
                        *show_value.borrow_mut() = !prev_value
                    },
                    if *self.show_value.borrow() { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                ),
                HandRadial::item("Close", None, || {}, HandMenuAction::Close),
            ],
        ));

        self.id = HandMenuRadial::build_id(ID);
        SkInfo::send_event(&self.sk_info, StepperAction::add(self.id.clone(), hand_menu_radial));

        true
    }

    fn start_completed(&mut self) -> bool {
        self.initialize_completed = true;
        SkInfo::send_event(
            &self.sk_info,
            StepperAction::event(self.id.clone(), HAND_MENU_RADIAL_FOCUS, &true.to_string()),
        );
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        let text = if *self.show_value.borrow() {
            &format!("{}\nValue is {}", self.text, *self.value_selected.borrow())
        } else {
            &self.text
        };
        Text::add_at(token, text, self.transform, self.text_style, None, None, None, None, None, None);
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    /// Close the HandMenuStepper0
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            //We indicate we give up before being shutdowned
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::event(self.id.clone(), HAND_MENU_RADIAL_FOCUS, &false.to_string()),
            );
            self.shutdown_completed = false;
            false
        } else {
            //One step further we can disappear in the darkness
            SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
            self.shutdown_completed = true;
            self.shutdown_completed
        }
    }
}
