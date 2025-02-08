use stereokit_rust::{
    material::{Cull, Material, Transparency},
    maths::{Matrix, Quat, Vec3, Vec4},
    model::{AnimMode, Model},
    prelude::*,
    shader::Shader,
    system::{Handed, Input},
    tex::SHCubemap,
    tools::notif::HudNotification,
    util::named_colors::{DARK_RED, WHITE},
};

#[derive(IStepper)]
pub struct Anim1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub title: String,
    mobile: Model,
    transform: Matrix,
    render_now: bool,
    stage: i32,
}

unsafe impl Send for Anim1 {}

impl Default for Anim1 {
    fn default() -> Self {
        let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr())).unwrap().copy();

        let calcaire = Material::find("clean_tile").unwrap_or_default();
        let mut brick_wall_material = calcaire.copy();
        let prefix_id = brick_wall_material.get_id().to_string();
        brick_wall_material
            .roughness_amount(0.7)
            .color_tint(DARK_RED)
            .tex_transform(Vec4::new(0.0, 0.0, 5.0, 5.0))
            .transparency(Transparency::None)
            .face_cull(Cull::None)
            .id(format!("BRICK_{}", prefix_id));
        Log::diag(format!("Brick material ID is {}", brick_wall_material.get_id()));
        // The nodes stay alive and keep Material alive so, no id .id("brick_wall");
        let mut ico_material = brick_wall_material.copy();
        let prefix_id = brick_wall_material.get_id().to_string();
        ico_material.face_cull(Cull::Back).color_tint(WHITE).id(format!("ICO_{}", prefix_id));
        Log::diag(format!("Ico material ID is {}", ico_material.get_id()));
        let nodes = &mobile.get_nodes();
        nodes
            .get_root_node()
            .material(&ico_material)
            .iterate()
            .unwrap()
            .material(&brick_wall_material)
            .iterate()
            .unwrap()
            .material(&ico_material)
            .iterate()
            .unwrap()
            .material(&ico_material);

        let mut anims = mobile.get_anims();
        anims.play_anim("flyRotate", AnimMode::Loop);

        let nodes = mobile.get_nodes();
        nodes.get_root_node().add_child(
            "toto",
            Matrix::IDENTITY,
            None, //
            None,
            true,
        );

        let transform =
            Matrix::trs(&(Vec3::new(0.0, 4.5, -2.0)), &Quat::from_angles(90.0, 0.0, 0.0), &(Vec3::ONE * 0.25));

        let render_now = true;
        let stage = 0;
        Self {
            id: "Sprite 1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            title: "Stereokit Sprites".to_owned(),
            mobile,
            transform,
            render_now,
            stage,
        }
    }
}

impl Anim1 {
    fn start(&mut self) -> bool {
        let mobile = &self.mobile;
        // We inspect the model
        let mut nodes = mobile.get_nodes();
        nodes.add(
            "tutu",
            Matrix::IDENTITY,
            None, //
            None,
            true,
        );
        Log::info(format!("model <~GRN>node count<~clr> : <~RED>{}<~clr> !!!", &mobile.get_nodes().get_count()));
        for n in nodes.all() {
            Log::info(format!("---- : {:?} id: {:?} ", n.get_name(), n.get_id()));
            if let Some(mesh) = n.get_mesh() {
                let material = n.get_material().expect("ModeNode with a mesh Should have a material!");
                Log::info(format!("------- material: {:?} / mesh: {:?}", material.get_id(), mesh.get_id()));
            } else {
                Log::info("------- no mesh, no material");
            }
        }

        // We ask for a notification to be displayed
        let mut notif = HudNotification::default();
        notif.position = Vec3::new(0.0, 0.3, -0.2);
        notif.text = "Close right hand to change animation".into();

        SkInfo::send_message(&self.sk_info, StepperAction::add("HudNotifAnim1", notif));
        true
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, token: &MainThreadToken) {
        self.mobile.draw(token, self.transform, None, None);

        if self.render_now {
            match self.stage % 3 {
                0 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("flyRotate", AnimMode::Loop);
                }
                1 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("fly", AnimMode::Loop);
                }
                2 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("rotate", AnimMode::Loop);
                }
                _ => {
                    self.stage = 0;
                }
            }
        }
        self.render_now = false;
        if Input::hand(Handed::Right).is_just_gripped() {
            self.stage += 1;
            self.render_now = true;
            let cube = SHCubemap::get_rendered_sky();
            Log::info(format!(
                "sample : {:?} / dominent direction {}",
                cube.sh.get_sample(glam::Vec3::ONE),
                cube.sh.get_dominent_light_direction()
            ))
        }
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // We ask for the start notification to be removed if it hasn't been done yet.
            SkInfo::send_message(&self.sk_info, StepperAction::remove("HudNotifAnim1"));
            self.shutdown_completed = true;
            true
        } else {
            self.shutdown_completed
        }
    }
}
