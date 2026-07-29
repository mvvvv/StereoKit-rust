use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Input, InputHaptic, InputHapticCaps, Text, TextBuilder, TextStyle},
    ui::{Ui, UiBtnLayout, UiCut},
    util::{
        Color128,
        named_colors::{GREEN, RED},
    },
};

/// A basic Stepper to test haptic feedback. We must ensure the StereoKit code stays in the main thread.
/// Default may be called in another thread.
#[derive(IStepper)]
pub struct Haptic1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_demo_pose: Pose,
    sprite_on: Option<Sprite>,
    sprite_off: Option<Sprite>,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Haptic1 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for Haptic1 {
    fn default() -> Self {
        Self {
            id: "Haptic1".to_string(),
            sk_info: None,

            window_demo_pose: Ui::popup_pose([0.0, 0.0, -0.1]),
            sprite_on: None,
            sprite_off: None,

            text: "Haptic1".to_owned(), // Default text.
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r(
                (Vec3::NEG_Z * 2.5) + Vec3::Y, //
                Quat::from_angles(0.0, 180.0, 0.0),
            ),
        }
    }
}

/// All the code here run in the main thread
impl Haptic1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.sprite_on = Some(Sprite::toggle_on());
        self.sprite_off = Some(Sprite::close());
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn button_indicator(&self, text: &str, has_cap: bool) -> bool {
        let color_tint: Color128 = if has_cap { GREEN.into() } else { RED.into() };
        let sprite = if has_cap { self.sprite_on.as_ref().unwrap() } else { self.sprite_off.as_ref().unwrap() };
        Ui::button(text)
            .image(sprite)
            .image_layout(UiBtnLayout::Left)
            .image_tint(color_tint.to_gamma())
            .press()
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("Haptic Demos").pose(&mut self.window_demo_pose).size(Vec2::new(0.40, 0.25)).begin();

        let controllers = [
            (UiCut::Left, "Left Controller", "Left", InputHaptic::LController),
            (UiCut::Right, "Right Controller", "Right", InputHaptic::RController),
        ];

        for (cut, label, id, haptic) in controllers {
            Ui::layout_push_cut(cut, 0.18, true);
            Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
            Ui::label(label).draw();

            Ui::push_id(id);
            Ui::hseparator();

            let caps = Input::haptic_caps(haptic);

            if self.button_indicator("Pulse", caps.contains(InputHapticCaps::Pulse)) {
                Input::haptic_pulse(haptic, 1.0, 0.5, 0.5);
            }
            if self.button_indicator("Waveform", caps.contains(InputHapticCaps::Waveform)) {
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
            if self.button_indicator("Curve", caps.contains(InputHapticCaps::Curve)) {
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
            if Ui::button("Stop").size(Vec2::new(stop_width, 0.0)).press() {
                Input::haptic_stop(haptic);
            }
            Ui::pop_tint();
            Ui::pop_id();
            Ui::layout_pop();
        }

        Ui::window_end();

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }
}
