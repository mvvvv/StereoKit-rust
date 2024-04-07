use stereokit_rust::material::{Cull, Material};
use stereokit_rust::mesh::Mesh;
use stereokit_rust::model::Model;
use stereokit_rust::sk::{IStepper, MainThreadToken, SkInfo, StepperId};
use stereokit_rust::system::{Handed, Input, Log};
use stereokit_rust::tex::{Tex, TexFormat, TexType};
use stereokit_rust::util::{
    named_colors::{BLACK, BLUE, LIGHT_BLUE, RED, YELLOW},
    Color128, Color32, Gradient,
};

use glam::{Mat4, Quat, Vec3};

use std::cell::RefCell;
use std::f32::consts::PI;
use std::ops::Mul;
use std::rc::Rc;

#[derive(Debug)]
pub struct Tex1 {
    pub title: String,
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    panels: Model,
    tex_vide: Tex,
    tex_vide2: Tex,
    tex_vide3: Tex,
    tex_vide4: Tex,
    tex_default: Tex,
    tex_color_32a: Tex,
    tex_color_32b: Tex,
    tex_particule: Tex,
    width: usize,
    height: usize,
    raw_dots128: Vec<Color128>,
    raw_dots_byte: Vec<u8>,
    raw_dots_u16: Vec<u16>,
    raw_dots_u32: Vec<f32>,
    base_color: Color32,
    base_color128: Color128,
    render_now: bool,
    stage: i8,
}

unsafe impl Send for Tex1 {}

impl Tex1 {
    /// Change the default title.
    pub fn new(title: String) -> Self {
        Self { title, ..Default::default() }
    }
}

