//! Hot reloading of one project plugin, inside one running StereoKit session.
//!
//! [`HotReloading`] is an [`IStepper`] keeping **one** session alive while the plugin library of the project under
//! development is rebuilt: the plugin is swapped **without closing the session**, and its views (the `Test`s) are
//! offered in a selector window. The `hot reloading` viewer is exactly this, plus the command line parsing.
//!
//! The plugin is a `cdylib` built from the same project, with the same version of `stereokit-rust` and the
//! `skc-shared` feature (both sides then link the same shared StereoKitC library and drive ONE engine, through the
//! 100% C boundary of [`crate::plugin_abi`]). Its `SkSettings` (`sk_run_sk_settings`, exposed by `src/plugin_shim.rs`)
//! must be read *before* `Sk::init`, and a plugin that is not loadable yet is only a warning:
//!
//! ```text
//! let mut hot_reloading = HotReloading::auto_detect();
//! let mut settings = SkSettings::default();
//! if let Err(err) = hot_reloading.apply_plugin_settings(&mut settings) {
//!     Log::warn(format!("hot_reloading: {err}"));
//! }
//! let mut sk = settings.init().expect("cannot initialize StereoKit");
//! sk.send_event(StepperAction::add("hot_reloading", hot_reloading));
//! SkClosures::new(sk, |_sk, _token| {}).run();
//! ```

use std::{
    cell::RefCell,
    ffi::{CStr, c_char, c_void},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::mpsc::{Receiver, Sender, TryRecvError, channel},
    thread,
    time::{Duration, SystemTime},
};

use libloading::{Library, Symbol};

use crate::{
    framework::{Appearence, IStepper, StepperId},
    sk::{MainThreadToken, SkInfo, SkSettings},
    tools::build_tools::get_cargo_name_in,
};

use crate::{
    framework::StepperAction,
    maths::{Pose, Quat, Vec2, Vec3},
    plugin_abi::{PluginViewInfo, SK_RUN_SK_ABI_VERSION},
    render::Renderer,
    system::Log,
    ui::{Ui, UiPad},
    util::Time,
};

/// Hot reloads the plugin library of **one** project inside the running StereoKit session: build, watch, load,
/// swap, select and capture are driven by this [`IStepper`], see the [module documentation](self).
pub struct HotReloading {
    /// The compiled plugin library of the project, see [`plugin_file`]. The builders below configure the rest.
    lib_path: PathBuf,
    build_cmd: Option<String>,
    watch_secs: f32,
    watch_roots: Vec<PathBuf>,
    start_view: Option<String>,
    test_steps: u32,
    show_ui: bool,
    appearence: Option<Appearence>,
    /// The pose of the selector window, see [`HotReloading::window_pose`].
    window_pose: Pose,
    /// The id of this stepper.
    id: StepperId,
    /// The [`SkInfo`] of the running session, handed to the plugin, see [`HotReloading::sk_info_ptr`].
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    /// The runtime state of the tool.
    loaded: Loaded,
}

// SAFETY: the `Rc<RefCell<SkInfo>>` and the loaded plugin are only ever touched from the main thread (IStepper
// initialize, step and shutdown are main thread only), but a stepper must be `Send` to be added to the session.
unsafe impl Send for HotReloading {}

impl HotReloading {
    /// Creates a tool for the COMPILED plugin library at `lib_path` (not a Rust source file). The auto-build is
    /// disabled until [`HotReloading::build_cmd`] is called: see [`HotReloading::auto_detect`].
    pub fn new(lib_path: impl AsRef<Path>) -> Self {
        Self {
            lib_path: lib_path.as_ref().to_path_buf(),
            build_cmd: None,
            watch_secs: 0.5,
            watch_roots: default_watch_roots(),
            start_view: None,
            test_steps: 0,
            show_ui: true,
            appearence: None,
            window_pose: Pose::new(Vec3::new(0.4, 1.35, -0.35), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            id: "hot_reloading".to_string(),
            sk_info: None,
            loaded: Loaded::default(),
        }
    }

    /// Creates a tool for the project of the current directory, see [`default_lib_path`] and [`default_build_cmd`].
    pub fn auto_detect() -> Self {
        Self::auto_detect_in(Path::new("."))
    }

    /// Creates a tool for the project at `dir`, see [`default_lib_path_in`] and [`default_build_cmd_in`].
    pub fn auto_detect_in(dir: impl AsRef<Path>) -> Self {
        let lib_path = default_lib_path_in(dir.as_ref());
        let build_cmd = default_build_cmd_in(dir.as_ref());
        if dir.as_ref() != Path::new(".") {
            return Self::new(lib_path).build_cmd(format!("cd \"{}\" && {build_cmd}", dir.as_ref().display()));
        }
        Self::new(lib_path).build_cmd(build_cmd)
    }

    /// Sets the build command run at startup then on each source change (through the `sh -c` / `cmd /C` shell of the
    /// platform). Build errors are never fatal: they are shown in the selector window and the last working plugin
    /// stays loaded.
    /// * `command` - The command line, for example `cargo build --lib --features skc-shared`.
    pub fn build_cmd(mut self, command: impl Into<String>) -> Self {
        self.build_cmd = Some(command.into());
        self
    }

    /// Disables the auto-build: no source watch, no build command. The plugin library is still watched (see
    /// [`HotReloading::watch`]), so a build made by hand is picked up.
    pub fn no_build(mut self) -> Self {
        self.build_cmd = None;
        self.watch_roots.clear();
        self
    }

    /// Sets the plugin library watch period in seconds, 0 disables the watch (default 0.5).
    pub fn watch(mut self, secs: f32) -> Self {
        self.watch_secs = secs;
        self
    }

    /// Sets the files and directories scanned to detect a source change (the default is [`default_watch_roots`]).
    /// * `roots` - The files and directories to scan.
    pub fn watch_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.watch_roots = roots;
        self
    }

    /// Sets the view to select at start, and after each reload.
    /// * `name` - The exact name of the view, its name ignoring the case, or a substring of its name.
    pub fn start_view(mut self, name: impl Into<String>) -> Self {
        self.start_view = Some(name.into());
        self
    }

