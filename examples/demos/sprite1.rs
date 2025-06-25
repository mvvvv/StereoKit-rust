use glam::Mat4;
use std::f32::consts::PI;
use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Vec3, Vec4},
    prelude::*,
    sprite::{Sprite, SpriteType},
    system::{AssetType, Assets, Lines, Pivot, Text, TextStyle},
    tex::Tex,
    util::{
        Color128, Gradient,
        named_colors::{BLACK, BLUE, CYAN, LIGHT_BLUE, WHITE, YELLOW},
    },
};

#[derive(IStepper)]
pub struct Sprite1 {
    pub title: String,
    id: StepperId,

    sk_info: Option<Rc<RefCell<SkInfo>>>,
    tex_particule1: Tex,
    tex_particule2: Tex,
    color1: Material,
    color2: Material,
    sprite1: Sprite,
    sprite_ico: Sprite,
    sprite3: Sprite,
    sprite4: Sprite,
    text_style: TextStyle,
}

impl Sprite1 {
    /// Change the default title.
    pub fn new(title: String) -> Self {
        Self { title, ..Default::default() }
    }
}

unsafe impl Send for Sprite1 {}

impl Default for Sprite1 {
    fn default() -> Self {
        //---- Textures
        let mut gradient = Gradient::new(None);
        gradient
            .add(Color128::BLACK_TRANSPARENT, 0.0)
            .add(YELLOW, 0.1)
            .add(LIGHT_BLUE, 0.4)
            .add(BLUE, 0.5)
            .add(BLACK, 0.7);
        let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
        let mut tex_particule2 = Tex::gen_particle(128, 128, 0.2, None);
        tex_particule2.id("tagada");

        //---- Some Text
        let font_files = if cfg!(windows) {
            ["C:\\Windows\\Fonts\\Arial.ttf", "C:\\Windows\\Fonts\\Calibri.ttf"]
        } else {
            [
                "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            ]
        };
        let font = Font::from_files(&font_files).unwrap_or_default();
        let text_style = Text::make_style(font, 0.50, CYAN);

        let mut this = Self {
            title: "Stereokit Sprites".to_owned(),
            id: "Sprite 1".to_string(),
            sk_info: None,

            //----- Materials
            color1: Material::pbr_clip().copy(),
            color2: Material::pbr_clip().copy(),

            //----- Sprites
            sprite1: Sprite::from_tex(&tex_particule1, None, None).unwrap(),
            sprite_ico: Sprite::from_file("textures/open_gltf.jpeg", Some(SpriteType::Single), Some("tagada")).unwrap(),
            sprite3: Sprite::from_tex(&tex_particule1, None, None).unwrap(),
            sprite4: Sprite::from_tex(&tex_particule2, None, None).unwrap(),
            tex_particule1,
            tex_particule2,
            text_style,
        };
        this.color1
            .id("color mat 1")
            .diffuse_tex(&this.tex_particule1)
            .tex_transform(Vec4::new(0.0, 0.0, 1.0, 1.0));
        this.color2
            .id("color mat 2")
            .diffuse_tex(&this.tex_particule2)
            .tex_transform(Vec4::new(0.0, 0.0, 1.0, 1.0));
        this.sprite1.id("basic1");
        this.sprite_ico.id("basic2");
        this
    }
}

impl Sprite1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // ---Some logs
        for sprite in [&self.sprite1, &self.sprite_ico] {
            Log::diag(format!(
                "sprite {} => aspect :{} / height : {} / width :{} / normalized dimensions :{}",
                sprite.get_id(),
                sprite.get_aspect(),
                sprite.get_height(),
                sprite.get_width(),
                sprite.get_normalized_dimensions()
            ));
        }

        Assets::block_for_priority(i32::MAX);

        for asset in Assets::all().filter(|s| !s.to_string().contains(" default/")) {
            Log::diag(format!("{}", asset));
        }

        for asset in Assets::all_of_type(AssetType::Sprite) {
            Log::diag(format!("{}", asset));
        }
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI
    fn draw(&mut self, token: &MainThreadToken) {
        self.sprite1
            .draw(token, Mat4::from_translation(glam::Vec3::new(-2.5, 1.5, -2.5)), Pivot::Center, None);

        self.sprite_ico.draw(
            token,
            Mat4::from_rotation_translation(glam::Quat::from_rotation_y(PI), glam::Vec3::new(0.0, 1.5, -2.5)),
            Pivot::BottomCenter,
            None,
        );

        self.sprite3.draw(token, Mat4::from_translation(glam::Vec3::new(2.5, 1.5, -2.5)), Pivot::YTop, None);

        self.sprite4.draw(token, Mat4::from_translation(glam::Vec3::new(0.0, 3.5, -2.5)), Pivot::YTop, None);

        Text::add_at(
            token,
            &self.title,
            Matrix::t_r(Vec3::new(0.0, 1.0, -4.0), Quat::from_angles(0.0, 180.0, 0.0)),
            Some(self.text_style),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        Lines::add(token, Vec3::Z, Vec3::X, WHITE, None, 0.03);

        Lines::add_axis(token, Matrix::t(Vec3::ONE * 2.0).get_pose(), None, None);
    }
}
