use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec3},
    render_list::RenderList,
    sk::{IStepper, SkInfo, StepperAction, StepperId},
    system::{Text, TextStyle},
    ui::Ui,
    util::named_colors::RED,
};

pub struct RenderList1 {
    id: StepperId,
    sk: Option<Rc<RefCell<SkInfo>>>,
    pub window_pose: Pose,
    primary: RenderList,
    clear_primary: bool,
    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

impl Default for RenderList1 {
    fn default() -> Self {
        Self {
            id: "RenderList1".to_string(),
            sk: None,
            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            primary: RenderList::primary(),
            clear_primary: false,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Stepper A".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl IStepper for RenderList1 {
    fn initialize(&mut self, id: StepperId, sk: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk = Some(sk);
        true
    }

    fn step(&mut self, _event_report: &[StepperAction]) {
        self.draw()
    }
}

impl RenderList1 {
    fn draw(&mut self) {
        if self.clear_primary {
            self.primary.clear()
        };

        Ui::window_begin("Render Lists", &mut self.window_pose, None, None, None);
        Ui::label(format!("Render items: {}/{}", self.primary.get_count(), self.primary.get_prev_count()), None, true);
        if let Some(value) = Ui::toggle("Clear", self.clear_primary, None) {
            self.clear_primary = value
        };
        Ui::window_end();

        Text::add_at(&self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
