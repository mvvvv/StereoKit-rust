use std::f32::consts::PI;

use crate::util::named_colors;
use crate::{
    framework::StepperId,
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Ray, Vec2, Vec3},
    mesh::{Inds, Mesh, Vertex},
    sk::MainThreadToken,
    sound::{Sound, SoundInst},
    sprite::Sprite,
    system::{Input, Lines, Renderer},
    tex::Tex,
    ui::{Ui, UiBtnLayout, UiMove, UiWin},
};

pub struct ScreenRepo {
    id_btn_show_hide_param: String,
    id_window_param: String,
    show_param: bool,
    sprite_hide_param: Sprite,
    sprite_show_param: Sprite,
    id_handle: String,
    id_material: String,
    // id_left_sound: String,
    // id_right_sound: String,
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
            // id_left_sound: id.clone() + "_left_sound",
            // id_right_sound: id.clone() + "_right_sound",
            id_slider_distance: id.clone() + "_slider_distance",
            id_slider_size: id.clone() + "_slider_size",
            id_slider_flattening: id.clone() + "_slider_radius",
        }
    }
}

/// The screen struct
/// ```ignore
/// // Create a screen with a first texture
/// let mut screen = Screen::new("my_screen", texture1);
///
/// // Add a second texture
/// screen.set_texture(1, Some(texture2));
///
/// // Switch to the second texture
/// screen.set_tex_curr(1);
///
/// // Return to the first texture
/// screen.set_tex_curr(0);
/// ```
pub struct Screen {
    id: StepperId,

    repo: ScreenRepo,
    screen_distance: f32,
    screen_flattening: f32,
    screen_size: Vec2,
    screen_diagonal: f32,
    screen_pose: Pose,
    screen: Mesh,
    sound_spacing_factor: f32,
    ray_thickness: f32,

    screen_material: Material,
    screen_textures: [Option<Tex>; 2],
    tex_curr: usize,

    sound_left: Sound,
    sound_left_inst: Option<SoundInst>,
    sound_right: Sound,
    sound_right_inst: Option<SoundInst>,
}

unsafe impl Send for Screen {}

/// All the code here run in the main thread
impl Screen {
    pub const MAX_DISTANCE: f32 = 6.0;
    pub const MAX_DIAGONAL: f32 = 15.0;
    pub const MIN_DIAGONAL: f32 = 0.2;

    /// Create the screen
    pub fn new(id: &str, screen_tex: impl AsRef<Tex>) -> Self {
        let screen_size = Vec2::new(3.840, 2.160);
        let screen_diagonal = (screen_size.x.powf(2.0) + screen_size.y.powf(2.0)).sqrt();
        let screen_material = Material::ui().copy();

        let mut this = Self {
            id: id.to_string(),

            repo: ScreenRepo::new(id.to_string()),
            screen_distance: 2.20,
            screen_flattening: 0.99,
            screen_size,
            screen_diagonal,
            screen_pose: Pose::IDENTITY,
            screen: Mesh::new(),
            sound_spacing_factor: 3.0,
            ray_thickness: 0.005,

            screen_material,
            screen_textures: [None, None],
            tex_curr: 0,

            sound_left: Sound::click(),
            sound_left_inst: None,
            sound_right: Sound::click(),
            sound_right_inst: None,
        };

        let screen_tex = screen_tex.as_ref().clone_ref();

        this.screen_textures[0] = Some(screen_tex.clone_ref());
        this.screen_material.id(&this.repo.id_material);
        this.update_material_texture();

        this.sound_left = Sound::create_stream(200.0).unwrap_or_default();
        this.sound_right = Sound::create_stream(200.0).unwrap_or_default();

        this.screen_pose = Input::get_head() * Matrix::r(Quat::from_angles(0.0, 180.0, 0.0));
        this.adapt_screen();

        this.sound_left_inst = Some(this.sound_left.play(this.sound_position(-1), Some(1.0)));
        this.sound_right_inst = Some(this.sound_right.play(this.sound_position(1), Some(1.0)));

        this
    }

