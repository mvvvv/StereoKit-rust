use stereokit_rust::{
    framework::{HandMenuAction, HandMenuRadial, HandRadial, HandRadialLayer, HAND_MENU_RADIAL_FOCUS},
    material::{Cull, Material, Transparency},
    maths::{Matrix, Quat, Vec2, Vec3, Vec4},
    mesh::Mesh,
    model::Model,
    prelude::*,
    system::Renderer,
    tex::{SHCubemap, Tex, TexFormat, TexSample},
    tools::{fly_over::ENABLE_FLY_OVER, log_window::SHOW_LOG_WINDOW, screenshot::SHOW_SCREENSHOT_WINDOW},
    util::{
        named_colors::{BLACK, BLUE, BURLY_WOOD, LIGHT_BLUE, LIGHT_CYAN, RED, SEA_GREEN, STEEL_BLUE, WHITE, YELLOW},
        Color128, Gradient, ShLight, SphericalHarmonics,
    },
};
pub const SHOW_SHADOWS: &str = "ShowShadows";
pub const SHOW_FLOOR: &str = "ShowFloor";
pub const CHANGE_FLOOR: &str = "ChangeFlor";
const ID: &str = "demo_1";

/// The basic Stepper. This stepper is used for Thread1 demo, we must ensure the StereoKit code stay in the main thread
/// Default may be called in an other thread
#[derive(IStepper)]
pub struct HandMenuRadial1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    initialize_completed: bool,
    shutdown_completed: bool,

    clean_tile: Material,
    parquet: Material,
    water: Material,
    water2: Material,
    test_material: Material,
    floor_model: Model,
    shadow_depth: Tex,
    pub floor_transform: Matrix,
    pub show_floor: bool,
    pub show_shadows: bool,
    pub floor: u8,
}

unsafe impl Send for HandMenuRadial1 {}

