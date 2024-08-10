use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    event_loop::{IStepper, StepperAction, StepperId},
    framework::{HandMenuAction, HandMenuRadial, HandRadial, HandRadialLayer},
    material::{Cull, Material},
    maths::{Matrix, Quat, Vec2, Vec3, Vec4},
    mesh::Mesh,
    model::Model,
    shader::Shader,
    sk::{MainThreadToken, SkInfo},
    system::Log,
    tex::{SHCubemap, Tex, TexSample},
    tools::{log_window::SHOW_LOG_WINDOW, screenshoot::SHOW_SCREENSHOT_WINDOW},
    util::{
        named_colors::{BLACK, BLUE, BURLY_WOOD, LIGHT_BLUE, LIGHT_CYAN, RED, SEA_GREEN, STEEL_BLUE, WHITE, YELLOW},
        Color128, Gradient, ShLight, SphericalHarmonics,
    },
};

pub const SHOW_FLOOR: &str = "ShowFloor";
pub const CHANGE_FLOOR: &str = "ChangeFlor";

/// The basic Stepper. This stepper is used for Thread1 demo, we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
pub struct HandMenuRadial1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    clean_tile: Material,
    parquet: Material,
    water: Material,
    water2: Material,
    floor_model: Model,
    pub floor_transform: Matrix,
    pub show_floor: bool,
    pub floor: u8,
}

unsafe impl Send for HandMenuRadial1 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for HandMenuRadial1 {
    fn default() -> Self {
        let mobile = Model::from_file("mobiles.gltf", Some(Shader::pbr())).unwrap();
        let tile = Material::find("mobiles.gltf/mat/Calcaire blanc").unwrap_or_default();
        Log::diag(format!("{:?}", mobile.get_id()));
        for iter in mobile.get_nodes().visuals() {
            Log::diag(format!("{:?}", iter.get_mesh().unwrap().get_id()));
        }

        // black paving
        let mut clean_tile = Material::pbr().copy();
        Log::diag("calcaire_blanc params:");
        for param in tile.get_all_param_info() {
            match param.get_name() {
                "metal" => {
                    let metal_tex = param.get_texture().unwrap();
                    metal_tex.sample_mode(TexSample::Anisotropic).anisotropy(6);
                    clean_tile.metal_tex(metal_tex);
                    &mut clean_tile
                }
                "diffuse" => clean_tile.diffuse_tex(param.get_texture().unwrap()),
                "normal" => clean_tile.normal_tex(param.get_texture().unwrap()),
                "occlusion" => clean_tile.occlusion_tex(param.get_texture().unwrap()),
                _ => &mut clean_tile,
            };
            Log::diag(format!(" --- {} :{}", param.get_name(), param.to_string().unwrap_or("no value".to_string())));
        }
        clean_tile
            .id("clean_tile")
            .tex_transform(Vec4::new(0.0, 0.0, 3.0, 3.0))
            .roughness_amount(0.7)
            .color_tint(BLACK)
            .queue_offset(11);

        // old parquet
        let parquet_img = Tex::from_file("textures/parquet2/parquet2.ktx2", true, None).unwrap();
        let parquetao = Tex::from_file("textures/parquet2/parquet2ao.ktx2", true, None).unwrap();
        //let parquetroughness = Tex::from_file("textures/parquet2/parquet2roughness.ktx2", true, None).unwrap();
        let parquetmetal = Tex::from_file("textures/parquet2/parquet2metal.ktx2", true, None).unwrap();
        let bump_tex = Tex::from_file("textures/water/bump_large.ktx2", true, None).unwrap();
        let bump_tile_tex = Tex::from_file("textures/water/bump_large_tiles.ktx2", true, None).unwrap();
        let bump_inverse_tex = Tex::from_file("textures/water/bump_large_inverse.ktx2", true, None).unwrap();
        let mut parquet = Material::pbr().copy();
        parquet
            .diffuse_tex(&parquet_img)
            .color_tint(BURLY_WOOD)
            .occlusion_tex(&parquetao)
            //.normal_tex(parquetroughness)
            .metal_tex(parquetmetal)
            .tex_transform(Vec4::new(0.0, 0.0, 12.0, 12.0))
            .roughness_amount(0.7)
            .metallic_amount(0.5)
            .face_cull(Cull::Back);

        // fresh water
        let mut sea = Material::from_file("shaders/water_pbr.hlsl.sks", "water_pbr".into()).unwrap_or_default();
        sea.diffuse_tex(&bump_inverse_tex)
            .normal_tex(&bump_tex)
            .tex_transform(Vec4::new(0.0, 0.0, 12.0, 12.0))
            .roughness_amount(0.4)
            .metallic_amount(0.6)
            .face_cull(Cull::Back)
            .color_tint(SEA_GREEN)
            .time(5.0);

        // fresh water2
        let mut water2 = Material::from_file("shaders/water_pbr2.hlsl.sks", "water_pbr2".into()).unwrap_or_default();
        water2
            .diffuse_tex(&bump_tile_tex)
            .normal_tex(&bump_tex)
            .tex_transform(Vec4::new(0.0, 0.0, 12.0, 12.0))
            .roughness_amount(0.4)
            .metallic_amount(0.6)
            .face_cull(Cull::Back)
            .color_tint(STEEL_BLUE)
            .time(5.0);

        let floor_model = Model::from_mesh(
            Mesh::generate_plane(Vec2::new(40.0, 40.0), Vec3::UP, Vec3::FORWARD, None, true),
            &clean_tile,
        );
        let floor_transform = Matrix::tr(&Vec3::new(0.0, 0.0, 0.0), &Quat::IDENTITY);

        Self {
            id: "HandMenuRadial1".to_string(),
            sk_info: None,

            clean_tile,
            parquet,
            water: sea,
            water2,
            floor_model,
            floor_transform,
            show_floor: true,
            floor: 0,
        }
    }
}

