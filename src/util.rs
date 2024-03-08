use crate::{
    maths::{lerp, Bool32T, Vec3},
    sk::DisplayBlend,
    system::TextContext,
    StereoKitError,
};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    fmt::Display,
    ops::{Div, DivAssign, Mul, MulAssign},
    ptr::NonNull,
};

/// A color value stored as 4 floats with values that are generally between 0 and 1! Note that there’s also a Color32
/// structure, and that 4 floats is generally a lot more than you need. So, use this for calculating individual
/// colors at quality, but maybe store them en-masse with Color32!
///
/// Also note that RGB is often a terrible color format for picking colors, but it’s how our displays work and we’re
/// stuck with it. If you want to create a color via code, try out the static Color.HSV method instead!
///
/// A note on gamma space vs. linear space colors! Color is not inherently one or the other, but most color values we
/// work with tend to be gamma space colors, so most functions in StereoKit are gamma space. There are occasional
/// functions that will ask for linear space colors instead, primarily in performance critical places, or places where
/// a color may not always be a color! However, performing math on gamma space colors is bad, and will result in
/// incorrect colors. We do our best to indicate what color space a function uses, but it’s not enforced through syntax!
/// <https://stereokit.net/Pages/StereoKit/Color.html>
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Color128 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl From<[f32; 4]> for Color128 {
    fn from(s: [f32; 4]) -> Self {
        Self::new(s[0], s[1], s[2], s[3])
    }
}

impl From<Color32> for Color128 {
    fn from(a: Color32) -> Self {
        Self::new(a.r as f32 / 255.0, a.g as f32 / 255.0, a.b as f32 / 255.0, a.a as f32 / 255.0)
    }
}

extern "C" {
    pub fn color_hsv(hue: f32, saturation: f32, value: f32, transparency: f32) -> Color128;
    pub fn color_to_hsv(color: *const Color128) -> Vec3;
    pub fn color_lab(l: f32, a: f32, b: f32, transparency: f32) -> Color128;
    pub fn color_to_lab(color: *const Color128) -> Vec3;
    pub fn color_to_linear(srgb_gamma_correct: Color128) -> Color128;
    pub fn color_to_gamma(srgb_linear: Color128) -> Color128;
}

impl Color128 {
    pub const BLACK: Color128 = Color128 { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const BLACK_TRANSPARENT: Color128 = Color128 { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const WHITE: Color128 = Color128 { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1.
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1. Alpha will be set to 1.0
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    pub const fn new_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a Red/Green/Blue gamma space color from Hue/Saturation/Value information.
    /// <https://stereokit.net/Pages/StereoKit/Color/HSV.html>
    pub fn hsv(hue: f32, saturation: f32, value: f32, transparency: f32) -> Color128 {
        unsafe { color_hsv(hue, saturation, value, transparency) }
    }

    /// Creates a Red/Green/Blue gamma space color from Hue/Saturation/Value information.
    /// <https://stereokit.net/Pages/StereoKit/Color/HSV.html>
    pub fn hsv_vec3(vec: Vec3, transparency: f32) -> Self {
        unsafe { color_hsv(vec.x, vec.y, vec.z, transparency) }
    }

    /// Creates a gamma space RGB color from a CIE-Lab color space. CIE-Lab is a color space that models human
    /// perception, and has significantly more accurate to perception lightness values, so this is an excellent color
    /// space for color operations that wish to preserve color brightness properly.
    ///
    /// Traditionally, values are L [0,100], a,b [-200,+200] but here we normalize them all to the 0-1 range. If you
    /// hate it, let me know why!
    /// <https://stereokit.net/Pages/StereoKit/Color/LAB.html>    
    pub fn lab(l: f32, a: f32, b: f32, transparency: f32) -> Self {
        unsafe { color_lab(l, a, b, transparency) }
    }

    /// Creates a gamma space RGB color from a CIE-Lab color space. CIE-Lab is a color space that models human
    /// perception, and has significantly more accurate to perception lightness values, so this is an excellent color
    /// space for color operations that wish to preserve color brightness properly.
    ///
    /// Traditionally, values are L [0,100], a,b [-200,+200] but here we normalize them all to the 0-1 range. If you
    /// hate it, let me know why!
    /// <https://stereokit.net/Pages/StereoKit/Color/LAB.html>    
    pub fn lab_vec3(vec: Vec3, transparency: f32) -> Self {
        unsafe { color_lab(vec.x, vec.y, vec.z, transparency) }
    }

    /// Create a color from an integer based hex value! This can make it easier to copy over colors from the web. This
    /// isn’t a string function though, so you’ll need to fill it out the whole way. Ex: Color.Hex(0x0000FFFF) would
    /// be RGBA(0,0,1,1).
    /// <https://stereokit.net/Pages/StereoKit/Color/Hex.html>    
    pub fn hex(hex_value: u32) -> Self {
        Self {
            r: (hex_value >> 24) as f32 / 255.0,
            g: ((hex_value >> 16) & 0x000000FF) as f32 / 255.0,
            b: ((hex_value >> 8) & 0x000000FF) as f32 / 255.0,
            a: (hex_value & 0x000000FF) as f32 / 255.0,
        }
    }

    /// This will linearly blend between two different colors! Best done on linear colors, rather than gamma corrected
    /// colors, but will work either way. This will not clamp the percentage to the 0-1 range.
    /// <https://stereokit.net/Pages/StereoKit/Color/Lerp.html>    
    pub fn lerp(a: Color128, b: Color128, t: f32) -> Self {
        Self { r: lerp(a.r, b.r, t), g: lerp(a.g, b.g, t), b: lerp(a.b, b.b, t), a: lerp(a.a, b.a, t) }
    }

    /// Converts this from a gamma space color, into a linear space color! If this is not a gamma space color, this
    /// will just make your color wacky!
    /// <https://stereokit.net/Pages/StereoKit/Color/ToLinear.html>    
    pub fn to_linear(&self) -> Self {
        unsafe { color_to_linear(*self) }
    }

    /// Converts this from a linear space color, into a gamma space color! If this is not a linear space color, this
    /// will just make your color wacky!
    /// <https://stereokit.net/Pages/StereoKit/Color/ToHSV.html>    
    pub fn to_gamma(&self) -> Self {
        unsafe { color_to_gamma(*self) }
    }

    /// Converts the gamma space color to a Hue/Saturation/Value format! Does not consider transparency when
    /// calculating the result.
    /// <https://stereokit.net/Pages/StereoKit/Color/ToHSV.html>    
    pub fn to_hsv(&self) -> Vec3 {
        unsafe { color_to_hsv(self) }
    }

    /// Converts the gamma space RGB color to a CIE LAB color space value! Conversion back and forth from LAB space
    /// could be somewhat lossy.
    /// <https://stereokit.net/Pages/StereoKit/Color/ToLAB.html>    
    pub fn to_lab(&self) -> Vec3 {
        unsafe { color_to_lab(self) }
    }
}

impl Display for Color128 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the color in debug mode. Looks like
    /// “[r, g, b, a]”
    /// <https://stereokit.net/Pages/StereoKit/Color/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r:{}, g:{}, b:{}, a:{}", self.r, self.g, self.b, self.a)
    }
}

/// This will add a color component-wise with another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Addition.html>
impl std::ops::Add for Color128 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Color128 { r: self.r + rhs.r, g: self.g + rhs.g, b: self.b + rhs.b, a: self.a + rhs.a }
    }
}

