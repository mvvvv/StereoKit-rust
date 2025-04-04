use crate::{
    StereoKitError,
    maths::{Bool32T, Vec3, lerp},
    sk::DisplayBlend,
    system::TextContext,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    path::Path,
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
///
/// see also [`Color32`] [`named_colors`]
/// ### Examples
/// ```
/// use stereokit_rust::{util::{Color128, named_colors}, maths::Vec3};
///
/// // Cyan from different sources:
/// let color_cyan1: Color128  = named_colors::CYAN.into();
/// let color_cyan2: Color128  = [0.0, 1.0, 1.0, 1.0].into();
/// let color_cyan3 = Color128 {r: 0.0, g: 1.0, b: 1.0, a: 1.0};
/// let color_cyan4 = Color128::new (0.0, 1.0, 1.0, 1.0);
/// let color_cyan5 = Color128::rgba(0.0, 1.0, 1.0, 1.0);
/// let color_cyan6 = Color128::rgb (0.0, 1.0, 1.0 );
/// let color_cyan7 = Color128::hsv (0.5, 1.0, 1.0, 1.0);
/// let color_cyan8 = Color128::hsv_vec3 (Vec3::new(0.5, 1.0, 1.0), 1.0);
/// let color_cyan9 = Color128::lab (0.911, -0.493, 0.042, 1.0);
/// let color_cyanA = Color128::lab_vec3 (Vec3::new(0.911, -0.493, 0.042), 1.0);
/// let color_cyanB = Color128::hex (0x00FFFFFF);
///
/// # assert_eq!(color_cyan1, color_cyan2);
/// # assert_eq!(color_cyan1, color_cyan3);
/// # assert_eq!(color_cyan1, color_cyan4);
/// # assert_eq!(color_cyan1, color_cyan5);
/// # assert_eq!(color_cyan1, color_cyan6);
/// # assert_eq!(color_cyan1, color_cyan7);
/// # assert_eq!(color_cyan1, color_cyan8);
/// # assert_eq!(color_cyan1, color_cyan9);
/// # assert_eq!(color_cyan1, color_cyanA);
/// assert_eq!(color_cyan1, color_cyanB);
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Color128 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl PartialEq for Color128 {
    fn eq(&self, other: &Self) -> bool {
        (self.r - other.r).abs() < 0.000001
            && (self.g - other.g).abs() < 0.000001
            && (self.b - other.b).abs() < 0.000001
            && (self.a - other.a).abs() < 0.000001
    }
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

unsafe extern "C" {
    pub fn color_hsv(hue: f32, saturation: f32, value: f32, transparency: f32) -> Color128;
    pub fn color_to_hsv(color: *const Color128) -> Vec3;
    pub fn color_lab(l: f32, a: f32, b: f32, transparency: f32) -> Color128;
    pub fn color_to_lab(color: *const Color128) -> Vec3;
    pub fn color_to_linear(srgb_gamma_correct: Color128) -> Color128;
    pub fn color_to_gamma(srgb_linear: Color128) -> Color128;
}

impl Color128 {
    /// Opaque black color
    pub const BLACK: Color128 = Color128 { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    /// Transparent black color
    pub const BLACK_TRANSPARENT: Color128 = Color128 { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// Opaque white color
    pub const WHITE: Color128 = Color128 { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1.
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1.
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1. Alpha will be set to 1.0
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    #[deprecated = "Use Color128::rgb instead"]
    pub const fn new_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Try hsv instead! But if you really need to create a color from RGB values, I suppose you’re in the right
    /// place. All parameter values are generally in the range of 0-1. Alpha will be set to 1.0
    /// <https://stereokit.net/Pages/StereoKit/Color/Color.html>
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a Red/Green/Blue gamma space color from Hue/Saturation/Value information.
    /// <https://stereokit.net/Pages/StereoKit/Color/HSV.html>
    pub fn hsv(hue: f32, saturation: f32, value: f32, transparency: f32) -> Color128 {
        unsafe { color_hsv(hue, saturation, value, transparency) }
    }

    /// Creates a Red/Green/Blue gamma space color from Hue/Saturation/Value information.
    /// <https://stereokit.net/Pages/StereoKit/Color/HSV.html>
    ///
    /// see also [`color_hsv`]
    pub fn hsv_vec3(vec: impl Into<Vec3>, transparency: f32) -> Self {
        let vec = vec.into();
        unsafe { color_hsv(vec.x, vec.y, vec.z, transparency) }
    }

    /// Creates a gamma space RGB color from a CIE-Lab color space. CIE-Lab is a color space that models human
    /// perception, and has significantly more accurate to perception lightness values, so this is an excellent color
    /// space for color operations that wish to preserve color brightness properly.
    ///
    /// Traditionally, values are L `[0,100], a,b [-200,+200]` but here we normalize them all to the 0-1 range. If you
    /// hate it, let me know why!
    /// <https://stereokit.net/Pages/StereoKit/Color/LAB.html>    
    ///
    /// see also [`color_lab`]
    pub fn lab(l: f32, a: f32, b: f32, transparency: f32) -> Self {
        unsafe { color_lab(l, a, b, transparency) }
    }

    /// Creates a gamma space RGB color from a CIE-Lab color space. CIE-Lab is a color space that models human
    /// perception, and has significantly more accurate to perception lightness values, so this is an excellent color
    /// space for color operations that wish to preserve color brightness properly.
    ///
    /// Traditionally, values are L `[0,100], a,b [-200,+200]` but here we normalize them all to the 0-1 range. If you
    /// hate it, let me know why!
    /// <https://stereokit.net/Pages/StereoKit/Color/LAB.html>    
    ///
    /// see also [`color_lab`]
    pub fn lab_vec3(vec: Vec3, transparency: f32) -> Self {
        unsafe { color_lab(vec.x, vec.y, vec.z, transparency) }
    }

    /// Create a color from an integer based hex value! This can make it easier to copy over colors from the web. This
    /// isn’t a string function though, so you’ll need to fill it out the whole way. Ex: Color32::Hex(0x0000FFFF) would
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
    /// * `a` - The first color, this will be the result if t is 0.
    /// * `b` - The second color, this will be the result if t is 1.
    /// * `t` - A percentage representing the blend between a and b. This is not clamped to the 0-1 range, and will
    ///   result in extrapolation outside this range.
    ///
    /// see also [`crate::maths::lerp`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{util::{Color128, named_colors}};
    ///
    /// let color_red: Color128  = named_colors::RED.into();
    /// let color_blue: Color128  = named_colors::BLUE.into();
    /// let color_mix = Color128::lerp(color_red, color_blue, 0.25);
    /// assert_eq!(color_mix, Color128::rgb(0.75, 0.0, 0.25));
    /// ```
    pub fn lerp(a: Color128, b: Color128, t: f32) -> Self {
        Self { r: lerp(a.r, b.r, t), g: lerp(a.g, b.g, t), b: lerp(a.b, b.b, t), a: lerp(a.a, b.a, t) }
    }

    /// Converts this from a gamma space color, into a linear space color! If this is not a gamma space color, this
    /// will just make your color wacky!
    /// <https://stereokit.net/Pages/StereoKit/Color/ToLinear.html>   
    ///
    /// see also [`color_to_linear`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{util::{Color128, named_colors}};
    ///
    /// let color = Color128::rgb(0.75, 0.0, 0.25);
    /// let color_linear = color.to_linear();
    /// assert_eq!(color_linear, Color128 { r: 0.5310492, g: 0.0, b: 0.04736614, a: 1.0 });
    /// ```
    pub fn to_linear(&self) -> Self {
        unsafe { color_to_linear(*self) }
    }

    /// Converts this from a linear space color, into a gamma space color! If this is not a linear space color, this
    /// will just make your color wacky!
    /// <https://stereokit.net/Pages/StereoKit/Color/ToGamma.html>
    ///    
    /// see also [`color_to_gamma`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{util::{Color128, named_colors}};
    ///
    /// let color = Color128 { r: 0.5310492, g: 0.0, b: 0.04736614, a: 1.0 };
    /// let color_gamma = color.to_gamma();
    /// assert_eq!(color_gamma, Color128::rgb(0.75, 0.0, 0.25));
    /// ```
    pub fn to_gamma(&self) -> Self {
        unsafe { color_to_gamma(*self) }
    }

    /// Converts the gamma space color to a Hue/Saturation/Value format! Does not consider transparency when
    /// calculating the result.
    /// <https://stereokit.net/Pages/StereoKit/Color/ToHSV.html>   
    ///
    /// Returns a Vec3 containing Hue, Saturation, and Value, stored in x, y, and z respectively. All values are
    /// between 0-1.
    /// see also [`color_to_hsv`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{util::{Color128, named_colors}, maths::Vec3};
    ///
    /// let color = Color128::rgb(0.75, 0.0, 0.25);
    /// let color_hsv = color.to_hsv();
    /// assert_eq!(color_hsv, Vec3::new(0.9444444, 1.0, 0.75));
    /// ```
    pub fn to_hsv(&self) -> Vec3 {
        unsafe { color_to_hsv(self) }
    }

    /// Converts the gamma space RGB color to a CIE LAB color space value! Conversion back and forth from LAB space
    /// could be somewhat lossy.
    /// <https://stereokit.net/Pages/StereoKit/Color/ToLAB.html>
    ///
    /// Returns a LAB Vec3 where x=L, y=A, z=B.    
    /// see also [`color_to_lab`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{util::{Color128, named_colors}, maths::Vec3};
    ///
    /// let color = Color128::rgb(0.75, 0.0, 0.25);
    /// let color_lab = color.to_lab();
    /// assert_eq!(color_lab, Vec3{ x: 0.403711, y: 0.6654343, z: 0.55437124 });
    /// ```
    pub fn to_lab(&self) -> Vec3 {
        unsafe { color_to_lab(self) }
    }
}

impl Display for Color128 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the color in debug mode. Looks like
    /// “Color128 {r: 1.0, g: 1.0, b: 1.0, a: 1.0}”
    /// <https://stereokit.net/Pages/StereoKit/Color/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r:{}, g:{}, b:{}, a:{}", self.r, self.g, self.b, self.a)
    }
}

/// This will add a color component-wise with another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Addition.html>
impl Add for Color128 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Color128 { r: self.r + rhs.r, g: self.g + rhs.g, b: self.b + rhs.b, a: self.a + rhs.a }
    }
}

/// This will add a color component-wise with another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Addition.html>
impl AddAssign for Color128 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }
}

