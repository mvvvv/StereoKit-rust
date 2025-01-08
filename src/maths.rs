/// Creates a text description of the Sphere, in the format of “[center:X radius:X]”
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::{
    material::Cull,
    mesh::{mesh_ray_intersect, Mesh, VindT},
    model::{model_ray_intersect, Model},
};

/// Native code use this as bool
pub type Bool32T = i32;

/// Blends (Linear Interpolation) between two scalars, based
/// on a 'blend' value, where 0 is a, and 1 is b. Doesn't clamp
/// percent for you.
/// <https://stereokit.net/Pages/SKMath/Lerp.html>
/// * a - First item in the blend, or '0.0' blend.
/// * b - Second item in the blend, or '1.0' blend.
/// * t - A blend value between 0 and 1. Can be outside   this range, it'll just interpolate outside of the a, b range.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Calculates the minimum angle 'distance' between two
/// angles. This covers wraparound cases like: the minimum distance
/// between 10 and 350 is 20.
/// <https://stereokit.net/Pages/SKMath/AngleDist.html>
/// * a - First angle, in degrees.
/// * b - Second angle, in degrees.
///   
/// returns : Degrees 0-180, the minimum angle between a and b.
pub fn angle_dist(a: f32, b: f32) -> f32 {
    let delta = (b - a + 180.0) % 360.0 - 180.0;
    (if delta < -180.0 { delta + 360.0 } else { delta }).abs()
}

pub mod units {
    /// Converts centimeters to meters. There are 100cm in 1m. In StereoKit
    /// 1 unit is also 1 meter, so `25 * Units.cm2m == 0.25`, 25 centimeters is .25
    /// meters/units.
    pub const CM2M: f32 = 0.01;
    /// Converts millimeters to meters. There are 1000mm in 1m. In StereoKit
    /// 1 unit is 1 meter, so `250 * Units.mm2m == 0.25`, 250 millimeters is .25
    /// meters/units.
    pub const MM2M: f32 = 0.001;
    ///Converts meters to centimeters. There are 100cm in 1m, so this just
    /// multiplies by 100.
    pub const M2CM: f32 = 100.0;
    ///Converts meters to millimeters. There are 1000mm in 1m, so this just
    /// multiplies by 1000.
    pub const M2MM: f32 = 1000.0;

    /// Converts centimeters to meters. There are 100cm in 1m. In StereoKit
    /// 1 unit is also 1 meter, so `25 * U.cm == 0.25`, 25 centimeters is .25
    /// meters/units.
    pub const CM: f32 = 0.01;
    /// Converts millimeters to meters. There are 1000mm in 1m. In StereoKit
    /// 1 unit is 1 meter, so `250 * Units.mm2m == 0.25`, 250 millimeters is .25
    /// meters/units.
    pub const MM: f32 = 0.001;
    /// StereoKit's default unit is meters, but sometimes it's
    /// nice to be explicit!
    pub const M: f32 = 1.0;
    /// Converts meters to kilometers. There are 1000m in 1km,
    /// so this just multiplies by 1000.
    pub const KM: f32 = 1000.0;
}

/// A vector with 2 components: x and y. This can represent a point in 2D space, a directional vector, or any other sort
/// of value with 2 dimensions to it!
/// <https://stereokit.net/Pages/StereoKit/Vec2.html>
///
/// see also [`glam::Vec2`]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl From<glam::Vec2> for Vec2 {
    fn from(val: glam::Vec2) -> Self {
        Vec2 { x: val.x, y: val.y }
    }
}

impl From<Vec2> for glam::Vec2 {
    fn from(val: Vec2) -> Self {
        Self::new(val.x, val.y)
    }
}

impl Vec2 {
    /// A Vec2 with all components at zero, this is the same as new Vec2(0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Zero.html>
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// A Vec2 with all components at one, this is the same as new Vec2(1,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/One.html>
    pub const ONE: Self = Self::new(1.0, 1.0);

    /// A normalized Vector that points down the X axis, this is the same as new Vec2(1,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/UnitX.html>
    pub const X: Self = Self::new(1.0, 0.0);

    /// A normalized Vector that points down the Y axis, this is the same as new Vec2(0,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/UnitY.html>
    pub const Y: Self = Self::new(0.0, 1.0);

    /// A normalized Vector that points up the X axis, this is the same as new Vec2(-1,0).
    pub const NEG_X: Self = Self::new(-1.0, 0.0);

    /// A normalized Vector that points up the Y axis, this is the same as new Vec2(0,-1).
    pub const NEG_Y: Self = Self::new(0.0, -1.0);

    /// A basic constructor, just copies the values in!
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Vec2.html>
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the counter-clockwise degrees from [1,0]. Resulting value is between 0 and 360. Vector does not need
    /// to be normalized.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Angle.html>
    #[inline]
    pub fn angle(&self) -> f32 {
        let mut result = self.y.atan2(self.x).to_degrees();
        if result < 0.0 {
            result += 360.0
        };
        result
    }

    /// Checks if a point is within a certain radius of this one. This is an easily readable shorthand of the squared
    /// distance check.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/InRadius.html>
    #[inline]
    pub fn in_radius(&self, point: Self, radius: f32) -> bool {
        Self::distance(*self, point) <= radius
    }

    /// Turns this vector into a normalized vector (vector with a length of 1) from the current vector. Will not work
    /// properly if the vector has a length of zero. Vec2::get_normalized is faster.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Normalize.html>
    #[inline]
    pub fn normalize(&mut self) {
        let n = *self * (self.length().recip());
        self.x = n.x;
        self.y = n.y;
    }

    /// This is the length of the vector! Or the distance from the origin to this point. Uses f32::sqrt, so it’s not
    /// dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Length.html>
    #[inline]
    pub fn length(&self) -> f32 {
        Self::dot(*self, *self).sqrt()
    }

    /// This is the squared length/magnitude of the vector! It skips the Sqrt call, and just gives you the squared
    /// version for speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/LengthSq.html>
    #[inline]
    pub fn length_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    /// Magnitude is the length of the vector! Or the distance from the origin to this point. Uses f32::sqrt, so it’s
    /// not dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Magnitude.html>
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.length()
    }

    /// This is the squared magnitude of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared. Vec2::length_squared is faster
    /// <https://stereokit.net/Pages/StereoKit/Vec2/MagnitudeSq.html>
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.length_sq()
    }

    /// Creates a normalized vector (vector with a length of 1) from the current vector. Will not work properly if the
    /// vector has a length of zero.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Normalized.html>
    #[inline]
    pub fn get_normalized(&self) -> Self {
        *self * (self.length().recip())
    }

    /// Promotes this Vec2 to a Vec3, using 0 for the Y axis.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/X0Y.html>
    #[inline]
    pub fn x0y(&self) -> Vec3 {
        Vec3 { x: self.x, y: 0.0, z: self.y }
    }

    /// Promotes this Vec2 to a Vec3, using 0 for the Z axis.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/XY0.html>
    #[inline]
    pub fn xy0(&self) -> Vec3 {
        Vec3 { x: self.x, y: self.y, z: 0.0 }
    }

    /// A transpose swizzle property, returns (y,x)
    /// <https://stereokit.net/Pages/StereoKit/Vec2/YX.html>
    #[inline]
    pub fn yx(&self) -> Self {
        Self { x: self.y, y: self.x }
    }

    /// Calculates a signed angle between two vectors in degrees! Sign will be positive if B is counter-clockwise (left)
    /// of A, and negative if B is clockwise (right) of A. Vectors do not need to be normalized. NOTE: Since this will
    /// return a positive or negative angle, order of parameters matters!
    /// <https://stereokit.net/Pages/StereoKit/Vec2/AngleBetween.html>
    #[inline]
    pub fn angle_between(a: Self, b: Self) -> f32 {
        (Self::dot(a, b) / (a.length_sq() * b.length_sq()).sqrt()).acos().to_degrees()
    }

    /// Creates a normalized delta vector that points out from an origin point to a target point!
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Direction.html>
    #[inline]
    pub fn direction(to: Self, from: Self) -> Self {
        (to - from).get_normalized()
    }

    /// Calculates the distance between two points in space! Make sure they’re in the same coordinate space! Uses a Sqrt,
    /// so it’s not blazing fast, prefer DistanceSq when possible.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Distance.html>
    #[inline]
    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    /// Calculates the distance between two points in space, but leaves them squared! Make sure they’re in the same
    /// coordinate space! This is a fast function :)
    /// <https://stereokit.net/Pages/StereoKit/Vec2/DistanceSq.html>
    #[inline]
    pub fn distance_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    /// The dot product is an extremely useful operation! One major use is to determine how similar two vectors are.
    /// If the vectors are Unit vectors (magnitude/length of 1), then the result will be 1 if the vectors are the same,
    /// -1 if they’re opposite, and a gradient in-between with 0 being perpendicular. See [Freya Holmer’s excellent
    /// visualization of this concept](<https://twitter.com/FreyaHolmer/status/1200807790580768768>)
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Dot.html>
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y)
    }

    /// Creates a vector pointing in the direction of the angle, with a length of 1. Angles are counter-clockwise, and
    /// start from (1,0), so an angle of 90 will be (0,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/FromAngle.html>
    #[inline]
    pub fn from_angles(degree: f32) -> Self {
        Self { x: degree.to_radians().cos(), y: degree.to_radians().sin() }
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b.
    /// Doesn’t clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Lerp.html>
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each elements is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Max.html>
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y) }
    }

    /// Returns a vector where each elements is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Min.html>
    #[inline]
    pub fn min(a: Self, b: Self) -> Self {
        Self { x: f32::min(a.x, b.x), y: f32::min(a.y, b.y) }
    }

    /// Absolute value of each component, this may be usefull in some case
    #[inline]
    pub fn abs(&self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs() }
    }
}

impl Display for Vec2 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks like “[x, y]”
    /// <https://stereokit.net/Pages/StereoKit/Vec2/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}]", self.x, self.y)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl Div<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl DivAssign<Vec2> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl Div<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Self::Output {
        Vec2 { x: self.div(rhs.x), y: self.div(rhs.y) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl Mul<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl MulAssign<Vec2> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 { x: self.mul(rhs.x), y: self.mul(rhs.y) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Addition.html>
impl Add<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Addition.html>
impl AddAssign<Vec2> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Subtraction.html>
impl Sub<Vec2> for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Subtraction.html>
impl SubAssign<Vec2> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec2) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
    }
}

/// Vector negation, returns a vector where each component has been negated.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_UnaryNegation.html>
impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

/// A vector with 3 components: x, y, z. This can represent a point in space, a directional vector, or any other sort of
/// value with 3 dimensions to it!
///
/// StereoKit uses a right-handed coordinate system, where +x is to the right, +y is upwards, and -z is forward.
/// <https://stereokit.net/Pages/StereoKit/Vec3.html>
///
/// see also [`glam::Vec3`]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<glam::Vec3> for Vec3 {
    fn from(val: glam::Vec3) -> Self {
        Vec3 { x: val.x, y: val.y, z: val.z }
    }
}

