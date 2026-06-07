// Rust port of https://github.com/StereoKit/StereoKit/commit/54618b4393b96730906cf8a156c42d141897c15e
// DemoCompute.cs — Gray-Scott reaction-diffusion simulation running entirely on the GPU via
// compute shaders, displayed on a quad.
//
// The compute shader (assets/shaders/compute_reaction.hlsl.sks) must be compiled with
// `cargo compile_sks` before running.

use stereokit_rust::{
    compute::{Compute, ComputeBuffer, ComputeBufferType},
    font::Font,
    maths::{Matrix, Pose, Quat, Vec2, Vec3, Vec4},
    prelude::*,
    sprite::{Sprite, SpriteType},
    system::{Text, TextStyle},
    tex::{Tex, TexFormat, TexType},
    ui::Ui,
    util::named_colors::RED,
};

/// Number of cells per side of the simulation grid.
const SIM_SIZE: u32 = 512;
/// Number of thread groups per side (shader uses [numthreads(8,8,1)]).
const GROUPS: u32 = SIM_SIZE / 8;

/// A single cell in the Gray-Scott reaction-diffusion model.
/// Must match the HLSL `float2` layout exactly (two f32 values, no padding).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Cell {
    a: f32,
    b: f32,
}

/// Gray-Scott reaction-diffusion simulation running on the GPU via ping-pong compute buffers.
struct ComputeReaction {
    // Ping-pong compute shaders: ping reads buffer_a, writes buffer_b; pong the reverse.
    compute_ping: Compute,
    compute_pong: Compute,
    buffer_a: ComputeBuffer<Cell>,
    buffer_b: ComputeBuffer<Cell>,
    // Kept alive as owner of the GPU texture referenced by output_sprite.
    #[allow(dead_code)]
    output: Tex,
    /// Sprite wrapping the output texture — used by Ui::image.
    output_sprite: Sprite,

    pub window_pose: Pose,

    pub active: bool,