/// This will subtract color b component-wise from color a, including alpha. Best done on colors in linear space. No
/// clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Subtraction.html>
impl Sub for Color128 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Color128 { r: self.r - rhs.r, g: self.g - rhs.g, b: self.b - rhs.b, a: self.a - rhs.a }
    }
}

/// This will subtract a color component-wise from another color, including alpha. Best done on colors in linear space.
/// No clamping is applied.
/// <https://stereokit.net/Pages/StereoKit/Color/op_Subtraction.html>
impl SubAssign for Color128 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.r -= rhs.r;
        self.g -= rhs.g;
        self.b -= rhs.b;
        self.a -= rhs.a;
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
/// see also [`Color128`]
/// ### Examples
/// ```
/// use stereokit_rust::util::{Color32, named_colors, Color128};
///
/// // Cyan from different sources:
/// let color_cyan1 = named_colors::CYAN;
/// let color_cyan2: Color32  = [0, 255, 255, 255].into();
/// let color_cyan3: Color32 = Color128 { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }.into();
/// let color_cyan4 = Color32 {r: 0, g: 255, b: 255, a: 255};
/// let color_cyan5 = Color32::new (0, 255, 255, 255);
/// let color_cyan6 = Color32::rgba(0, 255, 255, 255);
/// let color_cyan7 = Color32::rgb (0, 255, 255 );
/// let color_cyan8 = Color32::hex (0x00FFFFFF);
///
/// # assert_eq!(color_cyan1, color_cyan2);
/// # assert_eq!(color_cyan1, color_cyan3);
/// # assert_eq!(color_cyan1, color_cyan4);
/// # assert_eq!(color_cyan1, color_cyan5);
/// # assert_eq!(color_cyan1, color_cyan6);
/// # assert_eq!(color_cyan1, color_cyan7);
/// assert_eq!(color_cyan1, color_cyan8);
/// ```
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

impl Color32 {
    /// Opaque black color.
    pub const BLACK: Color32 = Color32 { r: 0, g: 0, b: 0, a: 255 };

    /// transparent black color.
    pub const BLACK_TRANSPARENT: Color32 = Color32 { r: 0, g: 0, b: 0, a: 0 };

    /// Opaque white color.
    pub const WHITE: Color32 = Color32 { r: 255, g: 255, b: 255, a: 255 };

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32::hex.
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32::hex.
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32::hex.
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Constructs a 32-bit color from bytes! You may also be interested in Color32::hex.
    /// a is set to 255 !
    /// <https://stereokit.net/Pages/StereoKit/Color32/Color32.html>
    #[deprecated = "Use Color32::rgb instead"]
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
    /// “Color32{r: 255, g: 255, b: 255, a: 255}”
    /// <https://stereokit.net/Pages/StereoKit/Color/ToString.html>  
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r:{}, g:{}, b:{}, a:{}", self.r, self.g, self.b, self.a)
    }
}

/// Named colors <https://www.w3.org/wiki/CSS/Properties/color/keywords>
pub mod named_colors {
    use super::Color32;

    // Basic :
    /// Black color rgb(0, 0, 0)
    pub const BLACK: Color32 = Color32::rgb(0, 0, 0);
    /// silver color rgb(192, 192, 192)
    pub const SILVER: Color32 = Color32::rgb(192, 192, 192);
    /// gray color rgb(128, 128, 128)
    pub const GRAY: Color32 = Color32::rgb(128, 128, 128);
    /// white color rgb(255, 255, 255)
    pub const WHITE: Color32 = Color32::rgb(255, 255, 255);
    /// maroon color rgb(128, 0, 0)
    pub const MAROON: Color32 = Color32::rgb(128, 0, 0);
    /// red color rgb(255, 0, 0)
    pub const RED: Color32 = Color32::rgb(255, 0, 0);
    /// purple color rgb(128, 0, 128)
    pub const PURPLE: Color32 = Color32::rgb(128, 0, 128);
    /// fuchsia color rgb(255, 0, 255)
    pub const FUCHSIA: Color32 = Color32::rgb(255, 0, 255);
    /// green color rgb(0, 128, 0)
    pub const GREEN: Color32 = Color32::rgb(0, 128, 0);
    /// lime color rgb(0, 255, 0)
    pub const LIME: Color32 = Color32::rgb(0, 255, 0);
    /// olive color rgb(128, 128, 0)
    pub const OLIVE: Color32 = Color32::rgb(128, 128, 0);
    /// yellow color rgb(255, 255, 0)
    pub const YELLOW: Color32 = Color32::rgb(255, 255, 0);
    /// navy color rgb(0, 0, 128)
    pub const NAVY: Color32 = Color32::rgb(0, 0, 128);
    /// blue color rgb(0, 0, 255)
    pub const BLUE: Color32 = Color32::rgb(0, 0, 255);
    /// teal color rgb(0, 128, 128)
    pub const TEAL: Color32 = Color32::rgb(0, 128, 128);
    /// aqua color rgb(0, 255, 255)
    pub const AQUA: Color32 = Color32::rgb(0, 255, 255);