impl From<Vec3> for glam::Vec3 {
    fn from(val: Vec3) -> Self {
        Self::new(val.x, val.y, val.z)
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(val: [f32; 3]) -> Self {
        Vec3 { x: val[0], y: val[1], z: val[2] }
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(val: Vec3) -> Self {
        [val.x, val.y, val.z]
    }
}

extern "C" {
    pub fn vec3_cross(a: *const Vec3, b: *const Vec3) -> Vec3;
}

impl Vec3 {
    /// StereoKit uses a right-handed coordinate system, which means that forward is looking down the -Z axis! This
    /// value is the same as new Vec3(0,0,-1). This is NOT the same as UnitZ!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Forward.html>
    pub const FORWARD: Self = Self::NEG_Z;

    /// Shorthand for a vector where all values are 1! Same as new Vec3(1,1,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec3/One.html>
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    /// Shorthand for a vector where all values are -1! Same as new Vec3(-1,-1,-1).
    pub const NEG_ONE: Self = Self::new(-1.0, -1.0, -1.0);

    /// When looking forward, this is the direction to the right! In StereoKit, this is the same as new Vec3(1,0,0)
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Right.html>
    pub const RIGHT: Self = Self::X;

    /// A normalized Vector that points down the X axis, this is the same as new Vec3(1,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec3/UnitX.html>
    pub const X: Self = Self::new(1.0, 0.0, 0.0);

    /// A normalized Vector that points down the Y axis, this is the same as new Vec3(0,1,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec3/UnitY.html>
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);

    /// A normalized Vector that points down the Z axis, this is the same as new Vec3(0,0,1).
    /// This is NOT the same as Forward!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/UnitZ.html>
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    ///  A normalized Vector that points up the Z axis, this is the same as new Vec3(0,0,-1).
    pub const NEG_X: Self = Self::new(-1.0, 0.0, 0.0);

    /// A normalized Vector that points up the Y axis, this is the same as new Vec3(0,-1,0).
    pub const NEG_Y: Self = Self::new(0.0, -1.0, 0.0);

    /// A normalized Vector that points up the Z axis, this is the same as new Vec3(0,0,-1).
    /// This is the same as Forward!
    pub const NEG_Z: Self = Self::new(0.0, 0.0, -1.0);

    /// A vector representing the up axis. In StereoKit, this is the same as new Vec3(0,1,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Up.html>
    pub const UP: Self = Self::Y;

    /// Shorthand for a vector where all values are 0! Same as new Vec3(0,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Zero.html>
    pub const ZERO: Vec3 = Self::new(0.0, 0.0, 0.0);

    /// Creates a vector from x, y, and z values! StereoKit uses a right-handed metric coordinate system, where +x is to
    /// the right, +y is upwards, and -z is forward.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Vec3.html>
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Checks if a point is within a certain radius of this one. This is an easily readable shorthand of the squared
    /// distance check.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/InRadius.html>
    #[inline]
    pub fn in_radius(&self, point: Self, radius: f32) -> bool {
        Self::distance(*self, point) <= radius
    }

    /// Turns this vector into a normalized vector (vector with a length of 1) from the current vector. Will not work
    /// properly if the vector has a length of zero. Vec3::get_normalized is faster.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Normalize.html>
    #[inline]
    pub fn normalize(&mut self) {
        let n = *self * (self.length().recip());
        self.x = n.x;
        self.y = n.y;
        self.z = n.z;
    }

    /// This is the length, or magnitude of the vector! The distance from the origin to this point.
    /// Uses f32::sqrt, so it’s not dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Length.html>
    #[inline]
    pub fn length(&self) -> f32 {
        Self::dot(*self, *self).sqrt()
    }

    /// This is the squared length of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/LengthSq.html>
    #[inline]
    pub fn length_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    /// Magnitude is the length of the vector! The distance from the origin to this point. Uses f32::sqrt, so it’s not
    /// dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Magnitude.html>
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.length()
    }

    /// This is the squared magnitude of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/MagnitudeSq.html>
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.length_sq()
    }

    /// Creates a normalized vector (vector with a length of 1) from the current vector. Will not work properly if the
    /// vector has a length of zero.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Normalized.html>
    #[inline]
    pub fn get_normalized(&self) -> Self {
        *self * (self.length().recip())
    }

    /// This returns a Vec3 that has been flattened to 0 on the Y axis. No other changes are made.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/X0Z.html>
    #[inline]
    pub fn x0z(&self) -> Self {
        Self { x: self.x, y: 0.0, z: self.z }
    }

    /// This returns a Vec3 that has been flattened to 0 on the Z axis. No other changes are made.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/XY0.html>
    #[inline]
    pub fn xy0(&self) -> Self {
        Self { x: self.x, y: self.y, z: 0.0 }
    }

    /// This returns a Vec3 that has been set to 1 on the Z axis. No other changes are made
    /// <https://stereokit.net/Pages/StereoKit/Vec3/XY1.html>
    #[inline]
    pub fn xy1(&self) -> Self {
        Self { x: self.x, y: self.y, z: 1.0 }
    }

    /// This extracts the Vec2 from the X and Y axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3.html>
    #[inline]
    pub fn xy(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// This extracts the Vec2 from the Y and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/YZ.html>
    #[inline]
    pub fn yz(&self) -> Vec2 {
        Vec2::new(self.y, self.z)
    }

    /// This extracts the Vec2 from the X and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3.html>
    #[inline]
    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }

    /// Calculates the angle between two vectors in degrees! Vectors do not need to be normalized, and the result will
    /// always be positive.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleBetween.html>
    #[inline]
    pub fn angle_between(a: Self, b: Self) -> f32 {
        (Self::dot(a, b) / (a.length_sq() * b.length_sq()).sqrt()).acos().to_degrees()
    }

    /// Creates a vector that points out at the given 2D angle! This creates the vector on the XY plane, and allows you
    /// to specify a constant z value.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleXY.html>
    #[inline]
    pub fn angle_xy(angle_deg: f32, z: f32) -> Vec3 {
        Self { x: angle_deg.to_radians().cos(), y: angle_deg.to_radians().sin(), z }
    }

    /// Creates a vector that points out at the given 2D angle! This creates the vector on the XZ plane, and allows you
    /// to specify a constant y value.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleXZ.html>
    #[inline]
    pub fn angle_xz(angle_deg: f32, y: f32) -> Self {
        Self { x: angle_deg.to_radians().cos(), y, z: angle_deg.to_radians().sin() }
    }

    /// The cross product of two vectors!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Cross.html>
    ///
    /// see also [`crate::maths::vec3_cross`]
    #[inline]
    pub fn cross(a: Self, b: Self) -> Self {
        unsafe { vec3_cross(&a, &b) }
    }

    /// Creates a normalized delta vector that points out from an origin point to a target point!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Direction.html>
    #[inline]
    pub fn direction(to: Self, from: Self) -> Self {
        (to - from).get_normalized()
    }

    /// Calculates the distance between two points in space! Make sure they’re in the same coordinate space! Uses a
    /// Sqrt, so it’s not blazing fast, prefer DistanceSq when possible.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Distance.html>
    #[inline]
    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    /// Calculates the distance between two points in space, but leaves them squared! Make sure they’re in the same
    /// coordinate space! This is a fast function :)
    /// <https://stereokit.net/Pages/StereoKit/Vec3/DistanceSq.html>
    #[inline]
    pub fn distance_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    /// The dot product is an extremely useful operation! One major use is to determine how similar two vectors are. If
    /// the vectors are Unit vectors (magnitude/length of 1), then the result will be 1 if the vectors are the same, -1
    /// if they’re opposite, and a gradient in-between with 0 being perpendicular.  See [Freya Holmer’s excellent
    /// visualization of this concept](<https://twitter.com/FreyaHolmer/status/1200807790580768768>)
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Dot.html>
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z)
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b. Doesn’t
    /// clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Lerp.html>
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each elements is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Max.html>
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y), z: f32::max(a.z, b.z) }
    }

    /// Returns a vector where each elements is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Min.html>
    #[inline]
    pub fn min(a: Self, b: Self) -> Self {
        Self { x: f32::min(a.x, b.x), y: f32::min(a.y, b.y), z: f32::min(a.z, b.z) }
    }

    /// Exactly the same as Vec3.Cross, but has some naming mnemonics for getting the order right when trying to find a
    /// perpendicular vector using the cross product. This’ll also make it more obvious to read if that’s what you’re
    /// actually going for when crossing vectors!
    /// If you consider a forward vector and an up vector, then the direction to the right is pretty trivial to imagine
    /// in relation to those vectors!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/PerpendicularRight.html>
    #[inline]
    pub fn perpendicular_right(forward: Self, up: Self) -> Self {
        Self::cross(forward, up)
    }

    /// Absolute value of each component, this may be usefull in some case
    #[inline]
    pub fn abs(&self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }

    /// get an array
    #[inline]
    pub const fn to_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Display for Vec3 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode.
    /// Looks like “[x, y, z]”
    /// <https://stereokit.net/Pages/StereoKit/Vec3/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}]", self.x, self.y, self.z)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl Div<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y), z: self.z.div(rhs.z) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl DivAssign<Vec3> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
        self.z.div_assign(rhs.z);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs), z: self.z.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
        self.z.div_assign(rhs);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl Div<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.div(rhs.x), y: self.div(rhs.y), z: self.div(rhs.z) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl Mul<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y), z: self.z.mul(rhs.z) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl MulAssign<Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
        self.z.mul_assign(rhs.z);
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs), z: self.z.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
        self.z.mul_assign(rhs);
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.mul(rhs.x), y: self.mul(rhs.y), z: self.mul(rhs.z) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Addition.html>
impl Add<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y), z: self.z.add(rhs.z) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Addition.html>
impl AddAssign<Vec3> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
        self.z.add_assign(rhs.z);
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y), z: self.z.sub(rhs.z) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Subtraction.html>
impl SubAssign<Vec3> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec3) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
        self.z.sub_assign(rhs.z);
    }
}

/// Vector negation, returns a vector where each component has been negated.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_UnaryNegation.html>
impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

/// A vector with 4 components: x, y, z, and w. Can be useful for things like shaders, where the registers are aligned
/// to 4 float vectors.
///
/// This is a wrapper on System.Numerics.Vector4, so it’s SIMD optimized, and can be cast to and from implicitly.
/// <https://stereokit.net/Pages/StereoKit/Vec4.html>
///
/// see also [`glam::Vec4`]
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
impl From<glam::Vec4> for Vec4 {
    fn from(value: glam::Vec4) -> Self {
        Self { x: value.x, y: value.y, z: value.z, w: value.w }
    }
}

impl From<Vec4> for glam::Vec4 {
    fn from(value: Vec4) -> Self {
        Self::new(value.x, value.y, value.z, value.w)
    }
}

