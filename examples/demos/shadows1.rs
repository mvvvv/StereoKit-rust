// !!!!!!!
// This is a Copycat of https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Demos/DemoShadows.cs
// !!!!!!!
use stereokit_rust::{
    font::Font,
    material::{Cull, Material, MaterialBuffer},
    maths::{Matrix, Pose, Quat, Vec2, Vec3, Vec4},
    mesh::Mesh,
    model::Model,
    prelude::*,
    sprite::Sprite,
    system::{Input, Log, RenderLayer, Renderer, Text, TextStyle},
    tex::{Tex, TexAddress, TexFormat, TexSample, TexSampleComp, TexType},
    ui::{Ui, UiBtnLayout},
    util::named_colors,
};

use crate::demos::hand_menu_radial1::SKY_DOME_CHANGED;

// ShadowBuffer mirrors the C# DemoShadows struct layout.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ShadowBuffer {
    pub shadow_map_transform: Matrix,
    pub light_direction: Vec3,
    pub shadow_map_bias: f32,
    pub light_color: Vec3,
    pub shadow_map_pixel_size: f32,
}

impl Default for ShadowBuffer {
    fn default() -> Self {
        Self {
            shadow_map_transform: Matrix::IDENTITY,
            light_direction: Vec3::ZERO,
            shadow_map_bias: 0.0,
            light_color: Vec3::new(1.0, 1.0, 1.0),
            shadow_map_pixel_size: 0.0,
        }
    }
}

const SHADOW_MAP_SIZE: f32 = 2.0;
const SHADOW_MAP_RESOLUTION: i32 = 1024; // Higher resolution for better quality
const SHADOW_MAP_NEAR_CLIP: f32 = 0.01;
const SHADOW_MAP_FAR_CLIP: f32 = 20.0;

// Shadow mode configuration
#[derive(Debug, Clone, Copy)]
enum ShadowMode {
    Quantized,        // Original quantized approach
    Stable,           // Non-quantized but stable position
    SceneCentered,    // Centered on scene objects
    TemporalFiltered, // Smoothed over time
}

const SHADOW_MODE: ShadowMode = ShadowMode::SceneCentered;

#[derive(IStepper)]
pub struct Shadows1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    pub text: String,
    description: String,
    transform_text: Matrix,
    text_style: TextStyle,
    transform_description: Matrix,
    description_style: TextStyle,

    model: Model,
    model_pose: Pose,
    shadow_maps: Vec<Tex>, // double buffered
    shadow_buffer: MaterialBuffer<ShadowBuffer>,
    shadow_buffer_last: ShadowBuffer,

    light_dir: Vec3,
    previous_light_pos: Vec3,        // For temporal filtering
    current_shadow_mode: ShadowMode, // Current shadow mode
    window_pose: Pose,               // UI window position
    radio_off: Sprite,
    radio_on: Sprite,
    frame_count: u64,
}

unsafe impl Send for Shadows1 {}

