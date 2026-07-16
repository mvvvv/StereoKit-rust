use crate::{
    StereoKitError,
    maths::{Bool32T, Vec3},
    render::{render_get_skylight, render_get_skytex, render_set_skylight, render_set_skytex},
    system::{AssetState, IAsset, Log},
    util::{Color32, Color128, Gradient, GradientKey, GradientT, SphericalHarmonics},
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    mem::size_of,
    path::{Path, PathBuf},
    ptr::{NonNull, null_mut},
};

bitflags::bitflags! {
    /// Textures come in various types and flavors! These are bit-flags
    /// that tell StereoKit what type of texture we want; and how the application
    /// might use it!
    /// <https://stereokit.net/Pages/StereoKit/TexType.html>
    ///
    /// see also [`Tex`]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct TexType: u32 {
        /// A standard color image, without any generated mip-maps.
        const ImageNomips  = 1 << 0;
        /// A size sided texture that's used for things like skyboxes, environment maps, and reflection probes. It
        /// behaves like a texture array with 6 textures.
        const Cubemap      = 1 << 1;
        /// This texture can be rendered to! This is great for textures that might be passed in as a target to
        /// Renderer.Blit, or other such situations.
        const Rendertarget = 1 << 2;
        /// This texture contains depth data, not color data! It is writeable, but not readable. This makes it great
        /// for zbuffers, but not shadowmaps or other textures that need to be read from later on.
        const Depth        = 1 << 3;
        /// This texture contains depth data, not color data! It is writeable, but not readable. This makes it great
        /// for zbuffers, but not shadowmaps or other textures that need to be read from later on.
        const Zbuffer      = 1 << 3;
        /// This texture will generate mip-maps any time the contents change. Mip-maps are a list of textures that are
        /// each half the size of the one before them! This is used to prevent textures from 'sparkling' or aliasing in
        /// the distance.
        const Mips         = 1 << 4;
        /// This texture's data will be updated frequently from the CPU (not renders)! This ensures the graphics card
        /// stores it someplace where writes are easy to do quickly.
        const Dynamic      = 1 << 5;
        /// This texture contains depth data, not color data! It is writeable and readable. This makes it great for
        /// shadowmaps or other textures that need to be read from later on.
        const Depthtarget  = 1 << 6;
        /// This texture can be used as a RWTexture in compute shaders. Create it with a format
        /// that supports storage images, such as [`TexFormat::Rgba128`].
        const Compute      = 1 << 7;
        /// A volumetric (3D) texture, sized with width, height, and depth. Volume textures are mutually exclusive with
        /// Cubemap and array textures, and don't pair with a zbuffer.
        const Volume       = 1 << 8;
        /// A standard color image that also generates mip-maps automatically.
        const Image        = Self::ImageNomips.bits() | Self::Mips.bits();
    }
}
impl TexType {
    pub fn as_u32(&self) -> u32 {
        self.bits()
    }
}
/// What type of color information will the texture contain? A good default here is Rgba32, which gives 8-bit sRGB
/// color with alpha! Most format names end in a short suffix telling you how the GPU interprets the bits when sampled
/// in a shader:
/// - no suffix or "un": unsigned normalized. Raw unsigned integers get normalized into the \[0,1\] floating point
///   range on read. The default flavor for most color and data formats.
/// - "sn": signed normalized. Raw signed integers get normalized into the \[-1,1\] floating point range on read.
/// - "ui": unsigned integer. Raw unsigned integers, no normalization! Great for IDs, counters, and exact-integer data.
/// - "si": signed integer. Raw signed integers, no normalization.
/// - "f": signed float, typically an IEEE half or single precision float.
/// - "uf": unsigned float, used by some HDR-leaning compact formats that can only represent non-negative values.
/// - "_srgb": stored in sRGB color space! The GPU auto-converts to linear when sampled and back to sRGB when written.
///   Use this for images viewed by humans, like photos and UI artwork.
/// - "_linear": stored in linear color space, no color-space conversion at sample time. Use this for data textures,
///   like normals, masks, roughness, and metallic. Any format that is _not_ "_srgb" is generally linear.
///
/// Block-compressed formats (BC, ETC, ASTC, PVRTC, ATC) trade a little quality for a big drop in memory and bandwidth:
/// each format packs an NxN block of pixels into a fixed payload, so cost is measured in bits-per-pixel rather than
/// bits-per-channel. Hardware support varies - prefer BC on desktop/console, ASTC on modern mobile. They're
/// sample-only; you can't render to them.
/// <https://stereokit.net/Pages/StereoKit/TexFormat.html>
///
/// see also [`Tex`] [`crate::render::Renderer`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TexFormat {
    /// Default zero value for TexFormat! Uninitialized formats land here and **** **** up so you know to assign one
    /// properly :)
    None = 0,
    /// 8-bit sRGB R/G/B/A. The default for human-viewed color images, and a clean match for the Color32 struct! For
    /// data textures (normals, masks, rough/metal) use Rgba32Linear instead.
    Rgba32Srgb = 1,
    /// 8-bit linear R/G/B/A. Use this for data textures (normals, masks, rough/metal) where you don't want the GPU's
    /// automatic sRGB conversion getting in the way.
    Rgba32Linear = 2,
    /// 8-bit sRGB B/G/R/A. Same as Rgba32Srgb but with R and B swapped to match the byte order some GPUs and Windows
    /// swapchains prefer. Most code can stick with Rgba32Srgb!
    Bgra32Srgb = 3,
    /// 8-bit linear B/G/R/A. Same as Rgba32Linear but with R and B swapped, mostly for compatibility with
    /// BGRA-preferring APIs like Windows swapchains.
    Bgra32Linear = 4,
    /// 16-bit unsigned-normalized R/G/B/A (64 bpp). Doubling the bit depth over Rgba32 gives much smoother gradients!
    Rgba64un = 5,
    /// 16-bit signed-normalized R/G/B/A (64 bpp).
    Rgba64sn = 6,
    /// 16-bit unsigned-integer R/G/B/A (64 bpp). Great for ID textures, counters, or any discrete-integer data.
    /// For \[0,1\] sampling, use Rgba64un instead.
    Rgba64ui = 7,
    /// 16-bit signed-integer R/G/B/A (64 bpp). For \[-1,1\] sampling, use Rgba64sn instead.
    Rgba64si = 8,
    /// 16-bit half-float R/G/B/A (64 bpp). A common HDR render-target format - full RGBA float precision at half the
    /// memory of Rgba128. Almost always supported as a render target, so a reliable fallback for formats like Rg11b10.
    Rgba64f = 9,
    /// 32-bit float R/G/B/A - basically 4 single-precision floats per pixel, which is bonkers expensive at 128 bpp!
    /// Don't reach for this unless you know -exactly- what you're doing. Useful for scientific data or compute buffers
    /// where you really need full 32-bit float precision per channel.
    Rgba128 = 10,
    /// Packed HDR R/G/B as unsigned floats - 11 bits for R and G, 10 for B, no alpha. A great compact HDR format: holds
    /// values way beyond the \[0,1\] range that Rgba32 maxes out at, while still fitting in 32 bpp! Great for HDR
    /// render targets and intermediate compute buffers. Not universally supported as a render target, so watch for that!
    Rg11b10 = 11,
    /// Packed unsigned-normalized R/G/B/A with 10 bits per color channel and 2 bits for alpha. A great presentation
    /// format for high bit-depth displays that still fits in 32 bpp, and you get a bit of transparency too! Alpha is
    /// effectively on/off/halfway though, so skip this if you need smooth alpha. Not universally supported as a render
    /// target!
    Rgb10a2 = 12,
    /// Shared-exponent HDR R/G/B with 9-bit mantissa per channel and a 5-bit shared exponent. A compact HDR format that
    /// packs values way beyond the \[0,1\] range into just 32 bpp! No alpha though, and sharing the exponent means all
    /// three channels need similar magnitudes - perfect for environment maps!
    /// Usually sample-only; GPUs typically can't render to it.
    Rgb9e5 = 13,
    /// 8-bit unsigned-normalized single channel. Great when you only need one channel and want to keep memory down.
    R8 = 14,
    /// 8-bit signed-normalized single channel. Useful for a single signed value like an elevation difference or signed
    /// mask.
    R8sn = 15,
    /// 8-bit unsigned-integer single channel. Good for small IDs, indices, or stencil-like data accessed as exact
    /// integers.
    R8ui = 16,
    /// 8-bit signed-integer single channel.
    R8si = 17,
    /// 8-bit sRGB single channel. Useful for single-channel sRGB data like a luminance map that should be linearized
    /// before lighting math.
    R8Srgb = 18,
    /// Two 8-bit unsigned-normalized channels (R, G). Useful for two-component data like compressed normals where the
    /// third axis is reconstructed in the shader, or two grayscale signals stored side by side.
    R8g8 = 19,
    /// 16-bit unsigned-normalized single channel. A good format for height maps, since it stores a fair bit of
    /// information!
    R16un = 20,
    /// 16-bit signed-normalized single channel. Good for signed height data or signed distance fields.
    R16sn = 21,
    /// 16-bit unsigned-integer single channel. A great format for index or ID data, since values are accessed as raw
    /// integers.
    R16ui = 22,
    /// 16-bit signed-integer single channel. Good for signed integer or ID data.
    R16si = 23,
    /// 16-bit half-float single channel. Good for HDR height/depth data that needs a range beyond what normalized
    /// formats give you.
    R16f = 24,
    /// 32-bit unsigned-integer single channel. Useful for counters, IDs, and atomic compute operations.
    R32ui = 25,
    /// 32-bit signed-integer single channel.
    R32si = 26,
    /// 32-bit single-precision float single channel. Treats each pixel as a generic float, so you can do all sorts of
    /// strange and interesting things with this! Great for scientific data, signed distance fields, or detailed height
    /// fields where 16 bits of precision aren't enough.
    R32f = 27,
    /// 16-bit depth - not a lot, but it can be enough if your far clipping plane is pretty close. If you're seeing
    /// z-fighting, either bring your far clip in or switch to 24/32-bit depth.
    Depth16 = 28,
    /// 16-bit depth + 8-bit stencil. A compact depth-with-stencil option for when precision needs are modest and
    /// memory is tight. If you see z-fighting, step up to Depth24s8 or Depth32s8.
    Depth16s8 = 29,
    /// 24-bit depth + 8-bit stencil. Depth tracks how close to the camera each pixel is so near objects correctly
    /// occlude far ones. Stencil data can be used for clipping effects, deferred rendering, or shadow effects. A
    /// sensible default for most scenes!
    Depth24s8 = 30,
    /// 32-bit depth. Pretty detailed, and excellent for experiences with very far view distances. No stencil bits
    /// though - if you need stencil too, use Depth32s8 instead.
    Depth32 = 31,
    /// 32-bit depth + 8-bit stencil (40 bpp). More depth precision than Depth24s8 but heavier on memory. Use this when
    /// you need both 32-bit depth precision and a stencil channel for masking effects.
    Depth32s8 = 32,
    /// BC1/DXT1 sRGB RGB, no alpha, 4 bpp. Each 4x4 block of pixels gets squished into 8 bytes, so a texture only
    /// takes a quarter of Rgba32's memory. Quality is good for opaque diffuse textures, though artifacts can show up
    /// in smooth gradients. Widely supported on desktop and console GPUs - not so much on mobile.
    Bc1RgbSrgb = 33,
    /// BC1/DXT1 linear RGB, no alpha, 4 bpp. Great for compressed data textures (normals, masks) on desktop and
    /// console GPUs. For color images for humans, use Bc1RgbSrgb.
    Bc1Rgb = 34,
    /// BC1/DXT1 sRGB with 1-bit alpha, 4 bpp. Alpha is either fully on or fully off per pixel - great for cutout
    /// effects like foliage or chain-link fences. Smooth fade-outs will band hard though; reach for Bc3 or Bc7 for
    /// smooth alpha.
    Bc1RgbaSrgb = 35,
    /// BC1/DXT1 linear with 1-bit alpha, 4 bpp. Good for opaque data textures with a sharp cutout mask on desktop and
    /// console GPUs. For smooth alpha, reach for Bc3 or Bc7 instead.
    Bc1Rgba = 36,
    /// BC2/DXT3 sRGB with explicit 4-bit alpha, 8 bpp. Alpha gets 16 discrete levels - fine for blocky or dithered
    /// alpha but bands hard on smooth gradients. Bc3 is usually a better choice for smooth alpha; Bc2 is mostly
    /// historical.
    Bc2RgbaSrgb = 37,
    /// BC2/DXT3 linear with explicit 4-bit alpha, 8 bpp. Bc3 is usually preferred for smooth alpha gradients; Bc2 is
    /// mostly historical.
    Bc2Rgba = 38,
    /// BC3/DXT5 sRGB color with smooth alpha, 8 bpp. Alpha is BC4-compressed, giving much better gradients than Bc1 or
    /// Bc2. A solid default for color-with-alpha textures on desktop and console GPUs!
    Bc3RgbaSrgb = 39,
    /// BC3/DXT5 linear color with smooth alpha, 8 bpp. Great for compressed data textures with alpha (RGBA masks) on
    /// desktop and console GPUs.
    Bc3Rgba = 40,
    /// BC4 unsigned-normalized single channel \[0,1\], 4 bpp. Ideal for compressed grayscale textures like heightmaps,
    /// ambient occlusion, or single-channel masks. Quality is excellent for smooth single-channel data.
    Bc4R = 41,
    /// BC4 signed-normalized single channel \[-1,1\], 4 bpp. Useful when your data is naturally signed, like signed
    /// distance fields or elevation difference maps.
    Bc4Rsn = 42,
    /// BC5 unsigned-normalized two channels, 8 bpp. Effectively two BC4 textures packed together. The standard format
    /// for compressed two-channel data on desktop/console - most commonly used for tangent-space normal maps where the
    /// Z component is reconstructed in the shader!
    Bc5Rg = 43,
    /// BC5 signed-normalized two channels (\[-1,1\] per channel), 8 bpp. Useful for signed two-channel data, like
    /// normal maps stored as \[-1,1\] directly rather than the typical \[0,1\] packed form.
    Bc5Rgsn = 44,
    /// BC6H HDR RGB, unsigned float (positive values only), 8 bpp. 16-bit half-float per channel, no alpha. The go-to
    /// format for compressing HDR cubemaps and environment maps - stores high-dynamic-range data at a fraction of the
    /// cost of Rgba64f.
    Bc6hRgbuf = 45,
    /// BC6H HDR RGB, signed float (can store negative values), 8 bpp. 16-bit half-float per channel, no alpha. Use
    /// this when your HDR data can contain negatives, like signed spherical harmonics coefficients.
    Bc6hRgbf = 46,
    /// BC7 sRGB color with full alpha, 8 bpp. The highest-quality BC format - noticeably better than Bc3 at the same
    /// compression ratio. Compression takes longer than Bc3 though, so reach for this when quality matters more than
    /// encoding speed.
    Bc7RgbaSrgb = 47,
    /// BC7 linear color with full alpha, 8 bpp. Highest-quality BC format - excellent for compressed RGBA data
    /// textures when Bc3 quality isn't enough.
    Bc7Rgba = 48,
    /// ETC1 RGB, no alpha, 4 bpp. Widely supported on older Android devices and OpenGL ES 2.0+ GPUs. Quality is
    /// acceptable for diffuse color but it's been superseded - prefer Etc2 or Astc on newer hardware!
    Etc1Rgb = 49,
    /// ETC2 sRGB color with full alpha, 8 bpp. The standard compressed RGBA format on OpenGL ES 3.0+ mobile devices,
    /// and mandatory in the spec - so it's widely available. A great default for sRGB color textures on mobile!
    Etc2RgbaSrgb = 50,
    /// ETC2 linear color with full alpha, 8 bpp. Standard compressed format for data textures with alpha on OpenGL ES
    /// 3.0+ mobile devices.
    Etc2Rgba = 51,
    /// ETC2/EAC single 11-bit unsigned-normalized channel, 4 bpp.
    /// The ETC equivalent of Bc4 - great for compressed grayscale
    /// or heightmap data on mobile GPUs!
    Etc2R11 = 52,
    /// ETC2/EAC two 11-bit unsigned-normalized channels, 8 bpp. The ETC equivalent of Bc5 - great for compressed
    /// two-channel data like tangent-space normal maps on mobile GPUs!
    Etc2Rg11 = 53,
    /// PVRTC1 sRGB RGB, 2 bpp. Used on iOS and other PowerVR GPUs. The 2bpp bitrate is super compact but quality is
    /// lower than ETC/BC - acceptable for low-detail or background textures. Requires power-of-two square textures!
    Pvrtc1RgbSrgb = 54,
    /// PVRTC1 linear RGB, 2 bpp. PowerVR GPUs only, requires power-of-two square textures.
    Pvrtc1Rgb = 55,
    /// PVRTC1 sRGB with full alpha, 4 bpp. The 4bpp variant is higher quality than the 2bpp variants. PowerVR GPUs
    /// only, requires power-of-two square textures.
    Pvrtc1RgbaSrgb = 56,
    /// PVRTC1 linear with full alpha, 4 bpp. PowerVR GPUs only, requires power-of-two square textures.
    Pvrtc1Rgba = 57,
    /// PVRTC2 sRGB with full alpha, 4 bpp. An update to PVRTC1 with better quality and fewer restrictions - works with
    /// non-power-of-two and non-square textures. Still PowerVR-specific though.
    Pvrtc2RgbaSrgb = 58,
    /// PVRTC2 linear with full alpha, 4 bpp. Better quality and more flexible texture sizes than PVRTC1. PowerVR GPUs
    /// only.
    Pvrtc2Rgba = 59,
    /// ASTC 4x4 sRGB color with full alpha, 8 bpp. ASTC is the modern mobile-standard compressed format - excellent
    /// quality, broadly supported. The 4x4 block size is the highest-quality (and largest-size) ASTC variant.
    Astc4x4RgbaSrgb = 60,
    /// ASTC 4x4 linear color with full alpha, 8 bpp. High-quality compressed format for data textures on modern mobile
    /// GPUs.
    Astc4x4Rgba = 61,
    /// ATC RGB on Qualcomm Adreno GPUs, 4 bpp. Historical Qualcomm-specific format - prefer Astc or Etc2 on newer
    /// Adreno hardware.
    AtcRgb = 62,
    /// ATC with alpha on Qualcomm Adreno GPUs, 8 bpp. Historical Qualcomm-specific format - prefer Astc or Etc2 on
    /// newer Adreno hardware.
    AtcRgba = 63,
    /// NV12 video format - a 2-plane 4:2:0 YUV layout! Plane 1 is a full-resolution Y (luminance) plane at 8 bpp,
    /// plane 2 is a half-resolution UV (chrominance) plane with U and V interleaved at 8 bits each. The most common
    /// output format from hardware video decoders!
    Nv12 = 64,
    /// P010 video format - like NV12 but with 10-bit channels stored in 16-bit fields. Full-resolution 10-bit Y plane
    /// plus a half-resolution interleaved 10-bit UV plane. Used for 10-bit HDR video!
    P010 = 65,
    /// A 3-plane 4:2:0 YUV layout - separate Y, U, and V planes each at 8 bpp, with U and V at half resolution. Common
    /// in software video decoders but less common from hardware decoders (which usually output NV12).
    Yuv420p = 66,
}

