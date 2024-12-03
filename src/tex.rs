use std::{
    ffi::{c_char, c_void, CStr, CString},
    mem::size_of,
    path::{Path, PathBuf},
    ptr::{null_mut, NonNull},
};

use crate::{
    maths::{Bool32T, Vec3},
    system::{
        render_enable_skytex, render_get_skylight, render_get_skytex, render_set_skylight, render_set_skytex,
        AssetState, IAsset, Log,
    },
    util::{Color128, Color32, Gradient, GradientKey, GradientT, SphericalHarmonics},
    StereoKitError,
};

bitflags::bitflags! {
    /// Textures come in various types and flavors! These are bit-flags
    /// that tell StereoKit what type of texture we want; and how the application
    /// might use it!
    /// <https://stereokit.net/Pages/StereoKit/TexType.html>
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct TexType: u32 {
        /// A standard color image; without any generated mip-maps.
        const ImageNomips = 1 << 0;
        /// A size sided texture that's used for things like skyboxes;
        /// environment maps; and reflection probes. It behaves like a texture
        /// array with 6 textures.
        const Cubemap = 1 << 1;
        /// This texture can be rendered to! This is great for textures
        /// that might be passed in as a target to Renderer.Blit; or other
        /// such situations.
        const Rendertarget = 1 << 2;
        /// This texture contains depth data; not color data!
        const Depth = 1 << 3;
        /// This texture will generate mip-maps any time the contents
        /// change. Mip-maps are a list of textures that are each half the
        /// size of the one before them! This is used to prevent textures from
        /// 'sparkling' or aliasing in the distance.
        const Mips = 1 << 4;
        /// This texture's data will be updated frequently from the
        /// CPU (not renders)! This ensures the graphics card stores it
        /// someplace where writes are easy to do quickly.
        const Dynamic = 1 << 5;
        /// A standard color image that also generates mip-maps
        /// automatically.
        const Image = Self::ImageNomips.bits() | Self::Mips.bits();
    }
}
impl TexType {
    pub fn as_u32(&self) -> u32 {
        self.bits()
    }
}
/// What type of color information will the texture contain? A
/// good default here is Rgba32.
/// <https://stereokit.net/Pages/StereoKit/TexFormat.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TexFormat {
    /// A default zero value for TexFormat! Uninitialized formats will get this value and **** **** up so you know to
    /// assign it properly :)
    None = 0,
    /// Red/Green/Blue/Transparency data channels, at 8 bits per-channel in sRGB color space. This is what you'll
    /// want most of the time you're dealing with color images! Matches well with the Color32 struct! If you're
    /// storing normals, rough/metal, or anything else, use Rgba32Linear.
    RGBA32 = 1,
    /// Red/Green/Blue/Transparency data channels, at 8 bits per-channel in linear color space. This is what you'll
    /// want most of the time you're dealing with color data! Matches well with the Color32 struct.
    RGBA32Linear = 2,
    /// Blue/Green/Red/Transparency data channels, at 8 bits per-channel in sRGB color space. This is a common swapchain
    /// format  on Windows.
    BGRA32 = 3,
    /// Blue/Green/Red/Transparency data channels, at 8 bits per-channel in linear color space. This is a common
    /// swapchain format on Windows.
    BGRA32Linear = 4,
    /// Red/Green/Blue data channels, with 11 bits for R and G, and 10 bits for blue. This is a great presentation format
    /// for high bit depth displays that still fits in 32 bits! This format has no alpha channel.
    RG11B10 = 5,
    /// Red/Green/Blue/Transparency data channels, with 10 bits for R, G, and B, and 2 for alpha. This is a great
    /// presentation format for high bit depth displays that still fits in 32 bits, and also includes at least a bit of
    /// transparency!
    RGB10A2 = 6,
    /// Red/Green/Blue/Transparency data channels, at 16 bits per-channel! This is not common, but you might encounter
    /// it with raw photos, or HDR images.  The u postfix indicates that the raw color data is stored as an unsigned
    /// 16 bit integer, which is then normalized into the 0, 1 floating point range on the GPU.
    RGBA64U = 7,
    /// Red/Green/Blue/Transparency data channels, at 16 bits per-channel! This is not common, but you might encounter
    /// it with raw photos, or HDR images. The s postfix indicates that the raw color data is stored as a signed 16 bit
    /// integer, which is then normalized into the -1, +1 floating point range on the GPU.
    RGBA64S = 8,
    /// Red/Green/Blue/Transparency data channels, at 16 bits per-channel! This is not common, but you might encounter
    /// it with raw photos, or HDR images. The f postfix indicates that the raw color data is stored as 16 bit floats,
    /// which may be tricky to work with in most languages.
    RGBA64F = 9,
    /// Red/Green/Blue/Transparency data channels at 32 bits per-channel! Basically 4 floats per color, which is bonkers
    /// expensive. Don't use this unless you know -exactly- what you're doing.
    RGBA128 = 10,
    /// A single channel of data, with 8 bits per-pixel! This can be great when you're only using one channel, and want
    /// to reduce memory usage. Values in the shader are always 0.0-1.0.
    R8 = 11,
    /// A single channel of data, with 16 bits per-pixel! This is a good format for height maps, since it stores a fair
    /// bit of information in it. Values in the shader are always 0.0-1.0.
    /// TODO: remove during major version update, prefer s, f, or u postfixed versions of this format, this item is the
    /// same as  r16u.
    //R16 = 12,
    /// A single channel of data, with 16 bits per-pixel! This is a good format for height maps, since it stores a fair
    /// bit of information in it. The u postfix indicates that the raw color data is stored as an unsigned 16 bit
    /// integer, which is then normalized into the 0, 1 floating point range on the GPU.
    R16u = 12,
    /// A single channel of data, with 16 bits per-pixel! This is a good format for height maps, since it stores a fair
    /// bit of information in it. The s postfix indicates that the raw color data is stored as a signed 16 bit integer,
    /// which is then normalized into the -1, +1 floating point range on the GPU.
    R16s = 13,
    /// A single channel of data, with 16 bits per-pixel! This is a good format for height maps, since it stores a fair
    /// bit of information in it. The f postfix indicates that the raw color data is stored as 16 bit floats, which may
    /// be tricky to work with in most languages.
    R16f = 14,
    /// A single channel of data, with 32 bits per-pixel! This basically treats each pixel as a generic float, so you
    /// can do all sorts of strange and interesting things with this.
    R32 = 15,
    /// A depth data format, 24 bits for depth data, and 8 bits to store stencil information! Stencil data can be used
    /// for things like clipping effects, deferred rendering, or shadow effects.
    DepthStencil = 16,
    /// 32 bits of data per depth value! This is pretty detailed, and is excellent for experiences that have a very far
    /// view distance.
    Depth32 = 17,
    /// 16 bits of depth is not a lot, but it can be enough if your far clipping plane is pretty close. If you're seeing
    /// lots of flickering where two objects overlap, you either need to bring your far clip in, or switch to 32/24 bit
    /// depth.
    Depth16 = 18,
    /// A double channel of data that supports 8 bits for the red channel and 8 bits for the green channel.
    R8G8 = 19,
}

