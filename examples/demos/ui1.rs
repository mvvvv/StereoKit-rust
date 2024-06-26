use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    event_loop::{IStepper, StepperId},
    font::Font,
    material::Material,
    maths::{units::CM, Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    sk::{MainThreadToken, SkInfo},
    system::{BtnState, Text, TextAlign, TextStyle},
    ui::{Ui, UiVisual},
    util::{named_colors::RED, Color128},
};

/// Copycat of the example https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tests/TestCustomButton.cs
pub struct Ui1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    pub ui_material: Material,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Ui1 {}

impl Default for Ui1 {
    fn default() -> Self {
        Self {
            id: "Ui1".to_string(),
            sk_info: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 36.0 * CM,
            ui_material: Material::ui().copy(),
            text: "Ui1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl IStepper for Ui1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl Ui1 {
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin(
            "Ui elements",
            &mut self.window_demo_pose,
            Some(Vec2::new(self.demo_win_width, 0.0)),
            None,
            None,
        );

        self.custom_button_mesh(token, "Custom Button Mesh");
        self.custom_button_element(token, "Custom Button Element");
        Ui::button("Standard Button", None);

        Ui::push_enabled(false, None);
        self.custom_button_mesh(token, "Custom Button Disabled");
        Ui::pop_enabled();

        Ui::push_tint(Color128::hsv(0.0, 0.2, 0.7, 1.0));
        self.custom_button_element(token, "Custom Button Tinted");
        Ui::pop_tint();
        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }

    pub fn custom_button_mesh(&mut self, token: &MainThreadToken, text: &str) -> bool {
        let id = Ui::stack_hash(text);
        let size = Text::size(text, Some(Ui::get_text_style()), None) * 2.0;
        let mut layout = Ui::layout_reserve(size, false, 0.0);
        let mut out_finger_offset: f32 = 0.0;
        let mut out_button_state: BtnState = BtnState::empty();
        let mut out_focus_state = BtnState::empty();
        let mut out_opt_hand: i32 = 0;
        Ui::button_behavior(
            layout.tlc(),
            size,
            text,
            &mut out_finger_offset,
            &mut out_button_state,
            &mut out_focus_state,
            Some(&mut out_opt_hand),
        );
        layout.center.z -= out_finger_offset / 2.0;
        layout.dimensions.z = out_finger_offset;
        Mesh::cube().draw(
            token,
            &self.ui_material,
            Matrix::ts(layout.center, layout.dimensions),
            Some(Ui::get_element_color(UiVisual::Button, Ui::get_anim_focus(id, out_focus_state, out_button_state))),
            None,
        );
        Text::add_at(
            token,
            text,
            Matrix::t(Vec3::new(layout.center.x, layout.center.y, -(out_finger_offset + 0.002))),
            Some(Ui::get_text_style()),
            None,
            Some(TextAlign::Center),
            None,
            None,
            None,
            None,
        );
        out_button_state.is_just_inactive()
    }

    pub fn custom_button_element(&mut self, token: &MainThreadToken, text: &str) -> bool {
        let id = Ui::stack_hash(text);
        let size = Text::size(text, Some(Ui::get_text_style()), None) * 2.0;
        let mut layout = Ui::layout_reserve(size, false, 0.0);
        let mut out_finger_offset: f32 = 0.0;
        let mut out_button_state: BtnState = BtnState::empty();
        let mut out_focus_state = BtnState::empty();
        let mut out_opt_hand: i32 = 0;
        Ui::button_behavior(
            layout.tlc(),
            size,
            text,
            &mut out_finger_offset,
            &mut out_button_state,
            &mut out_focus_state,
            Some(&mut out_opt_hand),
        );
        layout.center.z -= out_finger_offset / 2.0;
        layout.dimensions.z = out_finger_offset;
        Ui::draw_element(
            UiVisual::Button,
            None,
            layout.tlb(),
            layout.dimensions,
            Ui::get_anim_focus(id, out_focus_state, out_button_state),
        );
        Text::add_at(
            token,
            text,
            Matrix::t(Vec3::new(layout.center.x, layout.center.y, -(out_finger_offset + 0.002))),
            Some(Ui::get_text_style()),
            None,
            Some(TextAlign::Center),
            None,
            None,
            None,
            None,
        );

        out_button_state.is_just_inactive()
    }
}
