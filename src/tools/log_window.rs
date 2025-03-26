use crate::{
    font::Font,
    material::Cull,
    maths::{Matrix, Pose, Vec2, Vec3, units::CM},
    prelude::*,
    system::{LogItem, LogLevel, Text, TextAlign, TextFit, TextStyle},
    ui::{Ui, UiCut},
    util::Color128,
};
use std::sync::Mutex;

pub const SHOW_LOG_WINDOW: &str = "Tool_ShowLogWindow";

#[derive(IStepper)]
pub struct LogWindow<'a> {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub enabled: bool,

    pub pose: Pose,
    pub x_len: f32,
    pub y_len: f32,
    style_diag: TextStyle,
    style_info: TextStyle,
    style_warn: TextStyle,
    style_err: TextStyle,
    pub log_log: &'a Mutex<Vec<LogItem>>,
    log_index: f32,
    items_size: usize,
}

unsafe impl Send for LogWindow<'_> {}

impl<'a> LogWindow<'a> {
    pub fn new(log_log: &'a Mutex<Vec<LogItem>>) -> Self {
        let enabled = true;
        let pose = Pose::IDENTITY;
        let x_len = 110.0;
        let y_len = 15.0;

        let style_diag = TextStyle::from_font(Font::default(), 0.012, Color128::hsv(1.0, 0.0, 0.7, 1.0));
        let style_info = TextStyle::from_font(Font::default(), 0.012, Color128::hsv(1.0, 0.0, 1.0, 1.0));
        let style_warn = TextStyle::from_font(Font::default(), 0.012, Color128::hsv(0.17, 0.7, 1.0, 1.0));
        let style_err = TextStyle::from_font(Font::default(), 0.012, Color128::hsv(1.0, 0.7, 0.7, 1.0));
        for ui_text_style in [style_diag, style_info, style_warn, style_err] {
            ui_text_style.get_material().face_cull(Cull::Back); //.depth_test(DepthTest::Less).depth_write(true);
        }
        Self {
            id: "LogWindow".to_string(),
            sk_info: None,
            enabled,

            pose,
            x_len,
            y_len,
            style_diag,
            style_info,
            style_warn,
            style_err,
            log_log,
            log_index: 0.0,
            items_size: 0,
        }
    }

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(SHOW_LOG_WINDOW) {
            self.enabled = value.parse().unwrap_or(false)
        }
    }
    /// Called from IStepper::step, after check_event here you can draw your UI
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin("Log", &mut self.pose, Some(Vec2::new(self.x_len, 0.0) * CM), None, None);
        self.draw_logs(token);
        Ui::hseparator();
        Ui::window_end();
    }

    fn draw_logs(&mut self, token: &MainThreadToken) {
        let text_size = Vec2::new(Ui::get_layout_remaining().x, 0.024);
        let items = self.log_log.lock().unwrap();

        Ui::layout_push_cut(UiCut::Top, text_size.y * self.y_len, false);
        Ui::layout_push_cut(UiCut::Right, Ui::get_line_height() * 0.6, false);

        if self.items_size < items.len() {
            self.items_size = items.len();
            self.log_index = items.len() as f32;

            // if self.log_index < self.y_len {
            //     self.log_index = 0.0;
            // }
        }
        if let Some(pos) =
            Ui::vslider("scroll", &mut self.log_index, 0.0, items.len() as f32, Some(1.0), None, None, None)
        {
            self.log_index = f32::max(f32::min(pos, items.len() as f32 - 1.0), 0.0);
        }

        Ui::layout_pop();

        let start = Ui::get_layout_at();
        Ui::layout_reserve(Vec2::new(text_size.x, text_size.y * self.y_len), true, 0.0);

        let mut index = (self.log_index - self.y_len) as i32;
        let mut last_item_printed = self.log_index as i32;
        if index < 0 {
            index = 0;
            last_item_printed = self.y_len as i32;
        }
        for i in index..last_item_printed {
            if let Some(item) = items.get(i as usize) {
                let ts = match item.level {
                    LogLevel::Diagnostic => self.style_diag,
                    LogLevel::Inform => self.style_info,
                    LogLevel::Warning => self.style_warn,
                    LogLevel::Error => self.style_err,
                    _ => self.style_info,
                };

                let y = (i - index) as f32 * -text_size.y;
                Text::add_in(
                    token,
                    item.text.trim(),
                    Matrix::t(start + Vec3::new(0.0, y, -0.004)),
                    text_size,
                    TextFit::Clip | TextFit::Wrap,
                    Some(ts),
                    None,
                    Some(TextAlign::TopLeft),
                    Some(TextAlign::CenterLeft),
                    None,
                    None,
                    None,
                );

                if item.count > 1 {
                    let at = Vec3::new(start.x - text_size.x, start.y + y, start.z - 0.014);
                    Text::add_in(
                        token,
                        item.count.to_string(),
                        Matrix::t(at),
                        Vec2::new(text_size.x + 0.22, text_size.y),
                        TextFit::Clip,
                        Some(self.style_info),
                        None,
                        Some(TextAlign::TopLeft),
                        Some(TextAlign::CenterLeft),
                        None,
                        None,
                        None,
                    );
                }
            }
        }
        Ui::layout_pop();
    }
}