/// How does the shader grab pixels from the texture? Or more
/// specifically, how does the shader grab colors between the provided
/// pixels? If you'd like an in-depth explanation of these topics, check
/// out [this exploration of texture filtering]
/// (https://medium.com/@bgolus/sharper-mipmapping-using-shader-based-supersampling-ed7aadb47bec)
/// by graphics wizard Ben Golus.
/// <https://stereokit.net/Pages/StereoKit/TexSample.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TexSample {
    /// Use a linear blend between adjacent pixels, this creates a smooth, blurry look when texture resolution is too
    /// low.
    Linear = 0,
    /// Choose the nearest pixel's color! This makes your texture look like pixel art if you're too close.
    Point = 1,
    /// This helps reduce texture blurriness when a surface is viewed at an extreme angle!
    Anisotropic = 2,
}

/// What happens when the shader asks for a texture coordinate
/// that's outside the texture?? Believe it or not, this happens plenty
/// often!
///<https://stereokit.net/Pages/StereoKit/TexAddress.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TexAddress {
    /// Wrap the UV coordinate around to the other side of the texture! This is basically like a looping texture, and
    /// is an excellent default. If you can see weird bits of color at the edges of your texture, this may be due to
    /// Wrap blending the color with the other side of the texture, Clamp may be better in such cases.
    Wrap = 0,
    /// Clamp the UV coordinates to the edge of the texture! This'll create color streaks that continue to forever. This
    /// is actually really great for non-looping textures that you know will always be accessed on the 0-1 range.
    Clamp = 1,
    /// Like Wrap, but it reflects the image each time! Who needs this? I'm not sure!! But the graphics card can do it,
    /// so now you can too!
    Mirror = 2,
}