impl Vec4 {
    /// all components to 0
    pub const ZERO: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points down the X axis, this is the same as new Vec4(1,0,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitX.html>    
    pub const X: Vec4 = Vec4 { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points down the Y axis, this is the same as new Vec4(0,1,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitX.html>    
    pub const Y: Vec4 = Vec4 { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points down the 2 axis, this is the same as new Vec4(0,0,1,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitX.html>    
    pub const Z: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };

    /// A normalized Vector that points down the W axis, this is the same as new Vec4(0,0,0,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitW.html>
    pub const W: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// <https://stereokit.net/Pages/StereoKit/Vec4/Vec4.html>
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// This extracts the Vec2 from the X and Y axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XY.html>
    #[inline]
    pub fn xy(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// This extracts the Vec2 from the Y and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/YZ.html>
    #[inline]
    pub fn yz(&self) -> Vec2 {
        Vec2::new(self.y, self.z)
    }

    /// This extracts the Vec2 from the X and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XZ.html>
    #[inline]
    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }

    /// This extracts the Vec2 from the Z and W axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/ZW.html>
    #[inline]
    pub fn zw(&self) -> Vec2 {
        Vec2::new(self.z, self.w)
    }

    /// This extracts a Vec3 from the X, Y, and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XYZ.html>
    #[inline]
    pub fn xyz(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// A Vec4 and a Quat are only really different by name and purpose. So, if you need to do Quat math with your
    /// Vec4, or visa versa, who am I to judge?
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Quat.html>
    ///
    /// see also [`crate::maths::matrix_extract_translation`]
    #[inline]
    pub fn get_as_quat(&self) -> Quat {
        Quat { x: self.x, y: self.y, z: self.z, w: self.w }
    }

    /// What’s a dot product do for 4D vectors, you might ask? Well, I’m no mathematician, so hopefully you are! I’ve
    /// never used it before. Whatever you’re doing with this function, it’s SIMD fast!
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Dot.html>
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z) + (a.w * b.w)
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b. Doesn’t
    /// clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Lerp.html>
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each elements is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Max.html>
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y), z: f32::max(a.z, b.z), w: f32::max(a.w, b.w) }
    }

    /// Returns a vector where each elements is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Min.html>
    #[inline]
    pub fn min(a: Self, b: Self) -> Self {
        Self { x: f32::min(a.x, b.x), y: f32::min(a.y, b.y), z: f32::min(a.z, b.z), w: f32::min(a.w, b.w) }
    }
}

impl Display for Vec4 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks
    /// like “[x, y, z, w]”
    /// <https://stereokit.net/Pages/StereoKit/Bounds/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}, w:{}]", self.x, self.y, self.z, self.w)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl Div<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y), z: self.z.div(rhs.z), w: self.w.div(rhs.w) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl DivAssign<Vec4> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
        self.z.div_assign(rhs.z);
        self.w.div_assign(rhs.w);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs), z: self.z.div(rhs), w: self.w.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl DivAssign<f32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
        self.z.div_assign(rhs);
        self.w.div_assign(rhs);
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl Div<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn div(self, rhs: Vec4) -> Self::Output {
        Vec4 { x: self.div(rhs.x), y: self.div(rhs.y), z: self.div(rhs.z), w: self.div(rhs.w) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl Mul<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y), z: self.z.mul(rhs.z), w: self.w.mul(rhs.w) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl MulAssign<Vec4> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
        self.z.mul_assign(rhs.z);
        self.w.mul_assign(rhs.w)
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs), z: self.z.mul(rhs), w: self.w.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
        self.z.mul_assign(rhs);
        self.w.mul_assign(rhs);
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Self::Output {
        Vec4 { x: self.mul(rhs.x), y: self.mul(rhs.y), z: self.mul(rhs.z), w: self.mul(rhs.w) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Addition.html>
impl Add<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y), z: self.z.add(rhs.z), w: self.w.add(rhs.w) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Addition.html>
impl AddAssign<Vec4> for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
        self.z.add_assign(rhs.z);
        self.w.add_assign(rhs.w);
    }
}

impl Sub<Vec4> for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y), z: self.z.sub(rhs.z), w: self.w.sub(rhs.w) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Subtraction.html>
impl SubAssign<Vec4> for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec4) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
        self.z.sub_assign(rhs.z);
        self.w.sub_assign(rhs.w);
    }
}

/// Vector negation, returns a vector where each component has been negated.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_UnaryNegation.html>
impl Neg for Vec4 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

/// Native code use this as Quat
/// Quaternions are efficient and robust mathematical objects for representing rotations! Understanding the details of
/// how a quaternion works is not generally necessary for using them effectively, so don’t worry too much if they seem
/// weird to you. They’re weird to me too.
///
/// If you’re interested in learning the details though, 3Blue1Brown and Ben Eater have an excellent interactive lesson
/// about them!
/// <https://stereokit.net/Pages/StereoKit/Quat.html>
///
///  see also [`glam::Quat`]
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
impl From<glam::Quat> for Quat {
    fn from(val: glam::Quat) -> Self {
        Quat { x: val.x, y: val.y, z: val.z, w: val.w }
    }
}
impl From<Quat> for glam::Quat {
    fn from(val: Quat) -> Self {
        Self::from_xyzw(val.x, val.y, val.z, val.w)
    }
}
impl From<glam::Vec4> for Quat {
    fn from(val: glam::Vec4) -> Self {
        Quat { x: val.x, y: val.y, z: val.z, w: val.w }
    }
}
impl From<Quat> for glam::Vec4 {
    fn from(val: Quat) -> Self {
        Self::new(val.x, val.y, val.z, val.w)
    }
}
impl From<Vec4> for Quat {
    fn from(val: Vec4) -> Self {
        Quat { x: val.x, y: val.y, z: val.z, w: val.w }
    }
}
impl From<Quat> for Vec4 {
    fn from(val: Quat) -> Self {
        Self { x: val.x, y: val.y, z: val.z, w: val.w }
    }
}

extern "C" {
    pub fn quat_difference(a: *const Quat, b: *const Quat) -> Quat;
    pub fn quat_lookat(from: *const Vec3, at: *const Vec3) -> Quat;
    pub fn quat_lookat_up(from: *const Vec3, at: *const Vec3, up: *const Vec3) -> Quat;
    pub fn quat_from_angles(pitch_x_deg: f32, yaw_y_deg: f32, roll_z_deg: f32) -> Quat;
    pub fn quat_slerp(a: *const Quat, b: *const Quat, t: f32) -> Quat;
    pub fn quat_normalize(a: *const Quat) -> Quat;
    pub fn quat_inverse(a: *const Quat) -> Quat;
    pub fn quat_mul(a: *const Quat, b: *const Quat) -> Quat;
    pub fn quat_mul_vec(a: *const Quat, b: *const Vec3) -> Vec3;
    pub fn quat_to_axis_angle(a: Quat, out_axis: *mut Vec3, out_rotation_deg: *mut f32);
}

impl Quat {
    /// This is the ‘multiply by one!’ of the quaternion rotation world. It’s basically a default, no rotation
    /// quaternion.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Identity.html>
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// You may want to use static creation methods, like Quat.LookAt, or Quat.Identity instead of this one! Unless you
    /// know what you’re doing.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Quat.html>
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Makes this Quat the reverse rotation! If this quat goes from A to B, the inverse will go from B to A.
    /// Costly, see get_inverse for a faster way to get this.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Invert.html>
    ///
    /// see also [`QuatT::get_inverse`] [`crate::maths::quat_inverse`]
    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let m = unsafe { quat_inverse(self) };
        self.x = m.x;
        self.y = m.y;
        self.z = m.z;
        self.w = m.w;
        self
    }

    /// Normalize this quaternion with the same orientation, and a length of 1.
    /// Costly, see get_normalized for a faster way to get this.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Normalize.html>
    ///
    /// see also [`QuatT::get_normalized`] [`crate::maths::quat_normalize`]
    #[inline]
    pub fn normalize(&mut self) -> &mut Self {
        let m = unsafe { quat_normalize(self) };
        self.x = m.x;
        self.y = m.y;
        self.z = m.z;
        self.w = m.w;
        self
    }

    /// Rotates a quaternion making it relative to another rotation while preserving it’s “Length”!
    /// <https://stereokit.net/Pages/StereoKit/Quat/Relative.html>
    ///
    /// see also [`crate::maths::quat_mul`]
    #[inline]
    pub fn relative(&mut self, to: Self) -> &mut Self {
        let m = to.mul(*self).mul(to.get_inverse());
        self.x = m.x;
        self.y = m.y;
        self.z = m.z;
        self.w = m.w;
        self
    }

    /// This rotates a point around the origin by the Quat.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Rotate.html>
    ///
    /// see also [`crate::maths::quat_mul_vec`]
    #[inline]
    pub fn rotate_point(&self, point: Vec3) -> Vec3 {
        unsafe { quat_mul_vec(self, &point) }
    }

    /// This rotates a point around the origin by the Quat.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Rotate.html>
    ///
    /// see also [`crate::maths::quat_mul_vec`]
    #[inline]
    pub fn rotate(a: Self, point: Vec3) -> Vec3 {
        unsafe { quat_mul_vec(&a, &point) }
    }

    /// Creates a quaternion that goes from one rotation to another.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Delta.html>
    /// see also - operator
    ///
    /// see also [`crate::maths::quat_difference`]
    #[inline]
    pub fn delta(from: Self, to: Self) -> Self {
        unsafe { quat_difference(&from, &to) }
    }

    /// Creates a rotation that goes from one direction to another. Which is comes in handy when trying to roll
    /// something around with position data.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Delta.html>
    #[inline]
    pub fn delta_dir(from: Vec3, to: Vec3) -> Self {
        let c = Vec3::cross(from, to);
        let mut out = Quat { x: c.x, y: c.y, z: c.z, w: 1.0 + Vec3::dot(from, to) };
        *(out.normalize())
    }

    /// Creates a Roll/Pitch/Yaw rotation (applied in that order) from the provided angles in degrees!
    /// <https://stereokit.net/Pages/StereoKit/Quat/FromAngles.html>
    ///
    /// see also [`crate::maths::quat_from_angles`]
    #[inline]
    pub fn from_angles(pitch_x_deg: f32, yaw_y_deg: f32, roll_z_deg: f32) -> Self {
        unsafe { quat_from_angles(pitch_x_deg, yaw_y_deg, roll_z_deg) }
    }

    /// Creates a rotation that describes looking from a point, to another point! This is a great function for camera
    /// style rotation, or other facing behavior when you know where an object is, and where you want it to look at.
    /// This rotation works best when applied to objects that face Vec3.Forward in their resting/model space pose.
    /// <https://stereokit.net/Pages/StereoKit/Quat/LookAt.html>
    /// * up - Look From/At positions describe X and Y axis rotation well, but leave Z Axis/Roll
    ///        undefined. Providing an upDirection vector helps to indicate roll around the From/At line. If None : up
    ///        direction will be (0,1,0), to prevent roll.    
    ///
    /// see also [`crate::maths::quat_lookat`][`crate::maths::quat_lookat_up`]
    #[inline]
    pub fn look_at(from: Vec3, at: Vec3, up: Option<Vec3>) -> Self {
        match up {
            Some(up) => unsafe { quat_lookat_up(&from, &at, &up) },
            None => unsafe { quat_lookat(&from, &at) },
        }
    }

    /// Creates a rotation that describes looking towards a direction. This is great for quickly describing facing
    /// behavior! This rotation works best when applied to objects that face Vec3.Forward in their resting/model space
    /// pose.
    /// <https://stereokit.net/Pages/StereoKit/Quat/LookDir.html>
    ///
    /// see also [`crate::maths::quat_lookat`]
    #[inline]
    pub fn look_dir(direction: Vec3) -> Self {
        unsafe { quat_lookat(&Vec3::ZERO, &direction) }
    }

    /// Spherical Linear interpolation. Interpolates between two quaternions! Both Quats should be normalized/unit
    /// quaternions, or you may get unexpected results.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Slerp.html>
    ///
    /// see also [`crate::maths::quat_slerp`]
    #[inline]
    pub fn slerp(a: Self, b: Self, slerp: f32) -> Self {
        unsafe { quat_slerp(&a, &b, slerp) }
    }

    /// The reverse rotation! If this quat goes from A to B, the inverse will go from B to A.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Inverse.html>
    ///
    /// see also [`crate::maths::quat_inverse`]
    #[inline]
    pub fn get_inverse(&self) -> Self {
        unsafe { quat_inverse(self) }
    }

    /// A normalized quaternion has the same orientation, and a length of 1.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Normalized.html>
    ///
    /// see also [`crate::maths::quat_normalize`]
    #[inline]
    pub fn get_normalized(&self) -> Self {
        unsafe { quat_normalize(self) }
    }

    /// A Vec4 and a Quat are only really different by name and purpose. So, if you need to do Quat math with your
    /// Vec4, or visa versa, who am I to judge?
    /// <https://stereokit.net/Pages/StereoKit/Quat/Vec4.html>
    #[inline]
    pub fn get_as_vec4(&self) -> Vec4 {
        Vec4 { x: self.x, y: self.y, z: self.z, w: self.w }
    }

    /// This is the combination of rotations a and b. Note that order matters h
    /// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
    /// see also * operator
    ///
    /// see also [`crate::maths::quat_mul`]
    #[inline]
    pub fn mul(&self, rhs: &Self) -> Self {
        unsafe { quat_mul(self, rhs) }
    }

    /// This rotates a point around the origin by the Quat.
    /// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
    /// see also * operator
    ///
    /// see also [`crate::maths::quat_mul_vec`]
    #[inline]
    pub fn mul_vec3(&self, rhs: Vec3) -> Vec3 {
        unsafe { quat_mul_vec(self, &rhs) }
    }

    /// get an array
    #[inline]
    pub const fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