/// This will subtract color b component-wise from color a, including alpha. Best done on colors in linear space. No
/// clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Subtraction.html>
impl std::ops::Sub for Color128 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Color128 { r: self.r - rhs.r, g: self.g - rhs.g, b: self.b - rhs.b, a: self.a - rhs.a }
    }
}

/// This will divide a color component-wise against another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Division.html>
impl Div<Color128> for Color128 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { r: self.r.div(rhs.r), g: self.g.div(rhs.g), b: self.b.div(rhs.b), a: self.a.div(rhs.a) }
    }
}

/// This will divide a color component-wise against another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Division.html>
impl DivAssign<Color128> for Color128 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.r.div_assign(rhs.r);
        self.g.div_assign(rhs.g);
        self.b.div_assign(rhs.b);
        self.a.div_assign(rhs.a);
    }
}

/// This will divide a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Division.html>
impl Div<f32> for Color128 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { r: self.r.div(rhs), g: self.g.div(rhs), b: self.b.div(rhs), a: self.a.div(rhs) }
    }
}

/// This will divide a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Division.html>
impl DivAssign<f32> for Color128 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.r.div_assign(rhs);
        self.g.div_assign(rhs);
        self.b.div_assign(rhs);
        self.a.div_assign(rhs);
    }
}

/// This will divide a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Division.html>
impl Div<Color128> for f32 {
    type Output = Color128;
    #[inline]
    fn div(self, rhs: Color128) -> Self::Output {
        Self::Output { r: self.div(rhs.r), g: self.div(rhs.g), b: self.div(rhs.b), a: self.div(rhs.a) }
    }
}

/// This will multiply a color component-wise against another color, including alpha. Best done on colors in linear
/// space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Multiply.html>
impl Mul<Color128> for Color128 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { r: self.r.mul(rhs.r), g: self.g.mul(rhs.g), b: self.b.mul(rhs.b), a: self.a.mul(rhs.a) }
    }
}

/// This will multiply a color component-wise against another color, including alpha. Best done on colors in linear
/// space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Multiply.html>
impl MulAssign<Color128> for Color128 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.r.mul_assign(rhs.r);
        self.g.mul_assign(rhs.g);
        self.b.mul_assign(rhs.b);
        self.a.mul_assign(rhs.a);
    }
}

/// This will multiply a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Multiply.html>
impl Mul<f32> for Color128 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { r: self.r.mul(rhs), g: self.g.mul(rhs), b: self.b.mul(rhs), a: self.a.mul(rhs) }
    }
}

/// This will multiply a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Multiply.html>
impl MulAssign<f32> for Color128 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.r.mul_assign(rhs);
        self.g.mul_assign(rhs);
        self.b.mul_assign(rhs);
        self.a.mul_assign(rhs);
    }
}

/// This will multiply a color linearly, including alpha. Best done on a color in linear space. No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Multiply.html>
impl Mul<Color128> for f32 {
    type Output = Color128;
    #[inline]
    fn mul(self, rhs: Color128) -> Self::Output {
        Self::Output { r: self.mul(rhs.r), g: self.mul(rhs.g), b: self.mul(rhs.b), a: self.mul(rhs.a) }
    }
}

/// A 32 bit color struct! This is often directly used by StereoKit data structures, and so is often necessary for
/// setting texture data, or mesh data. Note that the Color type implicitly converts to Color32, so you can use the
/// static methods there to create Color32 values!
///
/// It’s generally best to avoid doing math on 32-bit color values, as they lack the precision necessary to create
/// results. It’s best to think of a Color32 as an optimized end stage format of a color.
/// <https://stereokit.net/Pages/StereoKit/Color32.html>
///
/// See also [Color128]
#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Color32 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color128> for Color32 {
    fn from(a: Color128) -> Self {
        Self::new((a.r * 255.0) as u8, (a.g * 255.0) as u8, (a.b * 255.0) as u8, (a.a * 255.0) as u8)
    }
}
impl From<[u8; 4]> for Color32 {
    fn from(s: [u8; 4]) -> Self {
        Self::new(s[0], s[1], s[2], s[3])
    }
}

macro_rules! named_color {
    ($name:ident, $r:expr, $g:expr, $b:expr) => {
        #[doc = "as defined here : https://www.w3.org/wiki/CSS/Properties/color/keywords"]
        pub const $name: crate::util::Color32 = crate::util::Color32 { r: $r, g: $g, b: $b, a: 255 };
    };
}

impl Color32 {
    pub const BLACK: Color32 = Color32 { r: 0, g: 0, b: 0, a: 255 };
    pub const BLACK_TRANSPARENT: Color32 = Color32 { r: 0, g: 0, b: 0, a: 0 };
    pub const WHITE: Color32 = Color32 { r: 255, g: 255, b: 255, a: 255 };

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32.Hex.
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32.Hex.
    /// a is set to 255 !
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    pub const fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a color from an integer based hex value! This can make it easier to copy over colors from the web. This
    /// isn’t a string function though, so you’ll need to fill it out the whole way. Ex: Color.Hex(0x0000FFFF) would be
    /// RGBA(0,0,255,255).
    /// <https://stereokit.net/Pages/StereoKit/Color32/Hex.html>
    pub fn hex(hex_value: u32) -> Self {
        Self {
            r: (hex_value >> 24) as u8,
            g: ((hex_value >> 16) & 0x000000FF) as u8,
            b: ((hex_value >> 8) & 0x000000FF) as u8,
            a: (hex_value & 0x000000FF) as u8,
        }
    }
}

impl Display for Color32 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the color in debug mode. Looks like
    /// “[r, g, b, a]”
    /// <https://stereokit.net/Pages/StereoKit/Color/ToString.html>  
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r:{}, g:{}, b:{}, a:{}", self.r, self.g, self.b, self.a)
    }
}

// Named colors <https://www.w3.org/wiki/CSS/Properties/color/keywords>
pub mod named_colors {
    // Basic :
    named_color!(BLACK, 0, 0, 0);
    named_color!(SILVER, 192, 192, 192);
    named_color!(GRAY, 128, 128, 128);
    named_color!(WHITE, 255, 255, 255);
    named_color!(MAROON, 128, 0, 0);
    named_color!(RED, 255, 0, 0);
    named_color!(PURPLE, 128, 0, 128);
    named_color!(FUCHSIA, 255, 0, 255);
    named_color!(GREEN, 0, 128, 0);
    named_color!(LIME, 0, 255, 0);
    named_color!(OLIVE, 128, 128, 0);
    named_color!(YELLOW, 255, 255, 0);
    named_color!(NAVY, 0, 0, 128);
    named_color!(BLUE, 0, 0, 255);
    named_color!(TEAL, 0, 128, 128);
    named_color!(AQUA, 0, 255, 255);