/// This is the texture asset class! This encapsulates 2D images, texture arrays, cubemaps, and rendertargets! It can
/// load any image format that stb_image can, (jpg, png, tga, bmp, psd, gif, hdr, pic, ktx2) plus more later on, and you
/// can also create textures procedurally.
/// <https://stereokit.net/Pages/StereoKit/Tex.html>
#[repr(C)]
#[derive(Debug)]
pub struct Tex(pub NonNull<_TexT>);
impl Drop for Tex {
    fn drop(&mut self) {
        unsafe { tex_release(self.0.as_ptr()) };
    }
}
impl AsRef<Tex> for Tex {
    fn as_ref(&self) -> &Tex {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _TexT {
    _unused: [u8; 0],
}
pub type TexT = *mut _TexT;

unsafe impl Send for Tex {}
unsafe impl Sync for Tex {}

extern "C" {
    pub fn tex_find(id: *const c_char) -> TexT;
    pub fn tex_create(type_: TexType, format: TexFormat) -> TexT;
    pub fn tex_create_rendertarget(
        width: i32,
        height: i32,
        msaa: i32,
        color_format: TexFormat,
        depth_format: TexFormat,
    ) -> TexT;
    pub fn tex_create_color32(in_arr_data: *mut Color32, width: i32, height: i32, srgb_data: Bool32T) -> TexT;
    pub fn tex_create_color128(in_arr_data: *mut Color128, width: i32, height: i32, srgb_data: Bool32T) -> TexT;
    pub fn tex_create_mem(data: *mut c_void, data_size: usize, srgb_data: Bool32T, load_priority: i32) -> TexT;
    pub fn tex_create_file(file_utf8: *const c_char, srgb_data: Bool32T, load_priority: i32) -> TexT;
    pub fn tex_create_file_arr(
        in_arr_files: *mut *const c_char,
        file_count: i32,
        srgb_data: Bool32T,
        load_priority: i32,
    ) -> TexT;
    pub fn tex_create_cubemap_file(cubemap_file: *const c_char, srgb_data: Bool32T, load_priority: i32) -> TexT;
    pub fn tex_create_cubemap_files(
        in_arr_cube_face_file_xxyyzz: *mut *const c_char,
        srgb_data: Bool32T,
        load_priority: i32,
    ) -> TexT;
    pub fn tex_set_id(texture: TexT, id: *const c_char);
    pub fn tex_get_id(texture: TexT) -> *const c_char;
    pub fn tex_set_fallback(texture: TexT, fallback: TexT);
    pub fn tex_set_surface(
        texture: TexT,
        native_surface: *mut c_void,
        type_: TexType,
        native_fmt: i64,
        width: i32,
        height: i32,
        surface_count: i32,
        multisample: i32,
        framebuffer_multisample: i32,
        owned: Bool32T,
    );
    pub fn tex_get_surface(texture: TexT) -> *mut c_void;
    pub fn tex_addref(texture: TexT);
    pub fn tex_release(texture: TexT);
    pub fn tex_asset_state(texture: TexT) -> AssetState;
    pub fn tex_on_load(
        texture: TexT,
        asset_on_load_callback: ::std::option::Option<unsafe extern "C" fn(texture: TexT, context: *mut c_void)>,
        context: *mut c_void,
    );
    pub fn tex_on_load_remove(
        texture: TexT,
        asset_on_load_callback: ::std::option::Option<unsafe extern "C" fn(texture: TexT, context: *mut c_void)>,
    );
    pub fn tex_set_colors(texture: TexT, width: i32, height: i32, data: *mut c_void);
    pub fn tex_set_color_arr(
        texture: TexT,
        width: i32,
        height: i32,
        data: *mut *mut c_void,
        data_count: i32,
        out_sh_lighting_info: *mut SphericalHarmonics,
        multisample: i32,
    );
    pub fn tex_set_mem(
        texture: TexT,
        data: *mut c_void,
        data_size: usize,
        srgb_data: Bool32T,
        blocking: Bool32T,
        priority: i32,
    );
    pub fn tex_add_zbuffer(texture: TexT, format: TexFormat);
    pub fn tex_set_zbuffer(texture: TexT, depth_texture: TexT);
    pub fn tex_get_zbuffer(texture: TexT) -> TexT;
    pub fn tex_get_data(texture: TexT, out_data: *mut c_void, out_data_size: usize, mip_level: i32);
    pub fn tex_gen_color(color: Color128, width: i32, height: i32, type_: TexType, format: TexFormat) -> TexT;
    pub fn tex_gen_particle(width: i32, height: i32, roundness: f32, gradient_linear: GradientT) -> TexT;
    pub fn tex_gen_cubemap(
        gradient: GradientT,
        gradient_dir: Vec3,
        resolution: i32,
        out_sh_lighting_info: *mut SphericalHarmonics,
    ) -> TexT;
    pub fn tex_gen_cubemap_sh(
        lookup: *const SphericalHarmonics,
        face_size: i32,
        light_spot_size_pct: f32,
        light_spot_intensity: f32,
    ) -> TexT;
    pub fn tex_get_format(texture: TexT) -> TexFormat;
    pub fn tex_get_width(texture: TexT) -> i32;
    pub fn tex_get_height(texture: TexT) -> i32;
    pub fn tex_set_sample(texture: TexT, sample: TexSample);
    pub fn tex_get_sample(texture: TexT) -> TexSample;
    pub fn tex_set_address(texture: TexT, address_mode: TexAddress);
    pub fn tex_get_address(texture: TexT) -> TexAddress;
    pub fn tex_set_anisotropy(texture: TexT, anisotropy_level: i32);
    pub fn tex_get_anisotropy(texture: TexT) -> i32;
    pub fn tex_get_mips(texture: TexT) -> i32;
    pub fn tex_set_loading_fallback(loading_texture: TexT);
    pub fn tex_set_error_fallback(error_texture: TexT);
    pub fn tex_get_cubemap_lighting(cubemap_texture: TexT) -> SphericalHarmonics;
}

impl IAsset for Tex {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

/// A Default texture is asked when a Tex creation or find returned an error.
impl Default for Tex {
    fn default() -> Self {
        Self::error()
    }
}

impl Tex {
    /// Sets up an empty texture container! Fill it with data using SetColors next! Creates a default unique asset Id.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Tex.html>
    ///
    /// see also [`crate::tex::tex_create`]
    pub fn new<S: AsRef<str>>(r#type: TexType, format: TexFormat, id: S) -> Tex {
        let tex = Tex(NonNull::new(unsafe { tex_create(r#type, format) }).unwrap());
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { tex_set_id(tex.0.as_ptr(), c_str.as_ptr()) };
        tex
    }

    /// Loads an image file stored in memory directly into a texture! Supported formats are: jpg, png, tga, bmp, psd,
    /// gif, hdr, pic, ktx2.
    /// Asset Id will be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromMemory.html>
    /// * priority - If None will be set to 10
    ///
    /// see also [`crate::tex::tex_create_mem`]
    pub fn from_memory(data: &[u8], srgb_data: bool, priority: Option<i32>) -> Result<Tex, StereoKitError> {
        let priority = priority.unwrap_or(10);
        Ok(Tex(NonNull::new(unsafe {
            tex_create_mem(data.as_ptr() as *mut c_void, data.len(), srgb_data as Bool32T, priority)
        })
        .ok_or(StereoKitError::TexMemory)?))
    }

    /// Loads an image file directly into a texture! Supported formats are: jpg, png, tga, bmp, psd, gif, hdr, pic, ktx2.
    /// Asset Id will be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromFile.html>
    /// * priority - If None will be set to 10
    ///
    /// see also [`crate::tex::tex_create_file`]
    pub fn from_file(
        file_utf8: impl AsRef<Path>,
        srgb_data: bool,
        priority: Option<i32>,
    ) -> Result<Tex, StereoKitError> {
        let priority = priority.unwrap_or(10);
        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str()
                .ok_or(StereoKitError::TexFile(path_buf.clone(), "CString conversion".to_string()))?,
        )?;
        Ok(Tex(NonNull::new(unsafe { tex_create_file(c_str.as_ptr(), srgb_data as Bool32T, priority) })
            .ok_or(StereoKitError::TexFile(path_buf, "tex_create failed".to_string()))?))
    }

    /// Loads an array of image files directly into a single array texture! Array textures are often useful for shader
    /// effects, layering, material merging, weird stuff, and will generally need a specific shader to support it.
    /// Supported formats are: jpg, png, tga, bmp, psd, gif, hdr, pic, ktx2. Asset Id will be the hash of all the
    /// filenames merged consecutively.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromFiles.html>
    /// * priority - If None will be set to 10
    ///
    /// see also [`crate::tex::tex_create_file`]
    pub fn from_files<P: AsRef<Path>>(
        files_utf8: &[P],
        srgb_data: bool,
        priority: Option<i32>,
    ) -> Result<Tex, StereoKitError> {
        let priority = priority.unwrap_or(10);
        let mut c_files = Vec::new();
        for path in files_utf8 {
            let path = path.as_ref();
            let path_buf = path.to_path_buf();
            let c_str =
                CString::new(path.to_str().ok_or(StereoKitError::TexCString(path_buf.to_str().unwrap().to_owned()))?)?;
            c_files.push(c_str);
        }
        let mut c_files_ptr = Vec::new();
        for str in c_files.iter() {
            c_files_ptr.push(str.as_ptr());
        }
        let in_arr_files_cstr = c_files_ptr.as_mut_slice().as_mut_ptr();
        let tex = Tex(NonNull::new(unsafe {
            tex_create_file_arr(in_arr_files_cstr, files_utf8.len() as i32, srgb_data as Bool32T, priority)
        })
        .ok_or(StereoKitError::TexFile(
            PathBuf::from(r"one_of_many_files"),
            "tex_create_file_arr failed".to_string(),
        ))?);
        Ok(tex)
    }

    /// Creates a texture and sets the texture’s pixels using a color array! This will be an image of type TexType.Image,
    /// and a format of TexFormat.Rgba32 or TexFormat.Rgba32Linear depending on the value of the sRGBData parameter.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromColors.html>
    ///
    ///  see also [`crate::tex::tex_create_color32`] [`crate::tex::Tex::gen_color`]
    pub fn from_color32(
        in_arr_data: &[Color32],
        width: usize,
        height: usize,
        srgb_data: bool,
    ) -> Result<Tex, StereoKitError> {
        if width * height != { in_arr_data }.len() {
            return Err(StereoKitError::TexColor(
                format!("{}x{} differ from {}", height, width, { in_arr_data }.len()),
                "tex_create_color32 failed".to_string(),
            ));
        }
        Ok(Tex(NonNull::new(unsafe {
            tex_create_color32(in_arr_data.as_ptr() as *mut Color32, width as i32, height as i32, srgb_data as i32)
        })
        .ok_or(StereoKitError::TexColor(
            format!("{}x{}", height, width),
            "tex_create_color32 failed".to_string(),
        ))?))
    }

    /// Creates a texture and sets the texture’s pixels using a color array! Color values are converted to 32 bit colors,
    /// so this means a memory allocation and conversion. Prefer the Color32 overload for performance, or create an empty
    /// Texture and use SetColors for more flexibility. This will be an image of type TexType.Image, and a format of
    /// TexFormat. Rgba32 or TexFormat.Rgba32Linear depending on the value of the sRGBData parameter.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromColors.html>
    ///
    /// The color conversion from 128 to 32 may crash if the data do not contains color128.
    ///
    ///  see also [`crate::tex::tex_create_color128`] [`crate::tex::Tex::gen_color()`]
    pub fn from_color128(
        in_arr_data: &[Color128],
        width: usize,
        height: usize,
        srgb_data: bool,
    ) -> Result<Tex, StereoKitError> {
        if width * height != { in_arr_data }.len() {
            return Err(StereoKitError::TexColor(
                format!("{}x{} differ from {}", height, width, { in_arr_data }.len()),
                "tex_create_color128 failed".to_string(),
            ));
        }
        Ok(Tex(NonNull::new(unsafe {
            tex_create_color128(in_arr_data.as_ptr() as *mut Color128, width as i32, height as i32, srgb_data as i32)
        })
        .ok_or(StereoKitError::TexColor(
            format!("{}x{}", height, width),
            "tex_create_color128 failed".to_string(),
        ))?))
    }

    /// This will assemble a texture ready for rendering to! It creates a render target texture with no mip maps and a
    /// depth buffer attached.
    /// <https://stereokit.net/Pages/StereoKit/Tex/RenderTarget.html>
    /// * width - in pixels
    /// * height - in pixels
    /// * multisample - Multisample level, or MSAA. This should be 1, 2, 4, 8, or 16. The results will have moother
    ///   edges with higher values, but will cost more RAM and time to render. Note that GL platforms cannot trivially
    ///   draw a multisample > 1 texture in a shader.
    /// * color_format - The format of the color surface.
    /// * depth _format - The format of the depth buffer. If this is None, no depth buffer will be attached to this
    ///   rendertarget.
    ///
    ///  see also [`crate::tex::tex_create_rendertarget()`]
    pub fn render_target(
        width: usize,
        height: usize,
        multisample: Option<i32>,
        color_format: Option<TexFormat>,
        depth_format: Option<TexFormat>,
    ) -> Result<Tex, StereoKitError> {
        let multisample = multisample.unwrap_or(1);
        let color_format = color_format.unwrap_or(TexFormat::RGBA32);
        let depth_format = depth_format.unwrap_or(TexFormat::Depth16);
        Ok(Tex(NonNull::new(unsafe {
            tex_create_rendertarget(width as i32, height as i32, multisample, color_format, depth_format)
        })
        .ok_or(StereoKitError::TexRenderTarget(
            format!("{}x{}", height, width),
            "tex_create_rendertarget failed".to_string(),
        ))?))
    }

    /// This generates a solid color texture of the given dimensions. Can be quite nice for creating placeholder textures!
    /// Make sure to match linear/gamma colors with the correct format.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenColor.html>
    ///
    ///  see also [`crate::tex::tex_gen_color`]
    pub fn gen_color(color: impl Into<Color128>, width: i32, height: i32, type_: TexType, format: TexFormat) -> Tex {
        Tex(NonNull::new(unsafe { tex_gen_color(color.into(), width, height, type_, format) }).unwrap())
    }

    /// Generates a ‘radial’ gradient that works well for particles, blob shadows, glows, or various other things.
    /// The roundness can be used to change the shape from round, ‘1’, to star-like, ‘0’. Default color is transparent white to opaque white,
    /// but this can be configured by providing a Gradient of your own.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenParticle.html>
    /// * gradient_linear : A color gradient that starts with the background/outside at 0, and progresses to the center
    ///   at 1. If None, use a white gradient.
    ///
    ///  see also [`crate::tex::tex_gen_particle`]
    pub fn gen_particle(width: i32, height: i32, roundness: f32, gradient_linear: Option<Gradient>) -> Tex {
        let gradient_linear = match gradient_linear {
            Some(gl) => gl,
            None => {
                let keys: [GradientKey; 2] = [
                    GradientKey { color: [1.0, 1.0, 1.0, 0.0].into(), position: 0.0 },
                    GradientKey { color: Color128::WHITE, position: 1.0 },
                ];
                Gradient::new(Some(&keys))
            }
        };
        Tex(NonNull::new(unsafe { tex_gen_particle(width, height, roundness, gradient_linear.0.as_ptr()) }).unwrap())
    }

    /// This is the texture that all Tex objects will fall back to by default if they are still loading. Assigning a texture here that isn’t fully
    /// loaded will cause the app to block until it is loaded.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetLoadingFallback.html>
    ///
    ///  see also [`crate::tex::tex_set_loading_fallback`]
    pub fn set_loading_fallback<T: AsRef<Tex>>(fallback: T) {
        unsafe { tex_set_loading_fallback(fallback.as_ref().0.as_ptr()) };
    }

    /// This is the texture that all Tex objects with errors will fall back to. Assigning a texture here that isn’t
    /// fully loaded will cause the app to block until it is loaded.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetErrorFallback.html>
    ///
    ///  see also [`crate::tex::tex_set_error_fallback`]
    pub fn set_error_fallback<T: AsRef<Tex>>(fallback: T) {
        unsafe { tex_set_error_fallback(fallback.as_ref().0.as_ptr()) };
    }

    /// Looks for a Material asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Tex/Find.html>
    ///
    /// see also [`crate::tex::material_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Tex, StereoKitError> {
        let c_str = CString::new(id.as_ref()).map_err(|_| StereoKitError::TexCString(id.as_ref().into()))?;
        Ok(Tex(
            NonNull::new(unsafe { tex_find(c_str.as_ptr()) }).ok_or(StereoKitError::TexFind(id.as_ref().into()))?
        ))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Find.html>
    ///
    /// see also [`crate::tex::tex_find()`]
    pub fn clone_ref(&self) -> Tex {
        Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Set a new id to the texture.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Id.html>
    ///
    /// see also [`crate::tex::tex_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { tex_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Only applicable if this texture is a rendertarget! This creates and attaches a zbuffer surface to the texture
    /// for use when rendering to it.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddZBuffer.html>
    ///
    /// see also [`crate::tex::tex_add_zbuffer`]
    pub fn add_zbuffer(&mut self, depth_format: TexFormat) -> &mut Self {
        unsafe { tex_add_zbuffer(self.0.as_ptr(), depth_format) };
        self
    }

    /// Loads an image file stored in memory directly into the created texture! Supported formats are: jpg, png, tga, bmp, psd, gif,
    /// hdr, pic, ktx2. This method introduces a blocking boolean parameter, which allows you to specify whether this method blocks until
    /// the image fully loads! The default case is to have it as part of the asynchronous asset pipeline, in which the Asset Id will
    /// be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetMemory.html>
    ///
    /// see also [`crate::tex::tex_set_mem`]
    pub fn set_memory(&mut self, data: &[u8], srgb_data: bool, blocking: i32, priority: i32) -> &mut Self {
        unsafe {
            tex_set_mem(
                self.0.as_ptr(),
                data.as_ptr() as *mut c_void,
                data.len(),
                srgb_data as Bool32T,
                blocking,
                priority,
            )
        };
        self
    }

    /// Set the texture’s pixels using a pointer to a chunk of memory! This is great if you’re pulling in some color
    /// data from native code, and don’t want to pay the cost of trying to marshal that data around.
    /// The data should contains width*height*(TextureFormat size) bytes.
    /// Warning: The check width*height*(TextureFormat size) upon the size of the data values must be done before
    /// calling this function.
    /// Warning: The color data type must be compliant with the format of the texture.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// see also [`crate::tex::tex_set_colors`]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn set_colors(&mut self, width: usize, height: usize, data: *mut std::os::raw::c_void) -> &mut Self {
        unsafe { tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data) };
        self
    }

    /// Set the texture’s pixels using a color array! This function should only be called on textures with a format of
    /// Rgba32 or Rgba32Linear. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is
    /// inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors32(&mut self, width: usize, height: usize, data: &[Color32]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::RGBA32) => (),
            Some(TexFormat::RGBA32Linear) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors32",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors32", self.get_id()));
                return self;
            }
        }
        if width * height != data.len() {
            Log::err(format!(
                "{}x{} differ from {} in Tex::set_color32 for texture {}",
                height,
                width,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// Set the texture’s pixels using a color array! This function should only be called on textures with a format of
    /// Rgba128. You can call this as many times as you’d like, even with different widths and heights. Calling this
    /// multiple times will mark it as dynamic on the graphics card.
    /// Calling this function can also result in building mip-maps, which has a non-zero cost: use TexType.ImageNomips
    /// when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors128(&mut self, width: usize, height: usize, data: &[Color128]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::RGBA128) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors128",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors128", self.get_id()));
                return self;
            }
        }
        if width * height != data.len() {
            Log::err(format!(
                "{}x{} differ from {} for Tex::set_color128 for texture {}",
                height,
                width,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// Set the texture’s pixels using a scalar array for channel R !  This function should only be called on textures
    /// with a format of R8. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is
    /// inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors_r8(&mut self, width: usize, height: usize, data: &[u8]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::R8) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors_r8",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors_r8", self.get_id()));
                return self;
            }
        }
        if width * height != data.len() {
            Log::err(format!(
                "{}x{} differ from {} for Tex::set_color_r8 for texture {}",
                height,
                width,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// Non canonical function !!
    /// Set the texture’s pixels using an u8 array !  This function should only be called for all textures format
    /// with a format of R8. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * color_size - number of byte for a pixel used by the format of this texture
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is
    /// inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors_u8(&mut self, width: usize, height: usize, data: &[u8], color_size: usize) -> &mut Self {
        if width * height * color_size != data.len() {
            Log::err(format!(
                "{}x{}x{} differ from {} for Tex::set_color_u8 for texture {}",
                height,
                width,
                color_size,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// Set the texture’s pixels using a scalar array for channel R ! This function should only be called on textures
    /// with a format of R16. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors_r16(&mut self, width: usize, height: usize, data: &[u16]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::R16f) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors_r16",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors_r16", self.get_id()));
                return self;
            }
        }
        if width * height != data.len() {
            Log::err(format!(
                "{}x{} differ from {} for Tex::set_color_r16 for texture {}",
                height,
                width,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// Set the texture’s pixels using a scalar array! This function should only be called on textures with a format of
    /// R32. You can call this as many times as you’d like, even with different widths and heights. Calling this
    /// multiple times will mark it as dynamic on the graphics card. Calling this function can also result in building
    /// mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    ///
    /// Warning, instead of [Tex::set_colors], this call may not be done if the asset is not loaded
    /// (see [Tex::get_asset_state]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`crate::tex::tex_set_colors`]
    pub fn set_colors_r32(&mut self, width: usize, height: usize, data: &[f32]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::R32) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors_r32",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors_r32", self.get_id()));
                return self;
            }
        }
        if width * height != data.len() {
            Log::err(format!(
                "{}x{} differ from {} for Tex::set_color_r32 for texture {}",
                height,
                width,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data.as_ptr() as *mut std::os::raw::c_void)
        };
        self
    }

    /// This allows you to attach a z/depth buffer from a rendertarget texture. This texture _must_ be a
    /// rendertarget to set this, and the zbuffer texture _must_ be a depth format (or null). For no-rendertarget
    /// textures, this will always be null.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetZBuffer.html>
    ///
    /// see also [`crate::tex::tex_set_zbuffer`]
    pub fn set_zbuffer(&mut self, tex: Option<Tex>) -> &mut Self {
        if let Some(tex) = tex {
            unsafe { tex_set_zbuffer(self.0.as_ptr(), tex.0.as_ptr()) }
        } else {
            unsafe { tex_set_zbuffer(self.0.as_ptr(), null_mut()) }
        }
        self
    }

    /// This function is dependent on the graphics backend! It will take a texture resource for the current graphics
    /// backend (D3D or GL) and wrap it in a StereoKit texture for use within StereoKit. This is a bit of an advanced
    /// feature.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetNativeSurface.html>
    ///
    /// see also [`crate::tex::tex_set_surface`]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_native_surface(
        &mut self,
        native_surface: *mut std::os::raw::c_void,
        r#type: TexType,
        native_fmt: i64,
        width: i32,
        height: i32,
        surface_count: i32,
        owned: bool,
    ) -> &mut Self {
        unsafe {
            tex_set_surface(
                self.0.as_ptr(),
                native_surface,
                r#type,
                native_fmt,
                width,
                height,
                surface_count,
                1,
                1,
                owned as Bool32T,
            )
        };
        self
    }

    /// Set the texture’s size without providing any color data. In most cases, you should probably just call SetColors
    /// instead, but this can be useful if you’re adding color data some other way, such as when blitting or rendering
    /// to it.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetSize.html>
    ///
    /// see also [`crate::tex::tex_set_size`]
    pub fn set_size(&mut self, width: usize, height: usize) -> &mut Self {
        unsafe { tex_set_colors(self.0.as_ptr(), width as i32, height as i32, null_mut()) };
        self
    }

    /// This will override the default fallback texture that gets used before the Tex has finished loading. This is
    /// useful for textures with a specific purpose where the normal fallback texture would appear strange, such as a
    /// metal/rough map.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FallbackOverride.html>
    ///
    ///  see also [`crate::tex::tex_set_fallback`]
    pub fn fallback_override<T: AsRef<Tex>>(&mut self, fallback: T) -> &mut Self {
        unsafe { tex_set_fallback(self.0.as_ptr(), fallback.as_ref().0.as_ptr()) };
        self
    }

    /// When sampling a texture that’s stretched, or shrunk beyond its screen size, how do we handle figuring out which
    /// color to grab from the texture? Default is Linear.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SampleMode.html>
    ///
    ///  see also [`crate::tex::tex_set_sample`]
    pub fn sample_mode(&mut self, sample: TexSample) -> &mut Self {
        unsafe { tex_set_sample(self.0.as_ptr(), sample) };
        self
    }

    //// When looking at a UV texture coordinate on this texture, how do we handle values larger than 1, or less than zero?
    /// Do we Wrap to the other side? Clamp it between 0-1, or just keep Mirroring back and forth? Wrap is the default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddressMode.html>
    ///
    ///  see also [`crate::tex::tex_set_address`]
    pub fn address_mode(&mut self, address_mode: TexAddress) -> &mut Self {
        unsafe { tex_set_address(self.0.as_ptr(), address_mode) };
        self
    }

    /// When SampleMode is set to Anisotropic, this is the number of samples the GPU takes to figure out the correct color.
    /// Default is 4, and 16 is pretty high.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisoptropy.html>
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisotropy.html>
    ///
    ///  see also [`crate::tex::tex_set_anisotropy`]
    pub fn anisotropy(&mut self, anisotropy_level: i32) -> &mut Self {
        unsafe { tex_set_anisotropy(self.0.as_ptr(), anisotropy_level) };
        self
    }

    /// Gets the unique identifier of this asset resource! This can be helpful for debugging, managine your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Tex/Id.html>
    ///
    /// see also [`crate::tex::tex_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(tex_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Textures are loaded asyncronously, so this tells you the current state of this texture! This also can tell if
    /// an error occured, and what type of error it may have been.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AssetState.html>
    ///
    /// see also [`crate::tex::tex_asset_state`]
    pub fn get_asset_state(&self) -> AssetState {
        unsafe { tex_asset_state(self.0.as_ptr()) }
    }

    /// The StereoKit format this texture was initialized with. This will be a blocking call if AssetState is less than
    /// LoadedMeta so None will be return instead
    /// <https://stereokit.net/Pages/StereoKit/Tex/Format.html>
    ///
    /// see also [`crate::tex::tex_get_format`]
    pub fn get_format(&self) -> Option<TexFormat> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_format(self.0.as_ptr()) })
    }

    /// This allows you to retreive a z/depth buffer from a rendertarget texture. This texture _must_ be a
    /// rendertarget to set this, and the zbuffer texture _must_ be a depth format (or null). For no-rendertarget
    /// textures, this will always be null.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetZBuffer.html>
    ///
    /// see also [`crate::tex::tex_get_zbuffer`]
    pub fn get_zbuffer(&self) -> Option<Tex> {
        NonNull::new(unsafe { tex_get_zbuffer(self.0.as_ptr()) }).map(Tex)
    }

    /// This will return the texture’s native resource for use with external libraries. For D3D, this will be an
    /// ID3D11Texture2D*, and for GL, this will be a uint32_t from a glGenTexture call, coerced into the IntPtr. This
    /// call will block execution until the texture is loaded, if it is not already.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetNativeSurface.html>
    ///
    /// see also [`crate::tex::tex_get_surface`]
    pub fn get_native_surface(&self) -> *mut c_void {
        unsafe { tex_get_surface(self.0.as_ptr()) }
    }

    /// The width of the texture, in pixels. This will be a blocking call if AssetState is less than LoadedMeta so None
    /// will be return instead
    /// <https://stereokit.net/Pages/StereoKit/Tex/Width.html>
    ///
    /// see also [`crate::tex::tex_get_width`]
    pub fn get_width(&self) -> Option<usize> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_width(self.0.as_ptr()) } as usize)
    }

    /// The height of the texture, in pixels. This will be a blocking call if AssetState is less than LoadedMeta so None
    /// will be return instead
    /// <https://stereokit.net/Pages/StereoKit/Tex/Height.html>
    ///
    /// see also [`crate::tex::tex_get_height`]
    pub fn get_height(&self) -> Option<usize> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_height(self.0.as_ptr()) } as usize)
    }

    /// Non-canon function which returns a tuple made of (width, heigh, size) of the corresponding texture.
    ///
    /// use mip < 0 for textures using [TexType::IMAGE_NO_MIPS]
    ///
    /// use mip >=0 to retrieve the info about one MIP of the texture
    ///
    /// the size corresponding to the mip texture and the width and height of this mip texture
    /// This will be a blocking call if AssetState is less than LoadedMeta so None will be return instead
    pub fn get_data_infos(&self, mip: i8) -> Option<(usize, usize, usize)> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => {
                Log::err(format!("Texture {} not loaded. Function tex_get_data_info failed!", self.get_id()));
                return None;
            }
        }
        let mut width = unsafe { tex_get_width(self.0.as_ptr()) } as usize;
        let mut height = unsafe { tex_get_height(self.0.as_ptr()) } as usize;
        let size_test;
        let mut mips_test = unsafe { tex_get_mips(self.0.as_ptr()) } as usize;

        if mip >= mips_test as i8 {
            Log::err(format!(
                "Texture {} has only {} mips. Index {} is too high. Function tex_get_data_info failed!",
                self.get_id(),
                mips_test,
                mip
            ));
            return None;
        }

        let deux: usize = 2;
        if mip <= 0 {
            size_test = width * height;
        } else {
            mips_test = deux.pow(mip as u32);
            width /= mips_test;
            height /= mips_test;

            size_test = width * height;
        }
        Some((width, height, size_test))
    }

    /// Retrieve the color data of the texture from the GPU. This can be a very slow operation,
    /// so use it cautiously. The out_data pointer must correspond to an array with the correct size.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetColorData.html>
    /// * mip_level - Retrieves the color data for a specific mip-mapping level. This function will log a fail and
    ///   return a black array if an invalid mip-level is provided.
    ///
    /// The function [`Tex::get_data_infos`] may help you to shape the right receiver.
    ///
    /// see also [`crate::tex::tex_get_data`][`crate::tex::tex_get_data_mip`]
    pub fn get_color_data<T>(&self, color_data: &[T], mut mip_level: i8) -> bool {
        let size_of_color = std::mem::size_of_val(color_data);
        let (width, height, size_test) = match self.get_data_infos(mip_level) {
            Some(value) => value,
            None => return false,
        };
        if size_test * size_of::<T>() != size_of_color {
            Log::err(format!(
                "Size of the Tex {} is {}x{}/mip={} when size of the given buffer is {} instead of {}. Function Tex::get_color failed!",
                self.get_id(),
                height,
                width,
                mip_level,
                size_of_color,
                size_test * size_of::<T>(),
            ));
            return false;
        }

        if mip_level < 0 {
            mip_level = 0
        }
        unsafe {
            tex_get_data(
                self.0.as_ptr(),
                color_data.as_ptr() as *mut std::os::raw::c_void,
                size_of_color,
                mip_level as i32,
            )
        };

        true
    }

    /// Non canonical function!
    /// Retrieve the color data of the texture from the GPU. This can be a very slow operation,
    /// so use it cautiously. The out_data pointer must correspond to an u8 array with the correct size.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetColorData.html>
    /// * color_size: number of bytes of the color (Color32: 4, Color128: 16 ...)
    /// * mip_level - Retrieves the color data for a specific mip-mapping level. This function will log a fail and
    ///   return a black array if an invalid mip-level is provided.
    ///
    /// The function [`Tex::get_data_infos`] may help you to shape the right receiver.
    ///
    /// see also [`crate::tex::tex_get_data`][`crate::tex::tex_get_data_mip`]
    pub fn get_u8_color_data(&self, color_data: &[u8], color_size: usize, mut mip_level: i8) -> bool {
        let size_of_color = std::mem::size_of_val(color_data);
        let (width, height, size_test) = match self.get_data_infos(mip_level) {
            Some(value) => value,
            None => return false,
        };

        if size_test * color_size != size_of_color {
            Log::err(format!(
                "Size of the Tex {} is {}x{}/mip={} when size of the given buffer is {} instead of {}. Function Tex::get_u8_color failed!",
                self.get_id(),
                height,
                width,
                mip_level,
                size_of_color,
                size_test * color_size,
            ));
            return false;
        }

        if mip_level < 0 {
            mip_level = 0
        }
        unsafe {
            tex_get_data(
                self.0.as_ptr(),
                color_data.as_ptr() as *mut std::os::raw::c_void,
                size_of_color,
                mip_level as i32,
            )
        };

        true
    }

    /// When sampling a texture that’s stretched, or shrunk beyond its screen size, how do we handle figuring out which
    /// color to grab from the texture? Default is Linear.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SampleMode.html>
    ///
    /// see also [`crate::tex::tex_get_sample`]
    pub fn get_sample_mode(&self) -> TexSample {
        unsafe { tex_get_sample(self.0.as_ptr()) }
    }

    /// When looking at a UV texture coordinate on this texture, how do we handle values larger than 1, or less than
    /// zero? Do we Wrap to the other side? Clamp it between 0-1, or just keep Mirroring back and forth? Wrap is the
    /// default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddressMode.html>
    ///
    /// see also [`crate::tex::tex_get_address`]
    pub fn get_address_mode(&self) -> TexAddress {
        unsafe { tex_get_address(self.0.as_ptr()) }
    }

    /// When SampleMode is set to Anisotropic, this is the number of samples the GPU takes to figure out the correct
    /// color. Default is 4, and 16 is pretty high.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisoptropy.html>
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisotropy.html>
    ///
    /// see also [`crate::tex::tex_get_anisotropy`]
    pub fn get_anisotropy(&self) -> i32 {
        unsafe { tex_get_anisotropy(self.0.as_ptr()) }
    }

    /// The number of mip-map levels this texture has. This will be 1 if the texture doesn’t have mip mapping enabled.
    /// This will be a blocking call if AssetState is less than LoadedMeta so None will be return instead.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Mips.html>
    ///
    /// see also [`crate::tex::tex_get_mips`]
    pub fn get_mips(&self) -> Option<i32> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_mips(self.0.as_ptr()) })
    }

    /// Get the associated lighting extracted from the cubemap.
    /// <https://stereokit.net/Pages/StereoKit/Tex/CubemapLighting.html>
    ///
    /// see also [`crate::tex::tex_gen_cubemap_sh`]
    pub fn get_cubemap_lighting(&self) -> SHCubemap {
        SHCubemap {
            sh: unsafe { tex_get_cubemap_lighting(self.0.as_ptr()) },
            tex: Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.0.as_ptr())) }).unwrap()),
        }
    }

    /// Default 2x2 black opaque texture, this is the texture referred to as ‘black’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Black.html>
    pub fn black() -> Self {
        Self::find("default/tex_black").unwrap()
    }

    /// This is a white checkered grid texture used to easily add visual features to materials. By default, this is used
    /// for the loading fallback texture for all Tex objects.
    /// <https://stereokit.net/Pages/StereoKit/Tex/DevTex.html>
    pub fn dev_tex() -> Self {
        Self::find("default/tex_devtex").unwrap()
    }

    /// This is a red checkered grid texture used to indicate some sort of error has occurred. By default, this is used
    /// for the error fallback texture for all Tex objects.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Error.html>
    pub fn error() -> Self {
        Self::find("default/tex_error").unwrap()
    }

    /// Default 2x2 flat normal texture, this is a normal that faces out from the, face, and has a color value of
    /// (0.5,0.5,1). This is the texture referred to as ‘flat’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Flat.html>
    pub fn flat() -> Self {
        Self::find("default/tex_flat").unwrap()
    }

    /// Default 2x2 middle gray (0.5,0.5,0.5) opaque texture, this is the texture referred to as ‘gray’ in the shader
    /// texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Gray.html>
    pub fn gray() -> Self {
        Self::find("default/tex_gray").unwrap()
    }

    /// Default 2x2 roughness color (1,1,0,1) texture, this is the texture referred to as ‘rough’ in the shader texture
    /// defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Rough.html>
    pub fn rough() -> Self {
        Self::find("default/tex_rough").unwrap()
    }

    /// Default 2x2 white opaque texture, this is the texture referred to as ‘white’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/White.html>
    pub fn white() -> Self {
        Self::find("default/tex").unwrap()
    }

    // /// The equirectangular texture used for the default dome
    // /// <https://stereokit.net/Pages/StereoKit/Tex.html>
    // pub fn cubemap() -> Self {
    //     Self::find("default/tex_cubemap").unwrap()
    // }
}

