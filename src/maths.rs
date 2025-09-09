use crate::{
    material::Cull,
    mesh::{Mesh, VindT, mesh_ray_intersect},
    model::{Model, model_ray_intersect},
};
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// Native code use this as bool
pub type Bool32T = i32;

/// Blends (Linear Interpolation) between two scalars, based on a 'blend' value, where 0 is a, and 1 is b. Doesn't clamp
/// percent for you.
/// <https://stereokit.net/Pages/StereoKit/SKMath/Lerp.html>
/// * `a` - First item in the blend, or '0.0' blend.
/// * `b` - Second item in the blend, or '1.0' blend.
/// * `t` - A blend value between 0 and 1. Can be outside   this range, it'll just interpolate outside of the a, b range.
///
/// Returns an unclamped blend of a and b.
/// see also [`crate::util::Color128::lerp`]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Calculates the minimum angle 'distance' between two angles. This covers wraparound cases like: the minimum distance
/// between 10 and 350 is 20.
/// <https://stereokit.net/Pages/StereoKit/SKMath/AngleDist.html>
/// * `a` - First angle, in degrees.
/// * `b` - Second angle, in degrees.
///   
/// returns : Degrees 0-180, the minimum angle between a and b.
pub fn angle_dist(a: f32, b: f32) -> f32 {
    let delta = (b - a + 180.0) % 360.0 - 180.0;
    (if delta < -180.0 { delta + 360.0 } else { delta }).abs()
}

pub mod units {
    /// Converts centimeters to meters. There are 100cm in 1m. In StereoKit 1 unit is also 1 meter,
    /// so `25 * Units.cm2m == 0.25`, 25 centimeters is .25 meters/units.
    pub const CM2M: f32 = 0.01;
    /// Converts millimeters to meters. There are 1000mm in 1m. In StereoKit 1 unit is 1 meter,
    /// so `250 * Units.mm2m == 0.25`, 250 millimeters is .25 meters/units.
    pub const MM2M: f32 = 0.001;
    ///Converts meters to centimeters. There are 100cm in 1m, so this just multiplies by 100.
    pub const M2CM: f32 = 100.0;
    ///Converts meters to millimeters. There are 1000mm in 1m, so this just multiplies by 1000.
    pub const M2MM: f32 = 1000.0;

    /// Converts centimeters to meters. There are 100cm in 1m. In StereoKit 1 unit is also 1 meter,
    /// so `25 * U.cm == 0.25`, 25 centimeters is .25 meters/units.
    pub const CM: f32 = 0.01;
    /// Converts millimeters to meters. There are 1000mm in 1m. In StereoKit 1 unit is 1 meter,
    /// so `250 * Units.mm2m == 0.25`, 250 millimeters is .25 meters/units.
    pub const MM: f32 = 0.001;
    /// StereoKit's default unit is meters, but sometimes it's nice to be explicit!
    pub const M: f32 = 1.0;
    /// Converts meters to kilometers. There are 1000m in 1km, so this just multiplies by 1000.
    pub const KM: f32 = 1000.0;
}

/// A vector with 2 components: x and y. This can represent a point in 2D space, a directional vector, or any other sort
/// of value with 2 dimensions to it!
/// <https://stereokit.net/Pages/StereoKit/Vec2.html>
///
/// see also [`glam::Vec2`]
/// ### Examples
/// ```
/// use stereokit_rust::maths::Vec2;
///
/// let vec2        = Vec2::new(1.0, 2.0);
/// let vec2a       = Vec2 { x: 1.0, y: 2.0 };
/// let vec2b       = Vec2::X + Vec2::Y * 2.0;
/// let vec3c: Vec2 = [1.0, 2.0].into();
///
/// assert_eq!(vec2, vec2a);
/// assert_eq!(vec2a + vec2b,                   Vec2 { x: 2.0, y: 4.0 });
/// assert_eq!(vec3c,                           Vec2 { x: 1.0, y: 2.0 });
/// assert_eq!(vec2a.length_sq(),               5.0);
/// assert_eq!(Vec2::Y.angle(),                 90.0);
/// assert_eq!(vec2a.magnitude(),               (5.0f32).sqrt());
/// assert!   (vec2a.get_normalized().length() - 1.0 < f32::EPSILON);
/// assert_eq!(Vec2::angle_between(Vec2::X, Vec2::Y), 90.0);
/// assert_eq!(Vec2::dot(Vec2::X, Vec2::Y), 0.0);
/// ```
#[derive(Debug, Default, Copy, Clone)]
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

impl From<[f32; 2]> for Vec2 {
    fn from(val: [f32; 2]) -> Self {
        Vec2 { x: val[0], y: val[1] }
    }
}

///  Warning: Equality with a precision of 0.1 millimeter
impl PartialEq for Vec2 {
    ///  Warning: Equality with a precision of 0.1 millimeter
    /// ```
    /// use stereokit_rust::maths::Vec2;
    /// assert_eq!(
    ///              Vec2 { x: 0.045863353, y: 0.030000005 } ,
    ///              Vec2 { x: 0.045863353, y: 0.030000005 } );
    /// ```
    /// ```
    /// use stereokit_rust::maths::Vec2;
    /// assert_ne!(
    ///              Vec2 { x: 10.045863353, y: 0.030000005 } ,
    ///              Vec2 { x: 0.045863353, y: 0.030000005 } );
    /// ```
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < 0.0001) && ((self.y - other.y).abs() < 0.0001)
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

    /// Returns the counter-clockwise degrees from `[1,0]`. Resulting value is between 0 and 360. Vector does not need
    /// to be normalized.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Angle.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.angle(), 45.0);
    ///
    /// let vec2 = Vec2::new(0.0, 1.0);
    /// assert_eq!(vec2.angle(), 90.0);
    ///
    /// let vec2 = Vec2::new(-1.0, 1.0);
    /// assert_eq!(vec2.angle(), 135.0);
    ///
    /// let vec2 = Vec2::new(-1.0, 0.0);
    /// assert_eq!(vec2.angle(), 180.0);
    ///
    /// let vec2 = Vec2::new(-1.0, -1.0);
    /// assert_eq!(vec2.angle(), 225.0);
    /// ```
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
    /// * `point` - The point to check against.
    /// * `radius` - The distance to check within.
    ///
    /// Returns true if the points are within radius of each other, false not.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// let vec2_in = Vec2::new(1.1, 1.1);
    /// let vec2_out = Vec2::new(2.0, 2.0);
    /// assert!(vec2.in_radius(vec2_in, 0.2));
    /// assert!(!vec2.in_radius(vec2_out, 0.2));
    /// ```
    #[inline]
    pub fn in_radius(&self, point: Self, radius: f32) -> bool {
        Self::distance(*self, point) <= radius
    }

    /// Turns this vector into a normalized vector (vector with a length of 1) from the current vector. Will not work
    /// properly if the vector has a length of zero. Vec2::get_normalized is faster.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Normalize.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2 = Vec2::new(1.0, 1.0);
    /// assert!((vec2.length() - 1.4142135).abs() < f32::EPSILON);
    ///
    /// vec2.normalize();
    /// assert!((vec2.length() - 1.0).abs() < f32::EPSILON);
    /// ```
    #[inline]
    pub fn normalize(&mut self) {
        let n = *self * (self.length().recip());
        self.x = n.x;
        self.y = n.y;
    }

    /// This is the length of the vector! Or the distance from the origin to this point. Uses f32::sqrt, so it’s not
    /// dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Length.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.length(), (2.0f32).sqrt());
    /// ```
    #[inline]
    pub fn length(&self) -> f32 {
        Self::dot(*self, *self).sqrt()
    }

    /// This is the squared length/magnitude of the vector! It skips the Sqrt call, and just gives you the squared
    /// version for speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/LengthSq.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.length_sq(), 2.0);
    /// ```
    #[inline]
    pub fn length_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    /// Magnitude is the length of the vector! Or the distance from the origin to this point. Uses f32::sqrt, so it’s
    /// not dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Magnitude.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.length(), vec2.magnitude());
    /// ```
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.length()
    }

    /// This is the squared magnitude of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared. Vec2::length_squared is faster
    /// <https://stereokit.net/Pages/StereoKit/Vec2/MagnitudeSq.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.length_sq(), vec2.magnitude_sq());
    /// ```
    #[inline]
    pub fn magnitude_sq(&self) -> f32 {
        self.length_sq()
    }

    /// Creates a normalized vector (vector with a length of 1) from the current vector. Will not work properly if the
    /// vector has a length of zero.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Normalized.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert!((vec2.get_normalized().length() - 1.0).abs() < f32::EPSILON);
    /// ```
    #[inline]
    pub fn get_normalized(&self) -> Self {
        *self * (self.length().recip())
    }

    /// Promotes this Vec2 to a Vec3, using 0 for the Y axis.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/X0Y.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.x0y(), Vec3::new(1.0, 0.0, 1.0));
    /// ```
    #[inline]
    pub fn x0y(&self) -> Vec3 {
        Vec3 { x: self.x, y: 0.0, z: self.y }
    }

    /// Promotes this Vec2 to a Vec3, using 0 for the Z axis.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/XY0.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.xy0(), Vec3::new(1.0, 1.0, 0.0));
    /// ```
    #[inline]
    pub fn xy0(&self) -> Vec3 {
        Vec3 { x: self.x, y: self.y, z: 0.0 }
    }

    /// A transpose swizzle property, returns (y,x)
    /// <https://stereokit.net/Pages/StereoKit/Vec2/YX.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec2 = Vec2::new(1.1, 1.2);
    /// assert_eq!(vec2.yx(), Vec2::new(1.2, 1.1));
    /// ```
    #[inline]
    pub fn yx(&self) -> Self {
        Self { x: self.y, y: self.x }
    }

    /// Calculates a signed angle between two vectors in degrees! Sign will be positive if B is counter-clockwise (left)
    /// of A, and negative if B is clockwise (right) of A. Vectors do not need to be normalized. NOTE: Since this will
    /// return a positive or negative angle, order of parameters matters!
    /// <https://stereokit.net/Pages/StereoKit/Vec2/AngleBetween.html>
    /// * `a` - The first, initial vector, A. Does not need to be normalized.
    /// * `b` - The second, final vector, B. Does not need to be normalized.
    ///
    /// Returns a signed angle between two vectors in degrees! Sign will be positive if B is counter-clockwise (left)
    /// of A, and negative if B is clockwise (right) of A.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::angle_between(vec2_a, vec2_b), 90.0);
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, -1.0);
    /// assert_eq!(Vec2::angle_between(vec2_a, vec2_b), 90.0);
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(-1.0, 0.0);
    /// assert_eq!(Vec2::angle_between(vec2_a, vec2_b), 180.0);
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(1.0, 0.0);
    /// assert_eq!(Vec2::angle_between(vec2_a, vec2_b), 0.0);
    /// ```
    #[inline]
    pub fn angle_between(a: Self, b: Self) -> f32 {
        (Self::dot(a, b) / (a.length_sq() * b.length_sq()).sqrt()).acos().to_degrees()
    }

    /// Creates a normalized delta vector that points out from an origin point to a target point!
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Direction.html>
    /// * `to` - The target point.
    /// * `from` - And the origin point!
    ///
    /// Returns direction from one point to another.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::direction(vec2_b, vec2_a), Vec2::new(-0.70710677, 0.70710677));
    /// ```
    #[inline]
    pub fn direction(to: Self, from: Self) -> Self {
        (to - from).get_normalized()
    }

    /// Calculates the distance between two points in space! Make sure they’re in the same coordinate space! Uses a Sqrt,
    /// so it’s not blazing fast, prefer DistanceSq when possible.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Distance.html>
    /// * `a` - The first point.
    /// * `b` - And the second point.
    ///
    /// Returns distance between the two points.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::distance(vec2_a, vec2_b), (2.0f32).sqrt());
    /// ```
    #[inline]
    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    /// Calculates the distance between two points in space, but leaves them squared! Make sure they’re in the same
    /// coordinate space! This is a fast function :)
    /// <https://stereokit.net/Pages/StereoKit/Vec2/DistanceSq.html>
    /// - `a` - The first point.
    /// - `b` - The second point.
    ///
    /// Returns distance between the two points, but squared.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::distance_sq(vec2_a, vec2_b), 2.0);
    /// ```
    #[inline]
    pub fn distance_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    /// The dot product is an extremely useful operation! One major use is to determine how similar two vectors are.
    /// If the vectors are Unit vectors (magnitude/length of 1), then the result will be 1 if the vectors are the same,
    /// -1 if they’re opposite, and a gradient in-between with 0 being perpendicular.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Dot.html>
    /// * `a` - The first vector.
    /// * `b` - The second vector.
    ///
    /// Returns the dot product!
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::dot(vec2_a, vec2_b), 0.0);
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(1.0, 0.0);
    /// assert_eq!(Vec2::dot(vec2_a, vec2_b), 1.0);
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(-1.0, 0.0);
    /// assert_eq!(Vec2::dot(vec2_a, vec2_b), -1.0);
    /// ```
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y)
    }

    /// Creates a vector pointing in the direction of the angle, with a length of 1. Angles are counter-clockwise, and
    /// start from (1,0), so an angle of 90 will be (0,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec2/FromAngle.html>
    /// * `degrees` - Counter-clockwise angle from(1,0) in degrees.
    ///
    /// Returns a unit vector (length of 1), pointing towards degrees.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::from_angles(0.0);
    /// assert_eq!(vec2, Vec2::new(1.0, 0.0));
    ///
    /// let vec2 = Vec2::from_angles(90.0);
    /// assert_eq!(vec2, Vec2::new(0.0, 1.0));
    ///
    /// let vec2 = Vec2::from_angles(180.0);
    /// assert_eq!(vec2, Vec2::new(-1.0, 0.0));
    ///
    /// let vec2 = Vec2::from_angles(270.0);
    /// assert_eq!(vec2, Vec2::new(0.0, -1.0));
    /// ```
    #[inline]
    pub fn from_angles(degree: f32) -> Self {
        Self { x: degree.to_radians().cos(), y: degree.to_radians().sin() }
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b.
    /// Doesn’t clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Lerp.html>
    /// * `a` - The first vector to blend, or '0.0' blend.
    /// * `b` - The second vector to blend, or '1.0' blend.
    /// * `blend` - A value between 0 and 1. Can be outside this range, it’ll just interpolate outside of the a, b range.
    ///
    /// Returns an unclamped blend of a and b.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.0);
    /// let vec2_b = Vec2::new(0.0, 1.0);
    /// assert_eq!(Vec2::lerp(vec2_a, vec2_b, 0.5), Vec2::new(0.5, 0.5));
    /// ```
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each element is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Max.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns the maximum value for each corresponding vector pair.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.5);
    /// let vec2_b = Vec2::new(0.9, 1.2);
    /// assert_eq!(Vec2::max(vec2_a, vec2_b), Vec2::new(1.0, 1.2));
    /// ```
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y) }
    }

    /// Returns a vector where each element is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec2/Min.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns the minimum value for each corresponding vector pair.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 0.2);
    /// let vec2_b = Vec2::new(0.1, 1.0);
    /// assert_eq!(Vec2::min(vec2_a, vec2_b), Vec2::new(0.1, 0.2));
    /// ```
    #[inline]
    pub fn min(a: Self, b: Self) -> Self {
        Self { x: f32::min(a.x, b.x), y: f32::min(a.y, b.y) }
    }

    /// Absolute value of each component, this may be usefull in some case
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(-1.4, 1.2);
    /// assert_eq!(vec2.abs(), Vec2::new(1.4, 1.2));
    /// ```
    #[inline]
    pub fn abs(&self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs() }
    }
}