impl Default for Shadows1 {
    fn default() -> Self {
        // Prepare a simple model: floor + a few cubes.
        let mut shadow_mat = Material::from_file("shaders/basic_shadow.hlsl.sks", None).unwrap_or_default();
        shadow_mat.depth_test(stereokit_rust::material::DepthTest::LessOrEq).face_cull(Cull::Back);

        let mut floor_mat = shadow_mat
            .tex_file_copy("textures/parquet2/parquet2.ktx2", true, None)
            .unwrap_or_else(|_| shadow_mat.copy());
        floor_mat.tex_transform(Vec4::new(0.0, 0.0, 2.0, 2.0));

        let model = Shadows1::generate_model(&floor_mat, &shadow_mat);

        // Create depth shadow maps (double buffer)
        let mut shadow_maps = Vec::new();
        for _ in 0..2 {
            // double buffer
            // Use None for the id to avoid duplicate asset ids when creating multiple depth targets
            let mut tex = Tex::new(TexType::Depthtarget, TexFormat::Depth16, None);
            tex.set_size(SHADOW_MAP_RESOLUTION as usize, SHADOW_MAP_RESOLUTION as usize, None, None)
                .sample_mode(TexSample::Linear)
                .sample_comp(Some(TexSampleComp::LessOrEq))
                .address_mode(TexAddress::Clamp);

            shadow_maps.push(tex);
        }

        Self {
            id: StepperId::default(),
            sk_info: None,
            shutdown_completed: false,

            text: "Shadows1".to_string(),
            description:
                "Using Renderer::render_to, set_global_texture,\n MaterialBuffer, we build a basic shadow map."
                    .to_string(),
            transform_text: { Matrix::t_r(Vec3::Y * 2.0 + Vec3::NEG_Z * 2.5, Quat::Y_180) },
            text_style: Text::make_style(Font::default(), 0.3, named_colors::RED),
            transform_description: Matrix::t_r(Vec3::Y * 1.5 + Vec3::NEG_Z * 2.6, Quat::Y_180),
            description_style: Text::make_style(Font::default(), 0.08, named_colors::WHITE),

            model,
            model_pose: Pose::new([0.0, 1.0, -0.5], None),
            shadow_maps,
            shadow_buffer: MaterialBuffer::<ShadowBuffer>::new(),
            shadow_buffer_last: ShadowBuffer::default(),

            light_dir: Renderer::get_sky_light().get_dominent_light_direction(),
            previous_light_pos: Vec3::ZERO,
            current_shadow_mode: SHADOW_MODE,
            window_pose: Pose::new([0.75, 1.8, -0.75], Some(Quat::Y_180)),
            radio_off: Sprite::radio_off(),
            radio_on: Sprite::radio_on(),
            frame_count: 0,
        }
    }
}

impl Shadows1 {
    // Called by derive macro during IStepper::initialize
    fn start(&mut self) -> bool {
        Renderer::set_global_buffer(12, &self.shadow_buffer);
        true
    }

