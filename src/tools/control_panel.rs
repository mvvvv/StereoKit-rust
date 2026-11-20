use openxr_sys::EnvironmentBlendMode;

use crate::{
    lighting::{Lighting, LightingSource},
    maths::{Vec2, Vec3},
    permission::{Permission, PermissionState, PermissionType},
    prelude::*,
    render::Renderer,
    sk::{AppFocus, AppWindow, DisplayBlend, MainThreadToken},
    sprite::Sprite,
    system::{
        Backend, BackendOpenXR, BackendXRType, DefaultInteractors, Input, Interaction, Interactor, Key, MouseMode,
    },
    tools::{
        notif::HudNotification,
        os_api::get_env_blend_modes,
        xr_fb_display_refresh_rate::{
            get_all_display_refresh_rates, get_display_refresh_rate, set_display_refresh_rate,
        },
        xr_meta_simultaneous_hands_controllers::{
            is_simultaneous_hands_and_controllers_supported, pause_simultaneous_hands_and_controllers,
            resume_simultaneous_hands_and_controllers,
        },
    },
    ui::{Ui, UiPad},
    util::{Device, Emulator, Time},
};

#[cfg(feature = "locales")]
use crate::tools::ui_widgets::Dropdown;

#[cfg(target_os = "android")]
use crate::{
    tools::android_soft_kdb::{ANDROID_SOFT_KBD_ID, AndroidSoftKbd},
    util::Platform,
};

// Texts of the panel, translated with the same `locales/*.toml` catalogues as the file browser and the voice panel (see
// `tools::locales::set_locale`). The second argument is the English text used as is when the `locales` feature is off;
// the `%{name}` placeholders are replaced with the `name = value` arguments.
#[cfg(feature = "locales")]
macro_rules! cp_text {
    ($key:literal, $english:literal $(, $name:ident = $value:expr)* $(,)?) => {
        rust_i18n::t!($key $(, $name = $value)*)
    };
}

#[cfg(not(feature = "locales"))]
macro_rules! cp_text {
    ($key:literal, $english:literal) => {
        std::borrow::Cow::Borrowed($english)
    };
    ($key:literal, $english:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut text = std::borrow::Cow::Borrowed($english);
        $(
            text = std::borrow::Cow::Owned(text.replace(concat!("%{", stringify!($name), "}"), &$value.to_string()));
        )+
        text
    }};
}

/// The translated label of a [`DefaultInteractors`] choice. The English label kept in
/// [`ControlPanel::interactor_choices`] is the one used by the logs.
fn interactor_label(interactor: DefaultInteractors, simultaneous: bool) -> std::borrow::Cow<'static, str> {
    match (interactor, simultaneous) {
        (DefaultInteractors::All, true) => {
            cp_text!("control_panel.interaction_hands_controllers", "Interaction: Hands & Controllers")
        }
        (DefaultInteractors::All, false) => cp_text!("control_panel.interaction_all", "Interaction: All"),
        (DefaultInteractors::Hands, _) => cp_text!("control_panel.interaction_hands", "Interaction: Hands"),
        (DefaultInteractors::Controllers, _) => {
            cp_text!("control_panel.interaction_controllers", "Interaction: Controllers")
        }
        (DefaultInteractors::Mouse, _) => cp_text!("control_panel.interaction_mouse", "Interaction: Mouse"),
        _ => cp_text!("control_panel.interaction_default", "Interaction: Default"),
    }
}

/// The language part of a locale id like `"fr_FR.UTF-8"` or `"pt-BR"`: trimmed, lower-cased and truncated at the first
/// `'.'`, `'_'`, `'-'` or `':'` (the `LANGUAGE` variable may hold a prioritized list). An empty id stays empty.
#[cfg(feature = "locales")]
fn language_code(locale: &str) -> String {
    locale.trim().to_lowercase().split(['.', '_', '-', ':']).next().unwrap_or_default().to_string()
}

/// The English name of the language of a catalogue, if known. The locale drop-down uses it instead of the endonym the
/// catalogue gives itself (`locales.language`) because the default StereoKit font has no CJK glyphs: "日本語",
/// "한국어" and "中文" would be drawn as "??".
#[cfg(feature = "locales")]
fn locale_display_name(code: &str) -> Option<&'static str> {
    match code {
        "de" => Some("German"),
        "en" => Some("English"),
        "es" => Some("Spanish"),
        "fr" => Some("French"),
        "it" => Some("Italian"),
        "ja" => Some("Japanese"),
        "ko" => Some("Korean"),
        "pt" => Some("Portuguese"),
        "zh" => Some("Chinese"),
        _ => None,
    }
}

/// The `(code, language name)` pairs of the locale drop-down: one per catalogue of
/// [`crate::tools::locales::available_locales`], labelled with the English name of the language (see
/// [`locale_display_name`]). A catalogue without a known name falls back to its code.
#[cfg(feature = "locales")]
fn available_locale_choices() -> Vec<(String, String)> {
    crate::tools::locales::available_locales()
        .into_iter()
        .map(|code| {
            let name = locale_display_name(&code).map(str::to_string).unwrap_or_else(|| code.clone());
            (code, name)
        })
        .collect()
}

/// The language code of the system locale (see [`crate::tools::os_api::get_locale`]) when a catalogue matches it,
/// `"en"` otherwise: the locale [`ControlPanel::new`] starts with.
#[cfg(feature = "locales")]
fn system_locale() -> String {
    let code = language_code(&crate::tools::os_api::get_locale());
    if crate::tools::locales::available_locales().iter().any(|locale| locale == &code) {
        code
    } else {
        "en".to_string()
    }
}