    /// Sets the test mode: after `steps` frames the active view is screenshotted (`screenshots/run_sk_<view>.jpeg`)
    /// and the app is asked to quit one frame later. 0 disables it (default), and the first view is selected at start
    /// when none is active.
    /// * `steps` - The number of steps.
    pub fn test_steps(mut self, steps: u32) -> Self {
        self.test_steps = steps;
        self
    }

    /// Shows or hides the selector window (shown by default); hiding it is useful for a test run.
    /// * `show` - `false` to hide the selector window.
    pub fn show_ui(mut self, show: bool) -> Self {
        self.show_ui = show;
        self
    }

    /// Replaces the look of the selector window (window size, ui scale, text styles, tints...).
    /// * `appearence` - The look of the selector window.
    pub fn appearence(mut self, appearence: Appearence) -> Self {
        self.appearence = Some(appearence);
        self
    }

    /// Sets the pose of the selector window (by default facing the user, slightly to the right).
    /// * `pose` - The pose of the selector window.
    pub fn window_pose(mut self, pose: Pose) -> Self {
        self.window_pose = pose;
        self
    }

    /// Reads the [`SkSettings`] of the project from its plugin (`sk_run_sk_settings` -> `sk_settings()`, extension
    /// requests included) and **replaces** `settings` with them before `Sk::init`. The plugin is kept loaded and
    /// *begun* at the first frame. Returns `Err(reason)`, with `settings` untouched, when the plugin is not loadable
    /// yet.
    pub fn apply_plugin_settings(&mut self, settings: &mut SkSettings) -> Result<(), String> {
        let mut copy_counter = self.loaded.copy_counter;
        match load_plugin(None, &self.lib_path, &mut copy_counter) {
            Ok(plugin) => {
                self.loaded.copy_counter = copy_counter;
                if plugin.settings(settings) == 0 {
                    self.loaded.preloaded = Some(plugin);
                    Log::info("hot_reloading: session settings provided by the plugin (sk_run_sk_settings).");
                    Ok(())
                } else {
                    drop(plugin);
                    cleanup_plugin_copies(None);
                    Err("sk_run_sk_settings failed, the session starts with the host settings".to_string())
                }
            }
            Err(err) => {
                self.loaded.copy_counter = copy_counter;
                cleanup_plugin_copies(None);
                Err(format!("plugin not ready yet ({err}); the session starts with the host settings"))
            }
        }
    }

    /// Reads the views (the `Test`s) of the plugin at `lib_path` without any session: load, view list, unload. The
    /// flag of each view tells that a screenshot file already exists. What the `--list` option of a viewer needs.
    /// * `lib_path` - The COMPILED plugin library.
    pub fn list_views(lib_path: impl AsRef<Path>) -> Result<Vec<(String, bool)>, String> {
        let mut copy_counter = 0;
        let plugin = load_plugin(None, lib_path.as_ref(), &mut copy_counter)?;
        Ok(plugin.views.iter().map(|view| (view.name.clone(), view.has_screenshot)).collect())
    }

    // ------------------------------------------------------------------
    // Main thread workflow
    // ------------------------------------------------------------------
    /// The opaque pointer handed to the plugin (`sk_run_sk_begin`, `sk_run_sk_step`): a pointer to the
    /// `Rc<RefCell<SkInfo>>` of the session, which lives at least as long as this stepper (the plugin clones it, see
    /// [`crate::plugin_abi`]).
    fn sk_info_ptr(&self) -> *mut c_void {
        match &self.sk_info {
            Some(sk_info) => (sk_info as *const Rc<RefCell<SkInfo>>).cast_mut().cast::<c_void>(),
            None => std::ptr::null_mut(),
        }
    }

    /// Starts the background watchers: the plugin library watch (unless [`HotReloading::watch`] is 0) and, unless
    /// the auto-build is disabled (see [`HotReloading::no_build`]), the source watch running the build command.
    fn start_watchers(&mut self) {
        let (sender, receiver) = channel::<HostMsg>();
        self.loaded.receiver = Some(receiver);
        if self.watch_secs > 0.0 {
            let sender = sender.clone();
            let path = self.lib_path.clone();
            let period = Duration::from_secs_f32(self.watch_secs);
            thread::spawn(move || watch_lib(path, period, sender));
        }
        if let Some(build_cmd) = &self.build_cmd
            && !self.watch_roots.is_empty()
        {
            let sender = sender.clone();
            let roots = self.watch_roots.clone();
            let cmd = build_cmd.clone();
            thread::spawn(move || watch_sources_and_build(roots, Duration::from_secs(1), cmd, sender));
        }
        Log::info(format!(
            "hot_reloading: plugin {}, build command: {}",
            self.lib_path.display(),
            self.build_cmd.as_deref().unwrap_or("none")
        ));
    }

    /// Begins the plugin preloaded by [`HotReloading::apply_plugin_settings`], selects its start view and keeps it
    /// as the running plugin. A `begin` failure drops it: the load logic retries.
    fn begin_preloaded(&mut self) {
        let Some(plugin) = self.loaded.preloaded.take() else { return };
        let begin_status = plugin.begin(self.sk_info_ptr());
        if begin_status != 0 {
            Log::err(format!(
                "hot_reloading: sk_run_sk_begin failed with status {begin_status}, the plugin will be reloaded."
            ));
            drop(plugin);
            cleanup_plugin_copies(None);
            return;
        }
        let selected = self
            .start_view
            .as_deref()
            .and_then(|name| find_view(&plugin.views, name))
            .filter(|&index| plugin.select(index as u32) == 0);
        let views = plugin.views.len();
        let selected_name =
            selected.map(|index| plugin.views[index].name.clone()).unwrap_or_else(|| "none".to_string());
        self.loaded.loaded_fp = fingerprint(&self.lib_path);
        self.loaded.active_view = selected;
        self.loaded.status = format!("{views} views, active: {selected_name}");
        Log::info(format!("hot_reloading: plugin loaded ({})", self.loaded.status));
        self.loaded.plugin = Some(plugin);
    }

    /// One frame of the workflow: the watcher messages, the zombie unload, the (re)load, the selector window, the
    /// plugin step and the test mode.
    fn step_project(&mut self, token: &MainThreadToken) {
        let sk_info_ptr = self.sk_info_ptr();
        self.drain_watcher_messages();
        self.unload_zombie();
        self.reload_if_needed(sk_info_ptr);
        self.selector_window();
        self.step_plugin(sk_info_ptr, token);
        self.run_test_mode();
    }