impl TexFormat {
    /// Returns the number of bytes per pixel for uncompressed formats, or `0` for block-compressed,
    /// depth/stencil, and multi-plane video formats where a simple per-pixel byte count doesn't apply.
    ///
    /// This is useful for interpreting raw pixel buffers, such as those returned by screenshot callbacks.
    /// For `0`-returning formats, the buffer layout is format-specific (block-based or multi-plane) and must be
    /// handled with knowledge of the individual format.
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            // 32-bit RGBA/BGRA (4 bytes)
            Self::Rgba32Srgb | Self::Rgba32Linear | Self::Bgra32Srgb | Self::Bgra32Linear => 4,
            // 64-bit RGBA (8 bytes)
            Self::Rgba64un | Self::Rgba64sn | Self::Rgba64ui | Self::Rgba64si | Self::Rgba64f => 8,
            // 128-bit RGBA (16 bytes)
            Self::Rgba128 => 16,
            // Packed 32-bit (4 bytes)
            Self::Rg11b10 | Self::Rgb10a2 | Self::Rgb9e5 => 4,
            // 8-bit single/dual channel (1-2 bytes)
            Self::R8 | Self::R8sn | Self::R8ui | Self::R8si | Self::R8Srgb => 1,
            Self::R8g8 => 2,
            // 16-bit single channel (2 bytes)
            Self::R16un | Self::R16sn | Self::R16ui | Self::R16si | Self::R16f => 2,
            // 32-bit single channel (4 bytes)
            Self::R32ui | Self::R32si | Self::R32f => 4,
            // Depth/stencil, compressed, and video formats: no simple per-pixel byte count
            _ => 0,
        }
    }

    /// 16-bit unsigned-normalized R/G/B/A (64 bpp). Alias for Rgba64un.
    #[allow(non_upper_case_globals)]
    pub const Rgba64: Self = Self::Rgba64un;

    /// 32-bit float R/G/B/A. Alias for Rgba128.
    #[allow(non_upper_case_globals)]
    pub const Rgba128f: Self = Self::Rgba128;

    /// Packed HDR R/G/B as unsigned floats. Alias for Rg11b10.
    #[allow(non_upper_case_globals)]
    pub const Rg11b10uf: Self = Self::Rg11b10;

    /// Shared-exponent HDR R/G/B. Alias for Rgb9e5.
    #[allow(non_upper_case_globals)]
    pub const Rgb9e5uf: Self = Self::Rgb9e5;

    /// 24-bit depth + 8-bit stencil. Alias for Depth24s8.
    #[allow(non_upper_case_globals)]
    pub const DepthStencil: Self = Self::Depth24s8;

    /// 16-bit unsigned-normalized single channel. Alias for R16un.
    pub const R16: Self = Self::R16un;

    /// 16-bit unsigned-normalized single channel. Alias for R16un.
    #[allow(non_upper_case_globals)]
    pub const R16u: Self = Self::R16un;

    /// 16-bit signed-normalized single channel. Alias for R16sn.
    #[allow(non_upper_case_globals)]
    pub const R16s: Self = Self::R16sn;

    /// 32-bit single-precision float single channel. Alias for R32f.
    pub const R32: Self = Self::R32f;

    /// Alias for Rgba32Srgb for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba32Srgb instead")]
    pub const RGBA32: Self = Self::Rgba32Srgb;

    /// Alias for Rgba32Linear for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba32Linear instead")]
    #[allow(non_upper_case_globals)]
    pub const RGBA32Linear: Self = Self::Rgba32Linear;

    /// Alias for Bgra32Srgb for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Bgra32Srgb instead")]
    pub const BGRA32: Self = Self::Bgra32Srgb;

    /// Alias for Bgra32Linear for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Bgra32Linear instead")]
    #[allow(non_upper_case_globals)]
    pub const BGRA32Linear: Self = Self::Bgra32Linear;

    /// Alias for Rg11b10 for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rg11b10 instead")]
    pub const RG11B10: Self = Self::Rg11b10;

    /// Alias for Rgb10a2 for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgb10a2 instead")]
    pub const RGB10A2: Self = Self::Rgb10a2;

    /// Alias for Rgba64un for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba64un instead")]
    pub const RGBA64U: Self = Self::Rgba64un;

    /// Alias for Rgba64sn for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba64sn instead")]
    pub const RGBA64S: Self = Self::Rgba64sn;

    /// Alias for Rgba64sn for backwards compatibility (old primary variant name).
    #[deprecated(since = "0.0.0", note = "Use Rgba64sn instead")]
    #[allow(non_upper_case_globals)]
    pub const Rgba64s: Self = Self::Rgba64sn;

    /// Alias for Rgba64f for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba64f instead")]
    pub const RGBA64F: Self = Self::Rgba64f;

    /// Alias for Rgba128 for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use Rgba128 instead")]
    pub const RGBA128: Self = Self::Rgba128;

    /// Alias for R8g8 for backwards compatibility.
    #[deprecated(since = "0.0.0", note = "Use R8g8 instead")]
    pub const R8G8: Self = Self::R8g8;
}

/// How does the shader grab pixels from the texture? Or more
/// specifically, how does the shader grab colors between the provided
/// pixels? If you'd like an in-depth explanation of these topics, check
/// out [this exploration of texture filtering]
/// <https://medium.com/@bgolus/sharper-mipmapping-using-shader-based-supersampling-ed7aadb47bec>
/// by graphics wizard Ben Golus.
/// <https://stereokit.net/Pages/StereoKit/TexSample.html>
///
/// see also [`Tex`]
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

/// When sampling from a texture with comparison enabled, the sampler compares the sampled texel value against a
/// reference value and returns a 0 or 1 based on the result. This is primarily useful for shadow mapping techniques,
/// where a depth texture is sampled to determine if a surface is in shadow.
/// <https://stereokit.net/Pages/StereoKit/TexSampleComp.html>
///
/// see also [`Tex`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TexSampleComp {
    /// No comparison is performed, the texture is sampled normally. This is the default behavior for most textures.
    None = 0,
    /// Returns 1 if the reference value is less than the sampled texel value.
    Less = 1,
    /// Returns 1 if the reference value is less than or equal to the sampled texel value.
    LessOrEq = 2,
    /// Returns 1 if the reference value is greater than the sampled texel value.
    Greater = 3,
    /// Returns 1 if the reference value is greater than or equal to the sampled texel value.
    GreaterOrEq = 4,
    /// Returns 1 if the reference value is equal to the sampled texel value.
    Equal = 5,
    /// Returns 1 if the reference value is not equal to the sampled texel value.
    NotEqual = 6,
    /// Always returns 1, regardless of values.
    Always = 7,
    /// Always returns 0, regardless of values.
    Never = 8,
}

/// What happens when the shader asks for a texture coordinate
/// that's outside the texture?? Believe it or not, this happens plenty
/// often!
/// <https://stereokit.net/Pages/StereoKit/TexAddress.html>
///
/// see also [`Tex`]
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
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Matrix, util::named_colors,
///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
///
/// let tex_left = Tex::from_file("textures/open_gltf.jpeg", true, None)
///                    .expect("tex_left should be created");
///
/// let tex_right = Tex::gen_color(named_colors::RED, 1, 1, TexType::Image, TexFormat::Rgba32Srgb);
///
/// let tex_back = Tex::gen_particle(128, 128, 0.2, None);
///
/// let tex_floor = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
///
/// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
/// let material_left  = Material::pbr().tex_copy(tex_left);
/// let material_right = Material::pbr().tex_copy(tex_right);
/// let material_back  = Material::unlit_clip().tex_copy(tex_back);
/// let material_floor = Material::pbr().tex_copy(tex_floor);
///
/// let transform_left  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, 0.0, 90.0]);
/// let transform_right = Matrix::t_r([ 0.5, 0.0, 0.0], [0.0, 0.0,-90.0]);
/// let transform_back  = Matrix::t_r([ 0.0, 0.0,-0.5], [90.0, 0.0, 0.0]);
/// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
///
/// filename_scr = "screenshots/tex.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     plane_mesh.draw(&material_left,  transform_left,  None, None);
///     plane_mesh.draw(&material_right, transform_right, None, None);
///     plane_mesh.draw(&material_back,  transform_back,  None, None);
///     plane_mesh.draw(&material_floor, transform_floor, None, None);
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
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

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _TexT {
    _unused: [u8; 0],
}

/// StereoKit ffi type.
pub type TexT = *mut _TexT;

unsafe impl Send for Tex {}
unsafe impl Sync for Tex {}

unsafe extern "C" {
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
    pub fn tex_copy(texture: TexT, type_: TexType, format: TexFormat) -> TexT;
    pub fn tex_gen_mips(texture: TexT) -> Bool32T;
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
        owned: Bool32T,
    );
    pub fn tex_get_surface(texture: TexT) -> *mut c_void;
    pub fn tex_create_from_hardware_buffer(hardware_buffer: *mut c_void, owns_buffer: Bool32T) -> TexT;
    pub fn tex_get_hardware_buffer(texture: TexT) -> *mut c_void;
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
    pub fn tex_set_colors_3d(texture: TexT, width: i32, height: i32, depth: i32, data: *mut c_void);
    pub fn tex_set_color_arr(
        texture: TexT,
        width: i32,
        height: i32,
        array_data: *mut *mut c_void,
        array_count: i32,
        multisample: i32,
        out_sh_lighting_info: *mut SphericalHarmonics,
    );
    pub fn tex_set_color_arr_mips(
        texture: TexT,
        width: i32,
        height: i32,
        array_data: *mut *mut c_void,
        array_count: i32,
        mip_count: i32,
        multisample: i32,
        out_sh_lighting_info: *mut SphericalHarmonics,
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
    pub fn tex_get_data(texture: TexT, out_data: *mut c_void, data_size: usize, mip_level: i32);
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
    pub fn tex_get_depth(texture: TexT) -> i32;
    pub fn tex_set_sample(texture: TexT, sample: TexSample);
    pub fn tex_get_sample(texture: TexT) -> TexSample;
    pub fn tex_set_sample_comp(texture: TexT, compare: TexSampleComp);
    pub fn tex_get_sample_comp(texture: TexT) -> TexSampleComp;
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

    fn as_asset(&self) -> crate::system::AssetT {
        self.0.as_ptr() as crate::system::AssetT
    }
}

