use crate::{
    StereoKitError,
    material::{Material, MaterialBuffer, MaterialBufferT, MaterialT},
    maths::{Bool32T, Matrix, Pose, Rect},
    mesh::{Mesh, MeshT},
    model::{Model, ModelT},
    system::{IAsset, Log, assets_releaseref_threadsafe},
    tex::{Tex, TexFormat, TexT},
    util::{Color128, SphericalHarmonics},
};
use std::{
    self,
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    ptr::{NonNull, null_mut},
};

/// When rendering to a rendertarget, this tells if and what of the rendertarget gets cleared before rendering. For
/// example, if you are assembling a sheet of images, you may want to clear everything on the first image draw, but not
/// clear on subsequent draws.
/// <https://stereokit.net/Pages/StereoKit/RenderClear.html>
///
/// see also [`Renderer`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum RenderClear {
    /// Don’t clear anything, leave it as it is.
    None = 0,
    /// Clear the rendertarget’s color data.
    Color = 1,
    /// Clear the rendertarget’s depth data, if present.
    Depth = 2,
    /// Clear both color and depth data.
    All = 3,
}

bitflags::bitflags! {
    /// When rendering content, you can filter what you’re rendering by the RenderLayer that they’re on. This allows
    /// you to draw items that are visible in one render, but not another. For example, you may wish to draw a player’s
    /// avatar in a ‘mirror’ rendertarget, but not in the primary display. See Renderer.LayerFilter for configuring
    /// what the primary display renders.
    /// Render layers can also be mixed and matched like bit-flags! Note that while this enum is 32 bits wide, render
    /// layers are stored internally in 16 bits when items are queued for drawing. Only the low 16 bits are usable as
    /// layers, so any custom flags above bit 15 will be silently truncated.
    /// <https://stereokit.net/Pages/StereoKit/RenderLayer.html>
    ///
    /// see also [`Renderer`] [`Mesh::draw`] [`Model::draw`] [`Model::draw_mat`] [`RenderList`]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct RenderLayer: u32 {
        /// The default render layer. All Draw use this layer unless otherwise specified.
        const Layer0 = 1 << 0;
        /// Render layer 1.
        const Layer1 = 1 << 1;
        /// Render layer 2.
        const Layer2 = 1 << 2;
        /// Render layer 3.
        const Layer3 = 1 << 3;
        /// Render layer 4.
        const Layer4 = 1 << 4;
        /// Render layer 5.
        const Layer5 = 1 << 5;
        /// Render layer 6.
        const Layer6 = 1 << 6;
        /// Render layer 7.
        const Layer7 = 1 << 7;
        /// Render layer 8.
        const Layer8 = 1 << 8;
        /// Render layer 9.
        const Layer9 = 1 << 9;
        /// The default VFX layer, StereoKit draws some non-standard mesh content using this flag, such as lines.
        const Vfx    = 1 << 10;
        /// For items that should only be drawn from the first person perspective. By default, this is enabled for
        /// renders that are from a 1st person viewpoint.
        const FirstPerson = 1 << 11;
        /// For items that should only be drawn from the third person perspective. By default, this is enabled for
        /// renders that are from a 3rd person viewpoint.
        const ThirdPerson = 1 << 12;
        /// The default layer for StereoKit's UI. Mesh and model content drawn by the UI system uses this layer, see
        /// [`UI::RenderLayer`] to change it.
        const UI          = 1 << 13;
        /// This is a flag that specifies all possible layers. If you want to render all layers, then this is the layer
        ///  filter you would use. This is the default for render filtering.
        const All = 0xFFFF;
        /// This is a combination of all layers that are not the VFX layer.
        const AllRegular = Self::Layer0.bits() | Self::Layer1.bits() | Self::Layer2.bits() | Self::Layer3.bits() | Self::Layer4.bits() | Self::Layer5.bits() | Self::Layer6.bits() | Self::Layer7.bits() | Self::Layer8.bits() | Self::Layer9.bits();
        /// All layers except for the third person layer.
        const AllFirstPerson = Self::All.bits() & !Self::ThirdPerson.bits();
        ///All layers except for the first person layer.
        const AllThirdPerson = Self::All.bits() & !Self::FirstPerson.bits();
    }
}

impl Default for RenderLayer {
    /// Layer_all is the default.
    fn default() -> Self {
        RenderLayer::All
    }
}

/// The projection mode used by StereoKit for the main camera! You can use this with Renderer.Projection. These options
/// are only available in flatscreen mode, as MR headsets provide very specific projection matrices.
/// <https://stereokit.net/Pages/StereoKit/Projection.html>
///
/// see also [`Renderer`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Projection {
    /// This is the default projection mode, and the one you’re most likely to be familiar with! This is where parallel
    /// lines will converge as they go into the distance.
    Perspective = 0,
    /// Orthographic projection mode is often used for tools, 2D rendering, thumbnails of 3D objects, or other similar
    /// cases. In this mode, parallel lines remain parallel regardless of how far they travel.
    Orthographic = 1,
}

/// Do you need to draw something? Well, you’re probably in the right place! This static class includes a variety of
/// different drawing methods, from rendering Models and Meshes, to setting rendering options and drawing to offscreen
/// surfaces! Even better, it’s entirely a static class, so you can call it from anywhere :)
/// <https://stereokit.net/Pages/StereoKit/Renderer.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{render::{Renderer, RenderLayer}, system::Assets, maths::{Matrix, Pose},
///                      render::RenderList,
///                      mesh::Mesh, model::Model, material::Material, util::named_colors};
///
/// let sun = Mesh::generate_sphere(5.0, None);
/// let material = Material::pbr();
/// let transform_sun = Matrix::t([-6.0, -4.0, -10.0]);
///
/// let plane = Model::from_file("plane.glb", None, None).expect("plane.glb should be there");
/// let transform_plane = Matrix::t_r_s([0.0, 0.2, -0.7], [0.0, 120.0, 0.0], [0.15, 0.15, 0.15]);
///
/// // We want to replace the gray background with a dark blue sky:
/// let mut primary = RenderList::primary();
/// assert_eq!(primary.get_count(), 0);
/// Renderer::clear_color(named_colors::BLUE);
///
/// Assets::block_for_priority(i32::MAX);
///
/// filename_scr = "screenshots/renderer.jpeg";
/// test_steps!( // !!!! Get a proper main loop !!!!
///     
///     primary.clear();
///
///     Renderer::add_mesh(&sun, &material, transform_sun,
///         Some(named_colors::RED.into()), None);
///
///     Renderer::add_model(&plane, transform_plane,
///         Some(named_colors::PINK.into()), Some(RenderLayer::FirstPerson));
///
///     Renderer::layer_filter(RenderLayer::All);
///  
///     if iter == number_of_steps {
///         // This is the way test_screenshot!() works:
///         Renderer::screenshot( filename_scr, 90, Pose::look_at(from_scr, at_scr),
///             width_scr, height_scr, Some(fov_scr) );
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/renderer.jpeg" alt="screenshot" width="200">
pub struct Renderer;

unsafe extern "C" {
    pub fn render_set_clip(near_plane: f32, far_plane: f32);
    pub fn render_get_clip(out_near_plane: *mut f32, out_far_plane: *mut f32);
    pub fn render_set_fov(vertical_field_of_view_degrees: f32);
    pub fn render_get_fov() -> f32;
    pub fn render_set_ortho_clip(near_plane: f32, far_plane: f32);
    pub fn render_set_ortho_size(viewport_height_meters: f32);
    pub fn render_get_ortho_size() -> f32;
    pub fn render_set_projection(proj: Projection);
    pub fn render_get_projection() -> Projection;
    pub fn render_get_cam_root() -> Matrix;
    pub fn render_set_cam_root(cam_root: *const Matrix);
    pub fn render_set_skytex(sky_texture: TexT);
    pub fn render_get_skytex() -> TexT;
    pub fn render_set_skymaterial(sky_material: MaterialT);
    pub fn render_get_skymaterial() -> MaterialT;
    pub fn render_set_skylight(light_info: *const SphericalHarmonics);
    pub fn render_get_skylight() -> SphericalHarmonics;
    pub fn render_set_filter(layer_filter: RenderLayer);
    pub fn render_get_filter() -> RenderLayer;
    pub fn render_set_scaling(display_tex_scale: f32);
    pub fn render_get_scaling() -> f32;
    pub fn render_set_viewport_scaling(viewport_rect_scale: f32);
    pub fn render_get_viewport_scaling() -> f32;
    pub fn render_set_multisample(display_tex_multisample: i32);
    pub fn render_get_multisample() -> i32;
    pub fn render_override_capture_filter(use_override_filter: Bool32T, layer_filter: RenderLayer);
    pub fn render_get_capture_filter() -> RenderLayer;
    pub fn render_has_capture_filter() -> Bool32T;
    pub fn render_set_clear_color(color_gamma: Color128);
    pub fn render_get_clear_color() -> Color128;
    pub fn render_enable_skytex(show_sky: Bool32T);
    pub fn render_enabled_skytex() -> Bool32T;

    pub fn render_global_texture(register_slot: i32, texture: TexT);
    pub fn render_global_buffer(register_slot: i32, buffer: MaterialBufferT);
    pub fn render_add_mesh(
        mesh: MeshT,
        material: MaterialT,
        transform: *const Matrix,
        color_linear: Color128,
        layer: RenderLayer,
    );
    pub fn render_add_model(model: ModelT, transform: *const Matrix, color_linear: Color128, layer: RenderLayer);
    pub fn render_add_model_mat(
        model: ModelT,
        material_override: MaterialT,
        transform: *const Matrix,
        color_linear: Color128,
        layer: RenderLayer,
    );
    pub fn render_blit(to_rendertarget: TexT, material: MaterialT);

    pub fn render_screenshot(
        file_utf8: *const c_char,
        file_quality_100: i32,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view_degrees: f32,
    );
    pub fn render_screenshot_capture(
        render_on_screenshot_callback: ::std::option::Option<
            unsafe extern "C" fn(data: *mut c_void, format: TexFormat, width: i32, height: i32, context: *mut c_void),
        >,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view_degrees: f32,
        tex_format: TexFormat,
        context: *mut c_void,
    );
    pub fn render_screenshot_viewpoint(
        render_on_screenshot_callback: ::std::option::Option<
            unsafe extern "C" fn(data: *mut c_void, format: TexFormat, width: i32, height: i32, context: *mut c_void),
        >,
        camera: Matrix,
        projection: Matrix,
        width: i32,
        height: i32,
        layer_filter: RenderLayer,
        clear: RenderClear,
        viewport: Rect,
        tex_format: TexFormat,
        context: *mut c_void,
    );
    pub fn render_to(
        to_rendertarget: TexT,
        to_target_index: i32,
        arr_camera: *const Matrix,
        arr_projection: *const Matrix,
        view_count: i32,
        layer_filter: RenderLayer,
        material_variant: i32,
        clear: RenderClear,
        viewport: Rect,
    );

    pub fn render_MaterialTo(
        to_rendertarget: TexT,
        override_material: MaterialT,
        camera: *const Matrix,
        projection: *const Matrix,
        layer_filter: RenderLayer,
        clear: RenderClear,
        viewport: Rect,
    );
    pub fn render_get_device(device: *mut *mut c_void, context: *mut *mut c_void);

}