/// The runtime control panel of an app: the controls that are handy in every build, gathered in one place and drawn
/// where the app wants it (typically on the same line as its "Exit" button).
///
/// It is **not** an IStepper: [`ControlPanel::new`] does the one time setup on the main thread, with the
/// `Rc<RefCell<SkInfo>>` of the running [`Sk`] (see [`Sk::get_sk_info_clone`]), then the app drives it:
/// * [`ControlPanel::update`] once per frame, to follow the app focus (see [`crate::sk::sk_app_focus`]),
/// * [`ControlPanel::draw`] to draw the panel itself, in the current [`Ui`] line,
/// * [`ControlPanel::notif_escape`] to warn the user when a mode is left with the `[Esc]` key.
///
/// The panel displays / controls:
/// * the FPS,
/// * the [`DefaultInteractors`] in use (cycling button, with the simultaneous hands & controllers choice when the
///   device supports it, see [`crate::tools::xr_meta_simultaneous_hands_controllers`]),
/// * the display refresh rate when the `XR_FB_display_refresh_rate` extension is available,
///   see [`crate::tools::xr_fb_display_refresh_rate`],
/// * the [`Renderer::viewport_scaling`] (reduced back to 2.0 above the value where the image gets distorted),
/// * the passthrough blend mode when the device supports it, see [`crate::tools::os_api::get_env_blend_modes`],
/// * the **Dictation** toggle when the `voice-dictation` feature is on: it shows the recording state of that panel and
///   raises its `SHOW_PARAM_WINDOW` event, so the app gets back the window its own close button hid,
/// * the world lighting source when the device estimates lighting from the user's surroundings, see
///   [`Lighting::is_source_available`] and [`crate::lighting::LightingSource::World`],
/// * the Android soft keyboard bridge on Android, see `crate::tools::android_soft_kdb`: on by default, the system
///   IME (dictation, translation, autocorrect…) mirrors the focused `Ui::input` field,
/// * the fullscreen and the relative mouse modes in the Simulator, with the `[Esc]` key to leave them.
///
/// The texts of the panel are translated with the same `locales/*.toml` catalogues as the file browser and the voice
/// panel when the `locales` feature is on (see `tools::locales::set_locale`); without it the English texts are used.
/// With the feature, an `arrow_down` drop-down button on the left of the panel lists the catalogues and selects the
/// language (see the `ControlPanel::set_locale` method), starting from the system locale.
///
/// ### Fields that can be set before drawing:
/// * `passthrough` - Whether the passthrough is active. Default is `false`: use [`ControlPanel::set_passthrough`]
///   to activate it at start.
/// * `passthrough_event` - An optional `(stepper id, event key)` event to send when the passthrough toggle changes
///   (`"true"` = activated, `"false"` = deactivated), so the app can react (e.g. hide its floor). Default is `None`.
/// * `world_lighting` - Whether the world lighting source is requested. Default is `false`: use
///   [`ControlPanel::set_world_lighting`] to request it at start.
/// * `world_lighting_event` - An optional `(stepper id, event key)` event to send when the world lighting source
///   changes (`"true"` = world requested, `"false"` = back to manual), so the app can react (e.g. reapply its own
///   ambient probe). Default is `None`.
/// * `dictation_recording` - Whether the voice dictation session records. Should be `false` at start. Only there
///   when the `voice-dictation` feature is on.
/// * `viewport_scaling` - The viewport scaling in use, initialized with [`Renderer::get_viewport_scaling`].
/// * `fps` - The smoothed FPS shown by the panel. Default is 72.0.
/// * `simulator_fullscreen` - Whether the Simulator window should be fullscreen, initialized from the settings,
///   see [`SkInfo::settings_from`].
/// * `mouse_mode_relative` - Whether the mouse is in [`MouseMode::Relative`] in the Simulator. Default is `false`.
/// * `android_keyboard` - Whether the Android soft keyboard bridge is active (Android only). Default is `true`.
///
/// ### Events:
/// * `passthrough_event` - see the field of the same name, sent to the stepper it names.
/// * `world_lighting_event` - see the field of the same name, sent to the stepper it names.
/// * the **Dictation** toggle sends `SHOW_PARAM_WINDOW` (`"true"`) to the voice dictation panel, and the panel listens to
///   the `DICTATION_STATE` it shares back (see [`ControlPanel::update`]), both when the `voice-dictation` feature is on.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Pose, Quat, Vec2, Vec3}, tools::control_panel::ControlPanel, ui::Ui};
///
/// let mut control_panel = ControlPanel::new(sk.get_sk_info_clone());
/// // Send an event to the "main" stepper when the passthrough toggle moves:
/// control_panel.passthrough_event = Some(("main".into(), "ShowFloor".into()));
///
/// let mut window_pose = Pose::new(Vec3::new(0.0, 0.0, -0.4), Some(Quat::look_dir(Vec3::Z)));
///
/// number_of_steps = 4; filename_scr = "screenshots/control_panel.jpeg"; fov_scr = 30.0;
/// from_scr = Vec3::new(0.0, 0.0, 0.4); at_scr = Vec3::new(0.0, 0.0, -0.4);
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     control_panel.update(token);
///     Ui::window("A window").pose(&mut window_pose).begin();
///     if Ui::button("Exit").size(Vec2::new(0.10, 0.10)).press() {
///         sk.send_event(StepperAction::quit("main", "Exit button pressed"));
///     }
///     // The panel is drawn on the same line as the last element, see `Ui::same_line`.
///     Ui::same_line();
///     control_panel.draw();
///     Ui::window_end();
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/control_panel.jpeg" alt="screenshot" width="200">
pub struct ControlPanel {
    /// Whether the passthrough is active. Only shown when [`ControlPanel::passthrough_blend_enabled`] is true:
    /// use [`ControlPanel::set_passthrough`] to change it and apply the display blend.
    pub passthrough: bool,
    /// `true` when the device can display a transparent blend (see [`get_env_blend_modes`]): only then does
    /// [`ControlPanel::draw`] show the passthrough toggle. Set by [`ControlPanel::new`].
    pub passthrough_blend_enabled: bool,
    /// Optional event sent when the passthrough toggle changes, as `(target stepper id, event key)`: the value sent
    /// is `"true"` when the passthrough is activated, `"false"` when it is deactivated. Default is `None`.
    pub passthrough_event: Option<(StepperId, String)>,
    /// Whether the world lighting source is requested. Only shown when [`ControlPanel::world_lighting_available`] is
    /// true: use [`ControlPanel::set_world_lighting`] to change it and request the source.
    pub world_lighting: bool,
    /// `true` when the device can estimate the scene lighting from the user's surroundings, see
    /// [`Lighting::is_source_available`]: only then does [`ControlPanel::draw`] show the world lighting toggle.
    /// Set by [`ControlPanel::new`].
    pub world_lighting_available: bool,
    /// Optional event sent when the world lighting source changes, as `(target stepper id, event key)`: the value
    /// sent is `"true"` when the world source is requested, `"false"` when the panel goes back to manual. Default is
    /// `None`.
    pub world_lighting_event: Option<(StepperId, String)>,
    /// Whether the voice dictation session records. Should be false at Start.
    /// `false`.
    #[cfg(voice_dictation)]
    pub dictation_recording: bool,
    /// Whether the Simulator window is fullscreen, initialized from the settings by [`ControlPanel::new`].
    pub simulator_fullscreen: bool,
    /// Whether the mouse is in [`MouseMode::Relative`] in the Simulator. Default is `false`.
    pub mouse_mode_relative: bool,
    /// Whether the Android soft keyboard bridge is active. When it is, the `AndroidSoftKbd` stepper mirrors the
    /// focused `Ui::input`.
    pub android_keyboard: bool,
    /// The current viewport scaling, see [`Renderer::viewport_scaling`].
    pub viewport_scaling: f32,
    /// The smoothed FPS displayed by the panel, in frames per second (the frame time comes from
    /// [`Time::get_step`], so this is a `f64`).
    pub fps: f64,
    /// The locale selected in the panel, e.g. `"fr"`: a code of `tools::locales::available_locales` whose catalogue
    /// translates the whole crate. [`ControlPanel::new`] starts with the system locale (English when no catalogue
    /// matches it); use [`ControlPanel::set_locale`] to change it, or the drop-down button drawn on the left of the
    /// panel.
    #[cfg(feature = "locales")]
    pub locale: String,

