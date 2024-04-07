use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time,
};

use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    model::Model,
    shader::Shader,
    sk::{IStepper, MainThreadToken, SkInfo, StepperId},
    system::{Log, Text, TextStyle},
    tex::{Tex, TexFormat, TexType},
    util::{
        named_colors::{GREEN_YELLOW, WHITE},
        Color32, Time,
    },
};

pub struct Threads2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    model: Model,
    run_for_ever: Arc<AtomicBool>,
    thread_blinker: Option<JoinHandle<()>>,
    pub transform_model: Matrix,
    pub transform: Matrix,
    text: String,
    text_style: TextStyle,
}

unsafe impl Send for Threads2 {}

impl Default for Threads2 {
    fn default() -> Self {
        Self {
            id: "Threads2".into(),
            sk_info: None,
            model: Model::new(),
            run_for_ever: Arc::new(AtomicBool::new(true)),
            thread_blinker: None,
            transform_model: Matrix::t(Vec3::new(0.0, 1.0, -0.6)),
            transform: Matrix::tr(&((Vec3::NEG_Z * 3.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Threads2".into(),
            text_style: Text::make_style(Font::default(), 0.3, GREEN_YELLOW),
        }
    }
}
const MODEL_ID: &str = "Threads2/model";

impl IStepper for Threads2 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        self.model.id(MODEL_ID);

        let run_for_ever1 = self.run_for_ever.clone();
        let run_for_ever2 = self.run_for_ever.clone();
        let thread_add = thread::spawn(move || {
            let mut id: u32 = 0;
            let model = Model::find(MODEL_ID).unwrap();

            while run_for_ever1.load(Ordering::Relaxed) {
                id += 1;
                let r: u8 = ((id * 30) % 255) as u8;
                let g: u8 = ((id * 20) % 255) as u8;
                let b: u8 = ((id * 10) % 255) as u8;
                let random = ((Time::get_totalf() * 20.0) % 5.0) / 6.0;
                let id_str = "Test ".to_string() + &id.to_string();
                let mesh = Mesh::generate_rounded_cube(Vec3::ONE * 0.03, 0.001, None);
                let tex = Tex::gen_color(Color32::new(r, g, b, 255), 16, 16, TexType::ImageNomips, TexFormat::RGBA32);
                let mut material = Material::default_copy();
                material.diffuse_tex(tex);
                let name = id_str.clone();
                let local_transform = Matrix::t(Vec3::new(id as f32 / 200.0, random, -random));
                model.get_nodes().add(name, local_transform, mesh, &material, true);
                thread::sleep(time::Duration::from_millis(500));
            }
            Log::diag("close thread_add");
        });
        self.thread_blinker = Some(thread::spawn(move || {
            let model = Model::find(MODEL_ID).unwrap();
            let blinker = Shader::from_file("shaders/blinker.hlsl.sks").unwrap();
            while run_for_ever2.load(Ordering::Relaxed) {
                let model_nodes = model.get_nodes();
                for node in model_nodes.visuals() {
                    if let Some(mut material) = node.get_material() {
                        material.shader(&blinker).color_tint(WHITE);
                    }
                }
                thread::sleep(time::Duration::from_millis(2000));
            }
            if let Err(error) = thread_add.join() {
                Log::err(format!("Thread1, thread_add panic  : {:?}", error));
            }
            Log::diag("close thread_blinker");
        }));
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }

    fn shutdown(&mut self) {
        self.run_for_ever.store(false, Ordering::SeqCst);
        if let Some(thread) = self.thread_blinker.take() {
            if let Err(error) = thread.join() {
                Log::err(format!("Thread1, thread_add panic  : {:?}", error));
            }
        }
        Log::diag("close Thread2 Demo");
    }
}

impl Threads2 {
    fn draw(&mut self, token: &MainThreadToken) {
        self.model.draw(token, self.transform_model, None, None);
        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
