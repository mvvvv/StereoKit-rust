//! XR_KHR_android_surface_swapchain extension implementation
//!
//! **This is a rust adaptation of <https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tools/XrCompLayers.cs>**
//!
//! <https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_KHR_android_surface_swapchain>

use crate::{
    maths::{Pose, Quat, Rect, Vec2},
    prelude::*,
    system::{Backend, BackendGraphics, BackendOpenXR, BackendXRType},
    tex::{Tex, TexFormat, TexType},
};
#[cfg(target_os = "android")]
use openxr_sys::{pfn::CreateSwapchainAndroidSurfaceKHR, platform::jobject};

use openxr_sys::{
    Duration, Handle, Session, Space, Swapchain, SwapchainImageWaitInfo,
    pfn::{
        AcquireSwapchainImage, CreateSwapchain, DestroySwapchain, EnumerateSwapchainImages, ReleaseSwapchainImage,
        WaitSwapchainImage,
    },
};

use openxr_sys::{
    CompositionLayerFlags, CompositionLayerQuad, Extent2Df, Extent2Di, EyeVisibility, Offset2Di, Posef, Quaternionf,
    Rect2Di, Result as XrResult, StructureType, SwapchainCreateFlags, SwapchainCreateInfo, SwapchainSubImage,
    SwapchainUsageFlags, Vector3f,
};

use std::ptr::null_mut;

#[derive(Debug)]

/// `XrCompLayers` provides low-level OpenXR composition layer functionality, while `SwapchainSk`
/// offers a high-level wrapper for creating and managing OpenXR swapchains with StereoKit integration.
///
/// ### Examples
/// ## Basic Usage with SwapchainSk
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ maths::{Vec3, Matrix, Pose, Vec2, Rect},  render_list::RenderList,
///     util::{named_colors, Color128, Time}, tex::TexFormat, material::Material, mesh::Mesh,
///     system::{Backend, BackendXRType, RenderClear}, tools::xr_comp_layers::* };
///
/// use openxr_sys::SwapchainUsageFlags;
///
/// // Check if OpenXR is available
/// if Backend::xr_type() == BackendXRType::OpenXR {
///     // Create XrCompLayers instance
///     if let Some(mut swapchain) = SwapchainSk::new(
///             TexFormat::RGBA32, 512, 512, None ){
///         
///         // Set up rendering components
///         let mut render_list = RenderList::new();
///         let mut material = Material::default().copy();
///         let projection = Matrix::orthographic(0.2, 0.2, 0.01, 10.0);
///         
///         // Add a sphere to the scene
///         render_list.add_mesh( Mesh::sphere(), material, Matrix::s(0.05 * Vec3::ONE),
///                               named_colors::WHITE, None);
///         
///         // Render loop example
///         test_steps!( // !!!! Get a proper main loop !!!!
///             // Acquire the next swapchain image
///             if let Ok(_image_index) = swapchain.acquire_image(None) {
///                 // Get the render target texture
///                 if let Some(render_tex) = swapchain.get_render_target() {
///                     // Render to the swapchain texture
///                     render_list.draw_now(
///                         render_tex,
///                         Matrix::look_at(Vec3::angle_xy(Time::get_totalf() * 90.0, 0.0), Vec3::ZERO, None),
///                         projection,
///                         Some(Color128::new(0.4, 0.3, 0.2, 1.0)),
///                         Some(RenderClear::Color),
///                         Rect::new(0.0, 0.0, 1.0, 1.0),
///                         None, None,
///                     );
///                 }
///                 
///                 // Release the image back to the swapchain
///                 swapchain.release_image().expect("Failed to release image");
///                 
///                 // Submit the quad layer to OpenXR
///                 XrCompLayers::submit_quad_layer(
///                     Pose::new(Vec3::new(0.0, 1.5, -1.0), None), // World position
///                     Vec2::new(0.3, 0.3),                        // Quad size
///                     swapchain.handle,                           // Swapchain handle
///                     Rect::new(0.0, 0.0, 512.0, 512.0),          // Texture rectangle
///                     0,                                          // Array index
///                     1,                                          // Sort order
///                     None,                                       // Eye visibility
///                     None,                                       // XR space
///                 );
///             }
///         );
///         
///         // Clean up
///         swapchain.destroy();
///     }
/// }
/// ```
///
pub struct XrCompLayers {
    // OpenXR function pointers
    #[cfg(target_os = "android")]
    xr_create_swapchain_android: Option<CreateSwapchainAndroidSurfaceKHR>,
    xr_create_swapchain: Option<CreateSwapchain>,
    xr_destroy_swapchain: Option<DestroySwapchain>,
    xr_enumerate_swaptchain_images: Option<EnumerateSwapchainImages>,
    xr_acquire_swapchain_image: Option<AcquireSwapchainImage>,
    xr_wait_swaptchain_image: Option<WaitSwapchainImage>,
    xr_release_swaptchain_image: Option<ReleaseSwapchainImage>,
}

