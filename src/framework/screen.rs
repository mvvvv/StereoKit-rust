use std::f32::consts::PI;

use crate::{
    font::Font,
    framework::StepperId,
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Vec2, Vec3},
    mesh::{Inds, Mesh, Vertex},
    sk::MainThreadToken,
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{Input, Renderer, Text, TextStyle},
    tex::Tex,
    ui::{Ui, UiBtnLayout, UiMove, UiWin},
    util::named_colors::RED,
};

pub struct ScreenRepo {
    id_btn_show_hide_param: String,
    id_window_param: String,
    show_param: bool,
    sprite_hide_param: Sprite,
    sprite_show_param: Sprite,
    id_handle: String,
    id_material: String,
    id_texture: String,
    id_left_sound: String,
    id_right_sound: String,
    id_slider_distance: String,
    id_slider_size: String,
    id_slider_flattening: String,
}

impl ScreenRepo {
    pub fn new(id: String) -> Self {
        Self {
            show_param: false,
            sprite_hide_param: Sprite::close(),
            sprite_show_param: Sprite::from_file("icons/hamburger.png", None, None).unwrap_or_default(),
            id_btn_show_hide_param: id.clone() + "_btn_show_hide",
            id_window_param: id.clone() + "_window_param",
            id_handle: id.clone() + "_handle",
            id_material: id.clone() + "_material",
            id_texture: id.clone() + "_texture",
            id_left_sound: id.clone() + "_left_sound",
            id_right_sound: id.clone() + "_right_sound",
            id_slider_distance: id.clone() + "_slider_distance",
            id_slider_size: id.clone() + "_slider_size",
            id_slider_flattening: id.clone() + "_slider_radius",
        }
    }
}

/// The video stepper
pub struct Screen {
    id: StepperId,

    repo: ScreenRepo,
    pub width: i32,
    pub height: i32,
    pub screen_distance: f32,
    pub screen_flattening: f32,
    pub screen_size: Vec2,
    pub screen_diagonal: f32,
    pub screen_pose: Pose,
    pub screen: Mesh,
    pub sound_spacing_factor: f32,
    pub text: String,
    pub transform: Matrix,
    pub text_style: Option<TextStyle>,
    screen_material: Material,

    sound_left: Sound,
    sound_left_inst: Option<SoundInst>,
    sound_right: Sound,
    sound_right_inst: Option<SoundInst>,
}

unsafe impl Send for Screen {}

