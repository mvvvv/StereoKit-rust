use crate::system::{Log, TextContext};
use crate::util::Platform;
/// Sets the locale used by some of the tools like [`FileBrowserB`](crate::tools::file_browser_b::FileBrowserB) texts
/// (window title, toolbar, panels, entry annotations, preview panel...), e.g. `set_locale("fr")`. It is a global setting
/// (shared with any other rust-i18n user of the crate)
/// applied from the next drawn frame — no need to close and reopen the browser.
///
/// The available locales are the `locales/*.toml` catalogues compiled into the crate (English fallback, Chinese,
/// French, German, Italian, Japanese, Korean, Portuguese and Spanish bundled, see [`available_locales`]); a locale
/// without a catalogue falls back to English, and a key missing from a catalogue falls back to English before
/// returning the key itself.
///
/// ### Example
/// ```
/// # stereokit_rust::test_init_sk!();
/// use stereokit_rust::tools::locales::{set_locale, locale_language_and_country};
///
/// set_locale("fr");
/// assert_eq!(locale_language_and_country(), ("Français".to_string(), "".to_string()));
///
/// set_locale("de");
/// assert_eq!(locale_language_and_country(), ("Deutsch".to_string(), "".to_string()));
///
/// set_locale("zh");
/// assert_eq!(locale_language_and_country(), ("中文".to_string(), "".to_string()));
/// ```
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// The locale currently used  is `"en"` until [`set_locale`] is called.
/// ```
/// # stereokit_rust::test_init_sk!();
/// use stereokit_rust::tools::locales::{set_locale, locale};
///
/// assert_eq!(locale(), "en".to_string());
///
/// set_locale("fr");
/// assert_eq!(locale(), "fr".to_string());
/// ```
pub fn locale() -> String {
    rust_i18n::locale().to_string()
}

/// Returns the locale language and country if any, e.g. (`"Français"`, `"Canada"`) for `fr-CA`,
/// (`"English"`, `""`) for `en` ...
pub fn locale_language_and_country() -> (String, String) {
    use rust_i18n::t;
    (t!("locales.language").into(), t!("locales.country").into())
}

/// The locales available, one per `locales/*.toml` catalogue compiled into the crate (e.g. `["de", "en", "es", "fr",
/// "it", "ja", "ko", "pt", "zh"]`). Add a catalogue file to this list to add a language, then select it with
/// [`set_locale`].
///
/// ### Example
/// ```
/// # stereokit_rust::test_init_sk!();
/// use stereokit_rust::tools::locales::available_locales;
///
/// assert_eq!(available_locales(), vec!["de", "en", "es", "fr", "it", "ja", "ko", "pt", "zh"]);
/// ```
pub fn available_locales() -> Vec<String> {
    rust_i18n::available_locales!().iter().map(|l| l.to_string()).collect()
}