impl Default for XrCompLayers {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "android")]
            xr_create_swapchain_android: BackendOpenXR::get_function::<CreateSwapchainAndroidSurfaceKHR>(
                "xrCreateSwapchainAndroidSurfaceKHR",
            ),
            //#[cfg(not(target_os = "android"))]
            xr_create_swapchain: BackendOpenXR::get_function::<CreateSwapchain>("xrCreateSwapchain"),

            xr_destroy_swapchain: BackendOpenXR::get_function::<DestroySwapchain>("xrDestroySwapchain"),
            xr_enumerate_swaptchain_images: BackendOpenXR::get_function::<EnumerateSwapchainImages>(
                "xrEnumerateSwapchainImages",
            ),
            xr_acquire_swapchain_image: BackendOpenXR::get_function::<AcquireSwapchainImage>("xrAcquireSwapchainImage"),
            xr_wait_swaptchain_image: BackendOpenXR::get_function::<WaitSwapchainImage>("xrWaitSwapchainImage"),
            xr_release_swaptchain_image: BackendOpenXR::get_function::<ReleaseSwapchainImage>(
                "xrReleaseSwapchainImage",
            ),
        }
    }
}

impl XrCompLayers {
    /// Initializes the composition layers helper by loading required OpenXR bindings.
    /// Returns `Some(Self)` if the XR runtime type is OpenXR and all bindings are present.
    pub fn new() -> Option<Self> {
        let this = Self::default();

        #[cfg(target_os = "android")]
        {
            if !BackendOpenXR::ext_enabled("XR_KHR_android_surface_swapchain") {
                Log::warn(
                    "XrCompLayers: XR_KHR_android_surface_swapchain extension is not enabled. You have to enable it before sk::init.",
                );
                return None;
            }
        }

        if !(Backend::xr_type() == BackendXRType::OpenXR && this.load_bindings()) {
            Log::warn(format!("❌ XrCompLayers: some function bindings are missing : {this:?}"));
            return None;
        }

        Some(this)
    }

    /// Checks presence of all required swapchain function bindings.
    fn load_bindings(&self) -> bool {
        let mut swapchain_func_present = true;
        swapchain_func_present &= self.xr_create_swapchain.is_some();

        #[cfg(target_os = "android")]
        {
            swapchain_func_present &= self.xr_create_swapchain_android.is_some();
        }

        swapchain_func_present
            && self.xr_destroy_swapchain.is_some()
            && self.xr_enumerate_swaptchain_images.is_some()
            && self.xr_acquire_swapchain_image.is_some()
            && self.xr_wait_swaptchain_image.is_some()
            && self.xr_release_swaptchain_image.is_some()
    }

    /// Convert a StereoKit `TexFormat` into the corresponding native OpenXR format value.
    pub fn to_native_format(format: TexFormat) -> i64 {
        match Backend::graphics() {
            BackendGraphics::D3D11 => match format {
                TexFormat::RGBA32 => 29,
                TexFormat::RGBA32Linear => 28,
                TexFormat::BGRA32 => 91,
                TexFormat::BGRA32Linear => 87,
                TexFormat::RGB10A2 => 24,
                TexFormat::RG11B10 => 26,
                _ => panic!("Unsupported texture format"),
            },
            BackendGraphics::OpenGLESEGL => match format {
                TexFormat::RGBA32 => 0x8C43,
                TexFormat::RGBA32Linear => 0x8058,
                TexFormat::RGB10A2 => 0x8059,
                TexFormat::RG11B10 => 0x8C3A,
                _ => panic!("Unsupported texture format"),
            },
            _ => panic!("Unsupported graphics backend"),
        }
    }