impl Display for Vec2 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks like “[x, y]”
    /// <https://stereokit.net/Pages/StereoKit/Vec2/ToString.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2 = Vec2::new(1.0, 1.0);
    /// assert_eq!(vec2.to_string(), "[x:1, y:1]");
    /// ```
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}]", self.x, self.y)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl Div<Vec2> for Vec2 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// assert_eq!(vec2_a / vec2_b, Vec2::new(0.5, 0.5));
    /// ```
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl DivAssign<Vec2> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// vec2_a /= vec2_b;
    /// assert_eq!(vec2_a, Vec2::new(0.5, 0.5));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// assert_eq!(vec2_a / 2.0, Vec2::new(0.5, 1.0));
    /// ```
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Division.html>
impl DivAssign<f32> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2_a = Vec2::new(1.0, 2.0);
    /// vec2_a /= 2.0;
    /// assert_eq!(vec2_a, Vec2::new(0.5, 1.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// assert_eq!(2.0 / vec2_a, Vec2::new(2.0, 1.0));
    /// ```
    #[inline]
    fn div(self, rhs: Vec2) -> Self::Output {
        Vec2 { x: self.div(rhs.x), y: self.div(rhs.y) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl Mul<Vec2> for Vec2 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// assert_eq!(vec2_a * vec2_b, Vec2::new(2.0, 8.0));
    /// ```
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl MulAssign<Vec2> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// let mut vec2_b = Vec2::new(2.0, 4.0);
    /// vec2_b *= vec2_a;
    /// assert_eq!(vec2_b, Vec2::new(2.0, 8.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// assert_eq!(vec2_a * 2.0, Vec2::new(2.0, 4.0));
    /// ```
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Multiply.html>
impl MulAssign<f32> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2_a = Vec2::new(1.0, 2.0);
    /// vec2_a *= 2.0;
    /// assert_eq!(vec2_a, Vec2::new(2.0, 4.0));
    ///```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// assert_eq!(2.0 * vec2_a, Vec2::new(2.0, 4.0));
    /// ```
    #[inline]
    fn mul(self, rhs: Vec2) -> Self::Output {
        Vec2 { x: self.mul(rhs.x), y: self.mul(rhs.y) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Addition.html>
impl Add<Vec2> for Vec2 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// assert_eq!(vec2_a + vec2_b, Vec2::new(3.0, 6.0));
    /// ```
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Addition.html>
impl AddAssign<Vec2> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// vec2_a += vec2_b;
    /// assert_eq!(vec2_a, Vec2::new(3.0, 6.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// assert_eq!(vec2_a - vec2_b, Vec2::new(-1.0, -2.0));
    /// ```
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec2/op_Subtraction.html>
impl SubAssign<Vec2> for Vec2 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let mut vec2_a = Vec2::new(1.0, 2.0);
    /// let vec2_b = Vec2::new(2.0, 4.0);
    /// vec2_a -= vec2_b;
    /// assert_eq!(vec2_a, Vec2::new(-1.0, -2.0));
    /// ```
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec2;
    ///
    /// let vec2_a = Vec2::new(1.0, 2.0);
    /// assert_eq!(-vec2_a, Vec2::new(-1.0, -2.0));
    /// ```
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
/// ### Examples
/// ```
/// use stereokit_rust::maths::Vec3;
///
/// let vec3        = Vec3::new(1.0, 2.0, 3.0);
/// let vec3a       = Vec3 { x: 1.0, y: 2.0, z:3.0 };
/// let vec3b       = Vec3::X + Vec3::Y * 2.0;
/// let vec3c: Vec3 = [1.0, 2.0, 3.0].into();
///
/// assert_eq!(vec3, vec3a);
/// assert_eq!(vec3a + vec3b,                   Vec3 { x: 2.0, y: 4.0, z: 3.0 });
/// assert_eq!(vec3c,                           Vec3 { x: 1.0, y: 2.0, z: 3.0 });
/// assert_eq!(vec3a.length_sq(),               14.0);
/// assert_eq!(vec3a.magnitude(),               (14.0f32).sqrt());
/// assert!   (vec3a.get_normalized().length() - 1.0 < f32::EPSILON);
/// assert_eq!(Vec3::angle_between(Vec3::X, Vec3::Y), 90.0);
/// assert_eq!(Vec3::dot(Vec3::X, Vec3::Y), 0.0);
/// ```
#[derive(Debug, Default, Copy, Clone)]
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

///  Warning: Equality with a precision of 0.1 millimeter
impl PartialEq for Vec3 {
    ///  Warning: Equality with a precision of 0.1 millimeter
    /// ### Example
    /// ```
    /// use stereokit_rust::maths::Vec3;
    /// assert_eq!(
    ///              Vec3 { x: 0.045863353, y: 0.030000005, z: 0.0 } ,
    ///              Vec3 { x: 0.045863353, y: 0.030000005, z: 0.0 } );
    /// ```
    /// ```
    /// use stereokit_rust::maths::Vec3;
    /// assert_ne!(
    ///              Vec3 { x: 10.045863353, y: 0.030000005, z: 0.0 } ,
    ///              Vec3 { x: 0.045863353, y: 0.030000005, z: 0.0 } );
    /// ```
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < 0.0001)
            && ((self.y - other.y).abs() < 0.0001)
            && ((self.z - other.z).abs() < 0.0001)
    }
}

unsafe extern "C" {
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
    /// * `point`- The point to check against.
    /// * `radius` - The radius to check within.
    ///
    /// Returns true if the points are within radius of each other, false not.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// let vec3_b = Vec3::new(2.0, 4.0, 6.0);
    /// assert_eq!(vec3_a.in_radius(vec3_b, 5.0), true);
    /// assert_eq!(vec3_a.in_radius(vec3_b, 2.0), false);
    /// ```
    #[inline]
    pub fn in_radius(&self, point: Self, radius: f32) -> bool {
        Self::distance(*self, point) <= radius
    }

    /// Turns this vector into a normalized vector (vector with a length of 1) from the current vector. Will not work
    /// properly if the vector has a length of zero. Vec3::get_normalized is faster.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Normalize.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// vec3_a.normalize();
    /// assert_eq!(vec3_a, Vec3::new(0.26726124, 0.5345225, 0.8017837));
    /// ```
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
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.length(), 3.7416575);
    /// ```
    #[inline]
    pub fn length(&self) -> f32 {
        Self::dot(*self, *self).sqrt()
    }

    /// This is the squared length of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/LengthSq.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.length_sq(), 14.0);
    /// ```
    #[inline]
    pub fn length_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    /// Magnitude is the length of the vector! The distance from the origin to this point. Uses f32::sqrt, so it’s not
    /// dirt cheap or anything.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Magnitude.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.length(), 3.7416575);
    /// ```
    #[inline]
    pub fn magnitude(&self) -> f32 {
        self.length()
    }

    /// This is the squared magnitude of the vector! It skips the Sqrt call, and just gives you the squared version for
    /// speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/MagnitudeSq.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.magnitude_squared(), 14.0);
    /// ```
    #[inline]
    pub fn magnitude_squared(&self) -> f32 {
        self.length_sq()
    }

    /// Creates a normalized vector (vector with a length of 1) from the current vector. Will not work properly if the
    /// vector has a length of zero.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Normalized.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.get_normalized(), Vec3::new(0.26726124, 0.5345225, 0.8017837));
    /// ```
    #[inline]
    pub fn get_normalized(&self) -> Self {
        *self * (self.length().recip())
    }

    /// This returns a Vec3 that has been flattened to 0 on the Y axis. No other changes are made.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/X0Z.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.x0z(), Vec3::new(1.0, 0.0, 3.0));
    /// ```
    #[inline]
    pub fn x0z(&self) -> Self {
        Self { x: self.x, y: 0.0, z: self.z }
    }

    /// This returns a Vec3 that has been flattened to 0 on the Z axis. No other changes are made.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/XY0.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.xy0(), Vec3::new(1.0, 2.0, 0.0));
    /// ```
    #[inline]
    pub fn xy0(&self) -> Self {
        Self { x: self.x, y: self.y, z: 0.0 }
    }

    /// This returns a Vec3 that has been set to 1 on the Z axis. No other changes are made
    /// <https://stereokit.net/Pages/StereoKit/Vec3/XY1.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.xy1(), Vec3::new(1.0, 2.0, 1.0));
    /// ```
    #[inline]
    pub fn xy1(&self) -> Self {
        Self { x: self.x, y: self.y, z: 1.0 }
    }

    /// This extracts the Vec2 from the X and Y axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.xy(), Vec2::new(1.0, 2.0));
    /// ```
    #[inline]
    pub fn xy(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// This extracts the Vec2 from the Y and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/YZ.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.yz(), Vec2::new(2.0, 3.0));
    /// ```
    #[inline]
    pub fn yz(&self) -> Vec2 {
        Vec2::new(self.y, self.z)
    }

    /// This extracts the Vec2 from the X and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec3.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3};
    ///
    /// let vec3_a = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(vec3_a.xz(), Vec2::new(1.0, 3.0));
    /// ```
    #[inline]
    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }

    /// Calculates the angle between two vectors in degrees! Vectors do not need to be normalized, and the result will
    /// always be positive.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleBetween.html>
    /// * `a` - The first, initial vector, A. Does not need to be normalized.
    /// * `b` - The second vector, B, that we're finding the angle to. Does not need to be normalized.
    ///
    /// Returns a positive angle between two vectors in degrees!
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let vec3_b = Vec3::new(0.0, 1.0, 0.0);
    /// assert_eq!(Vec3::angle_between(Vec3::X, vec3_b), 90.0);
    ///
    /// let vec3_c = Vec3::new(-1.0, 0.0, 0.0);
    /// assert_eq!(Vec3::angle_between(Vec3::X, vec3_c), 180.0);
    ///
    /// let vec3_d = Vec3::new(1.0, 0.0, 1.0);
    /// assert_eq!(Vec3::angle_between(Vec3::X, vec3_d), 45.0);
    /// ```
    #[inline]
    pub fn angle_between(a: Self, b: Self) -> f32 {
        (Self::dot(a, b) / (a.length_sq() * b.length_sq()).sqrt()).acos().to_degrees()
    }

    /// Creates a vector that points out at the given 2D angle! This creates the vector on the XY plane, and allows you
    /// to specify a constant z value.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleXY.html>
    /// * `angle_deg` - The angle in degrees, starting from the (1,0) at 0, and continuing to (0,1) at 90, etc.
    /// * `z` - The constant Z value for this vector.
    ///
    /// Returns a vector pointing at the given angle! If z is zero, this will be a normalized vector (vector with
    /// a length of 1).
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let angle = 45.0;
    /// let z = 0.0;
    /// let vector = Vec3::angle_xy(angle, z);
    /// assert_eq!(vector, Vec3::new(0.70710677, 0.70710677, 0.0));
    /// ```
    #[inline]
    pub fn angle_xy(angle_deg: f32, z: f32) -> Vec3 {
        Self { x: angle_deg.to_radians().cos(), y: angle_deg.to_radians().sin(), z }
    }

    /// Creates a vector that points out at the given 2D angle! This creates the vector on the XZ plane, and allows you
    /// to specify a constant y value.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/AngleXZ.html>
    /// * `angle_deg` - The angle in degrees, starting from the (1,0) at 0, and continuing to (0,1) at 90, etc.
    /// * `y` - The constant Y value for this vector.
    ///
    /// Returns A Vector pointing at the given angle! If y is zero, this will be a normalized vector (vector with a
    /// length of 1).
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    /// let vec = Vec3::angle_xz(90.0, 0.0);
    /// assert_eq!(vec, Vec3::new(0.0, 0.0, 1.0));
    /// ```
    #[inline]
    pub fn angle_xz(angle_deg: f32, y: f32) -> Self {
        Self { x: angle_deg.to_radians().cos(), y, z: angle_deg.to_radians().sin() }
    }

    /// The cross product of two vectors!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Cross.html>
    /// * `a` - The first vector.
    /// * `b` - The second vector.
    ///
    /// Returns is **not** a unit vector, even if both ‘a’ and ‘b’ are unit vectors.
    /// see also [`vec3_cross`]
    /// ### Examples
    ///```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// // Test case 1: Basic cross product
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.0, 1.0, 0.0);
    /// let result = Vec3::cross(a, b);
    /// assert_eq!(result, Vec3::new(0.0, 0.0, 1.0));
    ///
    /// // Test case 2: Cross product in different direction
    /// let a = Vec3::new(0.0, 1.0, 0.0);
    /// let b = Vec3::new(1.0, 0.0, 0.0);
    /// let result = Vec3::cross(a, b);
    /// assert_eq!(result, Vec3::new(0.0, 0.0, -1.0));
    ///
    /// // Test case 3: Cross product with non-unit vectors
    /// let a = Vec3::new(2.0, 0.0, 0.0);
    /// let b = Vec3::new(0.0, 3.0, 0.0);
    /// let result = Vec3::cross(a, b);
    /// assert_eq!(result, Vec3::new(0.0, 0.0, 6.0));
    ///
    /// // Test case 4: Cross product of parallel vectors
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(2.0, 0.0, 0.0);
    /// let result = Vec3::cross(a, b);
    /// assert_eq!(result, Vec3::ZERO);
    ///
    /// // Test case 5: cross product of orthogonal vector
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let result = Vec3::cross(a,b);
    /// assert_eq!(result, Vec3::new(-3.0, 6.0, -3.0));
    ///
    /// // Test case 6: cross product of orthogonal vector
    /// let a = Vec3::new(4.0, 5.0, 6.0);
    /// let b = Vec3::new(1.0, 2.0, 3.0);
    /// let result = Vec3::cross(a,b);
    /// assert_eq!(result, Vec3::new(3.0, -6.0, 3.0));
    /// ```
    #[inline]
    pub fn cross(a: Self, b: Self) -> Self {
        unsafe { vec3_cross(&a, &b) }
    }

    /// Creates a normalized delta vector that points out from an origin point to a target point!
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Direction.html>
    /// * `to` - The target point.
    /// * `from` - The origin point.
    ///
    /// Returns direction from one point to another.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let direction = Vec3::direction(a, b);
    /// assert_eq!(direction, Vec3 { x: -0.57735026, y: -0.57735026, z: -0.57735026 });
    /// ```
    #[inline]
    pub fn direction(to: Self, from: Self) -> Self {
        (to - from).get_normalized()
    }

    /// Calculates the distance between two points in space! Make sure they’re in the same coordinate space! Uses a
    /// Sqrt, so it’s not blazing fast, prefer DistanceSq when possible.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Distance.html>
    /// * `a` - The first point.
    /// * `b` - The second point.
    ///
    /// Returns the distance between the two points.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    /// let b = Vec3 { x: 4.0, y: 5.0, z: 6.0 };
    /// let distance = Vec3::distance(a, b);
    /// assert_eq!(distance, 5.196152);
    /// ```
    #[inline]
    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    /// Calculates the distance between two points in space, but leaves them squared! Make sure they’re in the same
    /// coordinate space! This is a fast function :)
    /// <https://stereokit.net/Pages/StereoKit/Vec3/DistanceSq.html>
    /// * `a` - The first point.
    /// * `b` - The second point.
    ///
    /// Returns the distance between the two points, but squared.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let distance = Vec3::distance_sq(a, b);
    /// assert_eq!(distance, 27.0);
    /// ```
    #[inline]
    pub fn distance_sq(a: Self, b: Self) -> f32 {
        (a - b).length_sq()
    }

    /// The dot product is an extremely useful operation! One major use is to determine how similar two vectors are. If
    /// the vectors are Unit vectors (magnitude/length of 1), then the result will be 1 if the vectors are the same, -1
    /// if they’re opposite, and a gradient in-between with 0 being perpendicular.  See [Freya Holmer’s excellent
    /// visualization of this concept](<https://twitter.com/FreyaHolmer/status/1200807790580768768>)
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Dot.html>
    /// * `a` - The first vector.
    /// * `b` - The second vector.
    ///
    /// Returns the dot product of the two vectors.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 1.0);
    /// let b = Vec3::new(1.0, 1.0, 0.0);
    /// let dot_product = Vec3::dot(a, b);
    /// assert_eq!(dot_product, 1.0);
    /// ```
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z)
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b. Doesn’t
    /// clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Lerp.html>
    /// * `a` - First item in the blend, or '0.0' blend.
    /// * `b` - Second item in the blend, or '1.0' blend.
    /// * `blend` - The blend value between 0 and 1. Can be outside this range, it’ll just interpolate outside of the a,
    ///   b range.
    ///
    /// Returns a blend value between 0 and 1. Can be outside this range, it’ll just interpolate outside of the a, b
    /// range.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let blend = 0.25;
    /// let result = Vec3::lerp(a, b, blend);
    /// assert_eq!(result, Vec3::new(1.75, 2.75, 3.75));
    /// ```
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each element is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Max.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns the maximum value for each corresponding vector pair.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 6.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let result = Vec3::max(a, b);
    /// assert_eq!(result, Vec3::new(4.0, 6.0, 6.0));
    /// ```
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y), z: f32::max(a.z, b.z) }
    }

    /// Returns a vector where each element is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Min.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns the minimum value for each corresponding vector pair.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 1.0, 6.0);
    /// let result = Vec3::min(a, b);
    /// assert_eq!(result, Vec3::new(1.0, 1.0, 3.0));
    /// ```
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
    /// * `forward` - What way are you facing?
    /// * `up` - Which direction is up?
    ///
    /// Returns your direction to the right! Result is -not- a unit vector, even if both ‘forward’ and ‘up’ are unit
    /// vectors.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let forward = Vec3::new(0.0, 0.0, -1.0);
    /// let up = Vec3::new(0.0, 1.0, 0.0);
    /// let right = Vec3::perpendicular_right(forward, up);
    /// assert_eq!(right, Vec3::new(1.0, 0.0, 0.0));
    ///
    /// // The same with constants:
    /// let forward = Vec3::FORWARD;
    /// let up = Vec3::UP;
    /// let right = Vec3::perpendicular_right(forward, up);
    /// assert_eq!(right, Vec3::RIGHT);
    /// ```
    #[inline]
    pub fn perpendicular_right(forward: Self, up: Self) -> Self {
        Self::cross(forward, up)
    }

    /// Returns a vector where each element is the absolute value of the corresponding element.
    /// <https://stereokit.net/Pages/StereoKit/Vec3/Abs.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(-1.0, 2.0, -3.0);
    /// assert_eq!(v.abs(), Vec3::new(1.0, 2.0, 3.0));
    /// ```
    #[inline]
    pub fn abs(&self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }

    /// get an array
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(v.to_array(), [1.0, 2.0, 3.0]);
    ///```
    #[inline]
    pub const fn to_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Display for Vec3 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode.
    /// Looks like “[x, y, z]”
    /// <https://stereokit.net/Pages/StereoKit/Vec3/ToString.html>
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(v.to_string(), "[x:1, y:2, z:3]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}]", self.x, self.y, self.z)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl Div<Vec3> for Vec3 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0) / Vec3::new(2.0, 2.0, 2.0);
    /// assert_eq!(v, Vec3::new(0.5, 1.0, 1.5));
    /// ```
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y), z: self.z.div(rhs.z) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl DivAssign<Vec3> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut v = Vec3::new(1.0, 2.0, 3.0);
    /// v /= Vec3::new(2.0, 2.0, 2.0);
    /// assert_eq!(v, Vec3::new(0.5, 1.0, 1.5));
    /// ```
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

    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// let v = v / 2.0;
    /// assert_eq!(v, Vec3::new(0.5, 1.0, 1.5));
    /// ```
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs), z: self.z.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Division.html>
impl DivAssign<f32> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut v = Vec3::new(1.0, 2.0, 3.0);
    /// v /= 2.0;
    /// assert_eq!(v, Vec3::new(0.5, 1.0, 1.5));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let v = Vec3::new(1.0, 2.0, 3.0);
    /// let v = v / Vec3::new(2.0, 2.0, 2.0);
    /// assert_eq!(v, Vec3::new(0.5, 1.0, 1.5));
    /// ```
    #[inline]
    fn div(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.div(rhs.x), y: self.div(rhs.y), z: self.div(rhs.z) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl Mul<Vec3> for Vec3 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let c = a * b;
    /// assert_eq!(c, Vec3::new(4.0, 10.0, 18.0));
    /// ```
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y), z: self.z.mul(rhs.z) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl MulAssign<Vec3> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// a *= b;
    /// assert_eq!(a, Vec3::new(4.0, 10.0, 18.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = a * 2.0;
    /// assert_eq!(b, Vec3::new(2.0, 4.0, 6.0));
    /// ```
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs), z: self.z.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Multiply.html>
impl MulAssign<f32> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut a = Vec3::new(1.0, 2.0, 3.0);
    /// a *= 2.0;
    /// assert_eq!(a, Vec3::new(2.0, 4.0, 6.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = 2.0 * Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(2.0, 4.0, 6.0);
    /// assert_eq!(a, b);
    /// ```
    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.mul(rhs.x), y: self.mul(rhs.y), z: self.mul(rhs.z) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Addition.html>
impl Add<Vec3> for Vec3 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let c = a + b;
    /// assert_eq!(c, Vec3::new(5.0, 7.0, 9.0));
    /// ```
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y), z: self.z.add(rhs.z) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Addition.html>
impl AddAssign<Vec3> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// a += b;
    /// assert_eq!(a, Vec3::new(5.0, 7.0, 9.0));
    /// ```
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
        self.z.add_assign(rhs.z);
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Self;

    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = Vec3::new(4.0, 5.0, 6.0);
    /// let c = a - b;
    /// assert_eq!(c, Vec3::new(-3.0, -3.0, -3.0));
    /// ```
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y), z: self.z.sub(rhs.z) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec3/op_Subtraction.html>
impl SubAssign<Vec3> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let mut a = Vec3::new(1.0, 3.0, 1.0);
    /// let b = Vec3::new(0.5, 0.5, 0.5);
    /// a -= b;
    /// assert_eq!(a, Vec3::new(0.5, 2.5, 0.5));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec3;
    ///
    /// let a = Vec3::new(1.0, 2.0, 3.0);
    /// let b = -a;
    /// assert_eq!(b, Vec3::new(-1.0, -2.0, -3.0));
    /// ```
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
/// ### Examples
/// ```
/// use stereokit_rust::maths::Vec4;
///
/// let vec4        = Vec4::new(1.0, 2.0, 3.0, 4.0);
/// let vec4a       = Vec4 { x: 1.0, y: 2.0, z:3.0, w:4.0 };
/// let vec4b       = Vec4::X + Vec4::Y * 2.0;
/// let vec4c: Vec4 = [1.0, 2.0, 3.0, 4.0].into();
///
/// assert_eq!(vec4, vec4a);
/// assert_eq!(vec4a + vec4b,                   Vec4 { x: 2.0, y: 4.0, z: 3.0, w: 4.0 });
/// assert_eq!(vec4c,                           Vec4 { x: 1.0, y: 2.0, z: 3.0, w: 4.0 });
/// assert_eq!(vec4a.length_sq(),               30.0);
/// assert_eq!(Vec4::dot(Vec4::X, Vec4::Y),     0.0);
/// ```
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
impl From<[f32; 4]> for Vec4 {
    fn from(val: [f32; 4]) -> Self {
        Vec4 { x: val[0], y: val[1], z: val[2], w: val[3] }
    }
}

impl From<Vec4> for glam::Vec4 {
    fn from(value: Vec4) -> Self {
        Self::new(value.x, value.y, value.z, value.w)
    }
}

///  Warning: Equality with a precision of 0.1 millimeter
impl PartialEq for Vec4 {
    ///  Warning: Equality with a precision of 0.1 millimeter
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < 0.0001)
            && ((self.y - other.y).abs() < 0.0001)
            && ((self.z - other.z).abs() < 0.0001)
            && ((self.w - other.w).abs() < 0.0001)
    }
}

