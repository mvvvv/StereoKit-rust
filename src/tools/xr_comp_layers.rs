// filepath: src/tools/xr_comp_layers.rs
use crate::{
    maths::{Matrix, Pose, Rect, Vec2},
    prelude::*,
    system::{Backend, BackendGraphics, BackendOpenXR, BackendXRType},
    tex::{Tex, TexFormat, TexType},
};
#[cfg(target_os = "android")]
use openxr_sys::{pfn::CreateSwapchainAndroidSurfaceKHR, platform::jobject};

use openxr_sys::{
    Duration, Session, Space, Swapchain, SwapchainImageWaitInfo,
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
/// Helper for loading and using the OpenXR composition layers extension.
/// Provides function pointers for swapchain management and layer submission.
///
///  This is a rust adaptations of <https://github.com/StereoKit/StereoKit/blob/master/Examples/StereoKitTest/Tools/XrCompLayers.cs>
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
            Log::warn(format!("XrCompLayers: some bindings are missing : {:?}", this));
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
    /// # Parameters
    /// - `world_pose`: Pose of the quad in world space (will be rotated by 180° around Y).
    /// - `size`: Dimensions of the quad.
    /// - `swapchain`: Swapchain handle to sample from.
    /// - `swapchain_rect`: Texture rectangle within the swapchain image.
    /// - `swapchain_array_index`: Array slice index for texture arrays.
    /// - `composition_sort_order`: Ordering for layer submission.
    /// - `visibility`: Optional eye visibility mask.
    /// - `xr_space`: Optional XR space handle.
    #[allow(clippy::too_many_arguments)]
    pub fn submit_quad_layer(
        mut world_pose: Pose,
        size: Vec2,
        swapchain: Swapchain,
        swapchain_rect: Rect,
        swapchain_array_index: u32,
        composition_sort_order: i32,
        visibility: Option<EyeVisibility>,
        xr_space: Option<u64>,
    ) {
        world_pose *= Matrix::Y_180;
        let mut quad_layer = CompositionLayerQuad {
            ty: StructureType::COMPOSITION_LAYER_QUAD,
            next: null_mut(),
            layer_flags: CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
            space: Space::from_raw(xr_space.unwrap_or_else(BackendOpenXR::space)),
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
                orientation: Quaternionf {
                    x: world_pose.orientation.x,
                    y: world_pose.orientation.y,
                    z: world_pose.orientation.z,
                    w: world_pose.orientation.w,
                },
                position: Vector3f { x: world_pose.position.x, y: world_pose.position.y, z: world_pose.position.z },
            },
            size: Extent2Df { width: size.x, height: size.y },
        };

        BackendOpenXR::add_composition_layer(&mut quad_layer, composition_sort_order);
    }

    /// Create an Android surface swapchain via `XR_KHR_android_surface_swapchain`.
    /// Returns the swapchain handle and raw `jobject` pointer on success.
    #[cfg(target_os = "android")]
    pub fn try_make_android_swapchain(
        &self,
        width: i32,
        height: i32,
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
            width: width as u32,
            height: height as u32,
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
                    Log::err(format!("xrDestroySwapchain failed: {otherwise}"));
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
                Log::err(format!("xrDestroySwapchain failed: {otherwise}"));
            }
        }
    }

    /// Create a standard XR swapchain with the given parameters.
    pub fn try_make_swapchain(
        &self,
        width: i32,
        height: i32,
        format: TexFormat,
        usage: SwapchainUsageFlags,
        single_image: bool,
    ) -> Option<Swapchain> {
        let mut swapchain = Swapchain::default();
        let create_flags = if single_image {
            SwapchainCreateFlags::STATIC_IMAGE
        } else {
            SwapchainCreateFlags::PROTECTED_CONTENT
        };

        let info = SwapchainCreateInfo {
            ty: StructureType::SWAPCHAIN_CREATE_INFO,
            next: null_mut(),
            create_flags,
            usage_flags: usage,
            format: Self::to_native_format(format),
            sample_count: 1,
            width: width as u32,
            height: height as u32,
            face_count: 1,
            array_size: 1,
            mip_count: 1,
        };

        match unsafe {
            self.xr_create_swapchain.unwrap()(Session::from_raw(BackendOpenXR::session()), &info, &mut swapchain)
        } {
            XrResult::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrCreateSwapchain failed: {otherwise}"));
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
pub struct SwapchainSk {
    pub xr_comp_layers: XrCompLayers,
    pub handle: Swapchain,
    pub width: i32,
    pub height: i32,
    pub acquired: u32,
    images: Vec<Tex>,
}

impl SwapchainSk {
    /// Create a new `SwapchainSk` for rendering into an OpenXR quad layer.
    /// Returns `Some<Self>` if the XR runtime and swapchain creation succeed.
    pub fn new(format: TexFormat, width: i32, height: i32, xr_comp_layers: Option<XrCompLayers>) -> Option<Self> {
        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;
        if Backend::xr_type() == BackendXRType::OpenXR {
            if let Some(handle) = xr_comp_layers.try_make_swapchain(
                512,
                512,
                TexFormat::RGBA32,
                SwapchainUsageFlags::COLOR_ATTACHMENT,
                false,
            ) {
                SwapchainSk::wrap(handle, format, width, height, Some(xr_comp_layers))
            } else {
                Log::warn("Failed to create XR swapchain: Try_make_swapchain failed");
                None
            }
        } else {
            Log::warn("Swapchain: OpenXR backend is not available");
            None
        }
    }

    /// Return a reference to the currently acquired render-target texture, if any.
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
        width: i32,
        height: i32,
        xr_comp_layers: Option<XrCompLayers>,
    ) -> Option<Self> {
        use openxr_sys::SwapchainImageOpenGLESKHR;

        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;

        let mut image_count = 0;
        match unsafe { xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(handle, 0, &mut image_count, null_mut()) }
        {
            XrResult::SUCCESS => {}
            otherwise => {
                Log::err(format!("xrEnumerateSwapchainImages failed: {otherwise}"));
                return None;
            }
        }

        if Backend::graphics() == BackendGraphics::OpenGLESEGL {
            let mut gles_images = {
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
            let gles_images = gles_images.as_mut_slice();

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
                    Log::err(format!("xrEnumerateSwapchainImages failed: {otherwise}"));
                    return None;
                }
            }

            assert_eq!(gles_images.len(), image_count as usize);
            assert_eq!(gles_images.len(), 3);

            let mut this = Self { xr_comp_layers, handle, width, height, acquired: 0, images: Vec::with_capacity(0) };

            for image in gles_images {
                Log::diag(format!("SwapchainSk: image: {:#?}", image));
                let mut image_sk = Tex::new(TexType::Rendertarget, format, None);
                unsafe {
                    image_sk.set_native_surface(
                        &mut image.image as *mut _ as *mut std::ffi::c_void,
                        TexType::Rendertarget,
                        XrCompLayers::to_native_format(format),
                        width,
                        height,
                        1,
                        true,
                    )
                };
                this.images.push(image_sk);
            }

            Some(this)
        } else {
            None
        }
    }

    /// Wrap D3D11 swapchain images into `Tex` objects for `windows` platforms.
    #[cfg(windows)]
    pub fn wrap(
        handle: Swapchain,
        format: TexFormat,
        width: i32,
        height: i32,
        xr_comp_layers: Option<XrCompLayers>,
    ) -> Option<Self> {
        use openxr_sys::SwapchainImageD3D11KHR;
        use std::ptr::null_mut;

        let xr_comp_layers = get_xr_comp_layers(xr_comp_layers)?;
        let mut this = Self { xr_comp_layers, handle, width, height, acquired: 0, images: Vec::with_capacity(0) };

        // First, get the image count
        let mut image_count = 0;
        match unsafe {
            xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(this.handle, 0, &mut image_count, null_mut())
        } {
            XrResult::SUCCESS => {}
            err => {
                Log::err(format!("xrEnumerateSwapchainImages failed: {err}"));
                return None;
            }
        }

        // Only proceed for D3D11 backend
        if Backend::graphics() == BackendGraphics::D3D11 {
            // Prepare D3D11 image array
            let mut d3d_images: Vec<SwapchainImageD3D11KHR> = vec![
                SwapchainImageD3D11KHR {
                    ty: StructureType::SWAPCHAIN_IMAGE_D3D11_KHR,
                    next: null_mut(),
                    texture: null_mut()
                };
                image_count as usize
            ];
            let mut final_count = 0;
            match unsafe {
                xr_comp_layers.xr_enumerate_swaptchain_images.unwrap()(
                    this.handle,
                    image_count,
                    &mut final_count,
                    d3d_images.as_mut_ptr() as *mut _,
                )
            } {
                XrResult::SUCCESS => {}
                err => {
                    Log::err(format!("xrEnumerateSwapchainImages failed: {err}"));
                    return None;
                }
            }

            assert_eq!(d3d_images.len(), image_count as usize);
            // Wrap each D3D11 texture into a Tex object
            for img in &mut d3d_images {
                let tex_native = img.texture as *mut std::ffi::c_void;
                let mut image_sk = Tex::new(TexType::Rendertarget, format, None);
                unsafe {
                    image_sk.set_native_surface(
                        tex_native,
                        TexType::Rendertarget,
                        XrCompLayers::to_native_format(format),
                        width,
                        height,
                        1,
                        true,
                    );
                }
                this.images.push(image_sk);
            }
            Some(this)
        } else {
            None
        }
    }

    /// Acquire the next image from the swapchain, waiting up to `timeout_ns` nanoseconds.
    /// Returns the image index on success.
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
    pub fn release_image(&mut self) -> std::result::Result<(), XrResult> {
        match unsafe { self.xr_comp_layers.xr_release_swaptchain_image.unwrap()(self.handle, null_mut()) } {
            XrResult::SUCCESS => Ok(()),
            otherwise => Err(otherwise),
        }
    }
}

/// Ensure an `XrCompLayers` instance is available, either using the provided one or creating a new one.
pub fn get_xr_comp_layers(xr_comp_layers: Option<XrCompLayers>) -> Option<XrCompLayers> {
    if let Some(comp_layers) = xr_comp_layers { Some(comp_layers) } else { XrCompLayers::new() }
}