    /// Drains the messages sent by the background watchers and updates the status line accordingly.
    fn drain_watcher_messages(&mut self) {
        let Some(receiver) = self.loaded.receiver.as_ref() else { return };
        loop {
            match receiver.try_recv() {
                Ok(HostMsg::LibChanged) => {
                    if fingerprint(&self.lib_path) != self.loaded.loaded_fp {
                        self.loaded.reload_requested = true;
                    }
                }
                Ok(HostMsg::BuildStarted) => self.loaded.status = "building...".to_string(),
                Ok(HostMsg::BuildFinished { ok, tail }) => {
                    if ok {
                        self.loaded.status = "build ok".to_string();
                        // In case the build produced a new lib while the watch thread was busy.
                        if fingerprint(&self.lib_path) != self.loaded.loaded_fp {
                            self.loaded.reload_requested = true;
                        }
                    } else {
                        self.loaded.status = format!("build FAILED:\n{tail}");
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    /// Unloads the library of the plugin ended on the previous frame (its steppers may still be shutting down).
    fn unload_zombie(&mut self) {
        if let Some(lib) = self.loaded.zombie.take() {
            drop(lib);
        }
    }

    /// (Re)loads the plugin when a new library is available or a reload has been requested: the NEW plugin is
    /// loaded and validated BEFORE the old one is retired, so a broken build never kills the running session. A file
    /// whose load FAILED is not retried every frame (log spam): only when it changes or on an explicit reload.
    /// * `sk_info_ptr` - The opaque pointer handed to `sk_run_sk_begin`.
    fn reload_if_needed(&mut self, sk_info_ptr: *mut c_void) {
        if !self.loaded.reload_requested
            && (self.loaded.plugin.is_some()
                || !self.lib_path.exists()
                || fingerprint(&self.lib_path) == self.loaded.failed_fp)
        {
            return;
        }
        self.loaded.reload_requested = false;
        let previous_name = self.loaded.active_view.and_then(|index| {
            self.loaded.plugin.as_ref().and_then(|plugin| plugin.views.get(index).map(|view| view.name.clone()))
        });
        Log::info(format!("hot_reloading: loading plugin from {}", self.lib_path.display()));
        let wanted_name = previous_name.or_else(|| self.start_view.clone());
        match load_plugin(Some(sk_info_ptr), &self.lib_path, &mut self.loaded.copy_counter) {
            Ok(new_plugin) => {
                if let Some(old) = self.loaded.plugin.take() {
                    old.end();
                    self.loaded.zombie = Some(old.lib); // dlclose next frame
                }
                self.loaded.loaded_fp = fingerprint(&self.lib_path);
                let selected = wanted_name.as_deref().and_then(|name| find_view(&new_plugin.views, name));
                self.loaded.active_view = selected.filter(|&index| new_plugin.select(index as u32) == 0);
                let views = new_plugin.views.len();
                let selected_name = self
                    .loaded
                    .active_view
                    .map(|index| new_plugin.views[index].name.clone())
                    .unwrap_or_else(|| "none".to_string());
                self.loaded.plugin = Some(new_plugin);
                self.loaded.status = format!("{views} views, active: {selected_name}");
                self.loaded.failed_fp = None;
                Log::info(format!("hot_reloading: plugin loaded ({})", self.loaded.status));
            }
            Err(err) => {
                Log::err(format!("hot_reloading: {err}"));
                self.loaded.failed_fp = fingerprint(&self.lib_path);
                if self.loaded.plugin.is_none() {
                    self.loaded.status = format!("load failed: {err}");
                }
            }
        }
    }

    /// The default [`Appearence`] of the selector window: a wide grid window, resizable and scalable with its
    /// handle. Only built once a StereoKit session is running, see [`HotReloading::appearence`].
    fn default_appearence() -> Appearence {
        let mut appearence = Appearence::default();
        appearence.window_size = Vec2::new(0.8, 0.6);
        appearence.min_window_size = Vec2::new(0.3, 0.3);
        appearence
    }

    /// Truncates `text` to at most `max_chars` characters, ending with an ellipsis when truncated.
    fn ellipsize(text: &str, max_chars: usize) -> String {
        if text.chars().count() <= max_chars {
            text.to_owned()
        } else {
            format!("{}…", text.chars().take(max_chars.saturating_sub(1)).collect::<String>())
        }
    }

    /// The new smoothed fps of the selector window after one more frame lasting `step` seconds: an exponential
    /// moving average of the instantaneous `1.0 / step` with a ONE-SECOND time constant, so the value settles in
    /// about a second whatever the frame rate (the previous half-and-half average was per frame, hence
    /// frame-rate dependent). A `step` of zero (e.g. the very first frame) leaves the value untouched.
    fn smooth_fps(fps: f64, step: f64) -> f64 {
        /// The smoothing period in seconds: the memory of the average.
        const SMOOTH_SECS: f64 = 1.0;
        if step > 0.0 { fps + (1.0 / step - fps) * (1.0 - (-step / SMOOTH_SECS).exp()) } else { fps }
    }

    /// Draws the selector window: the fps, the status, the forced `Reload`/`Capture` buttons and the views of the
    /// plugin in a panel laid out on as many columns as the window width allows. The window itself goes through
    /// [`Appearence`]: its scaled `UiSettings` while drawing, its width, and its grab-able scale handle after
    /// (drag it along the window local X to widen the grid, along Z to scale everything, the height always fits
    /// the grid). The choices are applied immediately: a pressed view is selected (the previous one is properly
    /// removed by the plugin), and a capture is taken right here.
    fn selector_window(&mut self) {
        let mut pressed: Option<usize> = None;
        let mut capture = false;
        if self.show_ui {
            // The fps label: an average smoothed over one second, not frame-rate dependent (`smooth_fps`).
            self.loaded.fps = Self::smooth_fps(self.loaded.fps, Time::get_step());
            let appearence = self.appearence.get_or_insert_with(Self::default_appearence);
            // The window is drawn with the Appearence-scaled UiSettings, restored exactly as they were afterwards.
            let prev_settings = Ui::get_settings();
            let settings = appearence.get_ui_settings_scaled();
            Ui::settings(settings);
            Ui::push_id(&self.id);

            // The window width comes from `Appearence`, its height AUTO-FITS the grid below: a StereoKit window
            // created with a fixed height would NOT grow with its content, the views would overflow it.
            Ui::window("run_sk")
                .pose(&mut self.window_pose)
                .size(Vec2::new(appearence.scaled_window_size().x, 0.0))
                .begin();
            Ui::push_text_style(appearence.label_style);
            if Ui::button("Reload").press() {
                self.loaded.reload_requested = true;
            }
            Ui::same_line();
            if Ui::button("Capture").press() {
                capture = true;
            }
            Ui::same_line();
            Ui::label(format!("{:.0} fps", self.loaded.fps)).use_padding(true).draw();

            Ui::label(&self.loaded.status).use_padding(true).draw();
            Ui::next_line();
            Ui::hseparator();
            Ui::pop_text_style();
            Ui::push_text_style(appearence.list_style);
            // The views of the plugin, in a panel whose column count adapts to the window width: fill it with as
            // many columns as possible, like the grid mode of the file browser. The panel is drawn in the window's
            // OWN layout flow: a child `Ui::layout_push` would isolate the grid extents (`parent = -1` in C), the
            // window would never learn the grid size and would not grow to fit it.
            match self.loaded.plugin.as_ref() {
                Some(plugin) if !plugin.views.is_empty() => {
                    let line = Ui::get_line_height();
                    // The content width the window aims at, then the column count from the minimum cell width (a
                    // few characters of text).
                    let content_w = (appearence.scaled_window_size().x - settings.margin * 2.0).max(line);
                    const MIN_CELL_CHARS: f32 = 5.0;
                    let columns = ((content_w + settings.gutter) / (line * MIN_CELL_CHARS + settings.padding))
                        .floor()
                        .max(1.0) as usize;
                    let cell_w = content_w / columns as f32 - settings.padding * 1.4;
                    let cell_h = line * 1.0;
                    // Names longer than the cell end with an ellipsis: one view stays on one single line.
                    let max_chars = (((cell_w - settings.padding * 2.0) / (line * 0.35)).floor() as usize).max(4);
                    let rows = plugin.views.len().div_ceil(columns);

                    Ui::panel_begin(Some(UiPad::Inside));
                    for row in 0..rows {
                        for col in 0..columns {
                            let index = row * columns + col;
                            let Some(view) = plugin.views.get(index) else { break };
                            if col > 0 {
                                Ui::same_line();
                            }
                            // No explicit `Ui::next_line()` at the end of a grid row: the LAST button of the row
                            // has already ended the line (`ui_layout_reserve` calls `ui_nextline` internally,
                            // undone only by `ui_sameline`), so an explicit one would consume a SECOND `gutter`
                            // per row.
                            let active = self.loaded.active_view == Some(index);
                            if active {
                                Ui::push_tint(appearence.button_tint);
                            }
                            let label = Self::ellipsize(
                                &format!("{}{}", view.name, if view.has_screenshot { " (img)" } else { "" }),
                                max_chars,
                            );
                            if Ui::button(label).size(Vec2::new(cell_w, cell_h)).press() {
                                pressed = Some(index);
                            }
                            if active {
                                Ui::pop_tint();
                            }
                        }
                    }
                    // `panel_end` reserves the panel size in the window layout, so the auto-fitted window height
                    // always ends up exactly around the grid.
                    Ui::panel_end();
                }
                _ => Ui::label("no plugin loaded").use_padding(true).draw(),
            }
            Ui::pop_text_style();
            Ui::window_end();

            Ui::pop_id();
            Ui::settings(prev_settings);

            // Grab-able knob anchored to the window: dragging it along the window local X resizes the width, along
            // Y the height, and along Z (towards the user) the whole scale, see `Appearence::scale_handle`.
            appearence.scale_handle(&self.window_pose, "run_sk_scale");
        }
        if let Some(index) = pressed
            && let Some(plugin) = self.loaded.plugin.as_ref()
        {
            self.loaded.active_view = if plugin.select(index as u32) == 0 { Some(index) } else { None };
        }
        if capture {
            let name = self.view_name().unwrap_or_else(|| "none".to_string());
            capture_screenshot(&name);
        }
    }

    /// The name of the active view, if any.
    fn view_name(&self) -> Option<String> {
        self.loaded
            .active_view
            .and_then(|index| self.loaded.plugin.as_ref().and_then(|plugin| plugin.views.get(index)))
            .map(|view| view.name.clone())
    }

    /// Steps the plugin: its own steppers then run pre-app then post-app, exactly like `SkClosures` does for the
    /// host.
    /// * `sk_info_ptr` - The opaque pointer handed to `sk_run_sk_step` (ignored by the plugin, params are reserved).
    /// * `token` - The token of the current frame.
    fn step_plugin(&mut self, sk_info_ptr: *mut c_void, token: &MainThreadToken) {
        if let Some(plugin) = self.loaded.plugin.as_ref() {
            let token_ptr: *mut c_void = (token as *const MainThreadToken).cast_mut().cast();
            plugin.step(sk_info_ptr, token_ptr);
        }
    }

    /// The test mode: screenshots the active view after [`HotReloading::test_steps`] frames, then asks the app to
    /// quit (through a [`StepperAction::Quit`], so the shutdown sequence stays the normal one).
    fn run_test_mode(&mut self) {
        if self.test_steps == 0 {
            return;
        }
        if self.loaded.test_step == 0
            && self.loaded.active_view.is_none()
            && let Some(plugin) = self.loaded.plugin.as_ref()
            && !plugin.views.is_empty()
            && plugin.select(0) == 0
        {
            self.loaded.active_view = Some(0);
        }
        self.loaded.test_step += 1;
        if self.loaded.test_step == self.test_steps {
            let name = self.view_name().unwrap_or_else(|| "none".to_string());
            capture_screenshot(&name);
        } else if self.loaded.test_step > self.test_steps {
            // The event loop delivers the action at the next frame: `Steppers::step` then asks the app to quit.
            SkInfo::send_event(
                &self.sk_info,
                StepperAction::Quit(self.id.clone(), "hot_reloading test mode".to_string()),
            );
            self.test_steps = 0; // one request is enough
        }
    }
}

impl IStepper for HotReloading {
    /// Stores the [`SkInfo`] of the session, starts the watchers and begins the plugin preloaded by
    /// [`HotReloading::apply_plugin_settings`].
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        // The look of the selector window: built here rather than in `new` because `Appearence` creates text style
        // assets, and the stepper is usually built before `Sk::init` (see `HotReloading::appearence`).
        let mut appearence = self.appearence.take().unwrap_or_else(Self::default_appearence);
        appearence.start();
        self.appearence = Some(appearence);
        self.start_watchers();
        self.begin_preloaded();
        true
    }

    /// Steps one frame of the workflow (see `step_project`).
    fn step(&mut self, token: &MainThreadToken) {
        self.step_project(token);
    }

    /// Positive: the plugin is stepped **after** the app callback, see [`crate::plugin_abi`].
    fn step_priority(&self) -> i32 {
        1
    }

    /// Asks the plugin to shut its steppers down (`sk_run_sk_end`) then drops the preloaded / zombie / watcher state.
    fn shutdown(&mut self) {
        if let Some(plugin) = self.loaded.plugin.take() {
            plugin.end();
        }
        self.loaded.preloaded = None;
        self.loaded.zombie = None;
    }
}

// ------------------------------------------------------------------
// Runtime state
// ------------------------------------------------------------------

/// Everything the tool needs between its first frame and its shutdown.
struct Loaded {
    /// The plugin currently driving the session.
    plugin: Option<Plugin>,
    /// The unique counter of the plugin copies under `target/run_sk`, see [`load_plugin`].
    copy_counter: u32,
    /// Fingerprint of the plugin library that is currently loaded.
    loaded_fp: Fingerprint,
    /// Fingerprint of a plugin library whose load FAILED: it is retried only when it changes.
    failed_fp: Fingerprint,
    /// The library of an ended plugin, unloaded on the next frame.
    zombie: Option<Library>,
    /// A (re)load has been requested: the first load, a watcher message, the `Reload` button.
    reload_requested: bool,
    /// The index of the active view, if any.
    active_view: Option<usize>,
    /// The messages sent by the background watchers, `None` until [`HotReloading::start_watchers`] is called.
    receiver: Option<Receiver<HostMsg>>,
    /// The plugin loaded by [`HotReloading::apply_plugin_settings`], which is begun at the first frame.
    preloaded: Option<Plugin>,
    /// The status line of the selector window.
    status: String,
    /// The smoothed fps shown in the selector window.
    fps: f64,
    /// The number of steps run by the test mode.
    test_step: u32,
}

impl Default for Loaded {
    fn default() -> Self {
        Self {
            plugin: None,
            copy_counter: 0,
            loaded_fp: None,
            failed_fp: None,
            zombie: None,
            reload_requested: false,
            active_view: None,
            receiver: None,
            preloaded: None,
            status: "waiting for plugin".to_string(),
            fps: 0.0,
            test_step: 0,
        }
    }
}

// ------------------------------------------------------------------
// Project probing
// ------------------------------------------------------------------

/// The file extension of the plugin library of the current platform (`so`, `dll` or `dylib`).
fn plugin_ext() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// The plugin library file name of the current platform: `lib<name>.so` on Linux, `<name>.dll` on Windows (no `lib`
/// prefix), `lib<name>.dylib` on macOS.
/// * `name` - The name of the plugin (the crate name for a project, the example name in this repository).
pub fn plugin_file(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
}

/// The paths scanned by the auto-build watcher: the usual roots of a StereoKit-rust project, if they exist.
pub fn default_watch_roots() -> Vec<PathBuf> {
    ["src", "examples", "assets", "shaders_src", "Cargo.toml", "build.rs"]
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .collect()
}

/// The name of the `examples/main*.rs` of the current directory (e.g. `main` for `examples/main.rs`, `main_pc` for
/// `examples/main_pc.rs`), if any. The exact `examples/main.rs` wins over the other `main_*.rs` files.
fn find_example_source_name() -> Option<String> {
    find_example_source_name_in(Path::new("."))
}

/// The name of the `examples/main*.rs` of the directory `dir` (e.g. `main` for `examples/main.rs`, `main_pc` for
/// `examples/main_pc.rs`), if any. The exact `examples/main.rs` wins over the other `main_*.rs` files.
fn find_example_source_name_in(dir: &Path) -> Option<String> {
    let examples_dir = dir.join("examples");
    if !examples_dir.is_dir() {
        return None;
    }
    // Prefer the exact examples/main.rs.
    if examples_dir.join("main.rs").is_file() {
        return Some("main".to_string());
    }
    // Otherwise the first examples/main_*.rs.
    if let Ok(entries) = fs::read_dir(&examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // `file_name` (not a rsplit on '/'): Windows paths use '\'.
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with("main_")
                && let Some(stem) = name.strip_suffix(".rs")
            {
                return Some(stem.to_string());
            }
        }
    }
    None
}

/// The default plugin library path, guessed from the layout of the current directory (source files present, not
/// build artifacts), see [`default_lib_path_in`].
///
/// see also [`HotReloading::auto_detect`]
pub fn default_lib_path() -> PathBuf {
    default_lib_path_in(Path::new("."))
}

/// The default plugin library path of the project at `dir`, guessed from its layout (source files present, not build
/// artifacts):
/// 1. `src/bin/main_<crate>.rs` exists in `dir` -> a `cargo-new_sk_rs_project` project, its plugin is
///    `dir/target/debug/<plugin of the crate>` (e.g. `libsk_project.so` on Linux / `sk_project.dll` on Windows for
///    the project `sk_project`),
/// 2. `dir/examples/main*.rs` exists -> the StereoKit-rust repository itself, its plugin is
///    `dir/target/debug/examples/<plugin of the example>` (e.g. `libmain_pc.so` / `main_pc.dll` for
///    `examples/main_pc.rs`),
/// 3. fallback: the first plugin file under `dir/target/debug/examples/` (e.g. cargo hashed copies), then the plugin
///    of `examples/main.rs`.
///
/// see also [`HotReloading::auto_detect_in`]
pub fn default_lib_path_in(dir: &Path) -> PathBuf {
    // A leading `.` would only pollute the produced paths (and the logs): `./target/...` -> `target/...`.
    let dir = dir.strip_prefix(".").unwrap_or(dir);
    // a project generated by cargo-new_sk_rs_project
    if let Ok(name) = get_cargo_name_in(dir) {
        let lib_name = name.replace(['-'], "_");
        if dir.join(format!("src/bin/main_{lib_name}.rs")).exists() {
            return dir.join("target/debug").join(plugin_file(&lib_name));
        }
    }
    // the StereoKit-rust repository itself
    if let Some(example) = find_example_source_name_in(dir) {
        return dir.join("target/debug/examples").join(plugin_file(&example));
    }
    // last resort: whatever is already built under dir/target/debug/examples
    if let Ok(entries) = fs::read_dir(dir.join("target/debug/examples")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == plugin_ext()) {
                return path;
            }
        }
    }
    dir.join("target/debug/examples").join(plugin_file("main"))
}

/// The build command producing the plugin at `lib_path`, derived from the same probing as [`default_lib_path`]:
/// `cargo build --example <name> --features skc-shared` in this repository, `cargo build --lib --features
/// skc-shared` otherwise (`--lib` never touches the running executable, which cannot be replaced on Windows).
pub fn default_build_cmd(lib_path: &Path) -> String {
    build_cmd_from(lib_path, find_example_source_name())
}

/// The build command producing the plugin of the project at `dir`, derived from the same probing as
/// [`default_lib_path_in`] (the example name found in `dir/examples` for the StereoKit-rust layout).
pub fn default_build_cmd_in(dir: &Path) -> String {
    build_cmd_from(&default_lib_path_in(dir), find_example_source_name_in(dir))
}

/// The cargo command building the plugin at `lib_path`: the example one for the StereoKit-rust layout, the lib one
/// otherwise (`--lib` never touches the running executable, which cannot be replaced on Windows).
fn build_cmd_from(lib_path: &Path, example: Option<String>) -> String {
    if lib_path.components().any(|component| component.as_os_str() == "examples") {
        let example = example.unwrap_or_else(|| "main".to_string());
        format!("cargo build --example {example} --features skc-shared")
    } else {
        "cargo build --lib --features skc-shared".to_string()
    }
}

// ------------------------------------------------------------------
// Plugin loading
// ------------------------------------------------------------------

// One fn pointer type per signature of the `plugin_abi` entry points, named by the [`Plugin`] field holding it.
type FnU32 = unsafe extern "C" fn() -> u32;
type FnCChar = unsafe extern "C" fn() -> *const c_char;
type FnViewInfo = unsafe extern "C" fn(u32, *mut PluginViewInfo) -> u32;
type FnSkPtr = unsafe extern "C" fn(*mut c_void) -> u32;
type FnSkTokenPtr = unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32;
type FnIndex = unsafe extern "C" fn(u32) -> u32;
type FnSkSettings = unsafe extern "C" fn(*mut SkSettings) -> u32;

/// One view (a `Test`) of the plugin.
struct View {
    name: String,
    /// Does a screenshot file already exist for this view?
    has_screenshot: bool,
}

/// A successfully loaded (and possibly begun) plugin: `lib` must outlive the `*_fn` pointers, and every call
/// follows the [`crate::plugin_abi`] contract (main thread only).
struct Plugin {
    /// The loaded library.
    lib: Library,
    views: Vec<View>,
    /// `sk_run_sk_begin`, once the session exists.
    begin_fn: FnSkPtr,
    /// `sk_run_sk_settings`, before the session exists.
    settings_fn: FnSkSettings,
    /// `sk_run_sk_select`, swaps the active view.
    select_fn: FnIndex,
    /// `sk_run_sk_step`, once per frame.
    step_fn: FnSkTokenPtr,
    /// `sk_run_sk_end`, before unloading.
    end_fn: FnU32,
}

impl Plugin {
    fn begin(&self, sk_info: *mut c_void) -> u32 {
        unsafe { (self.begin_fn)(sk_info) }
    }

