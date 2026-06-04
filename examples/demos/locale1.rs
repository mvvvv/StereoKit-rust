use stereokit_rust::{
    font::Font,
    locale::*,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, units::CM},
    prelude::*,
    system::{Log, Text, TextContext, TextStyle},
    ui::Ui,
    util::{
        Color128, Platform,
        named_colors::{RED, WHITE},
    },
};

// ─── Locale table ─────────────────────────────────────────────────────────────

/// A single entry in the locale table: a language code, a human-readable label,
/// and the keyboard layers to pass to [`Platform::keyboard_set_layout`].
struct LocaleEntry {
    /// Two-letter (or two+two) BCP-47-like code matched against the `LANG` env var
    /// (e.g. `"fr"`, `"de"`, `"pt"`, `"ja"`).
    code: &'static str,
    /// Label shown in the selection UI.
    label: &'static str,
    /// Keyboard layers (normal, shift, optional alt) from [`stereokit_rust::locale`].
    layers: Vec<&'static str>,
}

fn make_locale_entries() -> Vec<LocaleEntry> {
    vec![
        LocaleEntry {
            code: "fr",
            label: "Français (AZERTY)",
            layers: vec![FR_KEY_TEXT, FR_KEY_TEXT_SHIFT, FR_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "de",
            label: "Deutsch (QWERTZ)",
            layers: vec![DE_KEY_TEXT, DE_KEY_TEXT_SHIFT, DE_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "es",
            label: "Español (QWERTY)",
            layers: vec![ES_KEY_TEXT, ES_KEY_TEXT_SHIFT, ES_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "pt",
            label: "Português BR (QWERTY)",
            layers: vec![PT_BR_KEY_TEXT, PT_BR_KEY_TEXT_SHIFT, PT_BR_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "it",
            label: "Italiano (QWERTY)",
            layers: vec![IT_KEY_TEXT, IT_KEY_TEXT_SHIFT, IT_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "sv",
            label: "Svenska (QWERTY)",
            layers: vec![SV_KEY_TEXT, SV_KEY_TEXT_SHIFT, SV_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "no",
            label: "Norsk (QWERTY)",
            layers: vec![SV_KEY_TEXT, SV_KEY_TEXT_SHIFT, SV_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "da",
            label: "Dansk (QWERTY)",
            layers: vec![SV_KEY_TEXT, SV_KEY_TEXT_SHIFT, SV_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "pl",
            label: "Polski (QWERTY)",
            layers: vec![PL_KEY_TEXT, PL_KEY_TEXT_SHIFT, PL_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "cs",
            label: "Čeština (QWERTY)",
            layers: vec![CS_KEY_TEXT, CS_KEY_TEXT_SHIFT, CS_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "tr",
            label: "Türkçe (QWERTY)",
            layers: vec![TR_KEY_TEXT, TR_KEY_TEXT_SHIFT, TR_KEY_TEXT_ALT],
        },
        LocaleEntry {
            code: "ru", label: "Русский (ЙЦУКЕН)", layers: vec![RU_KEY_TEXT, RU_KEY_TEXT_SHIFT]
        },
        LocaleEntry {
            code: "uk", label: "Українська (Кирилиця)", layers: vec![UK_KEY_TEXT, UK_KEY_TEXT_SHIFT]
        },
        LocaleEntry { code: "el", label: "Ελληνικά (Greek)", layers: vec![GR_KEY_TEXT, GR_KEY_TEXT_SHIFT] },
        LocaleEntry { code: "ar", label: "العربية (Arabic)", layers: vec![AR_KEY_TEXT, AR_KEY_TEXT_SHIFT] },
        LocaleEntry { code: "he", label: "עברית (Hebrew)", layers: vec![HE_KEY_TEXT, HE_KEY_TEXT_SHIFT] },
        LocaleEntry {
            code: "ja",
            label: "日本語 (Hiragana)",
            layers: vec![JA_KEY_TEXT, JA_KEY_TEXT_SHIFT, JA_KEY_TEXT_ALT],
        },
    ]
}

// ─── Locale detection ─────────────────────────────────────────────────────────

/// Extract the two-letter language code from the process environment.
///
/// Checks the following variables in order: `LANG`, `LANGUAGE`, `LC_ALL`,
/// `LC_CTYPE`.  A value like `"fr_FR.UTF-8"` yields `"fr"`; `"ja_JP.UTF-8"`
/// yields `"ja"`.  Returns an empty string if no usable value is found
/// (common on Windows when `LANG` is unset).
fn detect_language_code() -> String {
    use stereokit_rust::tools::os_api;

    os_api::get_locale()
        .split('.')
        .next()
        .unwrap_or_default()
        .split('_')
        .next()
        .unwrap_or_default()
        .to_string()
}

// ─── IStepper ─────────────────────────────────────────────────────────────────

/// Demonstrates all international keyboard layouts from [`stereokit_rust::locale`].
///
/// On startup the stepper auto-detects the current system locale (via the `LANG`
/// environment variable) and pre-selects the matching layout.  The UI shows a
/// button for every available layout; clicking one applies it immediately.
/// A text-input field at the bottom lets the user type with the active keyboard.
#[derive(IStepper)]
pub struct Locale1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    pub window_demo_pose: Pose,
    pub demo_win_width: f32,

    /// All supported locale entries (code + label + keyboard layers).
    locale_entries: Vec<LocaleEntry>,
    /// Index of the currently active layout, or `None` when using SK default.
    selected_index: Option<usize>,
    /// The two-letter code detected at startup (for display purposes).
    detected_code: String,

    /// Text typed in the input field.
    pub text_sample: String,
    text_context: TextContext,
    text_style_test: TextStyle,

    /// Large 3D text displayed behind the window.
    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Locale1 {}

impl Default for Locale1 {
    fn default() -> Self {
        let locale_entries = make_locale_entries();
        let detected_code = detect_language_code();
        let selected_index = if detected_code.is_empty() {
            None
        } else {
            locale_entries.iter().position(|e| e.code == detected_code.as_str())
        };

        Self {
            id: "Locale1".to_string(),
            sk_info: None,

            window_demo_pose: Pose::new(Vec3::new(0.0, 1.5, -1.3), Some(Quat::look_dir(Vec3::new(1.0, 0.0, 1.0)))),
            demo_win_width: 80.0 * CM,

            locale_entries,
            selected_index,
            detected_code,

            text_sample: String::from("Type here / Tapez ici / Введите текст…"),
            text_context: TextContext::Text,
            text_style_test: Text::make_style(Font::default(), 0.025, WHITE),

            text: "locale1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Locale1 {
    /// Called from `IStepper::initialize`.  Applies the auto-detected layout (if any).
    fn start(&mut self) -> bool {
        if let Some(idx) = self.selected_index {
            self.apply_layout(idx);
        } else {
            Log::info(format!(
                "Locale1: no locale match for \"{}\", using StereoKit default keyboard",
                self.detected_code
            ));
        }
        true
    }

    /// Called from `IStepper::step` to handle incoming stepper events.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Apply the keyboard layout at the given index to both `Text` and `Password` contexts.
    fn apply_layout(&self, idx: usize) {
        let entry = &self.locale_entries[idx];
        if !Platform::keyboard_set_layout(TextContext::Text, &entry.layers) {
            Log::err(format!("Locale1: failed to set Text keyboard for \"{}\"", entry.label));
        }
        if !Platform::keyboard_set_layout(TextContext::Password, &entry.layers) {
            Log::err(format!("Locale1: failed to set Password keyboard for \"{}\"", entry.label));
        }
        Log::info(format!("Locale1: keyboard layout → {}", entry.label));
    }

    /// Called from `IStepper::step` after `check_event`.  Draws the UI.
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin(
            "Locale Keyboard Tester",
            &mut self.window_demo_pose,
            Some(Vec2::new(self.demo_win_width, 0.0)),
            None,
            None,
        );

        // ── Layout selection buttons with auto-wrapping ───────────────────────
        let ui_settings = Ui::get_settings();
        let style = Ui::get_text_style();

        let mut new_selection: Option<usize> = None;
        let selected = self.selected_index;
        let mut curr_width = ui_settings.margin * 2.0;

        for i in 0..self.locale_entries.len() {
            let btn_width =
                Text::size_layout(self.locale_entries[i].label, Some(style), None).x + ui_settings.padding * 2.0;

            // Wrap to next line when the button would overflow
            if i > 0 && curr_width + btn_width + ui_settings.gutter > self.demo_win_width {
                curr_width = ui_settings.margin * 2.0 + btn_width + ui_settings.gutter;
                // Don't call Ui::same_line() → new line starts automatically
            } else {
                if i > 0 {
                    Ui::same_line();
                }
                curr_width += btn_width + ui_settings.gutter;
            }

            // Highlight the currently active layout with a warm tint
            let is_selected = selected == Some(i);
            if is_selected {
                Ui::push_tint(Color128::hsv(0.12, 0.85, 1.0, 1.0));
            }

            if Ui::button_builder(self.locale_entries[i].label).size(Vec2::new(btn_width, 0.0)).press() {
                new_selection = Some(i);
            }

            if is_selected {
                Ui::pop_tint();
            }
        }

        // Apply a newly selected layout
        if let Some(idx) = new_selection {
            self.selected_index = Some(idx);
            self.apply_layout(idx);
        }

        Ui::next_line();

        // ── Status line ───────────────────────────────────────────────────────
        let status = if let Some(idx) = self.selected_index {
            format!("Active: {}  •  detected locale: \"{}\"", self.locale_entries[idx].label, self.detected_code)
        } else {
            format!("No match for detected locale \"{}\" — using StereoKit default", self.detected_code)
        };
        Ui::label_builder(&status).draw();
        Ui::next_line();
        Ui::hseparator();

        // ── Text input to test the active keyboard ────────────────────────────
        Ui::push_text_style(self.text_style_test);
        Ui::input(
            "locale_text_sample",
            &mut self.text_sample,
            Some(Vec2::new(self.demo_win_width - 0.02, 0.12)),
            Some(self.text_context),
        );
        Ui::pop_text_style();

        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