/// All the code here run in the main thread
impl IStepper for HandMenuRadial1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;

        // Open or close the log window
        let id = self.id.clone();
        let mut show_log = true;
        let event_loop_proxy = sk_info.borrow().get_event_loop_proxy().unwrap();
        let send_event_show_log = move |show: bool| {
            let _ = &event_loop_proxy.send_event(StepperAction::event(id, SHOW_LOG_WINDOW, &show.to_string()));
        };

        // Open or close the screenshot window
        let id = self.id.clone();
        let mut show_screenshot = false;
        let event_loop_proxy = sk_info.borrow().get_event_loop_proxy().unwrap();
        let send_event_show_screenshot = move |show: bool| {
            let _ = &event_loop_proxy.send_event(StepperAction::event(id, SHOW_SCREENSHOT_WINDOW, &show.to_string()));
        };

        // Change the material of the floor
        let id = self.id.clone();
        let event_loop_proxy = sk_info.borrow().get_event_loop_proxy().unwrap();
        let change_floor = move |floor_index: String| {
            let _ = &event_loop_proxy.send_event(StepperAction::event(id, CHANGE_FLOOR, &floor_index));
        };
        let change_floor0 = change_floor.clone();
        let change_floor1 = change_floor.clone();
        let change_floor2 = change_floor.clone();
        let change_floor3 = change_floor.clone();
        let change_floor4 = change_floor.clone();

        let mut menu_ico = Material::pbr_clip().copy();
        let tex = Tex::from_file("icons/hamburger.png", true, None).unwrap_or_default();
        menu_ico.diffuse_tex(tex).clip_cutoff(0.1);

        let mut screenshot_ico = Material::pbr_clip().copy();
        let tex = Tex::from_file("icons/screenshot.png", true, None).unwrap_or_default();
        screenshot_ico.diffuse_tex(tex).clip_cutoff(0.1);

        let mut log_ico = Material::pbr_clip().copy();
        let tex = Tex::from_file("icons/log_viewer.png", true, None).unwrap_or_default();
        log_ico.diffuse_tex(tex).clip_cutoff(0.1);

        //---- Sky domes and floor
        let mut gradient_sky = Gradient::new(None);
        gradient_sky
            .add(Color128::BLACK, 0.0)
            .add(BLUE, 0.4)
            .add(LIGHT_BLUE, 0.8)
            .add(LIGHT_CYAN, 0.9)
            .add(WHITE, 1.0);
        let cube0 = SHCubemap::gen_cubemap_gradient(gradient_sky, Vec3::Y, 1024);

        let mut gradient = Gradient::new(None);
        gradient
            .add(RED, 0.01)
            .add(YELLOW, 0.1)
            .add(LIGHT_CYAN, 0.3)
            .add(LIGHT_BLUE, 0.4)
            .add(BLUE, 0.5)
            .add(BLACK, 0.7);
        let cube1 = SHCubemap::gen_cubemap_gradient(&gradient, Vec3::NEG_Z, 1);

        let lights: [ShLight; 1] = [ShLight::new(Vec3::ONE, WHITE); 1];
        let sh = SphericalHarmonics::from_lights(&lights);
        let cube2 = SHCubemap::gen_cubemap_sh(sh, 15, 5.0, 0.02);

        let cube3 = SHCubemap::from_cubemap_equirectangular("hdri/sky_dawn.hdr", true, 0)
            .unwrap_or(SHCubemap::get_rendered_sky());

        //save the default cubemap.
        let cube_default = SHCubemap::get_rendered_sky();

        //---Load hand menu
        let hand_menu_stepper = HandMenuRadial::new(HandRadialLayer::new(
            "root",
            None,
            Some(0.0),
            vec![
                HandRadial::layer(
                    "\nSkydome",
                    Some(menu_ico.copy()),
                    None,
                    vec![
                        HandRadial::item(
                            "Day",
                            None,
                            move || {
                                cube0.render_as_sky();
                            },
                            HandMenuAction::Unchecked(1),
                        ),
                        HandRadial::item(
                            "Sunset",
                            None,
                            move || {
                                cube1.render_as_sky();
                            },
                            HandMenuAction::Unchecked(1),
                        ),
                        HandRadial::item(
                            "Black\nlight",
                            None,
                            move || {
                                cube2.render_as_sky();
                            },
                            HandMenuAction::Unchecked(1),
                        ),
                        HandRadial::item(
                            "HDRI\ndawn",
                            None,
                            move || {
                                cube3.render_as_sky();
                            },
                            HandMenuAction::Unchecked(1),
                        ),
                        HandRadial::item(
                            "Default",
                            None,
                            move || {
                                cube_default.render_as_sky();
                            },
                            HandMenuAction::Checked(1),
                        ),
                        HandRadial::item("Back", None, || {}, HandMenuAction::Back),
                        HandRadial::item("Close", None, || {}, HandMenuAction::Close),
                    ],
                ),
                HandRadial::layer(
                    "\nFloor",
                    Some(menu_ico),
                    None,
                    vec![
                        HandRadial::item(
                            "Black tile",
                            None,
                            move || {
                                change_floor0.clone()("0".into());
                            },
                            if self.floor == 0 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "Parquet",
                            None,
                            move || {
                                change_floor1.clone()("1".into());
                            },
                            if self.floor == 1 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "sea",
                            None,
                            move || {
                                change_floor2.clone()("2".into());
                            },
                            if self.floor == 2 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "water",
                            None,
                            move || {
                                change_floor3.clone()("3".into());
                            },
                            if self.floor == 3 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "None",
                            None,
                            move || {
                                change_floor4.clone()("4".into());
                            },
                            if self.floor == 4 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item("Back", None, || {}, HandMenuAction::Back),
                        HandRadial::item("Close", None, || {}, HandMenuAction::Close),
                    ],
                ),
                HandRadial::item(
                    "Screenshot",
                    Some(screenshot_ico),
                    move || {
                        show_screenshot = !show_screenshot;
                        send_event_show_screenshot.clone()(show_screenshot);
                    },
                    HandMenuAction::Unchecked(2),
                ),
                HandRadial::item(
                    "Log",
                    Some(log_ico),
                    move || {
                        show_log = !show_log;
                        send_event_show_log.clone()(show_log);
                    },
                    HandMenuAction::Checked(3),
                ),
                HandRadial::item("Close", None, || {}, HandMenuAction::Close),
            ],
        ));

        let event_loop_proxy = sk_info.borrow().get_event_loop_proxy().unwrap();
        let _err = event_loop_proxy.send_event(StepperAction::add("HandMenuStepper1", hand_menu_stepper));

        self.sk_info = Some(sk_info);

        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        for e in token.get_event_report().iter() {
            if let StepperAction::Event(_, key, value) = e {
                if key.eq(CHANGE_FLOOR) {
                    self.floor = value.parse().unwrap_or(0);
                } else if key.eq(SHOW_FLOOR) {
                    self.show_floor = value.parse().unwrap_or(true);
                }
            }
        }
        self.draw(token)
    }

    fn shutdown(&mut self) {
        let event_loop_proxy = self.sk_info.take().unwrap().borrow().get_event_loop_proxy().unwrap();
        let _err = event_loop_proxy.send_event(StepperAction::remove("HandMenuStepper1"));
    }
}

impl HandMenuRadial1 {
    fn draw(&mut self, token: &MainThreadToken) {
        // draw a floor if needed
        if self.show_floor {
            match self.floor {
                0 => self.floor_model.draw_with_material(token, &self.clean_tile, self.floor_transform, None, None),
                1 => self.floor_model.draw_with_material(token, &self.parquet, self.floor_transform, None, None),
                2 => self.floor_model.draw_with_material(token, &self.water, self.floor_transform, None, None),
                3 => self.floor_model.draw_with_material(token, &self.water2, self.floor_transform, None, None),
                _ => (),
            }
        }
    }
}
