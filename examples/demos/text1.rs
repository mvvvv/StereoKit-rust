use std::mem::transmute;
use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{Input, Key, Log, Text, TextContext, TextStyle},
    tools::os_api::show_soft_input,
    ui::{Ui, UiBtnLayout},
    util::{
        Platform,
        named_colors::{RED, WHITE},
    },
};

pub const FR_KEY_TEXT: &str = r#"²|&|é|"|'|(|\-|è|_|ç|à|)|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|a|z|e|r|t|y|u|i|o|p|^|$|[|]|\|
Entrée-\n-13-4|q|s|d|f|g|h|j|k|l|m|ù|*|#|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|w|x|c|v|b|n|,|;|:|!|`|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

pub const FR_KEY_TEXT_SHIFT: &str = r#"@|1|2|3|4|5|6|7|8|9|0|°|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|A|Z|E|R|T|Y|U|I|O|P|¨|£|Ê|É|È
Entrée-\n-13-4|Q|S|D|F|G|H|J|K|L|M|%|µ|Ç|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|W|X|C|V|B|N|?|.|/|§|À|Ô|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

pub const FR_KEY_TEXT_ALT: &str = r#"*|/|~|#|{|[|\||`|\\|^|@|]|}|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|à|â|ä|ç|é|è|ê|ë|î|ï|ô|ö|«|»|¤
Entrée-\n-13-4|ù|û|ü|ÿ|À|Â|Ä|Ç|É|È|Ê|Ë|%|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_1|Î|Ï|Ô|Ö|Ù|Û|Ü|Ÿ|$|£|€|¥|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

#[derive(IStepper)]
pub struct Text1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub transform: Matrix,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    pub android_keyboard: bool,
    pub android_keyboard_ime: bool,
    pub keyboard_layout_fr: bool,
    inst_play: Option<SoundInst>,
    pub show_keyboard: bool,
    pub text_sample: String,
    font_selected: u8,
    text_context: TextContext,
    text_style_test: TextStyle,
    text: String,
    text_style: TextStyle,
    next_value: Sprite,
    radio_on: Sprite,
    radio_off: Sprite,
}

unsafe impl Send for Text1 {}

