use crate::{
    sk::{AppFocus, MainThreadToken, QuitReason, Sk, SkInfo, sk_quit, sk_step},
    system::Log,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::VecDeque,
    fmt,
    rc::Rc,
    sync::mpsc,
    thread::sleep,
    time::Duration,
};

// --- Custom event loop types replacing winit ---
/// Error type for failed event loop creation (replaces `winit::error::EventLoopError`).
#[derive(Debug)]
pub struct EventLoopError(pub String);

impl fmt::Display for EventLoopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event loop error: {}", self.0)
    }
}

impl std::error::Error for EventLoopError {}

/// Error returned when sending an event to a closed event loop.
#[derive(Debug)]
pub struct EventLoopClosedError<T>(pub T);

impl<T> fmt::Display for EventLoopClosedError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event loop closed")
    }
}

impl<T: fmt::Debug> std::error::Error for EventLoopClosedError<T> {}

/// A thread-safe proxy to send events into the event loop from any thread.
/// Replaces `winit::event_loop::EventLoopProxy`.
pub struct EventLoopProxy<T: 'static> {
    sender: mpsc::Sender<T>,
}

impl<T> Clone for EventLoopProxy<T> {
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone() }
    }
}

impl<T: fmt::Debug> fmt::Debug for EventLoopProxy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoopProxy").finish()
    }
}

impl<T> EventLoopProxy<T> {
    /// Send an event to the event loop.
    pub fn send_event(&self, event: T) -> Result<(), EventLoopClosedError<T>> {
        self.sender.send(event).map_err(|e| EventLoopClosedError(e.0))
    }
}

/// A simple event loop that replaces `winit::event_loop::EventLoop`.
///
/// Uses an MPSC channel for cross-thread event delivery. The main loop polls
/// for incoming events each frame.
pub struct EventLoop<T: 'static> {
    receiver: mpsc::Receiver<T>,
    proxy: EventLoopProxy<T>,
}

impl<T> EventLoop<T> {
    /// Create a new event loop with an MPSC channel for user events.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let proxy = EventLoopProxy { sender };
        Self { receiver, proxy }
    }

    /// Create a proxy that can be used to send events from other threads.
    pub fn create_proxy(&self) -> EventLoopProxy<T> {
        self.proxy.clone()
    }

    /// Try to receive a pending event without blocking.
    pub(crate) fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// Simplified window event enum.
///
/// This provides a subset of window-related events. In XR mode most of these are
/// not relevant, but the enum is kept for API compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    /// The window has been destroyed.
    Destroyed,
    /// A redraw has been requested.
    RedrawRequested,
    /// The window gained or lost focus.
    Focused(bool),
    /// A close has been requested.
    CloseRequested,
    /// Keyboard text input.
    KeyboardInput { text: Option<String> },
}

type OnStepClosure<'a> = Box<dyn FnMut(&mut Sk, &MainThreadToken) + 'a>;
type OnDeviceEventClosure<'a> = Box<dyn FnMut(&mut Sk, WindowEvent) + 'a>;

#[derive(PartialEq)]
enum SleepPhase {
    Sleeping,
    WakingUp,
    WokeUp,
    Stopping,
}

/// Run the app with [SkClosures::run] or [SkClosures::run_app]
///
/// ### Example
/// ```
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::{maths::{Matrix, Pose},  model::Model, system::Renderer, system::Log,
///                      material::Material, tools::title::Title, util::named_colors,
///                      framework::{SkClosures, StepperAction}, sk::QuitReason};
///
/// let model = Model::from_file("cuve.glb", None).expect("Missing cube.glb").copy();
/// let material = Material::from_file("shaders/brick_pbr.hlsl.sks", None)
///     .expect("Missing shader");
/// let transform = Matrix::t_r([0.0, 0.0, -6.5], [90.0, 0.0, 0.0]);
///
/// let mut title = Title::new("SkClosures", None, None, None);
/// sk.send_event(StepperAction::add("Title_ID", title));
///
/// let mut iter = 0;
/// let mut hidden_time = std::time::SystemTime::now();
/// filename_scr = "screenshots/sk_closures.jpeg";
/// SkClosures::new(sk, |sk, token|  {
///     // Main loop where we draw stuff and do things!!
///     if iter > number_of_steps {sk.quit(None)}
///
///     model.draw_with_material(token, &material, transform ,  None, None);
///
///     iter+=1;
///                 
///     if iter == number_of_steps {
///         // render screenshot
///         system::Renderer::screenshot(token, filename_scr, 90,
///             maths::Pose::look_at(from_scr, at_scr),
///             width_scr, height_scr, Some(fov_scr) );
///     }
/// })
/// .on_sleeping_step(|_sk, _token| {
///     // This is called every 200ms when the app is sleeping
///     // when the android headset is off
///     let now = std::time::SystemTime::now();
///     if let Ok(duration) = now.duration_since(hidden_time) {
///         if duration.as_secs() > 15 {
///             Log::info("HIDDEN STEP -------> Dreaming ");
///             hidden_time = now;hidden_time = now;
///         }
///     }
/// })
/// .shutdown(|sk| {
///    // This is called when the app is shutting down
///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
///     Log::info(format!("QuitReason is {:?}", sk.get_quit_reason()));
/// })
/// .run();
/// Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_closures.jpeg" alt="screenshot" width="200">
pub struct SkClosures<'a> {
    sk: Sk,
    token: MainThreadToken,
    on_step: OnStepClosure<'a>,
    on_sleeping_step: OnStepClosure<'a>,
    on_window_event: OnDeviceEventClosure<'a>,
    shutdown: Box<dyn FnMut(&mut Sk) + 'a>,
    sleeping: SleepPhase,
}