    fn settings(&self, settings: &mut SkSettings) -> u32 {
        unsafe { (self.settings_fn)(settings) }
    }

    fn select(&self, index: u32) -> u32 {
        unsafe { (self.select_fn)(index) }
    }

    fn step(&self, sk_info: *mut c_void, token: *mut c_void) -> u32 {
        unsafe { (self.step_fn)(sk_info, token) }
    }

    fn end(&self) -> u32 {
        unsafe { (self.end_fn)() }
    }
}

/// The error message of a missing symbol.
fn missing(symbol: &'static str) -> impl Fn(libloading::Error) -> String {
    move |err| format!("plugin has no symbol {symbol}: {err}")
}

/// Removes the stale plugin copies of the previous loads from `target/run_sk`, see [`load_plugin`]. Removing a copy
/// already loaded by another session is harmless on Linux/macOS and simply fails on Windows (a later reload removes
/// it).
/// * `keep` - The copy being loaded right now, if any.
fn cleanup_plugin_copies(keep: Option<&Path>) {
    let Ok(entries) = fs::read_dir("target/run_sk") else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
        let stale = name.starts_with("plugin-")
            && ["so", "dll", "dylib"].iter().any(|ext| path.extension().is_some_and(|e| e == *ext));
        if stale && Some(path.as_path()) != keep {
            fs::remove_file(&path).ok();
        }
    }
}