impl Display for Quat {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks
    /// like “[x, y, z, w]”
    /// <https://stereokit.net/Pages/StereoKit/Quat/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}, w:{}]", self.x, self.y, self.z, self.w)
    }
}

/// This is the combination of rotations a and b. Note that order matters h
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`crate::maths::quat_mul`]
impl Mul<Quat> for Quat {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { quat_mul(&self, &rhs) }
    }
}

/// This is the combination of rotations a and b. Note that order matters h
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`crate::maths::quat_mul`]
impl MulAssign<Quat> for Quat {
    #[inline]
    fn mul_assign(&mut self, rhs: Quat) {
        *self = unsafe { quat_mul(self, &rhs) }
    }
}

/// This rotates a point around the origin by the Quat.
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`crate::maths::quat_mul_vec`]
impl Mul<Vec3> for Quat {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        unsafe { quat_mul_vec(&self, &rhs) }
    }
}

/// Gets a Quat representing the rotation from a to b.
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Subtraction.html>
/// see also QuatT::delta()
///
/// see also [`crate::maths::quat_difference`]
impl Sub<Quat> for Quat {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        unsafe { quat_difference(&self, &rhs) }
    }
}

/// A Matrix in StereoKit is a 4x4 grid of numbers that is used to represent a transformation for any sort of position
/// or vector! This is an oversimplification of what a matrix actually is, but it’s accurate in this case.
///
/// Matrices are really useful for transforms because you can chain together all sorts of transforms into a single
/// Matrix! A Matrix transform really shines when applied to many positions, as the more expensive operations get cached
/// within the matrix values.
///
/// Multiple matrix transforms can be combined by multiplying them. In StereoKit, to create a matrix that first scales
/// an object, followed by rotating it, and finally translating it you would use this order:
/// Matrix M = Matrix.S(...) * Matrix.R(...) * Matrix.T(...);
///
/// This order is related to the fact that StereoKit uses row-major order to store matrices. Note that in other 3D
/// frameworks and certain 3D math references you may find column-major matrices, which would need the reverse order
/// (i.e. TRS), so please keep this in mind when creating transformations.
///
/// Matrices are prominently used within shaders for mesh transforms!
/// see also [`glam::Mat4`]
/// <https://stereokit.net/Pages/StereoKit/Matrix.html>
#[repr(C)]
#[derive(Copy, Clone)]
pub union Matrix {
    pub row: [Vec4; 4usize],
    pub m: [f32; 16usize],
}

impl From<glam::Mat4> for Matrix {
    fn from(m: glam::Mat4) -> Self {
        Matrix { row: [m.x_axis.into(), m.y_axis.into(), m.z_axis.into(), m.w_axis.into()] }
    }
}

impl std::fmt::Debug for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

extern "C" {
    pub fn pose_matrix_out(pose: *const Pose, out_result: *mut Matrix, scale: Vec3);
    pub fn matrix_inverse(a: *const Matrix, out_Matrix: *mut Matrix);
    pub fn matrix_invert(a: *const Matrix) -> Matrix;
    pub fn matrix_mul(a: *const Matrix, b: *const Matrix, out_Matrix: *mut Matrix);
    // Deprecated: pub fn matrix_mul_point(transform: *const Matrix, point: *const Vec3) -> Vec3;
    // Deprecated: pub fn matrix_mul_point4(transform: *const Matrix, point: *const Vec4) -> Vec4;
    pub fn matrix_mul_direction(transform: *const Matrix, direction: *const Vec3) -> Vec3;
    // Deprecated: pub fn matrix_mul_rotation(transform: *const Matrix, orientation: *const Quat) -> Quat;
    // Deprecated: pub fn matrix_mul_pose(transform: *const Matrix, pose: *const Pose) -> Pose;
    pub fn matrix_transform_pt(transform: Matrix, point: Vec3) -> Vec3;
    pub fn matrix_transform_pt4(transform: Matrix, point: Vec4) -> Vec4;
    pub fn matrix_transform_dir(transform: Matrix, direction: Vec3) -> Vec3;
    pub fn matrix_transform_ray(transform: Matrix, ray: Ray) -> Ray;
    pub fn matrix_transform_quat(transform: Matrix, rotation: Quat) -> Quat;
    pub fn matrix_transform_pose(transform: Matrix, pose: Pose) -> Pose;
    pub fn matrix_transpose(transform: Matrix) -> Matrix;
    pub fn matrix_to_angles(transform: *const Matrix) -> Vec3;
    pub fn matrix_trs(position: *const Vec3, orientation: *const Quat, scale: *const Vec3) -> Matrix;
    pub fn matrix_t(position: Vec3) -> Matrix;
    pub fn matrix_r(orientation: Quat) -> Matrix;
    pub fn matrix_s(scale: Vec3) -> Matrix;
    pub fn matrix_ts(position: Vec3, scale: Vec3) -> Matrix;
    pub fn matrix_trs_out(out_result: *mut Matrix, position: *const Vec3, orientation: *const Quat, scale: *const Vec3);
    pub fn matrix_perspective(fov_degrees: f32, aspect_ratio: f32, near_clip: f32, far_clip: f32) -> Matrix;
    pub fn matrix_orthographic(width: f32, height: f32, near_clip: f32, far_clip: f32) -> Matrix;
    pub fn matrix_decompose(
        transform: *const Matrix,
        out_position: *mut Vec3,
        out_scale: *mut Vec3,
        out_orientation: *mut Quat,
    ) -> Bool32T;
    pub fn matrix_extract_translation(transform: *const Matrix) -> Vec3;
    pub fn matrix_extract_scale(transform: *const Matrix) -> Vec3;
    pub fn matrix_extract_rotation(transform: *const Matrix) -> Quat;
    pub fn matrix_extract_pose(transform: *const Matrix) -> Pose;
}

impl Matrix {
    /// Identity matrix made of [[Vec4T::X, Vec4T::Y, Vec4T::Z, Vec4T::W]]
    pub const IDENTITY: Matrix = Matrix { row: [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W] };

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. Orthographic
    /// projection matrices will preserve parallel lines. This is great for 2D scenes or content.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Orthographic.html>
    /// * width - in meters, of the area that will  be projected.
    /// * height - The height, in meters, of the area that will be projected
    /// * near_clip - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * far_clip - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.    
    ///
    /// Returns the final orthographic Matrix.
    /// see also [`crate::maths::matrix_orthographic`]
    #[inline]
    pub fn ortographic(width: f32, height: f32, near_clip: f32, far_clip: f32) -> Self {
        unsafe { matrix_orthographic(width, height, near_clip, far_clip) }
    }

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. Perspective
    /// projection matrices will cause parallel lines to converge at the horizon. This is great for normal looking
    /// content.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Perspective.html>
    /// * fov_degrees - This is the vertical field of view of the perspective matrix, units are in degrees.
    /// * aspect_ratio - The projection surface's width/height.
    /// * near_clip - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * far_clip - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.    
    ///
    /// Returns the final perspective matrix.
    /// see also [`crate::maths::matrix_perspective`]
    #[inline]
    pub fn perspective(fov_degrees: f32, aspect_ratio: f32, near_clip: f32, far_clip: f32) -> Self {
        unsafe { matrix_perspective(fov_degrees.to_radians(), aspect_ratio, near_clip, far_clip) }
    }

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. With the known camera
    /// intrinsics, you can replicate its perspective!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Perspective.html>
    /// * image_resolution - The resolution of the image. This should be the image's width and height in pixels.
    /// * focal_length_px - The focal length of camera in pixels, with image coordinates +X (pointing right) and +Y
    ///   (pointing up).
    /// * near_clip - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * far_clip - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.
    ///
    /// Returns the final perspective matrix.
    /// Remarks: Think of the optical axis as an imaginary line that passes through the camera lens. In front of the
    /// camera lens, there's an image plane, perpendicular to the optical axis, where the image of the scene being
    /// captured is formed. Its distance is equal to the focal length of the camera from the center of the lens. Here,
    /// we find the ratio between the size of the image plane and distance from the camera in one unit distance and
    /// multiply it by the near clip distance to find a near plane that is parallel.
    ///
    /// see also [`crate::maths::matrix_perspective`]
    pub fn perspective_focal(image_resolution: Vec2, focal_length_px: f32, near_clip: f32, far_clip: f32) -> Self {
        let near_plane_dimensions = image_resolution / focal_length_px * near_clip;

        let two_near = near_clip + near_clip;
        let f_range = far_clip / (near_clip - far_clip);

        Self {
            m: [
                two_near / near_plane_dimensions.x,
                0.0,
                0.0,
                0.0,
                //
                0.0,
                two_near / near_plane_dimensions.y,
                0.0,
                0.0,
                //
                0.0,
                0.0,
                f_range,
                -1.0,
                //
                0.0,
                0.0,
                f_range * near_clip,
                0.0,
            ],
        }
    }