/// fluent syntax for Texture cubemap
/// <https://stereokit.net/Pages/StereoKit/Tex.html>
///
/// see also [`crate::tex::Tex`] [crate::util::SphericalHarmonics]
#[derive(Debug)]
pub struct SHCubemap {
    pub sh: SphericalHarmonics,
    pub tex: Tex,
}

impl SHCubemap {
    /// Creates a cubemap texture from a single equirectangular image! You know, the ones that look like an unwrapped
    /// globe with the poles all stretched out. It uses some fancy shaders and texture blitting to create 6 faces from
    /// the equirectangular image.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromCubemapEquirectangular.html>
    ///
    /// see also [`crate::tex::tex_create_cubemap_file`]
    #[deprecated(since = "0.40.0", note = "please use `from_cubemap` instead")]
    pub fn from_cubemap_equirectangular(
        equirectangular_file_utf8: impl AsRef<Path>,
        srgb_data: bool,
        priority: i32,
    ) -> Result<SHCubemap, StereoKitError> {
        Self::from_cubemap(equirectangular_file_utf8, srgb_data, priority)
    }

    /// Creates a cubemap texture from a single file! This will load KTX2 files with 6 surfaces, or convert
    /// equirectangular images into cubemap images. KTX2 files are the _fastest_ way to load a cubemap, but
    /// equirectangular images can be acquired quite easily!
    ///
    /// Equirectangular images look like an unwrapped globe with the poles all stretched out, and are sometimes referred
    /// to as HDRIs.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromCubemap.html>
    /// * cubemap_file - Filename of the cubemap image.
    /// * srgb_data - Is this image color data in sRGB format, or is it normal/metal/rough/data that's not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * load_priority - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// Returns a [SHCubemap]
    ///
    /// see also [`crate::tex::tex_create_cubemap_file`]
    pub fn from_cubemap(
        cubemap_file: impl AsRef<Path>,
        srgb_data: bool,
        load_priority: i32,
    ) -> Result<SHCubemap, StereoKitError> {
        let path = cubemap_file.as_ref();
        let path_buf = path.to_path_buf();
        let c_str = CString::new(path.to_str().ok_or(StereoKitError::TexCString(path.to_str().unwrap().to_owned()))?)?;
        let tex =
            Tex(
                NonNull::new(unsafe { tex_create_cubemap_file(c_str.as_ptr(), srgb_data as Bool32T, load_priority) })
                    .ok_or(StereoKitError::TexFile(path_buf.clone(), "tex_create_cubemap_file failed".to_string()))?,
            );

        Ok(Tex::get_cubemap_lighting(&tex))
    }