    /// Set the screen distance
    pub fn screen_distance(&mut self, distance: f32) -> &mut Self {
        let max_size = self.screen_distance * PI;
        let screen_size = self.screen_size;
        if screen_size.x > max_size || self.screen_size.y > max_size {
            // self.screen_distance = old_value;
        } else {
            let min_distance = (self.screen_size.x.max(self.screen_size.y)) / PI;
            self.screen_distance = distance.max(min_distance).min(Self::MAX_DISTANCE);
            self.adapt_screen();
        }
        self
    }

    /// Set the screen flattening (0.0 to 1.0)
    pub fn screen_flattening(&mut self, flattening: f32) -> &mut Self {
        self.screen_flattening = flattening.clamp(0.0, 1.0);
        self.adapt_screen();
        self
    }

    /// Set the screen size
    pub fn screen_size(&mut self, size: impl Into<Vec2>) -> &mut Self {
        let size = size.into();
        let max_size = self.screen_distance * PI;
        let screen_diagonal = (size.x.powf(2.0) + size.y.powf(2.0)).sqrt();
        if size.x <= max_size
            && size.y <= max_size
            && size.x > 0.0
            && size.y > 0.0
            && screen_diagonal > Self::MIN_DIAGONAL
        {
            self.screen_size = size;
            self.screen_diagonal = screen_diagonal;
            self.adapt_screen();
        }
        self
    }

    /// Set the screen diagonal (automatically adjusts screen_size proportionally)
    pub fn screen_diagonal(&mut self, diagonal: f32) -> &mut Self {
        let max_size = self.screen_distance * PI;
        let screen_size = self.screen_size * diagonal / self.screen_diagonal;
        if screen_size.x > max_size || self.screen_size.y > max_size {
            // self.screen_diagonal = old_value;
        } else {
            self.screen_size = screen_size;
            self.screen_diagonal = diagonal;

            self.adapt_screen();
        }
        self
    }

    /// Set the screen orientation (position remains anchored to head)
    pub fn screen_orientation(&mut self, orientation: impl Into<Quat>) -> &mut Self {
        let orientation = orientation.into();
        let head = Input::get_head();
        self.screen_pose.orientation = orientation;
        self.screen_pose.position = head.position;
        self
    }

    /// Set the sound spacing factor
    pub fn sound_spacing_factor(&mut self, factor: f32) -> &mut Self {
        self.sound_spacing_factor = factor;
        self
    }

    /// Set the ray thickness
    pub fn ray_thickness(&mut self, thickness: f32) -> &mut Self {
        self.ray_thickness = thickness.max(0.001);
        self
    }

    /// Set the current texture index (0 or 1)
    pub fn set_tex_curr(&mut self, tex_index: usize) -> &mut Self {
        if tex_index < 2 {
            self.tex_curr = tex_index;
            self.update_material_texture();
        }
        self
    }

    /// Set a texture at the specified index (0 or 1)
    pub fn set_texture(&mut self, index: usize, texture: Option<Tex>) -> &mut Self {
        if index < 2 {
            self.screen_textures[index] = texture;
            if index == self.tex_curr {
                self.update_material_texture();
            }
        }
        self
    }

