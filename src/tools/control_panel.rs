use openxr_sys::EnvironmentBlendMode;

use crate::{
    lighting::{Lighting, LightingSource},
    maths::{Vec2, Vec3},
    permission::{Permission, PermissionState, PermissionType},
    prelude::*,
    render::Renderer,
    sk::{AppFocus, AppWindow, DisplayBlend},
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
/// * the world lighting source when the device estimates lighting from the user's surroundings, see
///   [`Lighting::is_source_available`] and [`crate::lighting::LightingSource::World`],
/// * the fullscreen and the relative mouse modes in the Simulator, with the `[Esc]` key to leave them.
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
/// * `viewport_scaling` - The viewport scaling in use, initialized with [`Renderer::get_viewport_scaling`].
/// * `fps` - The smoothed FPS shown by the panel. Default is 72.0.
/// * `simulator_fullscreen` - Whether the Simulator window should be fullscreen, initialized from the settings,
///   see [`SkInfo::settings_from`].
/// * `mouse_mode_relative` - Whether the mouse is in [`MouseMode::Relative`] in the Simulator. Default is `false`.
///
/// ### Events:
/// * `passthrough_event` - see the field of the same name, sent to the stepper it names.
/// * `world_lighting_event` - see the field of the same name, sent to the stepper it names.
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
/// from_scr = Vec3::new(0.0, 0.0, 0.35); at_scr = Vec3::new(0.0, 0.0, -0.4);
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     control_panel.update();
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
    /// Whether the Simulator window is fullscreen, initialized from the settings by [`ControlPanel::new`].
    pub simulator_fullscreen: bool,
    /// Whether the mouse is in [`MouseMode::Relative`] in the Simulator. Default is `false`.
    pub mouse_mode_relative: bool,
    /// The current viewport scaling, see [`Renderer::viewport_scaling`].
    pub viewport_scaling: f32,
    /// The smoothed FPS displayed by the panel, in frames per second (the frame time comes from
    /// [`Time::get_step`], so this is a `f64`).
    pub fps: f64,

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
    /// simultaneous hands & controllers tracking and the label of its cycling button.
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
    ///   (see [`SkInfo::settings_from`]).
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

        Self {
            passthrough: false,
            passthrough_blend_enabled,
            passthrough_event,
            world_lighting,
            world_lighting_available,
            world_lighting_event: None,
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
            last_focus: AppFocus::Background,
        }
    }

    /// Sends a [`StepperAction`] to the event loop: [`SkInfo::send_event`] takes the `Option<Rc<RefCell<SkInfo>>>`
    /// of an IStepper (which may not be initialized yet), while the panel always has its
    /// [`ControlPanel::sk_info`], see [`ControlPanel::new`].
    fn send_event(&self, action: StepperAction) {
        SkInfo::send_event(&Some(self.sk_info.clone()), action);
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

    /// To be called once per frame, before [`ControlPanel::draw`]: when the app focus changes, the display refresh
    /// rate is re-applied (a device usually forgets it when the app loses the focus) and the current interactors are
    /// logged, so the panel stays in sync with the device. The world lighting toggle also follows the source
    /// StereoKit really has (see [`Lighting::get_source`]), which settles asynchronously after a
    /// [`ControlPanel::set_world_lighting`] call.
    ///
    /// see also [`ControlPanel::draw`] [`crate::sk::sk_app_focus`]
    pub fn update(&mut self) {
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
        notif.text = "Press [Esc] key to go back to normal".into();
        self.send_event(StepperAction::add("HudNotifESC", notif));
    }

    /// Draws the control panel with the current [`Ui`] layout: the app decides where it goes (typically on the same
    /// line as its "Exit" button, see [`Ui::same_line`]), and the panel wraps its content in a
    /// [`Ui::panel_begin`]/[`Ui::panel_end`] pair.
    ///
    /// It draws:
    /// * the passthrough toggle when the device supports it (see [`ControlPanel::passthrough_blend_enabled`]) and
    ///   sends [`ControlPanel::passthrough_event`] when it changes,
    /// * the world lighting toggle when the device supports light estimation (see
    ///   [`ControlPanel::world_lighting_available`]), and sends [`ControlPanel::world_lighting_event`] when it
    ///   changes,
    /// * in the Simulator, the `fullscreen` and `mouse relative` toggles, and handles the `[Esc]` key (see
    ///   [`ControlPanel::notif_escape`]),
    /// * the smoothed FPS (see [`ControlPanel::fps`]),
    /// * the [`DefaultInteractors`] cycling button,
    /// * the display refresh rate cycling button, when the device accepts it,
    /// * the [`Renderer::viewport_scaling`] slider (see [`ControlPanel::viewport_scaling`]).
    ///
    /// see also [`ControlPanel::new`] [`ControlPanel::update`]
    pub fn draw(&mut self) {
        Ui::panel_begin(Some(UiPad::Inside));

        let mut passthrough_toggled = None;
        if self.passthrough_blend_enabled {
            passthrough_toggled = Ui::toggle("Passthrough MR", &mut self.passthrough).interact();
        }
        if let Some(new_value) = passthrough_toggled {
            self.set_passthrough(new_value);
        } else {
            match self.is_emulator {
                Some(Emulator::Simulator) => {
                    let fullscreen_toggled = Ui::toggle("fullscreen", &mut self.simulator_fullscreen).interact();
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
                    let mouse_mode_toggled = Ui::toggle("mouse relative", &mut self.mouse_mode_relative).interact();
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
            if let Some(new_value) = Ui::toggle("World lighting", &mut self.world_lighting).interact() {
                self.set_world_lighting(new_value);
            }
            if Lighting::is_source_pending() {
                // The permission dialog is in flight, the source isn't switched yet.
                Ui::same_line();
                Ui::label("(permission...)").use_padding(true).draw();
            }
        }
        if self.is_emulator.is_some() {
            Ui::same_line();
            if Ui::button("Inject F1 key").press() {
                Input::key_inject_press(Key::F1);
                Input::key_inject_release(Key::F1);
            }
        }
        Ui::same_line();
        self.fps = ((1.0 / Time::get_step()) + self.fps) / 2.0;
        Ui::label(format!("FPS: {:3.0}", self.fps)).size(Vec2::new(0.1, 0.0)).use_padding(true).draw();

        // DefaultInteractors choice - cycling button
        if self.interactor_choices.len() > 1 {
            let current = self.current_interactor_idx;
            let pressed = Ui::button(self.interactor_choices[current].2).image(&self.next_interactor_image).press();
            if pressed {
                // Deactivate simultaneous if currently active
                if self.interactor_choices[current].1 {
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
            Ui::label(self.interactor_choices[0].2).use_padding(true).draw();
        }

        Ui::same_line();

        if self.refresh_rate_editable {
            let pressed = Ui::button(format!("Up to {:?} FPS", self.current_refresh_rate as u32))
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
        Ui::label("Viewport scaling:").use_padding(true).draw();
        Ui::same_line();
        Ui::label(format!("{:.2}", self.viewport_scaling)).use_padding(true).draw();
        Ui::same_line();
        let new_scaling = Ui::hslider("scaling", &mut self.viewport_scaling, 0.1, 2.0).step(0.05).interact();
        if let Some(new_value) = new_scaling {
            Renderer::viewport_scaling(new_value);
            self.viewport_scaling = new_value;
        }

        Ui::panel_end();
    }
}
