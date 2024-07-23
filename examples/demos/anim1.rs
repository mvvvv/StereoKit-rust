use std::cell::RefCell;
use std::rc::Rc;
use stereokit_rust::{
    event_loop::{IStepper, StepperAction, StepperId},
    material::{Cull, Material, Transparency},
    maths::{Matrix, Quat, Vec3, Vec4},
    model::{AnimMode, Model},
    shader::Shader,
    sk::{MainThreadToken, SkInfo},
    system::{Handed, Input, Log},
    tex::SHCubemap,
    tools::notif::HudNotification,
    util::named_colors::{DARK_RED, WHITE},
};
#[derive(Debug)]
pub struct Anim1 {
    pub title: String,
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    mobile: Model,
    transform: Matrix,
    render_now: bool,
    stage: i32,
}

unsafe impl Send for Anim1 {}

impl Default for Anim1 {
    fn default() -> Self {
        let calcaire = Material::find("clean_tile").unwrap_or_default();
        let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr())).unwrap();
        let mut brick_wall = calcaire.copy();
        brick_wall
            .roughness_amount(0.7)
            .color_tint(DARK_RED)
            .tex_transform(Vec4::new(0.0, 0.0, 5.0, 5.0))
            .transparency(Transparency::None)
            .face_cull(Cull::None);
        // The nodes stay alive and keep Material alive so, no id .id("brick_wall");
        let mut ico_material = brick_wall.copy();
        ico_material.face_cull(Cull::Back).color_tint(WHITE);
        let nodes = &mobile.get_nodes();
        nodes
            .get_root_node()
            .material(&ico_material)
            .iterate()
            .unwrap()
            .material(&brick_wall)
            .iterate()
            .unwrap()
            .material(&ico_material)
            .iterate()
            .unwrap()
            .material(&ico_material);

        let mut anims = mobile.get_anims();
        anims.play_anim("flyRotate", AnimMode::Loop);

        Log::info(format!("model <~GRN>node count<~clr> : <~RED>{}<~clr> !!!", &mobile.get_nodes().get_count()));
        for n in mobile.get_nodes().all() {
            Log::info(format!("---- : {:?} id: {:?} ", n.get_name(), n.get_id()));
            let material = n.get_material().unwrap_or_default();
            Log::info(format!("------- material : {:?}", material.get_id()));
        }

        let transform =
            Matrix::trs(&(Vec3::new(0.0, 4.5, -2.0)), &Quat::from_angles(90.0, 0.0, 0.0), &(Vec3::ONE * 0.25));

        let render_now = true;
        let stage = 0;
        Self {
            title: "Stereokit Sprites".to_owned(),
            id: "Sprite 1".to_string(),
            sk_info: None,
            mobile,
            transform,
            render_now,
            stage,
        }
    }
}

impl IStepper for Anim1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        let mut notif = HudNotification::default();
        notif.position = Vec3::new(0.0, 0.3, -0.2);
        notif.text = "Close right hand to change animation".into();

        let rc_sk = self.sk_info.as_ref().unwrap();
        let sk = rc_sk.as_ref();
        let event_loop_proxy = sk.borrow().get_event_loop_proxy().unwrap();
        let _ = event_loop_proxy.send_event(StepperAction::add("HudNotifAnim1", notif));
        true
    }

    fn step(&mut self, token: &MainThreadToken) {
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
}
