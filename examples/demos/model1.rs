use std::{cell::RefCell, ffi::OsString, rc::Rc};
use stereokit_rust::{
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    model::{AnimMode, Model},
    sk::{IStepper, SkInfo, StepperAction, StepperId},
    sprite::Sprite,
    system::{Log, Renderer, Text, TextStyle},
    ui::{Ui, UiBtnLayout},
    util::named_colors::RED,
};

pub struct Model1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform: Matrix,
    pub model_pose: Pose,
    pub model_scale: Vec3,
    model: Option<Model>,
    gltf_dir: Vec<OsString>,
    pub window_model_pose: Pose,
    model_selected: u32,
    radio_off: Sprite,
    radio_on: Sprite,
    text: String,
    text_style: TextStyle,
}

impl Default for Model1 {
    fn default() -> Self {
        Self {
            id: "Model1".to_string(),
            sk_info: None,
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            model_pose: Pose::new(Vec3::new(0.0, 1.3, -0.3), None),
            model_scale: Vec3::ONE * 0.02,
            model: None,
            gltf_dir: vec![],
            window_model_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            model_selected: 0,
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            text: "Model1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

#[cfg(target_os = "android")]
pub fn get_gltf_files(sk_info: Rc<RefCell<SkInfo>>) -> Vec<OsString> {
    use std::ffi::CString;

    let mut sk_i = sk_info.borrow_mut();
    let app = sk_i.get_android_app();
    let mut vec = vec![];
    let cstr = CString::new("").unwrap();
    if let Some(asset_dir) = app.asset_manager().open_dir(cstr.as_c_str()) {
        for entry in asset_dir {
            if let Ok(entry_string) = entry.into_string() {
                vec.push(OsString::from(entry_string))
            }
        }
    }
    vec
}

#[cfg(not(target_os = "android"))]
pub fn get_gltf_files(_sk: Rc<RefCell<SkInfo>>) -> Vec<OsString> {
    use std::{fs::read_dir, path::Path};

    let path_text = env!("CARGO_MANIFEST_DIR").to_owned() + "/assets";
    let path_asset = Path::new(path_text.as_str());
    let mut vec = vec![];
    if path_asset.exists() && path_asset.is_dir() {
        if let Ok(read_dir) = read_dir(path_asset) {
            for file in read_dir.flatten().filter(|name| name.path().is_file()) {
                vec.push(file.file_name())
            }
        }
    }
    vec
}

impl IStepper for Model1 {
    fn initialize(&mut self, id: StepperId, sk: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.gltf_dir = get_gltf_files(sk.clone());
        self.sk_info = Some(sk);
        true
    }

    fn step(&mut self, _event_report: &[StepperAction]) {
        self.draw()
    }
}

impl Model1 {
    fn draw(&mut self) {
        // If a model has been selected, we draw it
        if let Some(model) = &self.model {
            Ui::handle("Model1", &mut self.model_pose, model.get_bounds() * self.model_scale, false, None, None);
            let model_transform = self.model_pose.to_matrix(Some(self.model_scale));
            Renderer::add_model(model, model_transform, None, None);
        }

        // The window to select existing model in this crate
        Ui::window_begin("Model files", &mut self.window_model_pose, Some(Vec2::new(0.5, 0.0)), None, None);
        let mut i = 0;
        for file_name in &self.gltf_dir {
            i += 1;
            let file_name_str = file_name.to_str().unwrap_or("OsString error!!");
            if Ui::radio_img(
                file_name_str,
                self.model_selected == i,
                &self.radio_off,
                &self.radio_on,
                UiBtnLayout::Left,
                None,
            ) {
                if let Ok(model) = Model::from_file(file_name, None) {
                    let mut anims = model.get_anims();
                    if anims.get_count() > 0 {
                        anims.play_anim_idx(0, AnimMode::Loop);
                    }
                    self.model = Some(model);
                } else {
                    Log::err(format!("Unable to load model {:?} !!", file_name));
                };
                self.model_selected = i;
            }

            if i % 3 != 0 {
                Ui::same_line();
            }
        }

        Ui::window_end();

        // Platform::file_picker(
        //     PickerMode::Open,
        //     |file| {
        //         if let Ok(new_model) = Model::from_file(file, None) {
        //             self.model = Some(new_model);
        //         };
        //     },
        //     || {},
        //     &["*.gltf", "*.glb"],
        // );

        Text::add_at(&self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}