impl Vec4 {
    /// all components to 0
    pub const ZERO: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };

    /// all components to 1
    pub const ONE: Vec4 = Vec4 { x: 1.0, y: 1.0, z: 1.0, w: 1.0 };

    /// A normalized Vector that points down the X axis, this is the same as new Vec4(1,0,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitX.html>    
    pub const X: Vec4 = Vec4 { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points down the Y axis, this is the same as new Vec4(0,1,0,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitY.html>    
    pub const Y: Vec4 = Vec4 { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points down the Z axis, this is the same as new Vec4(0,0,1,0).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitZ.html>    
    pub const Z: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };

    /// A normalized Vector that points down the W axis, this is the same as new Vec4(0,0,0,1).
    /// <https://stereokit.net/Pages/StereoKit/Vec4/UnitW.html>
    pub const W: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    /// A normalized Vector that points up the X axis, this is the same as new Vec4(1,0,0,0).
    pub const NEG_X: Vec4 = Vec4 { x: -1.0, y: 0.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points up the Y axis, this is the same as new Vec4(0,1,0,0).
    pub const NEG_Y: Vec4 = Vec4 { x: 0.0, y: -1.0, z: 0.0, w: 0.0 };

    /// A normalized Vector that points up the Z axis, this is the same as new Vec4(0,0,1,0).
    pub const NEG_Z: Vec4 = Vec4 { x: 0.0, y: 0.0, z: -1.0, w: 0.0 };

    /// A normalized Vector that points up the W axis, this is the same as new Vec4(0,0,0,1).
    pub const NEG_W: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: -1.0 };

    /// <https://stereokit.net/Pages/StereoKit/Vec4/Vec4.html>
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// This extracts the Vec2 from the X and Y axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XY.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec4};
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let xy = v.xy();
    /// assert_eq!(xy, Vec2::new(1.0, 2.0));
    /// ```
    #[inline]
    pub fn xy(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// This extracts the Vec2 from the Y and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/YZ.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec4};
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let yz = v.yz();
    /// assert_eq!(yz, Vec2::new(2.0, 3.0));
    /// ```
    #[inline]
    pub fn yz(&self) -> Vec2 {
        Vec2::new(self.y, self.z)
    }

    /// This extracts the Vec2 from the X and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XZ.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec4};
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let xz = v.xz();
    /// assert_eq!(xz, Vec2::new(1.0, 3.0));
    /// ```
    #[inline]
    pub fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }

    /// This extracts the Vec2 from the Z and W axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/ZW.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec4};
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let zw = v.zw();
    /// assert_eq!(zw, Vec2::new(3.0, 4.0));
    /// ```
    #[inline]
    pub fn zw(&self) -> Vec2 {
        Vec2::new(self.z, self.w)
    }

    /// This extracts a Vec3 from the X, Y, and Z axes.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/XYZ.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Vec4};
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let v3 = v.xyz();
    /// assert_eq!(v3, Vec3::new(1.0, 2.0, 3.0));
    /// ```
    #[inline]
    pub fn xyz(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// A Vec4 and a Quat are only really different by name and purpose. So, if you need to do Quat math with your
    /// Vec4, or visa versa, who am I to judge?
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Quat.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec4, Quat};
    ///
    /// let vec4 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let quat = vec4.get_as_quat();
    /// assert_eq!(quat, Quat::new(1.0, 2.0, 3.0, 4.0));
    /// ```
    #[inline]
    pub fn get_as_quat(&self) -> Quat {
        Quat { x: self.x, y: self.y, z: self.z, w: self.w }
    }

    /// This is the squared length/magnitude of the vector! It skips the Sqrt call, and just gives you the squared
    /// version for speedy calculations that can work with it squared.
    /// <https://stereokit.net/Pages/StereoKit/Vec4.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(v.length_sq(), 30.0);
    /// ```
    #[inline]
    pub fn length_sq(&self) -> f32 {
        Self::dot(*self, *self)
    }

    /// What’s a dot product do for 4D vectors, you might ask? Well, I’m no mathematician, so hopefully you are! I’ve
    /// never used it before. Whatever you’re doing with this function, it’s SIMD fast!
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Dot.html>
    /// * `a` - The first vector.
    /// * `b` - The second vector.
    ///
    /// Returns the dot product of the two vectors.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
    /// let result = Vec4::dot(a, b);
    /// assert_eq!(result, 70.0);
    /// ```
    #[inline]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z) + (a.w * b.w)
    }

    /// Blends (Linear Interpolation) between two vectors, based on a ‘blend’ value, where 0 is a, and 1 is b. Doesn’t
    /// clamp percent for you.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Lerp.html>
    /// * `a` - First item in the blend, or '0.0 blend.
    /// * `b` - Second item in the blend, or '1.0 blend.
    /// * `blend` - A blend value between 0 and 1. Can be outside this range, it’ll just interpolate outside of the a, b
    ///   range.
    ///
    /// Returns an unclamped blend of a and b.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    /// let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
    /// let b = Vec4::new(1.0, 1.0, 1.0, 1.0);
    /// let result = Vec4::lerp(a, b, 0.75);
    /// assert_eq!(result, Vec4::new(0.75, 0.75, 0.75, 0.75));
    /// ```
    #[inline]
    pub fn lerp(a: Self, b: Self, blend: f32) -> Self {
        a + ((b - a) * blend)
    }

    /// Returns a vector where each element is the maximum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Max.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns a vector where each element is the maximum value for each corresponding pair.
    /// #### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(4.0, 3.0, 2.0, 1.0);
    /// let c = Vec4::max(a, b);
    /// assert_eq!(c, Vec4::new(4.0, 3.0, 3.0, 4.0));
    /// ```
    #[inline]
    pub fn max(a: Self, b: Self) -> Self {
        Self { x: f32::max(a.x, b.x), y: f32::max(a.y, b.y), z: f32::max(a.z, b.z), w: f32::max(a.w, b.w) }
    }

    /// Returns a vector where each element is the minimum value for each corresponding pair.
    /// <https://stereokit.net/Pages/StereoKit/Vec4/Min.html>
    /// * `a` - Order isn't important here.
    /// * `b` - Order isn't important here.
    ///
    /// Returns a new vector with the minimum values.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(4.0, 3.0, 2.0, 1.0);
    /// let c = Vec4::min(a, b);
    /// assert_eq!(c, Vec4::new(1.0, 2.0, 2.0, 1.0));
    /// ```
    #[inline]
    pub fn min(a: Self, b: Self) -> Self {
        Self { x: f32::min(a.x, b.x), y: f32::min(a.y, b.y), z: f32::min(a.z, b.z), w: f32::min(a.w, b.w) }
    }
}

impl Display for Vec4 {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks
    /// like “[x, y, z, w]”
    /// <https://stereokit.net/Pages/StereoKit/Vec4/ToString.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.2, 3.0, 4.0);
    /// assert_eq!(format!("{}", v), "[x:1, y:2.2, z:3, w:4]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}, w:{}]", self.x, self.y, self.z, self.w)
    }
}
/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl Div<Vec4> for Vec4 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(2.0, 2.0, 2.0, 2.0);
    /// let c = a / b;
    /// assert_eq!(c, Vec4::new(0.5, 1.0, 1.5, 2.0));
    /// ```
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self { x: self.x.div(rhs.x), y: self.y.div(rhs.y), z: self.z.div(rhs.z), w: self.w.div(rhs.w) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl DivAssign<Vec4> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::*;
    ///
    /// let mut a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(2.0, 2.0, 2.0, 2.0);
    /// a /= b;
    /// assert_eq!(a, Vec4::new(0.5, 1.0, 1.5, 2.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = a / 2.0;
    /// assert_eq!(b, Vec4::new(0.5, 1.0, 1.5, 2.0));
    /// ```
    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        Self { x: self.x.div(rhs), y: self.y.div(rhs), z: self.z.div(rhs), w: self.w.div(rhs) }
    }
}

/// A component-wise vector division.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Division.html>
impl DivAssign<f32> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let mut v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// v /= 2.0;
    /// assert_eq!(v, Vec4::new(0.5, 1.0, 1.5, 2.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 7.0, 3.0, 4.0);
    /// let b = a / 2.0;
    /// assert_eq!(b, Vec4::new(0.5, 3.5, 1.5, 2.0));
    /// ```
    #[inline]
    fn div(self, rhs: Vec4) -> Self::Output {
        Vec4 { x: self.div(rhs.x), y: self.div(rhs.y), z: self.div(rhs.z), w: self.div(rhs.w) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl Mul<Vec4> for Vec4 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 5.0);
    /// let b = Vec4::new(2.0, 3.0, 4.0, 6.0);
    /// let c = a * b;
    /// assert_eq!(c, Vec4::new(2.0, 6.0, 12.0, 30.0));
    /// ```
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self { x: self.x.mul(rhs.x), y: self.y.mul(rhs.y), z: self.z.mul(rhs.z), w: self.w.mul(rhs.w) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl MulAssign<Vec4> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let mut a = Vec4::new(1.0, 2.0, 3.0, 5.0);
    /// let b = Vec4::new(2.0, 3.0, 4.0, 6.0);
    /// a *= b;
    /// assert_eq!(a, Vec4::new(2.0, 6.0, 12.0, 30.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 5.0);
    /// let b = a * 2.0;
    /// assert_eq!(b, Vec4::new(2.0, 4.0, 6.0, 10.0));
    /// ```
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Self { x: self.x.mul(rhs), y: self.y.mul(rhs), z: self.z.mul(rhs), w: self.w.mul(rhs) }
    }
}

/// A component-wise vector multiplication, same thing as a non-uniform scale. NOT a dot or cross product! Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Multiply.html>
impl MulAssign<f32> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let mut a = Vec4::new(1.0, 2.0, 7.0, 5.0);
    /// a *= 2.0;
    /// assert_eq!(a, Vec4::new(2.0, 4.0, 14.0, 10.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = 2.0;
    /// let b = Vec4::new(2.0, 3.0, 4.0, 6.0);
    /// let c = a * b;
    /// assert_eq!(c, Vec4::new(4.0, 6.0, 8.0, 12.0));
    /// ```
    #[inline]
    fn mul(self, rhs: Vec4) -> Self::Output {
        Vec4 { x: self.mul(rhs.x), y: self.mul(rhs.y), z: self.mul(rhs.z), w: self.mul(rhs.w) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Addition.html>
impl Add<Vec4> for Vec4 {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 5.0);
    /// let b = Vec4::new(2.0, 3.0, 4.0, 6.0);
    /// let c = a + b;
    /// assert_eq!(c, Vec4::new(3.0, 5.0, 7.0, 11.0));
    /// ```
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self { x: self.x.add(rhs.x), y: self.y.add(rhs.y), z: self.z.add(rhs.z), w: self.w.add(rhs.w) }
    }
}

/// Adds matching components together. Commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Addition.html>
impl AddAssign<Vec4> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let mut v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
    /// v1 += v2;
    /// assert_eq!(v1, Vec4::new(6.0, 8.0, 10.0, 12.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let b = Vec4::new(4.0, 3.0, 2.0, 1.0);
    /// let c = a - b;
    /// assert_eq!(c, Vec4::new(-3.0, -1.0, 1.0, 3.0));
    /// ```
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self { x: self.x.sub(rhs.x), y: self.y.sub(rhs.y), z: self.z.sub(rhs.z), w: self.w.sub(rhs.w) }
    }
}

/// Subtracts matching components from eachother. Not commutative.
/// <https://stereokit.net/Pages/StereoKit/Vec4/op_Subtraction.html>
impl SubAssign<Vec4> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let mut v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// v1 -= v2;
    /// assert_eq!(v1, Vec4::new(0.0, 0.0, 0.0, 0.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// let neg_v = -v;
    /// assert_eq!(neg_v, Vec4::new(-1.0, -2.0, -3.0, -4.0));
    /// ```
    #[inline]
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

/// Quaternions are efficient and robust mathematical objects for representing rotations! Understanding the details of
/// how a quaternion works is not generally necessary for using them effectively, so don’t worry too much if they seem
/// weird to you. They’re weird to me too.
///
/// If you’re interested in learning the details though, 3Blue1Brown and Ben Eater have an excellent interactive lesson
/// about them!
/// <https://stereokit.net/Pages/StereoKit/Quat.html>
///
///  see also [`glam::Quat`]
/// ### Examples
/// ```
/// use stereokit_rust::maths::{Quat, Vec3, Vec4};
///
/// let mut quat1 = Quat::new(0.0, 0.0, 0.0, 1.0);
/// let mut quat2 = Quat::from_angles(0.0, 90.0, 0.0);
/// let mut quat2b :Quat= [0.0, 90.0, 0.0].into();
/// let mut quat3 = Quat::look_at(Vec3::X, Vec3::ZERO, None);
/// let vec4_w: Vec4 = (quat2 - quat3).into();
///
/// // quat2 == quat3 but because of epsilon here is the demo:
/// assert_eq!(vec4_w.w,            1.0);
/// assert_eq!(vec4_w.length_sq(),  1.0);
/// assert_eq!(quat2b,              quat2);
///
/// assert_eq!(quat1,               Quat::IDENTITY);
/// assert_eq!(*quat1.normalize(),  Quat::IDENTITY);
/// assert_eq!(*quat1.invert(),     Quat::IDENTITY);
///
/// ```
#[repr(C)]
#[derive(Debug, Copy, Clone)]
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
impl From<[f32; 3]> for Quat {
    fn from(val: [f32; 3]) -> Self {
        Quat::from_angles(val[0], val[1], val[2])
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

impl Default for Quat {
    // Identity quaternion (no rotation)
    fn default() -> Self {
        Quat::IDENTITY
    }
}

///  Warning: Equality with a precision of 0.000001
impl PartialEq for Quat {
    ///  Warning: Equality with a precision of 0.00001
    /// ### Example
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let q0 = Quat::new(1.00002, 2.0, 3.0, 4.0);
    /// let q1 = Quat::new(1.000001, 2.000001, 3.000001, 4.0);
    /// let q2 = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// assert_ne!(q0, q1);
    /// assert_eq!(q1, q2);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < 0.00001)
            && ((self.y - other.y).abs() < 0.00001)
            && ((self.z - other.z).abs() < 0.00001)
            && ((self.w - other.w).abs() < 0.00001)
    }
}

unsafe extern "C" {
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

    /// This is a quaternion that represents a 180 degree rotation around the Y axis. It’s useful for
    /// representing a 180 degree turn in a 3D space, such as when you want to face the opposite direction.
    pub const Y_180: Self = Self { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };

    /// ZERO may be found when testing some [`crate::system::Input`], [`crate::system::Pointer`] or [`crate::system::Controller`]
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };

    /// You may want to use static creation methods, like Quat::look_at, or Quat::IDENTITY instead of this one! Unless you
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
    /// see also [`Quat::get_inverse`] [`quat_inverse`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let mut q = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// q.invert();
    /// assert_eq!(q, Quat { x: -0.033333335, y: -0.06666667, z: -0.1, w: 0.13333334 });
    /// ```
    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let m = unsafe { quat_inverse(self) };
        self.x = m.x;
        self.y = m.y;
        self.z = m.z;
        self.w = m.w;
        self
    }

    /// Return the conjugate
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let mut q = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// let q2 = q.conjugate();
    /// assert_eq!(q2, Quat { x: -1.0, y: -2.0, z: -3.0, w: 4.0 });
    /// ```
    #[inline]
    pub fn conjugate(&self) -> Self {
        Self { x: -self.x, y: -self.y, z: -self.z, w: self.w }
    }

    /// Normalize this quaternion with the same orientation, and a length of 1.
    /// Costly, see get_normalized for a faster way to get this.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Normalize.html>
    ///
    /// see also [`Quat::get_normalized`] [`quat_normalize`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let mut q = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// q.normalize();
    /// assert_eq!(q, Quat::new(0.18257419, 0.36514837, 0.54772256, 0.73029674));
    /// ```
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
    /// * `to` - The relative quaternion.
    ///
    /// Returns this quaternion made relative to another rotation.
    /// see also [`quat_mul`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let mut q = Quat::from_angles(0.0, 90.0, 0.0);
    /// assert_eq!(q, Quat { x: 0.0, y: 0.70710677, z: 0.0, w: 0.7071067 });
    ///
    /// let to = Quat::from_angles(0.0, 0.0, 90.0);
    /// q.relative(to);
    /// assert_eq!(q, Quat { x: 0.70710677, y: 0.0, z: 0.0, w: 0.7071067 });
    /// ```
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
    /// * `point` - The point to rotate around the origin.
    ///
    /// Returns the rotated point.
    /// see also [`quat_mul_vec`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Quat};
    ///
    /// let q = Quat::from_angles(0.0, 90.0, 0.0);
    /// let point = Vec3::new(1.0, 0.0, 0.0);
    /// let rotated_point = q.rotate_point(point);
    /// assert_eq!(rotated_point, Vec3::new(0.0, 0.0, -1.0));
    /// ```
    #[inline]
    pub fn rotate_point(&self, point: Vec3) -> Vec3 {
        unsafe { quat_mul_vec(self, &point) }
    }

    /// Get an array of the 3 angles in degrees of this Quat with x, y and z axis
    /// <https://stereokit.net/Pages/StereoKit/Quat.html>
    ///
    /// see also [`quat_to_axis_angle`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let quat = Quat::from_angles(90.0, 180.0, -180.0);
    ///
    /// let angles: Vec3  = quat.to_angles_degrees().into();
    /// assert_eq!(angles, [-270.0, 0.0, 0.0].into());
    /// ```
    #[inline]
    pub fn to_angles_degrees(&self) -> [f32; 3] {
        let mut axis = Vec3::ZERO;
        let mut rotation: f32 = 0.0;
        unsafe { quat_to_axis_angle(*self, &mut axis, &mut rotation) };
        axis.normalize();
        [rotation * axis.x, rotation * axis.y, rotation * axis.z]
    }

    /// Get an array of the 3 angles in radians of this Quat with x, y and z axis
    /// <https://stereokit.net/Pages/StereoKit/Quat.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let quat = Quat::from_angles(90.0, 0.0, 0.0);
    /// let angles = quat.to_angles();
    /// assert_eq!(angles, [1.5707964, 0.0, 0.0]);
    /// ```
    #[inline]
    pub fn to_angles(&self) -> [f32; 3] {
        self.to_angles_degrees().map(|x| x.to_radians())
    }

    /// This rotates a point around the origin by the Quat.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Rotate.html>
    /// * `a` - The Quat to use for rotation.
    /// * `point` - The point to rotate around the origin.
    ///
    /// Returns the rotated point.
    /// see also [`quat_mul_vec`] operator '*' [`Quat::rotate_point`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Quat};
    ///
    /// let a = Quat::from_angles(0.0, 90.0, 0.0);
    /// let point = Vec3::new(1.0, 0.0, 0.0);
    /// let result1 = a * point;
    /// let result2 = Quat::rotate(a, point);
    /// let result3 = a.rotate_point(point);
    /// assert_eq!(result1, result2);
    /// assert_eq!(result2, result3);
    /// assert_eq!(result1, Vec3::new(0.0, 0.0, -1.0));
    /// ```
    #[inline]
    pub fn rotate(a: Quat, point: Vec3) -> Vec3 {
        unsafe { quat_mul_vec(&a, &point) }
    }

    /// Creates a quaternion that goes from one rotation to another.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Delta.html>
    /// * `from` - The origin rotation.
    /// * `to` - The target rotation.
    ///
    /// Returns the quaternion between from and to.
    /// see also `-` operator [`quat_difference`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let from = Quat::from_angles(180.0, 0.0, 0.0);
    /// let to = Quat::from_angles(0.0, 180.0, 0.0);
    /// let delta = Quat::delta(from, to);
    /// assert_eq!(delta, Quat::from_angles(0.0, 0.0, 180.0));
    ///
    /// let delta_b = from - to;
    /// assert_eq!(delta_b, delta);
    /// ```
    #[inline]
    pub fn delta(from: Self, to: Self) -> Self {
        unsafe { quat_difference(&from, &to) }
    }

    /// Creates a rotation that goes from one direction to another. Which is comes in handy when trying to roll
    /// something around with position data.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Delta.html>
    /// * `from` - The origin direction.
    /// * `to` - The target direction.
    ///
    /// Returns the quaternion between the two directions.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let from = Vec3::new(1.0, 0.0, 0.0);
    /// let to = Vec3::new(0.0, 0.0, 1.0);
    /// let delta = Quat::delta_dir(from, to);
    /// assert_eq!(delta, Quat::from_angles(0.0, -90.0, 0.0));
    /// ```
    #[inline]
    pub fn delta_dir(from: Vec3, to: Vec3) -> Self {
        let c = Vec3::cross(from, to);
        let mut out = Quat { x: c.x, y: c.y, z: c.z, w: 1.0 + Vec3::dot(from, to) };
        *(out.normalize())
    }

    /// Creates a Roll/Pitch/Yaw rotation (applied in that order) from the provided angles in degrees! There is also a
    /// From<[f32; 3]> for Quat implementation that does the same thing.
    /// <https://stereokit.net/Pages/StereoKit/Quat/FromAngles.html>
    /// * `pitch_x_deg` - The angle to rotate around the X-axis in degrees.
    /// * `yaw_y_deg` - The angle to rotate around the Y-axis in degrees.
    /// * `roll_z_deg` - The angle to rotate around the Z-axis in degrees.
    ///
    /// Returns a quaternion representing the given Roll/Pitch/Yaw rotation!
    /// see also [`quat_from_angles`] [`Quat::from`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let quat = Quat::from_angles(45.0, 30.0, 60.0);
    /// let quat2: Quat = [45.0, 30.0, 60.0].into();
    /// assert_eq!(quat, quat2);
    ///
    /// let quat = Quat::from_angles(0.0, 180.0, 0.0);
    /// let quat2: Quat = Quat::Y_180;
    /// assert_eq!(quat, quat2);
    /// ```
    #[inline]
    pub fn from_angles(pitch_x_deg: f32, yaw_y_deg: f32, roll_z_deg: f32) -> Self {
        unsafe { quat_from_angles(pitch_x_deg, yaw_y_deg, roll_z_deg) }
    }

    /// Creates a rotation that describes looking from a point, to another point! This is a great function for camera
    /// style rotation, or other facing behavior when you know where an object is, and where you want it to look at.
    /// This rotation works best when applied to objects that face Vec3.Forward in their resting/model space pose.
    /// <https://stereokit.net/Pages/StereoKit/Quat/LookAt.html>
    /// * `from` - Position of where the 'object' is.
    /// * `at` - The position you want the 'object' to look at.
    /// * `up` - Look From/At positions describe X and Y axis rotation well, but leave Z Axis/Roll
    ///   undefined. Providing an upDirection vector helps to indicate roll around the From/At line. If None : up
    ///   direction will be (0,1,0), to prevent roll.    
    ///
    /// Returns a rotation that describes looking from a point, towards another point.
    /// see also [`quat_lookat`] [`quat_lookat_up`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let from = Vec3::new(1.0, 0.0, 0.0);
    /// let at = Vec3::new(0.0, 0.0, 1.0);
    /// let up = Vec3::new(0.0, 1.0, 0.0);
    /// let quat = Quat::look_at(from, at, Some(up));
    /// assert_eq!(quat, Quat::from_angles(0.0, 135.0, 0.0));
    ///
    /// let quat = Quat::look_at(from, at, None);
    /// assert_eq!(quat, Quat::from_angles(0.0, 135.0, 0.0));
    /// ```
    #[inline]
    pub fn look_at<V: Into<Vec3>>(from: V, at: V, up: Option<Vec3>) -> Self {
        let from = from.into();
        let at = at.into();
        match up {
            Some(up) => unsafe { quat_lookat_up(&from, &at, &up) },
            None => unsafe { quat_lookat(&from, &at) },
        }
    }

    /// Creates a rotation that describes looking towards a direction. This is great for quickly describing facing
    /// behavior! This rotation works best when applied to objects that face Vec3.Forward in their resting/model space
    /// pose.
    /// <https://stereokit.net/Pages/StereoKit/Quat/LookDir.html>
    /// * `direction` - The direction the rotation should be looking. Doesn't need to be normalized.
    ///
    /// Returns a  rotation that describes looking towards a direction.
    /// see also [`quat_lookat`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let direction = Vec3::new(1.0, 0.0, 0.0);
    /// let quat = Quat::look_dir(direction);
    /// assert_eq!(quat, Quat::from_angles(0.0, 270.0, 0.0));
    /// ```
    #[inline]
    pub fn look_dir<V: Into<Vec3>>(direction: V) -> Self {
        let direction = direction.into();
        unsafe { quat_lookat(&Vec3::ZERO, &direction) }
    }

    /// Spherical Linear interpolation. Interpolates between two quaternions! Both Quats should be normalized/unit
    /// quaternions, or you may get unexpected results.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Slerp.html>
    /// * `a` - Start quaternion, should be normalized/unit length.
    /// * `b` - End quaternion, should be normalized/unit length.
    /// * `slerp` - The interpolation amount! This’ll be a if 0, and b if 1. Unclamped.
    ///
    /// Returns a blend between the two quaternions!
    /// see also [`quat_slerp`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let a = Quat::from_angles(0.0, 0.0, 0.0);
    /// let b = Quat::from_angles(0.0, 0.0, 90.0);
    /// let result = Quat::slerp(a, b, 0.25);
    /// assert_eq!(result, Quat::from_angles(0.0, 0.0, 22.5));
    /// ```
    #[inline]
    pub fn slerp(a: Self, b: Self, slerp: f32) -> Self {
        unsafe { quat_slerp(&a, &b, slerp) }
    }

    /// The reverse rotation! If this quat goes from A to B, the inverse will go from B to A.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Inverse.html>
    ///
    /// see also [`quat_inverse`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let q = Quat::from_angles(90.0, 0.0, 0.0);
    /// let q_inv = q.get_inverse();
    /// assert_eq!(q_inv, Quat::from_angles(-90.0, 0.0, 0.0));
    /// ```
    #[inline]
    pub fn get_inverse(&self) -> Self {
        unsafe { quat_inverse(self) }
    }

    /// A normalized quaternion has the same orientation, and a length of 1.
    /// <https://stereokit.net/Pages/StereoKit/Quat/Normalized.html>
    ///
    /// see also [`quat_normalize`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let q = Quat::new(2.0, 0.0, 0.0, 0.0);
    /// let normalized = q.get_normalized();
    /// assert_eq!(normalized, Quat::new(1.0, 0.0, 0.0, 0.0));
    /// assert_eq!(normalized, Quat::from_angles(180.0, 0.0, 0.0));
    /// ```
    #[inline]
    pub fn get_normalized(&self) -> Self {
        unsafe { quat_normalize(self) }
    }

    /// A Vec4 and a Quat are only really different by name and purpose. So, if you need to do Quat math with your
    /// Vec4, or visa versa, who am I to judge?
    /// <https://stereokit.net/Pages/StereoKit/Quat/Vec4.html>
    ///
    /// see also [`Quat::from`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec4};
    ///
    /// let quat = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// let vec4 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(vec4, quat.get_as_vec4());
    /// assert_eq!(vec4, quat.into());
    /// assert_eq!(quat, vec4.into());
    /// ```
    #[inline]
    pub fn get_as_vec4(&self) -> Vec4 {
        Vec4 { x: self.x, y: self.y, z: self.z, w: self.w }
    }

    /// This is the combination of rotations a and b. Note that order matters h
    /// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
    ///
    /// see also `*` operator [`quat_mul`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let a = Quat::from_angles(180.0, 0.0, 0.0);
    /// let b = Quat::from_angles(0.0, 180.0, 0.0);
    /// let c = a * b;
    /// let d = a.mul(b);
    /// assert_eq!(c, Quat::from_angles(0.0, 0.0, -180.0));
    /// assert_eq!(d, Quat::from_angles(0.0, 0.0, -180.0));
    /// ```
    #[inline]
    pub fn mul(&self, rhs: Self) -> Self {
        unsafe { quat_mul(self, &rhs) }
    }

    /// This rotates a point around the origin by the Quat.
    /// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
    ///
    /// see also `*` operator [`quat_mul_vec`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let q = Quat::from_angles(0.0, 90.0, 0.0);
    /// let v = Vec3::new(1.0, 0.0, 0.0);
    /// let result = q.mul_vec3(v);
    /// assert_eq!(result, Vec3::new(0.0, 0.0, -1.0));
    ///
    /// let result_b = q * v;
    /// assert_eq!(result_b, result);
    /// ```
    #[inline]
    pub fn mul_vec3<V: Into<Vec3>>(&self, rhs: V) -> Vec3 {
        let rhs = rhs.into();
        unsafe { quat_mul_vec(self, &rhs) }
    }

    /// get an array
    ///
    /// see also [`Quat::from`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let q = Quat::new(1.0, 2.0, 3.0, 4.0);
    /// assert_eq!(q.to_array(), [1.0, 2.0, 3.0, 4.0]);
    /// ```
    #[inline]
    pub const fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