/// International keyboard layouts for StereoKit virtual-reality applications.
///
/// Each layout consists of 2 or 3 layers:
/// - **Layer 0** — Normal (default, lowercase).
/// - **Layer 1** — Shift (uppercase and secondary characters).
/// - **Layer 2** — Alt / AltGr (special characters, where applicable).
///
/// All the layouts are associated constants of this struct, named after their language code with an optional `_SHIFT`
/// / `_ALT` suffix selecting the layer — `Keyboard::FR`, `Keyboard::FR_SHIFT` and `Keyboard::FR_ALT` for the French
/// AZERTY, `Keyboard::PT_BR` for the Brazilian Portuguese, `Keyboard::JA_ALT` for the Japanese extra kana layer...
/// Languages without special characters have no alt layer. Use
/// [`Platform::keyboard_set_layout`](crate::util::Platform::keyboard_set_layout) to apply a layout manually, or
/// [`Keyboard::apply_locale`] to apply the layout matching a locale code — [`Keyboard::locales`] lists the available
/// `(code, label)` pairs.
///
/// ## Key-string format
///
/// Each row is separated by a newline; within a row keys are separated by `|`:
///
/// * Single printable character → that character (e.g. `a`, `é`).
/// * `display-output-keycode-width-action` → full descriptor (empty fields are allowed).
/// * `spr:sk/ui/…` prefix → render a built-in sprite instead of text.
///
/// **Escape sequences** (recognised by the StereoKit parser inside the raw strings):
/// `\-` = hyphen, `\|` = pipe, `\\` = backslash,
/// `\b` = backspace, `\t` = tab, `\n` = newline.
///
/// ## Example
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{
///     tools::locales::Keyboard,
///     system::TextContext,
///     util::Platform,
/// };
/// let layouts = vec![Keyboard::DE, Keyboard::DE_SHIFT, Keyboard::DE_ALT];
/// assert_eq!(Platform::keyboard_set_layout(TextContext::Text, &layouts), true);
/// ```
pub struct Keyboard;

// ─── Locale table ─────────────────────────────────────────────────────────────

/// A single entry of the locale table: a language code, a human readable label,
/// and the keyboard layers to pass to [`Platform::keyboard_set_layout`].
struct LocaleEntry {
    /// Two-letter (or two+two) BCP-47-like code matched against the system locale
    /// (e.g. `"fr"`, `"de"`, `"pt"`, `"ja"`).
    code: &'static str,
    /// Label shown in selection UIs.
    label: &'static str,
    /// Keyboard layers (normal, shift, optional alt) taken from [`Keyboard`]'s constants.
    layers: &'static [&'static str],
}

/// The locale table itself, listing code / label / layers for every layout shipped in this
/// module.  Norwegian (`"no"`) and Danish (`"da"`) share the Swedish/Nordic layout, hence
/// several codes can point to the same layers.
const LOCALES: &[LocaleEntry] = &[
    LocaleEntry {
        code: "fr",
        label: "Français (AZERTY)",
        layers: &[Keyboard::FR, Keyboard::FR_SHIFT, Keyboard::FR_ALT],
    },
    LocaleEntry {
        code: "de",
        label: "Deutsch (QWERTZ)",
        layers: &[Keyboard::DE, Keyboard::DE_SHIFT, Keyboard::DE_ALT],
    },
    LocaleEntry {
        code: "es",
        label: "Español (QWERTY)",
        layers: &[Keyboard::ES, Keyboard::ES_SHIFT, Keyboard::ES_ALT],
    },
    LocaleEntry {
        code: "pt",
        label: "Português BR (QWERTY)",
        layers: &[Keyboard::PT_BR, Keyboard::PT_BR_SHIFT, Keyboard::PT_BR_ALT],
    },
    LocaleEntry {
        code: "it",
        label: "Italiano (QWERTY)",
        layers: &[Keyboard::IT, Keyboard::IT_SHIFT, Keyboard::IT_ALT],
    },
    LocaleEntry {
        code: "sv",
        label: "Svenska (QWERTY)",
        layers: &[Keyboard::SV, Keyboard::SV_SHIFT, Keyboard::SV_ALT],
    },
    LocaleEntry { code: "no", label: "Norsk (QWERTY)", layers: &[Keyboard::SV, Keyboard::SV_SHIFT, Keyboard::SV_ALT] },
    LocaleEntry { code: "da", label: "Dansk (QWERTY)", layers: &[Keyboard::SV, Keyboard::SV_SHIFT, Keyboard::SV_ALT] },
    LocaleEntry { code: "pl", label: "Polski (QWERTY)", layers: &[Keyboard::PL, Keyboard::PL_SHIFT, Keyboard::PL_ALT] },
    LocaleEntry {
        code: "cs",
        label: "Čeština (QWERTY)",
        layers: &[Keyboard::CS, Keyboard::CS_SHIFT, Keyboard::CS_ALT],
    },
    LocaleEntry {
        code: "tr", label: "Türkçe (QWERTY)", layers: &[Keyboard::TR, Keyboard::TR_SHIFT, Keyboard::TR_ALT]
    },
    LocaleEntry { code: "ru", label: "Русский (ЙЦУКЕН)", layers: &[Keyboard::RU, Keyboard::RU_SHIFT] },
    LocaleEntry {
        code: "uk", label: "Українська (Кирилиця)", layers: &[Keyboard::UK, Keyboard::UK_SHIFT]
    },
    LocaleEntry { code: "el", label: "Ελληνικά (Greek)", layers: &[Keyboard::GR, Keyboard::GR_SHIFT] },
    LocaleEntry { code: "ar", label: "العربية (Arabic)", layers: &[Keyboard::AR, Keyboard::AR_SHIFT] },
    LocaleEntry { code: "he", label: "עברית (Hebrew)", layers: &[Keyboard::HE, Keyboard::HE_SHIFT] },
    LocaleEntry {
        code: "ja",
        label: "日本語 (Hiragana)",
        layers: &[Keyboard::JA, Keyboard::JA_SHIFT, Keyboard::JA_ALT],
    },
];

