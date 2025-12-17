use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time,
};
use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    model::Model,
    prelude::*,
    shader::Shader,
    system::{Text, TextStyle},
    util::{Color128, named_colors::GREEN_YELLOW},
};

#[derive(IStepper)]
pub struct Threads2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    model: Model,
    run_for_ever1: Arc<AtomicBool>,
    run_for_ever2: Arc<AtomicBool>,
    thread_blinker: Option<JoinHandle<()>>,
    pub transform_model: Matrix,
    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

unsafe impl Send for Threads2 {}

impl Default for Threads2 {
    fn default() -> Self {
        let run_for_ever1 = Arc::new(AtomicBool::new(true));
        let run_for_ever2 = Arc::new(AtomicBool::new(true));
        Self {
            id: "Threads2".into(),
            sk_info: None,
            shutdown_completed: false,

            model: Model::new(),
            run_for_ever1,
            run_for_ever2,
            thread_blinker: None,
            transform_model: Matrix::t(Vec3::new(0.0, 1.0, -0.6)),
            transform: Matrix::t_r((Vec3::NEG_Z * 3.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Threads2".into(),
            text_style: Text::make_style(Font::default(), 0.3, GREEN_YELLOW),
        }
    }
}
const MODEL_ID: &str = "Threads2/model";

impl Threads2 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        self.model.id(MODEL_ID);

        let run_for_ever1 = self.run_for_ever1.clone();
        let run_for_ever2 = self.run_for_ever2.clone();
        let run_for_ever2bis = self.run_for_ever2.clone();
        let thread_add = Some(thread::spawn(move || {
            let mut id: u32 = 0;
            let _model = Model::find(MODEL_ID).unwrap();

            let _material = Material::default_copy();
            let _color = Color128::BLACK;
            while run_for_ever1.load(Ordering::SeqCst) && id < 500 {
                id += 1;
                // let random = ((Time::get_totalf() * 20.0) % 5.0) / 6.0;
                // let id_str = "Cube ".to_string() + &id.to_string();
                // let mesh = Mesh::generate_cube(Vec3::ONE * 0.03, None);

                // let tex = Tex::gen_color(color, 16, 16, TexType::ImageNomips, TexFormat::RGBA32);
                // let mut material = Material::default_copy();
                // material.diffuse_tex(tex);
                // material.color_tint(color);
                // let name = id_str.clone();
                // let local_transform = Matrix::t(Vec3::new(id as f32 / 200.0, random, -random));
                // model.get_nodes().add(name, local_transform, &mesh, &material, true);
                Log::diag(format!("loop1 : {id} "));
                thread::sleep(time::Duration::from_millis(1));
            }
            run_for_ever2bis.store(false, Ordering::Release);
            Log::diag("closing thread_add.");
        }));
        self.thread_blinker = Some(thread::spawn(move || {
            let _model = Model::find(MODEL_ID).unwrap();
            let blinker = Shader::from_file("shaders/blinker.hlsl.sks").unwrap_or_default();
            let mut id = 0;
            let mut material = Material::default_copy();
            let color = Color128::WHITE;
            material.color_tint(color);
            material.shader(blinker);
            while run_for_ever2.load(Ordering::SeqCst) {
                id += 1;
                // let random = ((Time::get_totalf() * 20.0) % 5.0) / 6.0;
                // let id_str = "Sphere ".to_string() + &id.to_string();
                // let mesh = Mesh::generate_sphere(0.04, None);
                // let name = id_str.clone();
                // let local_transform = Matrix::t(Vec3::new(id as f32 / 200.0, random, -random));

                // let mut model_nodes = model.get_nodes();
                // model_nodes.add(name, local_transform, &mesh, &material, true);
                Log::diag(format!("loop2 : {id} "));
                thread::sleep(time::Duration::from_millis(1));
            }

            Log::diag("closing thread_blinker.");

            match thread_add.map(JoinHandle::join) {
                Some(Err(error)) => {
                    Log::err(format!("Thread2, thread panic  : {error:?}"));
                }
                Some(Ok(_)) => (),
                None => {
                    Log::err("Thread2, thread was not set");
                }
            }
        }));
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI and scene
    fn draw(&mut self, token: &MainThreadToken) {
        self.model.draw(token, self.transform_model, None, None);
        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            Log::diag("Closing Thread2 Demo ...");
            self.run_for_ever1.store(false, Ordering::SeqCst);
            self.run_for_ever2.store(false, Ordering::SeqCst);
            self.shutdown_completed = false;
        } else if let Some(join_handle) = self.thread_blinker.take() {
            if join_handle.is_finished() {
                if let Err(error) = join_handle.join() {
                    Log::err(format!("Thread2, join_handle panic  : {error:?}"));
                }
                self.shutdown_completed = true;
            } else {
                self.thread_blinker = Some(join_handle);
            }
        }
        self.shutdown_completed
    }
}