impl Default for Tex1 {
    fn default() -> Self {
        //sk.world_set_occlusion_enabled(true);
        let mut tex_default = Tex::from_file("textures/open_gltf.jpeg", true, None).unwrap();
        // !!! don't set an id here if you want to come back as the tex_default is saved as error_fallback
        Tex::set_error_fallback(&tex_default);
        tex_default.id("here I can set an id !!!");

        //---tex zarbi need a shader to exploit them
        let files = [
            //r"textures/exit.jpeg",
            //r"textures/open_gltf.jpeg",
            r"textures/screenshot.jpeg",
        ];
        let mut tex_zarbi = Tex::from_files(&files, true, None).unwrap();
        tex_zarbi.id("zarbi zarbi");

        //---- Textures
        //      licensed under the Creative Commons CC0 1.0 Universal License.

        let base_color128: Color128 = LIGHT_BLUE.into();
        let line_color128: Color128 = RED.into();
        let sub_line_color128: Color128 = BLACK.into();

        let base_color = Color32::new(255, 255, 255, 124);
        let line_color = Color32::new(130, 124, 255, 124);
        let sub_line_color = Color32::new(130, 124, 130, 124);

        let width = 128;
        let height = 128;
        Log::info(format!("size : {}/{}", width, height));
        let mut raw_dots = Vec::new(); //vec![base_color; width * height];
        let mut raw_dots128 = Vec::new();
        let mut raw_dots_byte: Vec<u8> = Vec::new();
        let mut raw_dots_u16: Vec<u16> = Vec::new();
        let mut raw_dots_u32 = Vec::new();
        for y in 0..height {
            for x in 0..width {
                if x % 128 == 0
                    || (x + 1) % 128 == 0
                    || (x - 1) % 128 == 0
                    || y % 128 == 0
                    || (y + 1) % 128 == 0
                    || (y - 1) % 128 == 0
                {
                    raw_dots.push(line_color);
                    raw_dots128.push(line_color128);
                    raw_dots_byte.push(220);
                    raw_dots_u16.push(1220);
                    raw_dots_u32.push(0.0);
                } else if x % 64 == 0 || y % 64 == 0 {
                    raw_dots.push(sub_line_color);
                    raw_dots128.push(sub_line_color128);
                    raw_dots_byte.push(0);
                    raw_dots_u16.push(30000);
                    raw_dots_u32.push(0.5);
                } else {
                    raw_dots.push(base_color);
                    raw_dots128.push(base_color128);
                    raw_dots_byte.push(50);
                    raw_dots_u16.push(64000);
                    raw_dots_u32.push(0.9);
                }
            }
        }

        let color_dots = raw_dots.as_slice();
        let color_dots128 = raw_dots128.as_slice();

        let mut tex_color_32a = Tex::new(TexType::Image, TexFormat::RGBA32, "tex_color");
        tex_color_32a
            .id("tex_color32a")
            .set_colors(width, height, color_dots.as_ptr() as *mut std::os::raw::c_void);

        let mut tex_color_32b = Tex::gen_color(BLUE, 10, 10, TexType::Dynamic, TexFormat::RGBA32);
        tex_color_32b
            .id("tex_color32b")
            .set_colors(width, height, color_dots.as_ptr() as *mut std::os::raw::c_void);

        let mut tex_color_32c = Tex::from_color32(color_dots, width, height, true).unwrap();
        tex_color_32c.id("tex_color32c");
        let mut tex_color_32d = Tex::from_color128(color_dots128, width, height, true).unwrap();
        tex_color_32d.id("tex_color32d");
        let tex_vide = Tex::new(TexType::ImageNomips, TexFormat::RGBA128, "tex_vide");
        let tex_vide2 = Tex::new(TexType::ImageNomips, TexFormat::R8, "tex_vide2");
        let tex_vide3 = Tex::new(TexType::ImageNomips, TexFormat::R16f, "tex_vide3");
        let tex_vide4 = Tex::new(TexType::ImageNomips, TexFormat::R32, "tex_vide4");

        let mut gradient = Gradient::new(None);
        gradient.add(RED, 0.01);
        gradient.add(YELLOW, 0.1);
        gradient.add(LIGHT_BLUE, 0.3);
        gradient.add(BLUE, 0.5);
        gradient.add(BLACK, 0.7);
        let tex_particule = Tex::gen_particle(128, 128, 0.2, Some(gradient));

        //----- Materials
        let mut basic_material = Material::copy(Material::unlit());
        basic_material.face_cull(Cull::None);
        let mut color = Material::copy(&basic_material);
        color
            .id("color mat")
            .diffuse_tex(&tex_color_32a)
            //.color_tint(RED)
            .tex_scale(2.0);

        let mut color2 = Material::copy(&basic_material);
        color2.id("color2 mat").diffuse_tex(&tex_color_32b).tex_scale(1.0);

        let mut color3 = Material::copy(&basic_material);
        color3.id("color3 mat").diffuse_tex(&tex_color_32c).tex_scale(8.0);

        let mut color4 = Material::copy(&basic_material);
        color4.id("color4 mat").diffuse_tex(&tex_color_32d).tex_scale(16.0);

        let mut vide = Material::copy(&basic_material);
        vide.id("vide mat").diffuse_tex(&tex_vide);

        let mut particule = Material::copy(&basic_material);
        particule.id("particule mat").diffuse_tex(&tex_particule);

        let mut vide2 = Material::copy(&basic_material);
        vide2.id("vide mat2").diffuse_tex(&tex_vide2).tex_scale(2.0);

        let mut vide3 = Material::copy(&basic_material);
        vide3.id("vide mat3").diffuse_tex(&tex_vide3).tex_scale(2.0);

        let mut vide4 = Material::copy(&basic_material);
        vide4.id("vide mat4").diffuse_tex(&tex_vide4).tex_scale(2.0);

        let mut zarbi = Material::copy(&basic_material);
        zarbi.id("zarbi").diffuse_tex(&tex_zarbi).tex_scale(2.0);

        let panels = Model::new();
        let mut nodes = panels.get_nodes();
        nodes
            .add(
                "p1",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(-2.5, 2.5, 0.0))),
                Mesh::screen_quad(),
                &color2,
                false,
            )
            .add(
                "p2",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(0.0, 2.5, 0.0))),
                Mesh::screen_quad(),
                &color4,
                false,
            )
            .add(
                "p3",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(2.5, 2.5, 0.0))),
                Mesh::screen_quad(),
                &particule,
                false,
            )
            .add(
                "p4",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(-2.5, 0.0, 0.0))),
                Mesh::screen_quad(),
                &color,
                false,
            )
            .add(
                "p5",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0))),
                Mesh::screen_quad(),
                &color3,
                false,
            )
            .add(
                "p6",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(2.5, 0.0, 0.0))),
                Mesh::screen_quad(),
                &vide,
                false,
            )
            .add(
                "p7",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(-2.5, -2.5, 0.0))),
                Mesh::screen_quad(),
                &vide2,
                false,
            )
            .add(
                "p8",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(0.0, -2.5, 0.0))),
                Mesh::screen_quad(),
                &vide3,
                false,
            )
            .add(
                "p9",
                Mat4::IDENTITY.mul(Mat4::from_translation(glam::Vec3::new(2.5, -2.5, 0.0))),
                Mesh::screen_quad(),
                &vide4,
                false,
            )
            .add(
                "pSol",
                Mat4::IDENTITY.mul(Mat4::from_rotation_translation(
                    glam::Quat::from_rotation_y(PI / 2.0),
                    glam::Vec3::new(0.0, -2.5, 1.0),
                )),
                Mesh::screen_quad(),
                &zarbi,
                false,
            );

        Log::info(format!("!!Nodes number: {:?}", nodes.get_count()));
        let n = nodes.all().map(|node| format!("---{:?} : {:?}", node.get_name(), node.get_id()));
        for s in n {
            Log::info(format!("{:?}", s));
        }

        panels.recalculate_bounds_exact();
        let bounds = panels.get_bounds();

        Log::info(format!(" center : {:#?}", bounds.center));

        let render_now = true;
        let stage = 0;

        let this = Self {
            title: "Tex1".to_owned(),
            id: "Tex1".to_owned(),
            sk_info: None,
            panels,
            tex_vide,
            tex_vide2,
            tex_vide3,
            tex_vide4,
            tex_default,
            tex_color_32a,
            tex_color_32b,
            tex_particule,
            width,
            height,
            raw_dots128,
            raw_dots_byte,
            raw_dots_u16,
            raw_dots_u32,
            base_color,
            base_color128,
            render_now,
            stage,
        };

        Log::info("go");

        this
    }
}