    /// The world lighting source StereoKit really had at the previous [`ControlPanel::update`], used to restore the
    /// app's environment when it is lost (see [`ControlPanel::set_world_lighting`]).
    world_lighting_active: bool,

    /// The [`SkInfo`] smart pointer of the running [`Sk`], needed to send the events of this panel (see
    /// [`SkInfo::send_event`]): [`ControlPanel::new`] receives it, see [`Sk::get_sk_info_clone`]. Unlike an IStepper
    /// of this crate, it is not an `Option`: [`ControlPanel::new`] is the only constructor and always gets one.
    sk_info: Rc<RefCell<SkInfo>>,
    /// Are we runnin an emulator/simulator ?
    is_emulator: Option<Emulator>,
    /// The [`DefaultInteractors`] choices of the current backend: the interactor set, whether it needs the
    /// simultaneous hands & controllers tracking and the English label used in the logs (the cycling button shows the
    /// text of [`interactor_label`]).
    interactor_choices: Vec<(DefaultInteractors, bool, &'static str)>,
    /// Index of the choice in use in [`ControlPanel::interactor_choices`].
    current_interactor_idx: usize,
    /// Whether the `XR_FB_display_refresh_rate` extension is available.
    refresh_rate_editable: bool,
    /// All the display refresh rates the device accepts, when [`ControlPanel::refresh_rate_editable`].
    refresh_rates: Vec<f32>,
    /// The display refresh rate in use, re-applied by [`ControlPanel::update`] on focus changes.
    current_refresh_rate: f32,
    /// The cycling button images.
    next_refresh_rate_image: Sprite,
    next_interactor_image: Sprite,
    /// The `(code, language name)` pairs of the locale drop-down, one per catalogue (see
    /// [`crate::tools::locales::available_locales`]).
    #[cfg(feature = "locales")]
    locale_choices: Vec<(String, String)>,
    /// The locale drop-down button image.
    #[cfg(feature = "locales")]
    next_locale_image: Sprite,
    /// The locale drop-down widget (open state and anchor), see [`Dropdown`].
    #[cfg(feature = "locales")]
    locale_dropdown: Dropdown,
    /// The app focus at the last call of [`ControlPanel::update`], see [`crate::sk::sk_app_focus`].
    last_focus: AppFocus,
}

impl ControlPanel {
    /// Does all the one time setup of the panel, on the main thread, after `Sk::initialize`, with the
    /// `Rc<RefCell<SkInfo>>` of the running [`Sk`] (see [`Sk::get_sk_info_clone`]) used to send the events:
    /// * sets the default interactors of the backend and activates the simultaneous hands & controllers tracking
    ///   when the device supports it (see [`Interaction::set_default_interactors`] and
    ///   [`resume_simultaneous_hands_and_controllers`]),
    /// * checks the passthrough availability (see [`ControlPanel::set_passthrough`] to activate it),
    /// * reads the display refresh rates (see [`get_all_display_refresh_rates`]),
    /// * captures [`Renderer::get_viewport_scaling`] and the `fullscreen` state of the settings
    ///   (see [`SkInfo::settings_from`]),
    /// * registers and activates the Android soft keyboard bridge on Android (see `set_android_keyboard`),
    /// * selects the system locale when the `locales` feature is on (see `ControlPanel::set_locale`).
    ///
    /// see also [`ControlPanel::update`] [`ControlPanel::draw`]
    pub fn new(sk_info: Rc<RefCell<SkInfo>>) -> Self {
        let is_emulator = Device::is_emulator();
        let passthrough_event = None;
        let simultaneous_hands_controllers_available = is_simultaneous_hands_and_controllers_supported(false);
        let simulator_fullscreen = SkInfo::settings_from(&Some(sk_info.clone())).fullscreen != 0;
        let mouse_mode_relative = false;
        let fps = 72.0;

        // First set the default interactors based on the backend, then try to activate simultaneous hand & controller if available
        // Build the list of DefaultInteractors choices based on the backend type
        let interactor_choices: Vec<(DefaultInteractors, bool, &'static str)> = {
            let xr_tp = Backend::xr_type();
            if xr_tp == BackendXRType::OpenXR {
                let mut choices: Vec<(DefaultInteractors, bool, &'static str)> = vec![
                    (DefaultInteractors::Default, false, "Interaction: Default"),
                    (DefaultInteractors::All, false, "Interaction: All"),
                ];
                if simultaneous_hands_controllers_available {
                    choices.insert(0, (DefaultInteractors::All, true, "Interaction: Hands & Controllers"));
                }
                choices.push((DefaultInteractors::Hands, false, "Interaction: Hands"));
                choices.push((DefaultInteractors::Controllers, false, "Interaction: Controllers"));
                choices
            } else {
                // Simulator only Mouse
                vec![
                    (DefaultInteractors::Default, false, "Interaction: Default"),
                    (DefaultInteractors::Mouse, false, "Interaction: Mouse"),
                ]
            }
        };
        let mut current_interactor_idx = 0usize;
        Interaction::set_default_interactors(interactor_choices[current_interactor_idx].0);

        // Activate simultaneous hand & controller
        if simultaneous_hands_controllers_available {
            Log::info("✅ Simultaneous hands and controllers tracking available");
            if interactor_choices[current_interactor_idx].1 {
                if resume_simultaneous_hands_and_controllers(sk_info.clone(), true) {
                    Log::info("Simultaneous hands and controllers tracking enabled at start");
                } else {
                    Log::err("❌ Failed to enable simultaneous hands and controllers tracking at start");
                    current_interactor_idx =
                        (current_interactor_idx + interactor_choices.len() - 1) % interactor_choices.len();
                    Interaction::set_default_interactors(interactor_choices[current_interactor_idx].0);
                }
            }
        } else {
            Log::diag("Simultaneous hands and controllers tracking not available");
        }

        let blend_modes = get_env_blend_modes(true);
        let passthrough_blend_enabled = blend_modes.contains(&EnvironmentBlendMode::ADDITIVE)
            || blend_modes.contains(&EnvironmentBlendMode::ALPHA_BLEND);
        if passthrough_blend_enabled {
            Log::info("Passthrough available, Deactived at start !!")
        } else {
            Log::diag("No Passthrough !!")
        }

        // The world lighting source needs light estimation from the XR runtime, and may still ask for a
        // permission when requested (see `Lighting::is_source_pending`).
        let world_lighting_available = Lighting::is_source_available(LightingSource::World);
        let world_lighting = Lighting::get_source() == LightingSource::World;
        if world_lighting_available {
            Log::info("✅ World lighting (light estimation) available")
        } else {
            // The source needs both the runtime's light estimation support (`XR_ANDROID_light_estimation`) and
            // the ambient estimation permission declared in the AndroidManifest, see `Permission::get_state`.
            let light_estimation_enabled = BackendOpenXR::ext_enabled("XR_ANDROID_light_estimation");
            let ambient_permission = Permission::get_state(PermissionType::AmbientEstimation);
            Log::diag("No world lighting source !!");
            if light_estimation_enabled && ambient_permission != PermissionState::Unavailable {
                // Both gates pass, so the runtime itself refuses to estimate the lighting: some devices, and
                // emulators, advertise the extension without supporting it.
                Log::diag("---- light estimation enabled and permitted, but the runtime reports no support");
            } else {
                Log::diag(format!(
                    "---- light estimation {}, ambient permission {}, reflection permission {}, emulator {:?}",
                    if light_estimation_enabled { "enabled" } else { "not enabled" },
                    ambient_permission,
                    Permission::get_state(PermissionType::ReflectionEstimation),
                    Device::is_emulator(),
                ));
            }
        }

        let next_refresh_rate_image = Sprite::arrow_right();
        let mut current_refresh_rate = get_display_refresh_rate().unwrap_or(0.0);
        let mut refresh_rates = vec![];
        let refresh_rate_editable = BackendOpenXR::ext_enabled("XR_FB_display_refresh_rate");
        if refresh_rate_editable {
            refresh_rates = get_all_display_refresh_rates(true);
            // Initialize current_refresh_rate with the maximum available refresh rate
            if let Some(&max_rate) = refresh_rates.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
                current_refresh_rate = max_rate;
            }
            Log::info(format!("Initial display rate is {current_refresh_rate:?}"));
        } else {
            Log::diag("No editable refresh rate !");
        }