impl<'a> SkClosures<'a> {
    /// This is the main loop call of the application. See [Steppers] for more details.
    fn step(&mut self) {
        if unsafe { sk_step(None) } == 0 {
            // Shutdown sequence
            self.sk.steppers.shutdown();
            (self.shutdown)(&mut self.sk);
            self.sleeping = SleepPhase::Stopping;
            Log::diag("sk_step() says stop()!!");
            return;
        }
        if !self.sk.steppers.step(&mut self.token) {
            self.sk.steppers.shutdown();
            unsafe { sk_quit(QuitReason::User) }
            Log::diag("The app demand to quit()!!");
        };
        while let Some(mut action) = self.sk.actions.pop_front() {
            action();
        }
        (self.on_step)(&mut self.sk, &self.token);

        self.token.event_report.clear();
    }

    /// Process pending external events from the event loop channel.
    fn poll_events(&mut self, event_loop: &EventLoop<StepperAction>) {
        // Poll Android activity events to prevent ANR (Application Not Responding).
        // Without this, the system event queue fills up and Android shows the ANR dialog.
        // Also detect Resume to wake up from sleep when the headset is turned back on.
        #[cfg(target_os = "android")]
        {
            use android_activity::{MainEvent, PollEvent};
            let sk_info = self.sk.get_sk_info_clone();
            let sk_info = sk_info.borrow();
            let android_app = sk_info.get_android_app();
            let mut resumed = false;
            android_app.poll_events(Some(Duration::ZERO), |event| match event {
                PollEvent::Main(MainEvent::Destroy) => {
                    Log::info("Android MainEvent::Destroy received");
                }
                PollEvent::Main(MainEvent::Resume { .. }) => {
                    resumed = true;
                }
                _ => {}
            });
            if resumed && self.sleeping == SleepPhase::Sleeping {
                self.sleeping = SleepPhase::WakingUp;
                Log::diag("Android Resume: waking up");
            }
        }

        while let Ok(action) = event_loop.try_recv() {
            Log::diag(format!("UserEvent {action:?}"));
            self.sk.send_event(action);
        }
    }

    /// Common way to run the main loop with only step and shutdown. This will block the thread until the application is
    /// shuting down.
    /// If you need a process when the headset is going to sleep ~~or track window events~~: use [SkClosures::new] instead.
    /// * `sk`: the stereokit context.
    /// * `on_step`: a callback that will be called every frame.
    /// * `on_shutdown`: a callback that will be called when the application is shutting down. After on_step and after the
    ///   steppers have been shutdown.
    ///
    /// see also [SkClosures::new]
    /// ### Example
    /// ```
    /// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    ///
    /// use stereokit_rust::{maths::{Matrix, Pose},  model::Model, system::Renderer, system::Log,
    ///                      framework::SkClosures , sk::QuitReason};
    ///
    /// let model = Model::from_file("cuve.glb", None).expect("Missing cube.glb").copy();
    /// let transform = Matrix::IDENTITY;
    ///
    /// let mut iter = 0;
    /// SkClosures::run_app(sk, |sk: &mut Sk, token: &MainThreadToken|  {
    ///     // Main loop where we draw stuff and do things!!
    ///     if iter > number_of_steps {sk.quit(None)}
    ///
    ///     model.draw(token,  transform ,  None, None);
    ///
    ///     iter += 1;
    /// },|sk: &mut Sk| {
    ///     // This is called when the app is shutting down
    ///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
    ///     Log::info(format!("QuitReason is {:?}", sk.get_quit_reason()));
    /// });
    /// Sk::shutdown();
    /// ```
    pub fn run_app<U: FnMut(&mut Sk, &MainThreadToken) + 'a, S: FnMut(&mut Sk) + 'a>(
        mut sk: Sk,
        on_step: U,
        on_shutdown: S,
    ) {
        let event_loop = sk.take_event_loop().expect("EventLoop already consumed");
        let mut this = Self {
            sk,
            on_step: Box::new(on_step),
            on_sleeping_step: Box::new(|_sk, _main_thread| {}),
            on_window_event: Box::new(|_sk, _window_event| {}),
            shutdown: Box::new(on_shutdown),
            token: MainThreadToken { event_report: vec![] },
            sleeping: SleepPhase::WakingUp,
        };
        this.run_loop(event_loop);
    }

