use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Text, TextStyle},
    ui::{Ui, UiConfirm, UiCut, UiNotify},
    util::named_colors::RED,
};

/// Demo showcasing all slider variants: hslider, hslider_f64, hslider_at, hslider_at_f64,
/// vslider, vslider_f64, vslider_at, vslider_at_f64.
#[derive(IStepper)]
pub struct Ui3 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_pose: Pose,

    // hslider values
    h_f32: f32,
    h_f32_step: f32,
    h_f32_pinch: f32,
    h_f32_finalize: f32,
    h_f32_finalized: f32,

    // hslider_at values
    hat_f32: f32,

    // vslider values
    v_f32: f32,
    v_f32_step: f32,
    v_f32_pinch: f32,

    // vslider_at values
    vat_f32: f32,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Ui3 {}

impl Default for Ui3 {
    fn default() -> Self {
        Self {
            id: "Ui3".to_string(),
            sk_info: None,
            window_pose: Pose::new(Vec3::new(0.0, 1.4, -0.6), Some(Quat::look_dir(-Vec3::FORWARD))),

            h_f32: 0.5,
            h_f32_step: 0.0,
            h_f32_pinch: 0.5,
            h_f32_finalize: 0.5,
            h_f32_finalized: 0.5,

            hat_f32: 0.5,

            v_f32: 0.5,
            v_f32_step: 0.0,
            v_f32_pinch: 0.5,

            vat_f32: 0.5,

            text: "Ui3".to_string(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Ui3 {
    fn start(&mut self) -> bool {
        true
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, token: &MainThreadToken) {
        // 50 cm wide window: right ~23 cm for vsliders, left remainder for hsliders
        Ui::window_begin("Sliders", &mut self.window_pose, Some(Vec2::new(50.0, 0.0) * CM), None, None);

        // ── Right panel: vertical sliders ────────────────────────────────
        Ui::layout_push_cut(UiCut::Right, 23.3 * CM, false);

        Ui::label(format!("vslider: {:.2}", self.v_f32)).draw();
        Ui::same_line();
        Ui::vslider("v_f32", &mut self.v_f32, 0.0, 1.0).space(11.7 * CM).interact();

        Ui::label(format!("step=0.1: {:.2}", self.v_f32_step)).draw();
        Ui::same_line();
        Ui::vslider("v_f32_step", &mut self.v_f32_step, 0.0, 1.0).step(0.1).space(11.7 * CM).interact();

        Ui::label(format!("Pinch: {:.2}", self.v_f32_pinch)).draw();
        Ui::same_line();
        Ui::vslider("v_f32_pinch", &mut self.v_f32_pinch, 0.0, 1.0)
            .space(11.7 * CM)
            .confirm_method(UiConfirm::Pinch)
            .interact();

        Ui::hseparator();

        Ui::label(format!("vslider_at:{:.2}", self.vat_f32)).draw();
        Ui::same_line();
        let at = Ui::get_layout_at();
        Ui::vslider("vat_f32", &mut self.vat_f32, 0.0, 1.0).at(at, Vec2::new(2.5 * CM, 8.3 * CM)).interact();

        Ui::layout_reserve(Vec2::new(0.0, 8.3 * CM), false, 0.0);

        Ui::layout_pop(); // end right panel

        // ── Left panel: horizontal sliders ───────────────────────────────
        Ui::label(format!("hslider: {:.2}", self.h_f32)).draw();
        Ui::hslider("h_f32", &mut self.h_f32, 0.0, 1.0).interact();

        Ui::label(format!("hslider step=0.1: {:.2}", self.h_f32_step)).draw();
        Ui::hslider("h_f32_step", &mut self.h_f32_step, 0.0, 1.0).step(0.1).interact();

        Ui::label(format!("hslider Pinch: {:.2}", self.h_f32_pinch)).draw();
        Ui::hslider("h_f32_pinch", &mut self.h_f32_pinch, 0.0, 1.0)
            .confirm_method(UiConfirm::Pinch)
            .interact();

        Ui::label(format!("hslider Finalize: {:.2}", self.h_f32_finalized)).draw();
        if let Some(finalized) = Ui::hslider("h_f32_finalize", &mut self.h_f32_finalize, 0.0, 1.0)
            .notify_on(UiNotify::Finalize)
            .interact()
        {
            self.h_f32_finalized = finalized;
        }

        Ui::hseparator();

        Ui::label(format!("hslider_at: {:.2}", self.hat_f32)).draw();
        let at = Ui::get_layout_at();
        Ui::hslider("hat_f32", &mut self.hat_f32, 0.0, 1.0)
            .at(at, Vec2::new(20.0 * CM, 1.5 * CM))
            .interact();
        Ui::layout_reserve(Vec2::new(0.0, 1.5 * CM), false, 0.0);

        Ui::layout_reserve(Vec2::new(0.0, 1.5 * CM), false, 0.0);

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