    //Extended
    named_color!(ALICE_BLUE, 240, 248, 255);
    named_color!(ANTIQUE_WHITE, 250, 235, 215);
    named_color!(AQUAMARINE, 127, 255, 212);
    named_color!(AZURE, 240, 255, 255);
    named_color!(BEIGE, 255, 228, 196);
    named_color!(BISQUE, 255, 228, 196);
    named_color!(BLANCHED_ALMOND, 255, 235, 205);
    named_color!(BLUE_VIOLET, 138, 43, 226);
    named_color!(BROWN, 165, 42, 42);
    named_color!(BURLY_WOOD, 222, 184, 135);
    named_color!(CADET_BLUE, 95, 158, 160);
    named_color!(CHARTREUSE, 127, 255, 0);
    named_color!(CHOCOLATE, 210, 105, 30);
    named_color!(CORAL, 255, 127, 80);
    named_color!(CORNFLOWER_BLUE, 100, 149, 237);
    named_color!(CORN_SILK, 255, 248, 220);
    named_color!(CRIMSON, 220, 20, 60);
    named_color!(CYAN, 0, 255, 255);
    named_color!(DARK_BLUE, 0, 0, 139);
    named_color!(DARK_CYAN, 0, 139, 139);
    named_color!(DARK_GOLDEN_ROD, 184, 134, 11);
    named_color!(DARK_GRAY, 169, 169, 169);
    named_color!(DARK_GREEN, 0, 100, 0);
    named_color!(DARK_GREY, 169, 169, 169);
    named_color!(DARK_KHAKI, 189, 183, 107);
    named_color!(DARK_MAGENTA, 139, 0, 139);
    named_color!(DARK_OLIVE_GREEN, 85, 107, 47);
    named_color!(DARK_ORANGE, 255, 140, 0);
    named_color!(DARK_ORCHID, 153, 50, 204);
    named_color!(DARK_RED, 139, 0, 0);
    named_color!(DARK_SALMON, 233, 150, 122);
    named_color!(DARK_SEA_GREEN, 143, 188, 143);
    named_color!(DARK_SLATE_BLUE, 72, 61, 139);
    named_color!(DARK_SLATE_GRAY, 47, 79, 79);
    named_color!(DARK_SLATE_GREY, 47, 79, 79);
    named_color!(DARK_TURQUOISE, 0, 206, 209);
    named_color!(DARK_VIOLET, 148, 0, 211);
    named_color!(DEEP_PINK, 255, 20, 147);
    named_color!(DEEP_SKY_BLUE, 0, 191, 255);
    named_color!(DIM_GRAY, 105, 105, 105);
    named_color!(DIM_GREY, 105, 105, 105);
    named_color!(DOGER_BLUE, 30, 144, 255);
    named_color!(FIRE_BRICK, 178, 34, 34);
    named_color!(FLORAL_WHITE, 255, 250, 240);
    named_color!(FOREST_GREEN, 34, 139, 34);
    named_color!(GAINSBORO, 220, 220, 220);
    named_color!(GHOST_WHITE, 248, 248, 255);
    named_color!(GOLD, 255, 215, 0);
    named_color!(GOLDENROD, 218, 165, 32);
    named_color!(GREEN_YELLOW, 173, 255, 47);
    named_color!(GREY, 128, 128, 128);
    named_color!(HONEYDEW, 240, 255, 240);
    named_color!(HOT_PINK, 255, 105, 180);
    named_color!(INDIAN_RED, 205, 92, 92);
    named_color!(INDIGO, 75, 0, 130);
    named_color!(IVORY, 255, 255, 240);
    named_color!(KHAKI, 240, 230, 140);
    named_color!(LAVENDER, 230, 230, 250);
    named_color!(LAVENDER_BLUSH, 255, 240, 245);
    named_color!(LAWN_GREEN, 124, 242, 0);
    named_color!(LEMON_CHIFFON, 255, 250, 205);
    named_color!(LIGHT_BLUE, 173, 216, 230);
    named_color!(LIGHT_CORAL, 240, 128, 128);
    named_color!(LIGHT_CYAN, 224, 255, 255);
    named_color!(LIGHT_GOLDENROD_YELLOW, 250, 250, 210);
    named_color!(LIGHT_GRAY, 211, 211, 211);
    named_color!(LIGHT_GREEN, 144, 238, 144);
    named_color!(LIGHT_GREY, 211, 211, 211);
    named_color!(LIGHT_PINK, 255, 182, 193);
    named_color!(LIGHT_SALMON, 255, 160, 122);
    named_color!(LIGHT_SEA_GREEN, 32, 178, 170);
    named_color!(LIGHT_SKY_BLUE, 135, 206, 250);
    named_color!(LIGHT_SLATE_GRAY, 119, 136, 153);
    named_color!(LIGHT_STEEL_BLUE, 176, 196, 222);
    named_color!(LIGHT_YELLOW, 255, 255, 224);
    named_color!(LIME_GREEN, 50, 205, 50);
    named_color!(LINEN, 250, 240, 230);
    named_color!(MAGENTA, 255, 0, 255);
    named_color!(MEDIUM_AQUAMARINE, 102, 205, 170);
    named_color!(MEDIUM_BLUE, 0, 0, 205);
    named_color!(MEDIUM_ORCHID, 186, 85, 211);
    named_color!(MEDIUM_PURPLE, 147, 112, 219);
    named_color!(MEDIUM_SEA_GREEN, 60, 179, 113);
    named_color!(MEDIUM_SLATE_BLUE, 123, 104, 238);
    named_color!(MEDIUM_SPRING_GREEN, 0, 250, 154);
    named_color!(MEDIUM_TURQUOISE, 72, 209, 204);
    named_color!(MEDIUM_VIOLET_RED, 199, 21, 133);
    named_color!(MIDNIGHT_BLUE, 25, 25, 112);
    named_color!(MINT_CREAM, 245, 255, 250);
    named_color!(MISTY_ROSE, 255, 228, 225);
    named_color!(MOCCASIN, 255, 228, 181);
    named_color!(NAVAJO_WHITE, 255, 222, 173);
    named_color!(OLD_LACE, 253, 245, 230);
    named_color!(OLIVE_DRAB, 107, 142, 35);
    named_color!(ORANGE, 255, 165, 0);
    named_color!(ORANGE_RED, 255, 69, 0);
    named_color!(ORCHID, 218, 112, 214);
    named_color!(PALE_GOLDEN_ROD, 238, 232, 170);
    named_color!(PALE_GREEN, 152, 251, 152);
    named_color!(PALE_TURQUOISE, 175, 238, 238);
    named_color!(PALE_VIOLET_RED, 219, 112, 147);
    named_color!(PAPAYAWHIP, 255, 239, 213);
    named_color!(PEACH_PUFF, 255, 218, 185);
    named_color!(PERU, 205, 133, 63);
    named_color!(PINK, 255, 192, 203);
    named_color!(PLUM, 221, 160, 221);
    named_color!(POWDER_BLUE, 176, 224, 230);
    named_color!(ROSY_BROWN, 188, 143, 143);
    named_color!(ROYAL_BLUE, 65, 105, 225);
    named_color!(SADDLE_BROWN, 139, 69, 19);
    named_color!(SALMON, 250, 128, 114);
    named_color!(SANDY_BROWN, 244, 164, 96);
    named_color!(SEA_GREEN, 46, 139, 87);
    named_color!(SEA_SHELL, 255, 245, 238);
    named_color!(SIENNA, 160, 82, 45);
    named_color!(SKY_BLUE, 135, 206, 235);
    named_color!(SLATE_BLUE, 106, 90, 205);
    named_color!(SLATE_GRAY, 112, 128, 144);
    named_color!(SLATE_GREY, 112, 128, 144);
    named_color!(SNOW, 255, 250, 250);
    named_color!(SPRING_GREEN, 0, 255, 127);
    named_color!(STEEL_BLUE, 70, 130, 180);
    named_color!(TAN, 210, 180, 140);
    named_color!(THISTLE, 216, 191, 216);
    named_color!(TOMATO, 255, 99, 71);
    named_color!(TURQUOISE, 64, 224, 208);
    named_color!(VIOLET, 238, 130, 238);
    named_color!(WHEAT, 245, 222, 179);
    named_color!(WHITE_SMOKE, 245, 245, 245);
    named_color!(YELLOW_GREEN, 154, 205, 50);
}

