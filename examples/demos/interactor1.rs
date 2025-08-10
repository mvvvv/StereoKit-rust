use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    sprite::Sprite,
    system::{
        DefaultInteractors, Interaction, Interactor, InteractorActivation, InteractorEvent, InteractorType, Lines, Log,
        Text, TextStyle,
    },
    ui::{Ui, UiBtnLayout, UiPad},
    util::{Color128, named_colors},
};

/// Simple demo showcasing basic Interactor capabilities
#[derive(IStepper)]
pub struct Interactor1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    // Single test interactor
    test_interactor: Option<Interactor>,

    // Demo state
    window_pose: Pose,

    // Target sphere for interaction testing
    target_sphere: (Vec3, f32, Color128), // position, radius, color

    // Interactor configuration
    shape_type: InteractorType,
    events: InteractorEvent,
    activation_type: InteractorActivation,
    min_distance: f32,
    capsule_radius: f32,
    secondary_motion_dimensions: i32,

    // Text rendering cache
    text_transform: Matrix,
    text_style: TextStyle,
    text_center: Vec3,        // Center position for text rendering
    text_touch_distance: f32, // Distance threshold for touch detection
    text_content: String,     // Fixed text content
}

unsafe impl Send for Interactor1 {}

impl Default for Interactor1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Interactor1 {
    pub fn new() -> Self {
        let text_center = Vec3::new(0.0, 1.5, -0.5); // Center position for text rendering
        let text_content = "Interactor1\nTouch me to close this demo\nwhen the interactor is unusable.".to_string();
        Self {
            id: "Interactor1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            test_interactor: None,
            window_pose: Pose::new(Vec3::new(0.6, 1.3, -0.5), Some(Quat::look_dir(-Vec3::FORWARD))),
            target_sphere: (Vec3::new(0.0, 1.1, -0.4), 0.05, named_colors::CYAN.into()),
            shape_type: InteractorType::Line,
            events: InteractorEvent::Pinch | InteractorEvent::Poke | InteractorEvent::Grip,
            activation_type: InteractorActivation::Position,
            min_distance: -1.0,
            capsule_radius: 0.01,
            secondary_motion_dimensions: 3,

            // Initialize text rendering cache
            text_transform: Matrix::t_r(text_center - Vec3::new(0.1, 0.00, 0.0), [0.0, 180.0, 0.0]), // final position to reset text
            text_style: Text::make_style(Font::default(), 0.03, named_colors::RED),
            text_center,              // Center position for text rendering
            text_touch_distance: 0.1, // 5cm radius for touch detection
            text_content,
        }
    }

    pub fn start(&mut self) -> bool {
        // Save current default interactors
        Interaction::set_default_interactors(DefaultInteractors::None);

        // Create interactor with current configuration
        self.recreate_interactor();

        Log::info("Interactor Demo: Created test interactor");
        true
    }

    fn recreate_interactor(&mut self) {
        // Destroy existing interactor if it exists
        if let Some(interactor) = self.test_interactor.take() {
            interactor.destroy();
        }

        // Create new interactor with current configuration
        let interactor = Interactor::create(
            self.shape_type,
            self.events,
            self.activation_type,
            -1,
            self.capsule_radius,
            self.secondary_motion_dimensions,
        );

        self.min_distance = interactor.get_min_distance();

        self.test_interactor = Some(interactor);

        Log::info(format!(
            "Recreated interactor: Type={:?}, Events={:?}, Activation={:?}",
            self.shape_type, self.events, self.activation_type
        ));
    }

    pub fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {
        // Handle events here if needed
    }

    pub fn draw(&mut self, token: &MainThreadToken) {
        // Update interactor with right hand data
        if let Some(interactor) = &self.test_interactor {
            use stereokit_rust::maths::Vec3;
            use stereokit_rust::system::{FingerId, Handed, Input, JointId};

            let right_hand = Input::hand(Handed::Right);

            // Use hand palm position for capsule start and index finger tip for capsule end
            let capsule_start = right_hand.get(FingerId::Little, JointId::KnuckleMajor).position;
            let capsule_end = right_hand.get(FingerId::Little, JointId::Tip).position;

            // Use hand pose for motion
            let motion = right_hand.palm;
            let motion_anchor = right_hand.palm.position;
            let secondary_motion = Vec3::ZERO; // No secondary motion for now

            // Use pinch state for activation
            let active = right_hand.pinch;
            let tracked = right_hand.tracked;

            interactor.update(capsule_start, capsule_end, motion, motion_anchor, secondary_motion, active, tracked);

            // Visualize the interactor with a red line
            if right_hand.tracked.is_active() {
                // Draw line in different colors based on activation state
                let line_color = if right_hand.pinch.is_active() {
                    named_colors::GREEN // Green when pinching (active)
                } else {
                    named_colors::RED // Red when not pinching (inactive)
                };
                Lines::add(token, capsule_start, capsule_end, line_color, None, self.capsule_radius);
            }
        }

        // Draw simple control panel
        self.draw_control_panel();

        // Draw basic 3D scene
        self.draw_3d_scene(token);
    }

