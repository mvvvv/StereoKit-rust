use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    sprite::Sprite,
    system::{Align, Text, TextBuilder, TextFit, TextStyle},
    ui::{Ui, UiBtnLayout, UiScroll},
    util::named_colors::{RED, WHITE},
};

pub const TEXTY: &str = r#"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"#;
pub const TEXTO: &str = r#"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
abcdefghijklmnopqrstuvwxyz=)àç_è-('"é&
αβγδϵζηθικλμνξοπρστυϕχψω
THE END"#;

/// The Text2 stepper
#[derive(IStepper)]
pub struct Text2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    font_selected: u8,
    text_style_test: TextStyle,
    text_size: f32,
    scroll1: Vec2,
    scroll2: Vec2,
    scroll3: Vec2,
    scroll4: Vec2,
    radio_on: Sprite,
    radio_off: Sprite,

    text: String,
    text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Text2 {}

impl Default for Text2 {
    fn default() -> Self {
        let text_size = 0.02;
        Self {
            id: "Text2".to_string(),
            sk_info: None,

            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 40.0 * CM,
            font_selected: 1,
            text_style_test: Text::make_style(Font::default(), text_size, WHITE),
            text_size,
            scroll1: Vec2::ZERO,
            scroll2: Vec2::ZERO,
            scroll3: Vec2::ZERO,
            scroll4: Vec2::ZERO,
            radio_on: Sprite::radio_on(),
            radio_off: Sprite::radio_off(),

            text: "Text2".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Text2 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::draw, here you can draw the UI and the scene
    fn draw(&mut self, _token: &MainThreadToken) {
        Ui::window("Text options")
            .pose(&mut self.window_demo_pose)
            .size(Vec2::new(self.demo_win_width, 0.0))
            .begin();
        //Bug #1020 to solve
        Ui::push_enabled(cfg!(windows), None);
        if Ui::radio("Default Font", self.font_selected == 1)
            .images(&self.radio_off, &self.radio_on)
            .image_layout(UiBtnLayout::Left)
            .press()
        {
            let font = Font::default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 1;
        }
        Ui::same_line();
        if Ui::radio("Font 1", self.font_selected == 2)
            .images(&self.radio_off, &self.radio_on)
            .image_layout(UiBtnLayout::Left)
            .press()
        {
            let font = Font::from_family("Arial, Helvetica, Verdana, Geneva, Tahoma, sans-serif;").unwrap_or_default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 2;
        }
        Ui::same_line();
        if Ui::radio("Font 2", self.font_selected == 3)
            .images(&self.radio_off, &self.radio_on)
            .image_layout(UiBtnLayout::Left)
            .press()
        {
            let font = Font::from_family("'Times New Roman', Times, serif;").unwrap_or_default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 3;
        }
        Ui::pop_enabled();
        Ui::push_text_style(self.text_style_test);
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("A");
        Ui::text(TEXTY).draw();
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("B");
        Ui::text(TEXTY)
            .scroll(&mut self.scroll1, UiScroll::Horizontal)
            .size([0.36, 0.08])
            .fit(TextFit::Overflow)
            .draw();
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("C");
        Ui::text(TEXTY)
            .scroll(&mut self.scroll2, UiScroll::Horizontal)
            .size([0.36, 0.08])
            .fit(TextFit::Squeeze)
            .draw();
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("D");
        Ui::text(TEXTO)
            .scroll(&mut self.scroll3, UiScroll::Both)
            .size([0.15, 0.1])
            .fit(TextFit::Overflow)
            .draw();
        Ui::pop_id();
        Ui::same_line();
        Ui::push_id("E");
        Ui::text(TEXTO)
            .at(Vec3::new(0.016, -0.40, -0.03), Vec2::new(0.18, 0.1))
            .scroll(&mut self.scroll4, UiScroll::Both)
            .text_align(Align::TopLeft)
            .fit(TextFit::Overflow)
            .draw();
        Ui::pop_id();
        Ui::pop_text_style();

        Ui::window_end();

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }
}
