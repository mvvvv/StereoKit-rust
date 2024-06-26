use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    event_loop::{IStepper, StepperId},
    font::Font,
    maths::{units::CM, Matrix, Pose, Quat, Vec2, Vec3},
    sk::{MainThreadToken, SkInfo},
    sprite::Sprite,
    system::{Text, TextAlign, TextFit, TextStyle},
    ui::{Ui, UiBtnLayout, UiScroll},
    util::named_colors::{RED, WHITE},
};

pub const TEXTY: &str = r#"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"#;
pub const TEXTO: &str = r#"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
abcdefghijklmnopqrstuvwxyz=)àç_è-('"é&
αβγδϵζηθικλμνξοπρστυϕχψω
THE END"#;

pub struct Text2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform: Matrix,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    font_selected: u8,
    text_style_test: TextStyle,
    text_size: f32,
    scroll1: Vec2,
    scroll2: Vec2,
    scroll3: Vec2,
    scroll4: Vec2,
    text: String,
    text_style: TextStyle,
    radio_on: Sprite,
    radio_off: Sprite,
}

unsafe impl Send for Text2 {}

impl Default for Text2 {
    fn default() -> Self {
        let text_size = 0.02;
        Self {
            id: "Text2".to_string(),
            sk_info: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 40.0 * CM,
            font_selected: 1,
            text_style_test: Text::make_style(Font::default(), text_size, WHITE),
            text_size,
            scroll1: Vec2::ZERO,
            scroll2: Vec2::ZERO,
            scroll3: Vec2::ZERO,
            scroll4: Vec2::ZERO,
            text: "Text2".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            radio_on: Sprite::radio_on(),
            radio_off: Sprite::radio_off(),
        }
    }
}

impl IStepper for Text2 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl Text2 {
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin(
            "Text options",
            &mut self.window_demo_pose,
            Some(Vec2::new(self.demo_win_width, 0.0)),
            None,
            None,
        );
        //Bug #1020 to solve
        Ui::push_enabled(false, None);
        if Ui::radio_img(
            "Default Font",
            self.font_selected == 1,
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            let font = Font::default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 1;
        }
        Ui::same_line();
        if Ui::radio_img("Font 1", self.font_selected == 2, &self.radio_off, &self.radio_on, UiBtnLayout::Left, None) {
            let font = Font::from_family("Courier").unwrap_or_default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 2;
        }
        Ui::same_line();
        if Ui::radio_img("Font 2", self.font_selected == 3, &self.radio_off, &self.radio_on, UiBtnLayout::Left, None) {
            let font = Font::from_family("Arial").unwrap_or_default();
            self.text_style_test = Text::make_style(font, self.text_size, WHITE);
            self.font_selected = 3;
        }
        Ui::pop_enabled();
        Ui::push_text_style(self.text_style_test);
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("A");
        Ui::text(TEXTY, None, None, None, None, None, None);
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("B");
        Ui::text(
            TEXTY,
            Some(&mut self.scroll1),
            Some(UiScroll::Horizontal),
            Some(0.08),
            Some(0.36),
            None,
            Some(TextFit::Overflow),
        );
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("C");
        Ui::text(
            TEXTY,
            Some(&mut self.scroll2),
            Some(UiScroll::Horizontal),
            Some(0.08),
            Some(0.36),
            None,
            Some(TextFit::Squeeze),
        );
        Ui::pop_id();
        Ui::next_line();
        Ui::hseparator();
        Ui::push_id("D");
        Ui::text(
            TEXTO,
            Some(&mut self.scroll3),
            Some(UiScroll::Both),
            Some(0.1),
            Some(0.15),
            None,
            Some(TextFit::Overflow),
        );
        Ui::pop_id();
        Ui::same_line();
        Ui::push_id("E");
        Ui::text_at(
            TEXTO,
            Some(&mut self.scroll4),
            Some(UiScroll::Both),
            TextAlign::TopLeft,
            TextFit::Overflow,
            Vec3::new(0.016, -0.40, -0.03),
            Vec2::new(0.18, 0.1),
        );
        Ui::pop_id();
        Ui::pop_text_style();

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