/// This code may be called in some threads, so no StereoKit code
impl Default for Screen {
    fn default() -> Self {
        let screen_size = Vec2::new(3.840, 2.160);
        let screen_diagonal = (screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt();
        let screen_material = Material::unlit().copy();

        Self {
            id: "Screen1".to_string(),

            repo: ScreenRepo::new("Screen1".to_string()),
            width: 3840,
            height: 2160,
            screen_distance: 2.20,
            screen_flattening: 0.99,
            screen_size,
            screen_diagonal,
            screen_pose: Pose::IDENTITY,
            screen: Mesh::new(),
            sound_spacing_factor: 3.0,
            text: "Screen1".to_owned(),
            transform: Matrix::t_r(
                Vec3::new(0.0, 2.0, -2.5), //
                Quat::from_angles(0.0, 180.0, 0.0),
            ),
            text_style: Some(Text::make_style(Font::default(), 0.3, RED)),
            screen_material,

            sound_left: Sound::click(),
            sound_left_inst: None,
            sound_right: Sound::click(),
            sound_right_inst: None,
        }
    }
}

/// All the code here run in the main thread
impl Screen {
    /// Create the video player
    pub fn new(id: &str, screen_tex: impl AsRef<Tex>) -> Self {
        let mut this = Self { ..Default::default() };

        this.id = id.to_string();
        this.repo = ScreenRepo::new(this.id.clone());

        let screen_tex = screen_tex.as_ref().clone_ref();

        this.repo.id_texture = screen_tex.as_ref().get_id().to_string();
        this.screen_material.id(&this.repo.id_material).diffuse_tex(&screen_tex);

        this.sound_left = Sound::create_stream(200.0).unwrap_or_default();
        this.sound_left.id(&this.repo.id_left_sound);
        this.sound_right = Sound::create_stream(200.0).unwrap_or_default();
        this.sound_right.id(&this.repo.id_right_sound);

        this.screen_pose = Input::get_head() * Matrix::r(Quat::from_angles(0.0, 180.0, 0.0));
        this.adapt_screen();

        this.sound_left_inst = Some(this.sound_left.play(this.sound_position(-1), Some(1.0)));
        this.sound_right_inst = Some(this.sound_right.play(this.sound_position(1), Some(1.0)));

        this
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    pub fn draw(&mut self, token: &MainThreadToken) {
        let screen_transform = self.screen_param();

        Renderer::add_mesh(token, &self.screen, &self.screen_material, screen_transform, None, None);

        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }

    /// Here is managed the screen position, its rotundity, size and distance
    fn screen_param(&mut self) -> Matrix {
        const GRAB_X_MARGIN: f32 = 0.4;

        const MAX_DISTANCE: f32 = 6.0;

        const MAX_DIAGONAL: f32 = 15.0;
        const MIN_DIAGONAL: f32 = 0.2;
        let bounds = self.screen.get_bounds();

        let factor_size = (self.screen_distance.max(1.0).powf(2.0) + self.screen_diagonal.max(1.0).powf(2.0)).sqrt();

        let grab_position = Vec3::new(
            0.0, //
            self.screen_size.y / 2.0 + 0.05 * factor_size,
            bounds.center.z,
        );
        let grab_dimension = Vec3::new(
            factor_size * 0.2, //
            factor_size * 0.01,
            factor_size * 0.01,
        );
        if Ui::handle(
            &self.repo.id_handle,
            &mut self.screen_pose,
            Bounds::new(grab_position, grab_dimension),
            true,
            Some(UiMove::Exact),
            None,
        ) {
            let head = Input::get_head();
            self.screen_pose.position = head.position;
        }
        let screen_transform = self.screen_pose.to_matrix(None);

        let mut adapt = false;
        if self.repo.show_param {
            let info_position = Vec3::new(bounds.center.x, bounds.center.y, GRAB_X_MARGIN * 1.5);
            let mut window_pose = Pose::new(info_position, None) * screen_transform;
            Ui::window_begin(
                &self.repo.id_window_param,
                &mut window_pose,
                Some(Vec2::new(0.4, 0.2)),
                Some(UiWin::Body),
                Some(UiMove::None),
            );

            if Ui::button_img(
                &self.repo.id_btn_show_hide_param,
                &self.repo.sprite_hide_param,
                Some(UiBtnLayout::CenterNoText),
                None,
                None,
            ) {
                self.repo.show_param = false;
            }
            Ui::label("Distance", None, true);
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_distance), None, true);
            Ui::same_line();
            let old_value = self.screen_distance;
            if let Some(_new_value) = Ui::hslider(
                &self.repo.id_slider_distance,
                &mut self.screen_distance,
                GRAB_X_MARGIN * 2.0,
                MAX_DISTANCE,
                None,
                None,
                None,
                None,
            ) {
                let max_size = self.screen_distance * PI;
                let screen_size = self.screen_size;
                if screen_size.x > max_size || self.screen_size.y > max_size {
                    self.screen_distance = old_value;
                } else {
                    adapt = true;
                }
            }

            Ui::label("Diagonal", None, true);
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_diagonal), None, true);
            Ui::same_line();
            let old_value = self.screen_diagonal;
            if let Some(new_value) = Ui::hslider(
                &self.repo.id_slider_size,
                &mut self.screen_diagonal,
                MIN_DIAGONAL,
                MAX_DIAGONAL,
                None,
                None,
                None,
                None,
            ) {
                let max_size = self.screen_distance * PI;
                let screen_size = self.screen_size * new_value / old_value;
                if screen_size.x > max_size || self.screen_size.y > max_size {
                    self.screen_diagonal = old_value;
                } else {
                    self.screen_size = screen_size;
                    adapt = true;
                }
            }