        let next_interactor_image = Sprite::arrow_right();

        let viewport_scaling = Renderer::get_viewport_scaling();

        #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
        let mut control_panel = Self {
            passthrough: false,
            passthrough_blend_enabled,
            passthrough_event,
            world_lighting,
            world_lighting_available,
            world_lighting_event: None,
            #[cfg(voice_dictation)]
            dictation_recording: false,
            simulator_fullscreen,
            mouse_mode_relative,
            viewport_scaling,
            fps,

            world_lighting_active: world_lighting,
            sk_info,
            is_emulator,
            interactor_choices,
            current_interactor_idx,
            refresh_rate_editable,
            refresh_rates,
            current_refresh_rate,
            next_refresh_rate_image,
            next_interactor_image,
            #[cfg(feature = "locales")]
            locale: String::new(),
            #[cfg(feature = "locales")]
            locale_choices: available_locale_choices(),
            #[cfg(feature = "locales")]
            next_locale_image: Sprite::arrow_down(),
            #[cfg(feature = "locales")]
            locale_dropdown: Dropdown::default(),
            last_focus: AppFocus::Background,
            android_keyboard: false,
        };

        // The Android soft keyboard bridge is activated by default: the system IME mirrors the focused `Ui::input`
        // field, which brings dictation, translation and autocorrect to the app.
        #[cfg(target_os = "android")]
        control_panel.set_android_keyboard(true);

        // The panel starts with the language of the system, when a catalogue matches it.
        #[cfg(feature = "locales")]
        control_panel.set_locale(&system_locale());

        control_panel
    }

