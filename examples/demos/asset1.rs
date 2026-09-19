use std::path::PathBuf;
use stereokit_rust::{
    font::Font,
    include_asset_tree,
    material::Material,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    model::Model,
    prelude::*,
    render::Renderer,
    sound::SoundInst,
    sprite::Sprite,
    system::{Handed, Input, Text, TextBuilder, TextStyle},
    tools::{
        asset_preview::AssetToShow,
        os_api::{PathEntry, get_assets},
    },
    ui::{Ui, UiBtnLayout},
    util::named_colors::{DARK_BLUE, RED, YELLOW},
};

const ASSET_DIR: &[&str] = include_asset_tree!("assets"); // you can't use get_assets_dir() here

/// Represents the Asset1 demo which let you open and display some of your assets.
#[derive(IStepper)]
pub struct Asset1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub transform: Matrix,
    pub asset_pose: Pose,
    pub asset_scale: Vec3,
    pub model_scale: f32,
    model_to_show: Option<Model>,
    sound_to_play: Option<SoundInst>,
    asset_files: Vec<PathEntry>,
    asset_sub_dir: PathBuf,
    exts: Vec<String>,
    pub window_pose: Pose,
    asset_selected: u32,
    radio_off: Sprite,
    radio_on: Sprite,
    text: String,
    text_style: TextStyle,
    hand_material: Material,
}

unsafe impl Send for Asset1 {}

impl Default for Asset1 {
    /// Creates a new instance of Asset1 with default values.
    fn default() -> Self {
        Self {
            id: "Asset1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
            asset_pose: Pose::new(Vec3::new(0.0, 1.3, -0.3), None),
            asset_scale: Vec3::ONE * 0.02,
            model_scale: 1.0,
            model_to_show: None,
            sound_to_play: None,
            asset_files: vec![],
            asset_sub_dir: PathBuf::new(),
            exts: vec![
                // ".sks".into(),
                // ".jpeg".into(),
                // ".png".into(),
                // ".ktx2".into(),
                // ".hdr".into(),
                // ".glb".into(),
                // ".gltf".into(),
                // ".mp3".into(),
            ],
            window_pose: Pose::new(Vec3::new(0.5, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            asset_selected: 0,
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            text: "Asset1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            hand_material: Material::hand(),
        }
    }
}

impl Asset1 {
    /// Initializes the Asset1 instance and loads asset files names.
    fn start(&mut self) -> bool {
        self.asset_files = get_assets(&self.sk_info, self.asset_sub_dir.clone(), &self.exts);

        // Some test about hand meshes
        let left_hand = Input::get_controller_model(Handed::Left);
        let right_hand = Input::get_controller_model(Handed::Right);
        Input::set_controller_model(Handed::Left, Some(&left_hand));
        Input::set_controller_model(Handed::Right, Some(&right_hand));
        let mut new_material_hand = self.hand_material.copy();
        new_material_hand.color_tint(YELLOW);
        Input::hand_material(Handed::Right, Some(new_material_hand));

        Log::diag(format!("{ASSET_DIR:?}"));

        true
    }

    /// Checks for events and handles them accordingly.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Draws the asset model and handles user interactions.
    fn draw(&mut self, _token: &MainThreadToken) {
        let mut new_asset_files = None;

        // The window to select existing model in this crate
        let window_text = if self.exts.is_empty() {
            format!("Assets/{:?}", self.asset_sub_dir)
        } else {
            format!("Assets/{:?} with type {:?}", self.asset_sub_dir, self.exts)
        };
        Ui::window(window_text).pose(&mut self.window_pose).size(Vec2::new(0.5, 0.0)).begin();

        let mut i = 0;
        for file_name in &self.asset_files {
            i += 1;

            if let PathEntry::File(name) = file_name {
                let file_name_str = name.to_str().unwrap_or("OsString error!!");
                Ui::same_line();
                if Ui::radio(file_name_str, self.asset_selected == i)
                    .images(&self.radio_off, &self.radio_on)
                    .image_layout(UiBtnLayout::Left)
                    .press()
                {
                    if let Some(mut sound_inst) = self.sound_to_play {
                        sound_inst.stop();
                    }
                    if let Some(asset_to_show) =
                        AssetToShow::from_file(&self.asset_sub_dir.join(name), self.asset_pose.position)
                    {
                        self.model_to_show = Some(asset_to_show.model);
                        self.sound_to_play = asset_to_show.sound_inst;
                    } else {
                        self.model_to_show = None;
                        self.sound_to_play = None;
                    }
                    self.asset_selected = i;
                }
            }
        }
        Ui::next_line();
        Ui::push_tint(DARK_BLUE);
        if let Some(sub_dir_name) = self.asset_sub_dir.to_str()
            && !sub_dir_name.is_empty()
        {
            //---back button
            if Ui::button("..").press() {
                self.asset_sub_dir.pop();
                new_asset_files = Some(get_assets(&self.sk_info, self.asset_sub_dir.clone(), &self.exts));
            }
        }
        let cur_dir = self.asset_sub_dir.clone();
        // we add the dir at the end
        let mut sub_dir: String = cur_dir.to_string_lossy().to_string();
        if !sub_dir.is_empty() {
            sub_dir += "/";
        }
        let sub_asset_dir = "assets/".to_string() + &sub_dir;
        for dir_name_str in ASSET_DIR {
            if dir_name_str.starts_with(&sub_asset_dir) && dir_name_str.len() > sub_asset_dir.len() + 1 {
                let split_pos =
                    dir_name_str.char_indices().nth_back(dir_name_str.len() - sub_asset_dir.len() - 1).unwrap().0;
                let name = &dir_name_str[split_pos..];
                if !name.contains('/') {
                    Ui::same_line();
                    if Ui::button(name).press() {
                        self.asset_sub_dir.push(name);
                        new_asset_files = Some(get_assets(&self.sk_info, self.asset_sub_dir.clone(), &self.exts));
                    }
                }
            }
        }
        Ui::pop_tint();

        if let Some(new_value) = new_asset_files {
            self.asset_files = new_value;
            self.asset_selected = 0;
        }
        Ui::window_end();

        // If a model has been selected, we draw it.
        // The handle gets the base bounds (model bounds * fixed asset_scale); StereoKit's
        // ui_handle_begin multiplies them internally by `model_scale`, keeping the grab
        // volume matched to the drawn size. Passing `model_scale` enables two-handed
        // translate/rotate AND uniform scaling.
        if let Some(model) = &self.model_to_show {
            if Ui::handle("Model1", &mut self.asset_pose, model.get_bounds() * self.asset_scale)
                .scale(&mut self.model_scale)
                .grab()
                && let Some(mut sound) = self.sound_to_play
            {
                sound.position(self.asset_pose.position);
            }
            // Combine the fixed per-axis base scale with the user-driven uniform scale.
            let model_transform = self.asset_pose.to_matrix(Some(self.asset_scale * self.model_scale));
            Renderer::add_model(model, model_transform, None, None);
        } else {
            self.asset_selected = 0;
        }

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    fn close(&mut self, _triggering: bool) -> bool {
        if _triggering {
            Input::hand_material(Handed::Right, Some(self.hand_material.clone_ref()));
            if let Some(mut sound_inst) = self.sound_to_play {
                sound_inst.stop();
            }
            self.shutdown_completed = true;
            true
        } else {
            self.shutdown_completed
        }
    }
}