/// Copies `src` to a unique file under `target/run_sk/` then loads it, checks the ABI and the crate versions, reads
/// the view list and eventually calls `begin`.
///
/// A loaded DLL cannot be overwritten on Windows: loading from a copy (`plugin-<pid>-<counter>.<ext>`, one name per
/// load) lets cargo replace the original file while the previous plugin stays loaded. The `target/` directory is
/// never created: without it the project was not built, and the plugin cannot exist.
/// * `sk_info` - The opaque pointer to the `Rc<RefCell<SkInfo>>` of the host session, or `None` for an inspection
///   without any session (the settings read of [`HotReloading::apply_plugin_settings`], a view list).
/// * `src` - The freshly built plugin library.
/// * `copy_counter` - The unique counter of the plugin copies of this session.
fn load_plugin(sk_info: Option<*mut c_void>, src: &Path, copy_counter: &mut u32) -> Result<Plugin, String> {
    let copy_dir = Path::new("target/run_sk");
    if !Path::new("target").is_dir() {
        return Err("the current directory has no `target/` directory: build the project first".to_string());
    }
    fs::create_dir_all(copy_dir).map_err(|e| format!("cannot create {}: {e}", copy_dir.display()))?;
    let copy = copy_dir.join(format!("plugin-{}-{copy_counter}.{}", std::process::id(), plugin_ext()));
    *copy_counter += 1;
    fs::copy(src, &copy).map_err(|e| format!("cannot copy {} to {}: {e}", src.display(), copy.display()))?;
    cleanup_plugin_copies(Some(&copy));

    // SAFETY: the symbols below follow the `stereokit_rust::plugin_abi` contract.
    unsafe {
        let lib =
            Library::new(&copy).map_err(|e| format!("cannot load {} (dlopen/LoadLibrary): {e}", copy.display()))?;

        let version: Symbol<FnU32> = lib.get(b"sk_run_sk_version").map_err(missing("sk_run_sk_version"))?;
        if version() != SK_RUN_SK_ABI_VERSION {
            return Err(format!("plugin ABI version {} does not match host ABI {SK_RUN_SK_ABI_VERSION}", version()));
        }

        let crate_version: Symbol<FnCChar> =
            lib.get(b"sk_run_sk_crate_version").map_err(missing("sk_run_sk_crate_version"))?;
        // SAFETY: the plugin returns a pointer to a static nul-terminated string.
        let plugin_version = CStr::from_ptr(crate_version()).to_string_lossy().into_owned();
        if plugin_version != env!("CARGO_PKG_VERSION") {
            return Err(format!(
                "plugin was built with stereokit-rust {plugin_version} but the host uses {}: rebuild both sides from the same source",
                env!("CARGO_PKG_VERSION")
            ));
        }

        let settings_fn: Symbol<FnSkSettings> =
            lib.get(b"sk_run_sk_settings").map_err(missing("sk_run_sk_settings"))?;

        let views_count: Symbol<FnU32> = lib.get(b"sk_run_sk_views_count").map_err(missing("sk_run_sk_views_count"))?;
        let view_info: Symbol<FnViewInfo> = lib.get(b"sk_run_sk_view_info").map_err(missing("sk_run_sk_view_info"))?;

        let count = views_count();
        let mut views = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut info = PluginViewInfo::new("", false);
            let status = view_info(index, &mut info);
            if status != 0 {
                return Err(format!("sk_run_sk_view_info({index}) failed with status {status}"));
            }
            views.push(View { name: info.name(), has_screenshot: info.has_screenshot != 0 });
        }

        let begin: Symbol<FnSkPtr> = lib.get(b"sk_run_sk_begin").map_err(missing("sk_run_sk_begin"))?;
        let select: Symbol<FnIndex> = lib.get(b"sk_run_sk_select").map_err(missing("sk_run_sk_select"))?;
        let step: Symbol<FnSkTokenPtr> = lib.get(b"sk_run_sk_step").map_err(missing("sk_run_sk_step"))?;
        let end: Symbol<FnU32> = lib.get(b"sk_run_sk_end").map_err(missing("sk_run_sk_end"))?;

        // Deref the Symbols (they borrow `lib`) before moving `lib` into the Plugin.
        let (begin, settings_fn, select, step, end) = (*begin, *settings_fn, *select, *step, *end);

        if let Some(sk_info) = sk_info {
            let status = begin(sk_info);
            if status != 0 {
                return Err(format!("sk_run_sk_begin failed with status {status}"));
            }
        }

        Ok(Plugin { lib, views, begin_fn: begin, settings_fn, select_fn: select, step_fn: step, end_fn: end })
    }
}