    /// <https://stereokit.net/Pages/StereoKit/Matrix/LookAt.html>
    ///
    /// see also [`crate::maths::matrix_r`]
    #[inline]
    pub fn look_at(from: Vec3, at: Vec3, up: Option<Vec3>) -> Self {
        let up = up.unwrap_or(Vec3::UP);
        let forward = (from - at).get_normalized();
        let right_v = Vec3::perpendicular_right(forward, up).get_normalized();
        let up_v = Vec3::perpendicular_right(right_v, forward).get_normalized();
        Self {
            m: [
                right_v.x,
                up_v.x,
                forward.x,
                0.0,
                //
                right_v.y,
                up_v.y,
                forward.y,
                0.0,
                //
                right_v.z,
                up_v.z,
                forward.z,
                0.0,
                //
                -Vec3::dot(from, right_v),
                -Vec3::dot(from, up_v),
                -Vec3::dot(from, forward),
                1.0,
            ],
        }

        // Self::from_cols(
        //     Vec4::new(right_v.x, up_v.x, -f.x, 0.0),
        //     Vec4::new(right_v.y, up_v.y, -f.y, 0.0),
        //     Vec4::new(right_v.z, up_v.z, -f.z, 0.0),
        //     Vec4::new(-eye.dot(right_v), -eye.dot(up_v), eye.dot(f), 1.0),
        // )
    }

    /// Create a rotation matrix from a Quaternion.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/R.html>
    ///
    /// see also [`crate::maths::matrix_r`]
    #[inline]
    pub fn r(rotation: Quat) -> Self {
        unsafe { matrix_r(rotation) }
    }

    /// Creates a scaling Matrix, where scale can be different on each axis (non-uniform).
    /// <https://stereokit.net/Pages/StereoKit/Matrix/S.html>
    ///
    /// see also [`crate::maths::matrix_s`]
    #[inline]
    pub fn s(scale: Vec3) -> Self {
        unsafe { matrix_s(scale) }
    }

    /// Translate. Creates a translation Matrix!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/T.html>
    ///
    /// see also [`crate::maths::matrix_t`]
    #[inline]
    pub fn t(translation: Vec3) -> Self {
        unsafe { matrix_t(translation) }
    }

    /// Translate, Rotate. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TR.html>
    ///
    /// see also [`crate::maths::matrix_tr`]
    #[inline]
    pub fn tr(translation: &Vec3, rotation: &Quat) -> Self {
        unsafe { matrix_trs(translation, rotation, &Vec3::ONE) }
    }

    /// Translate, Scale. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TS.html>
    ///
    /// see also [`crate::maths::matrix_ts`]
    #[inline]
    pub fn ts(translation: Vec3, scale: Vec3) -> Self {
        unsafe { matrix_ts(translation, scale) }
    }

    /// Translate, Rotate, Scale. Creates a transform Matrix using all these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    ///
    /// see also [`crate::maths::matrix_trs`]
    #[inline]
    pub fn trs(translation: &Vec3, rotation: &Quat, scale: &Vec3) -> Self {
        unsafe { matrix_trs(translation, rotation, scale) }
    }

    /// Translate, Rotate, Scale. Update a transform Matrix using all these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    ///
    /// see also [`crate::maths::matrix_trs_out`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn trs_to_pointer(translation: &Vec3, rotation: &Quat, scale: &Vec3, out_result: *mut Matrix) {
        unsafe { matrix_trs_out(out_result, translation, rotation, scale) }
    }

    /// Inverts this Matrix! If the matrix takes a point from a -> b, then its inverse takes the point from b -> a.
    /// The Matrix is modified so use get_inverse* for performance gains
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Invert.html>
    ///
    /// see also [`Matrix::get_inverse`] [`crate::maths::matrix_invert`]
    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let m = unsafe { matrix_invert(self) };
        unsafe {
            self.row[0] = m.row[0];
            self.row[1] = m.row[1];
            self.row[2] = m.row[2];
            self.row[3] = m.row[3];
        }
        self
    }

    /// Transposes this Matrix! Transposing is like rotating the matrix 90 clockwise, or turning the rows into columns.
    /// This can be useful for inverting orthogonal matrices, or converting matrices for use in a math library that
    /// uses different conventions! The Matrix is modified so use get_transposed* for performance gains
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transpose.html>
    ///
    /// see also [`crate::maths::matrix_transpose`]
    #[inline]
    pub fn transpose(&mut self) -> &mut Self {
        let m = unsafe { matrix_transpose(*self) };
        unsafe {
            self.row[0] = m.row[0];
            self.row[1] = m.row[1];
            self.row[2] = m.row[2];
            self.row[3] = m.row[3];
        }
        self
    }

    /// Returns this transformation matrix to its original translation, rotation and scale components. Not exactly a
    /// cheap function. If this is not a transform matrix, there’s a chance this call will fail, and return false.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Decompose.html>
    /// Returns the tuple (position:Vec3, scale:Vec3, orientation:QuatT)
    ///
    /// see also [`crate::maths::matrix_decompose`]
    #[inline]
    pub fn decompose(&self) -> Option<(Vec3, Vec3, Quat)> {
        let position: *mut Vec3 = &mut Vec3 { x: 0.0, y: 0.0, z: 0.0 };
        let scale: *mut Vec3 = &mut Vec3 { x: 0.0, y: 0.0, z: 0.0 };
        let orientation: *mut Quat = &mut Quat { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
        match unsafe { matrix_decompose(self, position, scale, orientation) } {
            0 => None,
            _ => unsafe { Some((*position, *scale, *orientation)) },
        }
    }

    /// Returns this transformation matrix to its original translation, rotation and scale components. Not exactly a
    /// cheap function. If this is not a transform matrix, there’s a chance this call will fail, and return false.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Decompose.html>
    ///
    /// see also [`crate::maths::matrix_decompose`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn decompose_to_ptr(&self, out_position: *mut Vec3, out_scale: *mut Vec3, out_orientation: *mut Quat) -> bool {
        unsafe { matrix_decompose(self, out_position, out_scale, out_orientation) != 0 }
    }

    /// Transforms a point through the Matrix! This is basically just multiplying a vector (x,y,z,1) with the Matrix.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html> see also the * operator
    ///
    /// see also [`crate::maths::matrix_transform_pt`]
    #[inline]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        unsafe { matrix_transform_pt(*self, point) }
    }

    /// Shorthand to transform a ray though the Matrix! This properly transforms the position with the point transform
    /// method, and the direction with the direction transform method. Does not normalize, nor does it preserve a
    /// normalized direction if the Matrix contains scale data.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html> see also the * operator
    ///
    /// see also [`crate::maths::matrix_transform_ray`]
    #[inline]
    pub fn transform_ray(&self, ray: Ray) -> Ray {
        unsafe { matrix_transform_ray(*self, ray) }
    }

    /// Shorthand for transforming a Pose! This will transform the position of the Pose with the matrix, extract a
    /// rotation Quat from the matrix and apply that to the Pose’s orientation. Note that extracting a rotation Quat
    /// is an expensive operation, so if you’re doing it more than once, you should cache the rotation Quat and do this
    /// transform manually.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html> see also the * operator
    ///
    /// see also [`crate::maths::matrix_transform_pose`]
    #[inline]
    pub fn transform_pose(&self, pose: Pose) -> Pose {
        unsafe { matrix_transform_pose(*self, pose) }
    }

    /// Shorthand for transforming a rotation! This will extract a
    /// rotation Quat from the matrix and apply that to the QuatT's orientation. Note that extracting a rotation Quat
    /// is an expensive operation, so if you’re doing it more than once, you should cache the rotation Quat and do this
    /// transform manually.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html> see also the * operator
    ///
    /// see also [`crate::maths::matrix_transform_quat`]
    #[inline]
    pub fn transform_quat(&self, rotation: Quat) -> Quat {
        unsafe { matrix_transform_quat(*self, rotation) }
    }

    /// Transforms a point through the Matrix, but excluding translation! This is great for transforming vectors that
    /// are -directions- rather than points in space. Use this to transform normals and directions. The same as
    /// multiplying (x,y,z,0) with the Matrix.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TransformNormal.html> do not correspond to * operator !
    ///
    /// see also [`crate::maths::matrix_transform_dir`]
    #[inline]
    pub fn transform_normal(&self, dir: Vec3) -> Vec3 {
        unsafe { matrix_transform_dir(*self, dir) }
    }

    /// Creates an inverse matrix! If the matrix takes a point from a -> b, then its inverse takes the point
    /// from b -> a.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Inverse.html>
    ///
    /// see also [`crate::maths::matrix_inverse`]
    #[inline]
    pub fn get_inverse(&self) -> Self {
        let out: *mut Matrix = &mut Matrix { row: [Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO] };
        unsafe {
            matrix_inverse(self, out);
            *out
        }
    }

    /// Creates an inverse matrix! If the matrix takes a point from a -> b, then its inverse takes the point
    /// from b -> a.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Inverse.html>
    ///
    /// see also [`crate::maths::matrix_inverse`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn get_inverse_to_ptr(&self, out: *mut Matrix) {
        unsafe {
            matrix_inverse(self, out);
        }
    }

    /// Extracts translation and rotation information from the transform matrix, and makes a Pose from it! Not exactly
    /// fast. This is backed by Decompose, so if you need any additional info, it’s better to just call Decompose instead.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Pose.html>
    ///
    /// see also [`crate::maths::matrix_extract_pose`]
    #[inline]
    pub fn get_pose(&self) -> Pose {
        unsafe { matrix_extract_pose(self) }
    }

    /// A slow function that returns the rotation quaternion embedded in this transform matrix. This is backed by
    /// Decompose, so if you need any additional info, it’s better to just call Decompose instead.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Rotation.html>
    ///
    /// see also [`crate::maths::matrix_extract_rotation`]
    #[inline]
    pub fn get_rotation(&self) -> Quat {
        unsafe { matrix_extract_rotation(self) }
    }

    /// Returns the scale embedded in this transform matrix. Not exactly cheap, requires 3 sqrt calls, but is cheaper
    /// than calling Decompose.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Scale.html>
    ///
    /// see also [`crate::maths::matrix_extract_scale`]
    #[inline]
    pub fn get_scale(&self) -> Vec3 {
        unsafe { matrix_extract_scale(self) }
    }

    /// A fast getter that will return or set the translation component embedded in this transform matrix.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Translation.html>
    ///
    /// see also [`crate::maths::matrix_extract_translation`]
    #[inline]
    pub fn get_translation(&self) -> Vec3 {
        unsafe { matrix_extract_translation(self) }
    }

    /// Creates a matrix that has been transposed! Transposing is like rotating the matrix 90 clockwise, or turning
    /// the rows into columns. This can be useful for inverting orthogonal matrices, or converting matrices for use
    /// in a math library that uses different conventions!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transposed.html>
    ///
    /// see also [`crate::maths::matrix_transpose`]
    #[inline]
    pub fn get_transposed(&self) -> Matrix {
        unsafe { matrix_transpose(*self) }
    }
}

impl Display for Matrix {
    /// Mostly for debug purposes, this is a decent way to log or inspect the matrix in debug mode. Looks
    /// like “[, , , ]”
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "[\r\n {},\r\n {},\r\n {},\r\n {}]", self.row[0], self.row[1], self.row[2], self.row[3]) }
    }
}
/// Multiplies the vector by the Matrix! Since only a 1x4 vector can be multiplied against a 4x4 matrix, this uses ‘1’
/// for the 4th element, so the result will also include translation! To exclude translation,
/// use Matrix.transform_normal.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point`]
impl Mul<Vec3> for Matrix {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        unsafe { matrix_transform_pt(self, rhs) }
    }
}

