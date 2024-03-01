use std::mem::transmute;

use stereokit_rust::{
    font::Font,
    maths::{units::CM, Matrix, Pose, Quat, Vec2, Vec3},
    sk::{IStepper, StepperAction, StepperId},
    sprite::Sprite,
    system::{Log, Text, TextContext, TextStyle},
    ui::{Ui, UiBtnLayout},
    util::{
        named_colors::{RED, WHITE},
        Platform,
    },
};
use winit::event_loop::EventLoopProxy;

pub const FR_KEY_TEXT: &str = r#"`-`-192|&-&-49|é-é-50|"-"-51|'-'-52|(-(-53|\--\--54|è-è-55|_-_-56|ç-ç-57|à-à-48|)-)-189|=-=-187|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|a-a-81|z-z-87|e-e-69|r-r-82|t-t-84|y-y-89|u-u-85|i-i-73|o-o-79|p-p-80|^-^-219|$-$-221|#-#-220
Enter-\n-13-4|q-q-65|s-s-83|d-d-68|f-f-70|g-g-71|h-h-72|j-j-74|k-k-75|l-l-76|m-m-186|ù-ù-222|Enter-\n-13-3
spr:sk/ui/shift--16-5-visit_1|<-<-90|w-w-88|x-x-67|c-c-86|v-v-66|b-b-78|n-n-77|,-,-188|;-;-190|:-:-191|spr:sk/ui/shift--16-2-visit_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-mod| - -32-9|Alt--18-3-mod|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|)"#;

pub const FR_KEY_TEXT_SHIFT: &str = r#"~-~-192|1-1-49|2-2-50|3-3-51|4-4-52|5-5-53|6-6-54|7-7-55|8-8-56|9-9-57|0-0-48|\--\--189|=-=-187|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|A-A-81|Z-Z-87|E-E-69|R-R-82|T-T-84|Y-Y-89|U-U-85|I-I-73|O-O-79|P-P-80|¨-¨-219|£-£-221|\|-\|-220
Enter-\n-13-4|A-A-65|S-S-83|D-D-68|F-F-70|G-G-71|H-H-72|J-J-74|K-K-75|L-L-76|:-:-186|"-"-222|Enter-\n-13-3
spr:sk/ui/shift--16-5-go_0|>->-90|W-W-88|X-X-67|C-C-86|V-V-66|B-B-78|N-N-77|?-?-188|.->.190|/-/-191|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-mod| - -32-9|Alt--18-3-mod|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

pub const FR_KEY_TEXT_ALT: &str = r#"²-²-192|*-*-49|~-~-50|#-#-51|{-{-52|[-[-53||-|-54|`-`-55|\-\-56|^-^-57|@-@-48|]-]-189|}-}-187|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|A-A-81|Z-Z-87|€-€-69|R-R-82|T-T-84|Y-Y-89|µ-µ-85|I-I-73|O-O-79|P-P-80|¨-¨-219|¤-¤-221|*-*|-220
Enter-\n-13-4|A-A-65|§-§-83|D-D-68|F-F-70|G-G-71|H-H-72|J-J-74|K-K-75|£-£-76|:-:-186|"-"-222|Enter-\n-13-3
spr:sk/ui/shift--16-5-go_0|>->-90|W-W-88|X-X-67|C-C-86|V-V-66|B-B-78|N-N-77|?-?-188|!-!.190|/-/-191|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-mod| - -32-9|Alt--18-3-mod|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

pub struct Text1 {
    id: StepperId,
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    pub transform: Matrix,
    pub window_demo_pose: Pose,
    pub demo_win_width: f32,
    pub keyboard_layout_fr: bool,
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

impl Default for Text1 {
    fn default() -> Self {
        Self {
            id: "Text1".to_string(),
            event_loop_proxy: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -0.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 80.0 * CM,
            keyboard_layout_fr: false,
            show_keyboard: false,
            text_sample: String::from("Text to modify"),
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

impl IStepper for Text1 {
    fn initialize(&mut self, id: StepperId, event_loop_proxy: EventLoopProxy<StepperAction>) -> bool {
        self.id = id;
        self.event_loop_proxy = Some(event_loop_proxy);
        true
    }

    fn step(&mut self, _event_report: &[StepperAction]) {
        self.draw()
    }
}

impl Text1 {
    fn draw(&mut self) {
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
        if Ui::radio_img("Font 1", self.font_selected == 2, &self.radio_off, &self.radio_on, UiBtnLayout::Left, None) {
            let font = Font::from_file("fonts/Courier Prime.ttf").unwrap_or_default();
            self.text_style_test = Text::make_style(font, 0.05, WHITE);
            self.font_selected = 2;
        }
        Ui::same_line();
        if Ui::radio_img("Font 2", self.font_selected == 3, &self.radio_off, &self.radio_on, UiBtnLayout::Left, None) {
            let font = Font::from_file("fonts/Courier Prime Italic.ttf").unwrap_or_default();
            self.text_style_test = Text::make_style(font, 0.05, WHITE);
            self.font_selected = 3;
        }
        Ui::same_line();
        Ui::next_line();
        if let Some(new_value) = Ui::toggle("French keyboard", self.keyboard_layout_fr, None) {
            self.keyboard_layout_fr = new_value;
            let keyboard_layouts = vec![FR_KEY_TEXT, FR_KEY_TEXT_SHIFT, FR_KEY_TEXT_ALT];
            if new_value {
                Log::diag("Setting keyboard to french");
                if !Platform::keyboard_set_layout(TextContext::Text, keyboard_layouts) {
                    Log::err("Setting french keyboard failed!");
                }
            }
        }

        Ui::same_line();
        if Ui::button_img(format!("{:?}", self.text_context), &self.next_value, None, None, None) {
            self.text_context = unsafe { transmute(((self.text_context as u32) + 1) % 4) };
        }
        Ui::next_line();
        Ui::hseparator();
        Ui::push_text_style(self.text_style_test);
        //Ui::push_preserve_keyboard(true);
        if let Some(new_value) = Ui::input("Text_Sample", &self.text_sample, None, Some(self.text_context)) {
            self.text_sample = new_value;
        }
        // Ui::next_line();
        // Ui::push_preserve_keyboard(true);
        // Ui::text(&self.text_sample, None, None, None);
        Ui::pop_text_style();

        Ui::window_end();

        Text::add_at(&self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