impl Default for Tex {
    /// A Default texture may be asked when a Tex creation or find returned an error. [`Tex::error()`] is a good default
    /// value.
    fn default() -> Self {
        Self::error()
    }
}

impl Tex {
    /// Sets up an empty texture container! Fill it with data using SetColors next! Creates a default unique asset Id.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Tex.html>
    /// * `texture_type` - What type of texture is it? Just a 2D Image? A Cubemap? Should it have mip-maps?
    /// * `format` - What information is the texture composed of? 32 bit colors, 64 bit colors, etc.
    /// * `id` - A unique asset Id for this texture, this is used to find the texture later on, and to reference it.
    ///   if
    ///
    /// see also [`tex_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::{named_colors, Color128},
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let color_dots = [named_colors::CYAN; 128 * 128];
    /// let mut tex_left = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, Some("tex_left_ID"));
    /// tex_left.set_colors32(128, 128, &color_dots);
    ///
    /// let color_dots = [Color128::new(0.5, 0.75, 0.25, 1.0); 128 * 128];
    /// let mut tex_right = Tex::new(TexType::Image, TexFormat::Rgba128, None);
    /// tex_right.set_colors128(128, 128, &color_dots);
    ///
    /// let material_left  = Material::pbr().tex_copy(tex_left);
    /// let material_right = Material::pbr().tex_copy(tex_right);
    ///
    /// let transform_left  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0,-45.0, 90.0]);
    /// let transform_right = Matrix::t_r([ 0.5, 0.0, 0.0], [0.0, 45.0,-90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material_left,  transform_left,  None, None);
    ///     plane_mesh.draw(&material_right, transform_right, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new(texture_type: TexType, format: TexFormat, id: Option<&str>) -> Tex {
        let tex =
            Tex(NonNull::new(unsafe { tex_create(texture_type, format) }).expect("Tex::new should create texture"));
        if let Some(id) = id {
            let c_str = CString::new(id).unwrap_or_default();
            unsafe { tex_set_id(tex.0.as_ptr(), c_str.as_ptr()) };
        }
        tex
    }

    /// Loads an image file stored in memory directly into a texture! Supported formats are: jpg, png, tga, bmp, psd,
    /// gif, hdr, pic, ktx2.
    /// Asset Id will be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromMemory.html>
    /// * `data` - The binary data of an image file, this is NOT a raw RGB color array!
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner. If None will be set to 10
    ///
    /// see also [`tex_create_mem`] [`Tex::set_memory`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, tex::Tex, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let left_data  = std::include_bytes!("../assets/textures/open_gltf.jpeg");
    /// let right_data = std::include_bytes!("../assets/textures/log_viewer.jpeg");
    ///
    /// let tex_left  = Tex::from_memory(left_data, true, None)
    ///                          .expect("open_gltf.jpeg should be loaded");
    /// let tex_right = Tex::from_memory(right_data, true, None)
    ///                          .expect("open_gltf.jpeg should be loaded");
    ///
    /// let material_left  = Material::pbr().tex_copy(tex_left);
    /// let material_right = Material::pbr().tex_copy(tex_right);
    ///
    /// let transform_left  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0,-45.0, 90.0]);
    /// let transform_right = Matrix::t_r([ 0.5, 0.0, 0.0], [0.0, 45.0,-90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material_left,  transform_left,  None, None);
    ///     plane_mesh.draw(&material_right, transform_right, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
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
    /// * `file_utf8` - An absolute filename, or a filename relative to the assets folder. Supports jpg, png, tga, bmp,
    ///   psd, gif, hdr, pic, ktx2.
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner. If None will be set to 10
    ///
    /// see also [`tex_create_file`] [`Tex::get_asset_state`] [`crate::material::Material::tex_file_copy`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, system::AssetState,
    ///                      tex::Tex, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let tex_left  = Tex::from_file("textures/open_gltf.jpeg", true, Some(9999))
    ///                          .expect("tex_left should be created");
    /// let tex_right = Tex::from_file("textures/log_viewer.jpeg", true, Some(9999))
    ///                          .expect("tex_right should be created");
    /// let tex_floor = Tex::from_file("not a file so we'll have error tex", true, Some(9999))
    ///                          .expect("tex_error should be loaded");
    ///
    /// let material_left  = Material::pbr().tex_copy(&tex_left);
    /// let material_right = Material::pbr().tex_copy(&tex_right);
    /// let material_floor = Material::pbr().tex_copy(&tex_floor);
    ///
    /// let transform_left  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0,-45.0, 90.0]);
    /// let transform_right = Matrix::t_r([ 0.5, 0.0, 0.0], [0.0, 45.0,-90.0]);
    /// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
    ///
    /// filename_scr = "screenshots/tex_from_file.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///
    ///     // We ensure to have the Tex loaded for the screenshot.
    ///     if    tex_left.get_asset_state()  != AssetState::Loaded
    ///        || tex_right.get_asset_state() != AssetState::Loaded { iter -= 1; }
    ///
    ///     plane_mesh.draw(&material_left,  transform_left,  None, None);
    ///     plane_mesh.draw(&material_right, transform_right, None, None);
    ///     plane_mesh.draw(&material_floor, transform_floor, None, None);
    /// );
    /// assert_eq!(tex_left.get_asset_state(),  AssetState::Loaded);
    /// assert_eq!(tex_right.get_asset_state(), AssetState::Loaded);
    /// assert_eq!(tex_floor.get_asset_state(), AssetState::NotFound);
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex_from_file.jpeg" alt="screenshot" width="200">
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
    /// * `files_utf8` - An absolute filenames, or filenames relative to the assets folder. Supports jpg, png, tga, bmp,
    ///   psd, gif, hdr, pic, ktx2.
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner. If None will be set to 10    
    ///
    /// see also [`tex_create_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{ Matrix},
    ///                      tex::{Tex}, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let tex  = Tex::from_files(&["textures/open_gltf.jpeg",
    ///                                   "textures/log_viewer.jpeg"], true, Some(100))
    ///                    .expect("tex should be created");
    ///
    /// let material  = Material::pbr().tex_copy(tex);
    ///
    /// let transform  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, -45.0, 90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material,  transform,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
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
            let c_str = CString::new(
                path.to_str().ok_or(StereoKitError::TexCString(path_buf.to_str().unwrap_or_default().to_owned()))?,
            )?;
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
    /// * `colors` - An array of 32 bit colors, should be a length of width*height.
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `srgb_data` - s this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    ///
    /// see also [`tex_create_color32`] [`Tex::gen_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::named_colors,
    ///                      tex::Tex, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let color_dots = [named_colors::RED; 128 * 128];
    /// let tex = Tex::from_color32(&color_dots, 128, 128, true)
    ///                            .expect("Tex should be created");
    ///
    /// let material  = Material::pbr().tex_copy(tex);
    ///
    /// let transform  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, -45.0, 90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material,  transform,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_color32(
        colors: &[Color32],
        width: usize,
        height: usize,
        srgb_data: bool,
    ) -> Result<Tex, StereoKitError> {
        if width * height != { colors }.len() {
            return Err(StereoKitError::TexColor(
                format!("{}x{} differ from {}", height, width, { colors }.len()),
                "tex_create_color32 failed".to_string(),
            ));
        }
        Ok(Tex(NonNull::new(unsafe {
            tex_create_color32(colors.as_ptr() as *mut Color32, width as i32, height as i32, srgb_data as i32)
        })
        .ok_or(StereoKitError::TexColor(
            format!("{height}x{width}"),
            "tex_create_color32 failed".to_string(),
        ))?))
    }

    /// Creates a texture and sets the texture’s pixels using a color array! Color values are converted to 32 bit colors,
    /// so this means a memory allocation and conversion. Prefer the Color32 overload for performance, or create an empty
    /// Texture and use SetColors for more flexibility. This will be an image of type TexType.Image, and a format of
    /// TexFormat. Rgba32 or TexFormat.Rgba32Linear depending on the value of the sRGBData parameter.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromColors.html>
    /// * `colors` - An array of 128 bit colors, should be a length of width*height.
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `srgb_data` - s this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    ///
    /// Important: The color conversion from 128 to 32 may crash if the data do not contains color128.
    ///
    /// see also [`tex_create_color128`] [`Tex::gen_color()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::Color128,
    ///                      tex::Tex, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let color_dots = [Color128::new(0.1, 0.2, 0.5, 1.0); 128 * 128];
    /// let tex = Tex::from_color128(&color_dots, 128, 128, true)
    ///                            .expect("Tex should be created");
    ///
    /// let material  = Material::pbr().tex_copy(tex);
    ///
    /// let transform  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, -45.0, 90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material, transform,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_color128(
        colors: &[Color128],
        width: usize,
        height: usize,
        srgb_data: bool,
    ) -> Result<Tex, StereoKitError> {
        if width * height != { colors }.len() {
            return Err(StereoKitError::TexColor(
                format!("{}x{} differ from {}", height, width, { colors }.len()),
                "tex_create_color128 failed".to_string(),
            ));
        }
        Ok(Tex(NonNull::new(unsafe {
            tex_create_color128(colors.as_ptr() as *mut Color128, width as i32, height as i32, srgb_data as i32)
        })
        .ok_or(StereoKitError::TexColor(
            format!("{height}x{width}"),
            "tex_create_color128 failed".to_string(),
        ))?))
    }

    /// This will assemble a texture ready for rendering to! It creates a render target texture with no mip maps and a
    /// depth buffer attached.
    /// <https://stereokit.net/Pages/StereoKit/Tex/RenderTarget.html>
    /// * `width` - in pixels
    /// * `height` - in pixels
    /// * `multisample` - Multisample level, or MSAA. This should be 1, 2, 4, 8, or 16. The results will have moother
    ///   edges with higher values, but will cost more RAM and time to render. Note that GL platforms cannot trivially
    ///   draw a multisample > 1 texture in a shader. If this is None, the default is 1.
    /// * `color_format` - The format of the color surface. If this is None, the default is RGBA32.
    /// * `depth_format` - The format of the depth buffer. If this is TexFormat::None, no depth buffer will be attached
    ///   to this. If this is None, the default is Depth16.
    ///   rendertarget.
    ///
    /// see also [`tex_create_rendertarget`]
    ///
    /// see also [`tex_get_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, tex::{Tex, TexFormat}, material::Material};
    ///
    /// let tex = Tex::render_target(128, 128, Some(2), Some(TexFormat::Rgba32Srgb), None)
    ///                            .expect("Tex should be created");
    ///
    /// let material  = Material::from_file("shaders/brick_pbr.hlsl.sks", None)
    ///                     .expect("Material should be created");
    ///
    /// Renderer::blit(&tex, &material);
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex_render_target.jpeg" alt="screenshot" width="200">
    pub fn render_target(
        width: usize,
        height: usize,
        multisample: Option<i32>,
        color_format: Option<TexFormat>,
        depth_format: Option<TexFormat>,
    ) -> Result<Tex, StereoKitError> {
        let multisample = multisample.unwrap_or(1);
        let color_format = color_format.unwrap_or(TexFormat::Rgba32Srgb);
        let depth_format = depth_format.unwrap_or(TexFormat::Depth16);
        Ok(Tex(NonNull::new(unsafe {
            tex_create_rendertarget(width as i32, height as i32, multisample, color_format, depth_format)
        })
        .ok_or(StereoKitError::TexRenderTarget(
            format!("{height}x{width}"),
            "tex_create_rendertarget failed".to_string(),
        ))?))
    }

    /// This generates a solid color texture of the given dimensions. Can be quite nice for creating placeholder textures!
    /// Make sure to match linear/gamma colors with the correct format.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenColor.html>
    /// * `color` - The color to use for the texture. This is interpreted slightly differently based on what TexFormat
    ///   gets used.
    /// * `width` - Width of the final texture, in pixels.
    /// * `height` - Height of the final texture, in pixels.
    /// * `tex_type` - Not all types here are applicable, but TexType.Image or TexType::ImageNomips are good options here.
    /// * `format` - Not all formats are supported, but this does support a decent range. The provided color is
    ///   interpreted slightly different depending on this format.
    ///
    /// see also [`tex_gen_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::{named_colors, Color128},
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    ///
    /// let tex_err = Tex::gen_color(named_colors::RED, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// Tex::set_error_fallback(&tex_err);
    ///
    /// let tex =  Tex::gen_color(Color128::new(0.1, 0.2, 0.5, 1.0), 128, 128, TexType::Image, TexFormat::Rgba128);
    ///
    /// let material  = Material::pbr().tex_copy(tex);
    ///
    /// let transform  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, -45.0, 90.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material, transform,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn gen_color(color: impl Into<Color128>, width: i32, height: i32, tex_type: TexType, format: TexFormat) -> Tex {
        let raw = unsafe { tex_gen_color(color.into(), width, height, tex_type, format) };
        match NonNull::new(raw) {
            Some(nn) => Tex(nn),
            None => {
                Log::err(format!(
                    "tex_gen_color failed for {width}x{height} {tex_type:?} {format:?}. Returning error fallback texture."
                ));
                Tex::error()
            }
        }
    }

    /// Generates a ‘radial’ gradient that works well for particles, blob shadows, glows, or various other things.
    /// The roundness can be used to change the shape from round, ‘1’, to star-like, ‘0’. Default color is transparent white to opaque white,
    /// but this can be configured by providing a Gradient of your own.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenParticle.html>
    /// * `width` - Width of the final texture, in pixels.
    /// * `height` - Height of the final texture, in pixels.
    /// * `gradient_linear` : A color gradient that starts with the background/outside at 0, and progresses to the center
    ///   at 1. If None, will use a white gradient.
    ///
    /// see also [`tex_gen_particle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, tex::Tex, mesh::Mesh, material::Material,
    ///                      util::{named_colors, Gradient, GradientKey, Color128}};
    ///
    /// let keys = [
    ///     GradientKey::new(Color128::BLACK_TRANSPARENT, 0.0),
    ///     GradientKey::new(named_colors::RED, 0.1),
    ///     GradientKey::new(named_colors::CYAN, 0.4),
    ///     GradientKey::new(named_colors::YELLOW, 0.5),
    ///     GradientKey::new(Color128::BLACK, 0.7)];
    ///
    /// let tex_back  = Tex::gen_particle(128, 128, 0.15, Some(Gradient::new(Some(&keys))));
    /// let tex_floor = Tex::gen_particle(128, 128, 0.3, Some(Gradient::new(Some(&keys))));
    /// let tex_right = Tex::gen_particle(128, 128, 0.6, Some(Gradient::new(Some(&keys))));
    /// let tex_left  = Tex::gen_particle(128, 128, 0.9, Some(Gradient::new(Some(&keys))));
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let material_left  = Material::unlit_clip().tex_copy(tex_left);
    /// let material_right = Material::unlit_clip().tex_copy(tex_right);
    /// let material_back  = Material::unlit_clip().tex_copy(tex_back);
    /// let material_floor = Material::unlit_clip().tex_copy(tex_floor);
    ///
    /// let transform_left  = Matrix::t_r([-0.5, 0.0, 0.0], [0.0, 0.0, 90.0]);
    /// let transform_right = Matrix::t_r([ 0.5, 0.0, 0.0], [0.0, 0.0, -90.0]);
    /// let transform_back  = Matrix::t_r([ 0.0, 0.0,-0.5], [90.0, 0.0, 0.0]);
    /// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
    ///
    /// filename_scr = "screenshots/tex_gen_particle.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material_left,  transform_left,  None, None);
    ///     plane_mesh.draw(&material_right, transform_right, None, None);
    ///     plane_mesh.draw(&material_back,  transform_back,  None, None);
    ///     plane_mesh.draw(&material_floor, transform_floor, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/tex_gen_particle.jpeg" alt="screenshot" width="200">
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
        Tex(NonNull::new(unsafe { tex_gen_particle(width, height, roundness, gradient_linear.0.as_ptr()) })
            .expect("Tex::gen_particle should create texture"))
    }

    /// This is the texture that all Tex objects will fall back to by default if they are still loading. Assigning a
    /// texture here that isn’t fully loaded will cause the app to block until it is loaded.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetLoadingFallback.html>
    ///
    /// see also [`tex_set_loading_fallback`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let tex_loading = Tex::gen_color(named_colors::GREEN, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// Tex::set_loading_fallback(&tex_loading);
    ///
    /// let tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// let material  = Material::pbr().tex_copy(tex);
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material,  transform_floor,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_loading_fallback<T: AsRef<Tex>>(fallback: T) {
        unsafe { tex_set_loading_fallback(fallback.as_ref().0.as_ptr()) };
    }

    /// This is the texture that all Tex objects with errors will fall back to. Assigning a texture here that isn’t
    /// fully loaded will cause the app to block until it is loaded.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetErrorFallback.html>
    ///
    /// see also [`tex_set_error_fallback`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let tex_err = Tex::gen_color(named_colors::RED, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// Tex::set_error_fallback(&tex_err);
    ///
    /// let tex = Tex::from_file("file that doesn't exist", true, None)
    ///                    .expect("tex should be created");
    /// let material  = Material::pbr().tex_copy(tex);
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material,  transform_floor,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_error_fallback<T: AsRef<Tex>>(fallback: T) {
        unsafe { tex_set_error_fallback(fallback.as_ref().0.as_ptr()) };
    }

    /// Looks for a Material asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Tex/Find.html>
    /// * `id` - The id of the texture to find.
    ///
    /// see also [`tex_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, tex::{Tex, TexFormat, TexType}};
    ///
    /// let mut tex_blue = Tex::gen_color(named_colors::BLUE, 1, 1, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert!(tex_blue.get_id().starts_with("auto/tex_"));
    /// tex_blue.id("my_tex_blue");
    /// let same_tex_blue = Tex::find("my_tex_blue").expect("my_tex_blue should be found");
    /// assert_eq!(tex_blue, same_tex_blue);
    ///
    /// let tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                    .expect("tex should be created");
    /// assert_eq!(tex.get_id(), "textures/open_gltf.jpeg");
    /// let same_tex = Tex::find("textures/open_gltf.jpeg")
    ///                    .expect("same_tex should be found");
    /// assert_eq!(tex, same_tex);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Tex, StereoKitError> {
        let c_str = CString::new(id.as_ref()).map_err(|_| StereoKitError::TexCString(id.as_ref().into()))?;
        Ok(Tex(
            NonNull::new(unsafe { tex_find(c_str.as_ptr()) }).ok_or(StereoKitError::TexFind(id.as_ref().into()))?
        ))
    }

    /// Copy the current texture into a new texture, with the option to convert it to a different format or type! This
    /// is a GPU blit operation, so the source texture does not need to be readable from the CPU. If the source texture
    /// doesn't have mip-maps but the destination type does, they'll be generated for you!
    /// <https://stereokit.net/Pages/StereoKit/Tex.html>
    /// * `tex_type` - What type of texture should the new texture be? Image types with mip-maps will have mips
    ///   generated for them if the source doesn't have them. If None has default value of TexType::Image.
    /// * `tex_format` - What format should the new texture be in? If None is specified, the new texture will use the
    ///   same format as the source (TexFormat::None).
    ///
    /// see also [`tex_copy`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Color32, Color128}, tex::{Tex, TexFormat, TexType}};
    ///
    ///
    /// let tex_blue = Tex::gen_color(Color32::new(64, 32, 255, 255), 5, 5,
    ///                               TexType::Image, TexFormat::Rgba32Linear);
    ///
    /// let tex_copy = tex_blue.copy(None, Some(TexFormat::Rgba32Srgb))
    ///                             .expect("copy should be done");
    /// let mut color_data = [Color32::WHITE; 25];
    /// assert!(tex_copy.get_color_data::<Color32>(&mut color_data, 0));
    /// assert_eq!(color_data[0], Color32 { r: 64, g: 32, b: 255, a: 255 });
    ///
    /// let tex_copy = tex_blue.copy(Some(TexType::Image), Some(TexFormat::Rgba128))
    ///                             .expect("copy should be done");
    /// let mut color_data = [Color128::WHITE; 25];
    ///
    /// assert!(tex_copy.get_color_data::<Color128>(&mut color_data, 0));
    /// assert_eq!(color_data[24], Color128 { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
    /// # sk::Sk::shutdown();
    /// ```
    pub fn copy(&self, tex_type: Option<TexType>, tex_format: Option<TexFormat>) -> Result<Tex, StereoKitError> {
        let type_ = tex_type.unwrap_or(TexType::Image);
        let format = tex_format.unwrap_or(TexFormat::None);
        Ok(Tex(NonNull::new(unsafe { tex_copy(self.0.as_ptr(), type_, format) })
            .ok_or(StereoKitError::TexCopy(self.get_id().into()))?))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Find.html>
    ///
    /// see also [`tex_find()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex_blue = Tex::gen_color(named_colors::BLUE, 1, 1, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert!(tex_blue.get_id().starts_with("auto/tex_"));
    /// let same_tex_blue = tex_blue.clone_ref();
    /// assert_eq!(tex_blue, same_tex_blue);
    ///
    /// let tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                    .expect("tex should be created");
    /// assert_eq!(tex.get_id(), "textures/open_gltf.jpeg");
    /// let same_tex = tex.clone_ref();
    /// assert_eq!(tex, same_tex);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> Tex {
        Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Set a new id to the texture.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Id.html>
    ///
    /// see also [`tex_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,tex::{Tex, TexFormat, TexType}};
    ///
    /// let mut tex_blue = Tex::gen_color(named_colors::BLUE, 1, 1, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert!(tex_blue.get_id().starts_with("auto/tex_"));
    /// tex_blue.id("my_tex_blue");
    /// assert_eq!(tex_blue.get_id(), "my_tex_blue");
    ///
    /// let tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                        .expect("tex should be created");
    /// assert_eq!(tex.get_id(), "textures/open_gltf.jpeg");
    /// tex_blue.id("my_tex_image");
    /// assert_eq!(tex_blue.get_id(), "my_tex_image");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { tex_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Only applicable if this texture is a rendertarget! This creates and attaches a zbuffer surface to the texture
    /// for use when rendering to it.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddZBuffer.html>
    /// * `depth_format` - The format of the depth texture, must be a depth format type!
    ///
    /// see also [`tex_add_zbuffer`] [`Tex::set_zbuffer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{render::Renderer, tex::{Tex, TexFormat}, material::Material};
    ///
    ///
    /// let mut tex = Tex::render_target(128, 128, Some(2), Some(TexFormat::Rgba32Srgb),
    ///                                  Some(TexFormat::None))
    ///                            .expect("Tex should be created");
    /// assert_eq!(tex.get_zbuffer(), None);
    ///
    /// tex.add_zbuffer(TexFormat::Depth16);
    /// assert_ne!(tex.get_zbuffer(), None);
    ///
    /// let material  = Material::pbr().tex_copy(&tex);
    ///
    /// Renderer::blit(&tex, &material);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn add_zbuffer(&mut self, depth_format: TexFormat) -> &mut Self {
        unsafe { tex_add_zbuffer(self.0.as_ptr(), depth_format) };
        self
    }

    /// Loads an image file stored in memory directly into the created texture! Supported formats are: jpg, png, tga,
    /// bmp, psd, gif, hdr, pic, ktx2. This method introduces a blocking boolean parameter, which allows you to specify
    /// whether this method blocks until the image fully loads! The default case is to have it as part of the
    /// asynchronous asset pipeline, in which the Asset Id will
    /// be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetMemory.html>
    /// * `data` - The binary data of an image file, this is NOT a raw RGB color array!
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that’s not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * `blocking` - Will this method wait for the image to load. By default, we try to load it asynchronously.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner. If None will be set to 10
    ///
    /// see also [`tex_set_mem`] [`Tex::from_memory`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let image_data = std::include_bytes!("../assets/textures/open_gltf.jpeg");
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    ///
    /// tex.set_memory(image_data, true, false, Some(0));
    ///
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let material  = Material::pbr().tex_copy(tex);
    /// let transform_floor = Matrix::t([0.0, -0.5, 0.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material, transform_floor, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_memory(&mut self, data: &[u8], srgb_data: bool, blocking: bool, priority: Option<i32>) -> &mut Self {
        let priority = priority.unwrap_or(10);
        unsafe {
            tex_set_mem(
                self.0.as_ptr(),
                data.as_ptr() as *mut c_void,
                data.len(),
                srgb_data as Bool32T,
                blocking as Bool32T,
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
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - A pointer to a chunk of memory containing color data! Should be widthheightsize_of_texture_format
    ///   bytes large. Color data should definitely match the format provided when constructing the texture!
    ///
    /// # Safety
    /// The data pointer must be a valid array for the size of the texture.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, tex::{Tex, TexFormat, TexType}};
    ///
    /// let mut color_dots = [named_colors::CYAN; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    ///
    /// unsafe { tex.set_colors(16, 16, color_dots.as_mut_ptr() as *mut std::os::raw::c_void); }
    ///
    /// let check_dots = [Color32::WHITE; 16 * 16];
    /// assert!(tex.get_color_data::<Color32>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
    pub unsafe fn set_colors(&mut self, width: usize, height: usize, data: *mut std::os::raw::c_void) -> &mut Self {
        unsafe { tex_set_colors(self.0.as_ptr(), width as i32, height as i32, data) };
        self
    }

    /// Set the texture’s pixels using a color array! This function should only be called on textures with a format of
    /// Rgba32 or Rgba32Linear. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 32 bit colors, should be a length of width*height.
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, tex::{Tex, TexFormat, TexType}};
    ///
    /// let color_dots = [named_colors::CYAN; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    ///
    /// tex.set_colors32(16, 16, &color_dots);
    ///
    /// let check_dots = [Color32::WHITE; 16 * 16];
    /// assert!(tex.get_color_data::<Color32>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors32(&mut self, width: usize, height: usize, data: &[Color32]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::Rgba32Srgb) => (),
            Some(TexFormat::Rgba32Linear) => (),
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
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 128 bit colors, should be a length of width*height.
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Color128, tex::{Tex, TexFormat, TexType}};
    ///
    /// let color_dots = [Color128{r: 0.25, g: 0.125, b: 1.0, a: 1.0}; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba128, None);
    ///
    /// tex.set_colors128(16, 16, &color_dots);
    ///
    /// let check_dots = [Color128::BLACK; 16 * 16];
    /// assert!(tex.get_color_data::<Color128>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors128(&mut self, width: usize, height: usize, data: &[Color128]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::Rgba128) => (),
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
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 8 bit values, should be a length of width*height.
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::{Tex, TexFormat, TexType};
    ///
    /// let color_dots = [125u8; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::R8, None);
    ///
    /// tex.set_colors_r8(16, 16, &color_dots);
    ///
    /// let check_dots = [0u8; 16 * 16];
    /// assert!(tex.get_color_data::<u8>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
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
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 8 bit values, should be a length of width*height.
    /// * `color_size` - number of byte for a pixel used by the format of this texture
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Color32, tex::{Tex, TexFormat, TexType}};
    ///
    /// let color_dots = [127u8; 16 * 16 * 4];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    ///
    /// tex.set_colors_u8(16, 16, &color_dots, TexFormat::Rgba32Srgb.bytes_per_pixel());
    ///
    /// let check_dots = [Color32::BLACK; 16 * 16];
    /// assert!(tex.get_color_data::<Color32>(&check_dots, 0));
    /// assert_eq!(check_dots[0],Color32{r:127,g:127,b:127,a:127});
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_u8(&mut self, width: usize, height: usize, data: &[u8], color_size: usize) -> &mut Self {
        if width * height * color_size != data.len() {
            Log::err(format!(
                "{}x{}x{} differ from {} for Tex::set_colors_u8 for texture {}",
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
    /// with a format of R16un. You can call this as many times as you’d like, even with different widths and heights.
    /// Calling this multiple times will mark it as dynamic on the graphics card. Calling this function can also result
    /// in building mip-maps, which has a non-zero cost: use TexType.ImageNomips when creating the Tex to avoid this.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 16 bit values, should be a length of width*height.
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::{Tex, TexFormat, TexType};
    ///
    /// let color_dots = [256u16; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::R16un, None);
    ///
    /// tex.set_colors_r16(16, 16, &color_dots);
    ///
    /// let check_dots = [0u16; 16 * 16];
    /// assert!(tex.get_color_data::<u16>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_r16(&mut self, width: usize, height: usize, data: &[u16]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::R16un) => (),
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
    /// * `width` - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `data` - An array of 32 bit values, should be a length of width*height.
    ///
    /// Warning, instead of [`Tex::set_colors`], this call may not be done if the asset is not loaded
    /// (see [`Tex::get_asset_state`]) or the size is inconsistent or the format is incompatible.
    ///
    /// see also [`tex_set_colors`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::{Tex, TexFormat, TexType};
    ///
    /// let color_dots = [0.13f32; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::R32f, None);
    ///
    /// tex.set_colors_r32(16, 16, &color_dots);
    ///
    /// let check_dots = [0.0f32; 16 * 16];
    /// assert!(tex.get_color_data::<f32>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_r32(&mut self, width: usize, height: usize, data: &[f32]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::R32f) => (),
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

    /// Set the contents of a 3D (volume) texture from a contiguous block of memory. The texture must be created
    /// with [`TexType::Volume`]. Pass `std::ptr::null_mut()` to allocate an empty volume (e.g. for use as a compute
    /// UAV). Slice-major layout: all of slice 0, then slice 1, etc., each slice being `width * height` pixels of
    /// the texture's format.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels.
    /// * `height` - Height in pixels.
    /// * `depth` - Depth in pixels (number of slices).
    /// * `data` - A pointer to `width * height * depth` pixels of the texture's format, or null to allocate an
    ///   empty volume.
    ///
    /// # Safety
    /// The data pointer must be valid for `width * height * depth * format_size` bytes, or null.
    ///
    /// see also [`tex_set_colors_3d`] [`Tex::set_colors_3d`] [`Tex::get_depth`] [`Tex::get_color_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Color32, named_colors}, tex::{Tex, TexFormat, TexType}};
    ///
    /// let mut volume_data = [named_colors::MAGENTA; 4 * 4 * 4];
    /// let mut tex = Tex::new(TexType::Volume, TexFormat::Rgba32Srgb, None);
    ///
    /// unsafe { tex.set_colors_3d_ptr(4, 4, 4, volume_data.as_mut_ptr() as *mut std::os::raw::c_void); }
    ///
    /// let check_data = [Color32::WHITE; 4 * 4];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data, 0), true);
    ///     assert_eq!(check_data, [named_colors::MAGENTA; 4 * 4]);
    ///     assert_eq!(tex.get_depth(), Some(4));
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub unsafe fn set_colors_3d_ptr(
        &mut self,
        width: usize,
        height: usize,
        depth: usize,
        data: *mut std::os::raw::c_void,
    ) -> &mut Self {
        unsafe { tex_set_colors_3d(self.0.as_ptr(), width as i32, height as i32, depth as i32, data) };
        self
    }

    /// Set the contents of a 3D (volume) texture from a byte array. The texture must be created with
    /// [`TexType::Volume`] and a single-channel format such as [`TexFormat::R8`]. Slice-major layout: all of
    /// slice 0, then slice 1, etc., each slice being `width * height` bytes.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels.
    /// * `height` - Height in pixels.
    /// * `depth` - Depth in pixels (number of slices).
    /// * `data` - An array of `width * height * depth` bytes.
    ///
    /// see also [`tex_set_colors_3d`] [`Tex::get_depth`] [`Tex::get_color_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::{Tex, TexFormat, TexType};
    ///
    /// let volume_data = [127u8; 4 * 4 * 4];
    /// let mut tex = Tex::new(TexType::Volume, TexFormat::R8, None);
    ///
    /// tex.set_colors_3d(4, 4, 4, &volume_data);
    ///
    /// let check_data = [0u8; 4 * 4];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<u8>(&check_data, 0), true);
    ///     assert_eq!(check_data, [127u8; 4 * 4]);
    ///     assert_eq!(tex.get_depth(), Some(4));
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_3d(&mut self, width: usize, height: usize, depth: usize, data: &[u8]) -> &mut Self {
        if width * height * depth != data.len() {
            Log::err(format!(
                "{}x{}x{} differ from {} for Tex::set_colors_3d_r8 for texture {}",
                width,
                height,
                depth,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors_3d(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                depth as i32,
                data.as_ptr() as *mut std::os::raw::c_void,
            )
        };
        self
    }

    /// Set the contents of a 3D (volume) texture from a [`Color32`] array. The texture must be created with
    /// [`TexType::Volume`] and a format of [`TexFormat::Rgba32Srgb`] or [`TexFormat::Rgba32Linear`].
    /// Slice-major layout: all of slice 0, then slice 1, etc., each slice being `width * height` pixels.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels.
    /// * `height` - Height in pixels.
    /// * `depth` - Depth in pixels (number of slices).
    /// * `data` - An array of `width * height * depth` [`Color32`] values.
    ///
    /// see also [`tex_set_colors_3d`] [`Tex::set_colors_3d_ptr`] [`Tex::get_depth`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Color32, named_colors}, tex::{Tex, TexFormat, TexType}};
    ///
    /// let volume_data = [named_colors::MAGENTA; 4 * 4 * 4];
    /// let mut tex = Tex::new(TexType::Volume, TexFormat::Rgba32Srgb, None);
    ///
    /// tex.set_colors_3d_32(4, 4, 4, &volume_data);
    ///
    /// let check_data = [Color32::WHITE; 4 * 4];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data, 0), true);
    ///     assert_eq!(check_data, [named_colors::MAGENTA; 4 * 4]);
    ///     assert_eq!(tex.get_depth(), Some(4));
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_3d_32(&mut self, width: usize, height: usize, depth: usize, data: &[Color32]) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::Rgba32Srgb) | Some(TexFormat::Rgba32Linear) => (),
            Some(_) => {
                Log::err(format!(
                    "The format of the texture {} is not compatible with Tex::set_colors_3d_32",
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors_3d_32", self.get_id()));
                return self;
            }
        }
        if width * height * depth != data.len() {
            Log::err(format!(
                "{}x{}x{} differ from {} for Tex::set_colors_3d_32 for texture {}",
                width,
                height,
                depth,
                data.len(),
                self.get_id()
            ));
            return self;
        }
        unsafe {
            tex_set_colors_3d(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                depth as i32,
                data.as_ptr() as *mut std::os::raw::c_void,
            )
        };
        self
    }

    /// This allows you to attach a z/depth buffer from a rendertarget texture. This texture _must_ be a
    /// rendertarget to set this, and the zbuffer texture _must_ be a depth format (or null). For no-rendertarget
    /// textures, this will always be None.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetZBuffer.html>
    /// * `tex` - TODO: None may crash the program
    ///
    /// see also [`tex_set_zbuffer`] [`Tex::add_zbuffer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::{Tex, TexFormat};
    ///
    ///
    /// let tex = Tex::render_target(128, 128, Some(2), Some(TexFormat::Rgba32Srgb),
    ///                                  Some(TexFormat::Depth16))
    ///                            .expect("Tex should be created");
    ///
    /// let zbuffer = tex.get_zbuffer().expect("Tex should have a zbuffer");
    ///
    /// let mut tex2 = Tex::render_target(128, 128, Some(2), Some(TexFormat::Rgba32Srgb),
    ///                                  Some(TexFormat::None))
    ///                            .expect("Tex2 should be created");
    /// tex2.set_zbuffer(Some(zbuffer));
    /// assert_ne!(tex2.get_zbuffer(), None);
    ///
    /// //tex2.set_zbuffer(None);
    /// //assert_eq!(tex2.get_zbuffer(), None);
    /// # sk::Sk::shutdown();
    /// ```
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
    /// # Safety
    /// native_surface must be a valid pointer to a texture resource for the current graphics backend.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetNativeSurface.html>
    /// * `native_surface` - For Vulkan, this should be a VkImage handle, coerced into the IntPtr.
    /// * `tex_type` - The image flags that tell SK how to treat the texture, this should match up with the settings the
    ///   texture was originally created with. If SK can figure the appropriate settings, it may override the value
    ///   provided here.
    /// * `native_fmt` - The texture’s format using the graphics backend’s value, not SK’s. This should match up with
    ///   the settings the texture was originally created with. If SK can figure the appropriate settings, it may
    ///   override the value provided here. 0 is a valide default value.
    /// * `width` - Width of the texture. This should match up with the settings the texture was originally created
    ///   with. If SK can figure the appropriate settings, it may override the value provided here. 0 is a valide default
    ///   value.
    /// * `height` - Height of the texture. This should match up with the settings the texture was originally created
    ///   with. If SK can figure the appropriate settings, it may override the value provided here. 0 is a valide default
    ///   value.
    /// * `surface_count` - Texture array surface count. This should match up with the settings the texture was
    ///   originally created with. If SK can figure the appropriate settings, it may override the value provided here.
    ///   1 is a valide default value.
    /// * `owned` - Should ownership of this texture resource be passed on to StereoKit? If so, StereoKit may delete
    ///   it when it’s finished with it. True is a valide default value, if this is not desired, pass in false.
    ///
    /// see also [`tex_set_surface`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// # use stereokit_rust::{tex::{Tex, TexFormat, TexType}};
    ///
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// let native_surface = tex.get_native_surface();
    /// unsafe { tex.set_native_surface(native_surface, TexType::Image, 0, 1, 1, 1, false); }
    /// # sk::Sk::shutdown();
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn set_native_surface(
        &mut self,
        native_surface: *mut std::os::raw::c_void,
        tex_type: TexType,
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
                tex_type,
                native_fmt,
                width,
                height,
                surface_count,
                1,
                owned as Bool32T,
            )
        };
        self
    }

    /// Imports an Android AHardwareBuffer as an external texture, YCbCr camera or decoder buffers come in via sampler
    /// conversion. This allows zero-copy access to hardware decoder or camera output. This is only functional on
    /// Android, and requires device support for hardware buffer import.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromHardwareBuffer.html>
    /// * `hardware_buffer` - An `AHardwareBuffer*` coerced into a `*mut c_void`.
    /// * `owns_buffer` - Should ownership of the hardware buffer be passed on to StereoKit? If so, StereoKit will release
    ///   it when the texture is destroyed.
    ///
    /// Returns a `Tex` asset wrapping the hardware buffer, or `None` if the buffer is null, the device doesn't support
    /// importing hardware buffers, or the import failed.
    ///
    /// # Safety
    /// `hardware_buffer` must be a valid `AHardwareBuffer*` pointer, or null. Passing an invalid pointer is undefined
    /// behavior.
    ///
    /// see also [`tex_create_from_hardware_buffer`] [`Tex::get_hardware_buffer`]
    pub unsafe fn from_hardware_buffer(hardware_buffer: *mut c_void, owns_buffer: bool) -> Option<Tex> {
        let inst = unsafe { tex_create_from_hardware_buffer(hardware_buffer, owns_buffer as Bool32T) };
        let nn = NonNull::new(inst)?;
        // Match the C# behavior: if the asset state indicates a failed/error texture, release the ref and return None.
        let state = unsafe { tex_asset_state(inst) };
        if (state as i32) < (AssetState::None as i32) {
            unsafe { crate::system::assets_releaseref_threadsafe(inst as *mut c_void) };
            return None;
        }
        Some(Tex(nn))
    }

    /// Set the texture’s size without providing any color data. In most cases, you should probably just call SetColors
    /// instead, but this can be useful if you’re adding color data some other way, such as when blitting or rendering
    /// to it.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetSize.html>
    /// * `width`  - Width in pixels of the texture. Powers of two are generally best!
    /// * `height` - Height in pixels of the texture. Powers of two are generally best!
    /// * `array_count` - How many surfaces (array layers) are in this texture? A normal texture only has 1, but
    ///   additional layers can be useful for certain rendering techniques or effects.
    /// * `msaa` - Multisample anti-aliasing level, only important for render target type textures. This is the number
    ///   of fragments drawn per pixel to reduce aliasing artifacts. Typical values: 1,2,4,8. None is 1.
    ///
    /// Internally this invokes the native `tex_set_color_arr` with a null data pointer, establishing only the
    /// dimensions/array layout.
    ///
    /// see also [`tex_set_color_arr`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::tex::{Tex, TexFormat, TexType};
    /// let mut tex = Tex::new(TexType::Rendertarget, TexFormat::Rgba32Srgb, None);
    /// // Use defaults (array_count=1, msaa=1)
    /// tex.set_size(64, 64, None, None);
    /// assert_eq!(tex.get_width(),  Some(64));
    /// assert_eq!(tex.get_height(), Some(64));
    ///
    /// // Explicit MSAA configuration
    /// tex.set_size(128, 64, None, Some(4)); // 1-layer array, 4x MSAA
    /// assert_eq!(tex.get_width(),  Some(128));
    /// assert_eq!(tex.get_height(), Some(64));
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_size(
        &mut self,
        width: usize,
        height: usize,
        array_count: Option<usize>,
        msaa: Option<i32>,
    ) -> &mut Self {
        let array_count = array_count.unwrap_or(1);
        let msaa = msaa.unwrap_or(1);
        unsafe {
            let data_ptr: *mut *mut std::os::raw::c_void = null_mut();
            tex_set_color_arr(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                data_ptr, // array_data = None
                array_count as i32,
                msaa,
                null_mut(), // out_sh_lighting_info = None
            )
        };
        self
    }

    /// Set the texture's pixels for a multi-layer and/or mip-mapped texture, using an array of raw pointers.
    /// Each pointer in `array_data` represents one layer (face for cubemaps, slice for array textures), and points
    /// to a tightly packed block containing all mip levels for that layer in the order `[mip0][mip1][mip2]...`.
    /// The memory layout per mip should match the texture's format. This is the raw pointer variant for advanced use
    /// cases like uploading pre-decoded image data from native code.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels of mip 0. Powers of two are generally best!
    /// * `height` - Height in pixels of mip 0. Powers of two are generally best!
    /// * `array_data` - An array of raw pointers, one per layer. Each layer points to packed mip data
    ///   `[mip0][mip1][mip2]...`.
    /// * `mip_count` - The number of mip levels packed into each layer's data. Use 1 if no mip data is provided
    ///   beyond the base.
    /// * `multisample` - Multisample count, only relevant for rendertarget textures. If None, defaults to 1.
    ///
    /// # Safety
    /// Each pointer in `array_data` must be valid for the corresponding layer's packed mip data.
    ///
    /// see also [`tex_set_color_arr_mips`] [`Tex::set_colors_arr_mips`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, tex::{Tex, TexFormat, TexType}};
    /// let layer0_mip0 = [named_colors::RED; 16 * 16];
    /// let layer0_mip1 = [named_colors::GREEN; 8 * 8];
    /// let layer1_mip0 = [named_colors::BLUE; 64 * 64];
    /// let layer1_mip1 = [named_colors::YELLOW; 32 * 32];
    /// let mut array_data = [
    ///     layer0_mip0.as_ptr() as *mut std::os::raw::c_void,
    ///     layer0_mip1.as_ptr() as *mut std::os::raw::c_void,
    ///     layer1_mip0.as_ptr() as *mut std::os::raw::c_void,
    ///     layer1_mip1.as_ptr() as *mut std::os::raw::c_void,
    /// ];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// unsafe {
    ///     tex.set_colors_arr_mips_ptr(16, 16, &mut array_data, 2, None);
    /// }
    /// let check_data256 = [named_colors::BLACK; 16 * 16];
    /// let check_data64 = [named_colors::BLACK; 8 * 8];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data256, 0), true);
    ///     assert_eq!(check_data256, [named_colors::RED; 16 * 16]);
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data64, 1), true);
    ///     assert_eq!(check_data64, [named_colors::GREEN; 8 * 8]);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub unsafe fn set_colors_arr_mips_ptr(
        &mut self,
        width: usize,
        height: usize,
        array_data: &mut [*mut c_void],
        mip_count: i32,
        multisample: Option<i32>,
    ) -> &mut Self {
        let multisample = multisample.unwrap_or(1);
        unsafe {
            tex_set_color_arr_mips(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                array_data.as_mut_ptr(),
                array_data.len() as i32,
                mip_count,
                multisample,
                null_mut(),
            )
        };
        self
    }

    /// Set the texture's pixels for a multi-layer and/or mip-mapped texture using a jagged [`Color32`] array.
    /// Each entry in `array_data` is one layer (face for cubemaps, slice for array textures), packed as
    /// `[mip0][mip1][mip2]...`. This function should only be called on textures with a format of
    /// [`TexFormat::Rgba32Srgb`] or [`TexFormat::Rgba32Linear`].
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels of mip 0. Powers of two are generally best!
    /// * `height` - Height in pixels of mip 0. Powers of two are generally best!
    /// * `array_data` - A slice of slices, where each inner slice contains all mip levels for that layer packed as
    ///   `[mip0][mip1][mip2]...` with mip 0 sized `width * height`, mip 1 sized `(width/2) * (height/2)`, etc.
    /// * `mip_count` - The number of mip levels packed into each layer's data. Use 1 if no mip data is provided
    ///   beyond the base.
    /// * `multisample` - Multisample count, only relevant for rendertarget textures. If None, defaults to 1.
    ///
    /// see also [`tex_set_color_arr_mips`] [`Tex::set_colors_arr_mips`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, tex::{Tex, TexFormat, TexType}};
    /// let layer0_mip0 = [named_colors::RED; 16 * 16];
    /// let layer0_mip1 = [named_colors::GREEN; 8 * 8];
    /// let layer1_mip0 = [named_colors::BLUE; 64 * 64];
    /// let layer1_mip1 = [named_colors::YELLOW; 32 * 32];
    /// let array_data = [
    ///     &layer0_mip0[..],
    ///     &layer0_mip1[..],
    ///     &layer1_mip0[..],
    ///     &layer1_mip1[..],
    /// ];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// tex.set_colors_arr_mips32(16, 16, &array_data, 2, None);
    /// let check_data256 = [named_colors::BLACK; 16 * 16];
    /// let check_data64 = [named_colors::BLACK; 8 * 8];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data256, 0), true);
    ///     assert_eq!(check_data256, [named_colors::RED; 16 * 16]);
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data64, 1), true);
    ///     assert_eq!(check_data64, [named_colors::GREEN; 8 * 8]);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_arr_mips32(
        &mut self,
        width: usize,
        height: usize,
        array_data: &[&[Color32]],
        mip_count: i32,
        multisample: Option<i32>,
    ) -> &mut Self {
        match self.get_format() {
            Some(TexFormat::Rgba32Srgb) => (),
            Some(TexFormat::Rgba32Linear) => (),
            Some(fmt) => {
                Log::err(format!(
                    "Can't set a {:?} format texture from Color32 data in Tex::set_colors_arr_mips32 for texture {}!",
                    fmt,
                    self.get_id()
                ));
                return self;
            }
            None => {
                Log::err(format!("The texture {} is not loaded during Tex::set_colors_arr_mips32", self.get_id()));
                return self;
            }
        }
        let multisample = multisample.unwrap_or(1);
        let mut ptrs: Vec<*mut c_void> = array_data.iter().map(|s| s.as_ptr() as *mut c_void).collect();
        unsafe {
            tex_set_color_arr_mips(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                ptrs.as_mut_ptr(),
                ptrs.len() as i32,
                mip_count,
                multisample,
                null_mut(),
            )
        };
        self
    }

    /// Set the texture's pixels for a multi-layer and/or mip-mapped texture using a jagged byte array. Each entry
    /// in `array_data` is one layer (face for cubemaps, slice for array textures), packed as
    /// `[mip0][mip1][mip2]...`. The byte layout per mip should match the texture's format.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SetColors.html>
    /// * `width` - Width in pixels of mip 0. Powers of two are generally best!
    /// * `height` - Height in pixels of mip 0. Powers of two are generally best!
    /// * `array_data` - A slice of slices, where each inner slice contains all mip levels for that layer as bytes,
    ///   packed as `[mip0][mip1][mip2]...`.
    /// * `mip_count` - The number of mip levels packed into each layer's data. Use 1 if no mip data is provided
    ///   beyond the base.
    /// * `multisample` - Multisample count, only relevant for rendertarget textures. If None, defaults to 1.
    ///
    /// see also [`tex_set_color_arr_mips`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, tex::{Tex, TexFormat, TexType}};
    /// let layer0_mip0 = [named_colors::RED; 16 * 16];
    /// let layer0_mip1 = [named_colors::GREEN; 8 * 8];
    /// let layer1_mip0 = [named_colors::BLUE; 64 * 64];
    /// let layer1_mip1 = [named_colors::YELLOW; 32 * 32];
    /// let (b0, b1, b2, b3) = unsafe { (
    ///     std::slice::from_raw_parts(layer0_mip0.as_ptr() as *const u8, 16 * 16 * 4),
    ///     std::slice::from_raw_parts(layer0_mip1.as_ptr() as *const u8, 8 * 8 * 4),
    ///     std::slice::from_raw_parts(layer1_mip0.as_ptr() as *const u8, 64 * 64 * 4),
    ///     std::slice::from_raw_parts(layer1_mip1.as_ptr() as *const u8, 32 * 32 * 4),
    /// )};
    /// let array_data = [b0, b1, b2, b3];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// tex.set_colors_arr_mips(16, 16, &array_data, 2, None);
    /// let check_data256 = [named_colors::BLACK; 16 * 16];
    /// let check_data64 = [named_colors::BLACK; 8 * 8];
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data256, 0), true);
    ///     assert_eq!(check_data256, [named_colors::RED; 16 * 16]);
    ///     assert_eq!(tex.get_color_data::<Color32>(&check_data64, 1), true);
    ///     assert_eq!(check_data64, [named_colors::GREEN; 8 * 8]);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_colors_arr_mips(
        &mut self,
        width: usize,
        height: usize,
        array_data: &[&[u8]],
        mip_count: i32,
        multisample: Option<i32>,
    ) -> &mut Self {
        let multisample = multisample.unwrap_or(1);
        let mut ptrs: Vec<*mut c_void> = array_data.iter().map(|s| s.as_ptr() as *mut c_void).collect();
        unsafe {
            tex_set_color_arr_mips(
                self.0.as_ptr(),
                width as i32,
                height as i32,
                ptrs.as_mut_ptr(),
                ptrs.len() as i32,
                mip_count,
                multisample,
                null_mut(),
            )
        };
        self
    }

    /// This will override the default fallback texture that gets used before the Tex has finished loading. This is
    /// useful for textures with a specific purpose where the normal fallback texture would appear strange, such as a
    /// metal/rough map.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FallbackOverride.html>
    ///
    /// see also [`tex_set_fallback`] [`Tex::set_loading_fallback`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType}, mesh::Mesh, material::Material};
    ///
    /// let tex_fallback = Tex::gen_color(named_colors::VIOLET, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// tex.fallback_override(&tex_fallback);
    ///
    /// let tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, Some("tex_left_ID"));
    /// let tex_metal = Tex::from_file("textures/parquet2/parquet2metal.ktx2", true, Some(9999))
    ///                          .expect("Metal tex should be created");
    /// let mut material  = Material::pbr().tex_copy(tex);
    /// material.metal_tex(&tex_metal);
    /// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
    /// let transform_floor = Matrix::t(  [0.0, -0.5, 0.0]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane_mesh.draw(&material,  transform_floor,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn fallback_override<T: AsRef<Tex>>(&mut self, fallback: T) -> &mut Self {
        unsafe { tex_set_fallback(self.0.as_ptr(), fallback.as_ref().0.as_ptr()) };
        self
    }

    /// When sampling a texture that’s stretched, or shrunk beyond its screen size, how do we handle figuring out which
    /// color to grab from the texture? Default is Linear.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SampleMode.html>
    ///
    /// see also [`tex_set_sample`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType, TexSample}};
    ///
    /// let mut tex = Tex::gen_color(named_colors::VIOLET, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert_eq!(tex.get_sample_mode(), TexSample::Linear);
    /// tex.sample_mode(TexSample::Anisotropic);
    /// assert_eq!(tex.get_sample_mode(), TexSample::Anisotropic);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sample_mode(&mut self, sample: TexSample) -> &mut Self {
        unsafe { tex_set_sample(self.0.as_ptr(), sample) };
        self
    }

    /// When sampling from a texture with comparison enabled, the sampler compares the sampled texel value against a
    /// reference value and returns a 0 or 1 based on the result. This is primarily useful for shadow mapping techniques,
    /// where a depth texture is sampled to determine if a surface is in shadow.
    /// <https://stereokit.net/Pages/StereoKit/Tex/SampleComp.html>
    ///
    /// see also [`tex_set_sample_comp`] [`Tex::get_sample_comp`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{util::named_colors, tex::{Tex, TexFormat, TexType, TexSampleComp}};
    /// let mut tex = Tex::gen_color(named_colors::BLACK, 4,4, TexType::Image, TexFormat::Rgba32Srgb);
    /// tex.sample_comp(Some(TexSampleComp::LessOrEq));
    /// assert_eq!(tex.get_sample_comp(), TexSampleComp::LessOrEq);
    ///
    /// tex.sample_comp(None);
    /// assert_eq!(tex.get_sample_comp(), TexSampleComp::None);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sample_comp(&mut self, compare: Option<TexSampleComp>) -> &mut Self {
        let compare = match compare {
            Some(c) => c,
            None => TexSampleComp::None,
        };
        unsafe { tex_set_sample_comp(self.0.as_ptr(), compare) };
        self
    }

    //// When looking at a UV texture coordinate on this texture, how do we handle values larger than 1, or less than zero?
    /// Do we Wrap to the other side? Clamp it between 0-1, or just keep Mirroring back and forth? Wrap is the default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddressMode.html>
    ///
    /// see also [`tex_set_address`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType, TexAddress}};
    ///
    /// let mut tex = Tex::gen_color(named_colors::VIOLET, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert_eq!(tex.get_address_mode(), TexAddress::Wrap);
    /// tex.address_mode(TexAddress::Mirror);
    /// assert_eq!(tex.get_address_mode(), TexAddress::Mirror);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn address_mode(&mut self, address_mode: TexAddress) -> &mut Self {
        unsafe { tex_set_address(self.0.as_ptr(), address_mode) };
        self
    }

    /// When SampleMode is set to Anisotropic, this is the number of samples the GPU takes to figure out the correct color.
    /// Default is 4, and 16 is pretty high.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisoptropy.html>
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisotropy.html>
    ///
    /// see also [`tex_set_anisotropy`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,
    ///                      tex::{Tex, TexFormat, TexType, TexSample}};
    ///
    /// let mut tex = Tex::gen_color(named_colors::VIOLET, 128, 128, TexType::Image, TexFormat::Rgba32Srgb);
    /// assert_eq!(tex.get_sample_mode(), TexSample::Linear);
    /// assert_eq!(tex.get_anisotropy(), 4);
    ///
    /// tex.sample_mode(TexSample::Anisotropic).anisotropy(10);
    ///
    /// assert_eq!(tex.get_anisotropy(), 10);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn anisotropy(&mut self, anisotropy_level: i32) -> &mut Self {
        unsafe { tex_set_anisotropy(self.0.as_ptr(), anisotropy_level) };
        self
    }

    /// Gets the unique identifier of this asset resource! This can be helpful for debugging, managing your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Tex/Id.html>
    ///
    /// see also [`tex_get_id`]
    /// see example in [`Tex::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(tex_get_id(self.0.as_ptr())) }.to_str().unwrap_or_default()
    }

    /// Textures are loaded asyncronously, so this tells you the current state of this texture! This also can tell if
    /// an error occured, and what type of error it may have been.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AssetState.html>
    ///
    /// see also [`tex_asset_state`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, system::AssetState,
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex = Tex::gen_color(named_colors::VIOLET, 128, 128,
    ///                          TexType::Image, TexFormat::Rgba32Srgb);
    /// assert_eq!(tex.get_asset_state(), AssetState::Loaded);
    ///
    /// let tex_icon = Tex::from_file("icons/checked.png", true, None)
    ///                         .expect("Tex_icon should be created");
    /// assert_ne!(tex_icon.get_asset_state(), AssetState::NotFound);
    ///
    /// let tex_not_icon = Tex::from_file("icccons/checddked.png", true, None)
    ///                             .expect("Tex_not_icon should be created");
    /// assert_ne!(tex_not_icon.get_asset_state(), AssetState::Loaded);    
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // We ensure to have the Tex loaded.
    ///     if    tex_icon.get_asset_state()     != AssetState::Loaded
    ///        || tex_not_icon.get_asset_state() == AssetState::Loading { iter -= 1; }     
    /// );
    /// assert_eq!(tex_icon.get_asset_state(),     AssetState::Loaded);    
    /// assert_eq!(tex_not_icon.get_asset_state(), AssetState::NotFound);    
    /// assert_eq!(tex_not_icon.get_width(),  None);
    /// assert_eq!(tex_not_icon.get_height(), None);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_asset_state(&self) -> AssetState {
        unsafe { tex_asset_state(self.0.as_ptr()) }
    }

    /// The StereoKit format this texture was initialized with. This will be a blocking call if AssetState is less than
    /// LoadedMeta so None will be return instead
    /// <https://stereokit.net/Pages/StereoKit/Tex/Format.html>
    ///
    /// see also [`tex_get_format`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, system::AssetState,
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex = Tex::gen_color(named_colors::VIOLET, 128, 128,
    ///                          TexType::Image, TexFormat::Rgba128);
    /// assert_eq!(tex.get_format(), Some(TexFormat::Rgba128));
    ///
    /// let tex_icon = Tex::from_file("icons/checked.png", true, None)
    ///                         .expect("Tex_icon should be created");
    ///
    /// let tex_not_icon = Tex::from_file("icccons/checddked.png", true, None)
    ///                             .expect("Tex_not_icon should be created");
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // We ensure to have the Tex loaded.
    ///     if    tex_icon.get_asset_state()     != AssetState::Loaded
    ///        || tex_not_icon.get_asset_state() == AssetState::Loading { iter -= 1; }     
    /// );
    /// assert_eq!(tex_icon.get_format(), Some(TexFormat::Rgba32Srgb));
    /// assert_eq!(tex_not_icon.get_format(), None);   
    /// # sk::Sk::shutdown();
    /// ```
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
    /// see also [`tex_get_zbuffer`]
    /// see example in [`Tex::set_zbuffer`]
    pub fn get_zbuffer(&self) -> Option<Tex> {
        NonNull::new(unsafe { tex_get_zbuffer(self.0.as_ptr()) }).map(Tex)
    }

    /// This will return the texture’s native resource for use with external libraries. For Vulkan, this should be a
    /// VkImage handle, coerced into the IntPtr. This call will block execution until the texture is loaded, if it is
    /// not already.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetNativeSurface.html>
    ///
    /// see also [`tex_get_surface`]
    /// see example in [`Tex::set_native_surface`]
    pub fn get_native_surface(&self) -> *mut c_void {
        unsafe { tex_get_surface(self.0.as_ptr()) }
    }

    /// This will return the `AHardwareBuffer*` backing this texture, if it was created from one. This call will block
    /// execution until the texture is loaded, if it is not already.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GetHardwareBuffer.html>
    ///
    /// Returns an `AHardwareBuffer*` coerced into a `*mut c_void`, or null if the texture is not backed by a hardware
    /// buffer, or when not on Android.
    ///
    /// see also [`tex_get_hardware_buffer`] [`Tex::from_hardware_buffer`]
    pub fn get_hardware_buffer(&self) -> *mut c_void {
        unsafe { tex_get_hardware_buffer(self.0.as_ptr()) }
    }

    /// The width of the texture, in pixels. This will be a blocking call if AssetState is less than LoadedMeta so None
    /// will be return instead
    /// <https://stereokit.net/Pages/StereoKit/Tex/Width.html>
    ///
    /// see also [`tex_get_width`]
    /// see example in [`Tex::set_size`] [`Tex::get_asset_state`]
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
    /// see also [`tex_get_height`]
    /// see example in [`Tex::set_size`] [`Tex::get_asset_state`]
    pub fn get_height(&self) -> Option<usize> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_height(self.0.as_ptr()) } as usize)
    }

    /// The depth of the texture, in pixels. Only meaningful for 3D (volume) textures created with
    /// [`TexType::Volume`] — for 2D, array, and cubemap textures this is 1. This will be a blocking call if
    /// AssetState is less than LoadedMeta so None will be returned instead.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Depth.html>
    ///
    /// see also [`tex_get_depth`] [`TexType::Volume`] [`Tex::set_colors`]
    pub fn get_depth(&self) -> Option<usize> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_depth(self.0.as_ptr()) } as usize)
    }

    /// Non-canon function which returns a tuple made of (width, heigh, size) of the corresponding texture.
    ///
    /// use `mip` < 0 for textures using [`TexType::ImageNomips`]
    ///
    /// use `mip` >=0 to retrieve the info about one MIP of the texture
    ///
    /// the size corresponding to the mip texture and the width and height of this mip texture
    /// This will be a blocking call if AssetState is less than LoadedMeta so None will be return instead
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32}, system::AssetState,
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let color_dots = [named_colors::CYAN; 16 * 16];
    /// let mut tex = Tex::new(TexType::Image, TexFormat::Rgba32Srgb, None);
    /// tex.set_colors32(16, 16, &color_dots);
    ///
    /// let check_dots = [Color32::WHITE; 16 * 16];
    /// assert!(tex.get_color_data::<Color32>(&check_dots, 0));
    /// assert_eq!(check_dots, color_dots);
    ///
    /// let (width, height, size) = tex.get_data_infos(0).expect("tex should be loaded");
    /// assert_eq!(width, 16);
    /// assert_eq!(height, 16);
    /// assert_eq!(size, 256);
    ///
    /// let (width, height, size) = tex.get_data_infos(1).expect("tex should be loaded");
    /// assert_eq!(width, 8);
    /// assert_eq!(height, 8);
    /// assert_eq!(size, 64);
    ///
    /// let tex_icon = Tex::from_file("icons/checked.png", true, None)
    ///                        .expect("Tex_icon should be created");
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // We ensure to have the Tex loaded.
    ///     if    tex_icon.get_asset_state()     != AssetState::Loaded { iter -= 1; }
    /// );
    /// assert_eq!(tex_icon.get_data_infos(0), Some((128, 128, 16384)));
    /// # sk::Sk::shutdown();
    /// ```
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
    /// see also [`tex_get_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors, Color32, Color128},
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex = Tex::gen_color(named_colors::CYAN, 8 , 8, TexType::Image, TexFormat::Rgba32Srgb);
    ///
    /// let check_dots = [Color32::WHITE; 8 * 8];
    /// assert!(tex.get_color_data::<Color32>(&check_dots, 0));
    /// assert_eq!(check_dots[5], named_colors::CYAN);
    ///
    /// let tex = Tex::gen_color(named_colors::MAGENTA, 8 , 8, TexType::Image, TexFormat::Rgba128);
    ///
    /// let check_dots = [Color128::WHITE; 8 * 8];
    /// assert!(tex.get_color_data::<Color128>(&check_dots, 0));
    /// assert_eq!(check_dots[5], named_colors::MAGENTA.into());
    /// # sk::Sk::shutdown();
    /// ```
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
    /// * `color_size`: number of bytes of the color (Color32: 4, Color128: 16 ...)
    /// * `mip_level` - Retrieves the color data for a specific mip-mapping level. This function will log a fail and
    ///   return a black array if an invalid mip-level is provided.
    ///
    /// The function [`Tex::get_data_infos`] may help you to shape the right receiver.
    ///
    /// see also [`tex_get_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex = Tex::gen_color(named_colors::CYAN, 8 , 8, TexType::Image, TexFormat::Rgba32Srgb);
    ///
    /// let mut check_dots = [0u8; 8 * 8 * 4];
    /// assert!(tex.get_color_data_u8(&mut check_dots, 4, 0));
    /// assert_eq!(check_dots[5*4], named_colors::CYAN.r);
    /// assert_eq!(check_dots[5*4+1], named_colors::CYAN.g);
    /// assert_eq!(check_dots[5*4+2], named_colors::CYAN.b);
    /// assert_eq!(check_dots[5*4+3], named_colors::CYAN.a);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_color_data_u8(&self, color_data: &[u8], color_size: usize, mut mip_level: i8) -> bool {
        let size_of_color = std::mem::size_of_val(color_data);
        let (width, height, size_test) = match self.get_data_infos(mip_level) {
            Some(value) => value,
            None => return false,
        };

        if size_test * color_size != size_of_color {
            Log::err(format!(
                "Size of the Tex {} is {}x{}/mip={} when size of the given buffer is {} instead of {}. Function Tex::get_color_data_u8 failed!",
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
    /// see also [`tex_get_sample`]
    /// see example in [`Tex::sample_mode`]
    pub fn get_sample_mode(&self) -> TexSample {
        unsafe { tex_get_sample(self.0.as_ptr()) }
    }

    /// Retrieves the texture comparison sampling mode. See [`Tex::sample_comp`].
    /// <https://stereokit.net/Pages/StereoKit/Tex/SampleComp.html>
    ///
    /// see also [`tex_get_sample_comp`]
    /// see example in [`Tex::sample_comp`]
    pub fn get_sample_comp(&self) -> TexSampleComp {
        unsafe { tex_get_sample_comp(self.0.as_ptr()) }
    }

    /// When looking at a UV texture coordinate on this texture, how do we handle values larger than 1, or less than
    /// zero? Do we Wrap to the other side? Clamp it between 0-1, or just keep Mirroring back and forth? Wrap is the
    /// default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/AddressMode.html>
    ///
    /// see also [`tex_get_address`]
    /// see example in [`Tex::address_mode`]
    pub fn get_address_mode(&self) -> TexAddress {
        unsafe { tex_get_address(self.0.as_ptr()) }
    }

    /// When SampleMode is set to Anisotropic, this is the number of samples the GPU takes to figure out the correct
    /// color. Default is 4, and 16 is pretty high.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisoptropy.html>
    /// <https://stereokit.net/Pages/StereoKit/Tex/Anisotropy.html>
    ///
    /// see also [`tex_get_anisotropy`]
    /// see example in [`Tex::anisotropy`]
    pub fn get_anisotropy(&self) -> i32 {
        unsafe { tex_get_anisotropy(self.0.as_ptr()) }
    }

    /// The number of mip-map levels this texture has. This will be 1 if the texture doesn’t have mip mapping enabled.
    /// This will be a blocking call if AssetState is less than LoadedMeta so None will be return instead.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Mips.html>
    ///
    /// see also [`tex_get_mips`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, system::{AssetState, Assets},
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex_nomips = Tex::gen_color(named_colors::VIOLET, 128, 128,
    ///                                 TexType::ImageNomips, TexFormat::Rgba32Srgb);
    ///
    /// let tex = Tex::gen_color(named_colors::VIOLET, 128, 128,
    ///                          TexType::Image, TexFormat::Rgba32Srgb);
    ///
    /// let tex_icon = Tex::from_file("icons/checked.png", true, None)
    ///                         .expect("Tex_icon should be created");
    /// assert_eq!(tex_icon.get_mips(), None);
    ///
    /// let tex_not_icon = Tex::from_file("Not an icon file", true, None)
    ///                             .expect("Tex_not_icon should be created");
    /// assert_eq!(tex_not_icon.get_mips(), None);
    ///
    /// Assets::block_for_priority(99);
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // We ensure to have the Tex loaded.
    ///     if    tex_icon.get_asset_state()     != AssetState::Loaded
    ///        || tex_not_icon.get_asset_state() == AssetState::Loading { iter -= 1; }
    /// );
    /// assert_eq!(tex_nomips.get_mips(), Some(1));
    /// assert_eq!(tex.get_mips(), Some(8));
    /// assert_eq!(tex_icon.get_mips(), Some(8));
    /// assert_eq!(tex_not_icon.get_mips(), None);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_mips(&self) -> Option<i32> {
        match self.get_asset_state() {
            AssetState::Loaded => (),
            AssetState::LoadedMeta => (),
            AssetState::None => (),
            _ => return None,
        }
        Some(unsafe { tex_get_mips(self.0.as_ptr()) })
    }

    /// ONLY valid for cubemap textures! This will calculate a spherical harmonics representation of the cubemap for use
    /// with StereoKit’s lighting. First call may take a frame  or two of time, but subsequent calls will pull from a
    /// cached value.
    /// <https://stereokit.net/Pages/StereoKit/Tex/CubemapLighting.html>
    ///
    /// see also [`tex_get_cubemap_lighting`] use instead [`SHCubemap`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, maths::Vec3,
    ///                      tex::{Tex, TexFormat, TexType}};
    ///
    /// let tex = Tex::gen_color(named_colors::VIOLET, 128, 128,
    ///                          TexType::Cubemap, TexFormat::Rgba32Srgb);
    ///
    /// // Cubemap must be created with SHCubemap static methods.
    /// let sh_cubemap = tex.get_cubemap_lighting();
    /// assert_eq!(sh_cubemap.sh.coefficients[2]/10.0, Vec3::ZERO);
    /// assert_eq!(sh_cubemap.sh.coefficients[5]/10.0, Vec3::ZERO);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_cubemap_lighting(&self) -> SHCubemap {
        SHCubemap {
            sh: unsafe { tex_get_cubemap_lighting(self.0.as_ptr()) },
            tex: Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.0.as_ptr())) })
                .expect("SHCubemap::get_cubemap_lighting Tex should be found!")),
        }
    }

    /// Default 2x2 black opaque texture, this is the texture referred to as ‘black’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Black.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex= Tex::black();
    /// assert_eq!(tex.get_id(), "default/tex_black");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn black() -> Self {
        Self::find("default/tex_black").unwrap_or_default()
    }

    /// This is a white checkered grid texture used to easily add visual features to materials. By default, this is used
    /// for the loading fallback texture for all Tex objects.
    /// <https://stereokit.net/Pages/StereoKit/Tex/DevTex.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::dev_tex();
    /// assert_eq!(tex.get_id(), "default/tex_devtex");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn dev_tex() -> Self {
        Self::find("default/tex_devtex").unwrap_or_default()
    }

    /// This is a red checkered grid texture used to indicate some sort of error has occurred. By default, this is used
    /// for the error fallback texture for all Tex objects.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Error.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::error();
    /// assert_eq!(tex.get_id(), "default/tex_error");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn error() -> Self {
        Self::find("default/tex_error").unwrap_or_default()
    }

    /// Default 2x2 flat normal texture, this is a normal that faces out from the, face, and has a color value of
    /// (0.5,0.5,1). This is the texture referred to as ‘flat’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Flat.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::flat();
    /// assert_eq!(tex.get_id(), "default/tex_flat");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn flat() -> Self {
        Self::find("default/tex_flat").unwrap_or_default()
    }

    /// Default 2x2 middle gray (0.5,0.5,0.5) opaque texture, this is the texture referred to as ‘gray’ in the shader
    /// texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Gray.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::gray();
    /// assert_eq!(tex.get_id(), "default/tex_gray");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn gray() -> Self {
        Self::find("default/tex_gray").unwrap_or_default()
    }

    /// Default 2x2 roughness color (1,1,0,1) texture, this is the texture referred to as ‘rough’ in the shader texture
    /// defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Rough.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::rough();
    /// assert_eq!(tex.get_id(), "default/tex_rough");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn rough() -> Self {
        Self::find("default/tex_rough").unwrap_or_default()
    }

    /// Default 2x2 white opaque texture, this is the texture referred to as ‘white’ in the shader texture defaults.
    /// <https://stereokit.net/Pages/StereoKit/Tex/White.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::white();
    /// assert_eq!(tex.get_id(), "default/tex");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn white() -> Self {
        Self::find("default/tex").unwrap_or_default()
    }

    /// Default 1x1x1 transparent black volume texture used as the fallback for unloaded or errored 3D textures.
    /// <https://stereokit.net/Pages/StereoKit/Tex.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::Tex;
    ///
    /// let tex = Tex::volume_3d();
    /// assert_eq!(tex.get_id(), "default/tex_3d");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn volume_3d() -> Self {
        Self::find("default/tex_3d").unwrap_or_default()
    }

    // /// The equirectangular texture used for the default dome
    // /// <https://stereokit.net/Pages/StereoKit/Tex.html>
    // pub fn cubemap() -> Self {
    //     Self::find("default/tex_cubemap").unwrap_or_default()
    // }
}