    /// Creates a cubemap texture from 6 different image files! If you have a single equirectangular image, use
    /// Tex.FromEquirectangular instead. Asset Id will be the first filename.
    /// order of the file names is +X -X +Y -Y +Z -Z
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromCubemapFile.html>
    /// * files_utf8 - 6 image filenames, in order of/ +X, -X, +Y, -Y, +Z, -Z.
    /// * srgb_data - Is this image color data in sRGB format, or is it normal/metal/rough/data that's not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have a
    ///   big impact on visuals.
    /// * load_priority - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// Returns a SHCubemap from the given files, or Err if any failed to load.
    /// see also [`crate::tex::tex_create_cubemap_files`]
    pub fn from_cubemap_files<P: AsRef<Path>>(
        files_utf8: &[P; 6],
        srgb_data: bool,
        load_priority: i32,
    ) -> Result<SHCubemap, StereoKitError> {
        let mut c_files = Vec::new();
        for path in files_utf8 {
            let path = path.as_ref();
            let path_buf = path.to_path_buf();
            let c_str =
                CString::new(path.to_str().ok_or(StereoKitError::TexCString(path_buf.to_str().unwrap().to_owned()))?)?;
            c_files.push(c_str);
        }
        let mut c_files_ptr = Vec::new();
        for str in c_files.iter() {
            c_files_ptr.push(str.as_ptr());
        }
        let in_arr_cube_face_file_xxyyzz = c_files_ptr.as_mut_slice().as_mut_ptr();
        let tex = Tex(NonNull::new(unsafe {
            tex_create_cubemap_files(in_arr_cube_face_file_xxyyzz, srgb_data as Bool32T, load_priority)
        })
        .ok_or(StereoKitError::TexFiles(
            PathBuf::from(r"one_of_6_files"),
            "tex_create_cubemap_files failed".to_string(),
        ))?);

        //Ok(Tex::get_cubemap_lighting(&tex))
        Ok(SHCubemap { sh: SphericalHarmonics::default(), tex })
    }