    //Extended
    /// alice blue color rgb(240, 248, 255)
    pub const ALICE_BLUE: Color32 = Color32::rgb(240, 248, 255);
    /// antique white color rgb(250, 235, 215)
    pub const ANTIQUE_WHITE: Color32 = Color32::rgb(250, 235, 215);
    /// aquamarine color rgb(127, 255, 212)
    pub const AQUAMARINE: Color32 = Color32::rgb(127, 255, 212);
    /// azure color rgb(240, 255, 255)
    pub const AZURE: Color32 = Color32::rgb(240, 255, 255);
    /// beige color rgb(255, 228, 196)
    pub const BEIGE: Color32 = Color32::rgb(255, 228, 196);
    /// bisque color rgb(255, 228, 196)
    pub const BISQUE: Color32 = Color32::rgb(255, 228, 196);
    /// blanched almond color rgb(255, 235, 205)
    pub const BLANCHED_ALMOND: Color32 = Color32::rgb(255, 235, 205);
    /// blue violet color rgb(138, 43, 226)
    pub const BLUE_VIOLET: Color32 = Color32::rgb(138, 43, 226);
    /// brown color rgb(165, 42, 42)
    pub const BROWN: Color32 = Color32::rgb(165, 42, 42);
    /// burlywood color rgb(222, 184, 135)
    pub const BURLY_WOOD: Color32 = Color32::rgb(222, 184, 135);
    /// cadet blue color rgb(95, 158, 160)
    pub const CADET_BLUE: Color32 = Color32::rgb(95, 158, 160);
    /// chartreuse color rgb(127, 255, 0)
    pub const CHARTREUSE: Color32 = Color32::rgb(127, 255, 0);
    /// chocolate color rgb(210, 105, 30)
    pub const CHOCOLATE: Color32 = Color32::rgb(210, 105, 30);
    /// coral color rgb(255, 127, 80)
    pub const CORAL: Color32 = Color32::rgb(255, 127, 80);
    /// cornflower blue color rgb(100, 149, 237)
    pub const CORNFLOWER_BLUE: Color32 = Color32::rgb(100, 149, 237);
    /// cornsilk color rgb(255, 248, 220)
    pub const CORN_SILK: Color32 = Color32::rgb(255, 248, 220);
    /// crimson color rgb(220, 20, 60)
    pub const CRIMSON: Color32 = Color32::rgb(220, 20, 60);
    /// cyan color rgb(0, 255, 255)
    pub const CYAN: Color32 = Color32::rgb(0, 255, 255);
    /// dark blue color rgb(0, 0, 139)
    pub const DARK_BLUE: Color32 = Color32::rgb(0, 0, 139);
    /// dark cyan color rgb(0, 139, 139)
    pub const DARK_CYAN: Color32 = Color32::rgb(0, 139, 139);
    /// dark goldenrod color rgb(184, 134, 11)
    pub const DARK_GOLDEN_ROD: Color32 = Color32::rgb(184, 134, 11);
    /// dark gray color rgb(169, 169, 169)
    pub const DARK_GRAY: Color32 = Color32::rgb(169, 169, 169);
    /// dark green color rgb(0, 100, 0)
    pub const DARK_GREEN: Color32 = Color32::rgb(0, 100, 0);
    /// dark grey color rgb(169, 169, 169)
    pub const DARK_GREY: Color32 = Color32::rgb(169, 169, 169);
    /// dark khaki color rgb(189, 183, 107)
    pub const DARK_KHAKI: Color32 = Color32::rgb(189, 183, 107);
    /// dark magenta color rgb(139, 0, 139)
    pub const DARK_MAGENTA: Color32 = Color32::rgb(139, 0, 139);
    /// dark olive green color rgb(85, 107, 47)
    pub const DARK_OLIVE_GREEN: Color32 = Color32::rgb(85, 107, 47);
    /// dark orange color rgb(255, 140, 0)
    pub const DARK_ORANGE: Color32 = Color32::rgb(255, 140, 0);
    /// dark orchid color rgb(153, 50, 204)
    pub const DARK_ORCHID: Color32 = Color32::rgb(153, 50, 204);
    /// dark red color rgb(139, 0, 0)
    pub const DARK_RED: Color32 = Color32::rgb(139, 0, 0);
    /// dark salmon color rgb(233, 150, 122)
    pub const DARK_SALMON: Color32 = Color32::rgb(233, 150, 122);
    /// dark sea green color rgb(143, 188, 143)
    pub const DARK_SEA_GREEN: Color32 = Color32::rgb(143, 188, 143);
    /// dark slate blue color rgb(72, 61, 139)
    pub const DARK_SLATE_BLUE: Color32 = Color32::rgb(72, 61, 139);
    /// dark slate gray color rgb(47, 79, 79)
    pub const DARK_SLATE_GRAY: Color32 = Color32::rgb(47, 79, 79);
    /// dark slate grey color rgb(47, 79, 79)
    pub const DARK_SLATE_GREY: Color32 = Color32::rgb(47, 79, 79);
    /// dark turquoise color rgb(0, 206, 209)
    pub const DARK_TURQUOISE: Color32 = Color32::rgb(0, 206, 209);
    /// dark violet color rgb(148, 0, 211)
    pub const DARK_VIOLET: Color32 = Color32::rgb(148, 0, 211);
    /// deep pink color rgb(255, 20, 147)
    pub const DEEP_PINK: Color32 = Color32::rgb(255, 20, 147);
    /// deep sky blue color rgb(0, 191, 255)
    pub const DEEP_SKY_BLUE: Color32 = Color32::rgb(0, 191, 255);
    /// dim gray color rgb(105, 105, 105)
    pub const DIM_GRAY: Color32 = Color32::rgb(105, 105, 105);
    /// dim grey color rgb(105, 105, 105)
    pub const DIM_GREY: Color32 = Color32::rgb(105, 105, 105);
    /// dodger blue color rgb(30, 144, 255)
    pub const DOGER_BLUE: Color32 = Color32::rgb(30, 144, 255);
    /// fire brick color rgb(178, 34, 34)
    pub const FIRE_BRICK: Color32 = Color32::rgb(178, 34, 34);
    /// floral white color rgb(255, 250, 240)
    pub const FLORAL_WHITE: Color32 = Color32::rgb(255, 250, 240);
    /// forest green color rgb(34, 139, 34)
    pub const FOREST_GREEN: Color32 = Color32::rgb(34, 139, 34);
    /// gainsboro color rgb(220, 220, 220)
    pub const GAINSBORO: Color32 = Color32::rgb(220, 220, 220);
    /// ghost white color rgb(248, 248, 255)
    pub const GHOST_WHITE: Color32 = Color32::rgb(248, 248, 255);
    /// gold color rgb(255, 215, 0)
    pub const GOLD: Color32 = Color32::rgb(255, 215, 0);
    /// goldenrod color rgb(218, 165, 32)
    pub const GOLDENROD: Color32 = Color32::rgb(218, 165, 32);
    /// green yellow color rgb(173, 255, 47)
    pub const GREEN_YELLOW: Color32 = Color32::rgb(173, 255, 47);
    /// grey color rgb(128, 128, 128)
    pub const GREY: Color32 = Color32::rgb(128, 128, 128);
    /// honeydew color rgb(240, 255, 240)
    pub const HONEYDEW: Color32 = Color32::rgb(240, 255, 240);
    /// hot pink color rgb(255, 105, 180)
    pub const HOT_PINK: Color32 = Color32::rgb(255, 105, 180);
    /// indian red color rgb(205, 92, 92)
    pub const INDIAN_RED: Color32 = Color32::rgb(205, 92, 92);
    /// indigo color rgb(75, 0, 130)
    pub const INDIGO: Color32 = Color32::rgb(75, 0, 130);
    /// ivory color rgb(255, 255, 240)
    pub const IVORY: Color32 = Color32::rgb(255, 255, 240);
    /// khaki color rgb(240, 230, 140)
    pub const KHAKI: Color32 = Color32::rgb(240, 230, 140);
    /// lavender color rgb(230, 230, 250)
    pub const LAVENDER: Color32 = Color32::rgb(230, 230, 250);
    /// lavender blush color rgb(255, 240, 245)
    pub const LAVENDER_BLUSH: Color32 = Color32::rgb(255, 240, 245);
    /// lawn green color rgb(124, 242, 0)
    pub const LAWN_GREEN: Color32 = Color32::rgb(124, 242, 0);
    /// lemon chiffon color rgb(255, 250, 205)
    pub const LEMON_CHIFFON: Color32 = Color32::rgb(255, 250, 205);
    /// light blue color rgb(173, 216, 230)
    pub const LIGHT_BLUE: Color32 = Color32::rgb(173, 216, 230);
    /// light coral color rgb(240, 128, 128)
    pub const LIGHT_CORAL: Color32 = Color32::rgb(240, 128, 128);
    /// light cyan color rgb(224, 255, 255)
    pub const LIGHT_CYAN: Color32 = Color32::rgb(224, 255, 255);
    /// light goldenrod yellow color rgb(250, 250, 210)
    pub const LIGHT_GOLDENROD_YELLOW: Color32 = Color32::rgb(250, 250, 210);
    /// light gray color rgb(211, 211, 211)
    pub const LIGHT_GRAY: Color32 = Color32::rgb(211, 211, 211);
    /// light green color rgb(144, 238, 144)
    pub const LIGHT_GREEN: Color32 = Color32::rgb(144, 238, 144);
    /// light grey color rgb(211, 211, 211)
    pub const LIGHT_GREY: Color32 = Color32::rgb(211, 211, 211);
    /// light pink color rgb(255, 182, 193)
    pub const LIGHT_PINK: Color32 = Color32::rgb(255, 182, 193);
    /// light salmon color rgb(255, 160, 122)
    pub const LIGHT_SALMON: Color32 = Color32::rgb(255, 160, 122);
    /// light sea green color rgb(32, 178, 170)
    pub const LIGHT_SEA_GREEN: Color32 = Color32::rgb(32, 178, 170);
    /// light sky blue color rgb(135, 206, 250)
    pub const LIGHT_SKY_BLUE: Color32 = Color32::rgb(135, 206, 250);
    /// light slate gray color rgb(119, 136, 153)
    pub const LIGHT_SLATE_GRAY: Color32 = Color32::rgb(119, 136, 153);
    /// light steel blue color rgb(176, 196, 222)
    pub const LIGHT_STEEL_BLUE: Color32 = Color32::rgb(176, 196, 222);
    /// light yellow color rgb(255, 255, 224)
    pub const LIGHT_YELLOW: Color32 = Color32::rgb(255, 255, 224);
    /// lime green color rgb(50, 205, 50)
    pub const LIME_GREEN: Color32 = Color32::rgb(50, 205, 50);
    /// linen color rgb(250, 240, 230)
    pub const LINEN: Color32 = Color32::rgb(250, 240, 230);
    /// magenta color rgb(255, 0, 255)
    pub const MAGENTA: Color32 = Color32::rgb(255, 0, 255);
    /// medium aquamarine color rgb(102, 205, 170)
    pub const MEDIUM_AQUAMARINE: Color32 = Color32::rgb(102, 205, 170);
    /// medium blue color rgb(0, 0, 205)
    pub const MEDIUM_BLUE: Color32 = Color32::rgb(0, 0, 205);
    /// medium orchid color rgb(186, 85, 211)
    pub const MEDIUM_ORCHID: Color32 = Color32::rgb(186, 85, 211);
    /// medium purple color rgb(147, 112, 219)
    pub const MEDIUM_PURPLE: Color32 = Color32::rgb(147, 112, 219);
    /// medium sea green color rgb(60, 179, 113)
    pub const MEDIUM_SEA_GREEN: Color32 = Color32::rgb(60, 179, 113);
    /// medium slate blue color rgb(123, 104, 238)
    pub const MEDIUM_SLATE_BLUE: Color32 = Color32::rgb(123, 104, 238);
    /// medium spring green color rgb(0, 250, 154)
    pub const MEDIUM_SPRING_GREEN: Color32 = Color32::rgb(0, 250, 154);
    /// medium turquoise color rgb(72, 209, 204)
    pub const MEDIUM_TURQUOISE: Color32 = Color32::rgb(72, 209, 204);
    /// medium violet red color rgb(199, 21, 133)
    pub const MEDIUM_VIOLET_RED: Color32 = Color32::rgb(199, 21, 133);
    /// midnight blue color rgb(25, 25, 112)
    pub const MIDNIGHT_BLUE: Color32 = Color32::rgb(25, 25, 112);
    /// mint cream color rgb(245, 255, 250)
    pub const MINT_CREAM: Color32 = Color32::rgb(245, 255, 250);
    /// misty rose color rgb(255, 228, 225)
    pub const MISTY_ROSE: Color32 = Color32::rgb(255, 228, 225);
    /// moccasin color rgb(255, 228, 181)
    pub const MOCCASIN: Color32 = Color32::rgb(255, 228, 181);
    /// navajo white color rgb(255, 222, 173)
    pub const NAVAJO_WHITE: Color32 = Color32::rgb(255, 222, 173);
    /// old lace color rgb(253, 245, 230)
    pub const OLD_LACE: Color32 = Color32::rgb(253, 245, 230);
    /// olive drab color rgb(107, 142, 35)
    pub const OLIVE_DRAB: Color32 = Color32::rgb(107, 142, 35);
    /// orange color rgb(255, 165, 0)
    pub const ORANGE: Color32 = Color32::rgb(255, 165, 0);
    /// orange red color rgb(255, 69, 0)
    pub const ORANGE_RED: Color32 = Color32::rgb(255, 69, 0);
    /// orchid color rgb(218, 112, 214)
    pub const ORCHID: Color32 = Color32::rgb(218, 112, 214);
    /// pale golden rod color rgb(238, 232, 170)
    pub const PALE_GOLDEN_ROD: Color32 = Color32::rgb(238, 232, 170);
    /// pale green color rgb(152, 251, 152)
    pub const PALE_GREEN: Color32 = Color32::rgb(152, 251, 152);
    /// pale turquoise color rgb(175, 238, 238)
    pub const PALE_TURQUOISE: Color32 = Color32::rgb(175, 238, 238);
    /// pale violet red color rgb(219, 112, 147)
    pub const PALE_VIOLET_RED: Color32 = Color32::rgb(219, 112, 147);
    /// papaya whip color rgb(255, 239, 213)
    pub const PAPAYAWHIP: Color32 = Color32::rgb(255, 239, 213);
    /// peach puff color rgb(255, 218, 185)
    pub const PEACH_PUFF: Color32 = Color32::rgb(255, 218, 185);
    /// peru color rgb(205, 133, 63)
    pub const PERU: Color32 = Color32::rgb(205, 133, 63);
    /// pink color rgb(255, 192, 203)
    pub const PINK: Color32 = Color32::rgb(255, 192, 203);
    /// plum color rgb(221, 160, 221)
    pub const PLUM: Color32 = Color32::rgb(221, 160, 221);
    /// powder blue color rgb(176, 224, 230)
    pub const POWDER_BLUE: Color32 = Color32::rgb(176, 224, 230);
    /// rosy brown color rgb(188, 143, 143)
    pub const ROSY_BROWN: Color32 = Color32::rgb(188, 143, 143);
    /// royal blue color rgb(65, 105, 225)
    pub const ROYAL_BLUE: Color32 = Color32::rgb(65, 105, 225);
    /// saddle brown color rgb(139, 69, 19)
    pub const SADDLE_BROWN: Color32 = Color32::rgb(139, 69, 19);
    /// salmon color rgb(250, 128, 114)
    pub const SALMON: Color32 = Color32::rgb(250, 128, 114);
    /// sandy brown color rgb(244, 164, 96)
    pub const SANDY_BROWN: Color32 = Color32::rgb(244, 164, 96);
    /// sea green color rgb(46, 139, 87)
    pub const SEA_GREEN: Color32 = Color32::rgb(46, 139, 87);
    /// sea shell color rgb(255, 245, 238)
    pub const SEA_SHELL: Color32 = Color32::rgb(255, 245, 238);
    /// sienna color rgb(160, 82, 45)
    pub const SIENNA: Color32 = Color32::rgb(160, 82, 45);
    /// sky blue color rgb(135, 206, 235)
    pub const SKY_BLUE: Color32 = Color32::rgb(135, 206, 235);
    /// slate blue color rgb(106, 90, 205)
    pub const SLATE_BLUE: Color32 = Color32::rgb(106, 90, 205);
    /// slate gray color rgb(112, 128, 144)
    pub const SLATE_GRAY: Color32 = Color32::rgb(112, 128, 144);
    /// slate grey color rgb(112, 128, 144)
    pub const SLATE_GREY: Color32 = Color32::rgb(112, 128, 144);
    /// snow color rgb(255, 250, 250)
    pub const SNOW: Color32 = Color32::rgb(255, 250, 250);
    /// spring green color rgb(0, 255, 127)
    pub const SPRING_GREEN: Color32 = Color32::rgb(0, 255, 127);
    /// steel blue color rgb(70, 130, 180)
    pub const STEEL_BLUE: Color32 = Color32::rgb(70, 130, 180);
    /// tan color rgb(210, 180, 140)
    pub const TAN: Color32 = Color32::rgb(210, 180, 140);
    /// thistle color rgb(216, 191, 216)
    pub const THISTLE: Color32 = Color32::rgb(216, 191, 216);
    /// tomato color rgb(255, 99, 71)
    pub const TOMATO: Color32 = Color32::rgb(255, 99, 71);
    /// turquoise color rgb(64, 224, 208)
    pub const TURQUOISE: Color32 = Color32::rgb(64, 224, 208);
    /// violet color rgb(238, 130, 238)
    pub const VIOLET: Color32 = Color32::rgb(238, 130, 238);
    /// wheat color rgb(245, 222, 179)
    pub const WHEAT: Color32 = Color32::rgb(245, 222, 179);
    /// white smoke color rgb(245, 245, 245)
    pub const WHITE_SMOKE: Color32 = Color32::rgb(245, 245, 245);
    /// yellow green color rgb(154, 205, 50)
    pub const YELLOW_GREEN: Color32 = Color32::rgb(154, 205, 50);
}

