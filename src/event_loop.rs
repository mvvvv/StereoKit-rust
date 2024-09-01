use crate::{
    sk::{sk_step, AppFocus, MainThreadToken, Sk, SkInfo},
    system::{Input, Log},
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::VecDeque,
    fmt,
    rc::Rc,
    thread::sleep,
    time::Duration,
};
pub use winit;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

type OnStepClosure<'a> = Box<dyn FnMut(&mut Sk, &MainThreadToken) + 'a>;

#[derive(PartialEq)]
enum SleepPhase {
    Sleeping,
    WakingUp,
    WokeUp,
}

/// What winit v0.30 want is : run_app()
///
pub struct SkClosures<'a> {
    sk: Sk,
    token: MainThreadToken,
    on_step: OnStepClosure<'a>,
    on_sleeping_step: OnStepClosure<'a>,
    shutdown: Box<dyn FnMut(&mut Sk) + 'a>,
    window_id: Option<WindowId>,
    sleeping: SleepPhase,
}

impl ApplicationHandler<StepperAction> for SkClosures<'_> {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, user_event: StepperAction) {
        Log::diag(format!("UserEvent {:?}", user_event));
        self.sk.push_action(user_event);
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        Log::info("Resumed !!");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if event == WindowEvent::Destroyed {
            Log::info(format!("SkClosure Window {:?} Destroyed !!!", window_id));
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                Log::diag("RedrawRequested: Time to wake up");
                self.sleeping = SleepPhase::WakingUp;
            }
            WindowEvent::Focused(value) => {
                self.window_id = Some(window_id);
                if value {
                    Log::diag("GainedFocus: Time to wake up");
                    self.sleeping = SleepPhase::WakingUp;
                }
            }
            WindowEvent::CloseRequested => {
                Log::info("SkClosure LoopExiting !!");
                (self.shutdown)(&mut self.sk);
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => match &event.state {
                winit::event::ElementState::Pressed => {}
                winit::event::ElementState::Released => {
                    Input::text_inject_chars(event.logical_key.to_text().unwrap_or("?"));
                }
            },
            _ => (),
        }
    }

    // commented due to indiscretion
    // fn device_event(&mut self, _event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
    //     Log::diag(format!("SkClosure DeviceEvent {:?} -> {:?}", device_id, event));
    // }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.sk.get_app_focus() == AppFocus::Hidden && self.sleeping == SleepPhase::WokeUp {
            self.sleeping = SleepPhase::Sleeping;
            Log::diag("Time to sleep")
        }
        match self.sleeping {
            SleepPhase::WokeUp => {
                self.step(event_loop);
            }
            SleepPhase::WakingUp => {
                self.step(event_loop);
                if self.sk.get_app_focus() != AppFocus::Hidden {
                    self.sleeping = SleepPhase::WokeUp;
                    Log::diag("WokeUp");
                }
            }
            SleepPhase::Sleeping => {
                sleep(Duration::from_millis(200));
                (self.on_sleeping_step)(&mut self.sk, &self.token);
            }
        }
    }

    // commented because it floods the log
    // fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
    //     Log::diag(format!("New events :{:?}", cause));
    // }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        Log::info("SkClosure Suspended !!");
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        Log::info("SkClosure Exiting !!");
    }

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
        Log::warn("SkClosure Memory Warning !!");
    }
}

impl<'a> SkClosures<'a> {
    fn step(&mut self, event_loop: &ActiveEventLoop) {
        if unsafe { sk_step(None) } == 0 {
            self.window_event(event_loop, self.window_id.unwrap_or(WindowId::dummy()), WindowEvent::CloseRequested);
        }
        if !self.sk.steppers.step(&mut self.token) {
            self.sk.quit(None)
        };
        while let Some(mut action) = self.sk.actions.pop_front() {
            action();
        }
        (self.on_step)(&mut self.sk, &self.token);
    }

    /// Common way to run the loop with step and shutdown
    /// If you need a process when the headset is going to sleep use new(..).on_hidden_step().run()
    pub fn run_app<U: FnMut(&mut Sk, &MainThreadToken) + 'a, S: FnMut(&mut Sk) + 'a>(
        sk: Sk,
        event_loop: EventLoop<StepperAction>,
        step: U,
        shutdown: S,
    ) {
        let mut this = Self {
            sk,
            on_step: Box::new(step),
            on_sleeping_step: Box::new(|_sk, _main_thread| {}),
            shutdown: Box::new(shutdown),
            token: MainThreadToken {
                #[cfg(feature = "event-loop")]
                event_report: vec![],
            },
            window_id: None,
            sleeping: SleepPhase::WakingUp,
        };
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Err(err) = event_loop.run_app(&mut this) {
            Log::err(format!("event_loop.run_app returned with an error : {:?}", err));
        }
    }

