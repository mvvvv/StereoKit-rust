use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Handed, Input, Text, TextStyle},
    ui::{IdHashT, Ui, UiCut, UiSliderData, UiVisual},
    util::{
        Color128,
        named_colors::{RED, WHITE},
    },
};

/// Copycat of the example https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tests/TestCustomButton.cs

#[derive(IStepper)]
pub struct Input1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    pub id_slider_left: String,
    id_slider_left_hash: IdHashT,
    pub id_slider_right: String,
    id_slider_right_hash: IdHashT,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Input1 {}

impl Default for Input1 {
    fn default() -> Self {
        let id_slider_left = "left sticker".into();
        let id_slider_right = "right sticker".into();
        Self {
            id: "Input1".to_string(),
            sk_info: None,
            window_demo_pose: Ui::popup_pose([0.0, 0.0, -0.1]),
            demo_win_width: 30.0 * CM,
            id_slider_left_hash: Ui::stack_hash(&id_slider_left),
            id_slider_left,
            id_slider_right_hash: Ui::stack_hash(&id_slider_right),
            id_slider_right,

            text: "Input1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r(
                (Vec3::NEG_Z * 2.5) + Vec3::Y, //
                Quat::from_angles(0.0, 180.0, 0.0),
            ),
        }
    }
}

impl Input1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.id_slider_left_hash = Ui::stack_hash(&self.id_slider_left);
        self.id_slider_right_hash = Ui::stack_hash(&self.id_slider_right);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin("Input", &mut self.window_demo_pose, Some(Vec2::new(self.demo_win_width, 0.4)), None, None);

        // Left
        Ui::layout_push_cut(UiCut::Left, 0.14, true);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
        Ui::label("Left", None, false);
        let move_ctrler = Input::controller(Handed::Left);
        let slider_pt = move_ctrler.stick * Vec2 { x: 1.0, y: -1.0 };
        let id_slider_hash = self.id_slider_left_hash;
        let stick = move_ctrler.stick_click;
        let stick_color: Color128 = if stick.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(stick_color.to_gamma());
        draw_slider(slider_pt, id_slider_hash);
        Ui::pop_tint();

        // Button Y
        Ui::hspace(0.07);
        let y = move_ctrler.x2;
        let y_color: Color128 = if y.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(y_color.to_gamma());
        Ui::button_at("Y", [0.06, -0.22, 0.005], [0.03, 0.03]);
        Ui::pop_tint();

        // Button X
        Ui::hspace(0.02);
        let x = move_ctrler.x1;
        let x_color: Color128 = if x.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(x_color.to_gamma());
        Ui::button_at("X", [0.10, -0.25, 0.005], [0.03, 0.03]);
        Ui::pop_tint();

        // Trigger
        Ui::vspace(0.07);
        Ui::hspace(0.01);
        let trigger = move_ctrler.trigger;
        let trigger_color: Color128 = if trigger > 0.0 { RED.into() } else { WHITE.into() };
        let trigger_text = format!("L_Trigger: {:.2}", trigger);
        Ui::push_tint(trigger_color.to_gamma());
        Ui::button_at(trigger_text, [0.12, -0.295, 0.005], [0.10, 0.03]);
        Ui::pop_tint();

        // Grip
        Ui::hspace(0.1);
        let grip = move_ctrler.grip;
        let grip_color: Color128 = if grip > 0.0 { RED.into() } else { WHITE.into() };
        Ui::push_tint(grip_color.to_gamma());
        let grip_text = format!("L_Grip: {:.2}", grip);
        Ui::button_at(grip_text, [0.10, -0.34, 0.005], [0.09, 0.03]);
        Ui::pop_tint();

        Ui::layout_pop();

        // Right
        Ui::layout_push_cut(UiCut::Right, 0.14, true);
        Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
        Ui::label("Right", None, false);
        let move_ctrler = Input::controller(Handed::Right);
        let slider_pt = move_ctrler.stick * Vec2 { x: 1.0, y: -1.0 };
        let id_slider_hash = self.id_slider_right_hash;
        let stick = move_ctrler.stick_click;
        let stick_color: Color128 = if stick.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(stick_color.to_gamma());
        draw_slider(slider_pt, id_slider_hash);
        Ui::pop_tint();

        // Button B
        Ui::hspace(0.07);
        let b = move_ctrler.x2;
        let b_color: Color128 = if b.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(b_color.to_gamma());
        Ui::button_at("B", [-0.03, -0.22, 0.005], [0.03, 0.03]);
        Ui::pop_tint();

        // Button A
        Ui::hspace(0.02);
        let a = move_ctrler.x1;
        let a_color: Color128 = if a.is_active() { RED.into() } else { WHITE.into() };
        Ui::push_tint(a_color.to_gamma());
        Ui::button_at("A", [-0.07, -0.25, 0.005], [0.03, 0.03]);
        Ui::pop_tint();

        // Trigger
        Ui::vspace(0.07);
        Ui::hspace(0.01);
        let trigger = move_ctrler.trigger;
        let trigger_color: Color128 = if trigger > 0.0 { RED.into() } else { WHITE.into() };
        let trigger_text = format!("R_Trigger: {:.2}", trigger);
        Ui::push_tint(trigger_color.to_gamma());
        Ui::button_at(trigger_text, [-0.02, -0.295, 0.005], [0.10, 0.03]);
        Ui::pop_tint();

        // Grip
        Ui::hspace(0.1);
        let grip = move_ctrler.grip;
        let grip_color: Color128 = if grip > 0.0 { RED.into() } else { WHITE.into() };
        Ui::push_tint(grip_color.to_gamma());
        let grip_text = format!("R_Grip: {:.2}", grip);
        Ui::button_at(grip_text, [-0.01, -0.34, 0.005], [0.09, 0.03]);
        Ui::pop_tint();

        Ui::layout_pop();

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}

fn draw_slider(mut slider_pt: Vec2, id_slider_hash: u64) {
    let size = Vec2::ONE * Ui::get_layout_remaining().x;
    let depth = Ui::get_settings().depth;
    let bounds = Ui::layout_reserve(size, false, depth);
    let btn_height = Ui::get_line_height() * 0.25;
    let btn_size = Vec3::new(btn_height, btn_height, depth);

    let mut slider = UiSliderData::default();
    let tlb = bounds.tlb();
    Ui::slider_behavior(
        tlb,
        bounds.dimensions.xy(),
        id_slider_hash,
        &mut slider_pt,
        Vec2::ONE * -1.0,
        Vec2::ONE,
        Vec2::ZERO,
        btn_size.xy(),
        None,
        &mut slider,
    );
    let focus = Ui::get_anim_focus(id_slider_hash, slider.focus_state, slider.active_state);
    Ui::draw_element(
        UiVisual::SliderLine,
        None,
        tlb,
        Vec3::new(bounds.dimensions.x, bounds.dimensions.y, depth * 0.1),
        if slider.focus_state.is_active() { 0.5 } else { 0.0 },
    );
    Ui::draw_element(UiVisual::SliderPush, None, slider.button_center.xy0() + btn_size.xy0(), btn_size, focus);

    Ui::label(format!("{:>5.2} * {:>5.2}", slider_pt.x, slider_pt.y), None, true);
}