/// fluent syntax for Texture cubemap
/// <https://stereokit.net/Pages/StereoKit/Tex.html>
///
/// see also [`Tex`] [`crate::util::SphericalHarmonics`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{tex::SHCubemap, system::AssetState};
///
/// let sh_cubemap = SHCubemap::from_cubemap("hdri/sky_dawn.hdr", true, 9999)
///                                .expect("Cubemap should be created");
///
/// sh_cubemap.render_as_sky();
/// assert_eq!(sh_cubemap.tex.get_asset_state(), AssetState::Loaded);
///
/// let tex = sh_cubemap.tex;
///
/// filename_scr = "screenshots/sh_cubemap.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     if tex.get_asset_state() != AssetState::Loaded {iter -= 1}
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sh_cubemap.jpeg" alt="screenshot" width="200">
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
    /// see also [`tex_create_cubemap_file`]
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
    /// * `cubemap_file` - Filename of the cubemap image.
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that's not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have
    ///   a big impact on visuals.
    /// * `load_priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// see also [`tex_create_cubemap_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, tex::SHCubemap, system::AssetState};
    ///
    /// let sh_cubemap = SHCubemap::from_cubemap("hdri/sky_dawn.hdr", true, 9999)
    ///                                .expect("Cubemap should be created");
    /// sh_cubemap.render_as_sky();
    ///
    /// let tex = sh_cubemap.tex;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if tex.get_asset_state() != AssetState::Loaded {iter -= 1}
    ///     
    ///     assert_ne!(sh_cubemap.sh.coefficients[0], Vec3::ZERO);
    ///     assert_ne!(sh_cubemap.sh.coefficients[8], Vec3::ZERO);
    /// );
    /// assert_eq!(tex.get_asset_state(), AssetState::Loaded);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_cubemap(
        cubemap_file: impl AsRef<Path>,
        srgb_data: bool,
        load_priority: i32,
    ) -> Result<SHCubemap, StereoKitError> {
        let path = cubemap_file.as_ref();
        let path_buf = path.to_path_buf();
        let c_str = CString::new(
            path.to_str().ok_or(StereoKitError::TexCString(path.to_str().unwrap_or_default().to_owned()))?,
        )?;
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
    /// * `files_utf8` - 6 image filenames, in order of/ +X, -X, +Y, -Y, +Z, -Z.
    /// * `srgb_data` - Is this image color data in sRGB format, or is it normal/metal/rough/data that's not for direct
    ///   display? sRGB colors get converted to linear color space on the graphics card, so getting this right can have a
    ///   big impact on visuals.
    /// * `load_priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// see also [`tex_create_cubemap_files`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{system::AssetState, tex::SHCubemap};
    ///
    /// let cubemap_files = [
    ///     "hdri/giza/right.png",
    ///     "hdri/giza/left.png",
    ///     "hdri/giza/top.png",
    ///     "hdri/giza/bottom.png",
    ///     "hdri/giza/front.png",
    ///     "hdri/giza/back.png",
    /// ];
    /// let sh_cubemap = SHCubemap::from_cubemap_files(&cubemap_files, true, 9999)
    ///                                 .expect("Cubemap should be created");
    /// sh_cubemap.render_as_sky();
    ///
    /// let tex = sh_cubemap.tex;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if tex.get_asset_state() != AssetState::Loaded {iter -= 1}
    /// );
    /// assert_eq!(tex.get_asset_state(), AssetState::Loaded);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_cubemap_files<P: AsRef<Path>>(
        files_utf8: &[P; 6],
        srgb_data: bool,
        load_priority: i32,
    ) -> Result<SHCubemap, StereoKitError> {
        let mut c_files = Vec::new();
        for path in files_utf8 {
            let path = path.as_ref();
            let path_buf = path.to_path_buf();
            let c_str = CString::new(
                path.to_str().ok_or(StereoKitError::TexCString(path_buf.to_str().unwrap_or_default().to_owned()))?,
            )?;
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
    /// * `gradient` - A color gradient the generator will sample from! This looks at the 0-1 range of the gradient.
    /// * `gradient_dir` - This vector points to where the ‘top’ of the color gradient will go. Conversely, the ‘bottom’
    ///   of the gradient will be opposite, and it’ll blend along that axis.
    /// * `resolution` - The square size in pixels of each cubemap face! This generally doesn’t need to be large, unless
    ///   you have a really complicated gradient. 16 is a good default value.
    ///
    /// see also [`tex_gen_cubemap`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, tex::SHCubemap, system::AssetState,
    ///                      util::{named_colors, Gradient, GradientKey, Color128}};
    ///
    /// let keys = [
    ///     GradientKey::new(Color128::BLACK_TRANSPARENT, 0.0),
    ///     GradientKey::new(named_colors::RED, 0.1),
    ///     GradientKey::new(named_colors::CYAN, 0.4),
    ///     GradientKey::new(named_colors::YELLOW, 0.5),
    ///     GradientKey::new(Color128::BLACK, 0.7)];
    ///
    /// let sh_cubemap = SHCubemap::gen_cubemap_gradient(Gradient::new(Some(&keys)),
    ///                                                  Vec3::UP, 128);
    /// sh_cubemap.render_as_sky();
    ///
    /// let tex = sh_cubemap.tex;
    /// assert_eq!(tex.get_asset_state(), AssetState::Loaded);
    /// assert_ne!(sh_cubemap.sh.coefficients[0], Vec3::ZERO);
    /// assert_ne!(sh_cubemap.sh.coefficients[8], Vec3::ZERO);
    /// test_steps!( // !!!! Get a proper main loop !!!!
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn gen_cubemap_gradient(
        gradient: impl AsRef<Gradient>,
        gradient_dir: impl Into<Vec3>,
        resolution: i32,
    ) -> SHCubemap {
        let mut sh = SphericalHarmonics::default();
        let tex = Tex(NonNull::new(unsafe {
            tex_gen_cubemap(gradient.as_ref().0.as_ptr(), gradient_dir.into(), resolution, &mut sh)
        })
        .expect("SHCubemap::gen_cubemap_gradient should create texture"));
        //unsafe { sk.tex_addref(&cubemap.1) }
        SHCubemap { sh, tex }
    }

    /// Create the associated cubemap texture with the light spot.
    /// warning ! The SphericalHarmonics is moved to the result struct.
    /// <https://stereokit.net/Pages/StereoKit/Tex/GenCubemap.html>
    /// * `lighting` - Lighting information stored in a SphericalHarmonics.
    /// * `resolution` - The square size in pixels of each cubemap face! This generally doesn’t need to be large, as
    ///   SphericalHarmonics typically contain pretty low frequency information.
    /// * `light_spot_size_pct` - The size of the glowing spot added in the primary light direction. You can kinda think
    ///   of the unit as a percentage of the cubemap face’s size, but it’s technically a Chebyshev distance from the
    ///   light’s point on a 2m cube.
    /// * `light_spot_intensity` - The glowing spot’s color is the primary light direction’s color, but multiplied by
    ///   this value. Since this method generates a 128bpp texture, this is not clamped between 0-1, so feel free to go
    ///   nuts here! Remember that reflections will often cut down some reflection intensity.
    ///
    ///
    /// see also [`tex_gen_cubemap_sh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, tex::SHCubemap, system::AssetState,
    ///                      util::{named_colors, SHLight, SphericalHarmonics}};
    ///
    /// let lights: [SHLight; 1] = [SHLight::new(Vec3::ONE, named_colors::WHITE); 1];
    /// let sh = SphericalHarmonics::from_lights(&lights);
    /// let sh_cubemap = SHCubemap::gen_cubemap_sh(sh, 128, 0.5, 1.0);
    /// sh_cubemap.render_as_sky();
    ///
    /// let tex = sh_cubemap.tex;
    /// assert_eq!(tex.get_asset_state(), AssetState::Loaded);
    /// assert_eq!(sh_cubemap.sh.get_dominent_light_direction(), -Vec3::ONE.get_normalized());
    /// assert_ne!(sh_cubemap.sh.coefficients[0], Vec3::ZERO);
    /// assert_ne!(sh_cubemap.sh.coefficients[1], Vec3::ZERO);
    /// assert_eq!(sh_cubemap.sh.coefficients[8], Vec3::ZERO);
    /// test_steps!( // !!!! Get a proper main loop !!!!
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn gen_cubemap_sh(
        lighting: SphericalHarmonics,
        resolution: i32,
        light_spot_size_pct: f32,
        light_spot_intensity: f32,
    ) -> SHCubemap {
        let tex = Tex(NonNull::new(unsafe {
            tex_gen_cubemap_sh(&lighting, resolution, light_spot_size_pct, light_spot_intensity)
        })
        .expect("SHCubemap::gen_cubemap_sh should create texture"));
        SHCubemap { sh: lighting, tex }
    }

    /// Get the associated lighting extracted from the cubemap.
    /// <https://stereokit.net/Pages/StereoKit/Tex/CubemapLighting.html>
    ///
    /// see also [`tex_gen_cubemap_sh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, tex::SHCubemap, system::AssetState,
    ///                      util::{named_colors, SHLight, SphericalHarmonics}};
    ///
    /// let lights: [SHLight; 1] = [SHLight::new(Vec3::ONE, named_colors::WHITE); 1];
    /// let sh = SphericalHarmonics::from_lights(&lights);
    /// let sh_cubemap = SHCubemap::gen_cubemap_sh(sh, 128, 0.5, 1.0);
    /// let tex = sh_cubemap.tex;
    ///
    /// let mut sh_cubemap2 = SHCubemap::get_cubemap_lighting(tex);
    /// sh_cubemap2.sh.brightness(1.0);
    /// sh_cubemap2.render_as_sky();
    /// let tex2 = sh_cubemap2.tex;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(tex2.get_asset_state(), AssetState::Loaded);
    ///     assert_eq!(sh_cubemap2.sh.get_dominent_light_direction(), -Vec3::ONE.get_normalized());
    ///     assert_ne!(sh_cubemap2.sh.coefficients[0], Vec3::ZERO);
    ///     assert_ne!(sh_cubemap2.sh.coefficients[1], Vec3::ZERO);
    ///     assert_eq!(sh_cubemap2.sh.coefficients[8], Vec3::ZERO);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_cubemap_lighting(cubemap_texture: impl AsRef<Tex>) -> SHCubemap {
        SHCubemap {
            sh: unsafe { tex_get_cubemap_lighting(cubemap_texture.as_ref().0.as_ptr()) },
            tex: Tex(NonNull::new(unsafe { tex_find(tex_get_id(cubemap_texture.as_ref().0.as_ptr())) })
                .expect("SHCubemap::get_cubemap_lighting Tex should be found!")),
        }
    }

    /// Get the cubemap texture and SH light of the the current skylight
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`crate::render::Renderer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::SHCubemap, system::AssetState};
    ///
    /// let sh_cubemap = SHCubemap::get_rendered_sky();
    ///
    /// let tex = sh_cubemap.tex;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if tex.get_asset_state() != AssetState::Loaded {iter -= 1}
    ///     assert_eq!(tex.get_id(), "default/cubemap");
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_rendered_sky() -> SHCubemap {
        let skytex_ptr = unsafe { render_get_skytex() };
        let tex = if let Some(nonnull_ptr) = NonNull::new(skytex_ptr) {
            Tex(nonnull_ptr)
        } else {
            // Si render_get_skytex() retourne null, on crée un SHCubemap par défaut
            Log::warn("render_get_skytex() returned null, creating default sky cubemap");
            let gradient_keys = [
                crate::util::GradientKey::new(crate::util::Color128::new(0.2, 0.4, 0.8, 1.0), 0.0), // Bleu ciel
                crate::util::GradientKey::new(crate::util::Color128::new(0.8, 0.9, 1.0, 1.0), 1.0), // Blanc nuageux
            ];
            let gradient = crate::util::Gradient::new(Some(&gradient_keys));
            let default_sh_cubemap = SHCubemap::gen_cubemap_gradient(gradient, crate::maths::Vec3::UP, 64);
            return default_sh_cubemap;
        };

        SHCubemap { sh: unsafe { render_get_skylight() }, tex }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Tex/Find.html>
    ///
    /// see also [`tex_find()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::tex::SHCubemap;
    ///
    /// let sh_cubemap = SHCubemap::get_rendered_sky();
    ///
    /// let cubemap = sh_cubemap.clone_ref();
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///    if cubemap.tex.get_asset_state() != system::AssetState::Loaded {iter -= 1}
    ///    assert_eq!(cubemap.tex.get_id(), "default/cubemap");
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> SHCubemap {
        SHCubemap { sh: self.sh, tex: self.tex.clone_ref() }
    }

    /// set the spherical harmonics as skylight and the the cubemap texture as skytex
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also see also [`crate::render::Renderer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::SHCubemap, render::Renderer};
    ///
    /// let sh_cubemap = SHCubemap::from_cubemap("hdri/sky_dawn.hdr", true, 9999)
    ///                                .expect("Cubemap should be created");
    /// assert_eq!(Renderer::get_enable_sky(), true);
    ///
    /// sh_cubemap.render_as_sky();
    ///
    /// Renderer::enable_sky(false);
    /// assert_eq!(Renderer::get_enable_sky(), false);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn render_as_sky(&self) {
        unsafe {
            render_set_skylight(&self.sh);
            render_set_skytex(self.tex.0.as_ptr());
        }
    }

    /// Get the cubemap tuple
    ///
    /// see also [`Tex`] [`crate::util::SphericalHarmonics`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::SHCubemap, maths::Vec3};
    ///
    /// let sh_cubemap = SHCubemap::get_rendered_sky();
    ///
    /// let (sh, tex) = sh_cubemap.get();
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if tex.get_asset_state() != system::AssetState::Loaded {iter -= 1}
    ///     assert_eq!(tex.get_id(), "default/cubemap");
    ///     assert_eq!(sh.get_dominent_light_direction(), Vec3 { x: -0.20119436, y: -0.92318374, z: -0.32749438 });
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get(&self) -> (SphericalHarmonics, Tex) {
        (
            self.sh,
            Tex(NonNull::new(unsafe { tex_find(tex_get_id(self.tex.0.as_ptr())) })
                .expect("SHCubemap::get Tex should be found!")),
        )
    }
}