/// What type of user motion is the device capable of tracking? For the normal fully capable XR headset, this should be
/// 6dof (rotation and translation), but more limited headsets may be restricted to 3dof (rotation) and flatscreen
/// computers with the simulator off would be none.
/// <https://stereokit.net/Pages/StereoKit/DeviceTracking.html>
///
/// see [`Device::get_tracking`]
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
///
/// see [`Device::get_display_type`]
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

/// TODO: waiting for C# implementation
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
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{util::{Device, DisplayType, DeviceTracking}, sk::DisplayBlend};
///
/// // These are the expected results for tests on a PC:
/// let display_type = Device::get_display_type();
/// assert_eq!(display_type, DisplayType::Flatscreen);
///
/// let display_blend = Device::get_display_blend();
/// assert_eq!(display_blend, DisplayBlend::Opaque);
///
/// let device_tracking = Device::get_tracking();
/// assert_eq!(device_tracking, DeviceTracking::None);
///
/// let device_name = Device::get_name().unwrap();
/// assert_eq!(device_name, "Offscreen");
///
/// let device_runtime = Device::get_runtime().unwrap();
/// assert_eq!(device_runtime, "None");
///
/// let device_gpu = Device::get_gpu().unwrap();
/// assert_ne!(device_gpu, "Name of your GPU");
///
/// assert_eq!(Device::has_eye_gaze(), false);
/// assert_eq!(Device::has_hand_tracking(), false);
///
/// assert_eq!(Device::valid_blend(DisplayBlend::None), false);
/// assert_eq!(Device::display_blend(DisplayBlend::None), false);
/// ```
pub struct Device;

unsafe extern "C" {
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
    /// see also [`device_display_set_blend`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Device, sk::DisplayBlend};
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::get_display_blend(), DisplayBlend::Opaque);
    ///
    /// assert_eq!(Device::display_blend(DisplayBlend::AnyTransparent), false);
    /// assert_eq!(Device::display_blend(DisplayBlend::None), false);
    /// assert_eq!(Device::display_blend(DisplayBlend::Additive), false);
    /// ```
    pub fn display_blend(blend: DisplayBlend) -> bool {
        unsafe { device_display_set_blend(blend) != 0 }
    }

    /// Allows you to set and get the current blend mode of the device! Setting this may not succeed if the blend mode
    /// is not valid.
    /// <https://stereokit.net/Pages/StereoKit/Device/DisplayBlend.html>
    ///
    /// see also [`device_display_get_blend`]
    /// see example in [`Device::display_blend`]
    pub fn get_display_blend() -> DisplayBlend {
        unsafe { device_display_get_blend() }
    }

    /// What type of display is this? Most XR headsets will report stereo, but the Simulator will report flatscreen.
    /// <https://stereokit.net/Pages/StereoKit/Device/DisplayType.html>
    ///
    /// see also [`device_display_get_type`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::{Device, DisplayType};
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::get_display_type(), DisplayType::Flatscreen);
    /// ```
    pub fn get_display_type() -> DisplayType {
        unsafe { device_display_get_type() }
    }

    /// This is the name of the OpenXR runtime that powers the current device! This can help you determine which
    /// implementation quirks to expect based on the codebase used. On the simulator, this will be "Simulator", and in
    /// other non-XR modes this will be "None".
    /// <https://stereokit.net/Pages/StereoKit/Device/Runtime.html>
    ///
    /// see also [`device_get_runtime`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Device;
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::get_runtime().unwrap(), "None");
    /// ```
    pub fn get_runtime<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_runtime()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// The reported name of the GPU, this will differ between D3D and GL.
    /// <https://stereokit.net/Pages/StereoKit/Device/GPU.html>
    ///
    /// see also [`device_get_gpu`]
    pub fn get_gpu<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_gpu()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// Does the device we’re on have eye tracking support present for input purposes? This is not an indicator that the
    /// user has given the application permission to access this information. See Input.Gaze for how to use this data.
    /// <https://stereokit.net/Pages/StereoKit/Device/HasEyeGaze.html>
    ///
    /// see also [`device_has_eye_gaze`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Device;
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::has_eye_gaze(), false);
    /// ```
    pub fn has_eye_gaze() -> bool {
        unsafe { device_has_eye_gaze() != 0 }
    }

    /// Tells if the device is capable of tracking hands. This does not tell if the user is actually using their hands
    /// for input, merely that it’s possible to!
    /// <https://stereokit.net/Pages/StereoKit/Device/HasHandTracking.html>
    ///
    /// see also [`device_has_hand_tracking`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Device;
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::has_hand_tracking(), false);
    /// ```
    pub fn has_hand_tracking() -> bool {
        unsafe { device_has_hand_tracking() != 0 }
    }

    /// This is the name of the active device! From OpenXR, this is the same as systemName from XrSystemProperties. The
    /// simulator will say “Simulator”.
    /// <https://stereokit.net/Pages/StereoKit/Device/Name.html>
    ///
    /// see also [`device_get_name`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Device;
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::get_name().unwrap(), "Offscreen");
    /// ```
    pub fn get_name<'a>() -> Result<&'a str, StereoKitError> {
        unsafe { CStr::from_ptr(device_get_name()) }
            .to_str()
            .map_err(|e| StereoKitError::CStrError(e.to_string()))
    }

    /// The tracking capabilities of this device! Is it 3DoF, rotation only? Or is it 6DoF, with positional tracking as
    /// well? Maybe it can’t track at all!
    /// <https://stereokit.net/Pages/StereoKit/Device/Tracking.html>
    ///
    /// see also [`device_get_tracking`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::{Device, DeviceTracking};
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::get_tracking(), DeviceTracking::None);
    /// ```
    pub fn get_tracking() -> DeviceTracking {
        unsafe { device_get_tracking() }
    }

    /// Tells if a particular blend mode is valid on this device. Some devices may be capable of more than one blend
    /// mode.
    /// <https://stereokit.net/Pages/StereoKit/Device/ValidBlend.html>
    ///
    /// see also [`device_display_valid_blend`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Device, sk::DisplayBlend};
    ///
    /// // These are the expected results for tests on a PC:
    /// assert_eq!(Device::valid_blend(DisplayBlend::Opaque), true);
    /// assert_eq!(Device::valid_blend(DisplayBlend::None), false);
    /// assert_eq!(Device::valid_blend(DisplayBlend::Additive), false);
    /// assert_eq!(Device::valid_blend(DisplayBlend::Blend), false);
    /// assert_eq!(Device::valid_blend(DisplayBlend::AnyTransparent), false);
    /// ```
    pub fn valid_blend(blend: DisplayBlend) -> bool {
        unsafe { device_display_valid_blend(blend) != 0 }
    }
}

/// A color/position pair for Gradient values!
/// <https://stereokit.net/Pages/StereoKit/GradientKey.html>
///
/// see [`Gradient`]
/// ### Examples
/// ```
/// use stereokit_rust::util::{GradientKey, Color128, named_colors};
///
/// let key0 = GradientKey::new(named_colors::GOLD, 0.75);
/// let key1 = GradientKey::new(   Color128::new(1.0, 0.84313726, 0.0, 1.0), 0.75);
/// let key2 = GradientKey{ color: Color128::new(1.0, 0.84313726, 0.0, 1.0), position: 0.75 };
///
/// assert_eq!(key0, key1);
/// assert_eq!(key1, key2);
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
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
    /// * `color_linear` - The color for this item, preferably in some form of linear color space. Gamma corrected
    ///   colors will definitely not math correctly.
    /// * `position` - Typically a value between 0-1! This is the position of the color along the ‘x-axis’ of the
    ///   gradient.
    pub fn new(color_linear: impl Into<Color128>, position: f32) -> Self {
        Self { color: color_linear.into(), position }
    }
}