impl Keyboard {
    // ─── French AZERTY (FR) ──────────────────────────────────────────────────────

    /// French AZERTY keyboard — normal layer.
    ///
    /// Matches the layout defined in the `text1` demo.  Identical to the constants
    /// exported from that example so that either source can be used interchangeably.
    pub const FR: &str = r#"²|&|é|"|'|(|\-|è|_|ç|à|)|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|a|z|e|r|t|y|u|i|o|p|^|$|[|]|\|
Entrée-\n-13-4|q|s|d|f|g|h|j|k|l|m|ù|*|#|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|w|x|c|v|b|n|,|;|:|!|`|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// French AZERTY keyboard — shift layer.
    pub const FR_SHIFT: &str = r#"@|1|2|3|4|5|6|7|8|9|0|°|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|A|Z|E|R|T|Y|U|I|O|P|¨|£|Ê|É|È
Entrée-\n-13-4|Q|S|D|F|G|H|J|K|L|M|%|µ|Ç|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|W|X|C|V|B|N|?|.|/|§|À|Ô|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// French AZERTY keyboard — alt layer (special characters and accented letters).
    pub const FR_ALT: &str = r#"*|/|~|#|{|[|\||`|\\|^|@|]|}|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|à|â|ä|ç|é|è|ê|ë|î|ï|ô|ö|«|»|¤
Entrée-\n-13-4|ù|û|ü|ÿ|À|Â|Ä|Ç|É|È|Ê|Ë|%|Entrée-\n-13-3
spr:sk/ui/shift--16-3-go_1|Î|Ï|Ô|Ö|Ù|Û|Ü|Ÿ|$|£|€|¥|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── German QWERTZ (DE) ──────────────────────────────────────────────────────

    /// German QWERTZ keyboard — normal layer.
    pub const DE: &str = r#"^|1|2|3|4|5|6|7|8|9|0|ß|´|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|z|u|i|o|p|ü|+|[|]|\|
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ö|ä|#|~|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|y|x|c|v|b|n|m|,|.|\-|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// German QWERTZ keyboard — shift layer.
    pub const DE_SHIFT: &str = r#"°|!|"|§|$|%|&|/|(|)|=|?|`|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Z|U|I|O|P|Ü|*|£|¥|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|Ö|Ä|'|±|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Y|X|C|V|B|N|M|;|:|_|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// German QWERTZ keyboard — alt layer (standard AltGr characters plus common European accents).
    ///
    /// Standard AltGr positions: `²` `³` `{` `[` `]` `}` `\` `@` `€` `µ` `|`.
    pub const DE_ALT: &str = r#"`|¹|²|³|¼|½|¬|{|[|]|}|\\|¸|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ā|€|ŕ|ŧ|ź|ú|í|ó|þ|ü|~|«|»|¤
Enter-\n-13-4|á|ß|ð|ó|ğ|ħ|ĵ|ĸ|ł|ø|ä||%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|£|$|ý|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Spanish QWERTY — Spain (ES) ─────────────────────────────────────────────

