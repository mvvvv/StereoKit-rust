use std::{cell::RefCell, rc::Rc};

use stereokit_macros::IStepper;
use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{units::CM, Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    system::{BtnState, Text, TextAlign, TextStyle},
    ui::{IdHashT, Ui, UiColor, UiCorner, UiLathePt, UiSliderData, UiVisual},
    util::{
        named_colors::{CYAN, DARK_BLUE, ORCHID, RED, YELLOW},
        Color128, Color32, Time,
    },
};

const LATHE_BUTTON: [UiLathePt; 6] = [
    UiLathePt { pt: Vec2::new(0.0, -0.5), normal: Vec2::new(0.0, 1.0), color: CYAN, connect_next: 1, flip_face: 0 },
    UiLathePt { pt: Vec2::new(0.95, -0.5), normal: Vec2::new(0.0, 1.0), color: CYAN, connect_next: 1, flip_face: 0 },
    UiLathePt { pt: Vec2::new(1.0, -0.45), normal: Vec2::new(1.0, 0.0), color: CYAN, connect_next: 1, flip_face: 0 },
    UiLathePt { pt: Vec2::new(1.0, -0.1), normal: Vec2::new(1.0, 0.0), color: CYAN, connect_next: 0, flip_face: 0 },
    UiLathePt {
        pt: Vec2::new(1.2, 0.49),
        normal: Vec2::new(0.0, 1.0),
        color: DARK_BLUE,
        connect_next: 1,
        flip_face: 1,
    },
    UiLathePt {
        pt: Vec2::new(0.0, 0.49),
        normal: Vec2::new(0.0, 1.0),
        color: Color32::new(0, 0, 139, 200),
        connect_next: 1,
        flip_face: 1,
    },
];

/// Copycat of the example https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tests/TestCustomButton.cs

#[derive(IStepper)]
pub struct Ui1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    pub ui_material: Material,
    pub id_slider: String,
    id_slider_hash: IdHashT,
    slider_pt: Vec2,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Ui1 {}

impl Default for Ui1 {
    fn default() -> Self {
        let id_slider = "touch panel".into();
        Self {
            id: "Ui1".to_string(),
            sk_info: None,
            transform: Matrix::tr(
                &((Vec3::NEG_Z * 2.5) + Vec3::Y), //
                &Quat::from_angles(0.0, 180.0, 0.0),
            ),
            window_demo_pose: Pose::new(
                Vec3::new(0.0, 1.5, -1.3), //
                Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0))),
            ),
            demo_win_width: 36.0 * CM,
            ui_material: Material::ui().copy(),
            id_slider_hash: Ui::stack_hash(&id_slider),
            id_slider,
            slider_pt: Vec2::ONE * 0.5,
            text: "Ui1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl Ui1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.id_slider_hash = Ui::stack_hash(&self.id_slider);
        //create extra slots
        Ui::set_theme_color(UiColor::ExtraSlot01, None, ORCHID);
        Ui::set_theme_color(UiColor::ExtraSlot02, None, YELLOW);
        Ui::set_element_color(UiVisual::ExtraSlot01, UiColor::ExtraSlot01);
        Ui::set_element_color(UiVisual::ExtraSlot02, UiColor::ExtraSlot02);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        let corner_radius = 0.005 * Time::get_totalf().sin().abs();
        if let Ok(mesh) = Ui::gen_quadrant_mesh(
            UiCorner::TopLeft & UiCorner::BottomRight,
            corner_radius,
            8,
            true,
            true,
            &LATHE_BUTTON,
        ) {
            Ui::set_element_visual(UiVisual::ExtraSlot03, mesh, None, None);
        }

        Ui::window_begin(
            "Ui elements",
            &mut self.window_demo_pose,
            Some(Vec2::new(self.demo_win_width, 0.0)),
            None,
            None,
        );

        self.custom_button_mesh(token, "Custom Button Mesh", UiVisual::ExtraSlot02);
        self.custom_button_element(token, "Custom Button Element");
        Ui::button("Standard Button", None);

        Ui::push_enabled(false, None);
        self.custom_button_mesh(token, "Custom Button Mesh Disabled", UiVisual::ExtraSlot01);
        Ui::pop_enabled();

        Ui::push_tint(Color128::hsv(0.0, 0.2, 0.7, 1.0));
        self.custom_button_element(token, "Custom Button Element Tinted");
        Ui::pop_tint();

        Ui::hseparator();

        //Slider behavior

        let size = Vec2::ONE * Ui::get_layout_remaining().x;
        self.ui_touch_panel(size);
        Ui::label(format!("{}x{}", self.slider_pt.x * 100.0, self.slider_pt.y * 100.0), None, true);

        Ui::hseparator();

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }

    pub fn custom_button_mesh(&mut self, token: &MainThreadToken, text: &str, slot: UiVisual) -> bool {
        let id = Ui::stack_hash(text);
        let size = Text::size_layout(text, Some(Ui::get_text_style()), None) * 1.7;
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
            Some(Ui::get_element_color(slot, Ui::get_anim_focus(id, out_focus_state, out_button_state))),
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
        let size = Text::size_layout(text, Some(Ui::get_text_style()), None) * 1.7;
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
            UiVisual::ExtraSlot03,
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

    /// Copycat from https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Docs/DocSliderBehavior.cs
    pub fn ui_touch_panel(&mut self, size: Vec2) -> bool {
        let depth = Ui::get_settings().depth;
        let bounds = Ui::layout_reserve(size, false, depth);
        let btn_height = Ui::get_line_height() * 0.75;
        let btn_size = Vec3::new(btn_height, btn_height, depth);

        let prev = self.slider_pt;
        let mut slider = UiSliderData::default();
        let tlb = bounds.tlb();
        Ui::slider_behavior(
            tlb,
            bounds.dimensions.xy(),
            self.id_slider_hash,
            &mut self.slider_pt,
            Vec2::ZERO,
            Vec2::ONE,
            Vec2::ZERO,
            btn_size.xy(),
            None,
            &mut slider,
        );
        let focus = Ui::get_anim_focus(self.id_slider_hash, slider.focus_state, slider.active_state);
        Ui::draw_element(
            UiVisual::SliderLine,
            None,
            tlb,
            Vec3::new(bounds.dimensions.x, bounds.dimensions.y, depth * 0.1),
            if slider.focus_state.is_active() { 0.5 } else { 0.0 },
        );
        Ui::draw_element(
            UiVisual::SliderPush,
            None,
            slider.button_center.xy0() + btn_size.xy0() / 2.0,
            btn_size,
            focus,
        );
        prev.x != self.slider_pt.x || prev.y != self.slider_pt.y
    }
}