/// screenshot_capture trampoline
///
/// see also [`Renderer::screenshot_capture`]
unsafe extern "C" fn sc_capture_trampoline<F: FnMut(&[u8], TexFormat, usize, usize)>(
    data: *mut c_void,
    format: TexFormat,
    width: i32,
    height: i32,
    context: *mut c_void,
) {
    let closure = unsafe { &mut *(context as *mut &mut F) };
    let pixel_count = (width * height) as usize;
    // Compute the byte length from the format's bytes-per-pixel, falling back to width*height
    // (one byte per pixel) for formats without a known per-pixel size.
    let bytes_per_pixel = format.bytes_per_pixel().max(1);
    let byte_len = pixel_count * bytes_per_pixel;
    closure(
        unsafe { std::slice::from_raw_parts(data as *const u8, byte_len) },
        format,
        width as usize,
        height as usize,
    )
}

impl Renderer {
    /// Sets the root transform of the camera! This will be the identity matrix by default. The user’s head
    /// location will then be relative to this point. This is great to use if you’re trying to do teleportation,
    /// redirected walking, or just shifting the floor around.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CameraRoot.html>
    ///
    /// see also [`render_set_cam_root`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, render::Renderer};
    ///
    /// let camera_root = Renderer::get_camera_root();
    /// assert_eq!(camera_root, Matrix::IDENTITY);
    ///
    /// let transform = Matrix::t([0.0, 0.0, -1.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Renderer::camera_root(transform);
    ///     let camera_root = Renderer::get_camera_root();
    ///     assert_eq!(camera_root, transform);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn camera_root(transform: impl Into<Matrix>) {
        unsafe { render_set_cam_root(&transform.into()) }
    }

    /// This is the gamma space color the renderer will clear the screen to when beginning to draw a new frame.
    /// [`Color128::BLACK_TRANSPARENT`] is the default and is mandatory for some Passthrough solutions.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ClearColor.html>
    ///
    /// see also [`render_set_clear_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer,
    ///                      render::RenderList, util::{named_colors, Color128}};
    ///
    /// // We want to replace the gray background with a dark blue sky:
    /// let mut primary = RenderList::primary();
    /// assert_eq!(primary.get_count(), 0);
    ///
    ///
    /// assert_eq!(Renderer::get_clear_color(), Color128::BLACK_TRANSPARENT);
    /// Renderer::clear_color(named_colors::BLUE);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     
    ///     primary.clear();
    ///
    ///     assert_eq!(Renderer::get_clear_color(), named_colors::BLUE.into());
    ///
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clear_color(color_gamma: impl Into<Color128>) {
        unsafe { render_set_clear_color(color_gamma.into()) }
    }

    /// Enables or disables rendering of the skybox texture! It’s enabled by default on Opaque displays, and completely
    /// unavailable for transparent displays.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/EnableSky.html>
    ///
    /// see also [`render_enable_skytex`] [`Renderer::clear_color`] [`crate::tex::SHCubemap`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// assert_eq!(Renderer::get_enable_sky(), true);
    ///
    /// Renderer::enable_sky(false);
    /// assert_eq!(Renderer::get_enable_sky(), false);
    ///
    /// Renderer::enable_sky(true);
    /// assert_eq!(Renderer::get_enable_sky(), true);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn enable_sky(enable: bool) {
        unsafe { render_enable_skytex(enable as Bool32T) }
    }

    /// By default, StereoKit renders all first-person layers. This is a bit flag that allows you to change which layers
    /// StereoKit renders for the primary viewpoint. To change what layers a visual is on, use a Draw method that
    /// includes a RenderLayer as a parameter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/LayerFilter.html>
    ///
    /// see also [`render_set_filter`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::{Renderer, RenderLayer};
    ///
    /// assert_eq!(Renderer::get_layer_filter(), RenderLayer::AllFirstPerson);
    ///
    /// Renderer::layer_filter(RenderLayer::All);
    /// assert_eq!(Renderer::get_layer_filter(), RenderLayer::All);
    ///
    /// Renderer::layer_filter(RenderLayer::AllFirstPerson);
    /// assert_eq!(Renderer::get_layer_filter(), RenderLayer::AllFirstPerson);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn layer_filter(filter: RenderLayer) {
        unsafe { render_set_filter(filter) }
    }

    /// Allows you to set the multisample (MSAA) level of the render surface. Valid values are 1, 2, 4, and 8, though
    /// this is clamped to what the GPU actually supports. Note that while this can greatly smooth out edges, it also
    /// increases RAM usage and fill rate. How much it costs depends a lot on the GPU! Tiled renderers, like the mobile
    /// chips in most standalone XR headsets, resolve MSAA in tile memory, which makes it nearly free. Desktop GPUs
    /// instead pay memory bandwidth for the multisampled surface and for resolving it, so MSAA is far more expensive
    /// there, especially at high resolutions. A value of 1 skips the multisampled surface entirely. If known in
    /// advance, set this via SKSettings in initialization. This is a _very_ costly change to make. Defaults to 4.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Multisample.html>
    ///
    /// see also [`render_set_multisample`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// assert_eq!(Renderer::get_multisample(), 1);
    ///
    /// Renderer::multisample(4);
    /// assert_eq!(Renderer::get_multisample(), 4);
    ///
    /// Renderer::multisample(1);
    /// assert_eq!(Renderer::get_multisample(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn multisample(level: i32) {
        unsafe { render_set_multisample(level) }
    }

    /// For flatscreen applications only! This allows you to change the camera projection between perspective and
    /// orthographic projection. This may be of interest for some category of UI work, but is generally a niche piece of
    /// functionality.
    /// Swapping between perspective and orthographic will also switch the clipping planes and field of view to the
    /// values associated with that mode. See set_clip/set_fov for perspective, and set_ortho_clip/set_ortho_size for
    /// orthographic.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Projection.html>
    ///
    /// see also [`render_set_projection`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::{Renderer, Projection};
    ///
    /// assert_eq!(Renderer::get_projection(), Projection::Perspective);
    ///
    /// Renderer::projection(Projection::Orthographic);
    /// assert_eq!(Renderer::get_projection(), Projection::Orthographic);
    ///
    /// Renderer::projection(Projection::Perspective);
    /// assert_eq!(Renderer::get_projection(), Projection::Perspective);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn projection(projection: Projection) {
        unsafe { render_set_projection(projection) }
    }

    /// OpenXR has a recommended default for the main render surface, this value allows you to set SK’s surface to a
    /// multiple of the recommended size. Note that the final resolution may also be clamped or quantized. Only works in
    /// XR mode. If known in advance, set this via [`crate::sk::SkSettings`] in initialization. This is a very costly change to make.
    /// Consider if Viewport_scaling will work for you instead, and prefer that.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Scaling.html>
    ///
    /// see also [`render_set_scaling`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// assert_eq!(Renderer::get_scaling(), 1.0);
    ///
    /// Renderer::scaling(0.5);
    /// assert_eq!(Renderer::get_scaling(), 0.5);
    ///
    /// Renderer::scaling(1.0);
    /// assert_eq!(Renderer::get_scaling(), 1.0);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn scaling(scaling: f32) {
        unsafe { render_set_scaling(scaling) }
    }

    /// This allows you to trivially scale down the area of the swapchain that StereoKit renders to! This can be used
    /// to boost performance in situations where full resolution is not needed, or to reduce GPU time. This value is
    /// locked to the 0-1 range
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ViewportScaling.html>
    ///
    /// see also [`render_set_viewport_scaling`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// assert_eq!(Renderer::get_viewport_scaling(), 1.0);
    ///
    /// Renderer::viewport_scaling(0.5);
    /// assert_eq!(Renderer::get_viewport_scaling(), 0.5);
    ///
    /// Renderer::viewport_scaling(1.0);
    /// assert_eq!(Renderer::get_viewport_scaling(), 1.0);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn viewport_scaling(scaling: f32) {
        unsafe { render_set_viewport_scaling(scaling) }
    }

    /// Sets the lighting information for the scene! You can build one through [`SphericalHarmonics::from_lights`], or grab
    /// one from [`crate::tex::SHCubemap`]
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    ///
    /// see also [`render_set_skylight`] [`crate::tex::SHCubemap`] [`crate::util::SHLight`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, maths::Vec3,
    ///                      util::{named_colors, SphericalHarmonics, SHLight}};
    ///
    /// let light1 = SHLight::new([0.0, 1.0, 0.0], named_colors::WHITE);
    /// let light2 = SHLight::new([0.0, 0.0, 1.0], named_colors::WHITE);
    ///
    /// let sh = SphericalHarmonics::from_lights(&[light1, light2]);
    ///
    /// Renderer::sky_light(sh);
    /// let sky_light = Renderer::get_sky_light();
    ///
    /// assert_eq!(sky_light, sh);
    /// assert_eq!(sh.get_dominent_light_direction(),
    ///            Vec3 { x: -0.0, y: -1.0, z: -1.0 }.get_normalized());
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sky_light(light_info: SphericalHarmonics) {
        unsafe { render_set_skylight(&light_info) }
    }

    /// Set a cubemap skybox texture for rendering a background! This is only visible on Opaque displays, since
    /// transparent displays have the real world behind them already! StereoKit has a a default procedurally generated
    /// skybox. You can load one with [`crate::tex::SHCubemap`]. If you’re trying to affect the lighting,
    /// see [`Renderer::sky_light`].
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`render_set_skytex`] [`crate::tex::SHCubemap`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, system::Assets, tex::SHCubemap};
    ///
    /// let sky_cubemap = SHCubemap::from_cubemap("hdri/sky_dawn.hdr", true, 9999)
    ///                        .expect("sky_cubemap should be created");
    ///
    /// let sky_tex = sky_cubemap.get().1;
    ///
    /// Assets::block_for_priority(i32::MAX);
    ///
    /// Renderer::sky_tex(&sky_tex);
    /// let sky_tex_get = Renderer::get_sky_tex();
    ///
    /// assert_eq!(sky_tex_get, sky_tex);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sky_tex(tex: impl AsRef<Tex>) {
        unsafe { render_set_skytex(tex.as_ref().0.as_ptr()) }
    }

    /// This is the Material that StereoKit is currently using to draw the skybox! It needs a special shader that's
    /// tuned for a full-screen quad. If you just want to change the skybox image, try setting [`Renderer::sky_tex`]
    /// instead.
    ///  
    /// This value will never be null! If you try setting this to null, it will assign SK's built-in default sky
    /// material. If you want to turn off the skybox, see [`Renderer::enable_sky`] instead.
    ///  
    /// Recommended Material settings would be:
    /// - DepthWrite: false
    /// - DepthTest: LessOrEq
    /// - QueueOffset: 100
    ///
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyMaterial.html>
    ///
    /// see also [`render_set_skymaterial`] [`crate::tex::SHCubemap`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, material::Material};
    ///
    /// let material = Material::pbr().copy();
    /// Renderer::sky_material(&material);
    ///
    /// let same_material = Renderer::get_sky_material();
    /// assert_eq!(same_material, material);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sky_material(material: impl AsRef<Material>) {
        unsafe { render_set_skymaterial(material.as_ref().0.as_ptr()) }
    }

    /// Adds a mesh to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Add.html>
    /// * `mesh` - A valid Mesh you wish to draw.
    /// * `material` - A Material to apply to the Mesh.
    /// * `transform` - A Matrix that will transform the mesh from Model Space into the current Hierarchy Space.
    /// * `color` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you’re adventurous and don’t need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user’s head from a 3rd person perspective, but filtering it out from the 1st person perspective.If None has
    ///   default value of RenderLayer::Layer0
    ///
    /// see also [`render_add_mesh`] [`Mesh::draw`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderLayer}, maths::Matrix,
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let sphere = Mesh::generate_sphere(0.5, None);
    /// let material = Material::pbr();
    /// let transform1 = Matrix::t([-0.5, 0.0, 0.0]);
    /// let transform2 = Matrix::t([ 0.5, 0.0, -1.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///
    ///     Renderer::add_mesh(&sphere, &material, transform1,
    ///         Some(named_colors::RED.into()), Some(RenderLayer::Layer0));
    ///
    ///     Renderer::add_mesh(&sphere, &material, transform2, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add_mesh(
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color = color.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_add_mesh(mesh.as_ref().0.as_ptr(), material.as_ref().0.as_ptr(), &transform.into(), color, layer)
        }
    }

    /// Adds a Model to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Add.html>
    /// * `model` -  A valid Model you wish to draw.
    /// * `transform` - A Matrix that will transform the Model from Model Space into the current Hierarchy Space.
    /// * `color` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you’re adventurous and don’t need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can
    ///   be useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user’s head from a 3rd person perspective, but filtering it out from the 1st person perspective. If None has
    ///   default value of RenderLayer::Layer0
    ///
    /// see also [`render_add_model`] [`Model::draw`] [`Model::draw_with_material`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderLayer}, maths::Matrix,
    ///                      model::Model, util::named_colors};
    ///
    /// let model = Model::from_file("plane.glb", None, None).expect("plane.glb should be there");
    /// let transform1 = Matrix::t([-2.5, 0.0, -5.0]);
    /// let transform2 = Matrix::t([ 2.5, 0.0, -5.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///
    ///     Renderer::add_model(&model, transform1,
    ///         Some(named_colors::RED.into()), Some(RenderLayer::Layer0));
    ///
    ///     Renderer::add_model(&model, transform2, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add_model(
        model: impl AsRef<Model>,
        transform: impl Into<Matrix>,
        color: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color = color.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { render_add_model(model.as_ref().0.as_ptr(), &transform.into(), color, layer) }
    }

    /// Renders a Material onto a rendertarget texture! StereoKit uses a 4 vert quad stretched over the surface of the
    /// texture, and renders the material onto it to the texture.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Blit.html>
    /// * `to_render_target` - A texture that’s been set up as a render target!
    /// * `material` - This material is rendered onto the texture! Set it up like you would if you were applying it to
    ///   a plane, or quad mesh.
    ///
    /// see also [`render_blit`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, material::Material, tex::{Tex, TexFormat}};
    ///
    /// let material = Material::from_file("shaders/brick_pbr.hlsl.sks", None)
    ///                            .expect("Brick shader should load and create material!");
    /// let tex = Tex::render_target(200,200, None, TexFormat::Rgba32Srgb, TexFormat::None)
    ///                    .expect("RenderTarget should be created");
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == number_of_steps {Renderer::blit(&tex, &material);}
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn blit(to_render_target: impl AsRef<Tex>, material: impl AsRef<Material>) {
        unsafe { render_blit(to_render_target.as_ref().0.as_ptr(), material.as_ref().0.as_ptr()) }
    }

    /// The capture_filter is a layer mask for Mixed Reality Capture, or 2nd person observer rendering. On HoloLens and
    /// WMR, this is the video rendering feature. This allows you to hide, or reveal certain draw calls when rendering
    /// video output.
    ///
    /// By default, the capture_filter will always be the same as [`Renderer::layer_filter`], overriding this will mean this
    /// filter no longer updates with layer_filter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/OverrideCaptureFilter.html>
    /// * `use_override_filter` - Enables (true) or disables (false) the overridden filter value provided here.
    /// * `override_filter` - The filter for capture rendering to use. This is ignored if useOverrideFilter is false.
    ///
    /// see also [`render_override_capture_filter`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderLayer},
    ///                      maths::Matrix, mesh::Mesh, material::Material};
    ///
    /// let sphere = Mesh::generate_sphere(0.2, None);
    /// let material = Material::pbr();
    ///
    /// assert_eq!(Renderer::has_capture_filter(), false);
    /// assert_eq!(Renderer::get_capture_filter(), RenderLayer::AllFirstPerson);
    ///
    /// Renderer::override_capture_filter(true, RenderLayer::Layer1);
    ///
    /// assert_eq!(Renderer::has_capture_filter(), true);
    /// assert_eq!(Renderer::get_capture_filter(), RenderLayer::Layer1);
    ///
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     sphere.draw(&material, Matrix::IDENTITY, None, Some(RenderLayer::Layer1));
    /// );
    ///
    /// Renderer::override_capture_filter(false, RenderLayer::Layer0);
    /// assert_eq!(Renderer::has_capture_filter(), false);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn override_capture_filter(use_override_filter: bool, override_filter: RenderLayer) {
        unsafe { render_override_capture_filter(use_override_filter as Bool32T, override_filter) }
    }

    /// This renders the current scene to the indicated rendertarget texture, from the specified viewpoint. This call
    /// enqueues a render that occurs immediately before the screen itself is rendered.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/RenderTo.html>
    /// * `to_render_target` - The texture to which the scene will be rendered to. This must be a Rendertarget type
    ///   texture.
    /// * `to_target_index` - Index of the render target's array slice we want to draw to. 0 for single view. This is
    ///   only relevant for array/render target textures with multiple slices.
    /// * `render` - A [`RenderBuilder`] describing the camera(s), projection(s), layer filter, clear behavior and
    ///   viewport to use for this render. Passing a builder with a single camera is equivalent to the old single-view
    ///   overload, while passing one with N cameras performs the multi-view render.
    ///
    /// see also [`render_to`] [`RenderBuilder::render_to`]
    /// ### Examples
    /// TODO: The plane tex is not rendered
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderBuilder}, maths::{Vec3, Quat, Matrix},
    ///                      mesh::Mesh, material::Material, util::named_colors,
    ///                      tex::{Tex, TexFormat}};
    ///
    /// let sun = Mesh::generate_sphere(2.0, None);
    /// let material_sun = Material::pbr();
    /// let transform_sun = Matrix::t([-0.0, 1.0, -4.0]);
    ///
    /// let plane = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let mut material = Material::pbr().copy();
    /// let tex_a = Tex::render_target(200,200, None, TexFormat::Rgba32Srgb, TexFormat::Depth16)
    ///                      .expect("RenderTarget should be created");
    /// let tex_b = Tex::render_target(200,200, None, TexFormat::Rgba32Srgb, TexFormat::Depth16)
    ///                      .expect("RenderTarget should be created");
    /// let transform_plane = Matrix::t([0.0, -0.55, 0.0]);
    ///
    /// let camera = Matrix::t_r(Vec3::Z * 1.0, Quat::look_at(Vec3::Z, Vec3::ZERO, None));
    /// let projection = Matrix::perspective(90.0, 1.0, 0.1, 50.0);
    ///
    /// let render = RenderBuilder::new().camera(camera).projection(projection);
    ///
    /// filename_scr = "screenshots/render_to.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     
    ///     Renderer::add_mesh(&sun, &material_sun, transform_sun,
    ///         Some(named_colors::RED.into()), None);
    ///
    ///     if iter != 0 {  // the read_tex must have been the write_tex first.
    ///         Renderer::add_mesh(&plane, &material, transform_plane,
    ///                            None, None);
    ///     }
    ///
    ///     let (read_tex, write_tex) = if iter % 2 == 0 {
    ///         (&tex_a, &tex_b)
    ///     } else {
    ///         (&tex_b, &tex_a)
    ///     };
    ///
    ///     material.diffuse_tex(&read_tex);
    ///
    ///     if iter % 2 == 0 { // 2 syntaxes for the same job:
    ///         Renderer::render_to(&write_tex, 0, &render);
    ///     } else {
    ///         render.render_to(&write_tex, 0)
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_to.jpeg" alt="screenshot" width="200">
    pub fn render_to(to_render_target: impl AsRef<Tex>, to_target_index: i32, render: &RenderBuilder) {
        render.render_to(to_render_target, to_target_index);
    }

    /// This attaches a texture resource globally across all shaders. StereoKit uses this to attach the sky cubemap for
    /// use in reflections across all materials (register 11). It can be used for things like shadowmaps, wind data, etc.
    ///  Prefer a higher registers (11+) to prevent conflicting with normal Material textures.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetGlobalTexture.html>
    /// * `texture_register` - The texture resource register the texture will bind to. SK uses register 11 already, so
    ///   values above that should be fine.
    /// * `tex` - The texture to assign globally. Setting None here will clear any texture that is currently bound.
    ///
    /// see also [`render_global_texture`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, tex::Tex};
    ///
    /// let tex = Tex::from_file("hdri/sky_dawn.jpeg", true, None)
    ///                    .expect("tex should be created");
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < 2 {
    ///         Renderer::set_global_texture(12, Some(&tex));
    ///     } else {
    ///         Renderer::set_global_texture( 12, None);
    ///     }
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_global_texture(texture_register: i32, tex: Option<&Tex>) {
        if let Some(tex) = tex {
            unsafe { render_global_texture(texture_register, tex.0.as_ptr()) }
        } else {
            unsafe { render_global_texture(texture_register, null_mut()) }
        }
    }

    /// This attaches a buffer resource globally across all shaders. StereoKit uses this to attach the stereokit
    /// rendering constants. It can be used for things like shadowmaps, wind data, etc.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetGlobalBuffer.html>
    /// * `buffer_register` - Valid values are 3-16. This is the register id that this data will be bound to. In HLSL,
    ///   you'll see the slot id for '3' indicated like this `: register(b3)`
    /// * `buffer` - The data buffer you would like to bind
    ///
    /// see also [`render_global_buffer`] []
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, material::MaterialBuffer};
    ///
    /// #[repr(C)]
    /// #[derive(Default, Copy, Clone)]
    /// struct Globals { time: f32, padding: [f32;3] }
    ///
    /// # {
    /// let mut globals = Globals { time: 1.0, ..Default::default() };
    /// let buffer = MaterialBuffer::<Globals>::new();
    /// buffer.set(&mut globals as *mut _);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // Bind the buffer to slot 3 so shaders can read it.
    ///     Renderer::set_global_buffer( 3, &buffer);
    /// );
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_global_buffer<T>(buffer_register: i32, buffer: &MaterialBuffer<T>) {
        unsafe {
            render_global_buffer(buffer_register, buffer.as_ref().as_ptr());
        }
    }

    /// Unbinds any global MaterialBuffer previously bound to this register slot (3-16). Equivalent to passing None
    /// to [`Renderer::set_global_buffer`]. Provided as a convenience method.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetGlobalBuffer.html>
    /// * `buffer_register` - Valid values are 3-16. This is the register id the data was bound to.
    ///
    /// see also [`render_global_buffer`] [`Renderer::set_global_buffer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, material::MaterialBuffer};
    ///
    /// #[repr(C)]
    /// #[derive(Default, Copy, Clone)]
    /// struct Globals { value: f32, padding: [f32;3] }
    ///
    /// # {
    /// let buffer = MaterialBuffer::<Globals>::new();
    /// test_steps!(
    ///     Renderer::set_global_buffer(4, &buffer);
    ///     // Later we decide to unbind it completely
    ///     if iter == number_of_steps -1 {
    ///         Renderer::unset_global_buffer(4);
    ///     }
    /// );
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn unset_global_buffer(buffer_register: i32) {
        unsafe { render_global_buffer(buffer_register, std::ptr::null_mut()) }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given pose, with a
    /// resolution the same size as the screen’s surface. It’ll be saved as a JPEG or PNG file depending on the filename
    /// extension provided.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * `filename` - Filename to write the screenshot to! This will be a PNG if the extension ends with (case
    ///   insensitive) “.png”, and will be a 90 quality JPEG if it ends with anything else.
    /// * `file_quality` - For JPEG files, this is the compression quality of the file from 0-100, 100 being highest
    ///   quality, 0 being smallest size. SK uses a default of 90 here.
    /// * `viewpoint` - is Pose::look_at(from_point, looking_at_point)
    /// * `width` - Size of the screenshot horizontally, in pixels.
    /// * `height`- Size of the screenshot vertically, in pixels
    /// * `field_of_view` - The angle of the viewport, in degrees. If None will use default value of 90°
    ///
    /// see also [`render_screenshot`]
    /// see example in [`Renderer`]
    pub fn screenshot(
        filename: impl AsRef<Path>,
        file_quality: i32,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view: Option<f32>,
    ) {
        let path = filename.as_ref();
        let c_str = CString::new(path.to_str().unwrap_or("!!!path.to_str error!!!").to_owned()).unwrap_or_default();
        let field_of_view = field_of_view.unwrap_or(90.0);
        unsafe { render_screenshot(c_str.as_ptr(), file_quality, viewpoint, width, height, field_of_view) }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given position at the given
    /// point, with a resolution the same size as the screen’s surface. This overload allows for retrieval of the color
    /// data directly from the render thread! You can use the color data directly by saving/processing it inside your
    /// callback, or you can keep the data alive for as long as it is referenced.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * `on_screenshot` : closure |&[u8], TexFormat, width:usize, height:usize|
    /// * `viewpoint` - is Pose::look_at(from_point, looking_at_point)
    /// * `width` - Size of the screenshot horizontally, in pixels.
    /// * `height`- Size of the screenshot vertically, in pixels
    /// * `field_of_view` - The angle of the viewport, in degrees. If None will use default value of 90°
    /// * `tex_format` - The pixel format of the color data. If None will use default value of TexFormat::Rgba32Srgb
    ///
    /// see also [`render_screenshot_capture`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, system::Assets, maths::{Pose, Matrix},
    ///                      tex::{Tex, TexFormat, TexType},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let sun = Mesh::generate_sphere(7.0, None);
    /// let material_sun = Material::pbr();
    /// let transform_sun = Matrix::t([-6.0, 3.0, -10.0]);
    ///
    /// let plane = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let mut material = Material::unlit().copy();
    /// let mut tex = Tex::gen_color(named_colors::WHITE, 200, 200,
    ///                          TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// tex.id("CAPTURE_TEXTURE_ID");
    /// material.diffuse_tex(&tex);
    /// let transform_plane = Matrix::t([0.0, -0.55, 0.0]);
    ///
    /// let camera_pose = Pose::new([0.0, 0.0, 1.0], None);
    /// Assets::block_for_priority(i32::MAX);
    ///
    /// number_of_steps = 20;
    /// filename_scr = "screenshots/screenshot_capture.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     
    ///     Renderer::add_mesh(&sun, &material_sun, transform_sun,
    ///         Some(named_colors::RED.into()), None);
    ///
    ///     Renderer::add_mesh(&plane, &material, transform_plane,
    ///         None, None);
    ///     
    ///     Renderer::screenshot_capture(move |dots, format, width, height| {
    ///             let tex = Tex::find("CAPTURE_TEXTURE_ID").ok();
    ///             match tex {
    ///                 Some(mut tex) => tex.set_colors_u8(width, height, dots, format.bytes_per_pixel()),
    ///                 None => panic!("CAPTURE_TEXTURE_ID not found!"),
    ///             };
    ///         },
    ///         camera_pose, 200, 200, None, None
    ///     );
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screenshot_capture.jpeg" alt="screenshot" width="200">
    pub fn screenshot_capture<F: FnMut(&[u8], TexFormat, usize, usize)>(
        mut on_screenshot: F,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view: Option<f32>,
        tex_format: Option<TexFormat>,
    ) {
        let field_of_view = field_of_view.unwrap_or(90.0);
        let tex_format = tex_format.unwrap_or(TexFormat::Rgba32Srgb);
        let mut closure = &mut on_screenshot;
        unsafe {
            render_screenshot_capture(
                Some(sc_capture_trampoline::<F>),
                viewpoint,
                width,
                height,
                field_of_view,
                tex_format,
                &mut closure as *mut _ as *mut c_void,
            )
        }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given position at the given
    /// point, with a resolution the same size as the screen’s surface. This overload allows for retrieval of the color
    /// data directly from the render thread! You can use the color data directly by saving/processing it inside your
    /// callback, or you can keep the data alive for as long as it is referenced.
    ///  <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * `on_screenshot` : closure |&[u8], TexFormat, width:usize, height:usize|
    /// * `render` - A [`RenderBuilder`] describing the camera, projection, layer filter, clear behavior and viewport to
    ///   use for this screenshot. The screenshot is taken from the camera at index `camera_index`.
    /// * `camera_index` - Index of the camera/projection pair in `render` to take the screenshot from. 0 in most cases.
    /// * `width` - Size of the screenshot horizontally, in pixels.
    /// * `height`- Size of the screenshot vertically, in pixels
    /// * `tex_format` - The pixel format of the color data. If None will use default value of TexFormat::Rgba32Srgb
    ///
    /// see also [`RenderBuilder::screenshot`] [`Renderer::screenshot_capture`] [`Renderer::screenshot`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderBuilder}, system::Assets, maths::{Vec3, Quat, Matrix},
    ///                      tex::{Tex, TexType, TexFormat},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let sun = Mesh::generate_sphere(7.0, None);
    /// let material_sun = Material::pbr();
    /// let transform_sun = Matrix::t([6.0, 3.0, -10.0]);
    ///
    /// let plane = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let mut material = Material::unlit().copy();
    /// let mut tex = Tex::gen_color(named_colors::VIOLET, 200, 200, TexType::Rendertarget, TexFormat::Rgba32Srgb);
    ///
    /// tex.id("CAPTURE_TEXTURE_ID");
    /// material.diffuse_tex(&tex);
    /// let transform_plane = Matrix::t([0.0, -0.55, 0.0]);
    ///
    /// let camera = Matrix::t_r(Vec3::Z * 2.0, Quat::look_at(Vec3::Z, Vec3::ZERO, None));
    /// let projection = Matrix::perspective(90.0, 1.0, 0.1, 20.0);
    /// let render = RenderBuilder::new().camera(camera).projection(projection);
    /// Assets::block_for_priority(i32::MAX);
    ///
    /// number_of_steps = 200;
    /// filename_scr = "screenshots/screenshot_viewpoint.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     
    ///     Renderer::add_mesh(&sun, &material_sun, transform_sun,
    ///         Some(named_colors::RED.into()), None);
    ///
    ///     Renderer::add_mesh(&plane, &material, transform_plane,
    ///         None, None);
    ///
    ///     Renderer::screenshot_viewpoint(move |dots, format, width, height| {
    ///             let tex = Tex::find("CAPTURE_TEXTURE_ID").ok();
    ///             match tex {
    ///                 Some(mut tex) => tex.set_colors_u8(width, height, dots, format.bytes_per_pixel()),
    ///                 None => panic!("CAPTURE_TEXTURE_ID not found!"),
    ///             };
    ///         },
    ///         &render, 0, 200, 200, None
    ///     );
    /// );
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/screenshot_viewpoint.jpeg" alt="screenshot" width="200">
    pub fn screenshot_viewpoint<F: FnMut(&[u8], TexFormat, usize, usize)>(
        on_screenshot: F,
        render: &RenderBuilder,
        camera_index: usize,
        width: i32,
        height: i32,
        tex_format: Option<TexFormat>,
    ) {
        render.screenshot(on_screenshot, camera_index, width, height, tex_format.unwrap_or(TexFormat::Rgba32Srgb));
    }

    /// Set the near and far clipping planes of the camera! These are important to z-buffer quality, especially when
    /// using low bit depth z-buffers as recommended for devices like the HoloLens. The smaller the range between the
    /// near and far planes, the better your z-buffer will look! If you see flickering on objects that are overlapping,
    /// try making the range smaller.
    ///
    /// These values only affect perspective mode projection, which is the default projection mode.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetClip.html>
    /// * `near_plane` - The GPU discards pixels that are too close to the camera, this is that distance! It must be
    ///   larger than zero, due to the projection math, which also means that numbers too close to zero will produce
    ///   z-fighting artifacts. This has an enforced minimum of 0.001, but you should probably stay closer to 0.1.
    /// * `far_plane` - At what distance from the camera does the GPU discard pixel? This is not true distance, but
    ///   rather Z-axis distance from zero in View Space coordinates!
    ///
    /// see also [`render_set_clip`] [`Renderer::get_clip`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// let (near, far) = Renderer::get_clip();
    /// assert_eq!(near, 0.02);
    /// assert_eq!(far, 50.0);
    ///
    /// Renderer::set_clip(0.01, 10.0);
    /// let (near, far) = Renderer::get_clip();
    /// assert_eq!(near, 0.01);
    /// assert_eq!(far, 10.0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_clip(near_plane: f32, far_plane: f32) {
        unsafe { render_set_clip(near_plane, far_plane) }
    }

    /// Only works for 2D windowed modes! This updates the camera's projection matrix with a new vertical field of view.
    ///
    /// This value only affects perspective mode projection, which is the default projection mode.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetFOV.html>
    /// * `vertical_field_of_view` - Vertical field of view in degrees.`
    ///
    /// see also [`render_set_fov`] [`Renderer::get_fov`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    /// let fov = Renderer::get_fov();
    /// assert_eq!(fov, 90.0);
    ///
    /// Renderer::set_fov(120.0);
    /// let fov = Renderer::get_fov();
    /// assert_eq!(fov, 120.0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_fov(vertical_field_of_view: f32) {
        unsafe { render_set_fov(vertical_field_of_view) }
    }

    /// Set the near and far clipping planes of the camera! These are important to z-buffer quality, especially when
    /// using low bit depth z-buffers as recommended for devices like the HoloLens. The smaller the range between the
    /// near and far planes, the better your z-buffer will look! If you see flickering on objects that are overlapping,
    /// try making the range smaller.
    ///
    /// These values only affect orthographic mode projection, which is only available in 2D window modes.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetOrthoClip.html>
    /// * `near_plane` - The GPU discards pixels that are too close to the camera, this is that distance! It must be
    ///   larger than zero, due to the projection math, which also means that numbers too close to zero will produce
    ///   z-fighting artifacts. This has an enforced minimum of 0.001, but you should probably stay closer to 0.1.
    /// * `far_plane` - At what distance from the camera does the GPU discard pixel? This is not true distance, but
    ///   rather Z-axis distance from zero in View Space coordinates!
    ///
    /// see also [`render_set_ortho_clip`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// Renderer::set_ortho_clip(0.01, 5.0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_ortho_clip(near_plane: f32, far_plane: f32) {
        unsafe { render_set_ortho_clip(near_plane, far_plane) }
    }

    /// This sets the size of the orthographic projection’s viewport. You can use this feature to zoom in and out of the
    /// scene.
    ///
    /// This value only affects orthographic mode projection, which is only available in 2D window modes.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetOrthoSize.html>
    /// * `viewport_height_meters` - The vertical size of the projection’s viewport, in meters.
    ///
    /// see also [`render_set_ortho_size`] [`Renderer::get_ortho_size`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::Renderer;
    ///
    /// let ortho_size = Renderer::get_ortho_size();
    /// assert_eq!(ortho_size, 1.0);
    ///
    /// Renderer::set_ortho_size(12.0);
    /// let ortho_size = Renderer::get_ortho_size();
    /// assert_eq!(ortho_size, 12.0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_ortho_size(view_port_height_meters: f32) {
        unsafe { render_set_ortho_size(view_port_height_meters) }
    }

    /// Gets the root transform of the camera! This will be the identity matrix by default. The user’s head
    /// location will then be relative to this point. This is great to use if you’re trying to do teleportation,
    /// redirected walking, or just shifting the floor around.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CameraRoot.html>
    ///
    /// see also [`render_get_cam_root`]
    /// see example in [`Renderer::camera_root`]
    pub fn get_camera_root() -> Matrix {
        unsafe { render_get_cam_root() }
    }

    /// This retrieves the current near and far clipping planes for the perspective matrix of the primary draw surface.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Renderer/GetClip.html>
    ///
    /// Returns a tuple (`near_plane`, `far_plane`)
    /// * `near_plane` - The GPU discards pixels that are too close to the camera, this is that distance! It will be larger
    ///   than zero, due to the projection math, which also means that numbers too close to zero will produce z-fighting artifacts. This
    ///   has an enforced minimum of 0.001, but will probably be closer to 0.1.
    /// * `far_plane` - At what distance from the camera does the GPU discard pixel? This is not true distance, but rather Z-axis
    ///   distance from zero in View Space coordinates!
    ///
    /// see also [`render_get_clip`]
    /// see example in [`Renderer::set_clip`]
    pub fn get_clip() -> (f32, f32) {
        let mut near_plane = 0.0;
        let mut far_plane = 0.0;
        unsafe { render_get_clip(&mut near_plane, &mut far_plane) }
        (near_plane, far_plane)
    }

    /// Only works for 2D windowed modes! This retrieves the vertical field of view of the camera's projection matrix when in
    /// perspective projection mode.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Renderer/GetFOV.html>
    ///
    /// see also [`render_get_fov`]
    /// see example in [`Renderer::set_fov`]
    pub fn get_fov() -> f32 {
        unsafe { render_get_fov() }
    }

    /// This retrieves the size the primary render surface's view when using orthographic projection mode.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Renderer/GetOrthoSize.html>
    ///
    /// see also [`render_get_ortho_size`] []
    /// see example in [`Renderer::set_ortho_size`]
    pub fn get_ortho_size() -> f32 {
        unsafe { render_get_ortho_size() }
    }

    /// This is the current render layer mask for Mixed Reality Capture, or 2nd person observer rendering. By default,
    /// this is directly linked to Renderer::layer_filter, but this behavior can be overridden via
    /// Renderer::override_capture_filter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CaptureFilter.html>
    ///
    /// see also [`render_get_capture_filter`]
    /// see example in [`Renderer::override_capture_filter`]
    pub fn get_capture_filter() -> RenderLayer {
        unsafe { render_get_capture_filter() }
    }

    /// This is the gamma space color the renderer will clear the screen to when beginning to draw a new frame.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ClearColor.html>
    ///
    /// see also [`render_get_clear_color`]
    /// see example in [`Renderer::clear_color`]
    pub fn get_clear_color() -> Color128 {
        unsafe { render_get_clear_color() }
    }

    /// Enables or disables rendering of the skybox texture! It’s enabled by default on Opaque displays, and completely
    /// unavailable for transparent displays.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/EnableSky.html>
    ///
    /// see also [`render_enabled_skytex`]
    /// see example in [`Renderer::enable_sky`]
    pub fn get_enable_sky() -> bool {
        unsafe { render_enabled_skytex() != 0 }
    }

    /// This tells if capture_filter has been overridden to a specific value via Renderer::override_capture_filter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/HasCaptureFilter.html>
    ///
    /// see also [`render_has_capture_filter`]
    /// see example in [`Renderer::override_capture_filter`]
    pub fn has_capture_filter() -> bool {
        unsafe { render_has_capture_filter() != 0 }
    }

    /// By default, StereoKit renders all first-person layers. This is a bit flag that allows you to change which layers
    /// StereoKit renders for the primary viewpoint. To change what layers a visual is on, use a Draw method that
    /// includes a RenderLayer as a parameter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/LayerFilter.html>
    ///
    /// see also [`render_get_filter`]
    /// see example in [`Renderer::layer_filter`]
    pub fn get_layer_filter() -> RenderLayer {
        unsafe { render_get_filter() }
    }

    /// Get the multisample (MSAA) level of the render surface. Valid values are 1, 2, 4, 8, 16, though
    /// some OpenXR runtimes may clamp this to lower values. Note that while this can greatly smooth out edges, it also
    /// greatly increases RAM usage and fill rate, so use it sparingly. Only works in XR mode. If known in advance, set
    /// this via [`crate::sk::SkSettings`] in initialization. This is a very costly change to make.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Multisample.html>
    ///
    /// see also [`render_get_multisample`]
    /// see example in [`Renderer::multisample`]
    pub fn get_multisample() -> i32 {
        unsafe { render_get_multisample() }
    }

    /// For flatscreen applications only! This allows you to get the camera projection between perspective and
    /// orthographic projection. This may be of interest for some category of UI work, but is generally a niche piece of
    /// functionality.
    /// Swapping between perspective and orthographic will also switch the clipping planes and field of view to the
    /// values associated with that mode. See set_clip/set_fov for perspective, and set_ortho_clip/set_ortho_size for
    /// orthographic.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Projection.html>
    ///
    /// see also [`render_get_projection`]
    /// see example in [`Renderer::projection`]
    pub fn get_projection() -> Projection {
        unsafe { render_get_projection() }
    }

    /// OpenXR has a recommended default for the main render surface, this value allows you to set SK’s surface to a
    /// multiple of the recommended size. Note that the final resolution may also be clamped or quantized. Only works in
    /// XR mode. If known in advance, set this via SKSettings in initialization. This is a very costly change to make.
    /// Consider if viewport_caling will work for you
    /// instead, and prefer that.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Scaling.html>
    ///
    /// see also [`render_get_scaling`]
    /// see example in [`Renderer::scaling`]
    pub fn get_scaling() -> f32 {
        unsafe { render_get_scaling() }
    }

    /// This allows you to trivially scale down the area of the swapchain that StereoKit renders to! This can be used to
    /// boost performance in situations where full resolution is not needed, or to reduce GPU time. This value is
    /// locked to the 0-1 range
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ViewportScaling.html>
    ///
    /// see also [`render_get_viewport_scaling`]
    /// see example in [`Renderer::viewport_scaling`]
    pub fn get_viewport_scaling() -> f32 {
        unsafe { render_get_viewport_scaling() }
    }

    /// Gets the lighting information for the scene! You can build one through SphericalHarmonics::from_lights, or grab
    /// one from [`crate::tex::SHCubemap`].
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    ///
    /// see also [`render_get_skylight`]
    /// see example in [`Renderer::sky_light`]
    pub fn get_sky_light() -> SphericalHarmonics {
        unsafe { render_get_skylight() }
    }

    /// Get the cubemap skybox texture for rendering a background! This is only visible on Opaque displays, since
    /// transparent displays have the real world behind them already! StereoKit has a a default procedurally generated
    /// skybox. You can load one with [`crate::tex::SHCubemap`]. If you’re trying to affect the lighting,
    /// see Renderer::sky_light.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`render_get_skytex`]
    /// see example in [`Renderer::sky_tex`]
    pub fn get_sky_tex() -> Tex {
        let skytex_ptr = unsafe { render_get_skytex() };
        if let Some(nonnull_ptr) = NonNull::new(skytex_ptr) {
            Tex(nonnull_ptr)
        } else {
            // Si render_get_skytex() retourne null, on retourne une texture d'erreur par défaut
            Log::warn("render_get_skytex() returned null, returning error texture");
            Tex::error()
        }
    }

    /// This is the Material that StereoKit is currently using to draw the skybox! It needs a special shader that's
    /// tuned for a full-screen quad. If you just want to change the skybox image, try setting [`Renderer::sky_tex`]
    /// instead.
    ///  
    /// This value will never be null! If you try setting this to null, it will assign SK's built-in default sky
    /// material. If you want to turn off the skybox, see [`Renderer::enable_sky`] instead.
    ///  
    /// Recommended Material settings would be:
    /// - DepthWrite: false
    /// - DepthTest: LessOrEq
    /// - QueueOffset: 100
    ///
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyMaterial.html>
    ///
    /// see also [`render_get_skymaterial`]
    /// see example in [`Renderer::sky_material`]
    pub fn get_sky_material() -> Material {
        Material(NonNull::new(unsafe { render_get_skymaterial() }).expect("Sky material should not be null!"))
    }
}