impl IStepper for Tex1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);

        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.panels.draw(
            token,
            Mat4::IDENTITY.mul(Mat4::from_scale_rotation_translation(
                Vec3::ONE * 0.25,
                Quat::from_rotation_y(0.0),
                Vec3::new(-0.5, 2.0, -2.0),
            )),
            None::<Color128>,
            None,
        );

        if self.render_now {
            match self.stage % 3 {
                // get a texture flood error
                0 => {}
                1 => {
                    self.tex_vide.set_colors128(self.width, self.height, self.raw_dots128.as_slice());
                    self.tex_vide2.set_colors_r8(self.width, self.height, self.raw_dots_byte.as_slice());
                    // see R16 --> R16f [sk_gpu] API/ERROR - 0x1 - GL_INVALID_OPERATION in glTexImage2D(format = GL_RED, type = GL_UNSIGNED_SHORT, internalformat = GL_R16F)
                    self.tex_vide3.set_colors_r16(self.width, self.height, self.raw_dots_u16.as_slice());
                    self.tex_vide4.set_colors_r32(self.width, self.height, self.raw_dots_u32.as_slice());
                    Log::info(format!(
                        "R8 height width : {}x{}",
                        self.tex_vide2.get_height().unwrap(),
                        self.tex_vide2.get_width().unwrap()
                    ));

                    // test with anisotropy using stage value
                    let mip = self.stage / 3;
                    if let Some((w, h, size)) = self.tex_default.get_data_infos(mip) {
                        let vec = vec![self.base_color; size];
                        let array = vec.as_slice();
                        self.tex_default.get_colors(array, mip);
                        self.tex_color_32a.set_colors32(w, h, array);
                        Log::info(format!(
                            "mips {:?} / anisotropy {} / size {}x{}",
                            self.tex_default.get_mips(),
                            self.tex_default.get_anisotropy(),
                            w,
                            h,
                        ));
                    }
                }
                2 => {
                    let tex_i = &self.tex_particule;
                    Log::info(format!("Tex in format {:?}", tex_i.get_format().unwrap()));
                    let mip = 1;
                    if let Some((w, h, size)) = tex_i.get_data_infos(mip) {
                        let color32_buff = vec![self.base_color; size];
                        Log::info(format!("--Tex in mip {} -> {}x{} = {}", mip, w, h, color32_buff.len()));
                        if tex_i.get_colors(color32_buff.as_slice(), mip).is_some() {
                            self.tex_color_32b.set_colors32(w, h, &color32_buff[..]);
                            Log::info(format!("--Tex out mips number {}", self.tex_color_32b.get_mips().unwrap()));
                        }
                    }

                    let tex_i = &self.tex_vide;
                    Log::info(format!("Tex in format {:?}", tex_i.get_format().unwrap()));
                    let mip = -1;
                    if let Some((w, h, size)) = tex_i.get_data_infos(mip) {
                        let color128_buff = vec![self.base_color128; size];
                        Log::info(format!("--Tex in mip {} -> {}x{} = {}", mip, w, h, color128_buff.len()));
                        if tex_i.get_colors(color128_buff.as_slice(), mip).is_some() {
                            self.tex_vide.set_colors128(w, h, &color128_buff[..]);
                            Log::info(format!("--Tex out mips number {}", self.tex_vide.get_mips().unwrap()));
                        }
                    }

                    let tex_i = &self.tex_vide2;
                    Log::info(format!("Tex in format {:?}", tex_i.get_format().unwrap()));
                    let mip = -1;
                    if let Some((w, h, size)) = tex_i.get_data_infos(mip) {
                        let color_r8_buff: Vec<u8> = vec![60; size];
                        Log::info(format!("--Tex in mip {} -> {}x{} = {}", mip, w, h, color_r8_buff.len()));
                        if tex_i.get_colors(color_r8_buff.as_slice(), mip).is_some() {
                            self.tex_vide2.set_colors_r8(w, h, &color_r8_buff[..]);
                            Log::info(format!("--Tex out mips number {}", self.tex_vide2.get_mips().unwrap()));
                        }
                    }

                    let tex_i = &self.tex_vide3;
                    Log::info(format!("Tex in format {:?}", tex_i.get_format().unwrap()));
                    let mip = -1;
                    if let Some((w, h, size)) = tex_i.get_data_infos(mip) {
                        let color_r16_buff: Vec<u16> = vec![60; size];
                        Log::info(format!("--Tex in mip {} -> {}x{} = {}", mip, w, h, color_r16_buff.len()));
                        if tex_i.get_colors(color_r16_buff.as_slice(), mip).is_some() {
                            self.tex_vide3.set_colors_r16(w, h, &color_r16_buff[..]);
                            Log::info(format!("--Tex out mips number {}", self.tex_vide3.get_mips().unwrap()));
                        }
                    }

                    let tex_i = &self.tex_vide4;
                    Log::info(format!("Tex in format {:?}", tex_i.get_format().unwrap()));
                    let mip = -1;
                    if let Some((w, h, size)) = tex_i.get_data_infos(mip) {
                        let color_r32_buff: Vec<f32> = vec![0.5; size];
                        Log::info(format!("--Tex in mip {} -> {}x{} = {}", mip, w, h, color_r32_buff.len()));
                        if tex_i.get_colors(color_r32_buff.as_slice(), mip).is_some() {
                            self.tex_vide4.set_colors_r32(w, h, &color_r32_buff[..]);
                            Log::info(format!("--Tex out mips number {}", self.tex_vide4.get_mips().unwrap()));
                        }
                    }
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
        }
    }

    fn shutdown(&mut self) {}
}