    // Gray-Scott simulation parameters (tweakable via UI)
    pub step: u32,
    pub feed: f32,
    pub kill: f32,
    pub diff_a: f32,
    pub diff_b: f32,
    pub timestep: f32,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

impl ComputeReaction {
    fn new() -> Result<Self, String> {
        let shader_path = "shaders/compute_reaction.hlsl.sks";

        // Two identical compute dispatches for ping-pong buffering
        let compute_ping = Compute::from_file(shader_path)
            .map_err(|e| format!("Failed to load shader '{shader_path}' — run `cargo compile_sks` ({e})"))?;
        let compute_pong = Compute::from_file(shader_path)
            .map_err(|e| format!("Failed to load shader '{shader_path}' — run `cargo compile_sks` ({e})"))?;

        // Allocate two ReadWrite buffers, one per ping-pong half
        let cell_count = (SIM_SIZE * SIM_SIZE) as i32;
        let mut buffer_a = ComputeBuffer::new(ComputeBufferType::ReadWrite, cell_count, 8)
            .map_err(|e| format!("Failed to create bufferA ({e})"))?;
        let mut buffer_b = ComputeBuffer::new(ComputeBufferType::ReadWrite, cell_count, 8)
            .map_err(|e| format!("Failed to create bufferB ({e})"))?;

        // Seed the simulation: fill with A=1,B=0 then place a small B=1 blob in the centre
        let mut data = vec![Cell { a: 1.0, b: 0.0 }; (SIM_SIZE * SIM_SIZE) as usize];
        let cx = (SIM_SIZE / 2) as i32;
        let cy = (SIM_SIZE / 2) as i32;
        let r = (SIM_SIZE / 16) as i32;
        for y in (cy - r)..(cy + r) {
            for x in (cx - r)..(cx + r) {
                data[(y * SIM_SIZE as i32 + x) as usize] = Cell { a: 0.0, b: 1.0 };
            }
        }
        buffer_a.set_data(&data);
        buffer_b.set_data(&data);

        // Output texture: Rgba32Linear (VK_FORMAT_R8G8B8A8_UNORM) — matches [[vk::image_format("rgba8")]] in the shader
        let mut output = Tex::new(TexType::ImageNomips | TexType::Compute, TexFormat::Rgba32Linear, None);
        output.set_size(SIM_SIZE as usize, SIM_SIZE as usize, None, None);

        // Ping: reads buffer_a, writes buffer_b, writes result texture
        let mut cp_param = compute_ping.get_all_param_info();
        if !cp_param.set_storage("input", &buffer_a) {
            Log::warn("Failed to set bufferA on compute_ping — check shader parameter names?");
        }
        if !cp_param.set_storage("output", &buffer_b) {
            Log::warn("Failed to set bufferB on compute_ping — check shader parameter names?");
        }
        if !cp_param.set_texture("out_tex", &output) {
            Log::warn("Failed to set output texture on compute_ping — check shader parameter names?");
        }

        // Pong: reads buffer_b, writes buffer_a, writes result texture
        let mut cp_param = compute_pong.get_all_param_info();
        if !cp_param.set_storage("input", &buffer_b) {
            Log::warn("Failed to set bufferB on compute_pong — check shader parameter names?");
        }
        if !cp_param.set_storage("output", &buffer_a) {
            Log::warn("Failed to set bufferA on compute_pong — check shader parameter names?");
        }
        if !cp_param.set_texture("out_tex", &output) {
            Log::warn("Failed to set output texture on compute_pong — check shader parameter names?");
        }

        let output_sprite = Sprite::from_tex(&output, Some(SpriteType::Single), None)
            .map_err(|e| format!("Failed to create sprite from output texture ({e})"))?;

        let mut this = Self {
            compute_ping,
            compute_pong,
            buffer_a,
            buffer_b,
            output,
            output_sprite,
            window_pose: Pose::new(Vec3::new(-0.2, 1.5, -0.6), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            active: true,
            step: 0,
            feed: 0.055,
            kill: 0.062,
            diff_a: 1.0,
            diff_b: 0.5,
            timestep: 1.0,

            text: "Compute1".to_string(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        };
        this.set_params();
        Ok(this)
    }
}

impl ComputeReaction {
    /// Push the current Gray-Scott parameters to both compute dispatches.
    fn set_params(&mut self) {
        for compute in [&mut self.compute_ping, &mut self.compute_pong] {
            compute
                .get_all_param_info()
                .set_float("feed", self.feed)
                .set_float("kill", self.kill)
                .set_float("diffuseA", self.diff_a)
                .set_float("diffuseB", self.diff_b)
                .set_float("timestep", self.timestep)
                .set_uint("size", SIM_SIZE);
        }
    }

    /// Re-seed the simulation with the initial conditions.
    fn reset(&mut self) {
        let mut data = vec![Cell { a: 1.0, b: 0.0 }; (SIM_SIZE * SIM_SIZE) as usize];
        let cx = (SIM_SIZE / 2) as i32;
        let cy = (SIM_SIZE / 2) as i32;
        let r = (SIM_SIZE / 16) as i32;
        for y in (cy - r)..(cy + r) {
            for x in (cx - r)..(cx + r) {
                data[(y * SIM_SIZE as i32 + x) as usize] = Cell { a: 0.0, b: 1.0 };
            }
        }
        self.buffer_a.set_data(&data);
        self.buffer_b.set_data(&data);
        self.step = 0;
    }

    /// Advance the simulation by one step (one ping-pong dispatch).
    fn advance(&mut self) {
        if self.step.is_multiple_of(2) {
            self.compute_ping.dispatch(GROUPS, GROUPS, 1);
        } else {
            self.compute_pong.dispatch(GROUPS, GROUPS, 1);
        }
        self.step += 1;
    }

    /// Advance the simulation and render the UI panel (image + controls).
    fn draw(&mut self, token: &MainThreadToken) {
        const STEPS_PER_FRAME: u32 = 10;
        if self.active {
            for _ in 0..STEPS_PER_FRAME {
                self.advance();
            }
        }

        Ui::window_begin("Compute Shader", &mut self.window_pose, Some(Vec2::new(0.28, 0.0)), None, None);
        Ui::toggle("Run the test", &mut self.active).interact();
        Ui::image(&self.output_sprite, Vec2::ONE * 0.26);
        Ui::label(format!("Step: {}", self.step)).use_padding(true).draw();

        let params_changed = {
            Ui::label("feed").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = Ui::hslider("feed", &mut self.feed, 0.01, 0.1, Some(0.001), None, None, None).is_some();
            Ui::label("kill").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("kill", &mut self.kill, 0.01, 0.1, Some(0.001), None, None, None).is_some();
            Ui::label("diffuseA").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("diffuseA", &mut self.diff_a, 0.1, 2.0, Some(0.01), None, None, None).is_some();
            Ui::label("diffuseB").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("diffuseB", &mut self.diff_b, 0.1, 1.0, Some(0.01), None, None, None).is_some();
            Ui::label("timestep").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            r | Ui::hslider("timestep", &mut self.timestep, 0.1, 2.0, Some(0.01), None, None, None).is_some()
        };
        if params_changed {
            self.set_params();
        }
        if Ui::button("Reset").press() {
            self.reset();
        }
        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}

// ─── ComputeTest ─────────────────────────────────────────────────────────────

/// Concentric-rings + spiral-arms pattern computed entirely on the GPU.
///
/// Shader: assets/shaders/compute_test.hlsl.sks
struct ComputeTest {
    compute: Compute,
    // Kept alive as owner of the GPU texture referenced by output_sprite.
    #[allow(dead_code)]
    output: Tex,
    /// Sprite wrapping the output texture — used by Ui::image.
    output_sprite: Sprite,

    pub window_pose: Pose,

    pub step: u32,
    pub ring_freq: f32,
    /// Stored as f32 for the slider; cast to i32 on upload.
    pub arm_count: f32,
    pub center_glow: bool,
    pub uv_x: f32,
    pub uv_y: f32,
    pub spiral_twist: f32,
    pub active: bool,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub color_speed: f32,
}

impl ComputeTest {
    fn new() -> Result<Self, String> {
        let shader_path = "shaders/compute_test.hlsl.sks";

        let compute = Compute::from_file(shader_path)
            .map_err(|e| format!("Failed to load shader '{shader_path}' — run `cargo compile_sks` ({e})"))?;

        // Output texture: Rgba128 (4×f32, VK_FORMAT_R32G32B32A32_SFLOAT) required by the compute shader
        let mut output = Tex::new(TexType::ImageNomips | TexType::Compute, TexFormat::Rgba128, None);
        output.set_size(SIM_SIZE as usize, SIM_SIZE as usize, None, None);

        if !compute.get_all_param_info().set_texture("out_tex", &output) {
            Log::warn("Failed to set out_tex on compute_test — check shader parameter names?");
        }

        let output_sprite = Sprite::from_tex(&output, Some(SpriteType::Single), None)
            .map_err(|e| format!("Failed to create sprite from output texture ({e})"))?;

        let mut this = Self {
            compute,
            output,
            output_sprite,
            window_pose: Pose::new(Vec3::new(0.2, 1.5, -0.6), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            active: true,
            step: 0,
            ring_freq: 1.0,
            arm_count: 5.0,
            center_glow: true,
            uv_x: 0.0,
            uv_y: 0.0,
            spiral_twist: 1.0,
            rotation: 0.0,
            rotation_speed: 0.003,
            color_speed: 0.003,
        };
        this.set_params();
        Ok(this)
    }

    /// Push the current parameters to the compute shader.
    fn set_params(&mut self) {
        self.compute
            .get_all_param_info()
            .set_float("ring_freq", self.ring_freq)
            .set_int("arm_count", self.arm_count as i32)
            .set_uint("tex_size", SIM_SIZE)
            .set_bool("center_glow", self.center_glow)
            .set_vector2("uv_offset", Vec2::new(self.uv_x, self.uv_y))
            .set_vector3("spiral_twist", Vec3::new(self.spiral_twist, 0.0, 0.0))
            .set_vector4("highlight", Vec4::new(1.0, 0.8, 0.2, 1.0))
            .set_vector4("base_color", Vec4::new(0.05, 0.0, 0.15, 1.0))
            .set_matrix("brightness", Matrix::IDENTITY)
            .set_float("rotation", self.rotation);
    }

    /// Dispatch one compute pass.
    fn advance(&mut self) {
        self.rotation += self.rotation_speed;
        let mut p = self.compute.get_all_param_info();
        p.set_float("rotation", self.rotation);
        if self.color_speed != 0.0 {
            let t = self.step as f32 * self.color_speed;
            // Hue-cycle highlight with three phase-shifted sinusoids
            let highlight =
                Vec4::new(t.sin() * 0.4 + 0.6, (t + 2.094).sin() * 0.4 + 0.6, (t + 4.189).sin() * 0.4 + 0.6, 1.0);
            // base_color: complementary phase (+ π), kept dark
            let base = Vec4::new(
                ((t + std::f32::consts::PI).sin() * 0.04 + 0.05).max(0.0),
                ((t + std::f32::consts::PI + 2.094).sin() * 0.04 + 0.04).max(0.0),
                ((t + std::f32::consts::PI + 4.189).sin() * 0.04 + 0.15).max(0.0),
                1.0,
            );
            // brightness: gentle pulse on the overall intensity
            let scale = (t * 2.0).sin() * 0.25 + 1.0;
            p.set_vector4("highlight", highlight)
                .set_vector4("base_color", base)
                .set_matrix("brightness", Matrix::s(Vec3::ONE * scale));
        }
        self.compute.dispatch(GROUPS, GROUPS, 1);
        self.step += 1;
    }

    /// Advance the simulation and render the UI panel (image + controls).
    fn draw(&mut self, _token: &MainThreadToken) {
        const STEPS_PER_FRAME: u32 = 10;
        if self.active {
            for _ in 0..STEPS_PER_FRAME {
                self.advance();
            }
        }

        Ui::window_begin("Compute Test", &mut self.window_pose, Some(Vec2::new(0.28, 0.0)), None, None);
        Ui::toggle("Run the test", &mut self.active).interact();
        Ui::image(&self.output_sprite, Vec2::ONE * 0.26);
        Ui::label(format!("Step: {}", self.step)).use_padding(true).draw();

        let params_changed = {
            Ui::label("ring_freq").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = Ui::hslider("ring_freq", &mut self.ring_freq, 0.1, 5.0, Some(0.1), None, None, None).is_some();
            Ui::label("arm_count").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("arm_count", &mut self.arm_count, 1.0, 12.0, Some(1.0), None, None, None).is_some();
            Ui::label("uv x").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("uv_x", &mut self.uv_x, -0.5, 0.5, Some(0.01), None, None, None).is_some();
            Ui::label("uv y").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("uv_y", &mut self.uv_y, -0.5, 0.5, Some(0.01), None, None, None).is_some();
            Ui::label("twist").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("spiral_twist", &mut self.spiral_twist, -3.0, 3.0, Some(0.1), None, None, None)
                .is_some();
            Ui::label("rot speed").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r =
                r | Ui::hslider("rotation_speed", &mut self.rotation_speed, -0.1, 0.1, Some(0.001), None, None, None)
                    .is_some();
            Ui::label("col speed").size(Vec2::new(0.07, 0.0)).draw();
            Ui::same_line();
            let r = r | Ui::hslider("color_speed", &mut self.color_speed, 0.0, 0.05, Some(0.001), None, None, None)
                .is_some();
            let prev_glow = self.center_glow;
            Ui::toggle("center_glow", &mut self.center_glow).interact();
            r | (self.center_glow != prev_glow)
        };
        if params_changed {
            self.set_params();
        }
        Ui::window_end();
    }
}

/// IStepper implementation for Compute1 — reaction-diffusion demo.
#[derive(IStepper)]
pub struct Compute1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    /// `Some` when initialisation succeeded, `None` on error.
    reaction: Option<ComputeReaction>,
    compute_test: Option<ComputeTest>,
}

unsafe impl Send for Compute1 {}

impl Default for Compute1 {
    fn default() -> Self {
        let reaction = match ComputeReaction::new() {
            Ok(r) => Some(r),
            Err(e) => {
                Log::err(format!("ComputeReaction init failed: {e}"));
                None
            }
        };

        let compute_test = match ComputeTest::new() {
            Ok(t) => Some(t),
            Err(e) => {
                Log::err(format!("ComputeTest init failed: {e}"));
                None
            }
        };

        Self { id: "Compute1".to_string(), sk_info: None, reaction, compute_test }
    }
}

impl Compute1 {
    /// Called from IStepper::initialize — return false to abort.
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step to handle events.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step — advance simulation and render.
    fn draw(&mut self, token: &MainThreadToken) {
        if let Some(ref mut reaction) = self.reaction {
            reaction.draw(token);
        }
        if let Some(ref mut test) = self.compute_test {
            test.draw(token);
        }
    }
}
