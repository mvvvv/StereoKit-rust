use stereokit_rust::{
    maths::{Pose, Vec2},
    prelude::*,
    system::{Input, InputHaptic},
    ui::{Ui, UiCut},
    util::{Color128, named_colors::RED},
};

/// A basic Stepper to test haptic feedback. We must ensure the StereoKit code stays in the main thread.
/// Default may be called in another thread.
#[derive(IStepper)]
pub struct Haptic1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_demo_pose: Pose,
}

unsafe impl Send for Haptic1 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for Haptic1 {
    fn default() -> Self {
        Self { id: "Haptic1".to_string(), sk_info: None, window_demo_pose: Ui::popup_pose([0.0, 0.0, -0.1]) }
    }
}

/// All the code here run in the main thread
impl Haptic1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window_begin("Haptic Demos", &mut self.window_demo_pose, Some(Vec2::new(0.40, 0.25)), None, None);

        let controllers = [
            (UiCut::Left, "Left Controller", "Left", InputHaptic::LController),
            (UiCut::Right, "Right Controller", "Right", InputHaptic::RController),
        ];

        for (cut, label, id, haptic) in controllers {
            Ui::layout_push_cut(cut, 0.18, true);
            Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
            Ui::label(label, None, false);
            Ui::push_id(id);
            Ui::hseparator();
            if Ui::button("Pulse", None) {
                Input::haptic_pulse(haptic, 1.0, 0.5, 0.5);
            }
            if Ui::button("Waveform", None) {
                let sample_rate = 60.0;
                let duration_seconds = 1.0;
                let num_samples = (sample_rate * duration_seconds) as usize;
                let mut samples = vec![0.0; num_samples];
                for (i, sample) in samples.iter_mut().enumerate() {
                    let t = i as f32 / sample_rate;
                    *sample = (t * 440.0 * std::f32::consts::TAU).sin()
                        * Vec2::dot([1.0, 0.5].into(), [(t * 0.5).sin(), (t * 0.25).sin()].into());
                }
                Input::haptic_waveform(haptic, &samples, sample_rate, false);
            }
            if Ui::button("Curve", None) {
                let sample_rate = 60.0;
                let duration_seconds = 1.0;
                let num_samples = (sample_rate * duration_seconds) as usize;
                let mut amplitudes = vec![0.0; num_samples];
                for (i, amplitude) in amplitudes.iter_mut().enumerate() {
                    let t = i as f32 / sample_rate;
                    *amplitude = (t * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                }
                Input::haptic_curve(haptic, &amplitudes, sample_rate);
            }

            let stop_width = 0.08;
            let space = Ui::get_layout_remaining().x - stop_width;
            if space > 0.0 {
                Ui::hspace(space - 0.01);
            }
            let red: Color128 = RED.into();
            Ui::push_tint(red.to_gamma());
            if Ui::button("Stop", Some(Vec2::new(stop_width, 0.0))) {
                Input::haptic_stop(haptic);
            }
            Ui::pop_tint();
            Ui::pop_id();
            Ui::layout_pop();
        }

        Ui::window_end();
    }
}
