use stereokit_rust::{
    anchor::{Anchor, AnchorCaps},
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Ray, Vec3},
    mesh::Mesh,
    prelude::*,
    spatial::{Spatial, SpatialCapability},
    system::{Handed, Input, Lines, Text, TextBuilder, TextStyle},
    ui::{Ui, UiCut, UiPad},
    util::named_colors::{RED, WHITE},
};

#[derive(IStepper)]
pub struct Anchor1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub transform: Matrix,
    pub window_pose: Pose,
    anchors: Vec<Anchor>,
    ui_box_material: Material,
    text: String,
    text_style: TextStyle,
    was_requested: bool,
    shutdown_completed: bool,
}

unsafe impl Send for Anchor1 {}

impl Default for Anchor1 {
    fn default() -> Self {
        Self {
            id: "Anchor1".to_string(),
            sk_info: None,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            window_pose: Pose::new(Vec3::NEG_Z * 0.5 + Vec3::Y * 1.5, Some(Quat::from_angles(0.0, 180.0, 0.0))),
            anchors: vec![],
            ui_box_material: Material::ui_box(),
            text: "Anchor1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            was_requested: false,
            shutdown_completed: false,
        }
    }
}

impl Anchor1 {
    fn start(&mut self) -> bool {
        // Another demo may have used Spatial::disable, which overrides Anchor's own request.
        self.was_requested = Spatial::is_requested(SpatialCapability::Anchor);
        Spatial::request(SpatialCapability::Anchor);
        for anchor in Anchor::anchors() {
            self.track(anchor);
        }
        true
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            if !self.was_requested {
                Spatial::disable(SpatialCapability::Anchor);
            }
            self.anchors.clear();
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }

    /// Each enumeration makes new Anchor objects, so Name is what identifies them.
    fn track(&mut self, anchor: Anchor) {
        if !self.anchors.iter().any(|a| a.get_name() == anchor.get_name()) {
            self.anchors.push(anchor);
        }
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, _token: &MainThreadToken) {
        // we need a pointer
        let right_hand = Input::hand(Handed::Right);
        let mut hand_pose = right_hand.palm;
        if hand_pose.position == Vec3::ZERO {
            hand_pose = Input::controller(Handed::Right).pose;
        }
        let ray = Ray::new(hand_pose.position, hand_pose.get_up());
        if right_hand.is_just_pinched() {
            Log::diag(format!("{ray:?}"));
        }
        Lines::add(ray.position, ray.position + ray.direction * 0.5, WHITE, None, 0.01);

        // window for working with the anchors
        Ui::window("Anchors").pose(&mut self.window_pose).begin();
        // checking if we support anchors
        Ui::layout_push_cut(UiCut::Left, 0.1, true);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), Some(UiPad::Outside));
        Ui::label("Capabilities:").draw();
        Ui::hseparator();

        let storable = !(Anchor::get_capabilities() & AnchorCaps::Storable).is_empty();
        let stability = !(Anchor::get_capabilities() & AnchorCaps::Stability).is_empty();
        if !storable && !stability {
            Ui::label("None").draw()
        } else {
            if storable {
                Ui::label("Storable").draw()
            }
            if stability {
                Ui::label("Stability").draw()
            }
        }
        Ui::layout_pop();

        // add a new anchor where the line point
        let mut pose_tip = hand_pose;
        pose_tip.position += ray.direction * 0.5;
        Ui::push_enabled(storable || stability, None);
        if Ui::button("Create New").press() {
            if let Ok(anchor) = Anchor::from_pose(pose_tip) {
                anchor.try_set_persistent(true);
                self.track(anchor);
            } else {
                Log::err("Failed to create anchor from pose")
            }
        }
        Ui::pop_enabled();
        Ui::window_end();

        // Show the anchors
        let mut selected: Option<&Anchor> = None;
        for anchor in self.anchors.iter() {
            let a_pose = anchor.get_pose();
            Lines::add_axis(a_pose, Some(0.1), None);
            if a_pose.position.in_radius(pose_tip.position, 0.05) {
                selected = Some(anchor);
            }
        }

        // outline the one pointed
        if let Some(anchor_selected) = selected {
            Mesh::cube().draw(
                &self.ui_box_material,
                anchor_selected.get_pose().to_matrix(Some(Vec3::ONE * 0.1)),
                None,
                None,
            );
        }

        // Persisted anchors can be discovered well after this demo starts.
        for anchor in Anchor::new_anchors() {
            Log::info(format!("New anchor: {}", anchor.get_name()));
            self.track(anchor);
        }
        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }
}