/// What type of user motion is the device capable of tracking? For the normal fully capable XR headset, this should be
/// 6dof (rotation and translation), but more limited headsets may be restricted to 3dof (rotation) and flatscreen
/// computers with the simulator off would be none.
/// <https://stereokit.net/Pages/StereoKit/DeviceTracking.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DeviceTracking {
    /// No tracking is available! This is likely a flatscreen application, not an XR applicaion.
    None = 0,
    /// This tracks rotation only, this may be a limited device without tracking cameras, or could be a more capable
    /// headset in a 3dof mode. DoF stands for Degrees of Freedom.
    Dof3 = 1,
    /// This is capable of tracking both the position and rotation of the device, most fully featured XR headsets
    /// (such as a HoloLens 2) will have this. DoF stands for Degrees of Freedom.
    Dof6 = 2,
}

/// This describes a type of display hardware!
/// <https://stereokit.net/Pages/StereoKit/DisplayType.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DisplayType {
    /// Not a display at all, or the variable hasn’t been initialized properly yet.
    None = 0,
    /// This is a stereo display! It has 2 screens, or two sections that display content in stereo, one for each eye.
    /// This could be a VR headset, or like a 3D tv.
    Stereo = 1,
    /// This is a single flat screen, with no stereo depth. This could be something like either a computer monitor, or a
    /// phone with passthrough AR.
    Flatscreen = 2,
}

/// <https://stereokit.net/Pages/StereoKit/FovInfo.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FovInfo {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// This class describes the device that is running this application! It contains identifiers, capabilities, and a few
/// adjustable settings here and there.
/// <https://stereokit.net/Pages/StereoKit/Device.html>
pub struct Device;
extern "C" {
    pub fn device_display_get_type() -> DisplayType;
    pub fn device_display_get_blend() -> DisplayBlend;
    pub fn device_display_set_blend(blend: DisplayBlend) -> Bool32T;
    pub fn device_display_valid_blend(blend: DisplayBlend) -> Bool32T;
    pub fn device_display_get_refresh_rate() -> f32;
    pub fn device_display_get_width() -> i32;
    pub fn device_display_get_height() -> i32;
    pub fn device_display_get_fov() -> FovInfo;
    pub fn device_get_tracking() -> DeviceTracking;
    pub fn device_get_name() -> *const c_char;
    pub fn device_get_runtime() -> *const c_char;
    pub fn device_get_gpu() -> *const c_char;
    pub fn device_has_eye_gaze() -> Bool32T;
    pub fn device_has_hand_tracking() -> Bool32T;
}

impl Device {
    /// Allows you to set and get the current blend mode of the device! Setting this may not succeed if the blend mode
    /// is not valid.
    /// <https://stereokit.net/Pages/StereoKit/Device/DisplayBlend.html>
    ///
    /// see also [`crate::util::device_display_set_blend`]
    pub fn display_blend(blend: DisplayBlend) -> bool {
        unsafe { device_display_set_blend(blend) != 0 }
    }

    /// Allows you to set and get the current blend mode of the device! Setting this may not succeed if the blend mode
    /// is not valid.
    /// <https://stereokit.net/Pages/StereoKit/Device/DisplayBlend.html>
    ///
    /// see also [`crate::util::device_display_get_blend`]
    pub fn get_display_blend() -> DisplayBlend {
        unsafe { device_display_get_blend() }
    }

    /// What type of display is this? Most XR headsets will report stereo, but the Simulator will report flatscreen.
    /// <https://stereokit.net/Pages/StereoKit/Device/DisplayType.html>
    ///
    /// see also [`crate::util::device_display_get_type`]
    pub fn get_display_type() -> DisplayType {
        unsafe { device_display_get_type() }
    }

    /// This is the name of the OpenXR runtime that powers the current device! This can help you determine which
    /// implementation quirks to expect based on the codebase used. On the simulator, this will be "Simulator", and in
    /// other non-XR modes this will be "None".
    /// <https://stereokit.net/Pages/StereoKit/Device/Runtime.html>
    ///
    /// see also [`crate::util::device_get_runtime`]
    pub fn get_runtime<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_runtime()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// The reported name of the GPU, this will differ between D3D and GL.
    /// <https://stereokit.net/Pages/StereoKit/Device/GPU.html>
    ///
    /// see also [`crate::util::device_get_gpu`]
    pub fn get_gpu<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_gpu()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// Does the device we’re on have eye tracking support present for input purposes? This is not an indicator that the
    /// user has given the application permission to access this information. See Input.Gaze for how to use this data.
    /// <https://stereokit.net/Pages/StereoKit/Device/HasEyeGaze.html>
    ///
    /// see also [`crate::util::device_has_eye_gaze`]
    pub fn has_eye_gaze() -> bool {
        unsafe { device_has_eye_gaze() != 0 }
    }

    /// Tells if the device is capable of tracking hands. This does not tell if the user is actually using their hands
    /// for input, merely that it’s possible to!
    /// <https://stereokit.net/Pages/StereoKit/Device/HasHandTracking.html>
    ///
    /// see also [`crate::util::device_has_hand_tracking`]
    pub fn has_hand_tracking() -> bool {
        unsafe { device_has_hand_tracking() != 0 }
    }