    /// Create a new SkClosures with a step function.
    /// Add some callbacks with [SkClosures::on_sleeping_step], ~~[SkClosures::on_window_event]~~  and [SkClosures::shutdown]
    /// * sk : the Sk context.
    /// * on_step : the function to call on each step.
    ///
    /// see also [SkClosures::run_app]
    /// ### Example
    /// ```
    /// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    ///
    /// use stereokit_rust::{maths::{Matrix, Pose},  model::Model, system::Renderer, system::Log,
    ///                      framework::SkClosures , sk::QuitReason};
    ///
    /// let model = Model::from_file("cuve.glb", None).expect("Missing cube.glb").copy();
    /// let transform = Matrix::IDENTITY;
    ///
    /// let mut iter = 0;
    /// let mut hidden_time = std::time::SystemTime::now();
    /// SkClosures::new(sk, |sk, token|  {
    ///     // Main loop where we draw stuff and do things!!
    ///     if iter > number_of_steps {sk.quit(None)}
    ///
    ///     model.draw(token,  transform ,  None, None);
    ///
    ///     iter+=1;
    /// })
    /// .on_sleeping_step(|_sk, _token| {
    ///     // This is called every 200ms when the app is sleeping
    ///     // when the android headset is off
    ///     let now = std::time::SystemTime::now();
    ///     if let Ok(duration) = now.duration_since(hidden_time) {
    ///         if duration.as_secs() > 15 {
    ///             Log::info("HIDDEN STEP -------> Dreaming ");
    ///             hidden_time = now;hidden_time = now;
    ///         }
    ///     }
    /// })
    /// .shutdown(|sk| {
    ///    // This is called when the app is shutting down
    ///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
    ///     Log::info(format!("QuitReason is {:?}", sk.get_quit_reason()));
    /// })
    /// .run();
    /// Sk::shutdown();
    /// ```
    pub fn new<U: FnMut(&mut Sk, &MainThreadToken) + 'a>(sk: Sk, on_step: U) -> Self {
        Self {
            sk,
            on_step: Box::new(on_step),
            on_sleeping_step: Box::new(|_sk, _main_thread| {}),
            on_window_event: Box::new(|_sk, _windows_event| {}),
            shutdown: Box::new(|_sk| {}),
            token: MainThreadToken { event_report: vec![] },
            sleeping: SleepPhase::WakingUp,
        }
    }

    /// Add a sleeping step function to this SkClosures. This will be called every 200ms when the headset is down.
    /// Only for Android apps and architectures where the headset can be turned off.
    /// * `on_sleeping_step` - The function to call when the headset is down.
    ///
    /// May be set after [SkClosures::new]
    /// see examples [SkClosures]
    pub fn on_sleeping_step<U: FnMut(&mut Sk, &MainThreadToken) + 'a>(&mut self, on_sleeping_step: U) -> &mut Self {
        self.on_sleeping_step = Box::new(on_sleeping_step);
        self
    }

    /// Not usefull right now, but will be used to handle external controller events in the future.
    /// * `on_window_event` - The function to call when a window event is received that as not been handled by the
    ///   Steppers controller.
    ///
    /// May be set after [SkClosures::new]
    pub fn on_window_event<U: FnMut(&mut Sk, WindowEvent) + 'a>(&mut self, on_window_event: U) -> &mut Self {
        self.on_window_event = Box::new(on_window_event);
        self
    }

    /// Add a shutdown function to this SkClosures. This will be called when the app is shutting down.
    /// After all steppers have been shutdown, then the app will exit.
    /// * `on_shutdown` - The function to call when the app is shutting down.
    ///
    /// May be set after [SkClosures::new]
    /// see examples [SkClosures]
    pub fn shutdown<S: FnMut(&mut Sk) + 'a>(&mut self, on_shutdown: S) -> &mut Self {
        self.shutdown = Box::new(on_shutdown);
        self
    }

    /// Run the main loop. This will block until the app is shutting down.
    ///
    /// Have to be launched after [SkClosures::new] and eventually [SkClosures::on_sleeping_step] and [SkClosures::shutdown]
    /// see examples [SkClosures]
    pub fn run(&mut self) {
        let event_loop = self.sk.take_event_loop().expect("EventLoop already consumed");
        self.run_loop(event_loop);
    }

    /// Internal main loop implementation.
    fn run_loop(&mut self, event_loop: EventLoop<StepperAction>) {
        loop {
            // Process external events from the channel
            self.poll_events(&event_loop);

            if self.sk.get_app_focus() == AppFocus::Hidden && self.sleeping == SleepPhase::WokeUp {
                self.sleeping = SleepPhase::Sleeping;
                Log::diag("Time to sleep")
            }
            match self.sleeping {
                SleepPhase::WokeUp => {
                    self.step();
                }
                SleepPhase::WakingUp => {
                    self.step();
                    if self.sk.get_app_focus() != AppFocus::Hidden {
                        self.sleeping = SleepPhase::WokeUp;
                        Log::diag("WokeUp");
                    }
                }
                SleepPhase::Sleeping => {
                    sleep(Duration::from_millis(200));
                    if cfg!(not(target_os = "android")) {
                        self.step();
                    }
                    (self.on_sleeping_step)(&mut self.sk, &self.token);
                    if self.sk.get_app_focus() == AppFocus::Active {
                        self.sleeping = SleepPhase::WakingUp;
                    }
                }
                SleepPhase::Stopping => break,
            }
        }
        Log::info("SkClosure Exiting !!");
    }
}

/// See derive macro [`crate::IStepper`] which is usefull to implement this trait.
/// This is a lightweight standard interface for fire-and-forget systems that can be attached to StereoKit! This is
/// particularly handy for extensions/plugins that need to run in the background of your application, or even for
/// managing some of your own simpler systems.
///
/// ISteppers can be added before or after the call to Sk.initialize, and this does affect when the IStepper.initialize
/// call will be made! IStepper.initialize is always called after Sk.initialize. This can be important to note when
/// writing code that uses SK functions that are dependant on initialization, you’ll want to avoid putting this code in
/// the constructor, and add them to Initialize instead.
///
/// ISteppers also pay attention to threading! Initialize and Step always happen on the main thread, even if the
/// constructor is called on a different one.
/// <https://stereokit.net/Pages/StereoKit.Framework/IStepper.html>
///
/// see also [`crate::IStepper`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ font::Font, material::Material, maths::{Matrix, Quat, Vec3},
///                       mesh::Mesh, system::{Text, TextStyle}, util::named_colors};
///
/// /// The basic Stepper.
/// pub struct AStepper {
///     id: StepperId,
///     sk_info: Option<Rc<RefCell<SkInfo>>>,
///     pub transform: Matrix,
///     round_cube: Option<Mesh>,
///     pub text: String,
///     text_style: Option<TextStyle>,
/// }
///
/// unsafe impl Send for AStepper {}
///
/// /// This code may be called in some threads or before sk_init()
/// /// so no StereoKit assets here.
/// impl Default for AStepper {
///     fn default() -> Self {
///         Self {
///             id: "AStepper".to_string(),
///             sk_info: None,
///             transform: Matrix::r([0.0, 180.0, 0.0]),
///             round_cube: None,
///             text: "IStepper\ntrait".to_owned(),
///             text_style: None,
///         }
///     }
/// }
///
/// /// All the code here run in the main thread
/// impl IStepper for AStepper {
///     fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
///         self.id = id;
///         self.sk_info = Some(sk_info);
///         self.round_cube = Some(Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.02, None));
///         self.text_style = Some(Text::make_style(Font::default(), 0.3, named_colors::BLACK));
///
///         true
///     }
///
///     fn step(&mut self, token: &MainThreadToken) {
///         if let Some(round_cube) = &self.round_cube {
///             round_cube.draw(token, Material::pbr(),
///                             self.transform, Some(named_colors::RED.into()), None);
///         }
///         Text::add_at(token, &self.text, self.transform, self.text_style,
///                      None, None, None, None, None, None);
///     }
/// }
///
/// sk.send_event(StepperAction::add_default::<AStepper>("My_Basic_Stepper_ID"));
///
/// filename_scr = "screenshots/a_stepper.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     // No code here as we only use AStepper
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/a_stepper.jpeg" alt="screenshot" width="200">
pub trait IStepper {
    /// This is called by StereoKit at the start of the next frame, and on the main thread. This happens before
    /// StereoKit’s main Step callback, and always after Sk.initialize.
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper/Initialize.html>
    /// id : The id of the stepper
    /// sk : The SkInfo of the runnin Sk instance.
    ///
    /// Return true if the initiaization was succesfull or need to run on many steps and/or in another thread
    /// (see [`IStepper::initialize_done`])
    fn initialize(&mut self, id: StepperId, sk: Rc<RefCell<SkInfo>>) -> bool;