    /// Spanish QWERTY keyboard (Spain) — normal layer.
    pub const ES: &str = r#"º|1|2|3|4|5|6|7|8|9|0|'|¡|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|`|+|[|]|\|
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ñ|´|#|~|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|z|x|c|v|b|n|m|,|.|\-|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Spanish QWERTY keyboard (Spain) — shift layer.
    pub const ES_SHIFT: &str = r#"ª|!|"|·|$|%|&|/|(|)|=|?|¿|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|^|*|£|¥|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|Ñ|¨|@|±|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Z|X|C|V|B|N|M|;|:|_|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Spanish QWERTY keyboard (Spain) — alt layer (standard AltGr characters plus common accents).
    ///
    /// Standard AltGr positions: `|` `@` `#` `~` `½` `¬` `{` `[` `]` `}` `\` `€`.
    pub const ES_ALT: &str = r#"°||@|#|~|½|¬|{|[|]|}|\\|¿|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ẃ|€|ŕ|ŧ|ý|ú|í|ó|þ|[|]|«|»|¤
Enter-\n-13-4|á|ś|ð|ó|ğ|ħ|ĵ|ĸ|ł|ñ|{|}|%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|£|ź|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Portuguese QWERTY — Brazil (PT_BR) ──────────────────────────────────────

    /// Portuguese QWERTY keyboard (Brazil) — normal layer.
    pub const PT_BR: &str = r#"'|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|´|[|]|#||
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ç|~|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|z|x|c|v|b|n|m|,|.|;|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Portuguese QWERTY keyboard (Brazil) — shift layer.
    pub const PT_BR_SHIFT: &str = r#""|!|@|#|$|%|¨|&|*|(|)|_|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|`|{|}|§|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|Ç|^||?|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|/|Z|X|C|V|B|N|M|<|>|:|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Portuguese QWERTY keyboard (Brazil) — alt layer (AltGr characters and common accented letters).
    pub const PT_BR_ALT: &str = r#"`|¹|²|³|£|€|¥|{|[|]|}|\\|~|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ẃ|€|ŕ|ŧ|ý|ú|í|ó|þ|´|{|}|«|»
Enter-\n-13-4|á|ã|ê|â|õ|ħ|ĵ|ĸ|ł|ç|~||%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|£|ź|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Italian QWERTY (IT) ─────────────────────────────────────────────────────

    /// Italian QWERTY keyboard — normal layer.
    pub const IT: &str = r#"\\|1|2|3|4|5|6|7|8|9|0|'|ì|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|è|+|[|]|~
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ò|à|ù|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|z|x|c|v|b|n|m|,|.|\-|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Italian QWERTY keyboard — shift layer.
    pub const IT_SHIFT: &str = r#"\||!|"|£|$|%|&|/|(|)|=|?|^|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|é|*|«|»|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|ç|°|§|?|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Z|X|C|V|B|N|M|;|:|_|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Italian QWERTY keyboard — alt layer (AltGr characters and common accented letters).
    ///
    /// Standard AltGr positions: `@` `#` `[` `]` `{` `}` `€` `~`.
    pub const IT_ALT: &str = r#"`|¹|²|³|¼|½|¬|{|[|]|}|@|~|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ẃ|€|ŕ|ŧ|ý|ú|í|ó|þ|[|]|£|¥|\|
Enter-\n-13-4|á|ś|ð|ó|ğ|ħ|ĵ|ĸ|ł|ø|ä|$|%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|£|ź|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Swedish / Nordic QWERTY (SV) ────────────────────────────────────────────

