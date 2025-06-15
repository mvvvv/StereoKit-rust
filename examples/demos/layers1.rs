// filepath: examples/demos/layers1.rs
use openxr_sys::SwapchainUsageFlags;
use std::rc::Rc;
use stereokit_rust::font::Font;
use stereokit_rust::maths::{Bounds, Rect};
use stereokit_rust::render_list::RenderList;
use stereokit_rust::sprite::Sprite;
use stereokit_rust::system::{Pivot, RenderClear, TextFit, TextStyle};
use stereokit_rust::tex::TexFormat;
use stereokit_rust::tools::xr_comp_layers::{SwapchainSk, XrCompLayers};
use stereokit_rust::util::named_colors::{self, RED};
use stereokit_rust::util::{Color128, Time};
use stereokit_rust::{
    material::Material,
    maths::{Matrix, Pose, Vec2, Vec3},
    mesh::Mesh,
    prelude::*,
    system::{Backend, BackendXRType, Renderer, Text},
    tex::Tex,
    ui::Ui,
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

    material: Material,
    window_pose: Pose,
    preview_pose: Pose,
    swapchain_sk: Option<SwapchainSk>,
    render_list: RenderList,
    projection: Matrix,
    sort_order: f32,

    pub transform: Matrix,
    pub text: String,
    text_style: Option<TextStyle>,
}

unsafe impl Send for Layers1 {}

impl Default for Layers1 {
    fn default() -> Self {
        let content_pose = Pose::new(Vec3::ZERO, None);
        let window_pose = content_pose * Matrix::t_r([-0.2, 1.5, -1.0], [0.0, 180.0, 0.0]);
        let preview_pose = content_pose * Matrix::t_r([0.2, 1.5, -1.0], [0.0, 180.0, 0.0]);
        Self {
            id: "Layers1".into(),
            sk_info: None,

            material: Material::pbr().copy(),
            window_pose,
            preview_pose,
            swapchain_sk: None,
            render_list: RenderList::new(),
            projection: Matrix::orthographic(0.2, 0.2, 0.01, 1010.0),
            sort_order: 1.0,

            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, [0.0, 180.0, 0.0]),
            text: "Layers1".to_owned(),
            text_style: None,
        }
    }
}

impl Layers1 {
    fn start(&mut self) -> bool {
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));

        // Wrap the swapchain
        if Backend::xr_type() == BackendXRType::OpenXR {
            if let Some(comp_layer) = XrCompLayers::new() {
                if let Some(handle) = comp_layer.try_make_swapchain(
                    512,
                    512,
                    TexFormat::RGBA32,
                    SwapchainUsageFlags::COLOR_ATTACHMENT,
                    false,
                ) {
                    self.swapchain_sk = SwapchainSk::wrap(handle, TexFormat::RGBA32, 512, 512, Some(comp_layer));
                } else {
                    Log::warn("Failed to create XR swapchain");
                    return false;
                }
            } else {
                Log::warn("XrCompLayers is not available, cannot start Layers1 demo");
                return false;
            }
            // prepare a simple scene
            let mut mat = Material::default().copy();
            mat.id("quadmat");
            if let Ok(floor) = Tex::from_file("textures/parquet2/parquet2.ktx2", true, None) {
                mat.diffuse_tex(&floor);
            }
            self.render_list
                .add_mesh(Mesh::sphere(), mat, Matrix::s(0.05 * Vec3::ONE), named_colors::BLUE, None);
            true
        } else {
            Log::warn("OpenXR backend is not available, cannot start Layers1 demo");
            false
        }
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {
        // no events
    }

    fn draw(&mut self, token: &MainThreadToken) {
        const SIZE: f32 = 0.3;
        // interactive handle
        Ui::handle(
            "QuadLayer",
            &mut self.preview_pose,
            Bounds::new([0.0, 0.0, 0.0], [SIZE, SIZE, SIZE]),
            false,
            None,
            None,
        );
        Mesh::cube().draw(
            token,
            &self.material,
            Matrix::t_s(self.preview_pose.position, Vec3::new(SIZE, SIZE, 0.04)),
            None,
            None,
        );

        if let Some(sc) = &mut self.swapchain_sk {
            let old_color = Renderer::get_clear_color();
            Renderer::clear_color(named_colors::SKY_BLUE);
            if let Err(e) = sc.acquire_image(None) {
                Log::warn(format!("Failed to acquire image from swapchain: {}", e));
                Log::warn("Skipping rendering for now...");
                self.swapchain_sk = None;
                return;
            }
            let render_tex = sc.get_render_target().expect("SwapchainSk should have a render target");
            self.render_list.draw_now(
                render_tex,
                //                Matrix::look_at(Vec3::angle_xy(Time::get_totalf() * 90.0, 0.0), Vec3::ZERO, None),
                Matrix::look_at(Vec3::X, Vec3::ZERO, None),
                self.projection,
                Some(Color128::new(0.4, 0.3, 0.2, 0.5)),
                Some(RenderClear::Color),
                Rect::new(0.0, 0.0, 1.0, 1.0),
                None,
            );

            //let sprite = Sprite::from_tex(render_tex, None, None).unwrap();
            let sprite = Sprite::from_file("icons/fly_over.png", None, None).unwrap();

            sprite.draw(token, self.transform, Pivot::Center, None);

            assert_eq!(render_tex.get_width(), Some(512));

            if let Err(e) = sc.release_image() {
                Log::warn(format!("Failed to release image from swapchain: {}", e));
                Log::warn("Skipping rendering for now...");
                self.swapchain_sk = None;
                return;
            }

            Renderer::clear_color(old_color);
            XrCompLayers::submit_quad_layer(
                self.preview_pose,
                Vec2::new(SIZE, SIZE),
                sc.handle,
                Rect::new(0.0, 0.0, sc.width as f32, sc.height as f32),
                0,
                self.sort_order as i32,
                None,
                None,
            );
        } else {
            Text::add_in(
                token,
                "Requires an OpenXR runtime!",
                self.preview_pose,
                Vec2::new(SIZE, SIZE),
                TextFit::Wrap,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
        }

        // UI window
        Ui::window_begin("Composition Layers", &mut self.window_pose, Some(Vec2::new(0.2, 0.0)), None, None);
        Ui::label(format!("Sort Order {}", self.sort_order as i32), Some(Vec2::new(0.1, 0.0)), false);
        Ui::same_line();
        Ui::hslider("Sort Order", &mut self.sort_order, -1.0, 1.0, Some(1.0), None, None, None);

        #[cfg(target_os = "android")]
        {
            Ui::hseparator();
            if Ui::button("Get android surface", None) {
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

        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }
}