/// Multiplies the vector by the Matrix! Since only a 1x4 vector can be multiplied against a 4x4 matrix, this uses ‘1’
/// for the 4th element, so the result will also include translation! To exclude translation,
/// use Matrix.transform_normal.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point`]
impl MulAssign<Matrix> for Vec3 {
    fn mul_assign(&mut self, rhs: Matrix) {
        let res = unsafe { matrix_transform_pt(rhs, *self) };
        self.x = res.x;
        self.y = res.y;
        self.z = res.z;
    }
}

/// Multiplies the vector by the Matrix! Since only a 1x4 vector can be multiplied against a 4x4 matrix, this uses ‘1’
/// for the 4th element, so the result will also include translation! To exclude translation,
/// use Matrix.transform_normal.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point`]
impl Mul<Matrix> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pt(rhs, self) }
    }
}

/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point4`]
impl Mul<Vec4> for Matrix {
    type Output = Vec4;

    fn mul(self, rhs: Vec4) -> Self::Output {
        unsafe { matrix_transform_pt4(self, rhs) }
    }
}

/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point4`]
impl MulAssign<Matrix> for Vec4 {
    fn mul_assign(&mut self, rhs: Matrix) {
        let res = unsafe { matrix_transform_pt4(rhs, *self) };
        self.x = res.x;
        self.y = res.y;
        self.z = res.z;
        self.w = res.w
    }
}

/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_point4`]
impl Mul<Matrix> for Vec4 {
    type Output = Vec4;

    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pt4(rhs, self) }
    }
}

/// Transforms a Ray by the Matrix! The position and direction are both multiplied by the matrix, accounting properly for
/// which should include translation, and which should not.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_transform_ray`]
impl Mul<Ray> for Matrix {
    type Output = Ray;

    fn mul(self, rhs: Ray) -> Self::Output {
        unsafe { matrix_transform_ray(self, rhs) }
    }
}

/// Transforms a Ray by the Matrix! The position and direction are both multiplied by the matrix, accounting properly for
/// which should include translation, and which should not.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_transform_ray`]
impl MulAssign<Matrix> for Ray {
    fn mul_assign(&mut self, rhs: Matrix) {
        let res = unsafe { matrix_transform_ray(rhs, *self) };
        self.position = res.position;
        self.direction = res.direction;
    }
}

/// Transforms a Ray by the Matrix! The position and direction are both multiplied by the matrix, accounting properly for
/// which should include translation, and which should not.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_transform_ray`]
impl Mul<Matrix> for Ray {
    type Output = Ray;

    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_ray(rhs, self) }
    }
}

/// Transform an orientation by the Matrix.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_rotation`]
impl Mul<Quat> for Matrix {
    type Output = Quat;

    fn mul(self, rhs: Quat) -> Self::Output {
        unsafe { matrix_transform_quat(self, rhs) }
    }
}

/// Transform an orientation by the Matrix.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_rotation`]
impl MulAssign<Matrix> for Quat {
    fn mul_assign(&mut self, rhs: Matrix) {
        let res = unsafe { matrix_transform_quat(rhs, *self) };
        self.x = res.x;
        self.y = res.y;
        self.z = res.z;
        self.w = res.w
    }
}

/// Transform an orientation by the Matrix.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_rotation`]
impl Mul<Matrix> for Quat {
    type Output = Quat;

    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_quat(rhs, self) }
    }
}

/// Transforms a Pose by the Matrix! The position and orientation are both transformed by the matrix, accounting
/// properly for the Pose’s quaternion.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_pose`]
impl Mul<Pose> for Matrix {
    type Output = Pose;

    fn mul(self, rhs: Pose) -> Self::Output {
        unsafe { matrix_transform_pose(self, rhs) }
    }
}

/// Transforms a Pose by the Matrix! The position and orientation are both transformed by the matrix, accounting
/// properly for the Pose’s quaternion.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_pose`]
impl MulAssign<Matrix> for Pose {
    fn mul_assign(&mut self, rhs: Matrix) {
        let res = unsafe { matrix_transform_pose(rhs, *self) };
        self.position = res.position;
        self.orientation = res.orientation;
    }
}

/// Transforms a Pose by the Matrix! The position and orientation are both transformed by the matrix, accounting
/// properly for the Pose’s quaternion.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul_pose`]
impl Mul<Matrix> for Pose {
    type Output = Pose;

    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pose(rhs, self) }
    }
}

/// Multiplies two matrices together! This is a great way to combine transform operations. Note that StereoKit’s
/// matrices are row-major, and multiplication order is important! To translate, then scale, multiply in order of
/// ‘translate * scale’.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul`]
impl Mul<Matrix> for Matrix {
    type Output = Self;

    fn mul(self, rhs: Matrix) -> Self::Output {
        let out: *mut Matrix = &mut Matrix { row: [Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO] };
        unsafe {
            matrix_mul(&self, &rhs, out);
            *out
        }
    }
}

/// Multiplies two matrices together! This is a great way to combine transform operations. Note that StereoKit’s
/// matrices are row-major, and multiplication order is important! To translate, then scale, multiply in order of
/// ‘translate * scale’.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`crate::maths::matrix_mul`]
impl MulAssign<Matrix> for Matrix {
    fn mul_assign(&mut self, rhs: Matrix) {
        unsafe { matrix_mul(&rhs, self, self) };
    }
}

/// fluent syntax for Bounds.
/// Bounds is an axis aligned bounding box type that can be used for storing the sizes of objects, calculating
/// containment, intersections, and more!
///
/// While the constructor uses a center+dimensions for creating a bounds, don’t forget the static From* methods that
/// allow you to define a Bounds from different types of data!
/// <https://stereokit.net/Pages/StereoKit/Bounds.html>
/// ## Examples
///
/// see also [`crate::maths::Bounds`]
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct Bounds {
    pub center: Vec3,
    pub dimensions: Vec3,
}
/// AsRef
impl AsRef<Bounds> for Bounds {
    fn as_ref(&self) -> &Bounds {
        self
    }
}

extern "C" {
    pub fn bounds_ray_intersect(bounds: Bounds, ray: Ray, out_pt: *mut Vec3) -> Bool32T;
    pub fn bounds_point_contains(bounds: Bounds, pt: Vec3) -> Bool32T;
    pub fn bounds_line_contains(bounds: Bounds, pt1: Vec3, pt2: Vec3) -> Bool32T;
    pub fn bounds_capsule_contains(bounds: Bounds, pt1: Vec3, pt2: Vec3, radius: f32) -> Bool32T;
    pub fn bounds_grow_to_fit_pt(bounds: Bounds, pt: Vec3) -> Bounds;
    pub fn bounds_grow_to_fit_box(bounds: Bounds, box_: Bounds, opt_box_transform: *const Matrix) -> Bounds;
    pub fn bounds_transform(bounds: Bounds, transform: Matrix) -> Bounds;
}

impl Bounds {
    /// Creates a bounding box object!
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Bounds.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn new<V: Into<Vec3>>(center: V, dimensions: V) -> Bounds {
        Bounds { center: center.into(), dimensions: dimensions.into() }
    }

    /// Creates a bounding box object centered around zero!
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Bounds.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn bounds_centered(dimensions: impl Into<Vec3>) -> Bounds {
        Bounds { center: Vec3::ZERO, dimensions: dimensions.into() }
    }

    /// Create a bounding box from a corner, plus box dimensions.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/FromCorner.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn from_corner<V: Into<Vec3>>(bottom_left_back: V, dimensions: V) -> Bounds {
        let dim = dimensions.into();
        Bounds { center: bottom_left_back.into() + dim / 2.0, dimensions: dim }
    }

    /// Create a bounding box between two corner points.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/FromCorners.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn from_corners<V: Into<Vec3>>(bottom_left_back: V, top_right_front: V) -> Bounds {
        let blb = bottom_left_back.into();
        let trf = top_right_front.into();
        Bounds { center: blb / 2.0 + trf / 2.0, dimensions: (trf - blb).abs() }
    }

    /// Grow the Bounds to encapsulate the provided point. Returns the result, and does NOT modify the current bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Grown.html>
    ///
    /// see also [`crate::maths::bounds_grow_to_fit_pt]
    #[inline]
    pub fn grown_point(&mut self, pt: impl Into<Vec3>) -> &mut Self {
        let b = unsafe { bounds_grow_to_fit_pt(*self, pt.into()) };
        self.center = b.center;
        self.dimensions = b.dimensions;
        self
    }

    /// Grow the Bounds to encapsulate the provided point. Returns the result, and does NOT modify the current bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Grown.html>
    /// * opt_box_transform - to center use Mat4::IDENTITY
    ///
    /// see also [`crate::maths::bounds_grow_to_fit_box]
    #[inline]
    pub fn grown_box<M: Into<Matrix>>(&mut self, box_: impl AsRef<Bounds>, opt_box_transform: M) -> &mut Self {
        let b = unsafe { bounds_grow_to_fit_box(*self, *(box_.as_ref()), &opt_box_transform.into()) };
        self.center = b.center;
        self.dimensions = b.dimensions;
        self
    }

    /// Scale this bounds. It will scale the center as well as the dimensions! Modifies this bounds object.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scale.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn scale(&mut self, scale: f32) -> &mut Self {
        self.dimensions *= scale;
        self.center *= scale;
        self
    }

    /// Scale this bounds. It will scale the center as well as the dimensions! Modifies this bounds object.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scale.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn scale_vec(&mut self, scale: Vec3) -> &mut Self {
        self.dimensions *= scale;
        self.center *= scale;
        self
    }

    /// Does the Bounds contain the given point? This includes points that are on the surface of the Bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    ///
    /// see also [`crate::maths::bounds_point_contains`]
    #[inline]
    pub fn contains_point(&self, pt: impl Into<Vec3>) -> bool {
        unsafe { bounds_point_contains(*self, pt.into()) != 0 }
    }

    /// Does the Bounds contain or intersects with the given line?
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    ///
    /// see also [`crate::maths::bounds_line_contains`]
    #[inline]
    pub fn contains_line<V3: Into<Vec3>>(&self, line_pt1: V3, line_pt2: V3) -> bool {
        unsafe { bounds_line_contains(*self, line_pt1.into(), line_pt2.into()) != 0 }
    }

    /// Does the bounds contain or intersect with the given capsule?
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    ///
    /// see also [`crate::maths::bounds_capsule_contains`]
    #[inline]
    pub fn contains_capsule<V3: Into<Vec3>>(&self, line_pt1: V3, line_pt2: V3, radius: f32) -> bool {
        unsafe { bounds_capsule_contains(*self, line_pt1.into(), line_pt2.into(), radius) != 0 }
    }

    /// Calculate the intersection between a Ray, and these bounds. Returns false if no intersection occurred, and ‘at’
    /// will contain the nearest intersection point to the start of the ray if an intersection is found!
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Intersect.html>
    /// * ray - Any Ray in the same coordinate space as the Bounds
    ///
    /// Returns the closest intersection point to the origin of the ray or None if there isn't an instersection
    /// see also [`crate::maths::bounds_ray_intersect`]
    #[inline]
    pub fn intersect(&self, ray: Ray) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { bounds_ray_intersect(*self, ray, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Scale the bounds. It will scale the center as well as the dimensions! Returns a new Bounds.
    /// equivalent to using multiply operator
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scaled.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        *self * scale
    }

    /// Scale the bounds. It will scale the center as well as the dimensions! Returns a new Bounds.
    /// equivalent to using multiply operator
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scaled.html>
    ///
    /// see also [`crate::maths::Bounds]
    #[inline]
    pub fn scaled_vec(&self, scale: impl Into<Vec3>) -> Self {
        *self * scale.into()
    }

    /// This returns a Bounds that encapsulates the transformed points of the current Bounds’s corners.
    /// Note that this will likely introduce a lot of extra empty volume in many cases, as the result is still always axis aligned.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Transformed.html>
    ///
    /// see also [`crate::maths::bounds_transform]
    #[inline]
    pub fn transformed(&self, transform: impl Into<Matrix>) -> Self {
        unsafe { bounds_transform(*self, transform.into()) }
    }

    /// From the front, this is the Top (Y+), Left (X+), Center
    /// (Z0) of the bounds. Useful when working with UI layout bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/TLC.html>
    #[inline]
    pub fn tlc(&self) -> Vec3 {
        self.center + self.dimensions.xy0() / 2.0
    }

    /// From the front, this is the Top (Y+), Left (X+), Back (Z+)
    /// of the bounds. Useful when working with UI layout bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/TLB.html>
    #[inline]
    pub fn tlb(&self) -> Vec3 {
        self.center + self.dimensions / 2.0
    }
}