    /// If initialization is to be performed in multiple steps, with or without threads and in order to avoid black or
    /// frozen screens, write the on going initialization here
    ///
    /// Return false as long as the initialization must keep going then true when the stepper has to be drawn.
    fn initialize_done(&mut self) -> bool {
        true
    }

    /// Is this IStepper enabled? When false, StereoKit will not call Step. This can be a good way to temporarily
    /// disable the IStepper without removing or shutting it down.
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper/Enabled.html>
    fn enabled(&self) -> bool {
        true
    }

    /// This Step method will be called every frame of the application, as long as Enabled is true. This happens
    /// immediately before the main application’s Step callback.
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper/Step.html>
    fn step(&mut self, token: &MainThreadToken);

    /// This is called when the IStepper is removed, or the application shuts down. This is always called on the main
    /// thread, and happens at the start of the next frame, before the main application’s Step callback.
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper/Shutdown.html>
    fn shutdown(&mut self) {}

    /// If shutdown is to be performed in multiple steps, with or without threads and in order to avoid black or
    /// frozen screens, write the on going shutdown here
    ///
    /// Return false as long as the shutdown must keep going then true when the stepper can be remove.
    fn shutdown_done(&mut self) -> bool {
        true
    }
}

/// Steppers actions list. These are the events you can trigger from any threads. There are 3 ways to
/// trigger an action:
/// - Main thread/program where you can use sk: [Sk::send_event] or [crate::sk::Sk::quit] directly.
/// - From any IStepper you should use [SkInfo::send_event].
/// - From any threads you should use [EventLoopProxy::send_event]. You can get a proxy clone
///   from [SkInfo::get_event_loop_proxy] or [Sk::get_event_loop_proxy].
///
/// <https://stereokit.net/Pages/StereoKit/SK.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
/// use std::any::TypeId; // we need this to remove all the steppers of a given type.
///
/// let mut title = Title::new("StepperAction", Some(named_colors::GREEN), None, None);
/// title.transform = Matrix::t_r([-1.0, 0.0, -1.0], [0.0, 155.0, 0.0]);
/// sk.send_event(StepperAction::add("Title_green_ID", title.clone()));
///
/// sk.send_event(StepperAction::add_default::<Title>("Title_white_ID"));
///
/// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
/// sk.send_event(StepperAction::event("main thread".into(), SHOW_SCREENSHOT_WINDOW, "true"));
///
/// filename_scr = "screenshots/stepper_actions.jpeg";
/// number_of_steps = 4;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     if iter == 0 {
///        assert_eq!(sk.get_steppers_count(), 3);
///        sk.send_event(StepperAction::remove("Title_white_ID"));
///    } else if iter == 1 {
///        assert_eq!(sk.get_steppers_count(), 2);
///        sk.send_event(StepperAction::remove_all(TypeId::of::<Title>()));
///    } else if iter == 2 {
///        assert_eq!(sk.get_steppers_count(), 1);
///        sk.send_event(StepperAction::add("Title_green_ID", title.clone()));
///    } else if iter < number_of_steps + 2{
///        assert_eq!(sk.get_steppers_count(), 2);
///    }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_actions.jpeg" alt="screenshot" width="200">
pub enum StepperAction {
    /// Add a new stepper of TypeID,  identified by its StepperID
    Add(Box<dyn for<'a> IStepper + Send + 'static>, TypeId, StepperId),
    /// Remove all steppers of TypeID
    RemoveAll(TypeId),
    /// Remove the stepper identified by its StepperID
    Remove(StepperId),
    /// Quit the app,
    Quit(StepperId, String),
    /// Event sent by a stepper for those who need it.
    /// Key -> Value  are strings.
    Event(StepperId, String, String),
}

impl fmt::Debug for StepperAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepperAction::Add(_stepper, _id, stepper_id) => {
                write!(f, "StepperAction::Add(..., type_id: ... , stepper_id:{stepper_id:?} )")
            }
            StepperAction::RemoveAll(type_id) => write!(f, "StepperAction::RemoveAll( type_id:{type_id:?} )"),
            StepperAction::Remove(stepper_id) => write!(f, "StepperAction::Remove( id:{stepper_id:?} )"),
            StepperAction::Quit(stepper_id, reason) => {
                write!(f, "StepperAction::Quit() sent by id:{stepper_id:?} for reason '{reason}'")
            }
            StepperAction::Event(stepper_id, key, value) => {
                write!(f, "StepperAction::Event( id:{stepper_id:?} => {key}->{value} )")
            }
        }
    }
}