impl Display for Quat {
    /// Mostly for debug purposes, this is a decent way to log or inspect the vector in debug mode. Looks
    /// like “[x, y, z, w]”
    /// <https://stereokit.net/Pages/StereoKit/Quat/ToString.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let quat = Quat::new(1.0, 2.0, 3.3, 4.0);
    /// assert_eq!(format!("{}", quat), "[x:1, y:2, z:3.3, w:4]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[x:{}, y:{}, z:{}, w:{}]", self.x, self.y, self.z, self.w)
    }
}

/// This is the combination of rotations a and b. Note that order matters h
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`quat_mul`]
impl Mul<Quat> for Quat {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let a = Quat::from_angles(180.0, 0.0, 0.0);
    /// let b = Quat::from_angles(0.0, 180.0, 0.0);
    /// let c = Quat::from_angles(0.0, 0.0, -180.0);
    /// assert_eq!(a * b, c);
    /// ```
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        unsafe { quat_mul(&self, &rhs) }
    }
}

/// This is the combination of rotations a and b. Note that order matters h
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`quat_mul`]
impl MulAssign<Quat> for Quat {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let mut q = Quat::new(1.0, 0.0, 0.0, 0.0);
    /// let r = Quat::new(0.0, 1.0, 0.0, 0.0);
    /// q *= r;
    /// assert_eq!(q, Quat::new(0.0, 0.0, -1.0, 0.0));
    /// ```
    #[inline]
    fn mul_assign(&mut self, rhs: Quat) {
        *self = unsafe { quat_mul(self, &rhs) }
    }
}

/// This rotates a point around the origin by the Quat.
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Multiply.html>
///
/// see also [`quat_mul_vec`]
impl Mul<Vec3> for Quat {
    type Output = Vec3;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Vec3};
    ///
    /// let q = Quat::from_angles(0.0, 90.0, 0.0);
    /// let v = Vec3::new(1.0, 0.0, 0.0);
    /// assert_eq!(q * v, Vec3::new(0.0, 0.0, -1.0));
    /// ```
    fn mul(self, rhs: Vec3) -> Self::Output {
        unsafe { quat_mul_vec(&self, &rhs) }
    }
}

/// Gets a Quat representing the rotation from a to b.
/// <https://stereokit.net/Pages/StereoKit/Quat/op_Subtraction.html>
/// see also QuatT::delta()
///
/// see also [`quat_difference`]
impl Sub<Quat> for Quat {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Quat;
    ///
    /// let from = Quat::from_angles(180.0, 0.0, 0.0);
    /// let to = Quat::from_angles(0.0, 180.0, 0.0);
    /// let delta = Quat::delta(from, to);
    /// assert_eq!(delta, Quat::from_angles(0.0, 0.0, 180.0));
    /// assert_eq!(from - to, delta);
    /// ```
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
/// <https://stereokit.net/Pages/StereoKit/Matrix.html>
///
/// see also [`glam::Mat4`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, model::Model};
///
/// let model = Model::from_file("center.glb", None).unwrap().copy();
/// let transform = Matrix::t_r_s(Vec3::NEG_Y * 0.7, [0.0, 155.0, 10.0], Vec3::ONE * 0.3);
///
/// filename_scr = "screenshots/matrix.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, transform, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/matrix.jpeg" alt="screenshot" width="200">
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

impl From<[f32; 16]> for Matrix {
    fn from(s: [f32; 16]) -> Self {
        Self { m: s }
    }
}

impl From<Pose> for Matrix {
    fn from(pose: Pose) -> Self {
        Self::t_r(pose.position, pose.orientation)
    }
}

impl std::fmt::Debug for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

///  Warning: Equality with a precision of 0.1 millimeter
impl PartialEq for Matrix {
    /// Warning: Equality with a precision of 0.00001
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Matrix;
    ///
    /// let matrix = Matrix::IDENTITY;
    /// assert_eq!(matrix, Matrix::IDENTITY);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.row[0] == other.row[0]
                && self.row[1] == other.row[1]
                && self.row[2] == other.row[2]
                && self.row[3] == other.row[3]
        }
    }
}

