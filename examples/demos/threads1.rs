use std::{
    any::TypeId,
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};

use stereokit_rust::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    sk::{IStepper, MainThreadToken, SkInfo, StepperAction, StepperId},
    system::{Log, Text, TextStyle},
    util::{named_colors::GREEN_YELLOW, Time},
};

use super::a_stepper::AStepper;

pub struct Threads1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    run_for_ever: Arc<AtomicBool>,
    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

unsafe impl Send for Threads1 {}

impl Default for Threads1 {
    fn default() -> Self {
        Self {
            id: "Threads1".into(),
            sk_info: None,
            run_for_ever: Arc::new(AtomicBool::new(true)),
            transform: Matrix::tr(&((Vec3::NEG_Z * 3.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Threads1".into(),
            text_style: Text::make_style(Font::default(), 0.3, GREEN_YELLOW),
        }
    }
}

impl IStepper for Threads1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        let rc_sk = self.sk_info.as_ref().unwrap();
        let sk = rc_sk.as_ref();
        let event_loop_proxy1 = sk.borrow().get_event_loop_proxy().clone();
        let event_loop_proxy2 = event_loop_proxy1.clone();
        let run_for_ever1 = self.run_for_ever.clone();
        let run_for_ever2 = self.run_for_ever.clone();
        let thread_add = thread::spawn(move || {
            let mut id = 0;
            while run_for_ever1.load(Ordering::Relaxed) {
                id += 1;
                let mut a = AStepper::default();
                let random = ((Time::get_totalf() * 100.0) % 1000.0) / 600.0;
                let id_str = "Test ".to_string() + &id.to_string();
                a.text.clone_from(&id_str);
                a.transform = Matrix::trs(
                    &Vec3::new(random, 1.0 + random, -1.0 - random),
                    &Quat::from_angles(0.0, 180.0, 0.0),
                    &(Vec3::ONE * 0.2),
                );
                if let Err(error) = event_loop_proxy1.send_event(StepperAction::add(&id_str, a)) {
                    Log::err(format!("Thread1, can't send_event add {} : {}", id_str, error));
                    return;
                }
                thread::sleep(time::Duration::from_millis(500));
            }
        });
        thread::spawn(move || {
            while run_for_ever2.load(Ordering::Relaxed) {
                if let Err(error) = event_loop_proxy2.send_event(StepperAction::remove_all(TypeId::of::<AStepper>())) {
                    Log::err(format!("Thread1, Can't send_event remove_all AStepper: {}", error));
                    return;
                }
                thread::sleep(time::Duration::from_millis(2000));
            }
            if let Err(error) = thread_add.join() {
                Log::err(format!("Thread1, thread_add panic  : {:?}", error));
            } else if let Err(error) = event_loop_proxy2.send_event(StepperAction::remove_all(TypeId::of::<AStepper>()))
            {
                Log::err(format!("Thread1, can't send_event final remove_all AStepper: {:?}", error));
            }
        });
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }

    fn shutdown(&mut self) {
        self.run_for_ever.store(false, Ordering::SeqCst);
    }
}

impl Threads1 {
    fn draw(&mut self, token: &MainThreadToken) {
        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
