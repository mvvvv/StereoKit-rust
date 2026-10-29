use std::sync::Once;

use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Matrix, Pose, Quat, Vec2, Vec3},
    mesh::Mesh,
    model::Model,
    prelude::*,
    render::Renderer,
    system::{Backend, BackendGraphics, BackendVulkan, Text, TextStyle, World},
    ui::{Ui, UiMove, UiPad, UiSettings, UiWin},
    util::{Time, named_colors},
};

/// Number of subpass post-process shaders available in `assets/shaders/subpass/`.
const NUM_EFFECTS: usize = 4;

/// Indices into the effects arrays.
const DEPTH_AWARE: usize = 0;
const DEPTH_WAVE: usize = 1;
const NIGHT_VISION: usize = 2;
const DYNAMIC_CLOUD: usize = 3;

/// Effects whose shader samples the depth input attachment of the main pass.
/// On Vulkan that needs `VK_KHR_depth_stencil_resolve`; the remaining effect is
/// colour only, and runs on any device.
const DEPTH_EFFECTS: [usize; 3] = [DEPTH_AWARE, DEPTH_WAVE, DYNAMIC_CLOUD];

/// IStepper implementation for testing the subpass post-process shaders.
///
/// This demo lets you toggle each of the four subpass effects (depth vignette,
/// scan-wave, night-vision, dynamic cloud fog) on or off, and exposes their
/// shader parameters as live sliders so you can experiment with them.
///
/// The three depth effects sample the depth of the MSAA main pass and need
/// `VK_KHR_depth_stencil_resolve` on Vulkan, see
/// [`Subpass1::depth_effects_supported`]; night vision is colour only and always
/// works.
#[derive(IStepper)]
pub struct Subpass1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    shutdown_completed: bool,

    // --- UI state ---
    pub window_pose: Pose,
    /// Compact text style for this window's dense parameter rows. Built once in
    /// [`Subpass1::default`]: a style owns a Material, and rebuilding it every
    /// frame creates a new pipeline every frame.
    compact_style: TextStyle,
    /// Per-effect enabled flag.
    effects_enabled: [bool; NUM_EFFECTS],
    /// Whether this device can sample the depth of the MSAA pass, see
    /// [`Subpass1::depth_effects_supported`]. False disables [`DEPTH_EFFECTS`].
    depth_effects_ok: bool,
    /// Per-effect tunable float parameters (defaults from the shader metadata).
    depth_aware_power: f32,
    depth_aware_smooth: f32,
    depth_wave_range: f32,
    depth_wave_distance: f32,
    depth_wave_width: f32,
    depth_wave_color: Vec3,
    night_vision_color: Vec3,
    night_vision_grain: f32,
    cloud_base_density: f32,
    cloud_noise_scale: f32,
    cloud_speed: f32,
    cloud_fog_color: Vec3,

    // --- Scene ---
    material_pbr: Material,
    material_red: Material,
    mesh_cube: Mesh,
    mesh_sphere: Mesh,
    mesh_round_cube: Mesh,
    model: Model,
    transform_model: Matrix,
    transform_cube: Matrix,
    transform_sphere: Matrix,
    transform_round_cube: Matrix,

    // --- Post-process materials (one per effect) ---
    effects: [Material; NUM_EFFECTS],

    fps: f64,
}

unsafe impl Send for Subpass1 {}