    fn draw_control_panel(&mut self) {
        let window_size = Some(Vec2::new(0.5, 0.7));
        Ui::window_begin(
            "Interactor Demo using the little finger of the right hand",
            &mut self.window_pose,
            window_size,
            None,
            None,
        );

        // Interactor Configuration Section
        Ui::text("Interactor Configuration:", None, None, None, None, None, None);

        let mut changed = false;

        // Get radio sprites once for reuse
        let radio_off = Sprite::radio_off();
        let radio_on = Sprite::radio_on();

        // Create horizontal layout with three columns
        Ui::layout_push(Vec3::new(0.20, -0.04, 0.0), Vec2::new(0.45, 0.4), false);
        // Column 1: Shape Type selection
        Ui::text("Shape Type:", None, None, None, None, None, None);
        Ui::panel_begin(Some(UiPad::Outside));
        if Ui::radio_img(
            "Point",
            self.shape_type == InteractorType::Point,
            &radio_off,
            &radio_on,
            UiBtnLayout::Left,
            None,
        ) && self.shape_type != InteractorType::Point
        {
            self.shape_type = InteractorType::Point;
            changed = true;
        }
        if Ui::radio_img(
            "Line",
            self.shape_type == InteractorType::Line,
            &radio_off,
            &radio_on,
            UiBtnLayout::Left,
            None,
        ) && self.shape_type != InteractorType::Line
        {
            self.shape_type = InteractorType::Line;
            changed = true;
        }
        Ui::panel_end();
        Ui::layout_pop();

        // Column 2: Events selection
        Ui::layout_push(Vec3::new(0.05, -0.04, 0.0), Vec2::new(0.45, 0.4), false);
        Ui::text("Events:", None, None, None, None, None, None);
        Ui::panel_begin(None);

        let mut poke_enabled = (self.events & InteractorEvent::Poke) != InteractorEvent::empty();
        if Ui::toggle("Poke", &mut poke_enabled, None).is_some() {
            if poke_enabled {
                self.events |= InteractorEvent::Poke;
            } else {
                self.events &= !InteractorEvent::Poke;
            }
            changed = true;
        }

        let mut pinch_enabled = (self.events & InteractorEvent::Pinch) != InteractorEvent::empty();
        if Ui::toggle("Pinch", &mut pinch_enabled, None).is_some() {
            if pinch_enabled {
                self.events |= InteractorEvent::Pinch;
            } else {
                self.events &= !InteractorEvent::Pinch;
            }
            changed = true;
        }

        let mut grip_enabled = (self.events & InteractorEvent::Grip) != InteractorEvent::empty();
        if Ui::toggle("Grip", &mut grip_enabled, None).is_some() {
            if grip_enabled {
                self.events |= InteractorEvent::Grip;
            } else {
                self.events &= !InteractorEvent::Grip;
            }
            changed = true;
        }

        Ui::panel_end();
        Ui::layout_pop();

        // Column 3: Activation Type selection
        Ui::layout_push(Vec3::new(-0.1, -0.04, 0.0), Vec2::new(0.45, 0.4), false);
        Ui::text("Activation:", None, None, None, None, None, None);
        Ui::panel_begin(Some(UiPad::Outside));
        if Ui::radio_img(
            "Position",
            self.activation_type == InteractorActivation::Position,
            &radio_off,
            &radio_on,
            UiBtnLayout::Left,
            None,
        ) && self.activation_type != InteractorActivation::Position
        {
            self.activation_type = InteractorActivation::Position;
            changed = true;
        }
        if Ui::radio_img(
            "State",
            self.activation_type == InteractorActivation::State,
            &radio_off,
            &radio_on,
            UiBtnLayout::Left,
            None,
        ) && self.activation_type != InteractorActivation::State
        {
            self.activation_type = InteractorActivation::State;
            changed = true;
        }
        Ui::panel_end();
        Ui::layout_pop();

        Ui::layout_push(Vec3::new(0.23, -0.22, 0.0), Vec2::new(0.46, 0.7), false);
        // Create a panel for all the controls at the bottom
        Ui::panel_begin(Some(UiPad::Outside));

        // Input Source slider (convert to f32 for slider)
        Ui::text("Min Distance::", None, None, None, None, None, None);
        if Ui::hslider("min_distance", &mut self.min_distance, -1.0, 3.0, None, Some(0.2), None, None).is_some()
            && let Some(interactor) = &self.test_interactor
        {
            interactor.min_distance(self.min_distance);
        }

        // Capsule Radius slider
        Ui::text("Capsule Radius:", None, None, None, None, None, None);
        if Ui::hslider("radius", &mut self.capsule_radius, 0.001, 0.1, Some(0.001), Some(0.2), None, None).is_some() {
            changed = true;
        }

        // Secondary Motion Dimensions slider (convert to f32 for slider)
        Ui::text("Secondary Motion Dims:", None, None, None, None, None, None);
        let mut sec_motion_f32 = self.secondary_motion_dimensions as f32;
        if Ui::hslider("dims", &mut sec_motion_f32, 0.0, 3.0, Some(1.0), Some(0.2), None, None).is_some() {
            self.secondary_motion_dimensions = sec_motion_f32 as i32;
            changed = true;
        }

        // Recreate interactor if any parameter changed
        if changed {
            self.recreate_interactor();
        }

        Ui::vspace(0.03);
        Ui::hseparator();
        Ui::vspace(0.03);

        // Show current interactor info
        if self.test_interactor.is_some() {
            Ui::text("Current Interactor:", None, None, None, None, None, None);
            Ui::text(format!("Type: {:?}", self.shape_type), None, None, None, None, None, None);
            Ui::text(format!("Events: {:?}", self.events), None, None, None, None, None, None);
            Ui::text(format!("Activation: {:?}", self.activation_type), None, None, None, None, None, None);
            Ui::text(format!("Min distance: {}", self.min_distance), None, None, None, None, None, None);
            Ui::text(format!("Radius: {:.3}", self.capsule_radius), None, None, None, None, None, None);
            Ui::text(format!("Sec. Motion: {}", self.secondary_motion_dimensions), None, None, None, None, None, None);
        }
        Ui::vspace(0.03);
        Ui::hseparator();
        Ui::panel_end();
        Ui::layout_pop();

        Ui::window_end();
    }

