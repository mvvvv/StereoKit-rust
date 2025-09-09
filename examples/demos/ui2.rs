use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec3},
    prelude::*,
    system::{Hierarchy, Text, TextStyle},
    ui::Ui,
    util::named_colors::RED,
};

/// Copycat of the example https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tests/TestWindowPoseOverride.cs
#[derive(IStepper)]
pub struct Ui2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    /// Pose that can be modified by UI interaction, then constrained
    pub modifiable_pose: Pose,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Ui2 {}

impl Default for Ui2 {
    fn default() -> Self {
        Self {
            id: "Ui2".to_string(),
            sk_info: None,

            modifiable_pose: Pose::new(Vec3::new(0.1, 1.5, -0.5), Some(Quat::look_dir(Vec3::new(0.0, 1.0, 1.0)))),

            text: "Ui2".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Ui2 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        // Window 1: Modified Pose
        // With this window, we're adding a constraint to a normal stateful
        // pose. This is a common scenario, where the developer wants a gravity
        // or floor oriented Handle/Window, like some sort of house plant
        // hologram. We do this by quashing out the X and Z rotation of the
        // Pose's orientation, and renormalizing it to make it a valid rotation.
        Ui::window_begin("Modified Pose", &mut self.modifiable_pose, None, None, None);

        // Pop the hierarchy to apply custom transformations
        Hierarchy::pop(token);

        // Quash X and Z rotation to constrain the pose
        let mut constrained_orientation = self.modifiable_pose.orientation;
        constrained_orientation.x = 0.0;
        constrained_orientation.z = 0.0;
        constrained_orientation.normalize();

        // Update the pose with constrained orientation
        let constrained_pose = Pose::new(self.modifiable_pose.position, Some(constrained_orientation));

        // Push the constrained pose back to hierarchy
        Hierarchy::push(token, Matrix::from(constrained_pose), None);

        Ui::text(
            "We're quashing rotation of this Window's pose on the X and Z axes after doing regular handle logic.",
            None,
            None,
            None,
            None,
            None,
            None,
        );

        Ui::window_end();

        // Window 2: Override Pose
        // With this window, we're completely overriding the pose. Detection of
        // interaction always happens with the initial pose (it has to), but
        // rendering happens completely using the override pose. This means
        // that this window will still be interactive from its _original_
        // location, despite not being visible there! This is an unusual
        // circumstance and not something we need to fix, usually the final
        // pose will be fed back in as the initial pose on the start of the
        // next frame.
        let mut override_pose = Pose::new(Vec3::new(-0.1, 1.0, -0.5), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 0.0))));

        Ui::window_begin("Override Pose", &mut override_pose, None, None, None);

        // Pop the hierarchy to completely override the pose
        Hierarchy::pop(token);

        // Push a completely different pose for rendering
        let rendering_pose = Pose::new(Vec3::new(-0.1, 1.3, -0.5), Some(Quat::look_dir(Vec3::new(0.0, 0.0, 1.0))));
        Hierarchy::push(token, Matrix::from(rendering_pose), None);

        Ui::text(
            "We're completely overriding this Window's pose to prove that even extreme modifications to the Window's pose will still properly apply.",
            None,
            None,
            None,
            None,
            None,
            None,
        );

        Ui::window_end();

        // Window 3: Pose zero
        // this will push an automatically determined pose onto the transform stack
        // and keep its position if you relaunch the demo
        Ui::window_begin_auto("Pose Zero", None, None, None);
        Ui::text("We memorize position", None, None, None, None, None, None);
        Ui::window_end();

        // Display the demo title text
        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