/// A RenderList is a collection of Draw commands that can be submitted to various surfaces. RenderList.Primary is
/// where all normal Draw calls get added to, and this RenderList is renderer to primary display surface.
///
/// Manually working with a RenderList can be useful for "baking down matrices" or caching a scene of objects. Or
/// for drawing a separate scene to an offscreen surface, like for thumbnails of Models.
/// <https://stereokit.net/Pages/StereoKit/RenderList.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Rect}, model::Model, util::Color128,
///                      tex::{Tex, TexType, TexFormat}, material::Material,
///                      mesh::Mesh, render::{RenderList, RenderClear, RenderBuilder}};
///
/// let model = Model::from_file("plane.glb", None, None).unwrap_or_default().copy();
///
/// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
/// let mut render_mat = Material::unlit().copy();
/// render_mat.diffuse_tex(&render_tex);
///
/// let from = Vec3::new(-3.0, 2.0, 20.9);
/// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 1010.0);
/// let transform_plane = Matrix::r([90.0, 90.0, 145.0]);
/// let transform_cam  = Matrix::look_at(from, Vec3::ZERO, Some(Vec3::new(1.0, 1.0, 1.0)));
///
/// let mut render_list = RenderList::new();
/// render_list.add_model(&model, transform_plane, Color128::WHITE, None);
///
/// let screen = Mesh::screen_quad();
///
/// let render = RenderBuilder::new()
///     .camera(transform_cam)
///     .projection(perspective)
///     .clear(RenderClear::Color)
///     .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
///
/// filename_scr = "screenshots/render_list.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     // The color will change so we redraw every frame
///     render_list.draw_now(&render_tex, &render,
///         Color128::new((iter % 100) as f32 * 0.01, 0.3, 0.2, 0.5));
///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RenderList(pub NonNull<_RenderListT>);