    /// Update the material's diffuse texture based on the current texture index
    fn update_material_texture(&mut self) {
        if let Some(ref texture) = self.screen_textures[self.tex_curr] {
            self.screen_material.diffuse_tex(texture);
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI and scene
    pub fn draw(&mut self, token: &MainThreadToken) {
        let screen_transform = self.screen_param();

        Renderer::add_mesh(token, &self.screen, &self.screen_material, screen_transform, None, None);
    }

    /// Here is managed the screen position, its rotundity, size and distance
    fn screen_param(&mut self) -> Matrix {
        const GRAB_X_MARGIN: f32 = 0.4;

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
            let mut screen_distance = self.screen_distance;
            if let Some(new_value) = Ui::hslider(
                &self.repo.id_slider_distance,
                &mut screen_distance,
                GRAB_X_MARGIN * 2.0,
                Self::MAX_DISTANCE,
                None,
                None,
                None,
                None,
            ) {
                self.screen_distance(new_value);
            }

            Ui::label("Diagonal", None, true);
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_diagonal), None, true);
            Ui::same_line();
            let mut screen_diagonal = self.screen_diagonal;
            if let Some(new_value) = Ui::hslider(
                &self.repo.id_slider_size,
                &mut screen_diagonal,
                Self::MIN_DIAGONAL,
                Self::MAX_DIAGONAL,
                None,
                None,
                None,
                None,
            ) {
                self.screen_diagonal(new_value);
            }

            Ui::label("Curvature", None, true);
            Ui::same_line();
            Ui::label(format!("{:.2}", self.screen_flattening), None, true);
            Ui::same_line();
            let mut screen_flattening = self.screen_flattening;
            if let Some(new_value) =
                Ui::hslider(&self.repo.id_slider_flattening, &mut screen_flattening, 0.0, 1.0, None, None, None, None)
            {
                self.screen_flattening(new_value);
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

    /// Check if the screen has been touched and return the position (x,y) in screen coordinates
    /// Returns Some((x, y)) if touched, None otherwise
    /// Coordinates are normalized between 0.0 and 1.0
    pub fn touched(&self, token: &MainThreadToken, index: i32) -> Option<(f32, f32)> {
        // no ray when adjusting params
        if self.repo.show_param {
            return None;
        }

        // Transform from world into the screen's local/model space
        // Our screen mesh is drawn with transform = self.screen_pose.to_matrix(None)
        // So to bring a world ray into model space, multiply by the inverse
        let screen_mtx = self.screen_pose.to_matrix(None);
        let inv = screen_mtx.get_inverse();

        let p = Input::pointer(index, None);

        // Bring the pointer ray into model space
        let local_ray = inv.transform_ray(p.ray);

        // Use a precise raycast that also gives us the first triangle index
        let (mut hit_ray, mut tri_start_index) = (Ray::default(), 0u32);
        let hit = self.screen.intersect_to_ptr(local_ray, None, &mut hit_ray, &mut tri_start_index);
        if !hit {
            return None;
        }

        // we draw the ray
        //self.draw_ray(token, p.ray);
        Lines::add_ray(token, p.ray, self.screen_distance, named_colors::WHITE, None, self.ray_thickness);

        if !p.state.is_just_inactive() {
            return None;
        }
        // Retrieve the triangle's vertices to barycentrically interpolate UV
        let tri = self.screen.get_triangle(tri_start_index)?;
        let [a, b, c] = tri;

        // Compute barycentric coordinates of hit point on triangle ABC
        let p_hit = hit_ray.position; // hit point in model space
        let v0 = b.pos - a.pos;
        let v1 = c.pos - a.pos;
        let v2 = p_hit - a.pos;
        let d00 = Vec3::dot(v0, v0);
        let d01 = Vec3::dot(v0, v1);
        let d11 = Vec3::dot(v1, v1);
        let d20 = Vec3::dot(v2, v0);
        let d21 = Vec3::dot(v2, v1);
        let denom = d00 * d11 - d01 * d01;
        if denom == 0.0 {
            return None;
        }
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;

        // Interpolate UVs and return normalized coordinates
        let hit_uv = a.uv * u + b.uv * v + c.uv * w;

        // UVs are already normalized [0,1] on our mesh
        Some((hit_uv.x, hit_uv.y))
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Get the screen mesh
    pub fn get_mesh(&self) -> &Mesh {
        &self.screen
    }

    /// Get the current screen distance
    pub fn get_screen_distance(&self) -> f32 {
        self.screen_distance
    }

    /// Get the current screen flattening
    pub fn get_screen_flattening(&self) -> f32 {
        self.screen_flattening
    }

    /// Get the current screen size
    pub fn get_screen_size(&self) -> Vec2 {
        self.screen_size
    }

    /// Get the current screen diagonal
    pub fn get_screen_diagonal(&self) -> f32 {
        self.screen_diagonal
    }

    /// Get the current screen orientation
    pub fn get_screen_orientation(&self) -> Quat {
        self.screen_pose.orientation
    }

    /// Get the current sound spacing factor
    pub fn get_sound_spacing_factor(&self) -> f32 {
        self.sound_spacing_factor
    }

    /// Get the current ray thickness
    pub fn get_ray_thickness(&self) -> f32 {
        self.ray_thickness
    }
}