impl StepperAction {
    /// This instantiates and registers an instance of the IStepper type provided as the generic parameter. SK will hold
    /// onto it, Initialize it, Step it every frame, and call Shutdown when the application ends. This is generally safe
    /// to do before Sk.initialize is called, the constructor is called right away, and Initialize is called right after
    /// Sk.initialize, or at the start of the next frame before the next main Step callback if SK is already initialized.
    /// <https://stereokit.net/Pages/StereoKit/SK/AddStepper.html>
    /// * `stepper_id` - The id to give to the stepper.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     } else if iter == number_of_steps + 2 {
    ///         /// We sk.quit() at 4 and at 5 the stepper has been removed.
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     } else {
    ///         panic!("there is not iter 6 !!!");
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add_default<T: IStepper + Send + Default + 'static>(stepper_id: impl AsRef<str>) -> Self {
        let stepper = <T>::default();
        let stepper_type = stepper.type_id();
        StepperAction::Add(Box::new(stepper), stepper_type, stepper_id.as_ref().to_owned())
    }

    /// This instantiates and registers an instance of the IStepper type provided as the generic parameter. SK will hold
    /// onto it, Initialize it, Step it every frame, and call Shutdown when the application ends. This is generally safe
    /// to do before Sk.initialize is called, the constructor is called right away, and Initialize is called right after
    /// Sk.initialize, or at the start of the next frame before the next main Step callback if SK is already initialized.
    /// <https://stereokit.net/Pages/StereoKit/SK/AddStepper.html>
    /// * `stepper_id` - The id of the stepper.
    /// * `stepper` - The stepper to add.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
    ///                      tools::title::Title, };
    /// use std::any::TypeId; // we need this to remove all the steppers of a given type.
    ///
    /// let mut title = Title::new("Stepper 1", Some(named_colors::GREEN), None, None);
    /// title.transform = Matrix::t_r([0.0, 0.0, -1.0], [0.0, 135.0, 0.0]);
    /// sk.send_event(StepperAction::add("Title_green_ID", title.clone()));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     } else if iter == number_of_steps + 2 {
    ///         /// We sk.quit() at 4 and at 5 the stepper has been removed.
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     } else {
    ///         panic!("there is not iter 6 !!!");
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add<T: IStepper + Send + 'static>(stepper_id: impl AsRef<str>, stepper: T) -> Self {
        let stepper_type = stepper.type_id();
        StepperAction::Add(Box::new(stepper), stepper_type, stepper_id.as_ref().to_string())
    }

    /// This removes all IStepper instances that are assignable to the generic type specified. This will call the
    /// IStepper’s Shutdown method on each removed instance before returning.
    /// <https://stereokit.net/Pages/StereoKit/SK/RemoveStepper.html>
    /// * `type_id` - The type of the steppers to remove.
    ///
    /// ### Examples
    /// ```no_run
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::{title::Title, screenshot::ScreenshotViewer};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    /// sk.send_event(StepperAction::add_default::<Title>("Title_white_ID1"));
    /// sk.send_event(StepperAction::add_default::<Title>("Title_white_ID2"));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter == number_of_steps  {
    ///         assert_eq!(sk.get_steppers_count(), 3);
    ///         sk.send_event(StepperAction::remove_all(std::any::TypeId::of::<Title>()));
    ///     } else if iter == number_of_steps + 1 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn remove_all(type_id: TypeId) -> Self {
        StepperAction::RemoveAll(type_id)
    }

    /// This removes one or all IStepper instances that are assignable to the generic type specified. This will call the
    /// IStepper’s Shutdown method on each removed instance before returning.
    /// <https://stereokit.net/Pages/StereoKit/SK/RemoveStepper.html>
    /// * `stepper_id` - The id of the stepper to remove.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::{title::Title, screenshot::ScreenshotViewer};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    /// sk.send_event(StepperAction::add_default::<Title>("Title_white_ID1"));
    /// sk.send_event(StepperAction::add_default::<Title>("Title_white_ID2"));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter == number_of_steps  {
    ///         assert_eq!(sk.get_steppers_count(), 3);
    ///         sk.send_event(StepperAction::remove("Title_white_ID1"));
    ///     } else if iter == number_of_steps + 1 {
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn remove(stepper_id: impl AsRef<str>) -> Self {
        StepperAction::Remove(stepper_id.as_ref().to_string())
    }

    /// Quit the app,
    /// <https://stereokit.net/Pages/StereoKit/SK/Quit.html>
    /// * `stepper_id` - The id of the stepper that ask to quit.
    /// * `reason` - The reason why the stepper ask to quit.
    ///
    /// see also [`Sk::quit`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter == number_of_steps {
    ///         // Same as Sk::quit()
    ///         sk.send_event(StepperAction::quit("ScreenshotViewer_ID", "test_quit"));
    ///     } else if iter >= number_of_steps + 1 {
    ///         /// We sk.quit() at 4 and at 5 the stepper has been removed.
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     } else {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn quit(stepper_id: impl AsRef<str>, reason: impl AsRef<str>) -> Self {
        StepperAction::Quit(stepper_id.as_ref().to_string(), reason.as_ref().to_string())
    }

    /// Event sent by a stepper for those who need it.
    /// Key -> Value  are strings.
    /// <https://stereokit.net/Pages/StereoKit/SK/SendEvent.html>
    /// * `stepper_id` - The id of the stepper that send the event or is the target of the event. This is useful for
    ///   filter the event.
    /// * `key` - The key of the event.
    /// * `value` - The value of the event.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tools::screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    /// sk.send_event(StepperAction::event("main thread", SHOW_SCREENSHOT_WINDOW, "true"));
    ///
    /// test_steps!(  // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     } else if iter == number_of_steps + 2 {
    ///         /// We sk.quit() at 4 and at 5 the stepper has been removed.
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     } else {
    ///         panic!("there is not iter 6 !!!");
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn event<S: AsRef<str>>(stepper_id: S, key: S, value: S) -> Self {
        StepperAction::Event(stepper_id.as_ref().to_string(), key.as_ref().to_owned(), value.as_ref().to_owned())
    }
}