    /// Swedish / Nordic QWERTY keyboard — normal layer.
    ///
    /// The same layout is used in Norway and Denmark with minor glyph differences
    /// (ø/ö and å/aa).
    pub const SV: &str = r#"§|1|2|3|4|5|6|7|8|9|0|+|´|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|å|¨|[|]|\|
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ö|ä|'|~|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|z|x|c|v|b|n|m|,|.|\-|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Swedish / Nordic QWERTY keyboard — shift layer.
    pub const SV_SHIFT: &str = r#"°|!|"|#|¤|%|&|/|(|)|=|?|`|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|Å|^|£|¥|±
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|Ö|Ä|*|µ|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Z|X|C|V|B|N|M|;|:|_|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Swedish / Nordic QWERTY keyboard — alt layer (standard AltGr characters).
    ///
    /// Standard AltGr positions: `@` `£` `$` `€` `¥` `{` `[` `]` `}` `\` `€` `~`.
    pub const SV_ALT: &str = r#"§|€|@|£|$|€|¥|{|[|]|}|\\|~|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ẃ|€|ŕ|ŧ|ý|ú|í|ó|þ|å|^|«|»|¤
Enter-\n-13-4|á|ś|ð|œ|ğ|ħ|ĵ|ĸ|ł|ø|ä||%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|£|ź|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Polish QWERTY (PL) ──────────────────────────────────────────────────────

    /// Polish QWERTY keyboard — normal layer (standard US QWERTY).
    pub const PL: &str = r#"`|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|[|]|\\|~|/
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|;|'|#|€|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|z|x|c|v|b|n|m|,|.|/|@|£|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Polish QWERTY keyboard — shift layer.
    pub const PL_SHIFT: &str = r#"~|!|@|#|$|%|^|&|*|(|)|_|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|{|}|\||¥|±
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|:|"|@|©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|\||Z|X|C|V|B|N|M|<|>|?|·|µ|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Polish QWERTY keyboard — alt layer (Polish diacritics via AltGr).
    ///
    /// AltGr positions: `ą` `ę` `ó` `ś` `ł` `ż` `ź` `ć` `ń`.
    pub const PL_ALT: &str = r#"`|1|2|3|4|5|6|7|8|9|0|\-|=|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|ę|r|t|y|u|i|ó|p|[|]|\\|«|»
Enter-\n-13-4|ą|ś|d|f|g|h|j|k|ł|;|'|#|%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|ż|ź|ć|v|b|ń|m|,|.|/|…|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Czech QWERTY (CS) ───────────────────────────────────────────────────────

    /// Czech QWERTY keyboard — normal layer.
    ///
    /// Czech diacritics appear on the number row: ě š č ř ž ý á í é.
    pub const CS: &str = r#";|+|ě|š|č|ř|ž|ý|á|í|é|=|´|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|i|o|p|ú|)|[|]|\|
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ů|§|"|~|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|\\|z|x|c|v|b|n|m|,|.|/|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Czech QWERTY keyboard — shift layer.
    pub const CS_SHIFT: &str = r#"°|1|2|3|4|5|6|7|8|9|0|%|ˇ|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|/|(|£|¥|±
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|"|!|'|µ|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0||Z|X|C|V|B|N|M|?|:|_|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Czech QWERTY keyboard — alt layer (AltGr characters: brackets, symbols, Latin extensions).
    pub const CS_ALT: &str = r#"`|~|ˇ|^|˘|°|˛|`|˙|´|˝|¸|¨|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|\\||€|®|Ŧ|ý|ú|ı|Ø|Þ|÷|×|«|»|¤
Enter-\n-13-4|Æ|Ł|Ð|Ŋ|Ħ|Ĥ|ĵ|ĸ|Ŀ|ø|¶|ˇ|%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|«|»|©|ƒ|{|[|]|}|<|>|@|…|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Turkish QWERTY (TR) ─────────────────────────────────────────────────────