/// A Gradient is a sparse collection of color keys that are used to represent a ramp of colors! This class is largely
/// just storing colors and allowing you to sample between them.
///
/// Since the Gradient is just interpolating values, you can use whatever color space you want here, as long as it's
/// linear and not gamma! Gamma space RGB can't math properly at all. It can be RGB(linear), HSV, LAB, just remember
/// which one you have, and be sure to convert it appropriately later. Data is stored as float colors, so this'll be a
/// high accuracy blend!
/// <https://stereokit.net/Pages/StereoKit/Gradient.html>
///
/// see also [GradientKey] [`crate::tex::Tex::gen_particle`] [`crate::tex::SHCubemap::gen_cubemap_gradient`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec3, system::AssetState, tex::{Tex, SHCubemap},
///                      util::{named_colors, Gradient, GradientKey, Color128}};
///
/// let mut keys = [
///     GradientKey::new(Color128::BLACK_TRANSPARENT, 0.0),
///     GradientKey::new(named_colors::RED, 0.1),
///     GradientKey::new(named_colors::CYAN, 0.4),
///     GradientKey::new(named_colors::BLUE, 0.5),
///     GradientKey::new(Color128::BLACK, 0.7)];
///
/// let sh_cubemap = SHCubemap::gen_cubemap_gradient(Gradient::new(Some(&keys)),
///                                                  Vec3::UP, 128);
/// sh_cubemap.render_as_sky();
///
/// let mut gradient = Gradient::new(None);
/// gradient
///     .add(Color128::BLACK_TRANSPARENT, 0.0)
///     .add(named_colors::YELLOW, 0.1)
///     .add(named_colors::LIGHT_BLUE, 0.4)
///     .add(named_colors::BLUE, 0.5)
///     .add(Color128::BLACK, 0.7);
/// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
/// ```
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

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _GradientT {
    _unused: [u8; 0],
}

/// StereoKit ffi type.
pub type GradientT = *mut _GradientT;

unsafe extern "C" {
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
    /// * `keys` - These can be in any order that you like, they’ll be sorted by their GradientKey.position value
    ///   regardless!
    ///
    /// see also [`gradient_create`][`gradient_create_keys`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, tex::{Tex, SHCubemap},
    ///                      util::{named_colors, Gradient, GradientKey, Color128}};
    ///
    /// let mut keys = [
    ///     GradientKey::new(Color128::BLACK_TRANSPARENT, 0.0),
    ///     GradientKey::new(named_colors::RED, 0.5),
    ///     GradientKey::new(Color128::BLACK, 0.7)];
    ///
    /// let gradient1 = Gradient::new(Some(&keys));
    /// assert_eq!(gradient1.get_count(), 3);
    /// let sh_cubemap = SHCubemap::gen_cubemap_gradient(gradient1, Vec3::UP, 16);
    /// sh_cubemap.render_as_sky();
    ///
    /// let mut gradient2 = Gradient::new(Some(&keys));
    /// gradient2.add(named_colors::CYAN, 0.4);
    /// assert_eq!(gradient2.get_count(), 4);
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient2));
    /// ```
    pub fn new(keys: Option<&[GradientKey]>) -> Self {
        match keys {
            Some(keys) => {
                Gradient(NonNull::new(unsafe { gradient_create_keys(keys.as_ptr(), keys.len() as i32) }).unwrap())
            }
            None => Gradient(NonNull::new(unsafe { gradient_create() }).unwrap()),
        }
    }

    /// This adds a color key into the list. It’ll get inserted to the right slot based on its position.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Add.html>
    /// * `color_linear` - Any linear space color you like!
    /// * `position` - Typically a value between 0-1! This is the position of the color along the ‘x-axis’ of the
    ///   gradient.
    ///
    /// see also [`gradient_add`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient, Color128}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    /// assert_eq!(gradient.get_count(), 5);
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// ```
    pub fn add(&mut self, color_linear: impl Into<Color128>, position: f32) -> &mut Self {
        unsafe { gradient_add(self.0.as_ptr(), color_linear.into(), position) };
        self
    }

    /// Updates the color key at the given index! This will NOT re-order color keys if they are moved past another
    /// key’s position, which could lead to strange behavior.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Set.html>
    /// * `index` - Index of the color key to change.
    /// * `color_linear` - Any linear space color you like!
    /// * `position` - Typically a value between 0-1! This is the position of the color along the ‘x-axis’ of the
    ///   gradient.
    ///
    /// see also [`gradient_set`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient,  Color128}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    /// assert_eq!(gradient.get_count(), 5);
    /// assert_eq!(gradient.get(0.3), Color128 { r: 0.7856209, g: 0.8980392, b: 0.6013072, a: 1.0 });
    ///
    /// gradient.set(2, named_colors::RED, 0.3);
    /// gradient.set(-20, named_colors::RED, -10.3); // out of bounds, should do nothing
    /// assert_eq!(gradient.get_count(), 5);
    /// assert_eq!(gradient.get(0.3), named_colors::RED.into());
    ///
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// ```
    pub fn set(&mut self, index: i32, color_linear: impl Into<Color128>, position: f32) -> &mut Self {
        if index < 0 || index >= self.get_count() {
            return self;
        }
        unsafe { gradient_set(self.0.as_ptr(), index, color_linear.into(), position) };
        self
    }

    /// Removes the color key at the given index! This won't reindex the gradient so get_count will still return the
    /// same value.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Remove.html>
    /// * `index` - The index of the color key to remove.
    ///
    /// see also [`gradient_remove`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient, Color128}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    /// assert_eq!(gradient.get_count(), 5);
    /// assert_eq!(gradient.get(0.4), named_colors::LIGHT_BLUE.into());
    ///
    /// gradient.remove(2);
    /// gradient.remove(19).remove(-189);
    /// assert_eq!(gradient.get_count(), 5);
    /// assert_eq!(gradient.get(0.4), Color128 { r: 0.25, g: 0.25, b: 0.75, a: 1.0 });
    ///
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// ```
    pub fn remove(&mut self, index: i32) -> &mut Self {
        if index < 0 || index >= self.get_count() {
            return self;
        }
        unsafe { gradient_remove(self.0.as_ptr(), index) };
        self
    }

    /// The number of color keys present in this gradient.
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Count.html>
    ///
    /// see also [`gradient_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient, Color128}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// assert_eq!(gradient.get_count(), 0);
    ///
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(Color128::BLACK, 0.7);
    /// assert_eq!(gradient.get_count(), 3);
    ///
    /// gradient.remove(1);
    /// assert_eq!(gradient.get_count(), 3);
    ///
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    ///```
    pub fn get_count(&self) -> i32 {
        unsafe { gradient_count(self.0.as_ptr()) }
    }

    /// Samples the gradient’s color at the given position!
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Get.html>
    /// * `at` - Typically a value between 0-1, but if you used larger or smaller values for your color key’s positions,
    ///   it’ll be in that range!
    ///
    /// Returns the interpolated color at the given position. If ‘at’ is smaller or larger than the gradient’s position
    /// range, then the color will be clamped to the color at the beginning or end of the gradient!
    /// see also [`gradient_get`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient, Color128}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    ///
    /// assert_eq!(gradient.get(0.3), Color128 { r: 0.7856209, g: 0.8980392, b: 0.6013072, a: 1.0 });
    /// assert_eq!(gradient.get(0.4), named_colors::LIGHT_BLUE.into());
    /// assert_eq!(gradient.get(0.5), named_colors::BLUE.into());
    ///
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// ```
    pub fn get(&self, at: f32) -> Color128 {
        unsafe { gradient_get(self.0.as_ptr(), at) }
    }

    /// Samples the gradient’s color at the given position, and converts it to a 32 bit color. If your RGBA color
    /// values are outside of the 0-1 range, then you’ll get some issues as they’re converted to 0-255 range bytes!
    /// <https://stereokit.net/Pages/StereoKit/Gradient/Remove.html>
    /// * `at` - Typically a value between 0-1, but if you used larger or smaller values for your color key’s positions,
    ///   it’ll be in that range!
    ///
    /// Returns the interpolated 32 bit color at the given position. If ‘at’ is smaller or larger than the gradient’s
    /// position range, then the color will be clamped to the color at the beginning or end of the gradient!
    /// see also [`gradient_get32`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tex::Tex, util::{named_colors, Gradient, Color128, Color32}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    ///
    /// assert_eq!(gradient.get32(0.3), Color32 { r: 200, g: 229, b: 153, a: 255 });
    /// assert_eq!(gradient.get32(0.4), named_colors::LIGHT_BLUE);
    /// assert_eq!(gradient.get32(0.5), named_colors::BLUE);
    ///
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// ```
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

/// TODO: UNSTABLE: When opening the Platform.FilePicker, this enum describes how the picker should look and behave.
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
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{util::{Platform}, system::TextContext};
///
/// Platform::keyboard_show(true, TextContext::Text);
///
/// let mut path = std::env::current_dir().expect("Current directory should be readable");
/// path.push("assets/textures/");
/// assert!(path.is_dir());
/// path.push("readme.md");
/// assert!(path.is_file());
///
/// let file_content = Platform::read_file_text(&path)
///                                  .expect("File should be readable");
/// assert!(file_content.starts_with("# Images "));
///
/// assert!(Platform::write_file_text(path, file_content).is_ok());
/// ```
pub struct Platform;