/// State of the stepper
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepperState {
    /// The stepper is being initialized.
    Initializing,
    /// Initialization is complete, and the stepper is running.
    Running,
    /// The stepper is being removed.
    Closing,
}

/// All you need to step a Stepper, then remove it
pub struct StepperHandler {
    id: StepperId,
    type_id: TypeId,
    stepper: Box<dyn IStepper>,
    state: StepperState,
}
impl StepperHandler {
    /// Get the stepper id
    pub fn get_id(&self) -> &StepperId {
        &self.id
    }

    /// Get the stepper type_id
    pub fn get_type_id(&self) -> TypeId {
        self.type_id
    }
    /// Get the stepper state
    pub fn get_state(&self) -> StepperState {
        self.state
    }
}

/// A lazy way to identify IStepper instances
pub type StepperId = String;
/// Event indicating that the stepper is running.
pub const ISTEPPER_RUNNING: &str = "IStepper_Running";
/// Event indicating that the stepper is removed.
pub const ISTEPPER_REMOVED: &str = "IStepper_Removed";

/// Steppers manager. This is used internally by StereoKit, but you can use it to prepare your next scene managers.
/// Non canonical.
/// <https://stereokit.net/Pages/StereoKit.Framework/IStepper.html>
///
/// ### Example
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors, framework::Steppers,
///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
///
/// let mut steppers = Steppers::new(sk.get_sk_info_clone());
///
/// let mut title = Title::new("Steppers", Some(named_colors::BLUE), None, None);
/// title.transform = Matrix::t_r([-0.5, 0.5, -1.5], [0.0, 155.0, 0.0]);
/// steppers.send_event(StepperAction::add("Title_blue_ID1", title.clone()));
///
/// title.transform = Matrix::t_r([-0.5, -0.5, -1.5], [0.0, 245.0, 0.0]);
/// // We may use the same ID for different steppers
/// steppers.send_event(StepperAction::add("Title_blue_ID2", title.clone()));
/// sk      .send_event(StepperAction::add("Title_blue_ID2", title.clone()));
///
/// steppers.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
/// steppers.send_event(StepperAction::event("main thread".into(), SHOW_SCREENSHOT_WINDOW, "true"));
///
/// filename_scr = "screenshots/steppers.jpeg";
/// number_of_steps = 4;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     if iter == 0 {
///         assert_eq!(steppers.get_count(), 0);
///         assert_eq!(sk.get_steppers_count(), 1);
///         sk.swap_steppers(&mut steppers);
///     } else if iter == 1 {
///         assert_eq!(steppers.get_count(), 1);
///         assert_eq!(sk.get_steppers_count(), 3);
///         sk.swap_steppers(&mut steppers);
///     } else if iter == 2 {
///         assert_eq!(steppers.get_count(), 3);
///         assert_eq!(sk.get_steppers_count(), 1);
///         sk.swap_steppers(&mut steppers);
///     } else if iter == number_of_steps + 1 {
///         assert_eq!(sk.get_steppers_count(), 3);
///         assert_eq!(steppers.get_count(), 1);
///         steppers.shutdown();
///     } else if iter == number_of_steps + 2 {
///         /// all the steppers should be removed from Sk and steppers
///         assert_eq!(sk.get_steppers_count(), 0);
///         assert_eq!(steppers.get_count(), 0);
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/steppers.jpeg" alt="screenshot" width="200">
pub struct Steppers {
    sk_info: Rc<RefCell<SkInfo>>,
    running_steppers: Vec<StepperHandler>,
    stepper_actions: VecDeque<StepperAction>,
}