    pub fn new<U: FnMut(&mut Sk, &MainThreadToken) + 'a>(sk: Sk, step: U) -> Self {
        Self {
            sk,
            on_step: Box::new(step),
            on_sleeping_step: Box::new(|_sk, _main_thread| {}),
            shutdown: Box::new(|_sk| {}),
            token: MainThreadToken {
                #[cfg(feature = "event-loop")]
                event_report: vec![],
            },
            window_id: None,
            sleeping: SleepPhase::WakingUp,
        }
    }

    pub fn on_sleeping_step<U: FnMut(&mut Sk, &MainThreadToken) + 'a>(&mut self, on_sleeping_step: U) -> &mut Self {
        self.on_sleeping_step = Box::new(on_sleeping_step);
        self
    }

    pub fn shutdown<S: FnMut(&mut Sk) + 'a>(&mut self, shutdown: S) -> &mut Self {
        self.shutdown = Box::new(shutdown);
        self
    }

    pub fn run(&mut self, event_loop: EventLoop<StepperAction>) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Err(err) = event_loop.run_app(self) {
            Log::err(format!("event_loop.run_app returned with an error : {:?}", err));
        }
    }
}

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
pub trait IStepper {
    /// This is called by StereoKit at the start of the next frame, and on the main thread. This happens before
    /// StereoKit’s main Step callback, and always after Sk.initialize.
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper/Initialize.html>
    fn initialize(&mut self, id: StepperId, sk: Rc<RefCell<SkInfo>>) -> bool;

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
}

/// List of action on steppers. This is the user events
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
    /// Suspended
    Suspended,
    /// Resume
    Resumed,
}

impl fmt::Debug for StepperAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepperAction::Add(_stepper, _id, stepper_id) => {
                write!(f, "StepperAction::Add(..., type_id: ... , stepper_id:{:?} )", stepper_id)
            }
            StepperAction::RemoveAll(type_id) => write!(f, "StepperAction::RemoveAll( type_id:{:?} )", type_id),
            StepperAction::Remove(stepper_id) => write!(f, "StepperAction::Remove( id:{:?} )", stepper_id),
            StepperAction::Quit(stepper_id, reason) => {
                write!(f, "StepperAction::Quit() sent by id:{:?} for reason '{}'", stepper_id, reason)
            }
            StepperAction::Event(stepper_id, key, value) => {
                write!(f, "StepperAction::Event( id:{:?} => {}->{} )", stepper_id, key, value)
            }
            StepperAction::Suspended => write!(f, "StepperAction::Suspended"),
            StepperAction::Resumed => write!(f, "StepperAction::Resumed"),
        }
    }
}

impl StepperAction {
    /// This instantiates and registers an instance of the IStepper type provided as the generic parameter. SK will hold
    /// onto it, Initialize it, Step it every frame, and call Shutdown when the application ends. This is generally safe
    /// to do before Sk.initialize is called, the constructor is called right away, and Initialize is called right after
    /// Sk.initialize, or at the start of the next frame before the next main Step callback if SK is already initialized.
    /// <https://stereokit.net/Pages/StereoKit/SK/AddStepper.html>
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
    pub fn add<T: IStepper + Send + 'static>(stepper_id: impl AsRef<str>, stepper: T) -> Self {
        let stepper_type = stepper.type_id();
        StepperAction::Add(Box::new(stepper), stepper_type, stepper_id.as_ref().to_string())
    }

    /// This removes all IStepper instances that are assignable to the generic type specified. This will call the
    /// IStepper’s Shutdown method on each removed instance before returning.
    /// <https://stereokit.net/Pages/StereoKit/SK/RemoveStepper.html>
    pub fn remove_all(type_id: TypeId) -> Self {
        StepperAction::RemoveAll(type_id)
    }

    /// This removes one or all IStepper instances that are assignable to the generic type specified. This will call the
    /// IStepper’s Shutdown method on each removed instance before returning.
    /// <https://stereokit.net/Pages/StereoKit/SK/RemoveStepper.html>
    pub fn remove(stepper_id: impl AsRef<str>) -> Self {
        StepperAction::Remove(stepper_id.as_ref().to_string())
    }

    pub fn event<S: AsRef<str>>(stepper_id: StepperId, key: S, value: S) -> Self {
        StepperAction::Event(stepper_id, key.as_ref().to_owned(), value.as_ref().to_owned())
    }
}

