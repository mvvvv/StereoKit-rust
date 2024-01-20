use stereokit_rust::material::Material;
use stereokit_rust::maths::{Matrix, Quat, Vec3};
use stereokit_rust::model::{AnimMode, Model};
use stereokit_rust::shader::Shader;
use stereokit_rust::sk::{IStepper, StepperAction, StepperId};
use stereokit_rust::system::{Handed, Input, Log};
use stereokit_rust::tex::SHCubemap;
use stereokit_rust::util::named_colors::DARK_RED;

use winit::event_loop::EventLoopProxy;

#[derive(Debug)]
pub struct Anim1 {
    pub title: String,
    id: StepperId,
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    mobile: Model,
    transform: Matrix,
    pinch: bool,
    render_now: bool,
    stage: i32,
}
impl Default for Anim1 {
    fn default() -> Self {
        let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr())).unwrap();
        let calcaire = Material::find("mobiles.gltf/mat/Calcaire blanc").unwrap_or_default();
        let mut gray_tile = Material::copy(calcaire);
        gray_tile.roughness_amount(0.7).color_tint(DARK_RED).tex_scale(5.0);
        //gray_tile.id("gray tiles"); // Previous gray_tile is not deleted
        let nodes = &mobile.get_nodes();
        nodes
            .get_root_node()
            .material(&gray_tile)
            .iterate()
            .unwrap()
            .material(&gray_tile)
            .iterate()
            .unwrap()
            .material(&gray_tile)
            .iterate()
            .unwrap();

        let mut anims = (&mobile).get_anims();
        anims.play_anim("flyRotate", AnimMode::Loop);
        anims.anim_time(1.0);

        Log::info(format!("model <~GRN>node count<~clr> : <~RED>{}<~clr> !!!", &mobile.get_nodes().get_count()));
        for n in mobile.get_nodes().all() {
            Log::info(format!("---- : {:?} id: {:?} ", n.get_name(), n.get_id()));
            let material = n.get_material().unwrap_or_default();
            Log::info(format!("------- material : {:?}", Material::from(material).get_id()));
        }

        let transform =
            Matrix::trs(&(Vec3::new(0.0, 4.5, -2.0)), &Quat::from_angles(90.0, 0.0, 0.0), &(Vec3::ONE * 0.25));

        let pinch = false;
        let render_now = true;
        let stage = 0;
        Self {
            title: "Stereokit Sprites".to_owned(),
            id: "Sprite 1".to_string(),
            event_loop_proxy: None,
            mobile,
            transform,
            pinch,
            render_now,
            stage,
        }
    }
}

impl IStepper for Anim1 {
    fn initialize(&mut self, id: StepperId, event_loop_proxy: EventLoopProxy<StepperAction>) -> bool {
        self.id = id;
        self.event_loop_proxy = Some(event_loop_proxy);
        true
    }

    fn step(&mut self, _event_report: &Vec<StepperAction>) {
        self.mobile.draw(self.transform, None, None);

        if self.render_now {
            match self.stage % 3 {
                0 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("flyRotate", AnimMode::Loop);
                    anims.anim_time(1.0);
                }
                1 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("fly", AnimMode::Loop);
                    anims.anim_time(1.0);
                }
                2 => {
                    let mut anims = self.mobile.get_anims();
                    anims.play_anim("rotate", AnimMode::Loop);
                    anims.anim_time(1.0);
                }
                _ => {
                    self.stage = 0;
                }
            }
        }
        self.render_now = false;
        match Input::hand(Handed::Right).pinch_activation.round() as i8 {
            0 => {
                if self.pinch {
                    self.pinch = false;
                    self.stage += 1;
                    self.render_now = true;
                    let cube = SHCubemap::get_rendered_sky();
                    Log::info(format!(
                        "sample : {:?} / dominent direction {}",
                        cube.sh.get_sample(glam::Vec3::ONE),
                        cube.sh.get_dominent_dir().to_string()
                    ))
                }
            }
            1 => {
                if !self.pinch {
                    self.pinch = true;
                }
            }
            _ => {}
        }
    }
}