    /// This is the name of the active device! From OpenXR, this is the same as systemName from XrSystemProperties. The
    /// simulator will say “Simulator”.
    /// <https://stereokit.net/Pages/StereoKit/Device/Name.html>
    ///
    /// see also [`crate::util::device_get_gpu`]
    pub fn get_name<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_name()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// The tracking capabilities of this device! Is it 3DoF, rotation only? Or is it 6DoF, with positional tracking as
    /// well? Maybe it can’t track at all!
    /// <https://stereokit.net/Pages/StereoKit/Device/Tracking.html>
    ///
    /// see also [`crate::util::device_get_tracking`]
    pub fn get_tracking() -> DeviceTracking {
        unsafe { device_get_tracking() }
    }

    /// Tells if a particular blend mode is valid on this device. Some devices may be capable of more than one blend
    /// mode.
    /// <https://stereokit.net/Pages/StereoKit/Device/ValidBlend.html>
    ///
    /// see also [`crate::util::device_display_valid_blend`]
    pub fn valid_blend(blend: DisplayBlend) -> bool {
        unsafe { device_display_valid_blend(blend) != 0 }
    }
}

/// A color/position pair for Gradient values!
/// <https://stereokit.net/Pages/StereoKit/GradientKey.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct GradientKey {
    /// The color for this item, preferably in some form of linear color space. Gamma corrected colors will definitely
    /// not math correctly.
    pub color: Color128,
    /// Typically a value between 0-1! This is the position of the color along the ‘x-axis’ of the gradient.
    pub position: f32,
}

impl GradientKey {
    /// A basic copy constructor for GradientKey.
    /// <https://stereokit.net/Pages/StereoKit/GradientKey/GradientKey.html>
    pub fn new(color_linear: impl Into<Color128>, position: f32) -> Self {
        Self { color: color_linear.into(), position }
    }
}

/// A Gradient is a sparse collection of color keys that are
/// used to represent a ramp of colors! This class is largely just
/// storing colors and allowing you to sample between them.
///
/// Since the Gradient is just interpolating values, you can use whatever
/// color space you want here, as long as it's linear and not gamma!
/// Gamma space RGB can't math properly at all. It can be RGB(linear),
/// HSV, LAB, just remember which one you have, and be sure to convert it
/// appropriately later. Data is stored as float colors, so this'll be a
/// high accuracy blend!
/// <https://stereokit.net/Pages/StereoKit/Gradient.html>
pub struct Gradient(pub NonNull<_GradientT>);
impl Drop for Gradient {
    fn drop(&mut self) {
        unsafe { gradient_destroy(self.0.as_ptr()) };
    }
}
impl AsRef<Gradient> for Gradient {
    fn as_ref(&self) -> &Gradient {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _GradientT {
    _unused: [u8; 0],
}
pub type GradientT = *mut _GradientT;

extern "C" {
    pub fn gradient_create() -> GradientT;
    pub fn gradient_create_keys(in_arr_keys: *const GradientKey, count: i32) -> GradientT;
    pub fn gradient_add(gradient: GradientT, color_linear: Color128, position: f32);
    pub fn gradient_set(gradient: GradientT, index: i32, color_linear: Color128, position: f32);
    pub fn gradient_remove(gradient: GradientT, index: i32);
    pub fn gradient_count(gradient: GradientT) -> i32;
    pub fn gradient_get(gradient: GradientT, at: f32) -> Color128;
    pub fn gradient_get32(gradient: GradientT, at: f32) -> Color32;
    pub fn gradient_destroy(gradient: GradientT);
}
impl Gradient {
    /// Creates a new, completely empty gradient.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Gradient.html>
    ///
    /// see also [`crate::util::gradient_create`][`crate::util::gradient_create_keys`]
    pub fn new(keys: Option<&[GradientKey]>) -> Self {
        match keys {
            Some(keys) => {
                Gradient(NonNull::new(unsafe { gradient_create_keys(keys.as_ptr(), keys.len() as i32) }).unwrap())
            }
            None => Gradient(NonNull::new(unsafe { gradient_create() }).unwrap()),
        }
    }

    ///This adds a color key into the list. It’ll get inserted to the right slot based on its position.
    ///<https://stereokit.net/Pages/StereoKit/Gradient/Add.html>
    ///
    /// see also [`crate::util::gradient_add`]
    pub fn add(&mut self, color_linear: impl Into<Color128>, position: f32) -> &mut Self {
        unsafe { gradient_add(self.0.as_ptr(), color_linear.into(), position) };
        self
    }

    /// Updates the color key at the given index! This will NOT re-order color keys if they are moved past another
    /// key’s position, which could lead to strange behavior.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Set.html>
    ///
    /// see also [`crate::util::gradient_set`]
    pub fn set(&mut self, index: i32, color_linear: impl Into<Color128>, position: f32) -> &mut Self {
        unsafe { gradient_set(self.0.as_ptr(), index, color_linear.into(), position) };
        self
    }

    /// Removes the color key at the given index!
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Remove.html>
    ///
    /// see also [`crate::util::gradient_remove`]
    pub fn remove(&mut self, index: i32) -> &mut Self {
        unsafe { gradient_remove(self.0.as_ptr(), index) };
        self
    }

    /// Samples the gradient’s color at the given position!
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Remove.html>
    ///
    /// see also [`crate::util::gradient_get`]
    pub fn get(&self, at: f32) -> Color128 {
        unsafe { gradient_get(self.0.as_ptr(), at) }
    }

    /// Samples the gradient’s color at the given position, and converts it to a 32 bit color. If your RGBA color
    /// values are outside of the 0-1 range, then you’ll get some issues as they’re converted to 0-255 range bytes!
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Remove.html>
    ///
    /// see also [`crate::util::gradient_get32`]
    pub fn get32(&self, at: f32) -> Color32 {
        unsafe { gradient_get32(self.0.as_ptr(), at) }
    }
}

/// file extension to filter ie ".txt" ".gltf"
/// <https://stereokit.net/Pages/StereoKit/Platform.html>
///
/// see also [`crate::system::Assets::MODEL_FORMATS`] [`crate::system::Assets::TEXTURE_FORMATS`]
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct FileFilter {
    ext: [c_char; 32usize],
}

impl FileFilter {
    pub fn new(str: impl AsRef<str>) -> Self {
        let mut value: [c_char; 32usize] = [0; 32usize];
        let c_array = CString::new(str.as_ref()).unwrap();
        let c_array = c_array.as_bytes_with_nul();

        let mut c_array_iter = c_array.iter().map(|v| *v as c_char);
        value[0..c_array.len()].fill_with(|| c_array_iter.next().unwrap());

        Self { ext: value }
    }
}

/// When opening the Platform.FilePicker, this enum describes how the picker should look and behave.
/// <https://stereokit.net/Pages/StereoKit/PickerMode.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum PickerMode {
    /// Allow opening a single file.
    Open = 0,
    /// Allow the user to enter or select the name of the destination file.
    Save = 1,
}

/// This class provides some platform related code that runs cross-platform. You might be able to do many of these
/// things with rust or C#, but you might not be able to do them in as a portable manner as these methods do!
/// <https://stereokit.net/Pages/StereoKit/Platform.html>
pub struct Platform;

extern "C" {
    pub fn platform_file_picker(
        mode: PickerMode,
        callback_data: *mut c_void,
        picker_callback: ::std::option::Option<
            unsafe extern "C" fn(callback_data: *mut c_void, confirmed: Bool32T, filename: *const c_char),
        >,
        filters: *const FileFilter,
        filter_count: i32,
    );
    pub fn platform_file_picker_sz(
        mode: PickerMode,
        callback_data: *mut c_void,
        picker_callback_sz: ::std::option::Option<
            unsafe extern "C" fn(
                callback_data: *mut c_void,
                confirmed: Bool32T,
                filename_ptr: *const c_char,
                filename_length: i32,
            ),
        >,
        in_arr_filters: *const FileFilter,
        filter_count: i32,
    );
    pub fn platform_file_picker_close();
    pub fn platform_file_picker_visible() -> Bool32T;
    pub fn platform_read_file(
        filename_utf8: *const c_char,
        out_data: *mut *mut c_void,
        out_size: *mut usize,
    ) -> Bool32T;
    pub fn platform_write_file(filename_utf8: *const c_char, data: *mut c_void, size: usize) -> Bool32T;
    pub fn platform_write_file_text(filename_utf8: *const c_char, text_utf8: *const c_char) -> Bool32T;
    pub fn platform_keyboard_get_force_fallback() -> Bool32T;
    pub fn platform_keyboard_set_force_fallback(force_fallback: Bool32T);
    pub fn platform_keyboard_show(visible: Bool32T, type_: TextContext);
    pub fn platform_keyboard_visible() -> Bool32T;
    pub fn platform_keyboard_set_layout(
        type_: TextContext,
        keyboard_layout: *mut *mut c_char,
        layouts_num: i32,
    ) -> Bool32T;
}

/// File_picker trampoline
///
/// see also [`Plaform::file_picker`]
unsafe extern "C" fn fp_trampoline<FS: FnMut(&str), FC: FnMut()>(
    user_data: *mut c_void,
    confirmed: Bool32T,
    filename: *const c_char,
) {
    let data = &mut *(user_data as *mut (&mut FS, &mut FC));
    let (update, cancel) = data;
    if confirmed != 0 {
        let c_str = CStr::from_ptr(filename).to_str().unwrap();
        update(c_str)
    } else {
        cancel()
    }
}

/// File_picker_sz trampoline
///
/// see also [`Plaform::file_picker`]
unsafe extern "C" fn fp_sz_trampoline<F: FnMut(bool, &str)>(
    user_data: *mut c_void,
    confirmed: Bool32T,
    filename: *const c_char,
    filename_length: i32,
) {
    let closure = &mut *(user_data as *mut &mut F);
    if confirmed != 0 && filename_length > 0 {
        let c_str = CStr::from_ptr(filename).to_str().unwrap();
        closure(true, c_str)
    } else {
        let c_str = "";
        closure(false, c_str)
    }
}

impl Platform {
    /// Force the use of StereoKit’s built-in fallback keyboard instead of the system keyboard. This may be great for
    /// testing or look and feel matching, but the system keyboard should generally be preferred for accessibility
    /// reasons.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ForceFallbackKeyboard.html>
    ///
    ///  see also [`crate::util::platform_keyboard_set_force_fallback`]
    pub fn force_fallback_keyboard(force_fallback: bool) {
        unsafe { platform_keyboard_set_force_fallback(force_fallback as Bool32T) }
    }