impl Default for Subpass1 {
    fn default() -> Self {
        // Only the colour effect can run without depth resolve, see `DEPTH_EFFECTS`.
        let depth_effects_ok = Self::depth_effects_supported();

        // ---- Post-process materials from the subpass shaders ----
        let depth_aware = Material::from_file("shaders/subpass/depth_aware.hlsl.sks", None).unwrap_or_default();
        let depth_wave = Material::from_file("shaders/subpass/depth_wave.hlsl.sks", None).unwrap_or_default();
        let night_vision = Material::from_file("shaders/subpass/night_vision.hlsl.sks", None).unwrap_or_default();
        let dynamic_cloud = Material::from_file("shaders/subpass/dynamic_cloud.hlsl.sks", None).unwrap_or_default();

        // ---- Scene materials / meshes / model ----
        let material_pbr = Material::pbr();
        let mut material_red = Material::pbr().copy();
        material_red.color_tint(stereokit_rust::util::named_colors::RED);

        let mesh_cube = Mesh::generate_cube(Vec3::ONE * 0.5, None);
        let mesh_sphere = Mesh::generate_sphere(0.5, None);
        let mesh_round_cube = Mesh::generate_rounded_cube(Vec3::ONE * 0.5, 0.1, None);
        let model = Model::from_file("plane.glb", None, None, None).unwrap_or_default();

        let transform_model = Matrix::t_r(Vec3::new(5.0, 11.0, -15.0), [45.0, 45.0, 45.0]);
        let transform_cube = Matrix::t_r(Vec3::new(-0.6, 1.0, -2.6), [45.0, 45.0, 0.0]);
        let transform_sphere = Matrix::t(Vec3::new(0.9, 2.8, -1.6));
        let transform_round_cube = Matrix::t_r(Vec3::new(0.6, 1.0, -1.6), [0.0, 45.0, 45.0]);

        Self {
            id: "Subpass1".to_string(),
            sk_info: None,
            shutdown_completed: false,

            window_pose: Pose::new(Vec3::new(0.4, 1.9, -0.5), Some(Quat::from_angles(0.0, 180.0, 0.0))),
            // Reduce line height by 20% for a more compact UI.
            compact_style: Text::make_style(
                Font::default(),
                Ui::get_text_style().get_layout_height() * 0.8,
                named_colors::WHITE,
            ),
            // Start with an effect on so the user immediately sees something: the
            // scan wave when the depth is readable, night vision when it isn't.
            effects_enabled: [false, depth_effects_ok, !depth_effects_ok, false],
            depth_effects_ok,
            depth_aware_power: 2.0,
            depth_aware_smooth: 0.5,
            depth_wave_range: 25.0,
            depth_wave_distance: 2.5,
            depth_wave_width: 0.15,
            depth_wave_color: Vec3::new(0.0, 0.8, 1.0),
            night_vision_color: Vec3::new(0.1, 1.0, 0.2),
            night_vision_grain: 1.0,
            cloud_base_density: 0.2,
            cloud_noise_scale: 0.5,
            cloud_speed: 0.2,
            cloud_fog_color: Vec3::new(0.5, 0.55, 0.6),

            material_pbr,
            material_red,
            mesh_cube,
            mesh_sphere,
            mesh_round_cube,
            model,
            transform_model,
            transform_cube,
            transform_sphere,
            transform_round_cube,

            effects: [depth_aware, depth_wave, night_vision, dynamic_cloud],

            fps: 0.0,
        }
    }
}

impl Subpass1 {
    /// Whether `effects_index` samples the depth of the main pass, and therefore
    /// can't run without [`Subpass1::depth_effects_supported`].
    fn effect_needs_depth(effects_index: usize) -> bool {
        DEPTH_EFFECTS.contains(&effects_index)
    }

    /// Whether the depth input attachment of the main pass can be sampled here.
    ///
    /// Reads the MSAA depth inside a subpass, which needs
    /// `VK_KHR_depth_stencil_resolve` on Vulkan. Without the extension
    /// sk_renderer drops such an effect and logs a critical error, so the demo
    /// keeps it out of the post-process chain instead.
    pub fn depth_effects_supported() -> bool {
        let supported = Backend::graphics() != BackendGraphics::Vulkan
            || BackendVulkan::ext_enabled("VK_KHR_depth_stencil_resolve");
        if !supported {
            // Asked again by every `Default`, so only say it once.
            static WARNED: Once = Once::new();
            WARNED.call_once(|| {
                Log::diag(
                    "Subpass1: depth effects disabled, this Vulkan device lacks VK_KHR_depth_stencil_resolve. The \
                     colour effects still work.",
                )
            });
        }
        supported
    }

    /// Called from IStepper::initialize. Here you can abort initialization by returning false.
    fn start(&mut self) -> bool {
        World::occlusion(stereokit_rust::system::OcclusionCaps::Mesh);
        true
    }

    /// Called from IStepper::step, here you can check the event report.
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Push the current per-effect parameters into the materials.
    fn sync_params(&mut self) {
        self.effects[DEPTH_AWARE]
            .get_all_param_info()
            .set_float("vignette_power", self.depth_aware_power)
            .set_float("vignette_smooth", self.depth_aware_smooth);
        self.effects[DEPTH_WAVE]
            .get_all_param_info()
            .set_float("scan_distance", self.depth_wave_distance)
            .set_float("scan_width", self.depth_wave_width)
            .set_vector3("scan_color", self.depth_wave_color);
        self.effects[NIGHT_VISION]
            .get_all_param_info()
            .set_vector3("nv_color", self.night_vision_color)
            .set_float("grain_size", self.night_vision_grain);
        self.effects[DYNAMIC_CLOUD]
            .get_all_param_info()
            .set_float("base_density", self.cloud_base_density)
            .set_float("noise_scale", self.cloud_noise_scale)
            .set_float("time", self.cloud_speed)
            .set_vector3("fog_color", self.cloud_fog_color);
    }

