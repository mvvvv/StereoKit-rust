use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    anchor::{Anchor, AnchorCaps},
    event_loop::{IStepper, StepperId},
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Ray, Vec3},
    mesh::Mesh,
    sk::{MainThreadToken, SkInfo},
    system::{Handed, Input, Lines, Log, Text, TextStyle},
    ui::{Ui, UiCut},
    util::named_colors::{RED, WHITE},
};

pub struct Anchor1 {
    id: StepperId,
    sk: Option<Rc<RefCell<SkInfo>>>,
    pub transform: Matrix,
    pub window_pose: Pose,
    anchors: Vec<Anchor>,
    ui_box_material: Material,
    text: String,
    text_style: TextStyle,
}

unsafe impl Send for Anchor1 {}

impl Default for Anchor1 {
    fn default() -> Self {
        Self {
            id: "Anchor1".to_string(),
            sk: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            window_pose: Pose::new(Vec3::NEG_Z * 0.1 + Vec3::Y * 1.5, Some(Quat::from_angles(0.0, 180.0, 0.0))),
            anchors: vec![],
            ui_box_material: Material::ui_box(),
            text: "Anchor1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl IStepper for Anchor1 {
    fn initialize(&mut self, id: StepperId, sk: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk = Some(sk);
        self.anchors = Anchor::anchors().collect();
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl Anchor1 {
    fn draw(&mut self, token: &MainThreadToken) {
        // we need a pointer
        let right_hand = Input::hand(Handed::Right);
        let mut hand_pose = right_hand.palm;
        if hand_pose.position == Vec3::ZERO {
            hand_pose = Input::controller(Handed::Right).pose;
        }
        let ray = Ray::new(hand_pose.position, hand_pose.get_up());
        if right_hand.is_just_pinched() {
            Log::diag(format!("{:?}", ray));
        }
        Lines::add(token, ray.position, ray.position + ray.direction * 0.5, WHITE, None, 0.01);

        // window for working with the anchors
        Ui::window_begin("Anchors", &mut self.window_pose, None, None, None);
        // checking if we support anchors
        Ui::layout_push_cut(UiCut::Left, 0.1, true);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
        Ui::label("Capabilities:", None, false);
        Ui::hseparator();

        let storable = !(Anchor::get_capabilities() & AnchorCaps::Storable).is_empty();
        let stability = !(Anchor::get_capabilities() & AnchorCaps::Stability).is_empty();
        if !storable && !stability {
            Ui::label("None", None, false)
        } else {
            if storable {
                Ui::label("Storable", None, false)
            }
            if stability {
                Ui::label("Stability", None, false)
            }
        }
        Ui::layout_pop();

        // add a new anchor where the line point
        let mut pose_tip = hand_pose;
        pose_tip.position += ray.direction * 0.5;
        Ui::push_enabled(storable || stability, None);
        if Ui::button("Create New", None) {
            let anchor = Anchor::from_pose(pose_tip);
            anchor.try_set_persistent(true);
            self.anchors.push(anchor);
        }
        Ui::pop_enabled();
        Ui::window_end();

        // Show the anchors
        let mut selected: Option<&Anchor> = None;
        for anchor in self.anchors.iter() {
            let a_pose = anchor.get_pose();
            Lines::add_axis(token, a_pose, Some(0.1), None);
            if a_pose.position.in_radius(pose_tip.position, 0.05) {
                selected = Some(anchor);
            }
        }

        // outline the one pointed
        if let Some(anchor_selected) = selected {
            Mesh::cube().draw(
                token,
                &self.ui_box_material,
                anchor_selected.get_pose().to_matrix(Some(Vec3::ONE * 0.1)),
                None,
                None,
            );
        }

        // log new anchor
        for anchor in Anchor::new_anchors() {
            Log::info(format!("New anchor : {}", anchor.get_name()));
        }
        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