    /// Starts a file picker window! This will create a native file picker window if one is available in the current
    /// setup, and if it is not, it’ll create a fallback filepicker build using StereoKit’s UI.
    ///
    /// Flatscreen apps will show traditional file pickers, and UWP has an OS provided file picker that works in MR. All
    /// others currently use the fallback system.
    ///
    /// A note for UWP apps, UWP generally does not have permission to access random files, unless the user has chosen
    /// them with the picker! This picker properly handles permissions for individual files on UWP, but may have issues
    /// with files that reference other files, such as .gltf files with external textures. See [`Platform::write_file`]
    /// and [`Platform.read_file`] for manually reading and writing files in a cross-platfom manner.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePicker.html>
    ///
    ///  see also [`platform_file_picker`]
    pub fn file_picker<FS: FnMut(&str), FC: FnMut()>(
        mode: PickerMode,
        mut on_select_file: FS,
        mut on_cancel: FC,
        filters: &[impl AsRef<str>],
    ) {
        let mut c_filters = Vec::new();
        for filter in filters {
            c_filters.push(FileFilter::new(filter));
        }

        let mut closure = (&mut on_select_file, &mut on_cancel);
        unsafe {
            platform_file_picker(
                mode,
                &mut closure as *mut _ as *mut c_void,
                Some(fp_trampoline::<FS, FC>),
                c_filters.as_slice().as_ptr(),
                c_filters.len() as i32,
            )
        }
    }

    /// Starts a file picker window! This will create a native file picker window if one is available in the current
    /// setup, and if it is not, it’ll create a fallback filepicker build using StereoKit’s UI.
    ///
    /// Flatscreen apps will show traditional file pickers, and UWP has an OS provided file picker that works in MR. All
    /// others currently use the fallback system. Some pickers will block the system and return right away, but others
    /// will stick around and let users continue to interact with the app.
    ///
    /// A note for UWP apps, UWP generally does not have permission to access random files, unless the user has chosen
    /// them with the picker! This picker properly handles permissions for individual files on UWP, but may have issues
    /// with files that reference other files, such as .gltf files with external textures. See [`Platform::write_file`]
    /// and [`Platform.read_file`] for manually reading and writing files in a cross-platfom manner.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePicker.html>
    /// * on_complete - This action will be called when the file picker has finished, either via a cancel event, or from
    /// a confirm event. First parameter is a bool, where true indicates the presence of a valid filename, and false
    /// indicates a failure or cancel event.
    ///
    ///  see also [`platform_file_picker_sz`]
    pub fn file_picker_sz<F: FnMut(bool, &str)>(mode: PickerMode, mut on_complete: F, filters: &[impl AsRef<str>]) {
        let mut c_filters = Vec::new();
        for filter in filters {
            c_filters.push(FileFilter::new(filter));
        }

        let mut closure = &mut on_complete;
        unsafe {
            platform_file_picker_sz(
                mode,
                &mut closure as *mut _ as *mut c_void,
                Some(fp_sz_trampoline::<F>),
                c_filters.as_slice().as_ptr(),
                c_filters.len() as i32,
            )
        }
    }

    /// If the picker is visible, this will close it and immediately trigger a cancel event for the active picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePickerClose.html>
    ///
    ///  see also [`crate::util::platform_file_picker_close`]
    pub fn file_picker_close() {
        unsafe { platform_file_picker_close() }
    }