    fn check_controller_touching_text(&self) -> bool {
        use stereokit_rust::system::{Handed, Input};

        // Check both controllers and hands using loops
        for handed in [Handed::Right, Handed::Left] {
            // Check controller
            let controller = Input::controller(handed);
            if controller.tracked.is_active() {
                let controller_pos = controller.pose.position;
                let distance = (controller_pos - self.text_center).magnitude();
                if distance < self.text_touch_distance {
                    return true;
                }
            }

            // Check hand (in case user doesn't use controllers)
            let hand = Input::hand(handed);
            if hand.tracked.is_active() {
                let hand_pos = hand.palm.position;
                let distance = (hand_pos - self.text_center).magnitude();
                if distance < self.text_touch_distance {
                    return true;
                }
            }
        }

        false
    }

    fn draw_3d_scene(&mut self, token: &MainThreadToken) {
        // Draw target sphere with interactive handle
        let (pos, radius, color) = &mut self.target_sphere;

        // Create a unique ID for the sphere handle
        let handle_id = "target_sphere";

        // Create a pose for the sphere handle
        let mut sphere_pose = Pose::new(*pos, Some(Quat::IDENTITY));

        // Create bounds for the sphere handle
        let bounds = Bounds::new(Vec3::ZERO, Vec3::new(*radius * 2.0, *radius * 2.0, *radius * 2.0));

        // Create an interactive handle for the sphere
        if Ui::handle(handle_id, &mut sphere_pose, bounds, false, None, None) {
            // Update sphere position when handle is moved
            *pos = sphere_pose.position;
        }

        // Draw the sphere mesh with the updated transform
        let scale = Vec3::new(*radius * 2.0, *radius * 2.0, *radius * 2.0);
        let transform = Matrix::t_s(*pos, scale);
        Mesh::sphere().draw(token, Material::default(), transform, Some(*color), None);

        // Draw the text using cached style and updated transform
        Text::add_at(
            token,
            &self.text_content, // Use cached text content
            self.text_transform,
            Some(self.text_style),
            None,
            None, // position
            None, // align
            None, // off_x
            None, // off_y
            None, // off_z
        );

        // Check if either controller is touching the text
        let controller_touching = self.check_controller_touching_text();

        // If a controller is touching the text, perform reset
        if controller_touching {
            // Reset functionality: close this specific Interactor1 instance and restart one
            use stereokit_rust::framework::StepperAction;

            // Close this specific Interactor1 stepper using its ID
            SkInfo::send_event(&self.sk_info, StepperAction::remove(&self.id));

            Log::info("Emergency stop button - Closed this Interactor1 instance");
        }
    }

    /// Clean up resources and restore default interactors
    pub fn close(&mut self, triggering: bool) -> bool {
        if !triggering {
            return true;
        }
        // Restore default interactors
        Interaction::set_default_interactors(DefaultInteractors::Default);

        // Clean up interactor
        if let Some(interactor) = self.test_interactor.take() {
            interactor.destroy();
        }

        Log::info("Interactor Demo: Cleaned up and restored default interactors");
        self.shutdown_completed = true;
        self.shutdown_completed
    }
}
