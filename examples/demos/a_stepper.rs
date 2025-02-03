use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    prelude::*,
    system::{Renderer, Text, TextStyle},
    util::named_colors::RED,
};
/// The basic Stepper. This stepper is used for Thread1 demo, we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
pub struct AStepper {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform: Matrix,
    round_cube: Option<Mesh>,
    pub text: String,
    text_style: Option<TextStyle>,
}

unsafe impl Send for AStepper {}

/// This code may be called in some threads, so no StereoKit code
impl Default for AStepper {
    fn default() -> Self {
        Self {
            id: "AStepper".to_string(),
            sk_info: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            round_cube: None,
            text: "Stepper A".to_owned(),
            text_style: None,
        }
    }
}

/// All the code here run in the main thread
impl IStepper for AStepper {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        self.round_cube = Some(Mesh::generate_rounded_cube(Vec3::ONE / 5.0, 0.2, Some(16)));
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));

        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }
}

impl AStepper {
    fn draw(&mut self, token: &MainThreadToken) {
        if let Some(round_cube) = &self.round_cube {
            Renderer::add_mesh(token, round_cube, Material::pbr(), self.transform, Some(RED.into()), None);
        }
        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }
}