impl Display for Bounds {
    /// Creates a text description of the Bounds, in the format of “[center:X dimensions:X]”
    /// <https://stereokit.net/Pages/StereoKit/Bounds/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[center:{} dimensions:{}]", self.center, self.dimensions)
    }
}
/// This operator will create a new Bounds that has been properly scaled up by the float. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl Mul<f32> for Bounds {
    type Output = Bounds;

    fn mul(self, rhs: f32) -> Self::Output {
        Bounds { center: self.center * rhs, dimensions: self.dimensions * rhs }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the float. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl MulAssign<f32> for Bounds {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.center.mul_assign(rhs);
        self.dimensions.mul_assign(rhs);
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the float. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl Mul<Bounds> for f32 {
    type Output = Bounds;

    fn mul(self, rhs: Bounds) -> Self::Output {
        Bounds { center: rhs.center * self, dimensions: rhs.dimensions * self }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the Vec3. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl Mul<Vec3> for Bounds {
    type Output = Bounds;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Bounds { center: self.center * rhs, dimensions: self.dimensions * rhs }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the Vec3. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl MulAssign<Vec3> for Bounds {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec3) {
        self.center.mul_assign(rhs);
        self.dimensions.mul_assign(rhs);
    }
}

/// fluent syntax for Plane.
/// Planes are really useful for collisions, intersections, and visibility testing!
///
/// This plane is stored using the ax + by + cz + d = 0 formula, where the normal is a,b,c, and the d is, well, d.
/// <https://stereokit.net/Pages/StereoKit/Plane.html>
///
/// see also [`crate::maths::Plane`]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Plane {
    /// The direction the plane is facing.
    pub normal: Vec3,
    /// The distance/travel along the plane's normal from
    /// the origin to the surface of the plane.
    pub d: f32,
}
/// AsRef
impl AsRef<Plane> for Plane {
    fn as_ref(&self) -> &Plane {
        self
    }
}

extern "C" {
    pub fn plane_from_points(p1: Vec3, p2: Vec3, p3: Vec3) -> Plane;
    pub fn plane_from_ray(ray: Ray) -> Plane;
    pub fn plane_ray_intersect(plane: Plane, ray: Ray, out_pt: *mut Vec3) -> Bool32T;
    pub fn plane_line_intersect(plane: Plane, p1: Vec3, p2: Vec3, out_pt: *mut Vec3) -> Bool32T;
    pub fn plane_point_closest(plane: Plane, pt: Vec3) -> Vec3;
}

impl Plane {
    /// Creates a Plane directly from the ax + by + cz + d = 0 formula!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Plane.html>
    pub fn new<V: Into<Vec3>>(normal: V, d: f32) -> Plane {
        Plane { normal: normal.into(), d }
    }

    /// Creates a plane from a normal, and any point on the plane!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Plane.html>
    ///
    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::{Plane,Vec3};
    /// let wall_x = Plane::from_point(Vec3::X*4.0,  Vec3::X);
    /// assert_eq!(wall_x.d , -4.0);
    /// ````
    #[inline]
    pub fn from_point<V: Into<Vec3>>(point_on_plane: V, plane_normal: V) -> Plane {
        let p_o_p = point_on_plane.into();
        let normal = plane_normal.into();
        // let plane0 = Plane { normal, d: 0.0 };
        // let p0 = plane0.closest(p_o_p);
        //Plane { normal, d: Vec3::distance(p0, p_o_p) }
        Plane { normal, d: -Vec3::dot(p_o_p, normal) }
    }

    /// Creates a plane from 3 points that are directly on that plane.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Plane/Plane.html>
    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::{Plane,Vec3};
    /// let ground = Plane::from_points(Vec3::X, Vec3::Z, Vec3::X + Vec3::Z);
    /// assert_eq!(ground.d , 0.0);
    /// ```
    #[inline]
    pub fn from_points<V: Into<Vec3>>(point_on_plane1: V, point_on_plane2: V, point_on_plane3: V) -> Plane {
        let p1 = point_on_plane1.into();
        let p2 = point_on_plane2.into();
        let p3 = point_on_plane3.into();
        let dir1 = p2 - p1;
        let dir2 = p2 - p3;
        let normal = Vec3::cross(dir1, dir2).get_normalized();
        //let plane0 = Plane { normal, d: 0.0 };
        //let p0 = plane0.closest(p2);
        //Plane { normal, d: Vec3::distance(p0, p2) }
        Plane { normal, d: -Vec3::dot(p2, normal) }

        // Do not save the problem : unsafe{plane_from_points(p1, p2, p3)}
    }

    /// Finds the closest point on this plane to the given point!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Closest.html>
    ///
    /// see also [`crate::maths::plane_point_closest`]
    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::{Plane,Vec3};
    /// let plane = Plane{normal : Vec3::X , d: -4.0};
    /// let closest = plane.closest(Vec3::ZERO);
    /// assert_eq!(closest , (Vec3::X * 4.0));
    /// let plane = Plane::from_points(Vec3::X , Vec3::Z, Vec3::Y );
    /// assert_eq!(plane, Plane{normal : Vec3::ONE / (3.0_f32.sqrt()), d:-1.0/3.0_f32.sqrt()} );
    /// let closest = plane.closest(Vec3::ZERO);
    /// assert_eq!(closest , Vec3::new(0.3333333,0.3333333,0.3333333));
    /// ```
    #[inline]
    pub fn closest<V: Into<Vec3>>(&self, to: V) -> Vec3 {
        unsafe { plane_point_closest(*self, to.into()) }
    }

    /// Checks the intersection of a ray with this plane!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Intersect.html>
    /// * ray - The ray we're checking with.
    ///
    /// Returns the intersection point or None if there isn't an instersection
    /// see also [`crate::maths::plane_ray_intersect`]
    #[inline]
    pub fn intersect(&self, ray: Ray) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { plane_ray_intersect(*self, ray, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Checks the intersection of a line with this plane!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Intersect.html>
    /// * line_start - Start of the line.
    /// * line_end - End of the line.
    ///
    /// Returns the intersection point or None if there isn't an instersection
    /// see also [`crate::maths::plane_line_intersect`]
    #[inline]
    pub fn intersect_line<V: Into<Vec3>>(&self, line_start: V, line_end: V) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { plane_line_intersect(*self, line_start.into(), line_end.into(), &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }
}

impl Display for Plane {
    /// Creates a text description of the Plane, in the format of “[normal:X distance:X]”
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[normal:{} distance:{}]", self.normal, self.d)
    }
}
/// Pose represents a location and orientation in space, excluding scale! The default value of a Pose use
/// Pose.Identity .
/// <https://stereokit.net/Pages/StereoKit/Pose.html>
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Pose {
    /// Origin with Quat::IDENTITY orientation
    pub const IDENTITY: Pose = Pose { position: Vec3::new(0.0, 0.0, 0.0), orientation: Quat::IDENTITY };

    /// Basic initialization constructor! Just copies in the provided values directly, and uses Identity for the
    /// orientation.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Pose.html>
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn new(position: impl Into<Vec3>, orientation: Option<Quat>) -> Self {
        let orientation = orientation.unwrap_or(Quat::IDENTITY);
        Self { position: position.into(), orientation }
    }

    /// Interpolates between two poses! It is unclamped, so values outside of (0,1) will extrapolate their position.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Lerp.html>
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn lerp(a: impl Into<Pose>, b: impl Into<Pose>, percent: f32) -> Self {
        let a = a.into();
        let b = b.into();
        Self {
            position: Vec3::lerp(a.position, b.position, percent),
            orientation: Quat::slerp(a.orientation, b.orientation, percent),
        }
    }

    /// Creates a Pose that looks from one location in the direction of another location. This leaves “Up” as the +Y
    /// axis.
    /// <https://stereokit.net/Pages/StereoKit/Pose/LookAt.html>
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn look_at(from: impl Into<Vec3>, at: impl Into<Vec3>) -> Self {
        let from = from.into();
        let at = at.into();
        Self { position: from, orientation: Quat::look_at(from, at, None) }
    }

    /// Converts this pose into a transform matrix.
    /// <https://stereokit.net/Pages/StereoKit/Pose/ToMatrix.html>
    /// * scale - Let you add a scale factor if needed.
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn to_matrix(&self, scale: Option<Vec3>) -> Matrix {
        match scale {
            Some(scale) => Matrix::trs(&self.position, &self.orientation, &scale),
            None => Matrix::tr(&self.position, &self.orientation),
        }
    }

    /// Calculates the forward direction from this pose. This is done by multiplying the orientation with
    /// Vec3::new(0, 0, -1). Remember that Forward points down the -Z axis!
    /// <https://stereokit.net/Pages/StereoKit/Pose/Forward.html>
    ///
    /// see also [`crate::maths::Pose::forward`]
    #[inline]
    pub fn get_forward(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::new(0.0, 0.0, -1.0))
    }

    /// This creates a ray starting at the Pose’s position, and pointing in the ‘Forward’ direction. The Ray
    /// direction is a unit vector/normalized.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Ray.html>
    ///
    /// see also [`crate::maths::Pose::ray`]
    #[inline]
    pub fn get_ray(&self) -> Ray {
        Ray { position: self.position, direction: Vec3::new(0.0, 0.0, -1.0) }
    }

    /// Calculates the right (+X) direction from this pose. This is done by multiplying the orientation with Vec3.Right.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Right.html>
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn get_right(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::RIGHT)
    }

    /// Calculates the up (+Y) direction from this pose. This is done by multiplying the orientation with Vec3.Up.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Up.html>
    ///
    /// see also [`crate::maths::Pose`]
    #[inline]
    pub fn get_up(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::UP)
    }
}

impl Display for Pose {
    /// A string representation of the Pose, in the format of “position, Forward”. Mostly for debug visualization.
    /// <https://stereokit.net/Pages/StereoKit/Pose/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[position:{} forward:{}]", self.position, self.orientation)
    }
}

/// fluent syntax for Sphere.
/// Represents a sphere in 3D space! Composed of a center point and a radius, can be used for raycasting, collision,
/// visibility, and other things!
///
/// <https://stereokit.net/Pages/StereoKit/Sphere.html>
///
/// see also [`crate::maths::Sphere`]
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct Sphere {
    /// Center of the sphere
    pub center: Vec3,
    ///Distance from the center, to the surface of the sphere, in meters. Half the diameter.
    pub radius: f32,
}
/// AsRef
impl AsRef<Sphere> for Sphere {
    fn as_ref(&self) -> &Sphere {
        self
    }
}

extern "C" {
    pub fn sphere_ray_intersect(sphere: Sphere, ray: Ray, out_pt: *mut Vec3) -> Bool32T;
    pub fn sphere_point_contains(sphere: Sphere, pt: Vec3) -> Bool32T;
}

impl Sphere {
    /// Creates a Sphere directly from the ax + by + cz + d = 0 formula!
    /// <https://stereokit.net/Pages/StereoKit/Sphere.html>
    ///
    /// see also [`crate::maths::Sphere]
    #[inline]
    pub fn new<V: Into<Vec3>>(center: V, radius: f32) -> Sphere {
        Sphere { center: center.into(), radius }
    }

    /// A fast check to see if the given point is contained in or on a sphere!
    /// <https://stereokit.net/Pages/StereoKit/Sphere/Contains.html>
    ///
    /// see also [`crate::maths::sphere_point_contains`]
    #[inline]
    pub fn contains<V: Into<Vec3>>(&self, point: V) -> bool {
        unsafe { sphere_point_contains(*self, point.into()) != 0 }
    }

    /// Intersects a ray with this sphere, and finds if they intersect, and if so, where that intersection is!
    /// This only finds the closest intersection point to the origin of the ray.
    /// <https://stereokit.net/Pages/StereoKit/Sphere/Intersect.html>
    /// * ray - A ray to intersect with
    ///
    /// Returns the closest intersection point to the ray's origin. Or None it there is no intersection.
    /// see also [`crate::maths::sphere_ray_intersect`]
    #[inline]
    pub fn intersect(&self, ray: Ray) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { sphere_ray_intersect(*self, ray, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }
}
impl Display for Sphere {
    /// Creates a text description of the Sphere, in the format of “[center:X radius:X]”
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[center:{} radius:{}]", self.center, self.radius)
    }
}
/// A pretty straightforward 2D rectangle, defined by the top left corner of the rectangle, and its width/height.
/// <https://stereokit.net/Pages/StereoKit/Rect.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Rect {
    /// The X axis position of the top left corner of the rectangle.
    pub x: f32,
    /// The Y axis position of the top left corner of the rectangle.
    pub y: f32,
    /// The width of the rectangle.
    pub width: f32,
    /// The height of the rectangle.
    pub height: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }
    }
}