    /// Submit a quad layer to the OpenXR composition.
    ///
    /// This method submits a rendered quad layer to the OpenXR compositor, which will
    /// be displayed in the XR environment. The quad appears as a 2D surface in 3D space.
    ///
    /// # Parameters
    /// - `world_pose`: Pose of the quad in world space (position and orientation).
    /// - `size`: Dimensions of the quad in meters.
    /// - `swapchain`: Swapchain handle to sample the texture from.
    /// - `swapchain_rect`: Texture rectangle within the swapchain image (in pixel coordinates).
    /// - `swapchain_array_index`: Array slice index for texture arrays (usually 0).
    /// - `composition_sort_order`: Ordering for layer submission (higher values render on top).
    /// - `visibility`: Optional eye visibility mask (None means both eyes).
    /// - `xr_space`: Optional XR space handle (None uses default space).
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn submit_quad_layer(
        world_pose: Pose,
        size: Vec2,
        swapchain: Swapchain,
        swapchain_rect: Rect,
        swapchain_array_index: u32,
        composition_sort_order: i32,
        visibility: Option<EyeVisibility>,
        xr_space: Option<u64>,
    ) {
        let orientation = (world_pose.orientation * Quat::from_angles(180.0, 0.0, 0.0)).conjugate();
        let xr_space = xr_space.unwrap_or_else(BackendOpenXR::space);
        let mut quad_layer = CompositionLayerQuad {
            ty: StructureType::COMPOSITION_LAYER_QUAD,
            next: null_mut(),
            layer_flags: CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
            space: Space::from_raw(xr_space),
            eye_visibility: visibility.unwrap_or(EyeVisibility::BOTH),
            sub_image: SwapchainSubImage {
                swapchain,
                image_rect: Rect2Di {
                    offset: Offset2Di { x: swapchain_rect.x as i32, y: swapchain_rect.y as i32 },
                    extent: Extent2Di { width: swapchain_rect.width as i32, height: swapchain_rect.height as i32 },
                },
                image_array_index: swapchain_array_index,
            },
            pose: Posef {
                orientation: Quaternionf { x: orientation.x, y: orientation.y, z: orientation.z, w: orientation.w },
                position: Vector3f { x: world_pose.position.x, y: world_pose.position.y, z: world_pose.position.z },
            },
            size: Extent2Df { width: size.x, height: size.y },
        };

        BackendOpenXR::add_composition_layer(&mut quad_layer, composition_sort_order);
    }

    /// Create an Android surface swapchain via `XR_KHR_android_surface_swapchain`.
    /// Returns the swapchain handle and raw `jobject` pointer on success.
    /// /// ## Android Surface Swapchain (Android only)
    ///
    /// ```no_run
    /// # #[cfg(target_os = "android")]
    /// # {
    /// use stereokit_rust::tools::xr_comp_layers::XrCompLayers;
    /// use openxr_sys::SwapchainUsageFlags;
    ///
    /// if let Some(comp_layers) = XrCompLayers::new() {
    ///     if let Some((swapchain_handle, android_surface)) = comp_layers.try_make_android_swapchain(
    ///         512, 512, SwapchainUsageFlags::COLOR_ATTACHMENT, false) {
    ///
    ///         println!("Created Android surface swapchain: {:?}", android_surface);
    ///         
    ///         // Use the surface
    ///         
    ///         // Clean up
    ///         comp_layers.destroy_android_swapchain(swapchain_handle);
    ///     }
    /// }
    /// # }
    #[cfg(target_os = "android")]
    pub fn try_make_android_swapchain(
        &self,
        width: u32,
        height: u32,
        usage: SwapchainUsageFlags,
        single_image: bool,
    ) -> Option<(Swapchain, *mut jobject)> {
        use openxr_sys::platform::jobject;

        let mut swapchain = Swapchain::default();
        let mut surface: *mut jobject = null_mut();

        let create_flags = if single_image {
            SwapchainCreateFlags::STATIC_IMAGE
        } else {
            SwapchainCreateFlags::PROTECTED_CONTENT
        };

        let info = SwapchainCreateInfo {
            ty: StructureType::SWAPCHAIN_CREATE_INFO,
            next: null_mut(),
            create_flags: create_flags,
            usage_flags: usage,
            format: 0,       // Required by spec to be zero for Android surface swapchains
            sample_count: 0, // Required by spec to be zero for Android surface swapchains
            width,
            height,
            face_count: 0, // Required by spec to be zero for Android surface swapchains
            array_size: 0, // Required by spec to be zero for Android surface swapchains
            mip_count: 0,  // Required by spec to be zero for Android surface swapchains
        };

        if let Some(func) = self.xr_create_swapchain_android {
            let res = unsafe {
                func(
                    Session::from_raw(BackendOpenXR::session()),
                    &info,
                    &mut swapchain,
                    &mut surface as *mut _ as *mut *mut std::ffi::c_void,
                )
            };
            match res {
                XrResult::SUCCESS => Some((swapchain, surface)),
                otherwise => {
                    Log::err(format!("❌ xrDestroySwapchain failed: {otherwise}"));
                    None
                }
            }
        } else {
            None
        }
    }