impl Drop for RenderList {
    fn drop(&mut self) {
        unsafe { assets_releaseref_threadsafe(self.0.as_ptr() as *mut c_void) };
    }
}

impl AsRef<RenderList> for RenderList {
    fn as_ref(&self) -> &RenderList {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _RenderListT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type RenderListT = *mut _RenderListT;

/// Controls whether a RenderList holds asset references for the items it contains. Tracked lists are safe to keep
/// around across frames at the cost of an addref/releaseref pair per item.
#[repr(u32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum RenderListRefs {
    /// The list calls addref on each item's mesh/material when added, and releaseref when cleared. This keeps assets
    /// alive for as long as the list holds them, and is the safe default.
    #[default]
    Tracked = 0,
    /// The list does not addref or releaseref its items. The caller is responsible for ensuring referenced assets
    /// remain valid until the list is cleared. Useful for per-frame lists that are filled and drained inside a single
    /// frame.
    None = 1,
}

unsafe extern "C" {
    pub fn render_list_find(id: *const c_char) -> RenderListT;
    pub fn render_list_set_id(render_list: RenderListT, id: *const c_char);
    pub fn render_list_get_id(render_list: RenderListT) -> *const c_char;
    pub fn render_get_primary_list() -> RenderListT;
    pub fn render_list_create(refs: RenderListRefs) -> RenderListT;
    pub fn render_list_addref(list: RenderListT);
    pub fn render_list_release(list: RenderListT);
    pub fn render_list_clear(list: RenderListT);
    pub fn render_list_item_count(list: RenderListT) -> i32;
    pub fn render_list_prev_count(list: RenderListT) -> i32;
    pub fn render_list_add_mesh(
        list: RenderListT,
        mesh: MeshT,
        material: MaterialT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_add_model(
        list: RenderListT,
        model: ModelT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_add_model_mat(
        list: RenderListT,
        model: ModelT,
        material_override: MaterialT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_draw_now(
        list: RenderListT,
        to_rendertarget: TexT,
        in_arr_cameras: *const Matrix,
        in_arr_projections: *const Matrix,
        view_count: i32,
        clear_color: Color128,
        clear: RenderClear,
        viewport_pct: Rect,
        layer_filter: RenderLayer,
        material_variant: i32,
    );
    pub fn render_list_push(list: RenderListT);
    pub fn render_list_pop();

}

impl Default for RenderList {
    fn default() -> Self {
        Self::new()
    }
}

impl IAsset for RenderList {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }

    fn as_asset(&self) -> crate::system::AssetT {
        self.0.as_ptr() as crate::system::AssetT
    }
}

impl RenderList {
    /// Creates a new empty RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/RenderList.html>
    ///
    /// * `refs` - Controls whether the list tracks asset references for the Meshes and Materials added to it. The
    ///   default, `Tracked`, is safe across frames. `None` skips the addref/release pair on each add and clear, but
    ///   the caller must ensure the list is cleared before any referenced asset could be released.
    ///
    /// see also [`render_list_create`] [`RenderList::new_with`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,material::Material, mesh::Mesh,
    ///                      render::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new() -> Self {
        RenderList(
            NonNull::new(unsafe { render_list_create(RenderListRefs::Tracked) }).expect("RenderList::new should work"),
        )
    }

    /// Creates a new empty RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/RenderList.html>
    ///
    /// * `refs` - Controls whether the list tracks asset references for the Meshes and Materials added to it. The
    ///   default, `Tracked`, is safe across frames. `None` skips the addref/release pair on each add and clear, but
    ///   the caller must ensure the list is cleared before any referenced asset could be released.
    ///
    /// see also [`render_list_create`] [`RenderList::new`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,material::Material, mesh::Mesh,
    ///                      render::{RenderList, RenderListRefs}};
    ///
    /// let mut render_list = RenderList::new_with(RenderListRefs::None);
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new_with(refs: RenderListRefs) -> Self {
        RenderList(NonNull::new(unsafe { render_list_create(refs) }).expect("RenderList::new_with should work"))
    }

    /// Looks for a RenderList matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Find.html>
    /// * `id` - The id of the RenderList we are looking for.
    ///
    /// see also [`render_list_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::RenderList;
    ///
    /// let mut render_list = RenderList::new();
    /// render_list.id("my_render_list");
    ///
    /// let same_list = RenderList::find("my_render_list").expect("my_render_list should be found");
    /// assert_eq!(render_list, same_list);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<RenderList, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        let render_list = NonNull::new(unsafe { render_list_find(c_str.as_ptr()) });
        match render_list {
            Some(render_list) => Ok(RenderList(render_list)),
            None => Err(StereoKitError::RenderListFind(id.as_ref().to_owned(), "not found".to_owned())),
        }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Find.html>
    ///
    /// see also [`render_list_find`] [`RenderList::find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::RenderList;
    ///
    /// let render_list = RenderList::new();
    ///
    /// let same_list = render_list.clone_ref();
    /// assert_eq!(render_list, same_list);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> RenderList {
        RenderList(
            NonNull::new(unsafe { render_list_find(render_list_get_id(self.0.as_ptr())) })
                .expect("<asset>::clone_ref failed!"),
        )
    }

    /// sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Id.html>
    ///
    /// see also [`render_list_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::RenderList;
    ///
    /// let mut render_list = RenderList::new();
    /// render_list.id("my_render_list");
    ///
    /// assert_eq!(render_list.get_id(), "my_render_list");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { render_list_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// Clears out and de-references all Draw items currently in the RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Clear.html>
    ///
    /// see also [`render_list_clear`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.clear();
    /// assert_eq!(render_list.get_count(), 0);
    /// assert_eq!(render_list.get_prev_count(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clear(&mut self) -> &mut Self {
        unsafe { render_list_clear(self.0.as_ptr()) }
        self
    }