impl Default for Text1 {
    fn default() -> Self {
        Self {
            id: "Text1".to_string(),
            sk_info: None,

            transform: Matrix::t_r((Vec3::NEG_Z * -2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 80.0 * CM,
            android_keyboard: false,
            android_keyboard_ime: false,
            keyboard_layout_fr: false,
            inst_play: None,
            show_keyboard: false,
            text_sample: String::from("😃‣‣‣‣😃"),
            font_selected: 1,
            text_context: TextContext::Text,
            text_style_test: Text::make_style(Font::default(), 0.05, WHITE),
            text: "Text1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            next_value: Sprite::arrow_right(),
            radio_on: Sprite::radio_on(),
            radio_off: Sprite::radio_off(),
        }
    }
}

impl Text1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and the scene
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin(
            "Text options",
            &mut self.window_demo_pose,
            Some(Vec2::new(self.demo_win_width, 0.0)),
            None,
            None,
        );
        if Ui::radio_img(
            "Default Font",
            self.font_selected == 1,
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            let font = Font::default();
            self.text_style_test = Text::make_style(font, 0.05, WHITE);
            self.font_selected = 1;
        }
        Ui::same_line();
        if Ui::radio_img(
            "Font Emoji",
            self.font_selected == 2,
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            let font = if cfg!(windows) {
                Font::from_files(&[
                    "C:\\Windows\\Fonts\\Seguiemj.ttf",
                    "fonts\\Noto_Emoji\\NotoEmoji-VariableFont_wght.ttf",
                ])
                .unwrap_or_default()
            } else {
                Font::from_file("fonts/Noto_Emoji/NotoEmoji-VariableFont_wght.ttf").unwrap_or_default()
            };
            self.text_style_test = Text::make_style(font, 0.05, WHITE);
            self.font_selected = 2;
        }
        Ui::same_line();
        if Ui::radio_img("Font text", self.font_selected == 3, &self.radio_off, &self.radio_on, UiBtnLayout::Left, None)
        {
            let font = if cfg!(windows) {
                Font::from_file("C:\\Windows\\Fonts\\Arial.ttf").unwrap_or_default()
            } else {
                Font::from_file("fonts/Inter/Inter-VariableFont_opsz_wght.ttf").unwrap_or_default()
            };
            self.text_style_test = Text::make_style(font, 0.05, WHITE);
            self.font_selected = 3;
        }
        Ui::next_line();

        if cfg!(target_os = "android") {
            if let Some(new_value) = Ui::toggle("Android Keyboard", &mut self.android_keyboard, None) {
                if new_value {
                    Platform::force_fallback_keyboard(false);
                } else {
                    Platform::force_fallback_keyboard(true);
                }
            }

            if self.android_keyboard && Platform::is_keyboard_visible() {
                Platform::keyboard_show(false, TextContext::Text);
                Input::key_inject_press(Key::Left);
                Input::key_inject_release(Key::Left);

                show_soft_input(true);
            }

            // Ui::same_line();
            // if let Some(new_value) = Ui::toggle("Winit IME Keyboard", self.android_keyboard_ime, None) {
            //     self.android_keyboard_ime = new_value;
            //     if new_value {
            //         Platform::force_fallback_keyboard(false);
            //     } else {
            //         Platform::force_fallback_keyboard(true);
            //     }
            // }

            // if self.android_keyboard_ime && Platform::is_keyboard_visible() {
            //     Platform::keyboard_show(false, TextContext::Text);
            //     Input::key_inject_press(Key::Left);
            //     Input::key_inject_release(Key::Left);
            //     show_soft_input_ime(self.sk_info.as_ref().unwrap().clone(), true);
            // }
        }
        Ui::same_line();
        if let Some(new_value) = Ui::toggle("French keyboard", &mut self.keyboard_layout_fr, None) {
            self.keyboard_layout_fr = true; // we can't reverse right now ^_^
            let keyboard_layouts = vec![FR_KEY_TEXT, FR_KEY_TEXT_SHIFT, FR_KEY_TEXT_ALT];
            if new_value {
                Log::diag("Setting keyboard to french");
                if !Platform::keyboard_set_layout(TextContext::Text, &keyboard_layouts) {
                    Log::err("Setting french keyboard for Text failed!");
                }
                if !Platform::keyboard_set_layout(TextContext::Password, &keyboard_layouts) {
                    Log::err("Setting french keyboard for Password failed!");
                }
            } else {
                let no = Sound::from_file("sounds/no.wav").unwrap();
                self.inst_play = Some(no.play(Vec3::ONE, None));
                Log::warn("Choosing the French keyboard is irrevocable!!");
            }
        }

        Ui::same_line();
        if Ui::button_img(format!("{:?}", self.text_context), &self.next_value, None, None, None) {
            self.text_context =
                unsafe { transmute::<u32, stereokit_rust::system::TextContext>(((self.text_context as u32) + 1) % 4) };
        }
        if Ui::button("Quit Demos", None) {
            SkInfo::send_event(&self.sk_info, StepperAction::quit(&self.id, "Quit button test"));
        }
        Ui::same_line();
        if Ui::button("test inject key F1", None) {
            Input::key_inject_press(Key::F1);
            Input::key_inject_release(Key::F1);
        }
        Ui::next_line();
        Ui::hseparator();
        Ui::push_text_style(self.text_style_test);
        //Ui::push_preserve_keyboard(true);
        Ui::input("Text_Sample", &mut self.text_sample, Some(Vec2::new(0.77, 0.8)), Some(self.text_context));
        // Ui::next_line();
        // Ui::push_preserve_keyboard(true);
        // Ui::text(&self.text_sample, None, None, None);
        Ui::pop_text_style();

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