impl Steppers {
    /// The only way to create a Steppers manager. You don't need it unless you want to swap different steppers.
    /// * `sk_info` - A SkInfo Rc clone of the running Sk instance.
    ///
    /// [Sk] Create the default one when it is initialized.
    /// ### Example
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, framework::Steppers};
    ///
    /// let mut steppers = Steppers::new(sk.get_sk_info_clone());
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(steppers.get_count(), 0);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new(sk_info: Rc<RefCell<SkInfo>>) -> Self {
        Self { sk_info, running_steppers: vec![], stepper_actions: VecDeque::new() }
    }

    /// Push an action to consumme befor next frame
    /// * `action` - The action to push.
    ///
    /// see also [Sk::send_event] [EventLoopProxy::send_event]
    /// ### Example
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors, framework::Steppers,
    ///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
    ///
    /// let mut steppers = Steppers::new(sk.get_sk_info_clone());
    ///
    /// let mut title = Title::new("Steppers", Some(named_colors::BLUE), None, None);
    /// steppers.send_event(StepperAction::add("Title_blue_ID1", title.clone()));
    /// steppers.send_event(StepperAction::add("Title_blue_ID2", title.clone()));
    /// steppers.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    /// steppers.send_event(StepperAction::event("main thread".into(), SHOW_SCREENSHOT_WINDOW, "true"));
    /// steppers.send_event(StepperAction::remove("Title_blue_ID2"));
    ///
    /// sk.swap_steppers(&mut steppers);
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(steppers.get_count(), 0);
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn send_event(&mut self, action: StepperAction) {
        self.stepper_actions.push_back(action);
    }

    /// Deque all the actions, create the frame event report, execute all the steppers if quit hasn't be asked
    /// return false if sk_quit must be triggered.
    /// * token - The token where the event report will be created for this frame.
    ///
    /// This must be call from the running [Sk] instance only.
    pub(crate) fn step(&mut self, token: &mut MainThreadToken) -> bool {
        while let Some(action) = self.stepper_actions.pop_front() {
            match action {
                StepperAction::Add(mut stepper, type_id, stepper_id) => {
                    if stepper.initialize(stepper_id.clone(), self.sk_info.clone()) {
                        let stepper_h =
                            StepperHandler { id: stepper_id, type_id, stepper, state: StepperState::Initializing };
                        self.running_steppers.push(stepper_h);
                    } else {
                        Log::warn(format!("Stepper {stepper_id} did not initialize"));
                        token.event_report.push(StepperAction::event(stepper_id.as_str(), ISTEPPER_REMOVED, "false"));
                    }
                }
                StepperAction::RemoveAll(stepper_type) => {
                    for stepper_h in
                        self.running_steppers.iter_mut().filter(|stepper_h| stepper_h.type_id == stepper_type)
                    {
                        stepper_h.stepper.shutdown();
                        stepper_h.state = StepperState::Closing;
                    }
                }
                StepperAction::Remove(stepper_id) => {
                    for stepper_h in self.running_steppers.iter_mut().filter(|stepper_h| stepper_h.id == stepper_id) {
                        stepper_h.stepper.shutdown();
                        stepper_h.state = StepperState::Closing;
                    }
                }
                StepperAction::Quit(from, reason) => {
                    Log::info(format!("Quit sent by {from} for reason: {reason}"));
                    return false;
                }
                _ => token.event_report.push(action),
            }
        }

        // 1 - Managing the not running steppers.
        let mut removed_steppers = vec![];
        for stepper_h in &mut self.running_steppers {
            match stepper_h.state {
                StepperState::Initializing => {
                    if stepper_h.stepper.initialize_done() {
                        Log::info(format!("Stepper {} is initialized.", &stepper_h.id));
                        stepper_h.state = StepperState::Running;
                        token.event_report.push(StepperAction::event(stepper_h.id.as_str(), ISTEPPER_RUNNING, "true"));
                    }
                }
                StepperState::Running => (),
                StepperState::Closing => {
                    if stepper_h.stepper.shutdown_done() {
                        removed_steppers.push(stepper_h.id.clone());
                        token.event_report.push(StepperAction::event(stepper_h.id.as_str(), ISTEPPER_REMOVED, "true"));
                    }
                }
            }
        }
        self.running_steppers.retain(|stepper_h| {
            if removed_steppers.contains(&stepper_h.id) {
                Log::info(format!("Stepper {} is removed.", &stepper_h.id));
                false
            } else {
                true
            }
        });

        // 2 - Running the Running steppers
        for stepper_h in
            &mut self.running_steppers.iter_mut().filter(|stepper_h| stepper_h.state == StepperState::Running)
        {
            stepper_h.stepper.step(token)
        }

        true
    }

    /// An enumerable list of all currently active ISteppers registered with StereoKit. This does not include Steppers
    /// that have been added, but are not yet initialized. Stepper initialization happens at the beginning of the frame,
    /// before the app's Step.
    ///
    /// see also [Sk::get_steppers] to get the default running Steppers.
    /// ### Example
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
    ///     framework::{Steppers, StepperState, StepperHandler},
    ///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
    /// use std::any::TypeId;
    ///
    /// let mut steppers = Steppers::new(sk.get_sk_info_clone());
    ///
    /// let mut title = Title::new("Steppers", Some(named_colors::BLUE), None, None);
    /// steppers.send_event(StepperAction::add("Title_blue_ID1", title.clone()));
    /// steppers.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert_eq!(steppers.get_stepper_handlers().len(), 0);
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///         sk.swap_steppers(&mut steppers);
    ///     } else if iter == 1 {
    ///         assert_eq!(steppers.get_stepper_handlers().len(), 0);
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///         sk.swap_steppers(&mut steppers);
    ///     } else if iter == 3 {
    ///         let steppers_handlers = steppers.get_stepper_handlers();
    ///         assert_eq!(steppers_handlers.len(), 2);
    ///         assert_eq!(steppers_handlers[0].get_id(), "Title_blue_ID1");
    ///         assert_eq!(steppers_handlers[0].get_type_id(), TypeId::of::<Title>());
    ///         assert_eq!(steppers_handlers[0].get_state(), StepperState::Running);
    ///
    ///         assert_eq!(steppers_handlers[1].get_id(), "ScreenshotViewer_ID");
    ///         assert_eq!(steppers_handlers[1].get_type_id(), TypeId::of::<ScreenshotViewer>());
    ///         assert_eq!(steppers_handlers[1].get_state(), StepperState::Running);
    ///
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_stepper_handlers(&self) -> &[StepperHandler] {
        self.running_steppers.as_slice()
    }

    /// Run the shutdown code for all running steppers.
    /// On default [Sk] Steppers, this is called when pushing StepperAction::Quit( origin , reason) but if you have other
    /// inactive [Steppers] you may want to call this function to shutdown all of them.
    /// Do not confuse with [Sk::shutdown] that you must call when exiting the program.
    ///
    /// see also [Sk::quit]
    /// ### Example
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
    ///     framework::{Steppers, StepperState, StepperHandler},
    ///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
    /// use std::any::TypeId;
    ///
    /// let mut steppers = Steppers::new(sk.get_sk_info_clone());
    ///
    /// let mut title = Title::new("Steppers", Some(named_colors::BLUE), None, None);
    /// steppers.send_event(StepperAction::add("Title_blue_ID1", title.clone()));
    /// steppers.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    ///
    /// sk.swap_steppers(&mut steppers);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert_eq!(steppers.get_count(), 0);
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///         sk.swap_steppers(&mut steppers);
    ///     } else if iter == 1 {
    ///         assert_eq!(steppers.get_count(), 2);
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///         steppers.shutdown();
    ///         sk.swap_steppers(&mut steppers);
    ///     } else {
    ///         assert_eq!(steppers.get_count(), 0);
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn shutdown(&mut self) {
        self.stepper_actions.clear();
        for stepper_h in self.running_steppers.iter_mut() {
            Log::diag(format!("Closing {}", stepper_h.id));
            stepper_h.stepper.shutdown();
            stepper_h.state = StepperState::Closing;
        }
        // 2 - Waiting for the shutdowns
        for _iter in 0..50 {
            let mut removed_steppers = vec![];
            for stepper_h in
                &mut self.running_steppers.iter_mut().filter(|stepper_h| stepper_h.state == StepperState::Closing)
            {
                if stepper_h.stepper.shutdown_done() {
                    removed_steppers.push(stepper_h.id.clone());
                }
            }
            self.running_steppers.retain(|stepper_h| {
                if removed_steppers.contains(&stepper_h.id) {
                    Log::info(format!("Stepper {} is removed.", &stepper_h.id));
                    false
                } else {
                    true
                }
            });
            if self.running_steppers.is_empty() {
                break;
            }
            sleep(Duration::from_millis(100));
        }
        self.running_steppers.clear();
    }

    /// The count of all ISteppers registered by this [Steppers]. This does not include Steppers
    /// that have been added, but are not yet initialized. Stepper initialization happens at the beginning of the frame,
    /// before the app's Step.
    ///
    /// see also [Sk::get_steppers_count]
    /// ### Example
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
    ///     framework::{Steppers, StepperState, StepperHandler},
    ///     tools::{title::Title, screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW}}, };
    /// use std::any::TypeId;
    ///
    /// let mut steppers = Steppers::new(sk.get_sk_info_clone());
    ///
    /// let mut title = Title::new("Steppers", Some(named_colors::BLUE), None, None);
    /// steppers.send_event(StepperAction::add("Title_blue_ID1", title.clone()));
    /// steppers.send_event(StepperAction::add_default::<ScreenshotViewer>("ScreenshotViewer_ID"));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(steppers.get_stepper_handlers().len(),  steppers.get_count());
    ///     if iter == 0 {
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///         sk.swap_steppers(&mut steppers);
    ///     } else if iter == 1 {
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///         sk.swap_steppers(&mut steppers);
    ///     } else if iter == 3 {
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_count(&self) -> usize {
        self.running_steppers.len()
    }
}