    /// Destroy the given Android swapchain handle.
    #[cfg(target_os = "android")]
    pub fn destroy_android_swapchain(&self, handle: Swapchain) {
        match unsafe { self.xr_destroy_swapchain.unwrap()(handle) } {
            XrResult::SUCCESS => {}
            otherwise => {
                Log::err(format!("❌ xrDestroySwapchain failed: {otherwise}"));
            }
        }
    }

    /// Create a standard XR swapchain with the given parameters.
    pub fn try_make_swapchain(
        &self,
        width: u32,
        height: u32,
        format: TexFormat,
        usage: SwapchainUsageFlags,
        single_image: bool,
    ) -> Option<Swapchain> {
        let mut swapchain = Swapchain::default();
        let create_flags = if single_image { SwapchainCreateFlags::STATIC_IMAGE } else { SwapchainCreateFlags::EMPTY };

        let info = SwapchainCreateInfo {
            ty: StructureType::SWAPCHAIN_CREATE_INFO,
            next: null_mut(),
            format: Self::to_native_format(format),
            create_flags,
            usage_flags: usage,
            sample_count: 1,
            width,
            height,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        };

        match unsafe {
            self.xr_create_swapchain.unwrap()(Session::from_raw(BackendOpenXR::session()), &info, &mut swapchain)
        } {
            XrResult::SUCCESS => {}
            otherwise => {
                Log::err(format!("❌ xrCreateSwapchain failed: {otherwise}"));
                return None;
            }
        }
        Some(swapchain)
    }

    /// Destroy the given XR swapchain handle.
    pub fn destroy_swapchain(swapchain: Swapchain) {
        // We need to get the function pointer directly
        let xr_destroy_swapchain = BackendOpenXR::get_function::<DestroySwapchain>("xrDestroySwapchain");

        if let Some(func) = xr_destroy_swapchain {
            unsafe {
                func(swapchain);
            }
        }
    }
}