    /// Add a Mesh/Material to the RenderList. The RenderList will hold a reference to these Assets until the list is
    /// cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * `mesh` - A valid Mesh you wish to draw.
    /// * `material` - A Material to apply to the Mesh.
    /// * `transform` - A transformation Matrix relative to the current Hierarchy.
    /// * `colorLinear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    ///   per-instance data for the shader!
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    ///   head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`render_list_add_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect},  util::{named_colors, Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render::{RenderBuilder, RenderList, RenderClear, RenderLayer}};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr();
    ///
    /// let at = Vec3::new(-2.0, 1.0, 2.0);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 120.0);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, None);
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY, named_colors::CYAN, None)
    ///     .add_mesh(&cylinder2, &cylinder_mat, Matrix::IDENTITY, named_colors::FUCHSIA,
    ///               Some(RenderLayer::Layer1));
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::screen_quad();
    /// filename_scr = "screenshots/render_list_add_mesh.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         let render = RenderBuilder::new()
    ///             .camera(transform_cam)
    ///             .projection(perspective)
    ///             .layer_filter(RenderLayer::AllThirdPerson)
    ///             .clear(RenderClear::Color)
    ///             .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
    ///         render_list.draw_now(&render_tex, &render, Color128::new(0.99, 0.3, 0.2, 0.5));
    ///     }
    ///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_mesh.jpeg" alt="screenshot" width="200">
    pub fn add_mesh(
        &mut self,
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) -> &mut Self {
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_list_add_mesh(
                self.0.as_ptr(),
                mesh.as_ref().0.as_ptr(),
                material.as_ref().0.as_ptr(),
                transform.into(),
                color_linear.into(),
                layer,
            )
        }
        self
    }

    /// Add a Model to the RenderList. The RenderList will hold a reference to these Assets until the list is cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * `model` - A valid Model you wish to draw.
    /// * `material` - Allows you to override the Material.
    /// * `transform` - A transformation Matrix relative to the current Hierarchy.
    /// * `colorLinear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    ///   per-instance data for the shader!
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    ///   head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`render_list_add_model`] [`RenderList::add_model_with_material`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect}, model::Model, util::{named_colors,Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render::{RenderBuilder, RenderList,RenderClear, RenderLayer}};
    ///
    /// let model = Model::from_file("plane.glb", None, None).unwrap_or_default().copy();
    ///
    /// let at = Vec3::new(-2.0, 8.0, 20.9);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 1550.0);
    /// let transform_plane1 = Matrix::t([ 5.0, 2.0, 0.0]);
    /// let transform_plane2 = Matrix::t([2.0, -6.0, -10.0]);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, Some(Vec3::new(0.0, -1.0, 0.0)));
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_model(&model, transform_plane1, named_colors::RED, None)
    ///     .add_model(&model, transform_plane2, named_colors::BLUE,
    ///                Some(RenderLayer::Layer1));
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::screen_quad();
    ///
    /// filename_scr = "screenshots/render_list_add_model.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         let render = RenderBuilder::new()
    ///             .camera(transform_cam)
    ///             .projection(perspective)
    ///             .layer_filter(RenderLayer::AllFirstPerson)
    ///             .clear(RenderClear::Color)
    ///             .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
    ///         render_list.draw_now(&render_tex, &render, Color128::new(0.0, 0.3, 0.2, 0.5));
    ///     }
    ///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_model.jpeg" alt="screenshot" width="200">
    pub fn add_model(
        &mut self,
        model: impl AsRef<Model>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) -> &mut Self {
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_list_add_model(
                self.0.as_ptr(),
                model.as_ref().0.as_ptr(),
                transform.into(),
                color_linear.into(),
                layer,
            )
        }
        self
    }

    /// Add a Model to the RenderList, overriding all its materials with the given one. The RenderList will hold a
    /// reference to these Assets until the list is cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * `model` - A valid Model you wish to draw.
    /// * `material_override` - The material that will override all materials of this model.
    /// * `transform` - A transformation Matrix relative to the current Hierarchy.
    /// * `colorLinear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    ///   per-instance data for the shader!
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    ///   head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`render_list_add_model_mat`] [`RenderList::add_model`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect}, model::Model, util::{named_colors,Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render::{RenderBuilder, RenderList,RenderClear, RenderLayer}};
    ///
    /// let model = Model::from_file("plane.glb", None, None).unwrap_or_default().copy();
    /// let material_override = Material::pbr().copy();
    ///
    /// let at = Vec3::new(-2.0, 8.0, 20.9);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 1550.0);
    /// let transform_plane1 = Matrix::t([ 5.0, 2.0, 0.0]);
    /// let transform_plane2 = Matrix::t([2.0, -6.0, -10.0]);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, Some(Vec3::new(0.0, -1.0, 0.0)));
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_model_with_material(&model, &material_override, transform_plane1, named_colors::RED, None)
    ///     .add_model_with_material(&model, &material_override, transform_plane2, named_colors::BLUE,
    ///                Some(RenderLayer::Layer1));
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::screen_quad();
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         let render = RenderBuilder::new()
    ///             .camera(transform_cam)
    ///             .projection(perspective)
    ///             .layer_filter(RenderLayer::AllFirstPerson)
    ///             .clear(RenderClear::Color)
    ///             .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
    ///         render_list.draw_now(&render_tex, &render, Color128::new(0.0, 0.3, 0.2, 0.5));
    ///     }
    ///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add_model_with_material(
        &mut self,
        model: impl AsRef<Model>,
        material_override: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) -> &mut Self {
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_list_add_model_mat(
                self.0.as_ptr(),
                model.as_ref().0.as_ptr(),
                material_override.as_ref().0.as_ptr(),
                transform.into(),
                color_linear.into(),
                layer,
            )
        }
        self
    }

    /// Draws the RenderList to a rendertarget texture immediately. It does _not_ clear the list.
    /// The camera(s), projection(s), layer filter, clear behavior and viewport are all provided by a [`RenderBuilder`].
    /// Passing a builder with a single camera is equivalent to the old single-view `draw_now`, while passing one with
    /// N cameras performs the multi-view render (see [`RenderList::draw_now`]-style usage).
    /// <https://stereokit.net/Pages/StereoKit/RenderList/DrawNow.html>
    /// * `to_render_target` - The rendertarget texture to draw to.
    /// * `render` - A [`RenderBuilder`] describing the camera(s), projection(s), layer filter, clear behavior and
    ///   viewport to use for this draw.
    /// * `clear_color` - If `clear` (set on `render`) clears the color of `to_render_target`, then this is the color
    ///   it will clear to.
    ///
    /// see also [`render_list_draw_now`] [`RenderBuilder::draw_now`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect},  util::{named_colors, Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render::{RenderList, RenderBuilder, RenderClear}};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr().copy();
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// let transform_screen = Matrix::t([0.0, 0.0, -1.0]);
    ///
    /// let at = Vec3::new(-1.0, 0.0, 1.0);
    /// let orthographic = Matrix::orthographic(1.5, 1.5, 0.01, 120.0);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, None);
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY, named_colors::CYAN, None)
    ///     .add_mesh(&cylinder2, &cylinder_mat, Matrix::IDENTITY, named_colors::FUCHSIA,None)
    ///     .add_mesh(&screen,    &render_mat,   transform_screen, named_colors::GRAY, None);
    ///
    /// let render = RenderBuilder::new()
    ///     .camera(transform_cam)
    ///     .projection(orthographic)
    ///     .clear(RenderClear::None)
    ///     .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
    ///
    /// filename_scr = "screenshots/render_list_draw_now.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
    ///     render_list.draw_now(&render_tex, &render, Color128::BLACK_TRANSPARENT);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_draw_now.jpeg" alt="screenshot" width="200">
    pub fn draw_now(&mut self, to_rendertarget: impl AsRef<Tex>, render: &RenderBuilder, clear_color: Color128) {
        render.draw_now(self, to_rendertarget, clear_color);
    }

    /// The default RenderList used by the Renderer for the primary display surface.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Primary.html>
    ///
    /// see also [`render_get_primary_list`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render::RenderList;
    ///
    /// let primary_list = RenderList::primary();
    /// assert_eq!   (primary_list.get_id(),"sk/render/primary_renderlist");
    /// assert_eq!   (primary_list.get_count(), 0);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn primary() -> Self {
        RenderList(NonNull::new(unsafe { render_get_primary_list() }).expect("RenderList::primary should work!"))
    }

    /// All draw calls that don't specify a render list will get submitted to the active RenderList at the top of the
    /// stack. By default, that's RenderList.Primary, but you can push your own list onto the stack here to capture draw
    /// calls, like those done in the UI.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Push.html>
    ///
    /// see also [`render_list_push`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::named_colors, material::Material,
    ///                      mesh::Mesh, render::RenderList};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr().copy();
    ///
    /// let mut render_list = RenderList::new();
    /// // render_list.add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY,  None);
    ///
    /// filename_scr = "screenshots/render_list_push.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     render_list.push();
    ///     cylinder1.draw(&cylinder_mat, Matrix::IDENTITY, Some(named_colors::GOLD.into()), None);
    ///     RenderList::pop();
    ///     cylinder2.draw(&cylinder_mat, Matrix::IDENTITY, Some(named_colors::RED.into()), None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_push.jpeg" alt="screenshot" width="200">
    pub fn push(&mut self) {
        unsafe { render_list_push(self.0.as_ptr()) }
    }

    /// This removes the current top of the RenderList stack, making the next list as active
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Pop.html>
    ///
    /// see also [`render_list_pop`]
    /// see example [`RenderList::push`]
    pub fn pop() {
        unsafe { render_list_pop() }
    }

    /// The id of this render list
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Id.html>
    ///
    /// see also [`render_list_get_id`]
    /// see example [`RenderList::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(render_list_get_id(self.0.as_ptr())) }.to_str().unwrap_or_default()
    }

    /// The number of Mesh/Material pairs that have been submitted to the render list so far this frame.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Count.html>
    ///
    /// see also [`render_list_item_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { render_list_item_count(self.0.as_ptr()) }
    }

    /// This is the number of items in the RenderList before it was most recently cleared. If this is a list that is
    /// drawn and cleared each frame, you can think of this as "last frame's count".
    /// <https://stereokit.net/Pages/StereoKit/RenderList/PrevCount.html>
    ///
    /// see also [`render_list_prev_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.clear();
    /// assert_eq!(render_list.get_count(), 0);
    /// assert_eq!(render_list.get_prev_count(), 1);
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_prev_count(&self) -> i32 {
        unsafe { render_list_prev_count(self.0.as_ptr()) }
    }
}