/// All you need to step a Stepper, then remove it
pub struct StepperHandler {
    id: StepperId,
    type_id: TypeId,
    stepper: Box<dyn IStepper>,
}

/// A lazy way to identify IStepper instances
pub type StepperId = String;

/// Steppers manager. Non canonical way you can create a scene with all the Steppers you need
/// <https://stereokit.net/Pages/StereoKit.Framework/IStepper.html<
#[cfg(feature = "event-loop")]
pub struct Steppers {
    sk: Rc<RefCell<SkInfo>>,
    stepper_handlers: Vec<StepperHandler>,
    stepper_actions: VecDeque<StepperAction>,
}

#[cfg(feature = "event-loop")]
impl Steppers {
    // the only way to create a Steppers manager
    pub fn new(sk: Rc<RefCell<SkInfo>>) -> Self {
        Self { sk, stepper_handlers: vec![], stepper_actions: VecDeque::new() }
    }

    /// push an action to consumme befor next frame
    pub fn push_action(&mut self, action: StepperAction) {
        self.stepper_actions.push_back(action);
    }

    /// Deque all the actions, create the frame event report, execute all the stepper if quit hasn't be asked
    /// return false if sk_quit must be triggered.
    pub fn step(&mut self, token: &mut MainThreadToken) -> bool {
        while let Some(action) = self.stepper_actions.pop_front() {
            match action {
                StepperAction::Add(mut stepper, type_id, stepper_id) => {
                    if stepper.initialize(stepper_id.clone(), self.sk.clone()) {
                        let stepper_h = StepperHandler { id: stepper_id, type_id, stepper };
                        self.stepper_handlers.push(stepper_h);
                    } else {
                        Log::warn(format!("Stepper {} did not initialize", stepper_id))
                    }
                }
                StepperAction::RemoveAll(stepper_type) => {
                    for stepper_h in
                        self.stepper_handlers.iter_mut().filter(|stepper_h| stepper_h.type_id == stepper_type)
                    {
                        stepper_h.stepper.shutdown();
                    }
                    self.stepper_handlers.retain(|stepper_h| stepper_h.type_id != stepper_type);
                }
                StepperAction::Remove(stepper_id) => {
                    for stepper_h in self.stepper_handlers.iter_mut().filter(|stepper_h| stepper_h.id == stepper_id) {
                        stepper_h.stepper.shutdown()
                    }
                    self.stepper_handlers.retain(|i| i.id != stepper_id);
                }
                StepperAction::Quit(_, _) => return false,
                _ => token.event_report.push(action),
            }
        }

        for stepper_h in &mut self.stepper_handlers {
            stepper_h.stepper.step(token)
        }

        token.event_report.clear();

        true
    }

    /// An enumerable list of all currently active ISteppers registered with StereoKit. This does not include Steppers
    /// that have been added, but are not yet initialized. Stepper initialization happens at the beginning of the frame,
    /// before the app's Step.
    pub fn get_stepper_handlers(&self) -> &[StepperHandler] {
        self.stepper_handlers.as_slice()
    }

    pub fn shutdown(&mut self) {
        self.stepper_actions.clear();
        for stepper_h in self.stepper_handlers.iter_mut() {
            stepper_h.stepper.shutdown()
        }
        self.stepper_handlers.clear();
    }
}

/// Helper to create the whole code of a Stepper in method IStepper::initialize() while avoiding multiple fields.
/// See Demo b_stepper.rs::BStepper
/// Non canonical structure
pub struct StepperClosures<'a> {
    on_step: Box<dyn FnMut(&MainThreadToken) + 'a>,
    shutdown: Box<dyn FnMut() + 'a>,
}

/// create an empty struct to fulfill with fn ClosureStepper::fn(&self)
impl Default for StepperClosures<'_> {
    fn default() -> Self {
        Self { on_step: Box::new(|_token| {}), shutdown: Box::new(|| {}) }
    }
}

impl<'a> StepperClosures<'a> {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn set<U: FnMut(&MainThreadToken) + 'static, S: FnMut() + 'static>(
        &mut self,
        on_step: U,
        shutdown: S,
    ) -> &mut Self {
        self.on_step = Box::new(on_step);
        self.shutdown = Box::new(shutdown);
        self
    }

    pub fn step(&mut self, token: &MainThreadToken) {
        (self.on_step)(token)
    }

    pub fn shutdown(&mut self) {
        (self.shutdown)()
    }
}