unsafe extern "C" {
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
        keyboard_layout: *const *const c_char,
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
    let data = unsafe { &mut *(user_data as *mut (&mut FS, &mut FC)) };
    let (update, cancel) = data;
    if confirmed != 0 {
        let c_str = unsafe { CStr::from_ptr(filename).to_str().unwrap() };
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
    let closure = unsafe { &mut *(user_data as *mut &mut F) };
    if confirmed != 0 && filename_length > 0 {
        let c_str = unsafe { CStr::from_ptr(filename).to_str().unwrap() };
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
    /// see also [`platform_keyboard_set_force_fallback`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Platform;
    ///
    /// assert_eq!(Platform::get_force_fallback_keyboard(), false);
    ///
    /// Platform::force_fallback_keyboard(true);
    /// assert_eq!(Platform::get_force_fallback_keyboard(), true);
    ///
    /// Platform::force_fallback_keyboard(false);
    /// assert_eq!(Platform::get_force_fallback_keyboard(), false);
    /// ```
    pub fn force_fallback_keyboard(force_fallback: bool) {
        unsafe { platform_keyboard_set_force_fallback(force_fallback as Bool32T) }
    }

    /// TODO: UNSTABLE: Starts a file picker window! This will create a native file picker window if one is available in the current
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
    /// * `mode` - Are we trying to Open a file, or Save a file? This changes the appearance and behavior of the picker
    ///   to support the specified action.
    /// * `on_select_file` - This Action will be called with the proper filename when the picker has successfully
    ///   completed! On a cancel or close event, this Action is not called.
    /// * `on_cancel` - If the user cancels the file picker, or the picker is closed via FilePickerClose, this Action is
    ///   called.
    /// * `filters` - A list of file extensions that the picker should filter for. This is in the format of “.glb” and
    ///   is case insensitive.
    ///
    /// see also [`platform_file_picker`]
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

    /// TODO: UNSTABLE: Starts a file picker window! This will create a native file picker window if one is available in the current
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
    /// * `mode` - Are we trying to Open a file, or Save a file? This changes the appearance and behavior of the picker
    ///   to support the specified action.
    /// * on_complete - This action will be called when the file picker has finished, either via a cancel event, or from
    ///   a confirm event. First parameter is a bool, where true indicates the presence of a valid filename, and false
    ///   indicates a failure or cancel event.
    /// * `filters` - A list of file extensions that the picker should filter for. This is in the format of “.glb” and
    ///   is case insensitive.
    ///
    /// see also [`platform_file_picker_sz`]
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

    /// TODO: UNSTABLE: If the picker is visible, this will close it and immediately trigger a cancel event for the active picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePickerClose.html>
    ///
    /// see also [`platform_file_picker_close`]
    pub fn file_picker_close() {
        unsafe { platform_file_picker_close() }
    }

    /// Request or hide a soft keyboard for the user to type on. StereoKit will surface OS provided soft keyboards where
    /// available, and use a fallback keyboard when not. On systems with physical keyboards, soft keyboards generally
    /// will not be shown if the user has interacted with their physical keyboard recently.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardShow.html>
    /// * `show` - Tells whether or not to open or close the soft keyboard.
    /// * `input_type` - Soft keyboards can change layout to optimize for the type of text that’s required. StereoKit
    ///   will request the soft keyboard layout that most closely represents the TextContext provided.
    ///
    /// see also [`platform_keyboard_show`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Platform, system::TextContext};
    ///
    /// // On desktop, this will not show the keyboard, as it is assumed that the user has a physical keyboard.
    /// Platform::keyboard_show(true, TextContext::Text);
    /// assert_eq!(Platform::is_keyboard_visible(), false);
    ///
    /// Platform::keyboard_show(false, TextContext::Text);
    /// assert_eq!(Platform::is_keyboard_visible(), false);
    /// ```
    pub fn keyboard_show(show: bool, input_type: TextContext) {
        unsafe { platform_keyboard_show(show as Bool32T, input_type) }
    }

    /// Replace the default keyboard type with a custom layout.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardSetLayout.html>
    /// * `keyboard_type` - The type of keyboard to replace.
    /// * `keyboard_layouts` - Custom keyboard layout to replace the defualt layout.
    ///
    /// Returns `true` if keyboard type was swapped with the provided layout.
    /// see also [`platform_keyboard_set_layout`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::Platform, system::TextContext};
    ///
    /// pub const FR_KEY_TEXT: &str = r#"²|&|é|"|'|(|\-|è|_|ç|à|)|=|{|}|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
    /// Tab-\t-9-3|a|z|e|r|t|y|u|i|o|p|^|$|[|]|\|
    /// Entrée-\n-13-4|q|s|d|f|g|h|j|k|l|m|ù|*|#|Entrée-\n-13-3
    /// spr:sk/ui/shift--16-3-go_1|<|w|x|c|v|b|n|,|;|:|!|`|@|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
    /// Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;
    ///
    /// pub const FR_KEY_TEXT_SHIFT: &str = r#"@|1|2|3|4|5|6|7|8|9|0|°|+|Æ|Œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
    /// Tab-\t-9-3|A|Z|E|R|T|Y|U|I|O|P|¨|£|Ê|É|È
    /// Entrée-\n-13-4|Q|S|D|F|G|H|J|K|L|M|%|µ|Ç|Entrée-\n-13-3
    /// spr:sk/ui/shift--16-3-go_0|>|W|X|C|V|B|N|?|.|/|§|À|Ô|spr:sk/ui/shift--16-2-go_0|spr:sk/ui/arrow_up--38
    /// Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_2| - -32-13|Alt--18-3-go_2|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;
    ///
    /// pub const FR_KEY_TEXT_ALT: &str = r#"*|/|~|#|{|[|\||`|\\|^|@|]|}|æ|œ|spr:sk/ui/backspace-\b-8-3|spr:sk/ui/close----close
    /// Tab-\t-9-3|à|â|ä|ç|é|è|ê|ë|î|ï|ô|ö|«|»|¤
    /// Entrée-\n-13-4|ù|û|ü|ÿ|À|Â|Ä|Ç|É|È|Ê|Ë|%|Entrée-\n-13-3
    /// spr:sk/ui/shift--16-3-go_1|Î|Ï|Ô|Ö|Ù|Û|Ü|Ÿ|$|£|€|¥|✋|spr:sk/ui/shift--16-2-go_1|spr:sk/ui/arrow_up--38
    /// Ctrl--17-4-mod|Cmd--91-3|Alt--18-3-go_0| - -32-13|Alt--18-3-go_0|Ctrl--17-3-mod|spr:sk/ui/arrow_left--37|spr:sk/ui/arrow_down--40|spr:sk/ui/arrow_right--39|"#;
    ///
    /// let keyboard_layouts = vec![FR_KEY_TEXT, FR_KEY_TEXT_SHIFT, FR_KEY_TEXT_ALT];
    ///
    /// assert_eq!(Platform::keyboard_set_layout(TextContext::Text, &keyboard_layouts), true);
    /// ```
    pub fn keyboard_set_layout(keyboard_type: TextContext, keyboard_layouts: &Vec<&str>) -> bool {
        let mut keyboard_layouts_c = vec![];
        for str in keyboard_layouts {
            let c_str = CString::new(*str).unwrap().into_raw() as *const c_char;
            keyboard_layouts_c.push(c_str);
        }
        unsafe {
            platform_keyboard_set_layout(
                keyboard_type,
                keyboard_layouts_c.as_slice().as_ptr(),
                keyboard_layouts_c.len() as i32,
            ) != 0
        }
    }

    /// Reads the entire contents of the file as a UTF-8 string, taking advantage of any permissions that may have been
    /// granted by Platform::file_picker(_sz?). Returns Err on failure.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ReadFileText.html>
    /// * `filename` - The path to the file to read.
    ///
    /// see also [`platform_read_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Platform}, system::TextContext};
    ///
    /// let mut path = std::env::current_dir().expect("Current directory should be readable");
    /// path.push("config.toml");
    /// assert!(path.is_file());
    ///
    /// let file_content = Platform::read_file_text(&path)
    ///                                  .expect("File should be readable");
    /// assert!(file_content.starts_with("[env]"));
    /// ```
    pub fn read_file_text<'a>(filename: impl AsRef<Path>) -> Result<&'a str, StereoKitError> {
        let path_buf = filename.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str() //
                .ok_or(StereoKitError::ReadFileError(path_buf.clone(), "Failed to convert path to string".into()))?,
        )?;
        let out_data = CString::new("H")?.into_raw() as *mut *mut c_void;
        let mut len = 0usize;
        let len_ptr: *mut usize = &mut len;
        if unsafe { platform_read_file(c_str.as_ptr(), out_data, len_ptr) != 0 } {
            unsafe { CStr::from_ptr(*out_data as *const c_char) }
                .to_str()
                .map_err(|e| StereoKitError::ReadFileError(path_buf.clone(), e.to_string()))
        } else {
            Err(StereoKitError::ReadFileError(path_buf, "Failed to read file".into()))
        }
    }

    /// Reads the entire contents of the file as a byte array, taking advantage of any permissions that may have been
    /// granted by Platform.FilePicker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ReadFile.html>
    /// * `filename` - The path to the file to read.
    ///
    /// see also [`platform_read_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Platform}, system::TextContext};
    ///
    /// let mut path = std::env::current_dir().expect("Current directory should be readable");
    /// path.push("assets/textures/");
    /// assert!(path.is_dir());
    /// path.push("readme.md");
    /// assert!(path.is_file());
    ///
    /// let file_content = Platform::read_file(&path)
    ///                                  .expect("File should be readable");
    /// assert!(file_content.starts_with(b"# Images "));
    /// ```
    pub fn read_file<'a>(filename: impl AsRef<Path>) -> Result<&'a [u8], StereoKitError> {
        let path_buf = filename.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str() //
                .ok_or(StereoKitError::ReadFileError(path_buf.clone(), "Failed to convert path to string".into()))?,
        )?;
        let out_data = CString::new("H")?.into_raw() as *mut *mut c_void;
        let mut len = 0usize;
        let len_ptr: *mut usize = &mut len;
        if unsafe { platform_read_file(c_str.as_ptr(), out_data, len_ptr) != 0 } {
            Ok(unsafe { std::slice::from_raw_parts(*out_data as *const u8, len) })
        } else {
            Err(StereoKitError::ReadFileError(path_buf, "Failed to read file".into()))
        }
    }

    /// Writes a UTF-8 text file to the filesystem, taking advantage of any permissions that may have been granted by
    /// Platform::file_picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/WriteFile.html>
    /// * `filename` - The path to the file to write.
    /// * `text` -  	A string to write to the file. This gets converted to a UTF-8 encoding.
    ///
    /// see also [`platform_write_file_text`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Platform}, system::TextContext};
    ///
    /// let mut path = std::env::current_dir().expect("Current directory should be readable");
    /// path.push("assets/icons/");
    /// assert!(path.is_dir());
    /// path.push("readme.md");
    /// assert!(path.is_file());
    ///
    /// let file_content = Platform::read_file_text(&path)
    ///                                  .expect("File should be readable");
    /// assert!(file_content.starts_with("# Images "));
    ///
    /// assert!(Platform::write_file_text(path, file_content).is_ok());
    /// ```
    pub fn write_file_text<S: AsRef<str>>(filename: impl AsRef<Path>, text: S) -> Result<bool, StereoKitError> {
        let path_buf = filename.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str() //
                .ok_or(StereoKitError::WriteFileError(path_buf.clone(), "Failed to convert path to string".into()))?,
        )?;
        let in_data = CString::new(text.as_ref())?.into_raw() as *const c_char;
        if unsafe { platform_write_file_text(c_str.as_ptr(), in_data) != 0 } {
            Ok(true)
        } else {
            Err(StereoKitError::WriteFileError(path_buf, "Failed to write file".into()))
        }
    }

    /// Writes an array of bytes to the filesystem, taking advantage of any permissions that may have been granted by
    /// Platform::file_picker.
    /// <https://stereokit.net/Pages/StereoKit/Platform/WriteFile.html>
    /// * `filename` - The path to the file to write.
    /// * `data` - The data to write to the file.
    ///
    /// see also [`platform_write_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{Platform}, system::TextContext};
    ///
    /// let mut path = std::env::current_dir().expect("Current directory should be readable");
    /// path.push("assets/icons/");
    /// assert!(path.is_dir());
    /// path.push("readme.md");
    /// assert!(path.is_file());
    ///
    /// let file_content = Platform::read_file(&path)
    ///                                  .expect("File should be readable");
    /// assert!(file_content.starts_with(b"# Images "));
    ///
    /// assert!(Platform::write_file(path, file_content).is_ok());
    /// ```
    pub fn write_file(filename: impl AsRef<Path>, data: &[u8]) -> Result<bool, StereoKitError> {
        let path_buf = filename.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str() //
                .ok_or(StereoKitError::WriteFileError(path_buf.clone(), "Failed to convert path to string".into()))?,
        )?;
        if unsafe { platform_write_file(c_str.as_ptr(), data.as_ptr() as *mut c_void, data.len()) != 0 } {
            Ok(true)
        } else {
            Err(StereoKitError::WriteFileError(path_buf, "Failed to write file".into()))
        }
    }

    /// TODO: UNSTABLE: This will check if the file picker interface is currently visible. Some pickers will never show this, as they
    /// block the application until the picker has completed.
    /// <https://stereokit.net/Pages/StereoKit/Platform/FilePickerVisible.html>
    ///
    /// see also [`platform_file_picker_visible`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Platform;
    ///
    /// assert_eq!(Platform::get_file_picker_visible(), false);
    /// ```
    pub fn get_file_picker_visible() -> bool {
        unsafe { platform_file_picker_visible() != 0 }
    }

    /// Force the use of StereoKit’s built-in fallback keyboard instead of the system keyboard. This may be great for
    /// testing or look and feel matching, but the system keyboard should generally be preferred for accessibility
    /// reasons.
    /// <https://stereokit.net/Pages/StereoKit/Platform/ForceFallbackKeyboard.html>
    ///
    /// see also [`platform_keyboard_get_force_fallback`]
    /// see example [`Platform::force_fallback_keyboard`]
    pub fn get_force_fallback_keyboard() -> bool {
        unsafe { platform_keyboard_get_force_fallback() != 0 }
    }

    /// Check if a soft keyboard is currently visible. This may be an OS provided keyboard or StereoKit’s fallback
    /// keyboard, but will not indicate the presence of a physical keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Platform/KeyboardVisible.html>
    ///
    /// see also [`platform_keyboard_visible`]
    /// see example [`Platform::keyboard_show`]
    pub fn is_keyboard_visible() -> bool {
        unsafe { platform_keyboard_visible() != 0 }
    }
}