/// High-level wrapper around an OpenXR swapchain.
/// Manages creation of render-target textures, image acquisition and release.
///
/// `SwapchainSk` provides a convenient interface for working with OpenXR swapchains
/// in StereoKit applications. It handles the complexity of swapchain image management
/// and provides StereoKit `Tex` objects for rendering.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ maths::{Vec3, Matrix, Pose, Vec2, Rect},  render_list::RenderList,
///     util::{named_colors, Color128, Time}, tex::TexFormat, material::Material, mesh::Mesh,
///     system::{Backend, BackendXRType, RenderClear}, tools::xr_comp_layers::* };
///
/// // Create a swapchain
/// if let Some(mut swapchain) = SwapchainSk::new(TexFormat::RGBA32, 512, 512, None) {
///    
///     // Set up rendering
///     let mut render_list = RenderList::new();
///     let material = Material::default().copy();
///     render_list.add_mesh(
///         Mesh::cube(),
///         material,
///         Matrix::IDENTITY,
///         named_colors::RED,
///         None
///     );
///    
///     // Render to swapchain
///     if let Ok(_) = swapchain.acquire_image(None) {
///         if let Some(render_target) = swapchain.get_render_target() {
///             render_list.draw_now(
///                 render_target,
///                 Matrix::look_at(Vec3::angle_xy(Time::get_totalf() * 90.0, 0.0), Vec3::ZERO, None),
///                 Matrix::orthographic(1.0, 1.0, 0.1, 10.0),
///                 None,
///                 Some(RenderClear::All),
///                 Rect::new(0.0, 0.0, 1.0, 1.0),
///                 None, None,
///             );
///         }
///         swapchain.release_image().expect("Failed to release image");
///     }
///    
///     // Clean up
///     swapchain.destroy();
/// }
/// ```
pub struct SwapchainSk {
    pub xr_comp_layers: XrCompLayers,
    pub handle: Swapchain,
    pub width: u32,
    pub height: u32,
    pub acquired: u32,
    images: Vec<Tex>,
    #[cfg(unix)]
    gles_images: Vec<openxr_sys::SwapchainImageOpenGLESKHR>,
    #[cfg(windows)]
    d3d_images: Vec<openxr_sys::SwapchainImageD3D11KHR>,
}

impl SwapchainSk {
    /// Create a new `SwapchainSk` for rendering into an OpenXR quad layer.
    /// Returns `Some<Self>` if the XR runtime and swapchain creation succeed.
    pub fn new(format: TexFormat, width: u32, height: u32, xr_comp_layers: Option<XrCompLayers>) -> Option<Self> {
        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;
        if Backend::xr_type() == BackendXRType::OpenXR {
            if let Some(handle) =
                xr_comp_layers.try_make_swapchain(width, height, format, SwapchainUsageFlags::COLOR_ATTACHMENT, false)
            {
                SwapchainSk::wrap(handle, format, width, height, Some(xr_comp_layers))
            } else {
                Log::warn("❌ Failed to create XR swapchain: Try_make_swapchain failed");
                None
            }
        } else {
            Log::warn("Swapchain: OpenXR backend is not available");
            None
        }
    }

    /// Return a reference to the currently acquired render-target texture, if any.
    ///
    /// This method provides access to the StereoKit `Tex` object that represents
    /// the currently acquired swapchain image. The texture can be used as a render
    /// target for drawing operations.
    ///
    /// # Returns
    /// - `Some(&Tex)`: Reference to the current render target texture.
    /// - `None`: No image is currently acquired or swapchain is empty.
    ///
    /// # Example
    /// ```no_run
    /// # use stereokit_rust::tools::xr_comp_layers::SwapchainSk;
    /// # let mut swapchain: SwapchainSk = todo!();
    /// if let Ok(_) = swapchain.acquire_image(None) {
    ///     if let Some(render_target) = swapchain.get_render_target() {
    ///         // Use render_target for drawing operations
    ///         println!("Render target size: {}x{}",
    ///                  render_target.get_width().unwrap_or(0),
    ///                  render_target.get_height().unwrap_or(0));
    ///         
    ///         // ... perform rendering to render_target ...
    ///     }
    ///     swapchain.release_image().expect("Failed to release");
    /// }
    /// ```
    pub fn get_render_target(&self) -> Option<&Tex> {
        if self.images.is_empty() {
            return None;
        }
        Some(&self.images[self.acquired as usize])
    }

