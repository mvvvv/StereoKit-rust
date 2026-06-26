// filepath: examples/demos/layers1.rs
use openxr_sys::SwapchainUsageFlags;
use std::rc::Rc;
use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Bounds, Matrix, Pose, Quat, Rect, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    render::{RenderBuilder, RenderClear, RenderList, Renderer},
    sprite::Sprite,
    system::{Backend, BackendXRType, Pivot, Text, TextBuilder, TextFit, TextStyle},
    tex::{Tex, TexFormat},
    tools::xr_comp_layers::{SwapchainSk, XrCompLayers},
    ui::Ui,
    util::{
        Color128, Time,
        named_colors::{self, RED},
    },
};

/// Composition Layers demo
///
/// OpenXR allows submitting extra quad or video layers
///
///  This is a rust copycat of <https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Demos/DemoLayers.cs>
#[derive(IStepper)]
pub struct Layers1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    priority: i32,
    shutdown_completed: bool,

    material: Material,
    window_pose: Pose,
    projection: Matrix,

    quad_sort_order: f32,
    quad_pose: Pose,
    quad_swapchain_sk: Option<SwapchainSk>,
    quad_render_list: RenderList,

    cylinder_sort_order: f32,
    cylinder_pose: Pose,
    cylinder_swapchain_sk: Option<SwapchainSk>,
    cylinder_render_list: RenderList,
    cylinder_radius: f32,
    cylinder_angle: f32,
    cylinder_aspect: f32,

    pub transparent_screen: Mesh,
    pub transparent_material: Material,
    pub transparent_transform: Matrix,
    pub transform: Matrix,
    pub text: String,
    text_style: TextStyle,
}

unsafe impl Send for Layers1 {}