/// Helper to create the whole code of a Stepper in method IStepper::initialize() while avoiding multiple fields.
///
/// Not really convincing with too many compromises.
/// The privileged solution is the use of the derive macro [crate::IStepper]
///
/// See Demo b_stepper.rs::BStepper
/// Non canonical structure
///
/// ### Examples
/// ```
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{font::Font, framework::StepperClosures, material::Material,
///                      maths::{Matrix, Quat, Vec3},mesh::Mesh, system::{Renderer, Text},
///                      util::{Time, named_colors},};
///
/// pub struct BStepper {
///     id: StepperId,
///     sk_info: Option<Rc<RefCell<SkInfo>>>,
///     pub text: String,
///     closures: StepperClosures<'static>,
/// }
///
/// unsafe impl Send for BStepper {}
///
/// /// This code may be called in some threads, so no StereoKit code
/// impl Default for BStepper {
///     fn default() -> Self {
///         Self {
///             id: "BStepper".to_string(),
///             sk_info: None,
///             text: "\nStepperClosures".to_owned(),
///             closures: StepperClosures::new(),
///         }
///     }
/// }
///
/// /// All the code here run in the main thread
/// impl IStepper for BStepper {
///     fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
///         self.id = id;
///         self.sk_info = Some(sk_info);
///
///         let mut transform = Matrix::t_r([0.0, 1.0, -0.5], [0.0, 180.0, 0.0]);
///         let mut round_cube = Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.005, Some(16));
///         round_cube.id("round_cube BStepper");
///         let text_style = Some(Text::make_style(Font::default(), 0.3, named_colors::GOLD));
///         let text = self.text.clone();
///
///         self.closures.set(
///             move |token| {
///                 Renderer::add_mesh(token, &round_cube, Material::pbr(),
///                                    transform, Some(named_colors::RED.into()), None);
///                 Text::add_at(token, &text, transform, text_style,
///                              None, None, None, None, None, None);
///                 // (1) You cannot do that here: self.text = "youpi".into();
///             },
///             || Log::diag("Closing Stepper B !!!"),
///         );
///         true
///     }
///
///     fn step(&mut self, token: &MainThreadToken) {
///         self.closures.step(token);
///         // (2) Add here The code about fields that are shared with shutdown >>>
///         {}
///         //<<<
///     }
///
///     fn shutdown(&mut self) {
///         self.closures.shutdown();
///         // (3) Add here The code about fields that are shared with step >>>
///         {}
///         //<<<
///     }
/// }
///
/// sk.send_event(StepperAction::add_default::<BStepper>("My_Basic_Stepper_ID"));
///
/// filename_scr = "screenshots/stepper_closures.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     // No code here as we only use BStepper
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_closures.jpeg" alt="screenshot" width="200">
pub struct StepperClosures<'a> {
    on_step: Box<dyn FnMut(&MainThreadToken) + 'a>,
    shutdown: Box<dyn FnMut() + 'a>,
}

impl Default for StepperClosures<'_> {
    /// Same as new, but with a more explicit name
    fn default() -> Self {
        Self { on_step: Box::new(|_token| {}), shutdown: Box::new(|| {}) }
    }
}

impl StepperClosures<'_> {
    /// Same as default, but with a more explicit name
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    /// Set the stepper closures.
    /// * `on_step` - The closure to call on each step.
    /// * `on_shutdown` - The closure to call on shutdown.
    ///
    pub fn set<U: FnMut(&MainThreadToken) + 'static, S: FnMut() + 'static>(
        &mut self,
        on_step: U,
        on_shutdown: S,
    ) -> &mut Self {
        self.on_step = Box::new(on_step);
        self.shutdown = Box::new(on_shutdown);
        self
    }

    /// To call on each step.
    /// * `token` - The main thread token.
    pub fn step(&mut self, token: &MainThreadToken) {
        (self.on_step)(token)
    }

    /// To call on shutdown.
    pub fn shutdown(&mut self) {
        (self.shutdown)()
    }
}