    /// Wrap OpenGL ES swapchain images into `Tex` objects for `unix` platforms.
    #[cfg(unix)]
    pub fn wrap(
        handle: Swapchain,
        format: TexFormat,
        width: u32,
        height: u32,
        xr_comp_layers: Option<XrCompLayers>,
    ) -> Option<Self> {
        use openxr_sys::SwapchainImageOpenGLESKHR;

        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;

        let mut image_count = 0;
        match unsafe { xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(handle, 0, &mut image_count, null_mut()) }
        {
            XrResult::SUCCESS => {}
            otherwise => {
                Log::err(format!("❌ xrEnumerateSwapchainImages failed: {otherwise}"));
                return None;
            }
        }

        if Backend::graphics() == BackendGraphics::OpenGLESEGL {
            let mut gles_images: Vec<SwapchainImageOpenGLESKHR> = {
                let images: Vec<SwapchainImageOpenGLESKHR> = vec![
                    SwapchainImageOpenGLESKHR {
                        image: 0,
                        ty: StructureType::SWAPCHAIN_IMAGE_OPENGL_ES_KHR,
                        next: null_mut()
                    };
                    image_count as usize
                ];
                images
            };

            let mut final_count = 0;
            match unsafe {
                xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(
                    handle,
                    image_count,
                    &mut final_count,
                    gles_images.as_mut_ptr() as *mut _,
                )
            } {
                XrResult::SUCCESS => {}
                otherwise => {
                    Log::err(format!("❌ xrEnumerateSwapchainImages failed: {otherwise}"));
                    return None;
                }
            }

            assert_eq!(gles_images.len(), image_count as usize);
            //assert_eq!(gles_images.len(), 3);

            let mut this =
                Self { xr_comp_layers, handle, width, height, acquired: 0, gles_images, images: Vec::with_capacity(0) };

            for image in &this.gles_images {
                Log::diag(format!("SwapchainSk: image: {image:#?}"));
                // let mut image_sk =
                //     Tex::gen_color(named_colors::BLUE_VIOLET, width, height, TexType::Rendertarget, format);
                //let mut image_sk = Tex::new(TexType::Rendertarget, format, None);
                let mut image_sk =
                    Tex::render_target(width as usize, height as usize, Some(2), Some(format), None).unwrap();
                unsafe {
                    image_sk.set_native_surface(
                        image.image as *mut std::ffi::c_void,
                        TexType::Rendertarget,
                        XrCompLayers::to_native_format(format),
                        width as i32,
                        height as i32,
                        1,
                        true,
                    )
                };
                this.images.push(image_sk);
            }
            Some(this)
        } else {
            Log::warn("❌ SwapchainSk: OpenGL ES backend is not available");
            None
        }
    }

    /// Wrap D3D11 swapchain images into `Tex` objects for `windows` platforms.
    #[cfg(windows)]
    pub fn wrap(
        handle: Swapchain,
        format: TexFormat,
        width: u32,
        height: u32,
        xr_comp_layers: Option<XrCompLayers>,
    ) -> Option<Self> {
        use openxr_sys::SwapchainImageD3D11KHR;
        use std::ptr::null_mut;

        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;

        // First, get the image count
        let mut image_count = 0;
        match unsafe { xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(handle, 0, &mut image_count, null_mut()) }
        {
            XrResult::SUCCESS => {}
            err => {
                Log::err(format!("❌ xrEnumerateSwapchainImages failed: {err}"));
                return None;
            }
        }

        // Only proceed for D3D11 backend
        if Backend::graphics() == BackendGraphics::D3D11 {
            // Prepare D3D11 image array
            let mut d3d_images: Vec<SwapchainImageD3D11KHR> = vec![
                SwapchainImageD3D11KHR {
                    texture: null_mut(),
                    ty: StructureType::SWAPCHAIN_IMAGE_D3D11_KHR,
                    next: null_mut(),
                };
                image_count as usize
            ];
            let mut final_count = 0;
            match unsafe {
                xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(
                    handle,
                    image_count,
                    &mut final_count,
                    d3d_images.as_mut_ptr() as *mut _,
                )
            } {
                XrResult::SUCCESS => {}
                err => {
                    Log::err(format!("❌ xrEnumerateSwapchainImages failed: {err}"));
                    return None;
                }
            }
            let mut this =
                Self { xr_comp_layers, handle, width, height, acquired: 0, d3d_images, images: Vec::with_capacity(0) };

            // Wrap each D3D11 texture into a Tex object
            for img in &this.d3d_images {
                Log::diag(format!("SwapchainSk: image: {:#?}", img));
                // let mut image_sk =
                //     Tex::gen_color(named_colors::BLUE_VIOLET, width, height, TexType::Rendertarget, format);
                // let mut image_sk = Tex::new(TexType::Rendertarget, format, None);
                let mut image_sk =
                    Tex::render_target(width as usize, height as usize, Some(1), Some(format), None).unwrap();

                unsafe {
                    image_sk.set_native_surface(
                        img.texture,
                        TexType::Rendertarget,
                        XrCompLayers::to_native_format(format),
                        width as i32,
                        height as i32,
                        1,
                        true,
                    );
                }
                this.images.push(image_sk);
            }
            Some(this)
        } else {
            Log::warn("❌ SwapchainSk: D3D11 backend is not available");
            None
        }
    }