    /// Generates a cubemap texture from a gradient and a direction! These are entirely suitable for skyboxes, which
    /// you can set via Renderer.SkyTex.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenCubemap.html>
    ///
    /// see also [`crate::tex::tex_gen_cubemap`]
    pub fn gen_cubemap_gradient(
        gradient: impl AsRef<Gradient>,
        gradient_dir: impl Into<Vec3>,
        resolution: i32,
    ) -> SHCubemap {
        let mut sh = SphericalHarmonics::default();
        let tex = Tex(NonNull::new(unsafe {
            tex_gen_cubemap(gradient.as_ref().0.as_ptr(), gradient_dir.into(), resolution, &mut sh)
        })
        .unwrap());
        //unsafe { sk.tex_addref(&cubemap.1) }
        SHCubemap { sh, tex }
    }

    /// Create the associated cubemap texture with the light spot.
    /// warning ! The SphericalHarmonics is moved to the result struct.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenCubemap.html>
    ///
    /// see also [`crate::tex::tex_gen_cubemap_sh`]
    pub fn gen_cubemap_sh(
        lookup: SphericalHarmonics,
        face_size: i32,
        light_spot_size_pct: f32,
        light_spot_intensity: f32,
    ) -> SHCubemap {
        let tex = Tex(NonNull::new(unsafe {
            tex_gen_cubemap_sh(&lookup, face_size, light_spot_size_pct, light_spot_intensity)
        })
        .unwrap());
        SHCubemap { sh: lookup, tex }
    }