unsafe extern "C" {
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
    /// Identity matrix made of [[Vec4::X, Vec4::Y, Vec4::Z, Vec4::W]]
    pub const IDENTITY: Matrix = Matrix { row: [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W] };

    /// Identity matrix rotated 180 degrees around the Y axis made of [[Vec4::NEG_X, Vec4::Y, Vec4::NEG_Z, Vec4::W]]
    /// This is mainly used for test screenshots, but it can be useful for other things too!
    pub const Y_180: Matrix = Matrix { row: [Vec4::NEG_X, Vec4::Y, Vec4::NEG_Z, Vec4::W] };

    /// Null or Zero matrix made of [[Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO]]
    pub const NULL: Matrix = Matrix { row: [Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO] };

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. Orthographic
    /// projection matrices will preserve parallel lines. This is great for 2D scenes or content.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Orthographic.html>
    /// * `width` - in meters, of the area that will  be projected.
    /// * `height` - The height, in meters, of the area that will be projected
    /// * `near_clip` - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * `far_clip` - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.    
    ///
    /// Returns the final orthographic Matrix.
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let translate = Matrix::orthographic(1.0, 1.0, 0.1, 100.0);
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    ///
    /// let projection = translate * point;
    /// assert_eq!(projection, Vec3 { x: 2.0, y: 4.0, z: -0.03103103 });
    /// ```
    /// see also [`matrix_orthographic`]
    #[inline]
    pub fn orthographic(width: f32, height: f32, near_clip: f32, far_clip: f32) -> Self {
        unsafe { matrix_orthographic(width, height, near_clip, far_clip) }
    }

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. Perspective
    /// projection matrices will cause parallel lines to converge at the horizon. This is great for normal looking
    /// content.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Perspective.html>
    /// * `fov_degrees` - This is the vertical field of view of the perspective matrix, units are in degrees.
    /// * `aspect_ratio` - The projection surface's width/height.
    /// * `near_clip` - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * `far_clip` - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.    
    ///
    /// Returns the final perspective matrix.
    /// see also [`matrix_perspective`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let translate = Matrix::perspective(90.0, 1.0, 0.1, 100.0);
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    ///
    /// let projection = translate * point;
    /// assert_eq!(projection,  Vec3 { x: 72.946686, y: 145.89337, z: -3.1031032 });
    /// ```
    #[inline]
    pub fn perspective(fov_degrees: f32, aspect_ratio: f32, near_clip: f32, far_clip: f32) -> Self {
        unsafe { matrix_perspective(fov_degrees.to_radians(), aspect_ratio, near_clip, far_clip) }
    }

    /// This creates a matrix used for projecting 3D geometry onto a 2D surface for rasterization. With the known camera
    /// intrinsics, you can replicate its perspective!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Perspective.html>
    /// * `image_resolution` - The resolution of the image. This should be the image's width and height in pixels.
    /// * `focal_length_px` - The focal length of camera in pixels, with image coordinates +X (pointing right) and +Y
    ///   (pointing up).
    /// * `near_clip` - Anything closer than this distance (in meters) will be discarded. Must not be zero, and if you
    ///   make this too small, you may experience glitching in your depth buffer.
    /// * `far_clip` - Anything further than this distance (in meters) will be discarded. For low resolution depth
    ///   buffers, this should not be too far away, or you'll see bad z-fighting artifacts.
    ///
    /// Returns the final perspective matrix.
    /// Remarks: Think of the optical axis as an imaginary line that passes through the camera lens. In front of the
    /// camera lens, there's an image plane, perpendicular to the optical axis, where the image of the scene being
    /// captured is formed. Its distance is equal to the focal length of the camera from the center of the lens. Here,
    /// we find the ratio between the size of the image plane and distance from the camera in one unit distance and
    /// multiply it by the near clip distance to find a near plane that is parallel.
    ///
    /// see also [`matrix_perspective`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec2, Vec3, Matrix};
    ///
    /// let translate = Matrix::perspective_focal(Vec2::new(1.0, 1.0), 1.0, 0.1, 100.0);
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    ///
    /// let projection = translate * point;
    /// assert_eq!(projection,  Vec3 { x: 2.0, y: 4.0, z: -3.1031032 });
    /// ```
    #[inline]
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

    /// A transformation that describes one position looking at another point. This is particularly useful for
    /// describing camera transforms!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/LookAt.html>
    ///
    /// see also [`matrix_r`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let from = Vec3::new(1.0, 0.0, 0.0);
    /// let at = Vec3::new(0.0, 0.0, 0.0);
    /// let up = Vec3::new(0.0, 1.0, 0.0);
    /// let transform = Matrix::look_at(from, at, Some(up));
    /// assert_eq!(transform, Matrix::from([
    ///     0.0, 0.0, 1.0, 0.0,
    ///     0.0, 1.0, 0.0, 0.0,
    ///     1.0, 0.0, 0.0, 0.0,
    ///     0.0, 0.0,-1.0, 1.0,
    /// ]));
    ///
    /// let transform_b = Matrix::look_at(from, at, None);
    /// assert_eq!(transform, transform_b);
    /// ```
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
    }

    /// Create a rotation matrix from a Quaternion. Consider using [`Matrix::update_r`] when you have to recalculate
    /// Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/R.html>
    /// * `rotation` - The quaternion describing the rotation for this transform.
    ///
    /// returns a Matrix that will rotate by the provided Quaternion orientation.
    /// see also [`matrix_r`] [`Matrix::update_r`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Matrix};
    ///
    /// let rotation = Quat::from_angles(90.0, 0.0, 0.0);
    /// let matrix = Matrix::r(rotation);
    /// assert_eq!(matrix, Matrix::from([
    ///     1.0, 0.0, 0.0, 0.0,
    ///     0.0, 0.0, 1.0, 0.0,
    ///     0.0,-1.0, 0.0, 0.0,
    ///     0.0, 0.0, 0.0, 1.0,
    /// ]));
    ///
    /// let rotation = Quat::from_angles(0.0, 90.0, 0.0);
    /// let matrix = Matrix::r(rotation);
    /// assert_eq!(matrix, Matrix::from([
    ///     0.0, 0.0, -1.0, 0.0,
    ///     0.0, 1.0,  0.0, 0.0,
    ///     1.0, 0.0,  0.0, 0.0,
    ///     0.0, 0.0,  0.0, 1.0,
    /// ]));
    ///
    /// ```
    #[inline]
    pub fn r<Q: Into<Quat>>(rotation: Q) -> Self {
        unsafe { matrix_r(rotation.into()) }
    }

    /// Create a rotation matrix from a Quaternion.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/R.html>
    /// * `rotation` - The quaternion describing the rotation for this transform.
    ///
    /// see also [`matrix_trs_out`] [`Matrix::r`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut delta_rotate = 90.0;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     delta_rotate = (delta_rotate + 10.0  * Time::get_stepf()) % 360.0;
    ///     let rotation = Quat::from_angles(0.0, delta_rotate, 0.0);
    ///
    ///     transform.update_r(&rotation);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::r(rotation));
    /// );
    #[inline]
    pub fn update_r(&mut self, rotation: &Quat) {
        unsafe { matrix_trs_out(self, &Vec3::ZERO, rotation, &Vec3::ONE) }
    }

    /// Creates a scaling Matrix, where scale can be different on each axis (non-uniform).Consider using
    /// [`Matrix::update_s`] when you have to recalculate Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/S.html>
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is the default.
    ///
    /// Returns a non-uniform scaling matrix.
    /// see also [`matrix_s`] [`Matrix::update_s`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let scale = Matrix::s(Vec3::new(2.0, 3.0, 4.0));
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let scaled_point = scale * point;
    /// assert_eq!(scaled_point, Vec3::new(2.0, 3.0, 4.0));
    /// ```
    #[inline]
    pub fn s<V: Into<Vec3>>(scale: V) -> Self {
        unsafe { matrix_s(scale.into()) }
    }

    /// Creates a scaling Matrix, where scale can be different on each axis (non-uniform).
    /// <https://stereokit.net/Pages/StereoKit/Matrix/S.html>
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is the default.
    ///
    /// see also [`matrix_trs_out`] [`Matrix::s`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut delta_scale = 2.0;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     let scale = Vec3::ONE * (delta_scale * Time::get_stepf()).cos();
    ///
    ///     transform.update_s(&scale);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::s(scale));
    /// );
    /// ```
    #[inline]
    pub fn update_s(&mut self, scale: &Vec3) {
        unsafe { matrix_trs_out(self, &Vec3::ZERO, &Quat::IDENTITY, scale) }
    }

    /// Translate. Creates a translation Matrix! Consider using [`Matrix::update_t`] when you have to recalculate
    /// Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/T.html>
    /// * `translation` - Move an object by this amount.
    ///
    /// Returns a Matrix containing a simple translation!
    /// see also [`matrix_t`] [`Matrix::update_t`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let translation = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
    /// let point = Vec3::new(0.0, 0.0, 0.0);
    /// let translated_point = translation * point;
    /// assert_eq!(translated_point, Vec3::new(1.0, 2.0, 3.0));
    /// ```    
    #[inline]
    pub fn t<V: Into<Vec3>>(translation: V) -> Self {
        unsafe { matrix_t(translation.into()) }
    }

    /// Translate. Creates a translation Matrix!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/T.html>
    /// * `translation` - Move an object by this amount.
    ///
    /// see also [`matrix_trs_out`] [`Matrix::t`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut position = Vec3::ZERO;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     position += Vec3::NEG_Z * Time::get_stepf();
    ///
    ///     transform.update_t(&position);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::t(position));
    /// );
    /// ```
    #[inline]
    pub fn update_t(&mut self, translation: &Vec3) {
        unsafe { matrix_trs_out(self, translation, &Quat::IDENTITY, &Vec3::ONE) }
    }

    /// Translate, Rotate. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TR.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    ///
    /// Returns a Matrix that combines translation and rotation information into a single Matrix!
    /// see also [`matrix_trs`]
    #[deprecated(since = "0.40.0", note = "please use Matrix::t_r or Matrix::update_t_r")]
    #[inline]
    pub fn tr(translation: &Vec3, rotation: &Quat) -> Self {
        unsafe { matrix_trs(translation, rotation, &Vec3::ONE) }
    }

    /// Translate, Rotate. Creates a transform Matrix using these components! Consider using
    /// [`Matrix::update_t_r`] when you have to recalculate Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TR.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    ///
    /// Returns a Matrix that combines translation and rotation information into a single Matrix!
    /// see also [`matrix_trs`] [`Matrix::update_t_r`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t_r([1.0, 2.0, 3.0], [0.0, 180.0, 0.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform * point;
    /// assert_eq!(transformed_point, Vec3::new(0.0, 3.0, 2.0));
    /// ```
    #[inline]
    pub fn t_r<V: Into<Vec3>, Q: Into<Quat>>(translation: V, rotation: Q) -> Self {
        unsafe { matrix_trs(&translation.into(), &rotation.into(), &Vec3::ONE) }
    }

    /// Translate, Rotate. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TR.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    ///
    /// see also [`matrix_trs_out`] [`Matrix::t_r`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut position = Vec3::ZERO;
    /// let mut delta_rotate = 90.0;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     position += Vec3::NEG_Z * Time::get_stepf();
    ///     delta_rotate = (delta_rotate + 10.0  * Time::get_stepf()) % 360.0;
    ///     let rotation = Quat::from_angles(0.0, delta_rotate, 0.0);
    ///
    ///     transform.update_t_r(&position, &rotation);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::t_r(position, rotation));
    /// );
    #[inline]
    pub fn update_t_r(&mut self, translation: &Vec3, rotation: &Quat) {
        unsafe { matrix_trs_out(self, translation, rotation, &Vec3::ONE) }
    }

    /// Translate, Scale. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TS.html>
    /// * `translation` - Move an object by this amount.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is the default.
    ///
    /// Returns a Matrix that combines translation and rotation information into a single Matrix!
    /// see also [`matrix_ts`]
    #[deprecated(since = "0.40.0", note = "please use Matrix::t_s or Matrix::update_t_s")]
    #[inline]
    pub fn ts<V: Into<Vec3>>(translation: V, scale: V) -> Self {
        unsafe { matrix_ts(translation.into(), scale.into()) }
    }

    /// Translate, Scale. Creates a transform Matrix using these components! Consider using
    /// [`Matrix::update_t_s`] when you have to recalculate Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TS.html>
    /// * `translation` - Move an object by this amount.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is the default.
    ///
    /// Returns a Matrix that combines translation and rotation information into a single Matrix!
    /// see also [`matrix_ts`] [`Matrix::update_t_s`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t_s([1.0, 2.0, 3.0], [2.0, 2.0, 2.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform * point;
    /// assert_eq!(transformed_point, Vec3::new(3.0, 4.0, 5.0));
    /// ```
    #[inline]
    pub fn t_s<V: Into<Vec3>>(translation: V, scale: V) -> Self {
        unsafe { matrix_ts(translation.into(), scale.into()) }
    }

    /// Translate, Scale. Creates a transform Matrix using these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TS.html>
    /// * `translation` - Move an object by this amount.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is the default.
    ///
    /// Returns a Matrix that combines translation and rotation information into a single Matrix!
    /// see also [`matrix_trs_out`] [`Matrix::t_s`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut position = Vec3::ZERO;
    /// let mut delta_scale = 2.0;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     position += Vec3::NEG_Z * Time::get_stepf();
    ///     let scale = Vec3::ONE * (delta_scale * Time::get_stepf()).cos();
    ///
    ///     transform.update_t_s(&position, &scale);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::t_s(position, scale));
    /// );
    /// ```
    #[inline]
    pub fn update_t_s(&mut self, translation: &Vec3, scale: &Vec3) {
        unsafe { matrix_trs_out(self, translation, &Quat::IDENTITY, scale) }
    }

    /// Translate, Rotate, Scale. Creates a transform Matrix using all these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is a good default.
    ///
    /// Returns a Matrix that combines translation, rotation and scale information into a single Matrix!
    /// see also [`matrix_trs`]
    #[deprecated(since = "0.40.0", note = "please use Matrix::t_r_s or Matrix::update_t_r_s")]
    #[inline]
    pub fn trs(translation: &Vec3, rotation: &Quat, scale: &Vec3) -> Self {
        unsafe { matrix_trs(translation, rotation, scale) }
    }

    /// Translate, Rotate, Scale. Creates a transform Matrix using all these components! Consider using
    /// [`Matrix::update_t_r_s`] when you have to recalculate Matrix at each steps in main loop.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is a good default.
    ///
    /// Returns a Matrix that combines translation, rotation and scale information into a single Matrix!
    /// see also [`matrix_trs`] [`Matrix::update_t_r_s`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t_r_s([1.0, 2.0, 3.0], [0.0, 180.0, 0.0], [2.0, 2.0, 2.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform * point;
    /// assert_eq!(transformed_point, Vec3::new(-1.0, 4.0, 1.0));
    /// ```
    #[inline]
    pub fn t_r_s<V: Into<Vec3>, Q: Into<Quat>>(translation: V, rotation: Q, scale: V) -> Self {
        unsafe { matrix_trs(&translation.into(), &rotation.into(), &scale.into()) }
    }

    /// Translate, Rotate, Scale. Creates a transform Matrix using all these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is a good default.
    ///
    /// Returns a Matrix that combines translation, rotation and scale information into a single Matrix!
    /// see also [`matrix_trs_out`] [`Matrix::t_r_s`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Quat, Matrix}, mesh::Mesh, material::Material,
    ///                      util::Time};
    ///
    /// let mesh = Mesh::cube();
    /// let material = Material::pbr();
    ///
    /// let mut transform = Matrix::IDENTITY;
    /// let mut position = Vec3::ZERO;
    /// let mut delta_scale = 2.0;
    /// let mut delta_rotate = 90.0;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     position += Vec3::NEG_Z * Time::get_stepf();
    ///     delta_rotate = (delta_rotate + 10.0  * Time::get_stepf()) % 360.0;
    ///     let rotation = Quat::from_angles(0.0, delta_rotate, 0.0);
    ///     let scale = Vec3::ONE * (delta_scale * Time::get_stepf()).cos();
    ///
    ///     transform.update_t_r_s(&position, &rotation, &scale);
    ///
    ///     mesh.draw(token, &material, transform, None, None);
    ///     assert_eq!(transform, Matrix::t_r_s(position, rotation, scale));
    /// );
    /// ```
    #[inline]
    pub fn update_t_r_s(&mut self, translation: &Vec3, rotation: &Quat, scale: &Vec3) {
        unsafe { matrix_trs_out(self, translation, rotation, scale) }
    }

    /// Translate, Rotate, Scale. Update a transform Matrix using all these components!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TRS.html>
    /// * `translation` - Move an object by this amount.
    /// * `rotation` - The quaternion describing the rotation for this transform.
    /// * `scale` - How much larger or smaller this transform makes things. Vec3::ONE is a good default.
    ///
    /// see also [`matrix_trs_out`]
    #[deprecated(since = "0.40.0", note = "please use Matrix::update_t_r, Matrix::update_t_s or Matrix::update_t_r_s")]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn trs_to_pointer(translation: &Vec3, rotation: &Quat, scale: &Vec3, out_result: *mut Matrix) {
        unsafe { matrix_trs_out(out_result, translation, rotation, scale) }
    }

    /// Inverts this Matrix! If the matrix takes a point from a -> b, then its inverse takes the point from b -> a.
    /// The Matrix is modified so use get_inverse* for performance gains
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Invert.html>
    ///
    /// see also [`Matrix::get_inverse`] [`matrix_invert`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let mut transform = Matrix::t([1.0, 2.0, 3.0]);
    /// transform.invert();
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    /// let transformed_point = transform * point;
    /// assert_eq!(transformed_point, Vec3::new(0.0, 0.0, 0.0));
    /// ```
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
    /// see also [`matrix_transpose`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let mut transform = Matrix::t([1.0, 2.0, 3.0]);
    /// assert_eq!(transform, Matrix::from([
    ///     1.0, 0.0, 0.0, 0.0,
    ///     0.0, 1.0, 0.0, 0.0,
    ///     0.0, 0.0, 1.0, 0.0,
    ///     1.0, 2.0, 3.0, 1.0,
    /// ]));
    ///
    /// transform.transpose();
    /// assert_eq!(transform, Matrix::from([
    ///     1.0, 0.0, 0.0, 1.0,
    ///     0.0, 1.0, 0.0, 2.0,
    ///     0.0, 0.0, 1.0, 3.0,
    ///     0.0, 0.0, 0.0, 1.0,
    /// ]));
    /// ```
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
    /// see also [`matrix_decompose`] [`Matrix::decompose_to_ptr`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Matrix, Vec3, Quat};
    ///
    /// let position = Vec3::new(1.0, 2.0, 3.0);
    /// let orientation = Quat::from_angles(0.0, 90.0, 0.0);
    /// let scale = Vec3::new(2.0, 2.0, 2.0);
    /// let matrix = Matrix::t_r_s(position, orientation, scale);
    ///
    /// let (pos, sca, ori) = matrix.decompose().unwrap();
    /// assert_eq!(pos, position);
    /// assert_eq!(sca, scale);
    /// assert_eq!(ori, orientation);
    /// ```
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
    /// * `out_position` - The translation component of the matrix.
    /// * `out_orientation` - The rotation component of the matrix, some lossiness may be encountered when
    ///   composing/decomposing.
    /// * `out_scale` - The scale component of the matrix.
    ///
    /// see also [`matrix_decompose`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Matrix, Vec3, Quat};
    ///
    /// let position = Vec3::new(1.0, 2.0, 3.0);
    /// let orientation = Quat::from_angles(0.0, 90.0, 0.0);
    /// let scale = Vec3::new(2.0, 2.0, 2.0);
    /// let matrix = Matrix::t_r_s(position, orientation, scale);
    ///
    /// let mut pos = Vec3::ZERO;
    /// let mut sca = Vec3::ZERO;
    /// let mut ori = Quat::ZERO;
    ///
    /// matrix.decompose_to_ptr(&mut pos, &mut ori, &mut sca);
    /// assert_eq!(pos, position);
    /// assert_eq!(sca, scale);
    /// assert_eq!(ori, orientation);
    /// ```
    #[inline]
    pub fn decompose_to_ptr(&self, out_position: &mut Vec3, out_orientation: &mut Quat, out_scale: &mut Vec3) -> bool {
        unsafe { matrix_decompose(self, out_position, out_scale, out_orientation) != 0 }
    }

    /// Transforms a point through the Matrix! This is basically just multiplying a vector (x,y,z,1) with the Matrix.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html>
    /// * `point` - The point to transform.
    ///
    /// Returns the point transformed by the Matrix.
    /// see also the '*' operator [`matrix_transform_pt`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform.transform_point(point);
    /// assert_eq!(transformed_point, Vec3::new(2.0, 3.0, 4.0));
    ///
    /// let transformed_point_b = transform * point;
    /// assert_eq!(transformed_point_b, transformed_point);
    /// ```
    #[inline]
    pub fn transform_point<V: Into<Vec3>>(&self, point: V) -> Vec3 {
        unsafe { matrix_transform_pt(*self, point.into()) }
    }

    /// Shorthand to transform a ray though the Matrix! This properly transforms the position with the point transform
    /// method, and the direction with the direction transform method. Does not normalize, nor does it preserve a
    /// normalized direction if the Matrix contains scale data.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html>
    /// * `ray` - A ray you wish to transform from one space to another.
    ///
    /// Returns the Ray transformed by the Matrix.
    /// see also the `*` operator [`matrix_transform_ray`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Ray};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let ray = Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    /// let transformed_ray = transform.transform_ray(ray);
    /// assert_eq!(transformed_ray, Ray::new([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]));
    ///
    /// let transformed_ray_b = transform * ray;
    /// assert_eq!(transformed_ray_b, transformed_ray);
    /// ```
    #[inline]
    pub fn transform_ray<R: Into<Ray>>(&self, ray: R) -> Ray {
        unsafe { matrix_transform_ray(*self, ray.into()) }
    }

    /// Shorthand for transforming a Pose! This will transform the position of the Pose with the matrix, extract a
    /// rotation Quat from the matrix and apply that to the Pose’s orientation. Note that extracting a rotation Quat
    /// is an expensive operation, so if you’re doing it more than once, you should cache the rotation Quat and do this
    /// transform manually.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html>
    /// * `pose` - The original pose.
    ///
    /// Returns the transformed pose.
    /// see also the `*` operator [`matrix_transform_pose`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Pose, Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let pose = Pose::new([0.0, 0.0, 0.0], None);
    /// let transformed_pose = transform.transform_pose(pose);
    /// assert_eq!(transformed_pose, Pose::new([1.0, 2.0, 3.0], None));
    ///
    /// let transformed_pose_b = transform * pose;
    /// assert_eq!(transformed_pose_b, transformed_pose);
    /// ```
    #[inline]
    pub fn transform_pose<P: Into<Pose>>(&self, pose: P) -> Pose {
        unsafe { matrix_transform_pose(*self, pose.into()) }
    }

    /// Shorthand for transforming a rotation! This will extract a rotation Quat from the matrix and apply that to the
    /// QuatT's orientation. Note that extracting a rotation Quat is an expensive operation, so if you’re doing it more
    /// than once, you should cache the rotation Quat and do this transform manually.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transform.html>
    /// * `rotation` - The original rotation
    ///
    /// Return the transformed quat.
    /// see also the `*` operator [`matrix_transform_quat`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Matrix};
    ///
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    /// let rotation = Quat::from_angles(0.0, 90.0, 0.0);
    /// let transformed_rotation = transform.transform_quat(rotation);
    /// assert_eq!(transformed_rotation, Quat::from_angles(0.0, 180.0, 0.0));
    ///
    /// let transformed_rotation_b = transform * rotation;
    /// assert_eq!(transformed_rotation_b, transformed_rotation);
    /// ```
    #[inline]
    pub fn transform_quat<Q: Into<Quat>>(&self, rotation: Q) -> Quat {
        unsafe { matrix_transform_quat(*self, rotation.into()) }
    }

    /// Transforms a point through the Matrix, but excluding translation! This is great for transforming vectors that
    /// are -directions- rather than points in space. Use this to transform normals and directions. The same as
    /// multiplying (x,y,z,0) with the Matrix. Do not correspond to `*` operator !
    /// <https://stereokit.net/Pages/StereoKit/Matrix/TransformNormal.html>
    /// * `normal` - A direction vector to be transformed.
    ///
    /// Returns the direction transformed by the Matrix.
    /// see also [`matrix_transform_dir`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    /// let normal = Vec3::new(1.0, 0.0, 0.0);
    /// let transformed_normal = transform.transform_normal(normal);
    /// assert_eq!(transformed_normal, Vec3::new(0.0, 0.0, -1.0));
    /// ```
    #[inline]
    pub fn transform_normal<V: Into<Vec3>>(&self, normal: V) -> Vec3 {
        unsafe { matrix_transform_dir(*self, normal.into()) }
    }

    /// Creates an inverse matrix! If the matrix takes a point from a -> b, then its inverse takes the point
    /// from b -> a.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Inverse.html>
    ///
    /// see also [`matrix_inverse`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let inverse = transform.get_inverse();
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    /// let transformed_point = inverse * point;
    /// assert_eq!(transformed_point, Vec3::new(0.0, 0.0, 0.0));
    /// ```
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
    /// * `out` - The output matrix.
    ///
    /// see also [`matrix_inverse`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let mut inverse = Matrix::NULL;
    ///
    /// transform.get_inverse_to_ptr(&mut inverse);
    /// let point = Vec3::new(1.0, 2.0, 3.0);
    /// let transformed_point = inverse * point;
    /// assert_eq!(transformed_point, Vec3::new(0.0, 0.0, 0.0));
    /// ```
    #[inline]
    pub fn get_inverse_to_ptr(&self, out: &mut Matrix) {
        unsafe {
            matrix_inverse(self, out);
        }
    }

    /// Extracts translation and rotation information from the transform matrix, and makes a Pose from it! Not exactly
    /// fast. This is backed by Decompose, so if you need any additional info, it’s better to just call Decompose instead.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Pose.html>
    ///
    /// see also [`matrix_extract_pose`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Quat, Matrix, Pose};
    ///
    /// let matrix = Matrix::t_r(Vec3::new(1.0, 2.0, 3.0), [0.0, 0.0, 0.0]);
    ///
    /// let pose = matrix.get_pose();
    /// assert_eq!(pose.position, Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(pose.orientation, Quat::IDENTITY)
    /// ```
    #[inline]
    pub fn get_pose(&self) -> Pose {
        unsafe { matrix_extract_pose(self) }
    }

    /// A slow function that returns the rotation quaternion embedded in this transform matrix. This is backed by
    /// Decompose, so if you need any additional info, it’s better to just call Decompose instead.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Rotation.html>
    ///
    /// see also [`matrix_extract_rotation`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Quat, Matrix};
    ///
    /// let matrix = Matrix::t_r(Vec3::new(1.0, 2.0, 3.0), [0.0, 90.0, 0.0]);
    ///
    /// let rotation = matrix.get_rotation();
    /// assert_eq!(rotation, Quat::from_angles(0.0, 90.0, 0.0))
    /// ```
    #[inline]
    pub fn get_rotation(&self) -> Quat {
        unsafe { matrix_extract_rotation(self) }
    }

    /// Returns the scale embedded in this transform matrix. Not exactly cheap, requires 3 sqrt calls, but is cheaper
    /// than calling Decompose.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Scale.html>
    ///
    /// see also [`matrix_extract_scale`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let matrix = Matrix::t_s(Vec3::new(1.0, 2.0, 3.0), Vec3::new(2.0, 2.0, 2.0));
    ///
    /// let scale = matrix.get_scale();
    /// assert_eq!(scale, Vec3::new(2.0, 2.0, 2.0))
    /// ```
    #[inline]
    pub fn get_scale(&self) -> Vec3 {
        unsafe { matrix_extract_scale(self) }
    }

    /// A fast getter that will return or set the translation component embedded in this transform matrix.
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Translation.html>
    ///
    /// see also [`matrix_extract_translation`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    /// let normal = Vec3::new(1.0, 0.0, 0.0);
    ///
    /// let transformed_normal = transform.transform_normal(normal);
    /// assert_eq!(transformed_normal, Vec3::new(0.0, 0.0, -1.0));
    /// ```
    #[inline]
    pub fn get_translation(&self) -> Vec3 {
        unsafe { matrix_extract_translation(self) }
    }

    /// Creates a matrix that has been transposed! Transposing is like rotating the matrix 90 clockwise, or turning
    /// the rows into columns. This can be useful for inverting orthogonal matrices, or converting matrices for use
    /// in a math library that uses different conventions!
    /// <https://stereokit.net/Pages/StereoKit/Matrix/Transposed.html>
    ///
    /// see also [`matrix_transpose`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// assert_eq!(transform, Matrix::from([
    ///     1.0, 0.0, 0.0, 0.0,
    ///     0.0, 1.0, 0.0, 0.0,
    ///     0.0, 0.0, 1.0, 0.0,
    ///     1.0, 2.0, 3.0, 1.0,
    /// ]));
    ///
    /// let transposed = transform.get_transposed();
    /// assert_eq!(transposed, Matrix::from([
    ///     1.0, 0.0, 0.0, 1.0,
    ///     0.0, 1.0, 0.0, 2.0,
    ///     0.0, 0.0, 1.0, 3.0,
    ///     0.0, 0.0, 0.0, 1.0,
    /// ]));
    /// ```
    #[inline]
    pub fn get_transposed(&self) -> Self {
        unsafe { matrix_transpose(*self) }
    }
}