    // Event handling stub (required by derive macro pattern)
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(SKY_DOME_CHANGED) {
            // Recalculate light direction when sky dome changes
            self.light_dir = Renderer::get_sky_light().get_dominent_light_direction();
            Log::info(format!("Light direction updated due to sky dome change: {value}/ {}", self.light_dir));
        }
    }

    // Per-frame drawing (called by macro's generated step)
    fn draw(&mut self, token: &MainThreadToken) {
        self.frame_count += 1;
        // Use cached light_dir instead of recalculating every frame
        self.setup_shadow_map(token, self.light_dir);

        // Show shadow settings window
        self.draw_shadow_settings_window(token);

        Ui::handle("ModelShadow", &mut self.model_pose, self.model.get_bounds(), false, None, None);
        self.model.draw(token, self.model_pose.to_matrix(None), None, None);

        // Display title and description text like in math1 demo
        Text::add_at(token, &self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);
        Text::add_at(
            token,
            &self.description,
            self.transform_description,
            Some(self.description_style),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }

    fn setup_shadow_map(&mut self, token: &MainThreadToken, light_dir: Vec3) {
        let head = Input::get_head();
        let light_orientation = Quat::look_at(Vec3::ZERO, light_dir, None);

        // Different shadow positioning strategies
        let light_pos = match self.current_shadow_mode {
            ShadowMode::Quantized => {
                // Original quantized approach
                let forward_pos =
                    head.position.x0z() + head.get_forward().x0z().get_normalized() * 0.5 * SHADOW_MAP_SIZE;
                Self::quantize_light_pos(
                    forward_pos + light_dir * -10.0,
                    light_orientation,
                    SHADOW_MAP_SIZE / SHADOW_MAP_RESOLUTION as f32,
                )
            }
            ShadowMode::Stable => {
                // Non-quantized but stable position relative to head
                let forward_pos =
                    head.position.x0z() + head.get_forward().x0z().get_normalized() * 0.5 * SHADOW_MAP_SIZE;
                forward_pos + light_dir * -10.0
            }
            ShadowMode::SceneCentered => {
                // Centered on the scene objects instead of following the head
                let scene_center = self.model_pose.position;
                scene_center + light_dir * -10.0
            }
            ShadowMode::TemporalFiltered => {
                // Temporal filtering to smooth position changes
                let forward_pos =
                    head.position.x0z() + head.get_forward().x0z().get_normalized() * 0.5 * SHADOW_MAP_SIZE;
                let target_pos = forward_pos + light_dir * -10.0;

                // Smooth interpolation with previous position
                let smooth_factor = 0.95; // Higher = more smoothing
                if self.previous_light_pos == Vec3::ZERO {
                    self.previous_light_pos = target_pos;
                }
                self.previous_light_pos = self.previous_light_pos * smooth_factor + target_pos * (1.0 - smooth_factor);
                self.previous_light_pos
            }
        };

        let view = Matrix::t_r(light_pos, light_orientation);
        let proj = Matrix::orthographic(SHADOW_MAP_SIZE, SHADOW_MAP_SIZE, SHADOW_MAP_NEAR_CLIP, SHADOW_MAP_FAR_CLIP);

        // Upload previous frame's data then update for next
        self.shadow_buffer.set(&mut self.shadow_buffer_last);
        self.shadow_buffer_last = ShadowBuffer {
            shadow_map_transform: (view.get_inverse() * proj).get_transposed(),
            // Adaptive bias based on shadow mode
            shadow_map_bias: match self.current_shadow_mode {
                ShadowMode::Quantized => 4.0 * (SHADOW_MAP_SIZE / SHADOW_MAP_RESOLUTION as f32).max(0.001),
                ShadowMode::Stable => 2.0 * (SHADOW_MAP_SIZE / SHADOW_MAP_RESOLUTION as f32).max(0.001),
                ShadowMode::SceneCentered => 3.0 * (SHADOW_MAP_SIZE / SHADOW_MAP_RESOLUTION as f32).max(0.001),
                ShadowMode::TemporalFiltered => 2.5 * (SHADOW_MAP_SIZE / SHADOW_MAP_RESOLUTION as f32).max(0.001),
            },
            light_direction: -light_dir,
            light_color: [1.0, 1.0, 1.0].into(),
            shadow_map_pixel_size: 1.0 / SHADOW_MAP_RESOLUTION as f32,
        };

        let render_to = (self.frame_count % self.shadow_maps.len() as u64) as usize;
        let render_next = ((self.frame_count + 1) % self.shadow_maps.len() as u64) as usize;

        Renderer::render_to(
            token,
            &self.shadow_maps[render_to],
            None,
            None,
            view,
            proj,
            Some(RenderLayer::All & !RenderLayer::VFX),
            None,
            None,
        );

        Renderer::set_global_texture(token, 12, Some(&self.shadow_maps[render_next]));
        Renderer::set_global_buffer(12, &self.shadow_buffer);
    }

    /// Draw the shadow settings UI window
    fn draw_shadow_settings_window(&mut self, _token: &MainThreadToken) {
        const WINDOW_SIZE: Vec2 = Vec2::new(0.34, 0.32);
        Ui::window_begin("Shadow Settings", &mut self.window_pose, Some(WINDOW_SIZE), None, None);

        Ui::label("Shadow Mode:", None, false);

        // Radio buttons for each shadow mode using radio_img
        if Ui::radio_img(
            "Quantized",
            matches!(self.current_shadow_mode, ShadowMode::Quantized),
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            self.current_shadow_mode = ShadowMode::Quantized;
            self.previous_light_pos = Vec3::ZERO; // Reset for temporal filtering
        }

        if Ui::radio_img(
            "Stable",
            matches!(self.current_shadow_mode, ShadowMode::Stable),
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            self.current_shadow_mode = ShadowMode::Stable;
            self.previous_light_pos = Vec3::ZERO;
        }

        if Ui::radio_img(
            "Scene Centered",
            matches!(self.current_shadow_mode, ShadowMode::SceneCentered),
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            self.current_shadow_mode = ShadowMode::SceneCentered;
            self.previous_light_pos = Vec3::ZERO;
        }

        if Ui::radio_img(
            "Temporal Filtered",
            matches!(self.current_shadow_mode, ShadowMode::TemporalFiltered),
            &self.radio_off,
            &self.radio_on,
            UiBtnLayout::Left,
            None,
        ) {
            self.current_shadow_mode = ShadowMode::TemporalFiltered;
            self.previous_light_pos = Vec3::ZERO;
        }

        // Display current mode info
        Ui::hseparator();
        let mode_description = match self.current_shadow_mode {
            ShadowMode::Quantized => "Quantized: Reduced shimmer via texel grid alignment",
            ShadowMode::Stable => "Stable: Simple non-quantized positioning",
            ShadowMode::SceneCentered => "Scene Centered: Fixed relative to scene objects",
            ShadowMode::TemporalFiltered => "Temporal Filtered: Smoothed position changes",
        };
        Ui::label(mode_description, Some([0.32, 0.0].into()), true);

        // Resolution info
        Ui::hseparator();
        Ui::label(format!("Resolution: {}x{}", SHADOW_MAP_RESOLUTION, SHADOW_MAP_RESOLUTION), None, false);

        Ui::window_end();
    }

    /// Generates the scene model: a floor plus a set of random cubes, mirroring the C# DemoShadows.GenerateModel.
    fn generate_model(floor_mat: &Material, cube_mat: &Material) -> Model {
        const WIDTH: f32 = 0.5;
        const HEIGHT: f32 = 0.5;
        let size_min = Vec3::new(0.04, 0.04, 0.04);
        let size_max = Vec3::new(0.06, 0.20, 0.06);
        let gen_width = WIDTH - size_max.x;
        let gen_height = HEIGHT - size_max.z;

        // Deterministic pseudo-random generator (seed=1) similar to C# Random(1)
        struct Lcg(u32);
        impl Lcg {
            fn next_f32(&mut self) -> f32 {
                self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
                ((self.0 >> 8) & 0xFFFFFF) as f32 / 16_777_216.0
            }
        }
        let mut rng = Lcg(1);

        let model = Model::new();
        let mut nodes = model.get_nodes();
        nodes.add("Floor", Matrix::s([WIDTH, 0.02, HEIGHT]), Some(&Mesh::cube()), Some(floor_mat), false);

        for i in 0..20 {
            let x = (rng.next_f32() - 0.5) * gen_width;
            let y = (rng.next_f32() - 0.5) * gen_height;
            let size = Vec3::new(
                size_min.x + rng.next_f32() * (size_max.x - size_min.x),
                size_min.y + rng.next_f32() * (size_max.y - size_min.y),
                size_min.z + rng.next_f32() * (size_max.z - size_min.z),
            );
            nodes.add(
                format!("Cube{i}"),
                Matrix::t_s([x, 0.01 + size.y * 0.5, y], size.into()),
                Some(&Mesh::cube()),
                Some(cube_mat),
                false,
            );
        }
        model
    }

    /// Quantizes the light position to the shadow map texel grid to reduce
    /// shimmering when the camera or objects move. This version quantizes
    /// the light position in world space based on texel size.
    fn quantize_light_pos(pos: Vec3, light_orientation: Quat, texel_size: f32) -> Vec3 {
        // Get the basis vectors from the light orientation
        let right = light_orientation * Vec3::X;
        let up = light_orientation * Vec3::Y;

        // Project the position onto the light's right and up axes
        let right_coord = Vec3::dot(pos, right);
        let up_coord = Vec3::dot(pos, up);
        let forward_coord = Vec3::dot(pos, light_orientation * Vec3::Z);

        // Quantize the coordinates to texel boundaries
        let quantized_right = (right_coord / texel_size).round() * texel_size;
        let quantized_up = (up_coord / texel_size).round() * texel_size;

        // Reconstruct the quantized position
        right * quantized_right + up * quantized_up + (light_orientation * Vec3::Z) * forward_coord
    }

    // Shutdown handling (called twice: triggering true then false)
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            self.shutdown_completed = true; // mark completed immediately
            true
        } else {
            self.shutdown_completed
        }
    }
}