impl Rect {
    /// Create a 2D rectangle, defined by the top left corner of the rectangle, and its width/height.
    /// <https://stereokit.net/Pages/StereoKit/Rect/Rect.html>
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// A position and a direction indicating a ray through space! This is a great tool for intersection testing with
/// geometrical shapes.
/// <https://stereokit.net/Pages/StereoKit/Ray.html>
///
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct Ray {
    /// The position or origin point of the Ray.
    pub position: Vec3,
    /// The direction the ray is facing, typically does not require being a unit vector, or normalized direction.
    pub direction: Vec3,
}

extern "C" {
    pub fn ray_intersect_plane(ray: Ray, plane_pt: Vec3, plane_normal: Vec3, out_t: *mut f32) -> Bool32T;
    pub fn ray_from_mouse(screen_pixel_pos: Vec2, out_ray: *mut Ray) -> Bool32T;
    pub fn ray_point_closest(ray: Ray, pt: Vec3) -> Vec3;
}

impl Ray {
    /// Basic initialization constructor! Just copies the parameters into the fields.
    /// <https://stereokit.net/Pages/StereoKit/Ray/Ray.html>
    /// * position - The position or origin point of the Ray.
    /// * direction - The direction the ray is facing, typically does not require being a unit vector, or normalized
    ///   direction.
    #[inline]
    pub fn new<V: Into<Vec3>>(pos: V, dir: V) -> Self {
        Self { position: pos.into(), direction: dir.into() }
    }

    /// A convenience function that creates a ray from point a, towards point b. Resulting direction is not normalized.
    /// <https://stereokit.net/Pages/StereoKit/Ray/FromTo.html>
    /// * a - Ray starting point.
    /// * b - Location the ray is pointing towards.
    ///
    /// Returns a ray from point a to point b. Not normalized.
    #[inline]
    pub fn from_to<V: Into<Vec3>>(&self, a: V, b: V) -> Ray {
        let position = a.into();
        let direction = b.into() - position;
        Ray { position, direction }
    }

    /// Gets a point along the ray! This is basically just position + direction*percent. If Ray.direction is
    /// normalized, then percent is functionally distance, and can be used to find the point a certain distance
    /// out along the ray.
    /// <https://stereokit.net/Pages/StereoKit/Ray/At.html>
    /// * percent - How far along the ray should we get the  point at? This is in multiples of self.direction's
    ///   magnitude. If self.direction is normalized, this is functionally the distance.
    ///
    /// Returns the point at position + direction*percent
    #[inline]
    pub fn get_at(&self, percent: f32) -> Vec3 {
        self.position + self.direction * percent
    }

    /// Calculates the point on the Ray that’s closest to the given point! This can be in front of, or behind the
    /// ray’s starting position.
    /// <https://stereokit.net/Pages/StereoKit/Ray/Closest.html>
    /// * to - Any point in the same coordinate space as the  Ray.
    ///
    /// Returns the point on the ray that's closest to the given point.
    /// see also [`crate::maths::ray_point_closest`]
    #[inline]
    pub fn closest<V: Into<Vec3>>(&self, to: V) -> Vec3 {
        unsafe { ray_point_closest(*self, to.into()) }
    }

    /// Checks the intersection of this ray with a plane!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * plane - Any plane you want to intersect with.
    ///
    /// Returns intersection point if there's an intersection information or None if there's no intersection
    /// see also [`crate::maths::plane_ray_intersect`]
    #[inline]
    pub fn intersect(&self, plane: Plane) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { plane_ray_intersect(plane, *self, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Checks the intersection of this ray with a sphere!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * sphere - Any sphere you want to intersect with.
    ///
    /// Returns the closest intersection point to the ray origin if there's an intersection information or None if
    /// there's no intersection
    /// see also [`crate::maths::sphere_ray_intersect`]
    #[inline]
    pub fn intersect_sphere(&self, sphere: Sphere) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { sphere_ray_intersect(sphere, *self, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Checks the intersection of this ray with a bounding box!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * bounds - Any bounds you want to intersect with.
    ///
    /// Returns the closest intersection point to the ray origin if there's an intersection information or None if
    /// there's no intersection
    /// see also [`crate::maths::bounds_ray_intersect`]
    #[inline]
    pub fn intersect_bound(&self, bounds: Bounds) -> Option<Vec3> {
        let mut pt = Vec3::default();
        match unsafe { bounds_ray_intersect(bounds, *self, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Checks the intersection point of this ray and a Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * mesh - A mesh containing collision data on the CPU. You can check this with mesh.get_keep_data().
    /// * cull - If None has default value of Cull::Back.
    ///
    /// Returns a tuple with
    /// - The intersection point of the ray and the mesh, if an intersection occurs. This is in model space, and must be
    ///   transformed back into world space later.
    /// - The indice of the mesh where the intersection occurs.
    ///
    /// see also [`stereokit::mesh_ray_intersect`]    
    #[inline]
    pub fn intersect_mesh(&self, mesh: &Mesh, cull: Option<Cull>) -> Option<(Vec3, VindT)> {
        let mut out_ray = Ray::default();
        let mut out_inds = 0;
        let cull = cull.unwrap_or(Cull::Back);

        match unsafe { mesh_ray_intersect(mesh.0.as_ptr(), *self, cull, &mut out_ray, &mut out_inds) != 0 } {
            true => Some((out_ray.position, out_inds)),
            false => None,
        }
    }

    /// Checks the intersection point of this ray and a Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * mesh - A mesh containing collision data on the CPU. You can check this with mesh.get_keep_data()
    /// * out_model_space_at - The intersection point and surface direction of the ray and the mesh, if an intersection
    ///   occurs. This is in model space, and must be transformed back into world space later. Direction is not
    ///   guaranteed to be normalized, especially if your own model->world transform contains scale/skew in it.
    /// * out_start_inds - The index of the first index of the triangle that was hit
    /// * cull - How should intersection work with respect to the direction the triangles are facing? Should we skip
    ///   triangles that are facing away from the ray, or don't skip anything? A good default would be Cull.Back.
    ///   If None has default value of Cull::Back.
    ///
    /// Returns true if an intersection occurs, false otherwise!
    /// see also [`crate::maths::mesh_ray_intersect`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_mesh_to_ptr(
        &self,
        mesh: &Mesh,
        cull: Option<Cull>,
        out_model_space_at: *mut Ray,
        out_start_inds: *mut u32,
    ) -> bool {
        let cull = cull.unwrap_or(Cull::Back);
        unsafe { mesh_ray_intersect(mesh.0.as_ptr(), *self, cull, out_model_space_at, out_start_inds) != 0 }
    }

    /// Checks the intersection point of this ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * model - Any Model that may or may not contain Solid flagged nodes, and Meshes with collision data.
    /// * cull - If None has default value of Cull::Back.
    ///
    /// Returns the intersection point of the ray and the model, if an intersection occurs. This is in model space, and
    /// must be transformed back into world space later.
    /// see also [`crate::maths::model_ray_intersect`]
    #[inline]
    pub fn intersect_model(&self, model: &Model, cull: Option<Cull>) -> Option<Vec3> {
        let mut out_ray = Ray::default();
        let cull = cull.unwrap_or(Cull::Back);

        match unsafe { model_ray_intersect(model.0.as_ptr(), *self, cull, &mut out_ray) != 0 } {
            true => Some(out_ray.position),
            false => None,
        }
    }

    /// Checks the intersection point of this ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * model - Any Model that may or may not contain Solid flagged nodes, and Meshes with collision data.
    /// * cull - If None has default value of Cull::Back.
    /// * out_model_space_at  - The intersection point and surface direction of the ray and the model, if an intersection
    ///   occurs. This is in model space, and must be transformed back into world space later. Direction is not
    ///   guaranteed to be normalized, especially if your own model->world transform contains scale/skew in it.
    ///
    /// Returns - true if an intersection occurs, false otherwise!
    /// see also [`crate::maths::model_ray_intersect`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_model_to_ptr(&self, model: &Model, cull: Option<Cull>, out_model_space_at: *mut Ray) -> bool {
        let cull = cull.unwrap_or(Cull::Back);
        unsafe { model_ray_intersect(model.0.as_ptr(), *self, cull, out_model_space_at) != 0 }
    }
}

impl Display for Ray {
    /// Creates a text description of the Ray, in the format of “[position:X direction:X]”
    /// <https://stereokit.net/Pages/StereoKit/Ray/ToString.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[position:{} direction:{}]", self.position, self.direction)
    }
}