/// This code may be called in some threads, so no StereoKit code
impl Default for HandMenuRadial1 {
    fn default() -> Self {
        // black paving
        let tile = Material::find("mobiles.gltf/mat/Calcaire blanc").unwrap_or_default();
        let mut clean_tile = Material::pbr().copy();
        Log::diag("calcaire_blanc params:");
        for param in tile.get_all_param_info() {
            match param.get_name() {
                "metal" => {
                    let mut metal_tex = param.get_texture().unwrap();
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
        let parquetroughness = Tex::from_file("textures/parquet2/parquet2roughness.ktx2", true, None).unwrap();
        let parquetmetal = Tex::from_file("textures/parquet2/parquet2metal.ktx2", true, None).unwrap();
        let bump_tex = Tex::from_file("textures/water/bump_large.ktx2", true, None).unwrap();
        let bump_tile_tex = Tex::from_file("textures/water/bump_large_tiles.ktx2", true, None).unwrap();
        let bump_inverse_tex = Tex::from_file("textures/water/bump_large_inverse.ktx2", true, None).unwrap();
        //let mut parquet = Material::pbr().copy();
        let mut parquet =
            Material::from_file("shaders/large_tile_pbr.hlsl.sks", "parquet_pbr".into()).unwrap_or_default();
        parquet
            .diffuse_tex(&parquet_img)
            .color_tint(BURLY_WOOD)
            .occlusion_tex(&parquetao)
            .normal_tex(parquetroughness)
            .metal_tex(parquetmetal)
            .tex_transform(Vec4::new(0.0, 0.0, 12.0, 12.0))
            .roughness_amount(0.7)
            .metallic_amount(0.5)
            .transparency(Transparency::Blend)
            .face_cull(Cull::Back);

        // fresh water
        let mut sea = Material::from_file("shaders/water_pbr.hlsl.sks", "water_pbr".into()).unwrap_or_default();
        sea.diffuse_tex(&bump_inverse_tex)
            .normal_tex(&bump_tex)
            .occlusion_tex(&bump_inverse_tex)
            .tex_transform(Vec4::new(0.0, 0.0, 6.0, 6.0))
            .roughness_amount(0.4)
            .metallic_amount(0.6)
            .face_cull(Cull::Back)
            .color_tint(SEA_GREEN)
            .transparency(Transparency::MSAA)
            .time(5.0);

        // fresh water2
        let mut water2 = Material::from_file("shaders/water_pbr2.hlsl.sks", "water_pbr2".into()).unwrap_or_default();
        water2
            .diffuse_tex(&bump_tile_tex)
            .occlusion_tex(&bump_inverse_tex)
            .normal_tex(&bump_tex)
            .tex_transform(Vec4::new(0.0, 0.0, 5.0, 5.0))
            .roughness_amount(0.4)
            .metallic_amount(0.6)
            .face_cull(Cull::Back)
            .color_tint(STEEL_BLUE)
            .transparency(Transparency::Blend)
            .time(5.0);

        let shadow_depth =
            Tex::render_target(1024, 1024, Some(1), Some(TexFormat::R8), Some(TexFormat::Depth32)).unwrap_or_default();
        let mut test_material = Material::unlit().copy();
        test_material.diffuse_tex(shadow_depth.get_zbuffer().unwrap_or_default());
        //test_material.diffuse_tex(&shadow_depth);
        //Renderer::clear_color(Color128::hsv(0.4, 0.3, 0.5, 1.0));

        let floor_model = Model::from_mesh(
            Mesh::generate_plane(Vec2::new(40.0, 40.0), Vec3::UP, Vec3::FORWARD, None, true),
            &clean_tile,
        );
        let floor_transform = Matrix::tr(&Vec3::new(0.0, 0.0, 0.0), &Quat::IDENTITY);

        Self {
            id: "HandMenuRadial1".to_string(),
            sk_info: None,
            initialize_completed: false,
            shutdown_completed: false,

            clean_tile,
            parquet,
            water: sea,
            water2,
            test_material,
            floor_model,
            floor_transform,
            shadow_depth,
            show_floor: true,
            show_shadows: true,
            floor: 0,
        }
    }
}

/// All the code here run in the main thread
impl HandMenuRadial1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        let id: &StepperId = &self.id;

        // Open or close the log window
        let mut show_log = true;
        let send_event_show_log = SkInfo::get_message_closure(&self.sk_info, id, SHOW_LOG_WINDOW);

        // Open or close the screenshot window
        let mut show_screenshot = false;
        let send_event_show_screenshot = SkInfo::get_message_closure(&self.sk_info, id, SHOW_SCREENSHOT_WINDOW);

        // Enable disable fly over
        let mut fly_over = true;
        let send_event_fly_over = SkInfo::get_message_closure(&self.sk_info, id, ENABLE_FLY_OVER);

        // Change the material of the floor
        let change_floor0 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);
        let change_floor1 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);
        let change_floor2 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);
        let change_floor3 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);
        let change_floor4 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);
        let change_floor5 = SkInfo::get_message_closure(&self.sk_info, id, CHANGE_FLOOR);

        let mut menu_ico = Material::pbr_clip().copy_for_tex("icons/hamburger.png", true, None).unwrap_or_default();
        menu_ico.clip_cutoff(0.1);

        let mut screenshot_ico =
            Material::pbr_clip().copy_for_tex("icons/screenshot.png", true, None).unwrap_or_default();
        screenshot_ico.clip_cutoff(0.1);

        let mut log_ico = Material::pbr_clip().copy_for_tex("icons/log_viewer.png", true, None).unwrap_or_default();
        log_ico.clip_cutoff(0.1);

        let mut fly_over_ico = Material::pbr_clip().copy_for_tex("icons/fly_over.png", true, None).unwrap_or_default();
        fly_over_ico.clip_cutoff(0.1);

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

        let cube3 = SHCubemap::from_cubemap("hdri/sky_dawn.hdr", true, 0).unwrap_or(SHCubemap::get_rendered_sky());

        // let cubemap_files = [
        //     "hdri/giza/right.png",
        //     "hdri/giza/left.png",
        //     "hdri/giza/top.png",
        //     "hdri/giza/bottom.png",
        //     "hdri/giza/front.png",
        //     "hdri/giza/back.png",
        // ];
        // let cube4 = SHCubemap::from_cubemap_files(&cubemap_files, true, 0).unwrap_or(SHCubemap::get_rendered_sky());

        // You can also zip the files:
        // ktx create --cubemap --encode uastc  --format R8G8B8A8_UNORM  --assign-oetf linear --assign-primaries bt709
        //            --generate-mipmap right.png left.png top.png bottom.png front.png back.png cubemap_rgba32.ktx2
        let cube4 =
            SHCubemap::from_cubemap("hdri/giza/cubemap_rgba32.ktx2", true, 0).unwrap_or(SHCubemap::get_rendered_sky());

        //save the default cubemap.
        let cube_default = SHCubemap::get_rendered_sky();

        //---Load hand menu
        let hand_menu_stepper = HandMenuRadial::new(HandRadialLayer::new(
            "root",
            None,
            Some(100.0),
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
                            "Gizah",
                            None,
                            move || {
                                cube4.render_as_sky();
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
                    "Floor",
                    Some(menu_ico),
                    None,
                    vec![
                        HandRadial::item(
                            "Black tile",
                            None,
                            move || {
                                change_floor0("0".into());
                            },
                            if self.floor == 0 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "Parquet",
                            None,
                            move || {
                                change_floor1("1".into());
                            },
                            if self.floor == 1 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "sea",
                            None,
                            move || {
                                change_floor2("2".into());
                            },
                            if self.floor == 2 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "water",
                            None,
                            move || {
                                change_floor3("3".into());
                            },
                            if self.floor == 3 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "None",
                            None,
                            move || {
                                change_floor4("4".into());
                            },
                            if self.floor == 4 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
                        ),
                        HandRadial::item(
                            "test",
                            None,
                            move || {
                                change_floor5("5".into());
                            },
                            if self.floor == 5 { HandMenuAction::Checked(1) } else { HandMenuAction::Unchecked(1) },
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
                        send_event_show_screenshot(show_screenshot.to_string());
                    },
                    HandMenuAction::Unchecked(2),
                ),
                HandRadial::item(
                    "Log",
                    Some(log_ico),
                    move || {
                        show_log = !show_log;
                        send_event_show_log(show_log.to_string());
                    },
                    HandMenuAction::Checked(3),
                ),
                HandRadial::item(
                    "Fly",
                    Some(fly_over_ico),
                    move || {
                        fly_over = !fly_over;
                        send_event_fly_over(fly_over.to_string());
                    },
                    HandMenuAction::Checked(4),
                ),
                HandRadial::item("Close", None, || {}, HandMenuAction::Close),
            ],
        ));
        self.id = HandMenuRadial::build_id(ID);
        SkInfo::send_message(&self.sk_info, StepperAction::add(self.id.clone(), hand_menu_stepper));

        true
    }

    fn start_completed(&mut self) -> bool {
        self.initialize_completed = true;
        SkInfo::send_message(
            &self.sk_info,
            StepperAction::event(self.id.clone(), HAND_MENU_RADIAL_FOCUS, &true.to_string()),
        );
        true
    }

    /// Here we check the event report and update the floor and the shadows
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(CHANGE_FLOOR) {
            self.floor = value.parse().unwrap_or(0);
        } else if key.eq(SHOW_FLOOR) {
            self.show_floor = value.parse().unwrap_or(true);
        } else if key.eq(SHOW_SHADOWS) {
            self.show_shadows = value.parse().unwrap_or(true);
        }
    }

    /// Called from IStepper::step after check_event, here you can draw your UI and the scene
    /// Here we draw or not draw the floor
    fn draw(&mut self, token: &MainThreadToken) {
        // draw a floor if needed
        if self.show_floor {
            if self.show_shadows && self.floor == 5 {
                let light_pos = Renderer::get_skylight().get_dominent_light_direction() * -500.0;
                let camera = Matrix::tr(&light_pos, &Quat::look_at(light_pos, Vec3::ZERO, None));
                //Log::diag(format!("Camera at {:}", &light_pos));

                // let mut list = RenderList::primary();
                // list.draw_now(
                //     &self.shadow_depth,
                //     camera,
                //     Matrix::perspective(90.0, 1.0, 10.01, 1010.0),
                //     Rect::new(0.0, 0.0, 1.0, 1.0),
                //     None,
                //     None,
                // );

                Renderer::render_to(
                    token,
                    &self.shadow_depth,
                    camera,
                    Matrix::perspective(90.0, 1.0, 10.01, 1010.0),
                    None,
                    None,
                    None,
                );
            }
            match self.floor {
                0 => self.floor_model.draw_with_material(token, &self.clean_tile, self.floor_transform, None, None),
                1 => self.floor_model.draw_with_material(token, &self.parquet, self.floor_transform, None, None),
                2 => self.floor_model.draw_with_material(token, &self.water, self.floor_transform, None, None),
                3 => self.floor_model.draw_with_material(token, &self.water2, self.floor_transform, None, None),
                4 => (),
                5 => self.floor_model.draw_with_material(token, &self.test_material, self.floor_transform, None, None),
                _ => (),
            }
        }
    }

    /// Called from IStepper::shutdown(triggering) then IStepper::shutdown_done(waiting for true response),
    /// here you can close your resources
    /// Close the HandMenuStepper1
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            //We indicate we give up before being shutdowned
            SkInfo::send_message(
                &self.sk_info,
                StepperAction::event(self.id.clone(), HAND_MENU_RADIAL_FOCUS, &false.to_string()),
            );
            self.shutdown_completed = false;
            false
        } else {
            //One step further we can disappear in the darkness
            SkInfo::send_message(&self.sk_info, StepperAction::remove(self.id.clone()));
            self.shutdown_completed = true;
            self.shutdown_completed
        }
    }
}