    /// Called from IStepper::step after check_event, here you can draw your UI.
    fn draw(&mut self, _token: &MainThreadToken) {
        // --- Draw a small scene to exercise the effects ---
        self.model.draw(self.transform_model, Some(named_colors::DARK_RED.into()), None);
        self.mesh_cube
            .draw(&self.material_red, self.transform_cube, Some(named_colors::ROYAL_BLUE.into()), None);
        self.mesh_sphere
            .draw(&self.material_pbr, self.transform_sphere, Some(named_colors::DARK_BLUE.into()), None);
        self.mesh_round_cube.draw(
            &self.material_pbr,
            self.transform_round_cube,
            Some(named_colors::SEA_GREEN.into()),
            None,
        );

        // Keep the depth effects out of the chain when the pass's depth can't be
        // read, whatever the UI state says.
        if !self.depth_effects_ok {
            for index in DEPTH_EFFECTS {
                self.effects_enabled[index] = false;
            }
        }

        // Animate the wave distance for a living scanline feel when enabled.
        if self.effects_enabled[DEPTH_WAVE] {
            self.depth_wave_distance = 0.5 + (Time::get_totalf().sin() * 0.5 + 0.5) * self.depth_wave_range;
        }

        // --- Push params and build the post-process chain from enabled effects ---
        self.sync_params();
        let mut chain: Vec<&Material> = Vec::with_capacity(NUM_EFFECTS);
        for (i, &enabled) in self.effects_enabled.iter().enumerate() {
            if enabled {
                chain.push(&self.effects[i]);
            }
        }
        Renderer::set_post_process(chain);

        // --- UI ---
        Ui::window("Subpass FX")
            .pose(&mut self.window_pose)
            .size(Vec2::new(0.40, 0.0))
            .move_type(UiMove::FaceUser)
            .window_type(UiWin::Head)
            .begin();

        // Reduce line height by 20% for a more compact UI.
        Ui::push_text_style(self.compact_style);

        // Reduce all UI spacing (padding, gutter, margin) by 50% as well.
        let old_settings = Ui::get_settings();
        Ui::settings(UiSettings {
            padding: old_settings.padding * 0.5,
            gutter: old_settings.gutter * 0.5,
            margin: old_settings.margin * 0.5,
            ..old_settings
        });

        Ui::label("Toggle effects and tune parameters:").use_padding(true).draw();
        if !self.depth_effects_ok {
            Ui::label("Depth effects need VK_KHR_depth_stencil_resolve here.").use_padding(true).draw();
        }

        Ui::hseparator();
        Ui::panel_begin(Some(UiPad::Outside));
        // Depth aware
        Ui::push_enabled(self.depth_effects_ok, None);
        if Ui::toggle("Depth Vignette", &mut self.effects_enabled[DEPTH_AWARE])
            .size([0.15, 0.0])
            .interact()
            .is_some()
        {
            self.depth_aware_power = 2.0;
            self.depth_aware_smooth = 0.5;
        }
        Ui::pop_enabled();
        Ui::push_enabled(self.depth_effects_ok && self.effects_enabled[DEPTH_AWARE], None);
        Ui::same_line();
        Ui::label(format!("power: {:.2}", self.depth_aware_power)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("dap", &mut self.depth_aware_power, 0.1, 8.0).step(0.1).interact();
        Ui::next_line();
        Ui::label(format!("smooth: {:.2}", self.depth_aware_smooth)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("dasm", &mut self.depth_aware_smooth, 0.01, 1.0).step(0.01).interact();
        Ui::pop_enabled();
        Ui::panel_end();

        Ui::hseparator();
        Ui::panel_begin(Some(UiPad::Outside));
        // Depth wave
        Ui::push_enabled(self.depth_effects_ok, None);
        if Ui::toggle("Scan Wave", &mut self.effects_enabled[DEPTH_WAVE])
            .size([0.15, 0.0])
            .interact()
            .is_some()
        {
            self.depth_wave_range = 25.0;
            self.depth_wave_width = 0.15;
        }
        Ui::pop_enabled();
        Ui::push_enabled(self.depth_effects_ok && self.effects_enabled[DEPTH_WAVE], None);
        Ui::same_line();
        Ui::label("color r/g/b:").use_padding(true).draw();
        Ui::same_line();
        Ui::vslider("dwr", &mut self.depth_wave_color.x, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("dwg", &mut self.depth_wave_color.y, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("dwb", &mut self.depth_wave_color.z, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::next_line();
        Ui::label(format!("dist (auto): {:.2}", self.depth_wave_distance)).use_padding(true).draw();
        Ui::next_line();
        Ui::label(format!("range: {:.1}", self.depth_wave_range)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("wave_dist", &mut self.depth_wave_range, 0.0, 30.0).step(0.5).interact();
        Ui::next_line();
        Ui::label(format!("width: {:.2}", self.depth_wave_width)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("dww", &mut self.depth_wave_width, 0.01, 1.0).step(0.01).interact();
        Ui::pop_enabled();
        Ui::panel_end();

        Ui::hseparator();
        Ui::panel_begin(Some(UiPad::Outside));
        // Night vision
        if Ui::toggle("Night Vision", &mut self.effects_enabled[NIGHT_VISION])
            .size([0.15, 0.0])
            .interact()
            .is_some()
        {
            self.night_vision_color = Vec3::new(0.1, 1.0, 0.2);
            self.night_vision_grain = 1.0;
        }
        Ui::push_enabled(self.effects_enabled[NIGHT_VISION], None);
        Ui::same_line();
        Ui::label("color r/g/b:").use_padding(true).draw();
        Ui::same_line();
        Ui::vslider("nvr", &mut self.night_vision_color.x, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("nvg", &mut self.night_vision_color.y, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("nvb", &mut self.night_vision_color.z, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::next_line();
        Ui::label(format!("grain: {:.2}", self.night_vision_grain)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("dasm", &mut self.night_vision_grain, 0.1, 10.0).step(0.1).interact();
        Ui::pop_enabled();
        Ui::panel_end();

        Ui::hseparator();
        Ui::panel_begin(Some(UiPad::Outside));
        // Dynamic cloud fog
        Ui::push_enabled(self.depth_effects_ok, None);
        if Ui::toggle("Dynamic Cloud", &mut self.effects_enabled[DYNAMIC_CLOUD])
            .size([0.15, 0.0])
            .interact()
            .is_some()
        {
            self.cloud_base_density = 0.2;
            self.cloud_noise_scale = 0.15;
            self.cloud_speed = 8.5;
            self.cloud_fog_color = Vec3::new(0.5, 0.55, 0.6);
        }
        Ui::pop_enabled();
        Ui::push_enabled(self.depth_effects_ok && self.effects_enabled[DYNAMIC_CLOUD], None);

        Ui::same_line();
        Ui::label("color r/g/b:").use_padding(true).draw();
        Ui::same_line();
        Ui::vslider("fcr", &mut self.cloud_fog_color.x, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("fcg", &mut self.cloud_fog_color.y, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::same_line();
        Ui::vslider("fcb", &mut self.cloud_fog_color.z, 0.0, 1.0).step(0.05).space(0.06).interact();
        Ui::label(format!("density: {:.2}", self.cloud_base_density)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("cbd", &mut self.cloud_base_density, 0.05, 1.0).step(0.05).interact();
        Ui::next_line();
        Ui::label(format!("noise scale: {:.2}", self.cloud_noise_scale)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("cns", &mut self.cloud_noise_scale, 0.05, 1.0).step(0.05).interact();
        Ui::next_line();
        Ui::label(format!("speed: {:.2}", self.cloud_speed)).use_padding(true).draw();
        Ui::same_line();
        Ui::hslider("csp", &mut self.cloud_speed, 0.1, 10.0).step(0.05).interact();
        Ui::pop_enabled();
        Ui::panel_end();

        Ui::hseparator();
        if Ui::button("All On").press() {
            // Only the effects this device can actually run.
            self.effects_enabled =
                std::array::from_fn(|index| !Self::effect_needs_depth(index) || self.depth_effects_ok);
        }
        Ui::same_line();
        if Ui::button("All Off").press() {
            self.effects_enabled = [false; NUM_EFFECTS];
        }

        Ui::next_line();
        self.fps = ((1.0 / Time::get_step()) + self.fps) / 2.0;
        Ui::label(format!(
            "Active: {} / {}  | FPS: {:.0}",
            self.effects_enabled.iter().filter(|e| **e).count(),
            NUM_EFFECTS,
            self.fps
        ))
        .use_padding(true)
        .draw();

        Ui::settings(old_settings);
        Ui::pop_text_style();

        Ui::window_end();
    }

    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            // Clear the post-process chain so it doesn't leak into other demos.
            Renderer::set_post_process(vec![]);
            World::occlusion(stereokit_rust::system::OcclusionCaps::None);
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }
}