    /// Request or hide a soft keyboard for the user to type on. StereoKit will surface OS provided soft keyboards where
    /// available, and use a fallback keyboard when not. On systems with physical keyboards, soft keyboards generally
    /// will not be shown if the user has interacted with their physical keyboard recently.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardShow.html>
    ///
    ///  see also [`crate::util::platform_keyboard_show`]
    pub fn keyboard_show(show: bool, input_type: TextContext) {
        unsafe { platform_keyboard_show(show as Bool32T, input_type) }
    }

    /// Replace the default keyboard type with a custom layout.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardSetLayout.html>
    ///
    ///  see also [`crate::util::platform_keyboard_set_layout`]
    pub fn keyboard_set_layout(type_key: TextContext, keyboard_layouts: &Vec<&str>) -> bool {
        let mut keyboard_layouts_c = vec![];
        for str in keyboard_layouts {
            let c_str = CString::new(*str).unwrap().into_raw();
            keyboard_layouts_c.push(c_str);
        }
        unsafe {
            platform_keyboard_set_layout(
                type_key,
                keyboard_layouts_c.as_mut_slice().as_mut_ptr(),
                keyboard_layouts_c.len() as i32,
            ) != 0
        }
    }

    /// Reads the entire contents of the file as a UTF-8 string, taking advantage of any permissions that may have been
    /// granted by Platform::file_picker(_sz?). Returns Err on failure.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ReadFileText.html>
    ///
    ///  see also [`crate::util::platform_read_file`]
    pub fn read_file_text<'a>(filename: impl AsRef<str>) -> Result<&'a str, StereoKitError> {
        let c_str = CString::new(filename.as_ref())?;
        let out_data = CString::new("H")?.into_raw() as *mut *mut c_void;
        let mut len = 0usize;
        let len_ptr: *mut usize = &mut len;
        if unsafe { platform_read_file(c_str.as_ptr(), out_data, len_ptr) != 0 } {
            unsafe { CStr::from_ptr(*out_data as *const c_char) }
                .to_str()
                .map_err(|e| StereoKitError::ReadFileError(e.to_string()))
        } else {
            Err(StereoKitError::ReadFileError(filename.as_ref().to_owned()))
        }
    }

    /// Reads the entire contents of the file as a byte array, taking advantage of any permissions that may have been
    /// granted by Platform.FilePicker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ReadFile.html>
    ///
    ///  see also [`crate::util::platform_read_file`]
    pub fn read_file<'a>(filename: impl AsRef<str>) -> Result<&'a [u8], StereoKitError> {
        let c_str = CString::new(filename.as_ref())?;
        let out_data = CString::new("H")?.into_raw() as *mut *mut c_void;
        let mut len = 0usize;
        let len_ptr: *mut usize = &mut len;
        if unsafe { platform_read_file(c_str.as_ptr(), out_data, len_ptr) != 0 } {
            Ok(unsafe { std::slice::from_raw_parts(*out_data as *const u8, len) })
        } else {
            Err(StereoKitError::ReadFileError(filename.as_ref().to_owned()))
        }
    }

    /// Writes a UTF-8 text file to the filesystem, taking advantage of any permissions that may have been granted by
    /// Platform::file_picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/WriteFile.html>
    ///
    ///  see also [`crate::util::platform_write_file_text`]
    pub fn write_file_text<S: AsRef<str>>(filename: S, text: S) -> Result<bool, StereoKitError> {
        let c_str = CString::new(filename.as_ref())?;
        let in_data = CString::new(text.as_ref())?.into_raw() as *const c_char;
        if unsafe { platform_write_file_text(c_str.as_ptr(), in_data) != 0 } {
            Ok(true)
        } else {
            Err(StereoKitError::ReadFileError(filename.as_ref().to_owned()))
        }
    }

    /// Writes an array of bytes to the filesystem, taking advantage of any permissions that may have been granted by
    /// Platform::file_picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/WriteFile.html>
    ///
    ///  see also [`crate::util::platform_write_file`]
    pub fn write_file<S: AsRef<str>>(filename: S, data: &[u8]) -> Result<bool, StereoKitError> {
        let c_str = CString::new(filename.as_ref())?;
        if unsafe { platform_write_file(c_str.as_ptr(), data.as_ptr() as *mut c_void, data.len()) != 0 } {
            Ok(true)
        } else {
            Err(StereoKitError::ReadFileError(filename.as_ref().to_owned()))
        }
    }

    /// This will check if the file picker interface is currently visible. Some pickers will never show this, as they
    /// block the application until the picker has completed.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePickerVisible.html>
    ///
    ///  see also [`crate::util::platform_file_picker_visible`]
    pub fn get_file_picker_visible() -> bool {
        unsafe { platform_file_picker_visible() != 0 }
    }

    /// Force the use of StereoKit’s built-in fallback keyboard instead of the system keyboard. This may be great for
    /// testing or look and feel matching, but the system keyboard should generally be preferred for accessibility
    /// reasons.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ForceFallbackKeyboard.html>
    ///
    ///  see also [`crate::util::platform_keyboard_get_force_fallback`]
    pub fn get_force_fallback_keyboard() -> bool {
        unsafe { platform_keyboard_get_force_fallback() != 0 }
    }

    /// Check if a soft keyboard is currently visible. This may be an OS provided keyboard or StereoKit’s fallback
    /// keyboard, but will not indicate the presence of a physical keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardVisible.html>
    ///
    ///  see also [`crate::util::platform_keyboard_visible`]
    pub fn get_keyboard_visible() -> bool {
        unsafe { platform_keyboard_visible() != 0 }
    }
}

/// A light source used for creating SphericalHarmonics data.
/// <https://stereokit.net/Pages/StereoKit/SHLight.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct ShLight {
    /// Direction to the light source.
    pub dir_to: Vec3,
    /// Color of the light in linear space! Values here can exceed 1.
    pub color: Color128,
}
impl ShLight {
    pub fn new(dir_to: impl Into<Vec3>, color: impl Into<Color128>) -> Self {
        Self { dir_to: dir_to.into(), color: color.into() }
    }
}
/// Spherical Harmonics are kinda like Fourier, but on a sphere. That doesn’t mean terribly much to me, and could be
/// wrong, but check out here for more details about how Spherical Harmonics work in this context!
///
/// However, the more prctical thing is, SH can be a function that describes a value over the surface of a sphere! This
/// is particularly useful for lighting, since you can basically store the lighting information for a space in this
/// value! This is often used for lightmap data, or a light probe grid, but StereoKit just uses a single SH for the
/// entire scene. It’s a gross oversimplification, but looks quite good, and is really fast! That’s extremely great
/// when you’re trying to hit 60fps, or even 144fps.
/// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics.html>
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct SphericalHarmonics {
    pub coefficients: [Vec3; 9usize],
}