impl Default for Layers1 {
    fn default() -> Self {
        let content_pose = Pose::new(Vec3::ZERO, None);
        let window_pose = content_pose * Matrix::t_r([0.5, 1.0, -0.6], [0.0, 180.0, 0.0]);
        let quad_pose = content_pose * Matrix::t([-0.2, 1.5, -1.0]);
        let cylinder_pose = content_pose * Matrix::t([-0.3, 1.5, 0.0]);
        let mut material = Material::ui_box().copy();
        material.color_tint(named_colors::GOLD).border_size(0.005);

        let transparent_material = Material::unlit();
        let transparent_transform =
            Matrix::t_r_s((Vec3::NEG_Z * 2.5) + Vec3::Y * 1.7, [90.0, 0.0, 90.0], Vec3::ONE * 1.5);
        Self {
            id: "Layers1".into(),
            sk_info: None,
            priority: 0,
            shutdown_completed: false,

            material,
            window_pose,
            quad_sort_order: 0.0,
            quad_pose,
            quad_swapchain_sk: None,
            quad_render_list: RenderList::new(),
            projection: Matrix::orthographic(0.2, 0.2, 0.01, 50.0),

            cylinder_sort_order: -1.0,
            cylinder_pose,
            cylinder_swapchain_sk: None,
            cylinder_render_list: RenderList::new(),
            cylinder_radius: 2.0,
            cylinder_angle: std::f32::consts::FRAC_PI_2 / 2.0,
            cylinder_aspect: 1.7777,

            transparent_screen: Mesh::sphere(),
            transparent_material,
            transparent_transform,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::Y_180),
            text: "Layers1\n\n\n".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl Layers1 {
    fn start(&mut self) -> bool {
        // Wrap the swapchain
        if Backend::xr_type() == BackendXRType::OpenXR {
            if let Some(comp_layer) = XrCompLayers::new() {
                if let Some(handle) = comp_layer.try_make_swapchain(
                    512,
                    512,
                    TexFormat::Rgba32Srgb,
                    SwapchainUsageFlags::COLOR_ATTACHMENT,
                    false,
                ) {
                    self.quad_swapchain_sk =
                        SwapchainSk::wrap(handle, TexFormat::Rgba32Srgb, 512, 512, Some(comp_layer));
                } else {
                    Log::warn("Failed to create XR swapchain");
                    return false;
                }
            } else {
                Log::warn("XrCompLayers is not available, cannot start Layers1 demo");
                return false;
            }
            // prepare the quad scene: spinning sphere
            let mut mat = Material::default().copy();
            mat.id("quadmat");
            if let Ok(floor) = Tex::from_file("textures/parquet2/parquet2.ktx2", true, None) {
                mat.diffuse_tex(&floor);
            }
            self.quad_render_list
                .add_mesh(Mesh::sphere(), mat, Matrix::s(0.1 * Vec3::ONE), named_colors::WHITE, None);

            // Create a second swapchain for the cylinder layer
            if let Some(comp_layer) = XrCompLayers::new() {
                if let Some(handle) = comp_layer.try_make_swapchain(
                    512,
                    256,
                    TexFormat::Rgba32Srgb,
                    SwapchainUsageFlags::COLOR_ATTACHMENT,
                    false,
                ) {
                    self.cylinder_swapchain_sk =
                        SwapchainSk::wrap(handle, TexFormat::Rgba32Srgb, 512, 256, Some(comp_layer));
                    // prepare the cylinder scene: rotating cube
                    let mut cyl_mat = Material::default().copy();
                    cyl_mat.id("cylmat");
                    self.cylinder_render_list.add_mesh(
                        Mesh::cube(),
                        cyl_mat,
                        Matrix::s(0.1 * Vec3::ONE),
                        named_colors::ORANGE,
                        None,
                    );
                } else {
                    Log::warn("Failed to create cylinder XR swapchain");
                }
            }
            // Ready to go we can change the sky.
            Renderer::enable_sky(false);
            Renderer::clear_color(Color128::rgba(0.1, 0.4, 0.9, 0.0));
            true
        } else {
            Log::warn("OpenXR backend is not available, cannot start Layers1 demo");
            false
        }
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {
        // no events
    }

    fn draw(&mut self, _token: &MainThreadToken) {
        const SIZE: f32 = 0.3;
        // interactive handle
        Ui::handle("QuadLayer", &mut self.quad_pose, Bounds::new([0.0, 0.0, 0.0], [SIZE, SIZE, 0.04])).grab();
        Mesh::cube().draw(&self.material, self.quad_pose.to_matrix(Some(Vec3::new(SIZE, SIZE, 0.04))), None, None);

        if let Some(sc) = &mut self.quad_swapchain_sk {
            let old_color = Renderer::get_clear_color();
            Renderer::clear_color(named_colors::SKY_BLUE);
            if let Err(e) = sc.acquire_image(None) {
                Log::warn(format!("Failed to acquire image from swapchain: {e}"));
                Log::warn("Skipping rendering for now...");
                self.quad_swapchain_sk = None;
            } else {
                let render_tex = sc.get_render_target().expect("SwapchainSk should have a render target");
                let quad_render = RenderBuilder::new()
                    .camera(Matrix::look_at(Vec3::angle_xz(Time::get_totalf() * 90.0, 0.0), Vec3::ZERO, None))
                    .projection(self.projection)
                    .clear(RenderClear::Color)
                    .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
                quad_render.draw_now(&self.quad_render_list, render_tex, Color128::new(0.4, 0.3, 0.2, 1.0));

                let sprite = Sprite::from_tex(render_tex, None, None).unwrap();

                sprite.draw(self.transform, Pivot::Center, None, None);

                assert_eq!(render_tex.get_width(), Some(512));

                if let Err(e) = sc.release_image() {
                    Log::warn(format!("Failed to release image from swapchain: {e}"));
                    Log::warn("Skipping rendering for now...");
                    self.quad_swapchain_sk = None;
                    return;
                }

                Renderer::clear_color(old_color);
                XrCompLayers::submit_quad_layer(
                    self.quad_pose,
                    Vec2::new(SIZE, SIZE),
                    sc.handle,
                    Rect::new(0.0, 0.0, sc.width as f32, sc.height as f32),
                    0,
                    self.quad_sort_order as i32,
                    None,
                    None,
                );
            }
        } else {
            TextBuilder::new("Requires an OpenXR runtime!")
                .transform(self.quad_pose)
                .size(Vec2::new(SIZE, SIZE))
                .fit(TextFit::Wrap)
                .add();
        }

        // --- Cylinder layer ---
        if let Some(cyl_sc) = &mut self.cylinder_swapchain_sk {
            if let Err(e) = cyl_sc.acquire_image(None) {
                Log::warn(format!("Failed to acquire cylinder image: {e}"));
                self.cylinder_swapchain_sk = None;
            } else {
                let render_tex = cyl_sc.get_render_target().expect("cylinder swapchain should have a render target");
                let cyl_render = RenderBuilder::new()
                    .camera(Matrix::look_at(Vec3::angle_xz(Time::get_totalf() * -60.0, 1.0) * 30.15, Vec3::ZERO, None))
                    .projection(self.projection)
                    .clear(RenderClear::Color)
                    .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
                cyl_render.draw_now(&self.cylinder_render_list, render_tex, Color128::new(0.1, 0.2, 0.4, 1.0));
                if let Err(e) = cyl_sc.release_image() {
                    Log::warn(format!("Failed to release cylinder image: {e}"));
                    self.cylinder_swapchain_sk = None;
                } else {
                    XrCompLayers::submit_cylinder_layer(
                        self.cylinder_pose,
                        self.cylinder_radius,
                        self.cylinder_angle,
                        self.cylinder_aspect,
                        cyl_sc.handle,
                        Rect::new(0.0, 0.0, cyl_sc.width as f32, cyl_sc.height as f32),
                        0,
                        self.cylinder_sort_order as i32,
                        None,
                        None,
                    );
                    self.transparent_screen.draw(
                        &self.transparent_material,
                        self.transparent_transform,
                        Some(Color128::BLACK_TRANSPARENT),
                        None,
                    );
                }
            }
        }

        // UI window
        Ui::window("Composition Layers").pose(&mut self.window_pose).size(Vec2::new(0.2, 0.0)).begin();
        Ui::label(format!("Sort Order {}", self.quad_sort_order as i32)).size(Vec2::new(0.1, 0.0)).draw();
        Ui::same_line();
        Ui::hslider("Sort Order", &mut self.quad_sort_order, -1.0, 1.0).step(1.0).interact();

        Ui::hseparator();
        Ui::label("Cylinder Layer").draw();
        Ui::label(format!("Sort Order {}", self.cylinder_sort_order as i32))
            .size(Vec2::new(0.1, 0.0))
            .draw();
        Ui::same_line();
        Ui::hslider("Cyl Sort Order", &mut self.cylinder_sort_order, -1.0, 1.0).step(1.0).interact();
        Ui::label(format!("Radius {:.2}m", self.cylinder_radius)).size(Vec2::new(0.1, 0.0)).draw();
        Ui::same_line();
        Ui::hslider("Cyl Radius", &mut self.cylinder_radius, 0.2, 3.0).interact();
        Ui::label(format!("Arc {:.0}°", self.cylinder_angle.to_degrees())).size(Vec2::new(0.1, 0.0)).draw();
        Ui::same_line();
        Ui::hslider("Cyl Angle", &mut self.cylinder_angle, 0.1, std::f32::consts::TAU).interact();
        Ui::label(format!("Aspect {:.2}", self.cylinder_aspect)).size(Vec2::new(0.1, 0.0)).draw();
        Ui::same_line();
        Ui::hslider("Cyl Aspect", &mut self.cylinder_aspect, 0.5, 2.0).interact();

        #[cfg(target_os = "android")]
        {
            Ui::hseparator();
            if Ui::button("Get android surface").press() {
                if let Some(comp_layer) = XrCompLayers::new() {
                    if let Some((handle, jobject)) =
                        comp_layer.try_make_android_swapchain(512, 512, SwapchainUsageFlags::COLOR_ATTACHMENT, false)
                    {
                        Log::info(format!("Created Android XR swapchain: {:#?}", jobject));
                        comp_layer.destroy_android_swapchain(handle);
                    } else {
                        Log::warn("Failed to create Android XR swapchain");
                    }
                } else {
                    Log::warn("XrCompLayers is not available anymore ??!!??");
                }
            }
        }
        Ui::window_end();

        TextBuilder::new(&self.text).transform(self.transform).style(self.text_style).add();
    }

    // we have to activate the sky rendering.
    fn close(&mut self, triggering: bool) -> bool {
        if triggering {
            Renderer::enable_sky(true);
            self.shutdown_completed = true;
        }
        self.shutdown_completed
    }
}