    /// Sends a [`StepperAction`] to the event loop: [`SkInfo::send_event`] takes the `Option<Rc<RefCell<SkInfo>>>`
    /// of an IStepper (which may not be initialized yet), while the panel always has its
    /// [`ControlPanel::sk_info`], see [`ControlPanel::new`].
    fn send_event(&self, action: StepperAction) {
        SkInfo::send_event(&Some(self.sk_info.clone()), action);
    }

    /// Selects `locale` — a code of `tools::locales::available_locales`, e.g. `"fr"` — for the whole crate: it is
    /// remembered by [`ControlPanel::locale`] and applied with `tools::locales::set_locale`, so the panel and the other
    /// tools of the app draw their texts in that language from the next frame. An unknown code is still applied:
    /// rust-i18n then falls back to English before returning the key itself.
    ///
    /// [`ControlPanel::new`] starts with the system locale (see the `system_locale` helper) and
    /// [`ControlPanel::draw`] calls this function when the locale drop-down selects another language.
    ///
    /// see also [`ControlPanel::locale`] [`ControlPanel::draw`]
    #[cfg(feature = "locales")]
    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
        crate::tools::locales::set_locale(locale);
        Log::info(format!("Control panel locale set to \"{}\"", self.locale));
    }

    /// Activates (`on` = `true`) or deactivates the `AndroidSoftKbd` stepper: when it is active, the focused
    /// `Ui::input` field is mirrored into a hidden `EditText`, and the Android IME (dictation, translation,
    /// autocorrect, emojis…) edits it while StereoKit's own fallback keyboard is dismissed. When it is off, the
    /// fallback keyboard comes back. The state is remembered by [`ControlPanel::android_keyboard`].
    ///
    /// [`ControlPanel::new`] activates it at start up (on Android only), and [`ControlPanel::draw`] shows a toggle
    /// calling this function. Nothing is done when the state does not change.
    ///
    /// see also [`ControlPanel::new`] [`ControlPanel::draw`]
    #[cfg(target_os = "android")]
    pub fn set_android_keyboard(&mut self, on: bool) {
        if self.android_keyboard == on {
            return;
        }
        self.android_keyboard = on;
        if on {
            Log::info("Android soft keyboard bridge activated");
            Platform::force_fallback_keyboard(false);
            self.send_event(StepperAction::add_default::<AndroidSoftKbd>(ANDROID_SOFT_KBD_ID));
        } else {
            Log::info("Android soft keyboard bridge deactivated, back to the StereoKit keyboard");
            Platform::force_fallback_keyboard(true);
            self.send_event(StepperAction::remove(ANDROID_SOFT_KBD_ID));
        }
    }

    /// Activates (`active` = `true`) or deactivates the passthrough: the state is remembered by
    /// [`ControlPanel::passthrough`] for the toggle of [`ControlPanel::draw`], the display blend is changed (see
    /// [`Device::display_blend`]) and [`ControlPanel::passthrough_event`] is sent. Nothing is done when the device
    /// has no transparent blend (see [`ControlPanel::passthrough_blend_enabled`]), except remembering the value.
    ///
    /// see also [`ControlPanel::new`] [`ControlPanel::draw`]
    pub fn set_passthrough(&mut self, active: bool) {
        self.passthrough = active;
        if !self.passthrough_blend_enabled {
            Log::diag("Passthrough is not available on this device");
            return;
        }
        if active {
            Log::info("Passthrough Activated");
            Device::display_blend(DisplayBlend::AnyTransparent);
        } else {
            Log::info("Passthrough Deactived");
            Device::display_blend(DisplayBlend::Opaque);
        }
        if let Some((target, key)) = &self.passthrough_event {
            // The event sends "false" when the passthrough is activated (e.g. to hide a floor), "true" otherwise.
            self.send_event(StepperAction::event(target.as_str(), key.as_str(), if active { "false" } else { "true" }));
        }
    }

    /// Rebuilds the scene lighting from the skybox that is still displayed, see [`ControlPanel::set_world_lighting`]:
    /// leaving the world source stops the estimation but keeps its last estimate, so the app's environment has to be
    /// applied again.
    fn restore_environment(&self) {
        let sky = Renderer::get_skybox_tex();
        Lighting::set_environment(Some(&sky));
    }

    /// Requests (`active` = `true`) or leaves the world lighting source, where the device estimates the scene
    /// lighting from the user's surroundings (see [`LightingSource::World`]): the state is remembered by
    /// [`ControlPanel::world_lighting`] for the toggle of [`ControlPanel::draw`], [`Lighting::request_source`] is
    /// called and [`ControlPanel::world_lighting_event`] is sent. Nothing is done when the device has no light
    /// estimation (see [`ControlPanel::world_lighting_available`]), except remembering the value.
    ///
    /// Requesting the world source may show a permission dialog: the source only switches once it is granted, check
    /// [`Lighting::is_source_pending`]. [`ControlPanel::update`] keeps [`ControlPanel::world_lighting`] in sync with
    /// the source StereoKit really has. Leaving the world source also reapplies the app's environment (from the
    /// skybox that is still displayed) when it was really in use, since the world source's estimates would otherwise
    /// stay in place.
    ///
    /// see also [`ControlPanel::new`] [`ControlPanel::draw`] [`Lighting::get_source`]
    pub fn set_world_lighting(&mut self, active: bool) {
        self.world_lighting = active;
        if !self.world_lighting_available {
            Log::diag("World lighting is not available on this device");
            return;
        }
        if active {
            // Asynchronous: a permission dialog may still be in flight (see `Lighting::is_source_pending`).
            Lighting::request_source(LightingSource::World);
            Log::info("World lighting requested");
        } else {
            let was_world = Lighting::get_source() == LightingSource::World;
            Lighting::request_source(LightingSource::Manual);
            Log::info("World lighting deactivated");
            // The manual source applies immediately.
            self.world_lighting_active = false;
            if was_world {
                // The world source overrode the environment, and leaving it doesn't restore it: rebuild the
                // lighting from the sky that is still displayed.
                self.restore_environment();
            }
        }
        if let Some((target, key)) = &self.world_lighting_event {
            self.send_event(StepperAction::event(target.as_str(), key.as_str(), if active { "true" } else { "false" }));
        }
    }

    /// To be called once per frame, before [`ControlPanel::draw`], with the [`MainThreadToken`] of the step: when the
    /// app focus changes, the display refresh rate is re-applied (a device usually forgets it when the app loses the
    /// focus) and the current interactors are logged, so the panel stays in sync with the device. The world lighting
    /// toggle also follows the source StereoKit really has (see [`Lighting::get_source`]), which settles asynchronously
    /// after a [`ControlPanel::set_world_lighting`] call.
    ///
    /// It also listens to the recording state the voice dictation panel shares, when the `voice-dictation` feature is
    /// on: `DICTATION_STATE` sent on its `VOICE_PANEL_ID` sets `ControlPanel::dictation_recording`, which picks the icon
    /// of the **Dictation** button.
    ///
    /// see also [`ControlPanel::draw`] [`crate::sk::sk_app_focus`]
    pub fn update(&mut self, token: &MainThreadToken) {
        // The voice dictation panel shares whether it records by event: the Dictation button shows it.
        #[cfg(voice_dictation)]
        for event in token.get_event_report() {
            if let StepperAction::Event(stepper_id, key, value) = event
                && stepper_id == crate::tools::voice_dictation::VOICE_PANEL_ID
                && key == crate::tools::voice_dictation::DICTATION_STATE
            {
                self.dictation_recording = value == "true";
            }
        }
        // Without the tool (feature off, or target without sherpa-onnx), the token is only there to keep a single
        // call shape.
        #[cfg(not(voice_dictation))]
        let _ = token;

        // The locale can also be changed outside the panel (`tools::locales::set_locale`): the drop-down button
        // follows the language really in use.
        #[cfg(feature = "locales")]
        {
            let current = crate::tools::locales::locale();
            if self.locale != current {
                self.locale = current;
            }
        }

        let app_focus = unsafe { crate::sk::sk_app_focus() };
        if self.last_focus != app_focus {
            self.last_focus = app_focus;
            Log::info(format!("App focus changed to : {:?}", self.last_focus));

            if !set_display_refresh_rate(self.current_refresh_rate, true) {
                self.current_refresh_rate = 0.0;
            }

            Log::diag("Current Interactors after focus change:");
            for interactor in Interactor::all() {
                Log::diag(format!("----Type: {:?} / {:?}", interactor.get_type(), interactor.get_source()));
            }
        }

        // The world lighting source settles asynchronously (typically a permission dialog), and may also be
        // changed outside the panel: follow the source StereoKit really has.
        if self.world_lighting_available && !Lighting::is_source_pending() {
            if Lighting::get_source() == LightingSource::World {
                self.world_lighting = true;
                self.world_lighting_active = true;
            } else {
                if self.world_lighting_active {
                    // The source was lost, or left outside the panel: its estimates would otherwise stay in place.
                    Log::warn("World lighting stopped, restoring the app environment");
                    self.restore_environment();
                } else if self.world_lighting {
                    // The request settled back to manual, e.g. the permission was declined.
                    Log::warn("World lighting request settled back to manual");
                }
                self.world_lighting = false;
                self.world_lighting_active = false;
            }
        }
    }

    /// Sends (or refreshes) the "Press `[Esc]` key to go back to normal" HUD notification, see [`HudNotification`]:
    /// [`ControlPanel::draw`] sends it each time the `[Esc]` key leaves the fullscreen or the relative mouse mode.
    ///
    /// see also [`ControlPanel::draw`]
    pub fn notif_escape(&mut self) {
        let mut notif = HudNotification::default();
        notif.duration = Some(5.0);
        notif.position = Vec3::new(0.0, 0.1, -0.6);
        notif.text = cp_text!("control_panel.notif_escape", "Press [Esc] key to go back to normal").into_owned();
        self.send_event(StepperAction::add("HudNotifESC", notif));
    }

    /// Draws the locale drop-down button, on the left of the panel: the `arrow_down` icon and the language name of
    /// [`ControlPanel::locale`]. It is a [`Dropdown`], whose list is drawn by [`ControlPanel::draw_locale_list`] after
    /// the panel so it floats over its content.
    #[cfg(feature = "locales")]
    fn draw_locale_button(&mut self) {
        let label = self
            .locale_choices
            .iter()
            .find(|(code, _)| code == &self.locale)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| self.locale.to_uppercase());
        self.locale_dropdown.button("cp_locale", &label, Some(&self.next_locale_image));
    }

    /// Draws the list of the locale drop-down when it is open, above the panel: pressing a language calls
    /// [`ControlPanel::set_locale`], which retranslates the whole panel from the next frame.
    #[cfg(feature = "locales")]
    fn draw_locale_list(&mut self) {
        let labels: Vec<&str> = self.locale_choices.iter().map(|(_, label)| label.as_str()).collect();
        if let Some(index) = self.locale_dropdown.list("cp_locale", &labels)
            && let Some((code, _)) = self.locale_choices.get(index)
        {
            let code = code.clone();
            self.set_locale(&code);
        }
    }

    /// Draws the control panel with the current [`Ui`] layout: the app decides where it goes (typically on the same
    /// line as its "Exit" button, see [`Ui::same_line`]), and the panel wraps its content in a
    /// [`Ui::panel_begin`]/[`Ui::panel_end`] pair.
    ///
    /// It draws:
    /// * the locale drop-down when the `locales` feature is on: an `arrow_down` button with the current language name,
    ///   listing the catalogues (see `ControlPanel::set_locale`),
    /// * the passthrough toggle when the device supports it (see [`ControlPanel::passthrough_blend_enabled`]) and
    ///   sends [`ControlPanel::passthrough_event`] when it changes,
    /// * the world lighting toggle when the device supports light estimation (see
    ///   [`ControlPanel::world_lighting_available`]), and sends [`ControlPanel::world_lighting_event`] when it
    ///   changes,
    /// * in the Simulator, the `fullscreen` and `mouse relative` toggles, and handles the `[Esc]` key (see
    ///   [`ControlPanel::notif_escape`]),
    /// * the **Dictation** toggle when the `voice-dictation` feature is on: it shows the recording state of that panel
    ///   and raises its `SHOW_PARAM_WINDOW` event, so the app gets back the window its own close button hid,
    /// * the smoothed FPS (see [`ControlPanel::fps`]),
    /// * the [`DefaultInteractors`] cycling button,
    /// * the display refresh rate cycling button, when the device accepts it,
    /// * the [`Renderer::viewport_scaling`] slider (see [`ControlPanel::viewport_scaling`]).
    ///
    /// see also [`ControlPanel::new`] [`ControlPanel::update`]
    pub fn draw(&mut self) {
        Ui::panel_begin(Some(UiPad::Inside));

        // The locale drop-down comes first, on the left of the panel.
        #[cfg(feature = "locales")]
        {
            self.draw_locale_button();
            Ui::same_line();
        }

        let mut passthrough_toggled = None;
        if self.passthrough_blend_enabled {
            passthrough_toggled =
                Ui::toggle(cp_text!("control_panel.passthrough_mr", "Passthrough MR"), &mut self.passthrough)
                    .interact();
        }
        if let Some(new_value) = passthrough_toggled {
            self.set_passthrough(new_value);
        } else {
            match self.is_emulator {
                Some(Emulator::Simulator) => {
                    let fullscreen_toggled =
                        Ui::toggle(cp_text!("control_panel.fullscreen", "fullscreen"), &mut self.simulator_fullscreen)
                            .interact();
                    if let Some(new_value) = fullscreen_toggled {
                        if let Some(window) = AppWindow::main() {
                            window.request_fullscreen(new_value);
                            Log::info(format!("Simulator fullscreen: {}", self.simulator_fullscreen));
                            if self.simulator_fullscreen {
                                self.notif_escape()
                            }
                        } else {
                            Log::warn("Unable to get AppWindow!");
                        }
                    }
                    Ui::same_line();
                    let mouse_mode_toggled = Ui::toggle(
                        cp_text!("control_panel.mouse_relative", "mouse relative"),
                        &mut self.mouse_mode_relative,
                    )
                    .interact();
                    if let Some(new_value) = mouse_mode_toggled {
                        if new_value {
                            Input::mouse_mode(MouseMode::Relative);
                            Log::info("Mouse mode <Relative>");
                            self.notif_escape()
                        } else {
                            Input::mouse_mode(MouseMode::Normal);
                            Log::info("Mouse mode <Normal>");
                        }
                    }

                    if Input::key(Key::Esc).is_just_active() {
                        self.simulator_fullscreen = false;
                        if let Some(window) = AppWindow::main() {
                            window.request_fullscreen(self.simulator_fullscreen);
                            Log::info(format!("Simulator fullscreen: {}", self.simulator_fullscreen));
                        }

                        self.mouse_mode_relative = false;
                        Input::mouse_mode(MouseMode::Normal);
                        Log::info("Mouse mode <Normal> (ESC key)");
                    }
                }
                _ => (),
            }
        }
        if self.world_lighting_available {
            Ui::same_line();
            if let Some(new_value) =
                Ui::toggle(cp_text!("control_panel.world_lighting", "World lighting"), &mut self.world_lighting)
                    .interact()
            {
                self.set_world_lighting(new_value);
            }
            if Lighting::is_source_pending() {
                // The permission dialog is in flight, the source isn't switched yet.
                Ui::same_line();
                Ui::label(cp_text!("control_panel.permission_pending", "(permission...)")).use_padding(true).draw();
            }
        }
        if self.is_emulator.is_some() {
            Ui::same_line();
            if Ui::button(cp_text!("control_panel.inject_f1", "Inject F1 key")).press() {
                Input::key_inject_press(Key::F1);
                Input::key_inject_release(Key::F1);
            }
        }

        Ui::same_line();
        self.fps = ((1.0 / Time::get_step()) + self.fps) / 2.0;
        Ui::label(cp_text!("control_panel.fps", "FPS: %{fps}", fps = format!("{:3.0}", self.fps)))
            .size(Vec2::new(0.1, 0.0))
            .use_padding(true)
            .draw();

        // The Android soft keyboard bridge, on the left of the interactors: toggled off, StereoKit's own fallback
        // keyboard comes back.
        #[cfg(target_os = "android")]
        {
            // The toggle writes into a copy: `android_keyboard` keeps the state in
            // use, which is what `set_android_keyboard` compares to know if it has
            // something to do.
            let mut on = self.android_keyboard;
            if Ui::toggle(cp_text!("control_panel.android_keyboard", "Android Keyboard"), &mut on)
                .interact()
                .is_some()
            {
                self.set_android_keyboard(on);
            }
            Ui::same_line();
        }
        // The Dictation toggle shows the recording state the voice dictation panel shares.
        #[cfg(voice_dictation)]
        {
            use crate::tools::voice_dictation::{SHOW_PARAM_WINDOW, VOICE_PANEL_ID};

            let mut shown = self.dictation_recording;
            if Ui::toggle(cp_text!("control_panel.dictation", "Dictation"), &mut shown).interact().is_some() {
                self.send_event(StepperAction::event(VOICE_PANEL_ID, SHOW_PARAM_WINDOW, "true"));
            }
            Ui::same_line();
        }
        // DefaultInteractors choice - cycling button
        if self.interactor_choices.len() > 1 {
            let current = self.current_interactor_idx;
            let (current_interactor, current_simultaneous, _) = self.interactor_choices[current];
            let pressed = Ui::button(interactor_label(current_interactor, current_simultaneous))
                .image(&self.next_interactor_image)
                .press();
            if pressed {
                // Deactivate simultaneous if currently active
                if current_simultaneous {
                    pause_simultaneous_hands_and_controllers(self.sk_info.clone(), true);
                }
                // Cycle to next
                self.current_interactor_idx = (current + 1) % self.interactor_choices.len();
                let (new_interactor, new_simultaneous, new_label) =
                    self.interactor_choices[self.current_interactor_idx];
                Interaction::set_default_interactors(new_interactor);
                if new_simultaneous && !resume_simultaneous_hands_and_controllers(self.sk_info.clone(), true) {
                    Log::err("Failed to enable simultaneous hands and controllers tracking");
                    // Fall back to previous choice
                    self.current_interactor_idx = (self.current_interactor_idx + self.interactor_choices.len() - 1)
                        % self.interactor_choices.len();
                    Interaction::set_default_interactors(self.interactor_choices[self.current_interactor_idx].0);
                }
                Log::info(format!("Interactors set to: {new_label}"));
            }
        } else {
            let (interactor, simultaneous, _) = self.interactor_choices[0];
            Ui::label(interactor_label(interactor, simultaneous)).use_padding(true).draw();
        }

        Ui::same_line();

        if self.refresh_rate_editable {
            let pressed = Ui::button(cp_text!(
                "control_panel.up_to_fps",
                "Up to %{rate} FPS",
                rate = self.current_refresh_rate as u32
            ))
            .image(&self.next_refresh_rate_image)
            .press();
            if pressed {
                let mut restart = true;
                for rate in &self.refresh_rates {
                    if *rate > self.current_refresh_rate {
                        self.current_refresh_rate = *rate;
                        restart = false;
                        break;
                    }
                }
                if restart {
                    self.current_refresh_rate = self.refresh_rates[0]
                }
                if !set_display_refresh_rate(self.current_refresh_rate, true) {
                    self.current_refresh_rate = 0.0;
                }
            }
        }

        Ui::next_line();
        Ui::label(cp_text!("control_panel.viewport_scaling", "Viewport scaling:")).use_padding(true).draw();
        Ui::same_line();
        Ui::label(format!("{:.2}", self.viewport_scaling)).use_padding(true).draw();
        Ui::same_line();
        let new_scaling = Ui::hslider("scaling", &mut self.viewport_scaling, 0.1, 2.0).step(0.05).interact();
        if let Some(new_value) = new_scaling {
            Renderer::viewport_scaling(new_value);
            self.viewport_scaling = new_value;
        }

        Ui::panel_end();

        #[cfg(feature = "locales")]
        self.draw_locale_list();
    }
}