impl Display for Matrix {
    /// Mostly for debug purposes, this is a decent way to log or inspect the matrix in debug mode. Looks
    /// like “[, , , ]”
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::Matrix;
    ///
    /// let matrix = Matrix::from([
    ///     1.0, 2.0, 3.3, 4.0,
    ///     5.0, 6.0, 7.0, 8.0,
    ///     9.0, 10.0, 11.0, 12.0,
    ///     13.0, 14.0, 15.0, 16.0,
    /// ]);
    /// assert_eq!(format!("{}", matrix),
    /// "[\r\n [x:1, y:2, z:3.3, w:4],\r\n [x:5, y:6, z:7, w:8],\r\n [x:9, y:10, z:11, w:12],\r\n [x:13, y:14, z:15, w:16]]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "[\r\n {},\r\n {},\r\n {},\r\n {}]", self.row[0], self.row[1], self.row[2], self.row[3]) }
    }
}
/// Multiplies the vector by the Matrix! Since only a 1x4 vector can be multiplied against a 4x4 matrix, this uses ‘1’
/// for the 4th element, so the result will also include translation! To exclude translation,
/// use Matrix.transform_normal.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pt`]
impl Mul<Vec3> for Matrix {
    type Output = Vec3;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    ///
    /// let transformed_point = transform * point;
    /// assert_eq!(transformed_point, Vec3::new(2.0, 3.0, 4.0));
    /// ```
    fn mul(self, rhs: Vec3) -> Self::Output {
        unsafe { matrix_transform_pt(self, rhs) }
    }
}

/// Multiplies the vector by the Matrix! Since only a 1x4 vector can be multiplied against a 4x4 matrix, this uses ‘1’
/// for the 4th element, so the result will also include translation! To exclude translation,
/// use Matrix.transform_normal.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pt`]
impl MulAssign<Matrix> for Vec3 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let mut point = Vec3::new(1.0, 1.0, 1.0);
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    ///
    /// point *= transform;
    /// assert_eq!(point, Vec3::new(2.0, 3.0, 4.0));
    /// ```
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
/// see also [`matrix_transform_pt`]
impl Mul<Matrix> for Vec3 {
    type Output = Vec3;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    ///
    /// let transformed_point = point * transform;
    /// assert_eq!(transformed_point, Vec3::new(2.0, 3.0, 4.0));
    /// ```
    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pt(rhs, self) }
    }
}

/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pt4`]
impl Mul<Vec4> for Matrix {
    type Output = Vec4;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec4, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let v4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
    ///
    /// let transformed_v4 = transform * v4;
    /// assert_eq!(transformed_v4, Vec4::new(2.0, 3.0, 4.0, 1.0));
    /// ```
    fn mul(self, rhs: Vec4) -> Self::Output {
        unsafe { matrix_transform_pt4(self, rhs) }
    }
}

/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pt4`]
impl MulAssign<Matrix> for Vec4 {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec4, Matrix};
    ///
    /// let mut v4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    ///
    /// v4 *= transform;
    /// assert_eq!(v4, Vec4::new(2.0, 3.0, 4.0, 1.0));
    /// ```
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
/// see also [`matrix_transform_pt4`]
impl Mul<Matrix> for Vec4 {
    type Output = Vec4;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec4, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let v4 = Vec4::new(1.0, 1.0, 1.0, 1.0);
    ///
    /// let transformed_v4 = v4 * transform;
    /// assert_eq!(transformed_v4, Vec4::new(2.0, 3.0, 4.0, 1.0));
    /// ```
    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pt4(rhs, self) }
    }
}

/// Transforms a Ray by the Matrix! The position and direction are both multiplied by the matrix, accounting properly for
/// which should include translation, and which should not.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_ray`]
impl Mul<Ray> for Matrix {
    type Output = Ray;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Ray};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let ray = Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    ///
    /// let transformed_ray = transform * ray;
    /// assert_eq!(transformed_ray, Ray::new([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]));
    /// ```
    fn mul(self, rhs: Ray) -> Self::Output {
        unsafe { matrix_transform_ray(self, rhs) }
    }
}

/// Transforms a Ray by the Matrix! The position and direction are both multiplied by the matrix, accounting properly for
/// which should include translation, and which should not.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_ray`]
impl MulAssign<Matrix> for Ray {
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Ray};
    ///
    /// let mut ray = Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    ///
    /// ray *= transform;
    /// assert_eq!(ray, Ray::new([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]));
    /// ```
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
/// see also [`matrix_transform_ray`]
impl Mul<Matrix> for Ray {
    type Output = Ray;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Ray};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let ray = Ray::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    ///
    /// let transformed_ray = ray * transform;
    /// assert_eq!(transformed_ray, Ray::new([1.0, 2.0, 3.0], [1.0, 0.0, 0.0]));
    /// ```
    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_ray(rhs, self) }
    }
}

/// Transform an orientation by the Matrix.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_quat`]
impl Mul<Quat> for Matrix {
    type Output = Quat;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Matrix};
    ///
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    /// let rotation = Quat::from_angles(0.0, 0.0, 0.0);
    ///
    /// let transformed_rotation = transform * rotation;
    /// assert_eq!(transformed_rotation, Quat::from_angles(0.0, 90.0, 0.0));
    /// ```
    fn mul(self, rhs: Quat) -> Self::Output {
        unsafe { matrix_transform_quat(self, rhs) }
    }
}

/// Transform an orientation by the Matrix.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_quat`]
impl MulAssign<Matrix> for Quat {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Matrix};
    ///
    /// let mut rotation = Quat::from_angles(0.0, 0.0, 0.0);
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    ///
    /// rotation *= transform;
    /// assert_eq!(rotation, Quat::from_angles(0.0, 90.0, 0.0));
    /// ```
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
/// see also [`matrix_transform_quat`]
impl Mul<Matrix> for Quat {
    type Output = Quat;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Quat, Matrix};
    ///
    /// let transform = Matrix::r([0.0, 90.0, 0.0]);
    /// let rotation = Quat::from_angles(0.0, 0.0, 0.0);
    ///
    /// let transformed_rotation = rotation * transform;
    /// assert_eq!(transformed_rotation, Quat::from_angles(0.0, 90.0, 0.0));
    /// ```
    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_quat(rhs, self) }
    }
}

/// Transforms a Pose by the Matrix! The position and orientation are both transformed by the matrix, accounting
/// properly for the Pose’s quaternion.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pose`]
impl Mul<Pose> for Matrix {
    type Output = Pose;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Pose, Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let pose = Pose::new([0.0, 0.0, 0.0], None);
    ///
    /// let transformed_pose = transform * pose;
    /// assert_eq!(transformed_pose, Pose::new([1.0, 2.0, 3.0], None));
    /// ```
    fn mul(self, rhs: Pose) -> Self::Output {
        unsafe { matrix_transform_pose(self, rhs) }
    }
}

/// Transforms a Pose by the Matrix! The position and orientation are both transformed by the matrix, accounting
/// properly for the Pose’s quaternion.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_transform_pose`]
impl MulAssign<Matrix> for Pose {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Pose, Vec3, Matrix};
    ///
    /// let mut pose = Pose::new([0.0, 0.0, 0.0], None);
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    ///
    /// pose *= transform;
    /// assert_eq!(pose, Pose::new([1.0, 2.0, 3.0], None));
    /// ```
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
/// see also [`matrix_transform_pose`]
impl Mul<Matrix> for Pose {
    type Output = Pose;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Pose, Vec3, Matrix};
    ///
    /// let transform = Matrix::t([1.0, 2.0, 3.0]);
    /// let pose = Pose::new([0.0, 0.0, 0.0], None);
    ///
    /// let transformed_pose = pose * transform;
    /// assert_eq!(transformed_pose, Pose::new([1.0, 2.0, 3.0], None));
    /// ```
    fn mul(self, rhs: Matrix) -> Self::Output {
        unsafe { matrix_transform_pose(rhs, self) }
    }
}

/// Multiplies two matrices together! This is a great way to combine transform operations. Note that StereoKit’s
/// matrices are row-major, and multiplication order is important! To translate, then scale, multiply in order of
/// ‘translate * scale’.
/// <https://stereokit.net/Pages/StereoKit/Matrix/op_Multiply.html>
///
/// see also [`matrix_mul`]
impl Mul<Matrix> for Matrix {
    type Output = Self;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let transform_a = Matrix::t([1.0, 2.0, 3.0]);
    /// let transform_b = Matrix::s([2.0, 2.0, 2.0]);
    ///
    /// let transform_c = transform_a * transform_b;
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform_c * point;
    /// assert_eq!(transformed_point, Vec3::new(4.0, 6.0, 8.0));
    /// ```
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
/// see also [`matrix_mul`]
impl MulAssign<Matrix> for Matrix {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix};
    ///
    /// let mut transform_a = Matrix::t([1.0, 2.0, 3.0]);
    /// let transform_b = Matrix::s([2.0, 2.0, 2.0]);
    ///
    /// transform_a *= transform_b;
    /// let point = Vec3::new(1.0, 1.0, 1.0);
    /// let transformed_point = transform_a * point;
    /// assert_eq!(transformed_point, Vec3::new(4.0, 6.0, 8.0));
    /// ```
    fn mul_assign(&mut self, rhs: Matrix) {
        unsafe { matrix_mul(self, &rhs, self) };
    }
}

/// Bounds is an axis aligned bounding box type that can be used for storing the sizes of objects, calculating
/// containment, intersections, and more!
///
/// While the constructor uses a center+dimensions for creating a bounds, don’t forget the static From* methods that
/// allow you to define a Bounds from different types of data!
/// <https://stereokit.net/Pages/StereoKit/Bounds.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Pose}, model::Model, ui::Ui,
///     mesh::Mesh, material::Material, util::named_colors};
///
/// let model = Model::from_file("center.glb", None).unwrap().copy();
/// let cube = Mesh::cube();
/// let mut material_cube = Material::ui_box();
/// material_cube.color_tint(named_colors::GOLD)
///              .border_size(0.05);
///
/// let scale = 0.4;
/// let bounds = model.get_bounds() * scale;
/// let transform = Matrix::s(Vec3::ONE * scale);
/// let transform_cube = Matrix::t_s( bounds.center, bounds.dimensions);
/// let mut handle_pose =
///     Pose::new([0.0,-0.95,-0.65], Some([0.0, 140.0, 0.0].into()));
///
/// filename_scr = "screenshots/bounds.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     Ui::handle_begin( "Model Handle", &mut handle_pose,
///                       bounds, false, None, None);
///     model.draw(token, transform, None, None);
///     cube.draw(token, &material_cube, transform_cube, None, None);
///     Ui::handle_end();
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/bounds.jpeg" alt="screenshot" width="200">
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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

unsafe extern "C" {
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
    /// * center - The exact center of the box.
    /// * dimensions - The total size of the box, from one end to the other. This is the width, height, and depth of
    ///   the Bounds.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::new([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    /// assert_eq!(bounds.center, Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(bounds.dimensions, Vec3::new(4.0, 5.0, 6.0));
    /// ```
    #[inline]
    pub fn new<V: Into<Vec3>>(center: V, dimensions: V) -> Bounds {
        Bounds { center: center.into(), dimensions: dimensions.into() }
    }

    /// Creates a bounding box object centered around zero!
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Bounds.html>
    /// * dimensions - The total size of the box, from one end to the other. This is the width, height, and depth of
    ///   the Bounds.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::bounds_centered( [4.0, 5.0, 6.0]);
    /// assert_eq!(bounds.center, Vec3::ZERO);
    /// assert_eq!(bounds.dimensions, Vec3::new(4.0, 5.0, 6.0));
    /// ```
    #[inline]
    pub fn bounds_centered(dimensions: impl Into<Vec3>) -> Bounds {
        Bounds { center: Vec3::ZERO, dimensions: dimensions.into() }
    }

    /// Create a bounding box from a corner, plus box dimensions.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/FromCorner.html>
    /// * bottom_left_back - The -X,-Y,-Z corner of the box.
    /// * dimensions - The dimensions of the bounding box.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corner( Vec3::ZERO, [1.0, 2.0, 3.0].into());
    /// assert_eq!(bounds.center, [0.5, 1.0, 1.5].into());
    /// assert_eq!(bounds.dimensions, Vec3::new(1.0, 2.0, 3.0));
    /// ```
    #[inline]
    pub fn from_corner<V: Into<Vec3>>(bottom_left_back: V, dimensions: V) -> Bounds {
        let dim = dimensions.into();
        Bounds { center: bottom_left_back.into() + dim / 2.0, dimensions: dim }
    }

    /// Create a bounding box between two corner points.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/FromCorners.html>
    /// * bottom_left_back - The -X,-Y,-Z corner of the box.
    /// * top_right_front - The +X,+Y,+Z corner of the box.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// assert_eq!(bounds.center, Vec3::ONE / 2.0);
    /// assert_eq!(bounds.dimensions, Vec3::ONE);
    /// ```
    #[inline]
    pub fn from_corners<V: Into<Vec3>>(bottom_left_back: V, top_right_front: V) -> Bounds {
        let blb = bottom_left_back.into();
        let trf = top_right_front.into();
        Bounds { center: blb / 2.0 + trf / 2.0, dimensions: (trf - blb).abs() }
    }

    /// Grow the Bounds to encapsulate the provided point. Returns the result, and does NOT modify the current bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Grown.html>
    /// * pt - The point to encapsulate! This should be in the same space as the bounds.
    ///
    /// see also [`bounds_grow_to_fit_pt]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// bounds.grown_point(Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(bounds.center, Vec3{ x: 0.5, y: 1.0, z: 1.5 });
    /// assert_eq!(bounds.dimensions, Vec3 { x: 1.0, y: 2.0, z: 3.0 });
    /// ```
    #[inline]
    pub fn grown_point(&mut self, pt: impl Into<Vec3>) -> &mut Self {
        let b = unsafe { bounds_grow_to_fit_pt(*self, pt.into()) };
        self.center = b.center;
        self.dimensions = b.dimensions;
        self
    }

    /// Grow the Bounds to encapsulate the provided box after it has been transformed by the provided matrix transform.
    /// This will transform each corner of the box, and expand the bounds to encapsulate each point!
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Grown.html>
    /// * box_ - the box to encapsulate! The corners of this box are transformed, and then used to grow the bounds.
    /// * opt_box_transform - to center use Matrix::IDENTITY
    ///
    /// see also [`bounds_grow_to_fit_box]
    /// see also [`bounds_grow_to_fit_pt]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// let bounds_plus = Bounds::from_corners( Vec3::ONE, Vec3::ONE * 2.0);
    /// bounds.grown_box(bounds_plus, Matrix::IDENTITY);
    /// assert_eq!(bounds.center, Vec3::ONE);
    /// assert_eq!(bounds.dimensions, Vec3::ONE * 2.0);
    /// ```
    #[inline]
    pub fn grown_box<M: Into<Matrix>>(&mut self, box_: impl AsRef<Bounds>, opt_box_transform: M) -> &mut Self {
        let b = unsafe { bounds_grow_to_fit_box(*self, *(box_.as_ref()), &opt_box_transform.into()) };
        self.center = b.center;
        self.dimensions = b.dimensions;
        self
    }

    /// Scale this bounds. It will scale the center as well as the dimensions! Modifies this bounds object.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scale.html>
    /// * scale - The scale to apply.
    ///
    /// see also [Bounds::scale_vec] [Bounds::scaled] [Bounds::scaled_vec] and '/' '*' operator
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// bounds.scale(2.0);
    /// let bounds_plus = Bounds::from_corners( Vec3::ZERO, Vec3::ONE) * 2.0;
    ///
    /// assert_eq!(bounds.center, bounds_plus.center);
    /// assert_eq!(bounds.dimensions, bounds_plus.dimensions);
    /// ```
    #[inline]
    pub fn scale(&mut self, scale: f32) -> &mut Self {
        self.dimensions *= scale;
        self.center *= scale;
        self
    }

    /// Scale this bounds. It will scale the center as well as the dimensions! Modifies this bounds object.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scale.html>
    /// * scale - The scale to apply.
    ///
    /// see also [Bounds::scale] [Bounds::scaled_vec] [Bounds::scaled] and '/' '*' operator
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// bounds.scale_vec([1.0, 2.0, 3.0]);
    /// let bounds_plus = Bounds::from_corners( Vec3::ZERO, Vec3::ONE) * Vec3::new(1.0, 2.0, 3.0);
    ///
    /// assert_eq!(bounds.center, bounds_plus.center);
    /// assert_eq!(bounds.dimensions, bounds_plus.dimensions);
    /// ```
    #[inline]
    pub fn scale_vec(&mut self, scale: impl Into<Vec3>) -> &mut Self {
        let scale = scale.into();
        self.dimensions *= scale;
        self.center *= scale;
        self
    }

    /// Does the Bounds contain the given point? This includes points that are on the surface of the Bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    /// * pt - A point in the same coordinate space as the Bounds.
    ///
    /// see also [`bounds_point_contains`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix};
    ///
    /// // A cube of 1 meter per side at this position, aligned with x y and z axis.
    /// let box_transform = Matrix::t([10.0, 1.0, 2.0]);
    /// let box_bounds = Bounds::bounds_centered([1.0,1.0,1.0]);
    ///
    /// let inv = box_transform.get_inverse();
    /// let point1 = inv.transform_point([10.2, 1.3, 2.4]);
    /// let point2 = inv.transform_point([10.9, 1.8, 2.1]);
    /// let point3 = inv.transform_point([10.5, 2.01, 1.7]);
    /// let point4 = inv.transform_point([9.5, 0.7, 1.5]);
    ///
    /// assert!(box_bounds.contains_point(point1),  "point1 should be inside");
    /// assert!(!box_bounds.contains_point(point2), "point2 should be outside");
    /// assert!(!box_bounds.contains_point(point3), "point3 should be outside");
    /// assert!(box_bounds.contains_point(point4),  "point4 should be inside");
    ///
    /// assert!(box_bounds.contains_point([0.5, 0.5, 0.5]),
    ///         "inverse point should be inside");
    /// ```
    #[inline]
    pub fn contains_point(&self, pt: impl Into<Vec3>) -> bool {
        unsafe { bounds_point_contains(*self, pt.into()) != 0 }
    }

    /// Does the Bounds contain or intersects with the given line?
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    /// * line_pt1 - The first point on the line.
    /// * line_pt2 - The second point on the line.
    ///
    /// see also [`bounds_line_contains`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix};
    ///
    /// // A cube of 1 meter per side at this position, aligned with x y and z axis.
    /// let box_transform = Matrix::t([10.0, 1.0, 2.0]);
    /// let box_bounds = Bounds::bounds_centered([1.0,1.0,1.0]);
    ///
    /// let inv = box_transform.get_inverse();
    /// let point1 = inv.transform_point([10.2, 1.3, 2.4]);
    /// let point2 = inv.transform_point([10.9, 1.8, 2.1]);
    /// let point3 = inv.transform_point([10.5, 2.01, 1.7]);
    /// let point4 = inv.transform_point([9.5, 0.7, 1.5]);
    ///
    /// assert!(box_bounds.contains_line(point1, point2),  "point1-point2 should be inside");
    /// assert!(!box_bounds.contains_line(point2, point3), "point2-point3 should be outside");
    /// assert!(box_bounds.contains_line(point3, point4),  "point3-point4 should be inside");
    /// assert!(box_bounds.contains_line(point4, point1),  "point4-point1 should be inside");
    ///
    /// assert!(box_bounds.contains_line([0.1, 0.1, 0.1], [0.9, 0.9, 0.9]),
    ///         "inverse line should be inside");
    /// ```
    #[inline]
    pub fn contains_line<V3: Into<Vec3>>(&self, line_pt1: V3, line_pt2: V3) -> bool {
        unsafe { bounds_line_contains(*self, line_pt1.into(), line_pt2.into()) != 0 }
    }

    /// Does the bounds contain or intersect with the given capsule?
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Contains.html>
    /// * pt1 - The first point of the capsule.
    /// * pt2 - The second point of the capsule.
    /// * radius - The radius of the capsule.
    ///
    /// see also [`bounds_capsule_contains`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix};
    ///
    /// // A cube of 1 meter per side at this position, aligned with x y and z axis.
    /// let box_transform = Matrix::t([10.0, 1.0, 2.0]);
    /// let box_bounds = Bounds::bounds_centered([1.0,1.0,1.0]);
    ///
    /// let inv = box_transform.get_inverse();
    /// let point1 = inv.transform_point([10.2, 1.3, 2.4]);
    /// let point2 = inv.transform_point([10.9, 1.8, 2.1]);
    /// let point3 = inv.transform_point([10.5, 2.01, 1.7]);
    /// let point4 = inv.transform_point([9.5, 0.7, 1.5]);
    ///
    /// assert!(box_bounds.contains_capsule(point1, point2, 0.1),  "point1-point2 should be inside");
    /// assert!(!box_bounds.contains_capsule(point2, point3, 0.1), "point2-point3 * 0.1 should be outside");
    /// assert!(box_bounds.contains_capsule(point2, point3, 0.4),  "point2-point3 * 0.4 should be inside");
    /// assert!(box_bounds.contains_capsule(point3, point4, 0.1),  "point3-point4 should be inside");
    /// assert!(box_bounds.contains_capsule(point4, point1, 0.1),  "point4-point1 should be inside");
    ///
    /// assert!(box_bounds.contains_capsule([0.1, 0.1, 0.1], [0.9, 0.9, 0.9], 10.0),
    ///         "inverse line * 10 should be inside");
    /// ```
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
    /// see also [`bounds_ray_intersect`] same as [`Ray::intersect_bounds`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix, Ray};
    ///
    /// // A cube of 1 meter per side at this position, aligned with x y and z axis.
    /// let box_transform = Matrix::t([10.0, 1.0, 2.0]);
    /// let box_bounds = Bounds::bounds_centered([1.0,1.0,1.0]);
    ///
    /// let inv = box_transform.get_inverse();
    /// let ray1 = inv.transform_ray(Ray::new([10.2, 1.3, 2.4],[0.0, 1.0, 0.0]));
    /// let ray2 = inv.transform_ray(Ray::new([10.9, 1.8, 2.6],[0.0, 0.0, 1.0]));
    /// let ray3 = inv.transform_ray(Ray::new([10.5, 2.0, 2.7],[1.0, 0.0, 1.0]));
    ///
    /// assert!(box_bounds.intersect(ray1).is_some(),  "should be a point of contact");
    /// assert_eq!(box_bounds.intersect(ray2), None);
    /// assert_eq!(box_bounds.intersect(ray3),  None);
    /// assert_eq!(box_bounds.intersect(Ray::new([1.1, 1.1, 1.1], [-1.9, -1.9, -1.9])),
    ///             Some([0.5, 0.5, 0.5].into()));
    ///
    /// // We want the contact point for ray1
    /// let contact_point = box_bounds.intersect(ray1)
    ///         .expect ("There should be a point of contact");
    ///
    /// let contact_point = box_transform.transform_point(contact_point);
    /// assert_eq!(contact_point,  [10.2, 1.3, 2.4 ].into());
    /// ```
    #[inline]
    pub fn intersect<R: Into<Ray>>(&self, ray: R) -> Option<Vec3> {
        let mut pt = Vec3::default();
        let ray = ray.into();
        match unsafe { bounds_ray_intersect(*self, ray, &mut pt) != 0 } {
            true => Some(pt),
            false => None,
        }
    }

    /// Scale the bounds. It will scale the center as well as the dimensions! Returns a new Bounds.
    /// equivalent to using multiply operator
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scaled.html>
    /// * scale - The scale to apply.
    ///
    /// see also [Bounds::scale] [Bounds::scaled_vec] [Bounds::scale_vec] and '/' '*' operator
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// let bounds_scaled = bounds.scaled(2.0);
    /// let bounds_plus = Bounds::from_corners( Vec3::ZERO, Vec3::ONE) * 2.0;
    ///
    /// assert_eq!(*bounds.scale(2.0), bounds_scaled);
    /// assert_eq!(bounds_scaled.center, bounds_plus.center);
    /// assert_eq!(bounds_scaled.dimensions, bounds_plus.dimensions);
    /// ```
    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        *self * scale
    }

    /// Scale the bounds. It will scale the center as well as the dimensions! Returns a new Bounds.
    /// equivalent to using multiply operator
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Scaled.html>
    /// * scale - The scale to apply.
    ///
    /// see also [Bounds::scale_vec] [Bounds::scale] [Bounds::scaled] and '/' '*' operator
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    /// let bounds_scaled = bounds.scaled_vec([1.0, 2.0, 3.0]);
    /// let bounds_plus = Bounds::from_corners( Vec3::ZERO, Vec3::ONE) * Vec3::new(1.0, 2.0, 3.0);
    ///
    /// assert_eq!(bounds * Vec3::new(1.0, 2.0, 3.0), bounds_scaled);
    /// assert_eq!(bounds_scaled.center, bounds_plus.center);
    /// assert_eq!(bounds_scaled.dimensions, bounds_plus.dimensions);
    /// ```
    #[inline]
    pub fn scaled_vec(&self, scale: impl Into<Vec3>) -> Self {
        *self * scale.into()
    }

    /// This returns a Bounds that encapsulates the transformed points of the current Bounds’s corners.
    /// Note that this will likely introduce a lot of extra empty volume in many cases, as the result is still always axis aligned.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/Transformed.html>
    /// * transform - A transform Matrix for the current Bounds’s corners.
    ///
    /// see also [`bounds_transform]
    /// ### Examples
    ///```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix};
    ///
    /// let matrix = Matrix::r([90.0, 90.0, 90.0]);
    /// let bounds = Bounds::bounds_centered([1.0, 1.0, 1.0]);
    /// let bounds_transformed = bounds.transformed(matrix);
    ///
    /// assert_eq!(bounds_transformed.dimensions, Vec3{x: 1.0000001, y: 0.99999994, z: 0.99999994});
    /// assert_eq!(bounds_transformed.center, Vec3{x:0.0, y:0.0, z:0.0})
    ///```
    #[inline]
    pub fn transformed(&self, transform: impl Into<Matrix>) -> Self {
        unsafe { bounds_transform(*self, transform.into()) }
    }

    /// From the front, this is the Top (Y+), Left (X+), Center
    /// (Z0) of the bounds. Useful when working with UI layout bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/TLC.html>
    ///
    /// see also [Bounds::tlb]
    /// ### Examples
    ///```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::bounds_centered([1.0, 1.0, 1.0]);
    ///
    /// assert_eq!(bounds.tlc(), Vec3{x:0.5, y:0.5, z: 0.0});
    ///```    
    #[inline]
    pub fn tlc(&self) -> Vec3 {
        self.center + self.dimensions.xy0() / 2.0
    }

    /// From the front, this is the Top (Y+), Left (X+), Back (Z+)
    /// of the bounds. Useful when working with UI layout bounds.
    /// <https://stereokit.net/Pages/StereoKit/Bounds/TLB.html>
    ///
    /// see also [Bounds::tlb]
    /// ### Examples
    ///```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::bounds_centered([1.0, 1.0, 1.0]);
    ///
    /// assert_eq!(bounds.tlb(), Vec3{x:0.5, y:0.5, z: 0.5});
    ///```
    #[inline]
    pub fn tlb(&self) -> Vec3 {
        self.center + self.dimensions / 2.0
    }
}