/// The index of the view `name` in `views`: its exact name, its name ignoring the case, or a substring of its name.
fn find_view(views: &[View], name: &str) -> Option<usize> {
    views
        .iter()
        .position(|view| view.name == name)
        .or_else(|| views.iter().position(|view| view.name.eq_ignore_ascii_case(name)))
        .or_else(|| {
            let lower = name.to_lowercase();
            views.iter().position(|view| view.name.to_lowercase().contains(&lower))
        })
}

// ------------------------------------------------------------------
// Background watchers
// ------------------------------------------------------------------

/// (mtime, size, head-hash) of a file. Cargo always rewrites the whole plugin file, so this is enough to detect a
/// new build.
type Fingerprint = Option<(SystemTime, u64, u64)>;

/// The [`Fingerprint`] of the file at `path` (`None` when it cannot be read).
fn fingerprint(path: &Path) -> Fingerprint {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let size = meta.len();
    let mut head = 0u64;
    if let Ok(mut file) = fs::File::open(path) {
        let mut buffer = [0u8; 4096];
        if let Ok(n) = file.read(&mut buffer) {
            for chunk in buffer[..n].chunks(8) {
                let mut bytes = [0u8; 8];
                bytes[..chunk.len()].copy_from_slice(chunk);
                head = head.wrapping_mul(31).wrapping_add(u64::from_le_bytes(bytes));
            }
        }
    }
    Some((mtime, size, head))
}