    /// Turkish QWERTY keyboard — normal layer.
    ///
    /// Turkish-specific keys: `ğ` `ü` `ş` `ı` (dotless-i) `i` (dotted-i) `ö` `ç`.
    pub const TR: &str = r#"`|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|q|w|e|r|t|y|u|ı|o|p|ğ|ü|[|]||
Enter-\n-13-4|a|s|d|f|g|h|j|k|l|ş|i|,|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|z|x|c|v|b|n|m|ö|ç|.|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Turkish QWERTY keyboard — shift layer.
    ///
    /// Note the Turkish İ (uppercase dotted-i) distinct from English I.
    pub const TR_SHIFT: &str = r#"~|!|'|^|+|%|&|/|(|)|=|?|_|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|Ğ|Ü|£|¥|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|Ş|İ|;|±|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Z|X|C|V|B|N|M|Ö|Ç|:|©|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Turkish QWERTY keyboard — alt layer (AltGr symbols and additional Latin letters).
    pub const TR_ALT: &str = r#"`|¹|²|³|¼|½|¾|{|[|]|}|\\|~|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|@|ω|€|ŕ|ŧ|ý|ú|í|ó|þ|ü|ñ|«|»|¤
Enter-\n-13-4|á|ś|ð|ó|ğ|ħ|ĵ|ĸ|ł|ø|ä||%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1||ź|χ|©|ν|β|ñ|µ|·|…|–|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Russian Cyrillic ЙЦУКЕН (RU) ────────────────────────────────────────────

    /// Russian ЙЦУКЕН keyboard — normal layer (lowercase Cyrillic).
    pub const RU: &str = r#"ё|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|й|ц|у|к|е|н|г|ш|щ|з|х|ъ|[|]|\\
Enter-\n-13-4|ф|ы|в|а|п|р|о|л|д|ж|э|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|/|я|ч|с|м|и|т|ь|б|ю|.|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Russian ЙЦУКЕН keyboard — shift layer (uppercase Cyrillic).
    pub const RU_SHIFT: &str = r#"Ё|!|"№|;|%|:|?|*|(|)|_|+|Æ|Œ|£|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Й|Ц|У|К|Е|Н|Г|Ш|Щ|З|Х|Ъ|«|»|¤
Enter-\n-13-4|Ф|Ы|В|А|П|Р|О|Л|Д|Ж|Э|/|©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|\\|Я|Ч|С|М|И|Т|Ь|Б|Ю|,|®|µ|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Ukrainian Cyrillic (UK) ──────────────────────────────────────────────────

    /// Ukrainian Cyrillic keyboard — normal layer (lowercase).
    ///
    /// Ukrainian-specific letters: `і` `ї` `є` `ґ` (absent in Russian).
    pub const UK: &str = r#"'|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|й|ц|у|к|е|н|г|ш|щ|з|х|ї|[|]|\\
Enter-\n-13-4|ф|і|в|а|п|р|о|л|д|ж|є|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|/|я|ч|с|м|и|т|ь|б|ю|ґ|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Ukrainian Cyrillic keyboard — shift layer (uppercase).
    pub const UK_SHIFT: &str = r#"'|!|"№|;|%|:|?|*|(|)|_|+|Æ|Œ|£|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Й|Ц|У|К|Е|Н|Г|Ш|Щ|З|Х|Ї|«|»|¤
Enter-\n-13-4|Ф|І|В|А|П|Р|О|Л|Д|Ж|Є|/|©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|\\|Я|Ч|С|М|И|Т|Ь|Б|Ю|Ґ|®|µ|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Greek (GR) ──────────────────────────────────────────────────────────────

    /// Greek keyboard — normal layer (lowercase Greek).
    ///
    /// Layout maps to standard Greek ISO keyboard positions.
    /// The `;` key produces the Greek semicolon/erotimatiko (U+003B maps to `;` here).
    pub const GR: &str = r#"`|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|;|ς|ε|ρ|τ|υ|θ|ι|ο|π|[|]|«|»|\\
Enter-\n-13-4|α|σ|δ|φ|γ|η|ξ|κ|λ|΄|'|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|<|ζ|χ|ψ|ω|β|ν|μ|,|.|/|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Greek keyboard — shift layer (uppercase Greek).
    pub const GR_SHIFT: &str = r#"~|!|@|#|$|%|^|&|*|(|)|_|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|:|Σ|Ε|Ρ|Τ|Υ|Θ|Ι|Ο|Π|{|}|£|¥|\|
Enter-\n-13-4|Α|Σ|Δ|Φ|Γ|Η|Ξ|Κ|Λ|¨|"|\||©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|>|Ζ|Χ|Ψ|Ω|Β|Ν|Μ|<|>|?|®|µ|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Arabic (AR) ─────────────────────────────────────────────────────────────