#[cfg(all(test, feature = "locales"))]
mod tests {
    use super::*;

    /// The texts of the panel come from the `control_panel` section of the catalogues: every locale has all of them,
    /// and a translated locale really changes them.
    #[test]
    fn control_panel_texts_are_translated() {
        macro_rules! assert_keys {
            ($($key:literal),+ $(,)?) => {
                for locale in crate::tools::locales::available_locales() {
                    $(
                        let text = rust_i18n::t!($key, locale = locale.as_str());
                        assert_ne!(text, $key, "no {} key in the locale {locale}", $key);
                    )+
                }
            };
        }

        assert_keys!(
            "control_panel.passthrough_mr",
            "control_panel.fullscreen",
            "control_panel.mouse_relative",
            "control_panel.world_lighting",
            "control_panel.permission_pending",
            "control_panel.inject_f1",
            "control_panel.dictation",
            "control_panel.fps",
            "control_panel.android_keyboard",
            "control_panel.up_to_fps",
            "control_panel.viewport_scaling",
            "control_panel.interaction_default",
            "control_panel.interaction_all",
            "control_panel.interaction_hands_controllers",
            "control_panel.interaction_hands",
            "control_panel.interaction_controllers",
            "control_panel.interaction_mouse",
            "control_panel.notif_escape",
        );
        assert_eq!(rust_i18n::t!("control_panel.dictation", locale = "fr"), "Dictée");
        assert_eq!(rust_i18n::t!("control_panel.fps", locale = "fr", fps = 72), "FPS : 72");
        // The cycling button of the interactors really goes through the catalogue.
        assert_eq!(interactor_label(DefaultInteractors::Hands, false), "Interaction: Hands");
        assert_eq!(interactor_label(DefaultInteractors::All, true), "Interaction: Hands & Controllers");
    }