impl Display for Bounds {
    /// Creates a text description of the Bounds, in the format of “[center:X dimensions:X]”
    /// <https://stereokit.net/Pages/StereoKit/Bounds/ToString.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::new([1.1, 2.0, 3.0], [4.0, 5.0, 6.0]);
    /// assert_eq!(format!("{}", bounds),
    ///            "[center:[x:1.1, y:2, z:3] dimensions:[x:4, y:5, z:6]]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[center:{} dimensions:{}]", self.center, self.dimensions)
    }
}
/// This operator will create a new Bounds that has been properly scaled up by the float. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl Mul<f32> for Bounds {
    type Output = Bounds;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    ///
    /// let bounds_scaled = bounds * 2.0;
    /// assert_eq!(bounds_scaled.center, Vec3::ONE);
    /// assert_eq!(bounds_scaled.dimensions, Vec3::new(2.0, 2.0, 2.0));
    /// ```
    fn mul(self, rhs: f32) -> Self::Output {
        Bounds { center: self.center * rhs, dimensions: self.dimensions * rhs }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the float. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl MulAssign<f32> for Bounds {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    ///
    /// bounds *= 2.0;
    /// assert_eq!(bounds.center, Vec3::ONE);
    /// assert_eq!(bounds.dimensions, Vec3::new(2.0, 2.0, 2.0));
    /// ```
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

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    ///
    /// let bounds_scaled = 2.0 * bounds;
    /// assert_eq!(bounds_scaled.center, Vec3::ONE);
    /// assert_eq!(bounds_scaled.dimensions, Vec3::new(2.0, 2.0, 2.0));
    /// ```
    fn mul(self, rhs: Bounds) -> Self::Output {
        Bounds { center: rhs.center * self, dimensions: rhs.dimensions * self }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the Vec3. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl Mul<Vec3> for Bounds {
    type Output = Bounds;

    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    ///
    /// let bounds_scaled = bounds * Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(bounds_scaled.center, Vec3::new(0.5, 1.0, 1.5));
    /// assert_eq!(bounds_scaled.dimensions, Vec3::new(1.0, 2.0, 3.0));
    /// ```
    fn mul(self, rhs: Vec3) -> Self::Output {
        Bounds { center: self.center * rhs, dimensions: self.dimensions * rhs }
    }
}

/// This operator will create a new Bounds that has been properly scaled up by the Vec3. This does affect the center
/// position of the Bounds.
/// <https://stereokit.net/Pages/StereoKit/Bounds/op_Multiply.html>
impl MulAssign<Vec3> for Bounds {
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds};
    ///
    /// let mut bounds = Bounds::from_corners( Vec3::ZERO, Vec3::ONE);
    ///
    /// bounds *= Vec3::new(1.0, 2.0, 3.0);
    /// assert_eq!(bounds.center, Vec3::new(0.5, 1.0, 1.5));
    /// assert_eq!(bounds.dimensions, Vec3::new(1.0, 2.0, 3.0));
    /// ```
    fn mul_assign(&mut self, rhs: Vec3) {
        self.center.mul_assign(rhs);
        self.dimensions.mul_assign(rhs);
    }
}

/// Planes are really useful for collisions, intersections, and visibility testing!
///
/// This plane is stored using the ax + by + cz + d = 0 formula, where the normal is a,b,c, and the d is, well, d.
/// <https://stereokit.net/Pages/StereoKit/Plane.html>
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Plane},
///                      mesh::Mesh, material::Material, util::named_colors};
///
/// let plane_mesh = Mesh::generate_plane_up([1.0,1.0], None, true);
/// let mut material_plane = Material::pbr();
///
/// let transform_wall = Matrix::t_r([-0.5, 0.0, 0.0],
///                                  [0.0, 0.0, 90.0]);
/// let transform_floor = Matrix::t([0.0, -0.5, 0.0]);
///
/// let wall =   Plane::new (Vec3::X, 0.5);
/// let wall_b = Plane::from_point( [-0.5, -1.1, -1.1].into(), Vec3::X);
/// let wall_c = Plane::from_points([-0.5, -2.2, -2.2],
///                                 [-0.5, -3.3, -3.3],
///                                 [-0.5, -4.4, -14.4]);
///
/// assert_eq!(wall.closest(Vec3::Y), Vec3 {x:-0.5, y:1.0, z:0.0});
/// assert_eq!(wall_b.closest(Vec3::Y), Vec3 {x:-0.5, y:1.0, z:0.0});
/// assert_eq!(wall_c.closest(Vec3::Y), Vec3 {x:-0.5, y:1.0, z:0.0});
/// assert_eq!(wall, wall_b);
/// //assert_eq!(wall, wall_c); // differents but the same
///
/// filename_scr = "screenshots/plane.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     plane_mesh.draw(token, &material_plane, transform_wall, Some(named_colors::CYAN.into()), None);
///     plane_mesh.draw(token, &material_plane, transform_floor, Some(named_colors::BLACK.into()), None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/plane.jpeg" alt="screenshot" width="200">
#[derive(Copy, Clone, Debug, Default)]
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

///  Warning: Equality with a precision of 0.1 millimeter
impl PartialEq for Plane {
    ///  Warning: Equality with a precision of 0.1 millimeter
    fn eq(&self, other: &Self) -> bool {
        self.normal == other.normal && self.d - other.d < 0.0001
    }
}

unsafe extern "C" {
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
    /// Unstable as some normal with NaN values may be created if the points are aligned.
    ///
    /// <https://stereokit.net/Pages/StereoKit/Plane/Plane.html>
    ///
    /// ## Examples
    /// ```
    /// use stereokit_rust::maths::{Plane,Vec3};
    /// let ground = Plane::from_points([1.0, 1.5, 0.0],
    ///                                 [0.0, 1.5, 1.0],
    ///                                 [1.0, 1.5, 1.0]);
    /// assert_eq!(ground.d , 1.5);
    /// assert!(ground.normal.y + 1.0 < 0.0001);
    ///
    /// let wall_c = Plane::from_points([-0.5, -0.4, -1.0],
    ///                                 [-0.5, 0.0, 3.0],
    ///                                 [-0.5, 0.3, -4.0]);
    /// println! ("{:?}", wall_c);
    /// assert_eq!(wall_c.d , 0.5);
    /// assert!(wall_c.normal.x - 1.0 < 0.0001);
    /// ```
    #[deprecated(since = "0.4.0", note = "Unstable! use `Plane::from_point` or `Plane::new` instead")]
    #[inline]
    pub fn from_points<V: Into<Vec3>>(point_on_plane1: V, point_on_plane2: V, point_on_plane3: V) -> Plane {
        let p1 = point_on_plane1.into();
        let p2 = point_on_plane2.into();
        let p3 = point_on_plane3.into();
        let dir1 = p2 - p1;
        let dir2 = p2 - p3;
        let mut normal = Vec3::cross(dir1, dir2).get_normalized();
        if normal.z.is_nan() {
            let dir1 = p1 - p3;
            let dir2 = p1 - p2;
            normal = Vec3::cross(dir1, dir2).get_normalized();
            if normal.z.is_nan() {
                let dir1 = p3 - p1;
                let dir2 = p3 - p2;
                normal = Vec3::cross(dir1, dir2).get_normalized();
            }
        }

        //let plane0 = Plane { normal, d: 0.0 };
        //let p0 = plane0.closest(p2);
        //Plane { normal, d: Vec3::distance(p0, p2) }
        Plane { normal, d: -Vec3::dot(p2, normal) }

        // Do not save the problem : unsafe{plane_from_points(p1, p2, p3)}
    }

    /// Finds the closest point on this plane to the given point!
    /// <https://stereokit.net/Pages/StereoKit/Plane/Closest.html>
    ///
    /// see also [`plane_point_closest`]
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
    /// see also [`plane_ray_intersect`] same as [`Ray::intersect_plane`]
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
    /// see also [`plane_line_intersect`]
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Plane};
    ///
    /// let plane = Plane::new([1.1, 2.0, 3.0], 4.0);
    /// assert_eq!(format!("{}", plane),
    ///            "[normal:[x:1.1, y:2, z:3] distance:4]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[normal:{} distance:{}]", self.normal, self.d)
    }
}

/// Pose represents a location and orientation in space, excluding scale! The default value of a Pose use
/// Pose.Identity .
/// <https://stereokit.net/Pages/StereoKit/Pose.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ui::Ui, maths::{Vec3, Pose, Matrix}, model::Model };
///
/// let plane = Model::from_file("plane.glb", None).unwrap_or_default();
/// let bounds = plane.get_bounds();
/// let mut handle_pose = Pose::look_at(
///     [0.0, -5.5, -10.0], (Vec3::Z + Vec3::X) * 100.0);
///
/// let mut window_pose = Pose::new(
///     [0.0, 0.05, 0.90], Some([0.0, 200.0, 0.0].into()));
///
/// filename_scr = "screenshots/pose.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     Ui::handle_begin( "Model Handle", &mut handle_pose,
///                       bounds, false, None, None);
///     plane.draw(token, Matrix::IDENTITY, None, None);
///     Ui::handle_end();
///
///     Ui::window_begin("My Window", &mut window_pose, None, None, None);
///     Ui::text("My Text", None, None, None, Some(0.14), None, None);
///     Ui::window_end();
/// );
/// ```
///
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/pose.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub orientation: Quat,
}

impl From<Matrix> for Pose {
    fn from(matrix: Matrix) -> Self {
        matrix.get_pose()
    }
}

impl Default for Pose {
    /// Position is Vec3::ZERO, and orientation is Quat::IDENTITY (no rotation)
    /// <https://stereokit.net/Pages/StereoKit/Pose/Identity.html>
    fn default() -> Self {
        Pose::IDENTITY
    }
}

impl Pose {
    /// The default Pose: Origin with Quat::IDENTITY orientation.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Identity.html>
    pub const IDENTITY: Pose = Pose { position: Vec3::new(0.0, 0.0, 0.0), orientation: Quat::IDENTITY };

    /// Zero may be encountered when testing some [`crate::system::Input`] [`crate::system::Pointer`] and [`crate::system::Controller`]
    pub const ZERO: Pose = Pose { position: Vec3::new(0.0, 0.0, 0.0), orientation: Quat::ZERO };

    /// Basic initialization constructor! Just copies in the provided values directly, and uses Identity for the
    /// orientation.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Pose.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose};
    /// let mut handle_pose = Pose::new( [0.0, 0.05, 0.90], None);
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.0, 0.05, 0.90], Some([0.0, 180.0 * 4.0, 0.0].into()));
    ///
    /// assert_eq!(handle_pose, window_pose);
    #[inline]
    pub fn new(position: impl Into<Vec3>, orientation: Option<Quat>) -> Self {
        let orientation = orientation.unwrap_or(Quat::IDENTITY);
        Self { position: position.into(), orientation }
    }

    /// Interpolates between two poses! It is unclamped, so values outside of (0,1) will extrapolate their position.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Lerp.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose};
    ///
    /// let mut pose1 = Pose::new( Vec3::Y, None);
    /// let mut pose2 = Pose::new( Vec3::Y, Some([0.0, 45.0, 0.0].into()));
    /// let mut pose3 = Pose::new( Vec3::Y, Some([0.0, 90.0, 0.0].into()));
    ///
    /// assert_eq!(Pose::lerp(pose1, pose3, 0.5), pose2);
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose};
    ///
    /// let mut pose1 = Pose::look_at(Vec3::ZERO, Vec3::NEG_Z );
    /// assert_eq!(pose1, Pose::default());
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Pose};
    ///
    /// let pose1 = Pose::IDENTITY;
    ///
    /// assert_eq!(pose1.to_matrix(None), Matrix::IDENTITY);    
    #[inline]
    pub fn to_matrix(&self, scale: Option<Vec3>) -> Matrix {
        match scale {
            Some(scale) => Matrix::t_r_s(self.position, self.orientation, scale),
            None => Matrix::t_r(self.position, self.orientation),
        }
    }

    /// Converts this pose into the inverse of the Pose's transform matrix. This can be used to
    /// transform points from the space represented by the Pose into world space.
    /// <https://stereokit.net/Pages/StereoKit/Pose/ToMatrixInv.html>
    /// * scale - A scale vector! Vec3.One would be an identity scale.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Matrix, Pose};
    ///
    /// let pose1 = Pose::IDENTITY;
    ///
    /// assert_eq!(pose1.to_matrix_inv(None), Matrix::IDENTITY);    
    ///
    /// assert_eq!(pose1.to_matrix_inv(Some(Vec3::new(0.5, 0.25, 0.125))),
    ///             [2.0, 0.0, 0.0, 0.0,
    ///              0.0, 4.0, 0.0, 0.0,
    ///              0.0, 0.0, 8.0, 0.0,
    ///              0.0, 0.0, 0.0, 1.0].into());
    /// ```
    #[inline]
    pub fn to_matrix_inv(&self, scale: Option<Vec3>) -> Matrix {
        let inv_orientation = self.orientation.conjugate();

        match scale {
            Some(scale) => {
                let inv_scale = 1.0 / scale;

                let inv_transform = inv_orientation.rotate_point(-self.position * inv_scale);

                Matrix::t_r_s(inv_transform, inv_orientation, inv_scale)
            }
            None => {
                let inv_transform = inv_orientation.rotate_point(-self.position);
                Matrix::t_r(inv_transform, inv_orientation)
            }
        }
    }

    /// Calculates the forward direction from this pose. This is done by multiplying the orientation with
    /// Vec3::new(0, 0, -1). Remember that Forward points down the -Z axis!
    /// <https://stereokit.net/Pages/StereoKit/Pose/Forward.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3,  Pose};
    ///
    /// let pose1 = Pose::default();
    /// assert_eq!(pose1.get_forward(), Vec3::NEG_Z);    
    ///
    /// let pose2 = Pose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0));
    /// assert_eq!(pose2.get_forward(), Vec3::NEG_X);    
    #[inline]
    pub fn get_forward(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::FORWARD)
    }

    /// This creates a ray starting at the Pose’s position, and pointing in the ‘Forward’ direction. The Ray
    /// direction is a unit vector/normalized.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Ray.html>
    ///
    /// see also [`Ray`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose, Ray};
    ///
    /// let pose1 = Pose::default();
    /// let ray_forward = Ray::new( Vec3::ZERO, Vec3::NEG_Z);
    /// assert_eq!(pose1.get_ray(), ray_forward);    
    ///
    /// let pose2 = Pose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.0));
    /// let ray_to_the_left = Ray::new(Vec3::X, Vec3::Y);
    /// assert_eq!(pose2.get_ray(), ray_to_the_left);   
    #[inline]
    pub fn get_ray(&self) -> Ray {
        Ray { position: self.position, direction: self.orientation.mul_vec3(Vec3::FORWARD) }
    }

    /// Calculates the right (+X) direction from this pose. This is done by multiplying the orientation with Vec3.Right.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Right.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose, Ray};
    ///
    /// let pose1 = Pose::default();
    /// assert_eq!(pose1.get_right(), Vec3::X);    
    ///
    /// let pose2 = Pose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(pose2.get_right(), Vec3::NEG_X);  
    /// ```
    #[inline]
    pub fn get_right(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::RIGHT)
    }

    /// Calculates the up (+Y) direction from this pose. This is done by multiplying the orientation with Vec3.Up.
    /// <https://stereokit.net/Pages/StereoKit/Pose/Up.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose, Ray};
    ///
    /// let pose1 = Pose::default();
    /// assert_eq!(pose1.get_up(), Vec3::Y);    
    ///
    /// let pose2 = Pose::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 3.0));
    /// assert_eq!(pose2.get_up(), Vec3::Y);  
    /// ```
    #[inline]
    pub fn get_up(&self) -> Vec3 {
        self.orientation.mul_vec3(Vec3::UP)
    }

    /// Check if this Pose is exactly equal to Pose::ZERO
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose};
    /// let pose1 = Pose::ZERO;
    /// let pose2 = Pose::IDENTITY;
    /// assert!(pose1.is_zero());
    /// assert!(!pose2.is_zero());
    /// ```
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.orientation.x == 0.0 && self.orientation.y == 0.0 && self.orientation.z == 0.0 && self.orientation.w == 0.0
    }
}