/// Messages sent by the background watcher threads to the main loop.
enum HostMsg {
    /// The watched plugin library changed and is stable.
    LibChanged,
    /// A source change was detected, the build command is starting.
    BuildStarted,
    /// The build command finished.
    BuildFinished {
        /// Did the build succeed?
        ok: bool,
        /// The last lines of stderr, to show a failed build.
        tail: String,
    },
}

/// Polls the plugin library and reports a `LibChanged` each time a NEW stable version appears (two consecutive
/// identical samples: cargo may still be writing the file on the first one).
fn watch_lib(path: PathBuf, period: Duration, sender: Sender<HostMsg>) {
    let mut candidate = fingerprint(&path);
    let mut stable = candidate;
    loop {
        thread::sleep(period);
        let current = fingerprint(&path);
        if current == candidate && current != stable {
            stable = current;
            sender.send(HostMsg::LibChanged).ok();
        }
        candidate = current;
    }
}

/// Sum of the modification times of everything under `roots`.
fn scan(roots: &[PathBuf]) -> u64 {
    fn visit(path: &Path, acc: &mut u64, depth: usize) {
        if depth > 6 {
            return;
        }
        let Ok(meta) = fs::metadata(path) else {
            return;
        };
        if meta.is_file() {
            if let Ok(mtime) = meta.modified()
                && let Ok(since_epoch) = mtime.duration_since(SystemTime::UNIX_EPOCH)
            {
                *acc = acc.wrapping_add(since_epoch.as_nanos() as u64);
            }
        } else if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                visit(&entry.path(), acc, depth + 1);
            }
        }
    }
    let mut acc = 0u64;
    for root in roots {
        visit(root, &mut acc, 0);
    }
    acc
}