// ... existing code ...

/// Stateful builder for render calls.
///
/// Configure properties with setters, then execute a specific operation with one of the render methods:
/// [`RenderBuilder::render_to`] [`RenderBuilder::draw_now`] [`RenderBuilder::screenshot`]
/// /// StereoKit original docs:
/// <https://stereokit.net/Pages/StereoKit/Renderer/RenderTo.html>
/// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
/// <https://stereokit.net/Pages/StereoKit/RenderList/DrawNow.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{render::{RenderBuilder, Renderer, RenderClear, RenderLayer, RenderList},
///                      maths::{Vec3, Quat, Matrix}, tex::{Tex, TexFormat}, sprite::Sprite, system::Pivot,
///                      mesh::Mesh, material::Material, util::{named_colors, Color128}};
///
/// let sun = Mesh::generate_sphere(2.0, None);
/// let material_sun = Material::pbr();
/// let transform_sun = Matrix::t([-0.0, 1.0, -4.0]);
///
/// let tex1 = Tex::render_target(200, 200, None, TexFormat::Rgba32Srgb, TexFormat::Depth16)
///                    .expect("RenderTarget should be created");
/// let sprite1 = Sprite::from_tex(&tex1, None, None).unwrap_or_default();
/// let tex2 = Tex::render_target(200, 200, None, TexFormat::Rgba32Srgb, TexFormat::Depth16)
///                    .expect("RenderTarget should be created");
/// let sprite2 = Sprite::from_tex(&tex2, None, None).unwrap_or_default();
///
/// let transform_sprite1 = Matrix::t_r([-0.9, -0.15, 0.0], Quat::Y_180);
/// let transform_sprite2 = Matrix::t_r([0.9, 0.7, -0.30], Quat::Y_180);
///
/// let camera = Matrix::t_r(Vec3::Z * 1.0, Quat::look_at(Vec3::Z, Vec3::ZERO, None));
/// let projection = Matrix::perspective(90.0, 1.0, 0.1, 50.0);
///
/// // A RenderList for draw_now.
/// let mut render_list = RenderList::new();
///
/// // Configure the builder once, before the main loop.
/// let render = RenderBuilder::new()
///     .camera(camera)
///     .projection(projection)
///     .layer_filter(RenderLayer::All)
///     .clear(RenderClear::All);
///
/// filename_scr = "screenshots/render_builder.jpeg";
/// number_of_steps = 30;
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     Renderer::add_mesh(&sun, &material_sun, transform_sun,
///         Some(named_colors::RED.into()), None);
///
///     // To not have to flip read_tex/write_tex we render at different steps
///     if iter < number_of_steps - 2 {
///         // 1 - screenshot
///         render.screenshot(move |_pixels: &[u8], _format: TexFormat, _w: usize, _h: usize| {},
///                           0, 200, 200, TexFormat::Rgba32Srgb);
///
///         // 2 - draw_now
///         render_list.add_mesh(&sun, &material_sun, transform_sun, named_colors::GOLD, None);
///         render.draw_now(&mut render_list, &tex2, Color128::WHITE);
///     } else if iter == number_of_steps -2 {
///         sprite2.draw(transform_sprite2, Pivot::Center, None, None);
///         // 2 - render_to:
///         render.render_to(&tex1, 0);
///     } else { // Say 'cheese'!
///         sprite1.draw(transform_sprite1, Pivot::TopLeft, None, None);
///         sprite2.draw(transform_sprite2, Pivot::Center, None, None);
///     }
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_builder.jpeg" alt="screenshot" width="200">
#[derive(Debug, Clone)]
#[must_use = "RenderBuilder does nothing until you call .render_to() .draw_now() or .screenshot() on it"]
pub struct RenderBuilder {
    cameras: Vec<Matrix>,
    projections: Vec<Matrix>,
    layer_filter: RenderLayer,
    material_variant: i32,
    clear: RenderClear,
    viewport_pct: Rect,
}

impl Default for RenderBuilder {
    fn default() -> Self {
        Self {
            cameras: vec![Matrix::IDENTITY],
            projections: vec![Matrix::IDENTITY],
            layer_filter: RenderLayer::All,
            material_variant: 0,
            clear: RenderClear::All,
            viewport_pct: Rect::default(),
        }
    }
}

impl RenderBuilder {
    /// Creates a new render builder with defaults matching renderer defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a single camera matrix which is a A TRS matrix representing the location and orientation of the camera.
    /// This matrix gets inverted later on, so no need to do it yourself.
    pub fn camera(mut self, camera: impl Into<Matrix>) -> Self {
        self.cameras = vec![camera.into()];
        self
    }

    /// Sets multiple camera matrices for multi-view rendering. One TRS matrix per view. Each matrix gets inverted
    /// internally. The length must equal projections.len() and is capped at 6.
    pub fn cameras(mut self, cameras: Vec<Matrix>) -> Self {
        self.cameras = cameras;
        self
    }

    /// Change the cameras of this builder in case you want to keep the Builder alive(ie: as a IStepper propery)
    pub fn update_cameras(&mut self, cameras: Vec<Matrix>) -> &mut Self {
        self.cameras = cameras;
        self
    }

    /// Sets a single projection matrix.
    pub fn projection(mut self, projection: impl Into<Matrix>) -> Self {
        self.projections = vec![projection.into()];
        self
    }

    /// Sets multiple projection matrices for multi-view rendering. The length must equal cameras.len()
    pub fn projections(mut self, projections: &[Matrix]) -> Self {
        self.projections = projections.to_vec();
        self
    }

    /// This is a bit flag that allows you to change which layers StereoKit renders for this particular render
    /// viewpoint. To change what layers a visual is on, use a Draw method that includes a RenderLayer as a parameter.
    /// If None has default value of [`RenderLayer::All`]
    pub fn layer_filter(mut self, layer_filter: RenderLayer) -> Self {
        self.layer_filter = layer_filter;
        self
    }

    /// Specifies which Material variant should be used for rendering. 0 will be the normal default material, any
    /// others will generally be application-defined by setting up each Material’s Variant with specific shaders. If a
    /// [`Material`] has no corresponding variant, it will not be drawn.
    pub fn material_variant(mut self, material_variant: i32) -> Self {
        self.material_variant = material_variant;
        self
    }

    /// Describes if and how the rendertarget should be cleared before rendering. Note that clearing the target is
    /// unaffected by the viewport, so this will clean the entire surface! Default value is [`RenderClear::All`]
    pub fn clear(mut self, clear: RenderClear) -> Self {
        self.clear = clear;
        self
    }

    /// Allows you to specify a region of the rendertarget to draw to! This is in normalized coordinates, 0-1. If the
    /// width of this value is zero, then this will render to the entire texture same as default value which is
    /// [0.0, 0.0, 1.0, 1.0].
    pub fn viewport(mut self, viewport: Rect) -> Self {
        self.viewport_pct = viewport;
        self
    }

    /// Queues a single render pass that draws the active list into 1 or N views at once, with one camera + projection
    /// per view, writing into N consecutive layers of an array rendertarget. The number of views is capped at 6.
    /// This is queued for the next pipeline frame.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/RenderTo.html>
    /// * `to_render_target` - An array or cubemap rendertarget with at least `cameras.len()` layers.
    /// * `to_target_index` - Index of the render target's array texture we want to draw to. 0 for single view.
    ///
    /// see also [`render_to`] [`Renderer::render_to`]
    /// ### Examples for multi-view rendering, simply build a [`RenderBuilder`] with N cameras and N projections:
    /// TODO: This multi-view do not render neither the sphere nor the quad (only the skydome).
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::{Renderer, RenderBuilder, RenderLayer, RenderClear},
    ///                      tex::{Tex, TexType, TexFormat}, material::{Material, Cull},
    ///                      mesh::Mesh, util::named_colors, maths::{Vec3, Quat, Matrix}};
    ///
    /// let sphere = Mesh::generate_sphere(1.0, None);
    /// let mut material = Material::pbr().copy();
    /// material.color_tint(named_colors::CYAN).face_cull(Cull::None);
    /// let transform_sphere = Matrix::t_s([0.0, -0.55, -0.30], [1.0, 1.0, 1.0]);
    ///
    /// let camera1 = Matrix::t_r(Vec3::Z * 3.0, Quat::look_at(Vec3::NEG_Z , Vec3::ZERO, None));
    /// let camera2 = Matrix::t_r(Vec3::Z * 3.0 , Quat::look_at(Vec3::NEG_Z, Vec3::ZERO, None));
    /// let projection = Matrix::perspective(150.0, 1.0, 0.1, 50.0);
    ///
    /// // Create a render target texture with array support
    /// let tex_type = TexType::ImageNomips | TexType::Rendertarget;
    /// let mut tex_a = Tex::new(tex_type, TexFormat::Rgba32Srgb, Some("render_tex_array_a"));
    /// tex_a.set_size(800, 800, Some(2), None);
    /// let mut tex_b = Tex::new(tex_type, TexFormat::Rgba32Srgb, Some("render_tex_array_b"));
    /// tex_b.set_size(800, 800, Some(2), None);
    ///
    /// let quad = Mesh::screen_quad();
    /// let mut material_quad = Material::from_file("shaders/stereo_array.hlsl.sks", None)
    ///                             .expect("stereo_array.hlsl.sks should be present");
    /// // Transforms to place the two quads side by side in the scene
    /// let transform_quad = Matrix::t_s([-0.0, 0.45, 0.12], [0.45, 0.45, 0.45]);
    ///
    /// let render = RenderBuilder::new()
    ///     .cameras(vec![camera1, camera2])
    ///     .projections(&[projection, projection])
    ///     .layer_filter(RenderLayer::All)
    ///     .clear(RenderClear::All);
    ///
    /// filename_scr = "screenshots/render_to_multiview.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Renderer::add_mesh(&sphere, &material, transform_sphere, None, None);
    ///
    ///     let (read_tex, write_tex) = if iter % 2 == 0 {
    ///         (&tex_a, &tex_b)
    ///     } else {
    ///         (&tex_b, &tex_a)
    ///     };
    ///     material_quad.diffuse_tex(&read_tex);
    ///
    ///     if iter % 2 == 0 { // 2 syntaxes for the same job:
    ///         Renderer::render_to(&write_tex, 0, &render);
    ///     } else {
    ///         render.render_to(&write_tex, 0)
    ///     }
    ///     // Display array slice 0 and slice 1 of the render target as screen quads
    ///     if iter != 0 {quad.draw(&material_quad, transform_quad, None, None);}
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_to_multiview.jpeg" alt="screenshot" width="200">
    pub fn render_to(&self, to_render_target: impl AsRef<Tex>, to_target_index: i32) {
        unsafe {
            render_to(
                to_render_target.as_ref().0.as_ptr(),
                to_target_index,
                self.cameras.as_ptr(),
                self.projections.as_ptr(),
                self.cameras.len() as i32,
                self.layer_filter,
                self.material_variant,
                self.clear,
                self.viewport_pct,
            )
        }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given position at the given
    /// point, with a resolution the same size as the screen’s surface. This overload allows for retrieval of the color
    /// data directly from the render thread! You can use the color data directly by saving/processing it inside your
    /// callback, or you can keep the data alive for as long as it is referenced.
    /// [`RenderBuilder::material_variant`] is not used here as we want a screenshot.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * `on_screenshot` - closure |&[u8], TexFormat, width:usize, height:usize|
    /// * `camera_index` - Index of the camera/projection pair to render from. 0 in most of the case.
    /// * `width` - Size of the screenshot horizontally, in pixels.
    /// * `height`- Size of the screenshot vertically, in pixels
    /// * `render_layer` - This is a bit flag that allows you to change which layers StereoKit renders for this
    ///   particular render viewpoint. To change what layers a visual is on, use a Draw method that includes a
    ///   RenderLayer as a parameter. If None will use default value of All
    /// * `tex_format` - The pixel format of the color data. If None will use default value of TexFormat::Rgba32Srgb
    ///
    /// see also [`render_screenshot_viewpoint`] [`Renderer::screenshot_capture`] [`Renderer::screenshot`]
    pub fn screenshot<F: FnMut(&[u8], TexFormat, usize, usize)>(
        &self,
        mut on_screenshot: F,
        camera_index: usize,
        width: i32,
        height: i32,
        tex_format: TexFormat,
    ) {
        let mut closure = &mut on_screenshot;
        unsafe {
            render_screenshot_viewpoint(
                Some(sc_capture_trampoline::<F>),
                self.cameras[camera_index],
                self.projections[camera_index],
                width,
                height,
                self.layer_filter,
                self.clear,
                self.viewport_pct,
                tex_format,
                &mut closure as *mut _ as *mut c_void,
            )
        }
    }

    /// Renders the list once across single or multiple views in a single pass, with one camera +
    /// projection per view. Each view writes to its corresponding layer of the (array) render target. The number of
    /// views is capped at 6.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/DrawNow.html>
    /// * `render_list` - The list of stuff to draw.
    /// * `to_render_target` - An array or cubemap rendertarget with at least `cameras.len()` layers.
    /// * `clear_color` - If `clear` clears color, this is the color used. Default is transparent black.
    ///
    /// see also [`RenderList::draw_now`] [`render_list_draw_now`]
    /// ### Examples for multi-view rendering, simply build a [`RenderBuilder`] with N cameras and N projections:
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect},  util::{named_colors, Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render::{RenderList, RenderBuilder, RenderClear}};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr().copy();
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::Rgba32Srgb);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// let transform_screen = Matrix::t([0.0, 0.0, -1.0]);
    ///
    /// let at = Vec3::new(-1.0, 0.0, 1.0);
    /// let orthographic = Matrix::orthographic(1.5, 1.5, 0.01, 120.0);
    /// let transform_cam1  = Matrix::look_at(at, [0.0, 0.0, 0.0], None);
    /// let transform_cam2  = Matrix::look_at(at, [0.1, 0.0, 0.0], None);
    /// let cameras = [transform_cam1, transform_cam2];
    /// let projections = [orthographic; 2];
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY, named_colors::RED, None)
    ///     .add_mesh(&cylinder2, &cylinder_mat, Matrix::IDENTITY, named_colors::GREEN,None)
    ///     .add_mesh(&screen,    &render_mat,   transform_screen, named_colors::GRAY, None);
    ///
    /// let render = RenderBuilder::new()
    ///     .cameras(cameras.to_vec())
    ///     .projections(&projections)
    ///     .clear(RenderClear::None)
    ///     .viewport(Rect::new(0.0, 0.0, 1.0, 1.0));
    ///
    /// filename_scr = "screenshots/render_list_draw_now_multi_view.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     screen.draw(&render_mat, Matrix::IDENTITY, None, None);
    ///     if iter % 2 == 0 { // 2 syntaxes for the same job:
    ///         render_list.draw_now(&render_tex, &render, Color128::BLACK_TRANSPARENT);
    ///     } else {
    ///         render.draw_now(&render_list, &render_tex, Color128::WHITE)
    ///     }
    ///     
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_draw_now_multi_view.jpeg" alt="screenshot" width="200">
    pub fn draw_now(
        &self,
        render_list: &RenderList,
        to_rendertarget: impl AsRef<Tex>,
        clear_color: impl Into<Color128>,
    ) {
        unsafe {
            render_list_draw_now(
                render_list.0.as_ptr(),
                to_rendertarget.as_ref().0.as_ptr(),
                self.cameras.as_ptr(),
                self.projections.as_ptr(),
                self.cameras.len() as i32,
                clear_color.into(),
                self.clear,
                self.viewport_pct,
                self.layer_filter,
                self.material_variant,
            )
        }
    }
}