/// A light source used for creating SphericalHarmonics data.
/// <https://stereokit.net/Pages/StereoKit/SHLight.html>
///
/// see [`SphericalHarmonics`] see also [`crate::tex::SHCubemap`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec3, tex::SHCubemap, util::{SHLight, named_colors, Color128}};
///
/// let light0 = SHLight::new([1.0, 0.2, 0.3], named_colors::RED);
/// let light1 = SHLight::new(Vec3::new(1.0, 0.2, 0.3), Color128::new(1.0, 0.0, 0.0, 1.0));
/// let light2 = SHLight{ dir_to: Vec3::new(1.0, 0.2, 0.3), color: named_colors::RED.into()};
///
/// assert_eq!(light0, light1);
/// assert_eq!(light1, light2);
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct SHLight {
    /// Direction to the light source.
    pub dir_to: Vec3,
    /// Color of the light in linear space! Values here can exceed 1.
    pub color: Color128,
}
impl SHLight {
    /// A basic constructor for SHLight.
    /// <https://stereokit.net/Pages/StereoKit/SHLight.html>
    /// * `dir_to` - Direction to the light source.
    /// * `color` - Color of the light in linear space! Values here can exceed 1.
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
///
/// see also: [`crate::tex::SHCubemap`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec3,
///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
///
/// let light0 = SHLight::new([1.0, 0.0, 0.0], named_colors::RED);
/// let light1 = SHLight::new([0.0, 1.0, 0.0], named_colors::GREEN);
/// let light2 = SHLight::new([0.0, 0.0, 1.0], named_colors::BLUE);
///
/// let mut sh = SphericalHarmonics::from_lights(&[light0, light1, light2]);
/// sh.brightness(1.0)
///   .add(Vec3::NEG_Y, named_colors::GREEN)
///   .add(Vec3::NEG_Z, named_colors::BLUE)
///   .add(Vec3::NEG_X, named_colors::RED);
///
/// assert_eq!(sh.get_sample(Vec3::UP), Color128 { r: 0.5813507, g: 0.8046322, b: 0.5813487, a: 1.0 });
/// assert_eq!(sh.get_dominent_light_direction(), Vec3 { x: 0.27644092, y: 0.2728996, z: 0.9214696 });
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct SphericalHarmonics {
    pub coefficients: [Vec3; 9usize],
}

unsafe extern "C" {
    pub fn sh_create(in_arr_lights: *const SHLight, light_count: i32) -> SphericalHarmonics;
    pub fn sh_brightness(ref_harmonics: *mut SphericalHarmonics, scale: f32);
    pub fn sh_add(ref_harmonics: *mut SphericalHarmonics, light_dir: Vec3, light_color: Vec3);
    pub fn sh_lookup(harmonics: *const SphericalHarmonics, normal: Vec3) -> Color128;
    pub fn sh_dominant_dir(harmonics: *const SphericalHarmonics) -> Vec3;
}
impl SphericalHarmonics {
    /// Creates a SphericalHarmonics approximation of the irradiance given from a set of directional lights!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/FromLights.html>
    /// * `lights` - A list of directional lights!
    ///
    /// Returns a SphericalHarmonics approximation of the irradiance given from a set of directional lights!
    /// see also [`SHLight`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3,
    ///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
    ///
    /// let light0 = SHLight::new([1.0, 1.0, 1.0], named_colors::RED);
    /// let light1 = SHLight::new([0.5, 0.5, 0.5], named_colors::RED);
    /// let light2 = SHLight::new([0.25, 0.25, 0.25], named_colors::RED);
    ///
    /// let sh = SphericalHarmonics::from_lights(&[light0, light1, light2]);
    /// //TODO: assert_eq!(sh.get_sample(Vec3::UP), named_colors::RED.into());
    /// assert_eq!(sh.get_sample(Vec3::UP), Color128 { r: 2.2098913, g: 0.0, b: 0.0, a: 1.0 });
    /// assert_eq!(sh.get_dominent_light_direction(), -Vec3::ONE.get_normalized());
    /// ```
    pub fn from_lights(lights: &[SHLight]) -> Self {
        unsafe { sh_create(lights.as_ptr(), lights.len() as i32) }
    }

    /// Creates a SphericalHarmonic from an array of coefficients. Useful for loading stored data!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/SphericalHarmonics.html>
    /// * `coefficients` - Must be an array with a length of 9!
    ///
    /// see also [`sh_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3,
    ///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
    ///
    /// let mut sh0 = SphericalHarmonics::default();
    /// sh0.add([1.0, 0.0, 1.0], named_colors::RED);
    /// let coefficient = sh0.coefficients;
    ///
    /// let sh = SphericalHarmonics::new(coefficient);
    /// //TODO: assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), named_colors::RED.into());
    /// assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), Color128 { r: 11.453729, g: 0.0, b: 0.0, a: 1.0 });
    /// assert_eq!(sh.get_dominent_light_direction(), Vec3::new(-1.0, 0.0, -1.0).get_normalized());
    /// ```
    pub fn new(coefficients: [Vec3; 9]) -> Self {
        SphericalHarmonics { coefficients }
    }

    /// Adds a ‘directional light’ to the lighting approximation. This can be used to bake a multiple light setup, or
    /// accumulate light
    /// from a field of points.
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Add.html>
    /// * `light_dir` - The direction of the light source.
    /// * `light_color` - Color of the light, in linear color space.
    ///
    /// see also [`sh_add`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3,
    ///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
    ///
    /// let mut sh = SphericalHarmonics::default();
    /// sh.add([1.0, 0.0, 1.0], named_colors::RED)
    ///   .add([0.0, 1.0, 0.0], named_colors::GREEN)
    ///   .add([0.0, 0.0, 1.0], named_colors::BLUE);
    ///
    /// // TODO:
    /// assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), Color128 { r: 11.453729, g: -0.2956792, b: 4.4505944, a: 1.0 });
    /// assert_eq!(sh.get_dominent_light_direction(), Vec3 { x: -0.21951628, y: -0.21670417, z: -0.95123714 });
    ///```
    pub fn add(&mut self, light_dir: impl Into<Vec3>, light_color: impl Into<Color128>) -> &mut Self {
        let light_dir = light_dir.into();
        let color = light_color.into();
        unsafe { sh_add(self, light_dir, Vec3 { x: color.r, y: color.g, z: color.b }) };
        self
    }