/// Runs the build command once (`sh -c <cmd>` on Linux, `cmd /C <cmd>` on Windows) and reports the outcome to the
/// main loop.
fn run_build(build_cmd: &str, sender: &Sender<HostMsg>) {
    sender.send(HostMsg::BuildStarted).ok();
    #[cfg(target_os = "windows")]
    let mut shell = Command::new("cmd");
    #[cfg(target_os = "windows")]
    shell.arg("/C");
    #[cfg(not(target_os = "windows"))]
    let mut shell = Command::new("sh");
    #[cfg(not(target_os = "windows"))]
    shell.arg("-c");
    match shell.arg(build_cmd).output() {
        Ok(output) => {
            let ok = output.status.success();
            let tail = String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            sender.send(HostMsg::BuildFinished { ok, tail }).ok();
        }
        Err(err) => {
            sender
                .send(HostMsg::BuildFinished { ok: false, tail: format!("cannot run build command: {err}") })
                .ok();
        }
    }
}

/// Watches the sources and runs the build command once at startup then each time they settle down: a fresh project
/// is built before the viewer needs its plugin, and [`watch_lib`] picks up the rebuilt library.
fn watch_sources_and_build(roots: Vec<PathBuf>, debounce: Duration, build_cmd: String, sender: Sender<HostMsg>) {
    run_build(&build_cmd, &sender);
    let mut state = scan(&roots);
    loop {
        thread::sleep(Duration::from_millis(400));
        let mut current = scan(&roots);
        if current == state {
            continue;
        }
        // Debounce: wait for the sources to be quiet (an edit or a cargo run touches many files).
        loop {
            thread::sleep(debounce);
            let again = scan(&roots);
            if again == current {
                break;
            }
            current = again;
        }
        state = current;
        run_build(&build_cmd, &sender);
    }
}

/// Saves a screenshot of the current viewpoint as `screenshots/run_sk_<view>.jpeg`.
fn capture_screenshot(view_name: &str) {
    fs::create_dir_all("screenshots").ok();
    let snake = view_name.to_lowercase().replace([' ', '/'], "_");
    let file = format!("screenshots/run_sk_{snake}.jpeg");
    Log::info(format!("hot_reloading: capturing {file}"));
    // the viewpoint of the test mode of the demos, in front of the views laid out around the origin
    Renderer::screenshot(
        &file,
        90,
        Pose::look_at(Vec3::new(2.0, 1.5, 1.5), Vec3::new(0.0, 1.0, 0.0)),
        800,
        600,
        Some(80.0),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_fps_ignores_a_zero_step() {
        // The very first frame may report a zero step: no inf, no NaN, no change.
        assert_eq!(HotReloading::smooth_fps(72.0, 0.0), 72.0);
        assert_eq!(HotReloading::smooth_fps(0.0, 0.0), 0.0);
    }

    #[test]
    fn smooth_fps_has_a_one_second_memory() {
        // One simulated second of constant frames lands at 1 - e^-1 (~63%) of the target WHATEVER the frame
        // rate: the smoothing is time based, not frame based.
        let expected_ratio = 1.0 - (-1.0f64).exp();
        for (steps, step) in [(30, 1.0 / 30.0), (90, 1.0 / 90.0), (144, 1.0 / 144.0)] {
            let target = 1.0 / step;
            let mut fps = 0.0;
            for _ in 0..steps {
                fps = HotReloading::smooth_fps(fps, step);
            }
            let expected = target * expected_ratio;
            assert!(
                (fps - expected).abs() < expected * 0.01,
                "{steps} frames of {step:.4}s gave {fps} instead of {expected}"
            );
        }
    }

    #[test]
    fn smooth_fps_converges_to_the_target() {
        // Ten simulated seconds at 90 fps: the display is the real fps.
        let step = 1.0 / 90.0;
        let mut fps = 0.0;
        for _ in 0..(90 * 10) {
            fps = HotReloading::smooth_fps(fps, step);
        }
        assert!((fps - 90.0).abs() < 0.01, "fps {fps}");
    }

    #[test]
    fn smooth_fps_absorbs_one_hiccup() {
        // A single dropped frame barely moves a settled value, and it stays finite.
        let step = 1.0 / 90.0;
        let mut fps = 90.0;
        for _ in 0..(90 * 2) {
            fps = HotReloading::smooth_fps(fps, step);
        }
        fps = HotReloading::smooth_fps(fps, 0.1); // one 100 ms frame
        assert!(fps > 75.0 && fps < 90.0, "one hiccup gave {fps}");
        assert!(fps.is_finite());
    }

    #[test]
    fn auto_detect_in_finds_this_repository_plugin() {
        // This repository is an `examples/main*.rs` project: the tool points at the plugin of examples/main.rs and
        // the build command is left untouched in the current directory.
        let tool = HotReloading::auto_detect_in(Path::new("."));
        assert_eq!(tool.lib_path, Path::new("target/debug/examples").join(plugin_file("main")));
        assert_eq!(tool.build_cmd, Some("cargo build --example main --features skc-shared".to_string()));
    }

    #[test]
    fn auto_detect_in_cds_into_the_given_directory() {
        // A `cargo-new_sk_rs_project` layout dropped in a temporary directory: the tool points at its cdylib and
        // the build command `cd`s into that directory.
        let dir = std::env::temp_dir().join(format!("sk_auto_detect_in_{}", std::process::id()));
        fs::create_dir_all(dir.join("src/bin")).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"sk_project\"\n").unwrap();
        fs::write(dir.join("src/bin/main_sk_project.rs"), "fn main() {}").unwrap();
        let tool = HotReloading::auto_detect_in(&dir);
        assert_eq!(tool.lib_path, dir.join("target/debug").join(plugin_file("sk_project")));
        assert_eq!(
            tool.build_cmd,
            Some(format!("cd \"{}\" && cargo build --lib --features skc-shared", dir.display()))
        );
        fs::remove_dir_all(&dir).ok();
    }
}
