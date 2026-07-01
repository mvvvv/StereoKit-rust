use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Quat, Rect, Vec3},
    mesh::Mesh,
    prelude::*,
    render::{RenderBuilder, RenderClear, RenderList, RenderListRefs},
    system::{Assets, Text, TextBuilder},
    tex::Tex,
    util::{Time, named_colors},
};

/// Stereo1: renders the scene from two eye cameras into a rendertarget(single or array), and displays the result on a
/// board using the custom `stereo*.hlsl` shader. That shader picks the correct horizontal half of the texture for each
/// eye, so the user sees a true stereoscopic image of the scene as it would be captured from their eyes.
#[derive(IStepper)]
pub struct Stereo1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    // Offscreen render list and the mesh to display it
    scene_list: RenderList,
    display_quad: Mesh,
    // Some scene content to render from the eye cameras so the stereo effect is actually visible.
    scene_sphere: Mesh,
    scene_cube: Mesh,
    pbr_mat: Material,

    // 1a - Render target that holds left/right eye views for side by side (stereo.hlsl)
    render_tex_stereo: Tex,
    render_left: RenderBuilder,
    render_right: RenderBuilder,
    // Displays the stereo image, and the material built from our custom `stereo.hlsl` shader.
    stereo_transform: Matrix,
    stereo_mat: Material,

    // 1b - Render target array with 2 slices: slice 0 = left eye, slice 1 = right eye
    render_tex_array: Tex,
    render_array: RenderBuilder,
    // Display the stereo image array, and the material built from our custom `stereo_array.hlsl` shader.
    stereo_array_transform: Matrix,
    stereo_array_mat: Material,

    title_text: TextBuilder,
}

unsafe impl Send for Stereo1 {}

impl Default for Stereo1 {
    fn default() -> Self {
        let eye_res = 512;
        let ipd = 0.064; // typical human interpupillary distance in meters
        let [cam_left, cam_right] = Self::eye_cameras(ipd);
        let proj = Matrix::perspective(100.0, 1.0, 0.05, 50.0);

        // Offscreen render list (untracked: filled and drained each frame).
        let mut scene_list = RenderList::new_with(RenderListRefs::None);
        scene_list.id("Stereo1Scene");
        // Simple scene content.
        let scene_sphere = Mesh::generate_sphere(0.25, None);
        let scene_cube = Mesh::generate_cube(Vec3::ONE * 0.3, None);
        let pbr_mat = Material::pbr().copy();

        // The board the user looks at: a screen-space quad facing the user.
        let display_quad = Mesh::generate_plane([1.0, 1.0], Vec3::Z, Vec3::Y, None, true);

        //-----------------------------------------------------
        // 1-A A rendertarget that will receive both eyes side-by-side. We use a 2:1 aspect texture and render each eye
        // into its own half through a viewport rect.
        let render_tex_stereo = Tex::render_target(eye_res * 2, eye_res, None, None, None)
            .expect("Stereo1 render target should be created");
        // Left eye into the left half of the texture.
        let render_left = RenderBuilder::new()
            .camera(cam_left)
            .projection(proj)
            .clear(RenderClear::All)
            .viewport(Rect::new(0.0, 0.0, 0.5, 1.0));
        // Right eye into the right half. Don't clear color again so we keep the left half.
        let render_right = RenderBuilder::new()
            .camera(cam_right)
            .projection(proj)
            .clear(RenderClear::None)
            .viewport(Rect::new(0.5, 0.0, 0.5, 1.0));
        // Custom stereo shader material.
        let mut stereo_mat =
            Material::from_file("shaders/stereo.hlsl.sks", Some("stereo")).unwrap_or_else(|_| Material::unlit().copy());
        stereo_mat.diffuse_tex(&render_tex_stereo).color_tint(named_colors::WHITE);
        let stereo_transform = Matrix::t_r(Vec3::new(0.0, 1.7, -1.2), Quat::Y_180);

        //-----------------------------------------------------
        // 1-B Render target array with 2 slices for stereo: each eye renders to its own slice.
        // We need to create a texture array:
        let render_tex_array = Self::create_render_target_array(eye_res, eye_res, 2);
        // Single RenderBuilder with 2 cameras and 2 projections for multi-view rendering.
        // StereoKit will automatically route each view to consecutive array slices:
        // View 0 (left eye) -> slice 0, View 1 (right eye) -> slice 1
        let render_array = RenderBuilder::new()
            .cameras(vec![cam_left, cam_right])
            .projections(&[proj, proj])
            .clear(RenderClear::All)
            .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
        // Custom stereo_array shader material.
        let mut stereo_array_mat = Material::from_file("shaders/stereo_array.hlsl.sks", Some("stereo_array"))
            .unwrap_or_else(|_| Material::unlit().copy());
        stereo_array_mat.diffuse_tex(&render_tex_array).color_tint(named_colors::WHITE);
        let stereo_array_transform = Matrix::t_r(Vec3::new(0.0, 0.6, -1.2), Quat::Y_180);

        let title_text = TextBuilder::new("Stereo1")
            .transform(Matrix::t_r(Vec3::new(0.0, 1.1, -1.5), Quat::Y_180))
            .style(Text::make_style(Font::default(), 0.08, named_colors::CYAN));
        Self {
            id: "Stereo1".to_string(),
            sk_info: None,

            scene_list,
            scene_sphere,
            scene_cube,
            pbr_mat,

            render_tex_stereo,
            render_left,
            render_right,
            stereo_transform,
            stereo_mat,

            render_tex_array,
            render_array,
            stereo_array_transform,
            stereo_array_mat,

            display_quad,

            title_text,
        }
    }
}