    /// Scales all the SphericalHarmonic’s coefficients! This behaves as if you’re modifying the brightness of the
    /// lighting this object represents.
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Brightness.html>
    /// * `scale` - A multiplier for the coefficients! A value of 1 will leave everything the same, 0.5 will cut the
    ///   brightness in half, and a 2 will double the brightness.
    ///
    /// see also [`sh_brightness`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3,
    ///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
    ///
    /// let mut sh = SphericalHarmonics::default();
    /// sh.add([1.0, 0.0, 1.0], named_colors::RED)
    ///   .brightness(0.5);
    ///
    /// // TODO:
    /// assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), Color128 { r: 5.726864, g: 0.0, b: 0.0, a: 1.0 });
    /// assert_eq!(sh.get_dominent_light_direction(), Vec3::new(-1.0, 0.0, -1.0).get_normalized());
    ///
    /// sh.brightness(2.0);
    /// assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), Color128 { r: 11.453729, g: 0.0, b: 0.0, a: 1.0 });
    /// assert_eq!(sh.get_dominent_light_direction(), Vec3::new(-1.0, 0.0, -1.0).get_normalized());
    ///
    ///
    /// sh.brightness(0.0);
    /// assert_eq!(sh.get_sample([1.0, 0.0, 1.0]), Color128::BLACK);
    /// assert_eq!(sh.get_dominent_light_direction().x.is_nan(), true);
    /// ```
    pub fn brightness(&mut self, scale: f32) -> &mut Self {
        unsafe { sh_brightness(self, scale) };
        self
    }

    /// Look up the color information in a particular direction!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/Sample.html>
    /// * `normal` - The direction to look in. Should be normalized.
    ///
    /// Returns the color represented by the SH in the given direction.
    /// see also [`sh_brightness`]
    pub fn get_sample(&self, normal: impl Into<Vec3>) -> Color128 {
        unsafe { sh_lookup(self, normal.into()) }
    }

    /// Returns the dominant direction of the light represented by this spherical harmonics data. The direction value is
    /// normalized.
    /// You can get the color of the light in this direction by using the struct’s Sample method:
    /// light.get_sample(-light.get_dominent_light_direction()).
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/DominantLightDirection.html>
    ///
    /// see also [`sh_brightness`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3,
    ///                      util::{SHLight, named_colors, Color128, SphericalHarmonics}};
    ///
    /// let mut sh = SphericalHarmonics::default();
    /// sh.add([1.0, 0.0, 1.0], named_colors::RED)
    ///   .add([1.0, 1.0, 0.0], named_colors::GREEN)
    ///   .add([0.0, 1.0, 1.0], named_colors::BLUE);
    ///
    /// assert_eq!(sh.get_dominent_light_direction(), Vec3 { x: -0.3088678, y: -0.6715365, z: -0.6735276 });
    /// ```
    pub fn get_dominent_light_direction(&self) -> Vec3 {
        unsafe { sh_dominant_dir(self) }
    }

    /// Converts the SphericalHarmonic into a vector of coefficients 9 long. Useful for storing calculated data!
    /// <https://stereokit.net/Pages/StereoKit/SphericalHarmonics/ToArray.html>
    ///
    /// Returns an array of coefficients 9 long.
    /// see also [`sh_brightness`]
    pub fn to_array(&self) -> [Vec3; 9] {
        self.coefficients
    }
}

/// This class contains time information for the current session and frame!
/// <https://stereokit.net/Pages/StereoKit/Time.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::util::Time;
///
/// // These are the expected results for tests on a PC:
/// assert_eq!(Time::get_totalf(), 0.0);
/// assert_eq!(Time::get_total(), 0.0);
/// assert_eq!(Time::get_stepf(), 0.0);
/// assert_eq!(Time::get_step(), 0.0);
///
/// // Time passes slowly:
/// Time::scale(0.5);
///
/// let mut total = 0.0f64;
/// let mut totalf = 0.0f32;
/// number_of_steps = 100;
/// test_steps!( // !!!! Get a proper main loop !!!!
///    if iter < number_of_steps + 2 {
///        assert_eq!(Time::get_frame(), iter + 1);
///
///        assert_eq!(Time::get_step_unscaled(), Time::get_step() * 2.0);
///
///        assert_eq!(Time::get_total(),          total + Time::get_step());
///        assert_eq!(Time::get_total_unscaled(), total * 2.0 + Time::get_step() * 2.0);
///
///        // precision is worse for f32
///        assert!((Time::get_totalf()          
///                 - totalf - Time::get_stepf()).abs() < 0.000001);
///        assert!((Time::get_total_unscaledf()
///                 - totalf * 2.0 - Time::get_stepf() * 2.0).abs() < 0.000001);
///    }
///    totalf = Time::get_totalf();
///    total = Time::get_total();
/// );
/// ```
pub struct Time;

unsafe extern "C" {
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
    /// see also [`time_scale`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// // Time passes faster:
    /// Time::scale(2.0);
    ///
    /// let mut total = 0.0f64;
    /// let mut totalf = 0.0f32;
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     assert_eq!(Time::get_step_unscaled(), Time::get_step() / 2.0);
    /// );
    /// ```
    pub fn scale(factor: f64) {
        unsafe { time_scale(factor) }
    }

    /// This allows you to override the application time! The application will progress from this time using the current
    /// timescale.
    /// <https://stereokit.net/Pages/StereoKit/Time/SetTime.html>
    /// * `total_seconds` - What time should it now be? The app will progress from this point in time.
    /// * `frame_elapsed_seconds` - How long was the previous frame? This is a number often used in motion calculations.
    ///   If left to zero, it’ll use the previous frame’s time, and if the previous frame’s time was also zero, it’ll
    ///   use 1/90.
    ///
    /// see also [`time_set_time`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// // Time passes faster:
    /// Time::set_time(10.0, 0.01);
    ///
    /// assert_eq!(Time::get_total(), 10.0);
    /// assert_eq!(Time::get_step(), 0.01);
    /// ```
    pub fn set_time(total_seconds: f64, frame_elapsed_seconds: f64) {
        unsafe { time_set_time(total_seconds, frame_elapsed_seconds) }
    }

    /// The number of frames/steps since the app started.
    /// <https://stereokit.net/Pages/StereoKit/Time/Frame.html>
    ///
    /// see also [`time_frame`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// assert_eq!(Time::get_frame(), 0);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(Time::get_frame(), iter + 1);
    ///     }
    /// );
    /// ```
    pub fn get_frame() -> u64 {
        unsafe { time_frame() }
    }

    /// How many seconds have elapsed since the last frame? 64 bit time precision, calculated at the start of the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Step.html>
    ///
    /// see also [`time_step`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// assert_eq!(Time::get_step(), 0.0);
    ///
    /// let mut total = 0.0f64;
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(Time::get_total(), total + Time::get_step());
    ///     }
    ///     total = Time::get_total();
    /// );
    /// ```    
    pub fn get_step() -> f64 {
        unsafe { time_step() }
    }

    /// How many seconds have elapsed since the last frame? 32 bit time precision, calculated at the start of the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Stepf.html>
    ///
    /// see also [`time_stepf`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// assert_eq!(Time::get_stepf(), 0.0);
    ///
    /// let mut totalf = 0.0f32;
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert!((Time::get_totalf() - totalf - Time::get_stepf()).abs() < 0.000001);
    ///     }
    ///     totalf = Time::get_totalf();
    /// );
    /// ```
    pub fn get_stepf() -> f32 {
        unsafe { time_stepf() }
    }

    /// How many seconds have elapsed since the last frame? 64 bit time precision, calculated at the start of the frame.
    /// This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/StepUnscaled.html>
    ///
    /// see also [`time_step_unscaled`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// assert_eq!(Time::get_step_unscaled(), 0.0);
    ///
    /// let mut total = 0.0f64;
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(Time::get_total_unscaled(), total + Time::get_step_unscaled());
    ///     }
    ///     total = Time::get_total_unscaled();
    /// );
    /// ```
    pub fn get_step_unscaled() -> f64 {
        unsafe { time_step_unscaled() }
    }

    /// How many seconds have elapsed since the last frame? 32 bit time precision, calculated at the start of the frame.
    /// This version is unaffected by the Time.Scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/StepUnscaledf.html>
    ///
    /// see also [`time_stepf_unscaled`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// assert_eq!(Time::get_step_unscaledf(), 0.0);
    ///
    /// let mut totalf = 0.0f32;
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert!((Time::get_total_unscaledf() - totalf - Time::get_step_unscaledf().abs() < 0.000001));
    ///     }
    ///     totalf = Time::get_total_unscaledf();
    /// );
    /// ```
    pub fn get_step_unscaledf() -> f32 {
        unsafe { time_stepf_unscaled() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 64 bit time precision, calculated at the start of
    /// the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Total.html>
    ///
    /// see also [`time_total`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// // Time passes faster:
    /// Time::scale(2.0);
    ///
    /// assert_eq!(Time::get_total(), 0.0);
    ///
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(Time::get_total(), Time::get_total_unscaled() * 2.0);
    ///     }
    /// );
    /// ```
    pub fn get_total() -> f64 {
        unsafe { time_total() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 32 bit time precision, calculated at the start of
    /// the frame.
    /// <https://stereokit.net/Pages/StereoKit/Time/Totalf.html>
    ///
    /// see also [`time_totalf`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::util::Time;
    ///
    /// // Time passes faster:
    /// Time::scale(0.25);
    ///
    /// assert_eq!(Time::get_totalf(), 0.0);
    ///
    /// number_of_steps = 1000;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter < number_of_steps + 2 {
    ///         assert_eq!(Time::get_totalf(), Time::get_total_unscaledf() / 4.0);
    ///     }
    /// );
    /// ```
    pub fn get_totalf() -> f32 {
        unsafe { time_totalf() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 64 bit time precision, calculated at the start of
    /// the frame. This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/TotalUnscaled.html>
    ///
    /// see also [`time_total_unscaled`]
    /// see example in [`Time::get_total`]
    pub fn get_total_unscaled() -> f64 {
        unsafe { time_total_unscaled() }
    }

    /// How many seconds have elapsed since StereoKit was initialized? 32 bit time precision, calculated at the start of
    /// the frame. This version is unaffected by the Time::scale value!
    /// <https://stereokit.net/Pages/StereoKit/Time/TotalUnscaledf.html>
    ///
    /// see also [`time_totalf_unscaled`]
    /// see example in [`Time::get_total_unscaled`]
    pub fn get_total_unscaledf() -> f32 {
        unsafe { time_totalf_unscaled() }
    }
}