    /// Get the associated lighting extracted from the cubemap.
    /// <https://stereokit.net/Pages/StereoKit/Tex/CubemapLighting.html>
    ///
    /// see also [`crate::tex::tex_gen_cubemap_sh`]
    pub fn get_cubemap_lighting(cubemap_texture: impl AsRef<Tex>) -> SHCubemap {
        SHCubemap {
            sh: unsafe { tex_get_cubemap_lighting(cubemap_texture.as_ref().0.as_ptr()) },
            tex: Tex(NonNull::new(unsafe { tex_find(tex_get_id(cubemap_texture.as_ref().0.as_ptr())) }).unwrap()),
        }
    }

    /// Get the cubemap texture and SH light of the the current skylight
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`crate::system::Renderer`]
    pub fn get_rendered_sky() -> SHCubemap {
        SHCubemap {
            sh: unsafe { render_get_skylight() },
            tex: Tex(NonNull::new(unsafe { render_get_skytex() }).unwrap()),
        }
    }

    /// set the spherical harmonics as skylight and the the cubemap texture as skytex
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also see also [`crate::system::Renderer`]
    pub fn render_as_sky(&self) {
        unsafe {
            render_set_skylight(&self.sh);
            render_set_skytex(self.tex.0.as_ptr());
        }
    }

    /// Enabled or disabled the rendering of the skytex cubemap texture
    /// <https://stereokit.net/Pages/StereoKit/Renderer/EnableSky.html>
    ///
    /// see also see also [`crate::system::Renderer`] [`crate::tex::Tex`]
    pub fn render_enabled_skytex(&mut self, enable: bool) -> &mut Self {
        unsafe { render_enable_skytex(enable as Bool32T) };
        self
    }

    /// Get the cubemap tuple
    ///
    /// see also [`crate::tex::Tex`] [`crate::util::SphericalHarmonics`]
    pub fn get(&self) -> (SphericalHarmonics, Tex) {
        (self.sh, Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.tex.0.as_ptr())) }).unwrap()))
    }
}