impl Stereo1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        Assets::block_for_priority(i32::MAX);
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        let t = Time::get_totalf();

        // 1) Fill the offscreen list with some animated scene content placed
        // in front of where the eyes will be looking.
        self.scene_list.clear();

        let sphere_transform = Matrix::t_r_s(
            Vec3::new(t.sin() * 0.4, 1.0 + (t * 1.7).sin() * 0.2, -1.0),
            Quat::from_angles(t * 60.0, t * 40.0, 0.0),
            Vec3::ONE,
        );
        let cube_transform = Matrix::t_r_s(
            Vec3::new(-t.sin() * 0.4, 1.0 - (t * 1.3).sin() * 0.2, -1.0),
            Quat::from_angles(t * 50.0, -t * 30.0, 0.0),
            Vec3::ONE,
        );
        self.scene_list
            .add_mesh(&self.scene_sphere, &self.pbr_mat, sphere_transform, named_colors::HONEYDEW, None);
        self.scene_list
            .add_mesh(&self.scene_cube, &self.pbr_mat, cube_transform, named_colors::HOT_PINK, None);

        // 2) Render the scene from both eyes into the side-by-side rendertarget.
        self.render_left.draw_now(&self.scene_list, &self.render_tex_stereo, named_colors::DARK_CYAN);
        self.render_right.draw_now(&self.scene_list, &self.render_tex_stereo, named_colors::DARK_CYAN);

        // 3) Render the scene from both eyes into the texture array:
        //    Left eye -> array slice 0, Right eye -> array slice 1
        //    Multi-view rendering: one call renders both views to consecutive array slices
        self.render_array.draw_now(&self.scene_list, &self.render_tex_array, named_colors::DARK_GOLDEN_ROD);

        // 4) Display the stereo texture on a board using the custom stereo shader, which selects the proper half per eye.
        self.display_quad.draw(&self.stereo_mat, self.stereo_transform, None, None);

        // 5) Display the stereo array texture on a second board below the first. The shader will automatically select
        //    the correct array slice per eye.
        self.display_quad.draw(&self.stereo_array_mat, self.stereo_array_transform, None, None);

        self.title_text.add();
    }

    /// Helper to create a render target with array slices.
    /// Uses low-level tex_create and tex_set_color_arr to configure a texture array for rendering.
    fn create_render_target_array(width: usize, height: usize, array_count: usize) -> Tex {
        use stereokit_rust::tex::{TexFormat, TexType};

        // Create a render target texture with array support
        let tex_type = TexType::ImageNomips | TexType::Rendertarget;
        let mut tex = Tex::new(tex_type, TexFormat::Rgba32Srgb, Some("render_tex_array"));

        tex.set_size(width, height, Some(array_count), None);

        tex
    }

    /// Build two eye cameras: a left eye and a right eye offset by a small interpupillary distance. The eyes look
    /// slightly inward (toed-in cameras) so the scene converges on a focus point in front of the user.
    fn eye_cameras(ipd: f32) -> [Matrix; 2] {
        let focus_distance = 1.0;
        let head_pos = Vec3::NEG_Y;
        let head_rot = Quat::IDENTITY;

        // The point the eyes converge on: straight ahead of the head.
        let forward = head_rot * Vec3::NEG_Z;
        let focus = head_pos + forward * focus_distance;

        let half_ipd = ipd * 0.5;
        let right_dir = head_rot * Vec3::X;

        let left_pos = head_pos - right_dir * half_ipd;
        let right_pos = head_pos + right_dir * half_ipd;

        [
            Matrix::t(left_pos) * Matrix::r(Quat::look_at(left_pos, focus, None)),
            Matrix::t(right_pos) * Matrix::r(Quat::look_at(right_pos, focus, None)),
        ]
    }
}