            Ui::label("Curvature", None, true);
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_flattening), None, true);
            Ui::same_line();
            if let Some(new_value) = Ui::hslider(
                &self.repo.id_slider_flattening,
                &mut self.screen_flattening,
                0.0,
                1.0,
                None,
                None,
                None,
                None,
            ) {
                self.screen_flattening = new_value;
                adapt = true;
            }
        } else {
            let info_position = Vec3::new(
                0.0, //
                self.screen_size.y / 2.0 + 0.04 * factor_size,
                bounds.center.z,
            );
            let mut window_pose = Pose::new(info_position, None) * screen_transform;
            Ui::window_begin(&self.repo.id_window_param, &mut window_pose, None, Some(UiWin::Body), Some(UiMove::None));
            if Ui::button_img(
                &self.repo.id_btn_show_hide_param,
                &self.repo.sprite_show_param,
                Some(UiBtnLayout::CenterNoText),
                Some(Vec2::new(0.03 * factor_size, 0.03 * factor_size)),
                None,
            ) {
                self.repo.show_param = true;
                let head = Input::get_head();
                self.screen_pose.position = head.position;
            }
        }
        Ui::window_end();
        if adapt {
            self.adapt_screen();
        }
        screen_transform
    }

    /// Calculate sound position. If factor < 0 this is for left else for right
    fn sound_position(&self, factor: i8) -> Vec3 {
        let up = self.screen_pose.get_up();
        let forward = self.screen_pose.get_forward();
        let cross = Vec3::cross(up, forward);
        cross * factor as f32 * self.sound_spacing_factor
    }

    fn adapt_screen(&mut self) {
        let distance = self.screen_distance;
        let flattening = if self.screen_flattening <= 0.0 { 500.0 } else { 1.0 / self.screen_flattening - 1.0 };
        let radius = distance + flattening;

        let width = self.screen_size.x;
        let height = self.screen_size.y;

        self.screen = {
            let mut verts: Vec<Vertex> = vec![];
            let mut inds: Vec<Inds> = vec![];

            let aspect_ratio = width / height;

            let perimeter = 2.0 * PI * radius;

            let subdiv_v = 30u32;
            let subdiv_u = (subdiv_v as f32 * aspect_ratio) as u32;

            let angle_v = 2.0 * PI * height / perimeter;
            let angle_u = 2.0 * PI * width / perimeter;
            let delta_v = angle_v / subdiv_v as f32;
            let delta_u = angle_u / subdiv_u as f32;

            for j in 0..subdiv_v {
                let v = -angle_v / 2.0 + (j as f32 * delta_v) + PI / 2.0;
                for i in 0..subdiv_u {
                    let u = -angle_u / 2.0 + (i as f32 * delta_u) + PI / 2.0;
                    let x = radius * v.sin() * u.cos();
                    let y = radius * v.cos();
                    let z = radius * v.sin() * u.sin() - flattening;

                    verts.push(Vertex::new(
                        Vec3::new(x, y, z), //
                        Vec3::FORWARD,
                        Some(Vec2::new(i as f32 / (subdiv_u - 1) as f32, j as f32 / (subdiv_v - 1) as f32)),
                        None,
                    ));

                    //Log::diag(format!("vertex: {} {} {}", x, y, z));

                    let nb_row = subdiv_u;
                    let last_line = j == subdiv_v - 1;
                    if !last_line {
                        let row_is_even = i % 2 == 0;
                        let last_row = i == nb_row - 1;
                        let a = j * nb_row + i;
                        let b = j * nb_row + i + 1;
                        let c = (j + 1) * nb_row + i;
                        if row_is_even {
                            if !last_row {
                                inds.push(a);
                                inds.push(b);
                                inds.push(c);
                                inds.push(a);
                                inds.push(c);
                                inds.push(b);
                                //Log::diag(format!("inds: a{} b{} c{}", a, b, c));
                            }
                        } else {
                            let c_previous = (j + 1) * nb_row + i - 1;
                            let c_following = (j + 1) * nb_row + i + 1;
                            inds.push(a);
                            inds.push(c);
                            inds.push(c_previous);
                            inds.push(a);
                            inds.push(c_previous);
                            inds.push(c);
                            if !last_row {
                                inds.push(a);
                                inds.push(c_following);
                                inds.push(c);
                                inds.push(a);
                                inds.push(c);
                                inds.push(c_following);

                                inds.push(a);
                                inds.push(b);
                                inds.push(c_following);
                                inds.push(a);
                                inds.push(c_following);
                                inds.push(b);
                                //Log::diag(format!("inds: a{} b{} c{} c-{} c+{} ", a, b, c, c_previous, c_following));
                            } else {
                                //Log::diag(format!("inds: a{} c{} c-{} ", a, c, c_previous));
                            }
                        }
                    }
                }
            }

            let mut mesh = Mesh::new();
            mesh.set_inds(inds.as_slice());
            mesh.set_verts(verts.as_slice(), true);

            mesh
        };
    }
}