    /// Acquire the next image from the swapchain, waiting up to `timeout_ns` nanoseconds.
    ///
    /// This method must be called before rendering to the swapchain. It acquires an available
    /// image from the swapchain and waits for it to be ready for rendering.
    ///
    /// # Parameters
    /// - `timeout_ns`: Optional timeout in nanoseconds. If `None`, waits indefinitely.
    ///
    /// # Returns
    /// - `Ok(image_index)`: The index of the acquired image on success.
    /// - `Err(XrResult)`: OpenXR error code if acquisition fails.
    ///
    /// # Example
    /// ```no_run
    /// # use stereokit_rust::tools::xr_comp_layers::SwapchainSk;
    /// # let mut swapchain: SwapchainSk = todo!();
    /// // Acquire with default timeout
    /// match swapchain.acquire_image(None) {
    ///     Ok(image_index) => {
    ///         println!("Acquired image {}", image_index);
    ///         // Render to swapchain.get_render_target()
    ///         // ... rendering code ...
    ///         swapchain.release_image().expect("Failed to release");
    ///     }
    ///     Err(e) => eprintln!("Failed to acquire image: {:?}", e),
    /// }
    /// ```
    pub fn acquire_image(&mut self, timeout_ns: Option<i64>) -> std::result::Result<u32, XrResult> {
        let timeout_ns = timeout_ns.unwrap_or(0x7fffffffffffffff);
        let timeout = Duration::from_nanos(timeout_ns);
        match unsafe {
            self.xr_comp_layers.xr_acquire_swapchain_image.unwrap()(self.handle, null_mut(), &mut self.acquired)
        } {
            XrResult::SUCCESS => {}
            otherwise => return Err(otherwise),
        }

        let swapchain_image_wait_info =
            SwapchainImageWaitInfo { ty: StructureType::SWAPCHAIN_IMAGE_WAIT_INFO, next: null_mut(), timeout };

        match unsafe { self.xr_comp_layers.xr_wait_swaptchain_image.unwrap()(self.handle, &swapchain_image_wait_info) }
        {
            XrResult::SUCCESS => Ok(self.acquired),
            otherwise => Err(otherwise),
        }
    }

    /// Release the currently held image back to the swapchain.
    ///
    /// This method must be called after finishing rendering to the acquired image.
    /// It signals to the OpenXR runtime that rendering is complete and the image
    /// can be used for composition.
    ///
    /// # Returns
    /// - `Ok(())`: Successfully released the image.
    /// - `Err(XrResult)`: OpenXR error code if release fails.
    ///
    /// # Example
    /// ```no_run
    /// # use stereokit_rust::tools::xr_comp_layers::SwapchainSk;
    /// # let mut swapchain: SwapchainSk = todo!();
    /// // After acquiring and rendering to the image
    /// if let Ok(_) = swapchain.acquire_image(None) {
    ///     // ... render to swapchain.get_render_target() ...
    ///     
    ///     // Must release the image when done
    ///     swapchain.release_image().expect("Failed to release image");
    /// }
    /// ```
    pub fn release_image(&mut self) -> std::result::Result<(), XrResult> {
        match unsafe { self.xr_comp_layers.xr_release_swaptchain_image.unwrap()(self.handle, null_mut()) } {
            XrResult::SUCCESS => Ok(()),
            otherwise => Err(otherwise),
        }
    }

    /// Destroy the swapchain and all associated resources.
    pub fn destroy(&mut self) {
        XrCompLayers::destroy_swapchain(self.handle);
        self.handle = Swapchain::default();
    }
}

/// Ensure an `XrCompLayers` instance is available, either using the provided one or creating a new one.
pub fn get_xr_comp_layers(xr_comp_layers: Option<XrCompLayers>) -> Option<XrCompLayers> {
    if let Some(comp_layers) = xr_comp_layers { Some(comp_layers) } else { XrCompLayers::new() }
}