    /// Arabic keyboard — normal layer.
    ///
    /// Based on the standard Arabic (101) keyboard layout.
    /// Ligatures لا لإ لأ لآ are pre-composed Unicode sequences.
    pub const AR: &str = r#"ذ|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|ض|ص|ث|ق|ف|غ|ع|ه|خ|ح|ج|د|«|»|\\
Enter-\n-13-4|ش|س|ي|ب|ل|ا|ت|ن|م|ك|ط|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|ئ|ء|ؤ|ر|لا|ى|ة|و|ز|ظ|/|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Arabic keyboard — shift layer (diacritics, special punctuation and ligatures).
    pub const AR_SHIFT: &str = r#"ّ|!|@|#|$|%|^|&|*|(|)|_|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|َ|ً|ُ|ٌ|لإ|إ|`|÷|×|؛|<|>|£|¥|¤
Enter-\n-13-4|ِ|ٍ|]|[|لأ|أ|ـ|،|/|:|"|©|€|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|~|\||{|}|لآ|آ|'|,|.|?|®|µ|$|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Hebrew (HE) ─────────────────────────────────────────────────────────────

    /// Hebrew keyboard — normal layer.
    ///
    /// Based on the standard Israeli Hebrew keyboard layout.
    /// The shift layer provides Latin/ASCII characters (standard Israeli dual layout).
    pub const HE: &str = r#";|1|2|3|4|5|6|7|8|9|0|\-|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|/|'|ק|ר|א|ט|ו|ן|ם|פ|]|[|«|»|\\
Enter-\n-13-4|ש|ד|ג|כ|ע|י|ח|ל|ך|ף|,|\\|/|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|ז|ס|ב|ה|נ|מ|צ|ת|ץ|.|\\|€|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Hebrew keyboard — shift layer (Latin/ASCII characters, standard Israeli dual layout).
    pub const HE_SHIFT: &str = r#"~|!|@|#|$|%|^|&|*|(|)|_|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|Q|W|E|R|T|Y|U|I|O|P|{|}|£|¥|¤
Enter-\n-13-4|A|S|D|F|G|H|J|K|L|:|"|/@|©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|Z|X|C|V|B|N|M|<|>|?|/@|®|µ|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3| - -32-13|Alt--18-3|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Japanese Hiragana / Katakana (JA) ───────────────────────────────────────

    /// Japanese keyboard — normal layer (hiragana basic syllabary).
    ///
    /// The 46 basic hiragana are distributed over four rows.  The remaining
    /// hiragana (れ ろ わ を ん) and the small forms (ぁ–ょ) are on the alt layer.
    pub const JA: &str = r#"あ|い|う|え|お|か|き|く|け|こ|ん|ー|っ|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|さ|し|す|せ|そ|た|ち|つ|て|と|「|」|、|。|・
Enter-\n-13-4|な|に|ぬ|ね|の|は|ひ|ふ|へ|ほ|、|。|・|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|ま|み|む|め|も|や|ゆ|よ|ら|り|る|・|〜|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Japanese keyboard — shift layer (katakana basic syllabary).
    ///
    /// Mirrors the hiragana layer exactly but uses katakana characters.
    pub const JA_SHIFT: &str = r#"ア|イ|ウ|エ|オ|カ|キ|ク|ケ|コ|ン|ー|ッ|々|Æ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|サ|シ|ス|セ|ソ|タ|チ|ツ|テ|ト|『|』|・|…|¤
Enter-\n-13-4|ナ|ニ|ヌ|ネ|ノ|ハ|ヒ|フ|ヘ|ホ|・|…|©|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_0|マ|ミ|ム|メ|モ|ヤ|ユ|ヨ|ラ|リ|ル|〜|®|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    /// Japanese keyboard — alt layer (remaining hiragana, dakuten voiced forms, small kana).
    ///
    /// Contains: れ ろ わ を small forms ぁ–ょ, voiced consonants (が–ぽ), and
    /// common Japanese punctuation/symbols.
    pub const JA_ALT: &str = r#"れ|ろ|わ|を|ぁ|ぃ|ぅ|ぇ|ぉ|ゃ|ゅ|ょ|ヴ|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
Tab-\t-9-3|が|ぎ|ぐ|げ|ご|ざ|じ|ず|ぜ|ぞ|〔|〕|【|】|¤
Enter-\n-13-4|だ|ぢ|づ|で|ど|ば|び|ぶ|べ|ぼ|！|？|%|Enter-\n-13-3
spr:sk/ui/shift--16-3-go_1|ぱ|ぴ|ぷ|ぺ|ぽ|〜|…|，|．|：|；|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;

    // ─── Locale table ────────────────────────────────────────────────────────────

    /// List all the keyboard layouts known to [`Keyboard`] as `(code, label)` pairs.
    ///
    /// The `code` is the language part of a BCP-47-like locale id (`"fr"`, `"de"`, `"pt"`...)
    /// and the `label` a human readable name suitable for UI display.  The pairs follow the
    /// order of the layout constants of this struct, and codes sharing a layout (`"sv"`,
    /// `"no"` and `"da"` all use the Nordic one) are listed as separate entries.
    ///
    /// see also [`Keyboard::apply_locale`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::tools::locales::Keyboard;
    ///
    /// let locales = Keyboard::locales();
    /// assert!(locales.contains(&("fr", "Français (AZERTY)")));
    /// assert!(locales.iter().all(|(code, _)| code.len() == 2));
    /// ```
    pub fn locales() -> Vec<(&'static str, &'static str)> {
        LOCALES.iter().map(|entry| (entry.code, entry.label)).collect()
    }

    /// Apply the keyboard layout matching `locale` — the layout layers and the platform calls
    /// are exactly those of the `locale1` demo — to the `Text` and `Password` system keyboards.
    ///
    /// `locale` is normalized before matching: it is trimmed, lower-cased and truncated at
    /// the first `'.'`, `'_'` or `'-'`, so the raw output of
    /// [`os_api::get_locale`](crate::tools::os_api::get_locale) (`"fr_FR.UTF-8"`, `"pt_BR"`)
    /// as well as bare language codes (`"fr"`, `"FR"`) are all accepted.
    ///
    /// Returns `Some(label)` when a keyboard layout matches the locale code, or `None` when
    /// no layout is known for it — in which case the keyboards are left untouched.  A failing
    /// [`Platform::keyboard_set_layout`] call does **not** turn the result into `None`; the
    /// error is reported through [`Log::err`](crate::system::Log::err) instead.
    ///
    /// see also [`Keyboard::locales`] [`Platform::keyboard_set_layout`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::locales::Keyboard;
    ///
    /// assert_eq!(Keyboard::apply_locale("fr_FR.UTF-8"), Some("Français (AZERTY)"));
    /// assert_eq!(Keyboard::apply_locale("xx"), None);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn apply_locale(locale: &str) -> Option<&'static str> {
        let entry = Self::find_entry(locale)?;

        for context in [TextContext::Text, TextContext::Password] {
            if !Platform::keyboard_set_layout(context, entry.layers) {
                Log::err(format!(
                    "Keyboard::get_locale: failed to set the {:?} keyboard for \"{}\"",
                    context, entry.label
                ));
            }
        }
        Log::info(format!("Keyboard::get_locale: keyboard layout → \"{}\"", entry.label));

        Some(entry.label)
    }

    /// Normalize `locale` and return its entry of the locale table, if the code is known.
    fn find_entry(locale: &str) -> Option<&'static LocaleEntry> {
        let lower = locale.trim().to_lowercase();
        let language = lower.split(['.', '_', '-']).next().unwrap_or_default();
        LOCALES.iter().find(|entry| entry.code == language)
    }
}