extern "C" {
    pub fn sh_create(in_arr_lights: *const ShLight, light_count: i32) -> SphericalHarmonics;
    pub fn sh_brightness(ref_harmonics: *mut SphericalHarmonics, scale: f32);
    pub fn sh_add(ref_harmonics: *mut SphericalHarmonics, light_dir: Vec3, light_color: Vec3);
    pub fn sh_lookup(harmonics: *const SphericalHarmonics, normal: Vec3) -> Color128;
    pub fn sh_dominant_dir(harmonics: *const SphericalHarmonics) -> Vec3;
}
impl SphericalHarmonics {
    /// Creates a SphericalHarmonics approximation of the irradiance given from a set of directional lights!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/FromLights.html>
    ///
    ///  see also [`crate::util::SHLight`]
    pub fn from_lights(lights: &[ShLight]) -> Self {
        unsafe { sh_create(lights.as_ptr(), lights.len() as i32) }
    }

    /// Loading a previously saved coefficients array.
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/SphericalHarmonics.html>
    ///
    ///  see also [`crate::util::sh_create`]
    pub fn new(coefficients: [Vec3; 9]) -> Self {
        SphericalHarmonics { coefficients }
    }

    /// Adds a ‘directional light’ to the lighting approximation. This can be used to bake a multiple light setup, or accumulate light
    /// from a field of points.
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Add.html>
    ///
    ///  see also [`crate::util::sh_add`]
    pub fn add_light(&mut self, light_dir: Vec3, light_color: Vec3) -> &mut Self {
        unsafe { sh_add(self, light_dir, light_color) };
        self
    }

    /// Scales all the SphericalHarmonic’s coefficients! This behaves as if you’re modifying the brightness of the lighting this object represents.
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Brightness.html>
    ///
    ///  see also [`crate::util::sh_brightness`]
    pub fn brightness(&mut self, scale: f32) -> &mut Self {
        unsafe { sh_brightness(self, scale) };
        self
    }

    /// Look up the color information in a particular direction!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Sample.html>
    ///
    ///  see also [`crate::util::sh_brightness`]
    pub fn get_sample(&self, normal: impl Into<Vec3>) -> Color128 {
        unsafe { sh_lookup(self, normal.into()) }
    }

    /// Returns the dominant direction of the light represented by this spherical harmonics data. The direction value is normalized.
    /// You can get the color of the light in this direction by using the struct’s Sample method: light.Sample(-light.DominantLightDirection).
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/DominantLightDirection.html>
    ///
    ///  see also [`crate::util::sh_brightness`]
    pub fn get_dominent_light_direction(&self) -> Vec3 {
        unsafe { sh_dominant_dir(self) }
    }

    /// Converts the SphericalHarmonic into a vector of coefficients 9 long. Useful for storing calculated data!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/ToArray.html>
    ///
    ///  see also [`crate::util::sh_brightness`]
    pub fn to_array(&self) -> Vec<Vec3> {
        self.coefficients.to_vec()
    }
}

/// This class contains time information for the current session and frame!
/// <https://stereokit.net/Pages/StereoKit/Time.html>
pub struct Time;

extern "C" {
    // Deprecated: pub fn time_get_raw() -> f64;
    // Deprecated: pub fn time_getf_unscaled() -> f32;
    // Deprecated: pub fn time_get_unscaled() -> f64;
    // Deprecated: pub fn time_getf() -> f32;
    // Deprecated: pub fn time_get() -> f64;
    // Deprecated: pub fn time_elapsedf_unscaled() -> f32;
    // Deprecated: pub fn time_elapsed_unscaled() -> f64;
    // Deprecated: pub fn time_elapsedf() -> f32;
    // Deprecated: pub fn time_elapsed() -> f64;
    pub fn time_total_raw() -> f64;
    pub fn time_totalf_unscaled() -> f32;
    pub fn time_total_unscaled() -> f64;
    pub fn time_totalf() -> f32;
    pub fn time_total() -> f64;
    pub fn time_stepf_unscaled() -> f32;
    pub fn time_step_unscaled() -> f64;
    pub fn time_stepf() -> f32;
    pub fn time_step() -> f64;
    pub fn time_scale(scale: f64);
    pub fn time_set_time(total_seconds: f64, frame_elapsed_seconds: f64);
    pub fn time_frame() -> u64;
}

impl Time {
    /// Time is scaled by this value! Want time to pass slower? Set it to 0.5! Faster? Try 2!
    /// <https://stereokit.net/Pages/StereoKit/Time/Scale.html>
    ///
    ///  see also [`crate::util::time_scale`]
    pub fn scale(factor: f64) {
        unsafe { time_scale(factor) }
    }

    /// This allows you to override the application time! The application will progress from this time using the current
    /// timescale.
    /// <https://stereokit.net/Pages/StereoKit/Time/SetTime.html>
    ///
    ///  see also [`crate::util::time_set_time`]
    pub fn set_time(total_seconds: f64, frame_elapsed_seconds: f64) {
        unsafe { time_set_time(total_seconds, frame_elapsed_seconds) }
    }

    /// The number of frames/steps since the app started.
    /// <https://stereokit.net/Pages/StereoKit/Time/Frame.html>
    ///
    ///  see also [`crate::util::time_frame`]
    pub fn get_frame() -> u64 {
        unsafe { time_frame() }
    }

    /// How many seconds have elapsed since the last frame? 64 bit time precision, calculated at the start of the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Step.html>
    ///
    ///  see also [`crate::util::time_step`]
    pub fn get_step() -> f64 {
        unsafe { time_step() }
    }

    /// How many seconds have elapsed since the last frame? 32 bit time precision, calculated at the start of the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Stepf.html>
    ///
    ///  see also [`crate::util::time_stepf`]
    pub fn get_stepf() -> f32 {
        unsafe { time_stepf() }
    }

    /// How many seconds have elapsed since the last frame? 64 bit time precision, calculated at the start of the frame.
    /// This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/StepUnscaled.html>
    ///
    ///  see also [`crate::util::time_step_unscaled`]
    pub fn get_step_unscaled() -> f64 {
        unsafe { time_step_unscaled() }
    }

    /// How many seconds have elapsed since the last frame? 32 bit time precision, calculated at the start of the frame.
    /// This version is unaffected by the Time.Scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/StepUnscaledf.html>
    ///
    ///  see also [`crate::util::time_stepf_unscaled`]
    pub fn get_step_unscaledf() -> f32 {
        unsafe { time_stepf_unscaled() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 64 bit time precision, calculated at the start of
    /// the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Total.html>
    ///
    ///  see also [`crate::util::time_total`]
    pub fn get_total() -> f64 {
        unsafe { time_total() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 32 bit time precision, calculated at the start of
    /// the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Totalf.html>
    ///
    ///  see also [`crate::util::time_totalf`]
    pub fn get_totalf() -> f32 {
        unsafe { time_totalf() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 64 bit time precision, calculated at the start of
    /// the frame. This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/TotalUnscaled.html>
    ///
    ///  see also [`crate::util::time_total_unscaled`]
    pub fn get_total_unscaled() -> f64 {
        unsafe { time_total_unscaled() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 32 bit time precision, calculated at the start of
    /// the frame. This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/TotalUnscaledf.html>
    ///
    ///  see also [`crate::util::time_totalf_unscaled`]
    pub fn get_total_unscaledf() -> f32 {
        unsafe { time_totalf_unscaled() }
    }
}
