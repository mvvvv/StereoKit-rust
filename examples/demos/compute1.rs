// Rust port of https://github.com/StereoKit/StereoKit/commit/54618b4393b96730906cf8a156c42d141897c15e
// DemoCompute.cs — Gray-Scott reaction-diffusion simulation running entirely on the GPU via
// compute shaders, displayed on a quad.
//
// The compute shader (assets/shaders/compute_reaction.hlsl.sks) must be compiled with
// `cargo compile_sks` before running.

use stereokit_rust::{
    compute::{Compute, ComputeBuffer, ComputeBufferType},
    material::{Cull, Material},
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    tex::{Tex, TexFormat, TexType},
    ui::Ui,
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

/// IStepper implementation for Compute1 — reaction-diffusion demo.
#[derive(IStepper)]
pub struct Compute1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    // Ping-pong compute shaders: ping reads bufferA, writes bufferB; pong the reverse.
    compute_ping: Compute,
    compute_pong: Compute,
    buffer_a: ComputeBuffer,
    buffer_b: ComputeBuffer,
    // Kept alive as owner of the GPU texture referenced by display_mat.
    #[allow(dead_code)]
    output: Tex,

    display_mesh: Mesh,
    display_mat: Material,
    pub display_transform: Matrix,
    pub window_pose: Pose,

    // Gray-Scott simulation parameters (tweakable via UI)
    step: u32,
    feed: f32,
    kill: f32,
    diff_a: f32,
    diff_b: f32,
    timestep: f32,
}

unsafe impl Send for Compute1 {}

impl Default for Compute1 {
    fn default() -> Self {
        let shader_path = "shaders/compute_reaction.hlsl.sks";

        // Two identical compute dispatches for ping-pong buffering
        let compute_ping = Compute::from_file(shader_path)
            .expect("compute_reaction shader should be compiled — run `cargo compile_sks`");
        let compute_pong = Compute::from_file(shader_path)
            .expect("compute_reaction shader should be compiled — run `cargo compile_sks`");

        // Allocate two ReadWrite buffers, one per ping-pong half
        let cell_count = (SIM_SIZE * SIM_SIZE) as i32;
        let mut buffer_a =
            ComputeBuffer::new(ComputeBufferType::ReadWrite, cell_count, 8).expect("Failed to create bufferA");
        let mut buffer_b =
            ComputeBuffer::new(ComputeBufferType::ReadWrite, cell_count, 8).expect("Failed to create bufferB");

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

        // Output texture: Rgba128 storage image written by the compute shader
        let mut output = Tex::new(TexType::ImageNomips | TexType::Compute, TexFormat::Rgba128, None);
        output.set_size(SIM_SIZE as usize, SIM_SIZE as usize, None, None);

        // Ping: reads bufferA, writes bufferB, writes result texture
        let mut cp_param = compute_ping.get_all_param_info();
        if !cp_param.set_buffer("input", &buffer_a) {
            Log::warn("Failed to set bufferA on compute_ping — check shader parameter names?");
        }
        if !cp_param.set_buffer("output", &buffer_b) {
            Log::warn("Failed to set bufferB on compute_ping — check shader parameter names?");
        }
        if !cp_param.set_texture("out_tex", &output) {
            Log::warn("Failed to set output texture on compute_ping — check shader parameter names?");
        }

        // Pong: reads bufferB, writes bufferA, writes result texture
        let mut cp_param = compute_pong.get_all_param_info();
        if !cp_param.set_buffer("input", &buffer_b) {
            Log::warn("Failed to set bufferB on compute_pong — check shader parameter names?");
        }
        if !cp_param.set_buffer("output", &buffer_a) {
            Log::warn("Failed to set bufferA on compute_pong — check shader parameter names?");
        }
        if !cp_param.set_texture("out_tex", &output) {
            Log::warn("Failed to set output texture on compute_pong — check shader parameter names?");
        }

        // Display quad
        let display_mesh = Mesh::generate_plane(Vec2::ONE * 0.4, Vec3::NEG_Z, Vec3::Y, None, true);
        let mut display_mat = Material::unlit().copy();
        display_mat.diffuse_tex(&output).face_cull(Cull::None);

        let feed = 0.055;
        let kill = 0.062;
        let diff_a = 1.0;
        let diff_b = 0.5;
        let timestep = 1.0;

        let mut this = Self {
            id: "Compute1".to_string(),
            sk_info: None,

            compute_ping,
            compute_pong,
            buffer_a,
            buffer_b,
            output,
            display_mesh,
            display_mat,
            display_transform: Matrix::t_r(Vec3::new(0.0, 1.5, -0.6), Quat::from_angles(0.0, 180.0, 0.0)),
            window_pose: Pose::new(Vec3::new(0.4, 1.5, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            step: 0,
            feed,
            kill,
            diff_a,
            diff_b,
            timestep,
        };
        this.set_params();
        this
    }
}

impl Compute1 {
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
    fn reset_sim(&mut self) {
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

    /// Called from IStepper::initialize — return false to abort.
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step to handle events.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step — advance simulation and render.
    fn draw(&mut self, token: &MainThreadToken) {
        // Advance the simulation several steps per frame
        const STEPS_PER_FRAME: u32 = 10;
        for _ in 0..STEPS_PER_FRAME {
            if self.step.is_multiple_of(2) {
                self.compute_ping.dispatch(GROUPS, GROUPS, 1);
            } else {
                self.compute_pong.dispatch(GROUPS, GROUPS, 1);
            }
            self.step += 1;
        }

        // Draw the output texture on a quad
        self.display_mesh.draw(token, &self.display_mat, self.display_transform, None, None);

        // UI panel for tweaking parameters
        Ui::window_begin("Compute Shader", &mut self.window_pose, Some(Vec2::new(0.28, 0.0)), None, None);
        Ui::label(format!("Step: {}", self.step), None, true);

        if Ui::hslider("feed", &mut self.feed, 0.01, 0.1, Some(0.001), None, None, None).is_some()
            | Ui::hslider("kill", &mut self.kill, 0.01, 0.1, Some(0.001), None, None, None).is_some()
            | Ui::hslider("diffuseA", &mut self.diff_a, 0.1, 2.0, Some(0.01), None, None, None).is_some()
            | Ui::hslider("diffuseB", &mut self.diff_b, 0.1, 1.0, Some(0.01), None, None, None).is_some()
            | Ui::hslider("timestep", &mut self.timestep, 0.1, 2.0, Some(0.01), None, None, None).is_some()
        {
            self.set_params();
        }
        if Ui::button("Reset", None) {
            self.reset_sim();
        }
        Ui::window_end();
    }
}