    /// The language part of the system locale ids the panel accepts.
    #[test]
    fn locale_codes_are_normalized() {
        assert_eq!(language_code("fr_FR.UTF-8"), "fr");
        assert_eq!(language_code(" pt-BR "), "pt");
        assert_eq!(language_code("ja_JP.eucJP"), "ja");
        assert_eq!(language_code("FR"), "fr");
        assert_eq!(language_code("fr:de:es"), "fr");
        assert_eq!(language_code(""), "");
    }

    /// The drop-down lists every catalogue, labelled with an ASCII language name.
    #[test]
    fn locale_choices_are_the_catalogues() {
        let choices = available_locale_choices();
        let codes: Vec<String> = choices.iter().map(|(code, _)| code.clone()).collect();
        assert_eq!(codes, crate::tools::locales::available_locales());
        assert_eq!(choices.iter().find(|(code, _)| code == "fr").map(|(_, label)| label.as_str()), Some("French"));
        for (code, label) in &choices {
            assert!(!label.is_empty(), "no language name for {code}");
            // The default StereoKit font has no CJK glyphs: the labels must stay ASCII.
            assert!(label.is_ascii(), "the {code} name is not ASCII: {label}");
        }
    }
}

#[cfg(all(test, not(feature = "locales")))]
mod fallback_tests {
    /// Without the `locales` feature the panel keeps its English texts, `%{name}` placeholders included.
    #[test]
    fn cp_text_falls_back_to_english() {
        assert_eq!(cp_text!("control_panel.passthrough_mr", "Passthrough MR"), "Passthrough MR");
        assert_eq!(cp_text!("control_panel.fps", "FPS: %{fps}", fps = format!("{:3.0}", 72.0)), "FPS:  72");
        assert_eq!(cp_text!("control_panel.up_to_fps", "Up to %{rate} FPS", rate = 90u32), "Up to 90 FPS");
    }
}