impl Display for Pose {
    /// A string representation of the Pose, in the format of “position, Forward”. Mostly for debug visualization.
    /// <https://stereokit.net/Pages/StereoKit/Pose/ToString.html>
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Pose};
    ///
    /// let pose = Pose::new([1.1, 2.0, 3.0], Some([0.0, 90.0, 0.0].into()));
    /// assert_eq!(format!("{}", pose),
    ///            "[position:[x:1.1, y:2, z:3] forward:[x:0, y:0.70710677, z:0, w:0.7071067]]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[position:{} forward:{}]", self.position, self.orientation)
    }
}

/// Represents a sphere in 3D space! Composed of a center point and a radius, can be used for raycasting, collision,
/// visibility, and other things!
/// <https://stereokit.net/Pages/StereoKit/Sphere.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Sphere, Ray}, system::Lines,
///     mesh::Mesh, material::Material, util::named_colors};
///
/// let sphere = Sphere::new(Vec3::ZERO, 0.5);
/// let sphere_mesh = Mesh::generate_sphere(sphere.radius * 2.0, Some(12));
/// let mut material_sphere = Material::pbr().copy();
/// material_sphere.color_tint(named_colors::GOLD)
///                .border_size(0.05);
///
/// let scale = 0.1;
/// let transform = Matrix::t(sphere.center);
/// let ray_x = Ray::new(Vec3::X, Vec3::NEG_X);
/// let ray_y = Ray::new(Vec3::Y, Vec3::NEG_Y);
/// let contact_x = sphere.intersect(ray_x).expect("X Should be there");
/// let contact_y = sphere.intersect(ray_y).expect("Y Should be there");
///
/// assert_eq!(contact_x, Vec3::X * sphere.radius);
/// assert_eq!(contact_y, Vec3::Y * sphere.radius);
///
/// filename_scr = "screenshots/sphere.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     sphere_mesh.draw(token, &material_sphere, transform, None, None);
///     Lines::add_ray(token, ray_x, 0.30, named_colors::RED, None, 0.04);
///     Lines::add_ray(token, ray_y, 0.30, named_colors::GREEN, None, 0.04);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sphere.jpeg" alt="screenshot" width="200">
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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

unsafe extern "C" {
    pub fn sphere_ray_intersect(sphere: Sphere, ray: Ray, out_pt: *mut Vec3) -> Bool32T;
    pub fn sphere_point_contains(sphere: Sphere, pt: Vec3) -> Bool32T;
}

impl Sphere {
    /// Creates a Sphere directly from the ax + by + cz + d = 0 formula!
    /// <https://stereokit.net/Pages/StereoKit/Sphere.html>
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Sphere};
    ///
    /// let sphere = Sphere::new(Vec3::ZERO, 0.5);
    /// let sphere_b = Sphere {center : Vec3::ZERO, radius : 0.5};
    ///
    /// assert_eq!(sphere, sphere_b);
    /// ```
    #[inline]
    pub fn new<V: Into<Vec3>>(center: V, radius: f32) -> Sphere {
        Sphere { center: center.into(), radius }
    }

    /// A fast check to see if the given point is contained in or on a sphere!
    /// <https://stereokit.net/Pages/StereoKit/Sphere/Contains.html>
    ///
    /// see also [`sphere_point_contains`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Sphere};
    ///
    /// let sphere = Sphere::new(Vec3::ZERO, 0.5);
    ///
    /// assert!(sphere.contains(Vec3::ONE * 0.28), "Should be contained");
    /// assert!(!sphere.contains(Vec3::ONE * 0.29), "Should not be contained");
    /// ```    
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
    /// see also [`sphere_ray_intersect`] same as [`Ray::intersect_sphere`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Sphere, Ray};
    ///
    /// let sphere = Sphere::new(Vec3::ZERO, 0.5);
    ///
    /// assert_eq!(sphere.intersect(Ray::new(Vec3::Z, Vec3::NEG_Z)), Some(Vec3::Z * 0.5));
    /// assert_ne!(sphere.intersect(Ray::new(Vec3::Z, Vec3::Z)),     None);
    /// ```
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Sphere};
    ///
    /// let sphere = Sphere::new([1.1, 2.0, 3.0], 4.0);
    /// assert_eq!(format!("{}", sphere),
    ///            "[center:[x:1.1, y:2, z:3] radius:4]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[center:{} radius:{}]", self.center, self.radius)
    }
}
/// A pretty straightforward 2D rectangle, defined by the top left corner of the rectangle, and its width/height.
/// <https://stereokit.net/Pages/StereoKit/Rect.html>
#[derive(Debug, Copy, Clone, PartialEq)]
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
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Rect};
    ///
    /// let rect = Rect::new(0.0, 0.0, 1920.0, 1080.0);
    /// let rect_b = Rect {x:0.0, y:0.0, width:1920.0, height:1080.0};
    ///
    /// assert_eq!(rect, rect_b);
    /// ```
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// A position and a direction indicating a ray through space! This is a great tool for intersection testing with
/// geometrical shapes.
/// <https://stereokit.net/Pages/StereoKit/Ray.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Ray}, model::Model, system::Lines,
///     mesh::Mesh, material::Material, util::named_colors};
///
/// let point = Mesh::sphere();
/// let mut material_point =Material::unlit();
/// let model = Model::from_file("center.glb", None).unwrap().copy();
/// let cube = Mesh::cube();
/// let mut material_cube =Material::ui_box();
/// material_cube.color_tint(named_colors::GOLD)
///              .border_size(0.05);
///
/// let center = Vec3::new(0.0, -2.5, -2.5);
/// let bounds = model.get_bounds();
/// let transform = Matrix::t_r(center, [0.0, 220.0, 0.0]);
/// let transform_cube = Matrix::t_s( bounds.center, bounds.dimensions) * transform;
/// let inv = transform.get_inverse();
///
/// let ray_x = Ray::new(Vec3{x:4.0, y: 0.0, z:  -2.5}, Vec3::NEG_X);
/// let inv_ray_x = inv.transform_ray(ray_x);
/// let inv_contact_x = bounds.intersect(inv_ray_x).expect("should be a point of contact");
/// let contact_x = transform.transform_point(inv_contact_x);
/// let transform_point_x = Matrix::t_s(contact_x, Vec3::ONE * 0.3);
///
/// filename_scr = "screenshots/ray.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, transform, None, None);
///     cube.draw(token, &material_cube, transform_cube, None, None);
///     Lines::add_ray(token, ray_x, 1.5, named_colors::WHITE, None, 0.2);
///     point.draw(token, &material_point, transform_point_x, Some(named_colors::RED.into()), None );
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ray.jpeg" alt="screenshot" width="200">
#[derive(Default, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Ray {
    /// The position or origin point of the Ray.
    pub position: Vec3,
    /// The direction the ray is facing, typically does not require being a unit vector, or normalized direction.
    pub direction: Vec3,
}

unsafe extern "C" {
    pub fn ray_intersect_plane(ray: Ray, plane_pt: Vec3, plane_normal: Vec3, out_t: *mut f32) -> Bool32T;
    pub fn ray_from_mouse(screen_pixel_pos: Vec2, out_ray: *mut Ray) -> Bool32T;
    pub fn ray_point_closest(ray: Ray, pt: Vec3) -> Vec3;
}

impl Ray {
    /// Ray Zero is the default
    pub const ZERO: Self = Ray { position: Vec3::ZERO, direction: Vec3::ZERO };

    /// Basic initialization constructor! Just copies the parameters into the fields.
    /// <https://stereokit.net/Pages/StereoKit/Ray/Ray.html>
    /// * position - The position or origin point of the Ray.
    /// * direction - The direction the ray is facing, typically does not require being a unit vector, or normalized
    ///   direction.
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    /// let ray_b = Ray {position: Vec3::ZERO, direction: Vec3::ONE};
    ///
    /// assert_eq!(ray, ray_b);
    /// ```
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
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    /// let ray_b = Ray::from_to( Vec3::ZERO, Vec3::ONE);
    ///
    /// assert_eq!(ray, ray_b);
    /// ```
    #[inline]
    pub fn from_to<V: Into<Vec3>>(a: V, b: V) -> Ray {
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
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    ///
    /// assert_eq!(ray.get_at(3.0), Vec3::ONE * 3.0);
    /// ```
    #[inline]
    pub fn get_at(&self, percent: f32) -> Vec3 {
        self.position + self.direction * percent
    }

    /// Calculates the point on the Ray that’s closest to the given point! This will be clamped if the point is behind
    /// the ray's origin.
    /// <https://stereokit.net/Pages/StereoKit/Ray/Closest.html>
    /// * to - Any point in the same coordinate space as the  Ray.
    ///
    /// Returns the point on the ray that's closest to the given point.
    /// see also [`ray_point_closest`]
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Plane, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    ///
    /// assert_eq!(ray.closest(Vec3::Z), Vec3::ONE / 3.0);
    /// ```
    #[inline]
    pub fn closest<V: Into<Vec3>>(&self, to: V) -> Vec3 {
        unsafe { ray_point_closest(*self, to.into()) }
    }

    /// Checks the intersection of this ray with a plane!
    /// <https://stereokit.net/Pages/StereoKit/Ray/Intersect.html>
    /// * plane - Any plane you want to intersect with.
    ///
    /// Returns intersection point if there's an intersection information or None if there's no intersection
    /// see also [`plane_ray_intersect`] same as [`Plane::intersect`]
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Plane, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    /// let plane = Plane::new(Vec3::NEG_Y, 2.5);
    ///
    /// assert_eq!(ray.intersect_plane(plane), Some(Vec3::ONE * 2.5));
    /// ```
    #[inline]
    pub fn intersect_plane(&self, plane: Plane) -> Option<Vec3> {
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
    /// see also [`sphere_ray_intersect`] same as [`Sphere::intersect`]
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Sphere, Ray};
    ///
    /// let ray = Ray::new(Vec3::ZERO, Vec3::ONE);
    /// let sphere = Sphere::new(Vec3::Y, 0.5);
    ///
    /// assert_eq!(ray.intersect_sphere(sphere), Some(Vec3::ONE * 0.5));
    /// ```
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
    /// see also [`bounds_ray_intersect`] same as [`Bounds::intersect`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Bounds, Matrix, Ray};
    ///
    /// // A cube of 1 meter per side at this position, aligned with x y and z axis.
    /// let box_transform = Matrix::t([10.0, 1.0, 2.0]);
    /// let box_bounds = Bounds::bounds_centered([1.0,1.0,1.0]);
    ///
    /// let inv = box_transform.get_inverse();
    /// let ray1 = inv.transform_ray(Ray::new([10.2, 1.3, 2.4],[0.0, 1.0, 0.0]));
    /// let ray2 = inv.transform_ray(Ray::new([10.9, 1.8, 2.6],[0.0, 0.0, 1.0]));
    /// let ray3 = inv.transform_ray(Ray::new([10.5, 2.0, 2.7],[1.0, 0.0, 1.0]));
    ///
    /// assert!(ray1.intersect_bounds(box_bounds).is_some(),  "should be a point of contact");
    /// assert_eq!(ray2.intersect_bounds(box_bounds), None);
    /// assert_eq!(ray3.intersect_bounds(box_bounds), None);
    ///
    /// // We want the contact point for ray1
    /// let contact_point = box_bounds.intersect(ray1)
    ///         .expect ("There should be a point of contact");
    ///
    /// let contact_point = box_transform.transform_point(contact_point);
    /// assert_eq!(contact_point,  [10.2, 1.3, 2.4 ].into());
    /// ```
    #[inline]
    pub fn intersect_bounds(&self, bounds: Bounds) -> Option<Vec3> {
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
    /// see also [`mesh_ray_intersect`] [`Ray::intersect_mesh_to_ptr`]  same as [`Mesh::intersect`]    
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Quat, Ray}, system::Lines,
    ///     util::{named_colors}, mesh::Mesh, material::{Material, Cull}};
    ///
    /// // Create Meshes
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
    /// let sphere = Mesh::generate_sphere(1.0, Some(4));
    ///
    /// let material = Material::pbr().copy();
    /// let transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
    /// let inv = transform.get_inverse();
    ///
    /// let ray = Ray::new([-3.0, 2.0, 0.5 ], [3.0, -2.0, -0.25]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let (contact_sphere, ind_sphere) = inv_ray.intersect_mesh( &sphere, Some(Cull::Front))
    ///     .expect("Ray should touch sphere");
    /// let (contact_cube, ind_cube) = inv_ray.intersect_mesh( &cube, Some(Cull::Back))
    ///     .expect("Ray should touch cube");
    /// assert_eq!(ind_sphere, 672);
    /// assert_eq!(ind_cube, 9);
    ///
    /// let transform_contact_sphere = Matrix::t_s(
    ///     transform.transform_point(contact_sphere), Vec3::ONE * 0.1);
    /// let transform_contact_cube = Matrix::t_s(
    ///     transform.transform_point(contact_cube), Vec3::ONE * 0.1);
    ///
    /// filename_scr = "screenshots/intersect_meshes.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cube.draw(token, &material, transform, Some(named_colors::CYAN.into()), None);
    ///     sphere.draw(token, &material, transform, Some(named_colors::BLUE.into()), None);
    ///     Lines::add_ray(token, ray, 2.2, named_colors::WHITE, None, 0.02);
    ///     sphere.draw(token, &material, transform_contact_cube,
    ///                 Some(named_colors::YELLOW.into()), None );
    ///     sphere.draw(token, &material, transform_contact_sphere,
    ///                 Some(named_colors::RED.into()), None );
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_meshes.jpeg" alt="screenshot" width="200">
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
    /// see also [`mesh_ray_intersect`] [`Ray::intersect_mesh`] same as [`Mesh::intersect_to_ptr`]  
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Quat, Ray}, system::Lines,
    ///     util::{named_colors}, mesh::Mesh, material::{Material, Cull}};
    ///
    /// // Create Meshes
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
    /// let sphere = Mesh::generate_sphere(1.0, Some(4));
    ///
    /// let material = Material::pbr().copy();
    /// let transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
    /// let inv = transform.get_inverse();
    ///
    /// let ray = Ray::new([-3.0, 2.0, 0.5 ], [3.0, -2.0, -0.25]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let (mut contact_sphere_ray, mut ind_sphere) = (Ray::default(), 0u32);
    /// assert!(inv_ray.intersect_mesh_to_ptr(
    ///             &sphere, Some(Cull::Front),
    ///             &mut contact_sphere_ray, &mut ind_sphere)
    ///     ,"Ray should touch sphere");
    ///
    /// let (mut contact_cube_ray, mut ind_cube) = (Ray::default(), 0u32);
    /// assert!( inv_ray.intersect_mesh_to_ptr(
    ///             &cube, Some(Cull::Back),
    ///             &mut contact_cube_ray, &mut ind_cube)
    ///     ,"Ray should touch cube");
    ///
    /// assert_eq!(ind_sphere, 672);
    /// assert_eq!(ind_cube, 9);
    ///
    /// assert_eq!(transform.transform_ray(contact_sphere_ray),
    ///         Ray { position:  Vec3 { x: 0.36746234, y: -0.244975, z: 0.21937825 },
    ///               direction: Vec3 { x: 0.58682406, y: -0.6427875, z: 0.49240398 }});
    /// assert_eq!(transform.transform_ray(contact_cube_ray),
    ///         Ray { position:  Vec3 { x: -0.39531866, y: 0.26354572, z: 0.2829433 },
    ///               direction: Vec3 { x: -0.77243483, y: -0.2620026, z: 0.57853174 } });
    /// ```
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
    /// see also [`model_ray_intersect`] [`Ray::intersect_model_to_ptr`] same as [`Model::intersect`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Ray}, model::Model, system::Lines,
    ///     mesh::Mesh, material::{Material, Cull}, util::named_colors};
    ///
    /// let model = Model::from_file("center.glb", None).unwrap().copy();
    /// let transform = Matrix::t_r([0.0,-2.25,-2.00], [0.0, 140.0, 0.0]);
    ///
    /// let inv_ray = Ray::new([1.0, 2.0, -3.0], [-1.5, 2.0, 3.0]);
    ///
    /// let contact_model = inv_ray.intersect_model( &model, Some(Cull::Back))
    ///     .expect("Ray should touch model");
    ///
    /// let ray = transform.transform_ray(inv_ray);
    /// let transform_contact_model = Matrix::t(transform.transform_point(contact_model));
    /// let point = Mesh::generate_sphere(0.2, Some(2));
    /// let material = Material::pbr();
    ///
    /// filename_scr = "screenshots/intersect_model.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform, None, None);
    ///     Lines::add_ray(token, ray, 1.2, named_colors::WHITE, None, 0.02);
    ///     point.draw(token, &material, transform_contact_model,
    ///                Some(named_colors::RED.into()), None );
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_model.jpeg" alt="screenshot" width="200">
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
    /// see also [`model_ray_intersect`] [`Ray::intersect_model`] same as [`Model::intersect_to_ptr`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Ray}, model::Model,
    ///     mesh::Mesh, material::{Material, Cull}, util::named_colors};
    ///
    /// let model = Model::from_file("center.glb", None).unwrap().copy();
    /// let transform = Matrix::t_r([0.0,-2.25,-2.00], [0.0, 140.0, 0.0]);
    ///
    /// let inv_ray = Ray::new([1.0, 2.0, -3.0], [-1.5, 2.0, 3.0]);
    ///
    /// let mut inv_contact_model_ray = Ray::default();
    /// assert!(inv_ray.intersect_model_to_ptr( &model, Some(Cull::Back), &mut inv_contact_model_ray)
    ///     ,"Ray should touch model");
    ///
    /// let contact_model_ray = transform.transform_ray(inv_contact_model_ray);
    /// assert_eq!(contact_model_ray,
    ///            Ray { position:  Vec3 { x: -0.3688636, y: 1.2613544, z: -1.3526915 },
    ///                  direction: Vec3 { x: -0.4004621, y: -0.016381653, z: 0.9161662 } });
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
    /// ### Examples
    /// ```
    /// use stereokit_rust::maths::{Vec3, Ray};
    ///
    /// let ray = Ray::new([1.1, 2.0, 3.0], [4.0, 5.0, 6.0]);
    /// assert_eq!(format!("{}", ray),
    ///            "[position:[x:1.1, y:2, z:3] direction:[x:4, y:5, z:6]]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[position:{} direction:{}]", self.position, self.direction)
    }
}
