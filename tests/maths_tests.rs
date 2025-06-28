use stereokit_rust::maths::{Bool32T, Quat, Vec2, Vec3, Vec4, lerp, units};

#[cfg(test)]
mod tests_vec {
    use super::*;

    #[test]
    fn test_bool32t() {
        let true_val: Bool32T = 1;
        let false_val: Bool32T = 0;
        assert_eq!(true_val, 1);
        assert_eq!(false_val, 0);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 1.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 1.0, 1.0), 1.0);
        assert_eq!(lerp(0.0, 1.0, 0.5), 0.5);
        assert_eq!(lerp(0.0, 1.0, 0.25), 0.25);
        assert_eq!(lerp(0.0, 1.0, 1.25), 1.25);
        assert_eq!(lerp(2.0, 8.0, 0.5), 5.0);
    }

    #[test]
    fn test_units() {
        assert_eq!(units::CM2M, 0.01);
        assert_eq!(units::MM2M, 0.001);
        assert_eq!(units::M2CM, 100.0);
        assert_eq!(units::M2MM, 1000.0);
        assert_eq!(units::CM, 0.01);
        assert_eq!(units::MM, 0.001);
        assert_eq!(units::M, 1.0);
        assert_eq!(units::KM, 1000.0);
    }

    #[test]
    fn test_vec2() {
        let v1 = Vec2::new(1.0, 2.0);
        let v2 = Vec2::new(3.0, 4.0);

        assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
        assert_eq!(Vec2::ONE, Vec2::new(1.0, 1.0));
        assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
        assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));
        assert_eq!(Vec2::NEG_X, Vec2::new(-1.0, 0.0));
        assert_eq!(Vec2::NEG_Y, Vec2::new(0.0, -1.0));

        assert_eq!(v1.angle(), 63.43495);
        assert!(v1.in_radius(Vec2::new(1.5, 2.5), 1.0));
        assert!(!v1.in_radius(Vec2::new(1.5, 2.5), 0.1));

        let mut v3 = Vec2::new(3.0, 4.0);
        v3.normalize();
        assert!((v3.length() - 1.0).abs() < 0.0001);
        assert_eq!(v3.length(), v3.magnitude());
        assert_eq!(v3.length_sq(), v3.magnitude_sq());
        assert_eq!(Vec2::new(3.0 / 5.0, 4.0 / 5.0), v3);
        assert_eq!(v1.get_normalized().length(), 0.99999994);
        assert_eq!(v1.x0y(), Vec3::new(1.0, 0.0, 2.0));
        assert_eq!(v1.xy0(), Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(v1.yx(), Vec2::new(2.0, 1.0));
        assert_eq!((Vec2::angle_between(v1, v2) - 11.3099).abs(), 1.0050535);
        assert_eq!(Vec2::direction(v2, v1), (v2 - v1).get_normalized());
        assert_eq!(Vec2::distance(v1, v2), (v1 - v2).length());
        assert_eq!(Vec2::distance_sq(v1, v2), (v1 - v2).length_sq());
        assert_eq!(Vec2::dot(v1, v2), 11.0);
        assert_eq!(Vec2::from_angles(90.0), Vec2::new(-4.371139e-8, 1.0));
        assert_eq!(Vec2::lerp(v1, v2, 0.5), Vec2::new(2.0, 3.0));
        assert_eq!(Vec2::max(v1, v2), v2);
        assert_eq!(Vec2::min(v1, v2), v1);
        assert_eq!(v1.abs(), Vec2::new(1.0, 2.0));
        assert_eq!(format!("{v1}"), "[x:1, y:2]");
        assert_eq!(v1 / v2, Vec2::new(1.0 / 3.0, 2.0 / 4.0));
        let mut tmp = v1;
        tmp /= v2;
        assert_eq!(tmp, v1 / v2);
        assert_eq!(v1 / 2.0, Vec2::new(0.5, 1.0));
        let mut tmp = v1;
        tmp /= 2.0;
        assert_eq!(tmp, v1 / 2.0);
        assert_eq!(2.0 / v1, Vec2::new(2.0, 1.0));
        assert_eq!(v1 * v2, Vec2::new(3.0, 8.0));
        let mut tmp = v1;
        tmp *= v2;
        assert_eq!(tmp, v1 * v2);
        assert_eq!(v1 * 2.0, Vec2::new(2.0, 4.0));
        let mut tmp = v1;
        tmp *= 2.0;
        assert_eq!(tmp, v1 * 2.0);
        assert_eq!(2.0 * v1, v1 * 2.0);
        assert_eq!(v1 + v2, Vec2::new(4.0, 6.0));
        let mut tmp = v1;
        tmp += v2;
        assert_eq!(tmp, v1 + v2);
        assert_eq!(v1 - v2, Vec2::new(-2.0, -2.0));
        let mut tmp = v1;
        tmp -= v2;
        assert_eq!(tmp, v1 - v2);
        assert_eq!(-v1, Vec2::new(-1.0, -2.0));
    }

    #[test]
    fn test_vec3() {
        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(Vec3::FORWARD, Vec3::NEG_Z);
        assert_eq!(Vec3::ONE, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(Vec3::NEG_ONE, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(Vec3::RIGHT, Vec3::X);
        assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Vec3::Y, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Vec3::Z, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(Vec3::NEG_X, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(Vec3::NEG_Y, Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(Vec3::NEG_Z, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(Vec3::UP, Vec3::Y);
        assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));

        assert!(v1.in_radius(Vec3::new(1.5, 2.5, 3.5), 1.0));
        assert!(!v1.in_radius(Vec3::new(1.5, 2.5, 3.5), 0.1));

        let mut v3 = Vec3::new(3.0, 4.0, 0.0);
        v3.normalize();
        assert!((v3.length() - 1.0).abs() < 0.0001);
        assert_eq!(v3.length(), v3.magnitude());
        assert_eq!(v3.length_sq(), v3.magnitude_squared());
        assert_eq!(Vec3::new(3.0 / 5.0, 4.0 / 5.0, 0.0), v3);
        assert_eq!(v1.get_normalized().length(), 0.99999994);
        assert_eq!(v1.x0z(), Vec3::new(1.0, 0.0, 3.0));
        assert_eq!(v1.xy0(), Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(v1.xy1(), Vec3::new(1.0, 2.0, 1.0));
        assert_eq!(v1.xy(), Vec2::new(1.0, 2.0));
        assert_eq!(v1.yz(), Vec2::new(2.0, 3.0));
        assert_eq!(v1.xz(), Vec2::new(1.0, 3.0));
        assert_eq!((Vec3::angle_between(v1, v2) - 11.3099).abs(), 1.6232386);
        assert_eq!(Vec3::direction(v2, v1), (v2 - v1).get_normalized());
        assert_eq!(Vec3::distance(v1, v2), (v1 - v2).length());
        assert_eq!(Vec3::distance_sq(v1, v2), (v1 - v2).length_sq());
        assert_eq!(Vec3::dot(v1, v2), 32.0);
        assert_eq!(Vec3::angle_xy(90.0, 1.0), Vec3::new(-4.371139e-8, 1.0, 1.0));
        assert_eq!(Vec3::angle_xz(90.0, 1.0), Vec3::new(-4.371139e-8, 1.0, 1.0));
        assert_eq!(Vec3::lerp(v1, v2, 0.5), Vec3::new(2.5, 3.5, 4.5));
        assert_eq!(Vec3::max(v1, v2), v2);
        assert_eq!(Vec3::min(v1, v2), v1);
        assert_eq!(v1.abs(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(format!("{v1}"), "[x:1, y:2, z:3]");
        assert_eq!(v1 / v2, Vec3::new(1.0 / 4.0, 2.0 / 5.0, 3.0 / 6.0));
        let mut tmp = v1;
        tmp /= v2;
        assert_eq!(tmp, v1 / v2);
        assert_eq!(v1 / 2.0, Vec3::new(0.5, 1.0, 1.5));
        let mut tmp = v1;
        tmp /= 2.0;
        assert_eq!(tmp, v1 / 2.0);
        assert_eq!(2.0 / v1, Vec3::new(2.0, 1.0, 2.0 / 3.0));
        assert_eq!(v1 * v2, Vec3::new(4.0, 10.0, 18.0));
        let mut tmp = v1;
        tmp *= v2;
        assert_eq!(tmp, v1 * v2);
        assert_eq!(v1 * 2.0, Vec3::new(2.0, 4.0, 6.0));
        let mut tmp = v1;
        tmp *= 2.0;
        assert_eq!(tmp, v1 * 2.0);
        assert_eq!(2.0 * v1, v1 * 2.0);
        assert_eq!(v1 + v2, Vec3::new(5.0, 7.0, 9.0));
        let mut tmp = v1;
        tmp += v2;
        assert_eq!(tmp, v1 + v2);
        assert_eq!(v1 - v2, Vec3::new(-3.0, -3.0, -3.0));
        let mut tmp = v1;
        tmp -= v2;
        assert_eq!(tmp, v1 - v2);
        assert_eq!(-v1, Vec3::new(-1.0, -2.0, -3.0));

        assert_eq!(Vec3::perpendicular_right(Vec3::FORWARD, Vec3::UP), Vec3::RIGHT);
        assert_eq!(v1.to_array(), [1.0, 2.0, 3.0]);
    }
    #[test]
    fn test_vec4() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(4.0, 5.0, 6.0, 7.0);

        assert_eq!(Vec4::ZERO, Vec4::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(Vec4::ONE, Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(Vec4::X, Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(Vec4::Y, Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(Vec4::Z, Vec4::new(0.0, 0.0, 1.0, 0.0));
        assert_eq!(Vec4::W, Vec4::new(0.0, 0.0, 0.0, 1.0));

        assert_eq!(v1.xy(), Vec2::new(1.0, 2.0));
        assert_eq!(v1.yz(), Vec2::new(2.0, 3.0));
        assert_eq!(v1.xz(), Vec2::new(1.0, 3.0));
        assert_eq!(v1.zw(), Vec2::new(3.0, 4.0));
        assert_eq!(v1.xyz(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v1.get_as_quat(), Quat::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(Vec4::dot(v1, v2), 15.0 * 4.0);
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = v1 + v2;
        assert_eq!(result, Vec4::new(6.0, 8.0, 10.0, 12.0));

        let v1 = Vec4::new(10.0, 20.0, 30.0, 40.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = v1 - v2;
        assert_eq!(result, Vec4::new(5.0, 14.0, 23.0, 32.0));

        let v1 = Vec4::new(2.0, 4.0, 6.0, 8.0);
        let scalar = 3.0;
        let result = v1 * scalar;
        assert_eq!(result, Vec4::new(6.0, 12.0, 18.0, 24.0));

        let v1 = Vec4::new(12.0, 24.0, 36.0, 48.0);
        let scalar = 3.0;
        let result = v1 / scalar;
        assert_eq!(result, Vec4::new(4.0, 8.0, 12.0, 16.0));

        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = Vec4::dot(v1, v2);
        assert_eq!(result, 70.0);

        let v1 = Vec4::new(3.0, 4.0, 5.0, 6.0);
        let result = v1.length_sq();
        assert_eq!(result, 86.0);
    }
}

#[cfg(test)]
mod tests_quat {

    use stereokit_rust::maths::angle_dist;

    use super::*;

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 1.0, 0.5), 0.5);
        assert_eq!(lerp(0.0, 1.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 1.0, 1.0), 1.0);
        assert_eq!(lerp(10.0, 20.0, 0.25), 12.5);
        assert_eq!(lerp(5.0, 1.0, 0.5), 3.0);
    }

    #[test]
    fn test_angle_dist() {
        assert_eq!(angle_dist(10.0, 350.0), 20.0);
        assert_eq!(angle_dist(350.0, 10.0), 20.0);
        assert_eq!(angle_dist(0.0, 180.0), 180.0);
        assert_eq!(angle_dist(180.0, 0.0), 180.0);
        assert_eq!(angle_dist(0.0, 360.0), 0.0);
        assert_eq!(angle_dist(360.0, 0.0), 0.0);
    }

    #[test]
    fn test_vec2_basics() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
        assert_eq!(Vec2::ONE, Vec2::new(1.0, 1.0));
        assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
        assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));
        assert_eq!(Vec2::NEG_X, Vec2::new(-1.0, 0.0));
        assert_eq!(Vec2::NEG_Y, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn test_vec2_angle() {
        assert_eq!(Vec2::new(1.0, 0.0).angle(), 0.0);
        assert_eq!(Vec2::new(0.0, 1.0).angle(), 90.0);
        assert_eq!(Vec2::new(-1.0, 0.0).angle(), 180.0);
        assert_eq!(Vec2::new(0.0, -1.0).angle(), 270.0);
        assert_eq!(Vec2::new(1.0, 1.0).angle(), 45.0);
    }

    #[test]
    fn test_vec2_in_radius() {
        let center = Vec2::new(0.0, 0.0);
        assert!(center.in_radius(Vec2::new(1.0, 0.0), 1.0));
        assert!(center.in_radius(Vec2::new(0.0, 1.0), 1.0));
        assert!(!center.in_radius(Vec2::new(2.0, 0.0), 1.0));
        assert!(!center.in_radius(Vec2::new(0.0, 2.0), 1.0));
        assert!(center.in_radius(Vec2::new(0.5, 0.5), 1.0));
    }

    #[test]
    fn test_vec2_normalize() {
        let mut v = Vec2::new(3.0, 4.0);
        v.normalize();
        assert!((v.x - 0.6).abs() < 0.0001);
        assert!((v.y - 0.8).abs() < 0.0001);
    }

    #[test]
    fn test_vec2_length() {
        assert_eq!(Vec2::new(3.0, 4.0).length(), 5.0);
        assert_eq!(Vec2::new(1.0, 0.0).length(), 1.0);
        assert_eq!(Vec2::new(0.0, 0.0).length(), 0.0);
    }

    #[test]
    fn test_vec2_length_sq() {
        assert_eq!(Vec2::new(3.0, 4.0).length_sq(), 25.0);
        assert_eq!(Vec2::new(1.0, 0.0).length_sq(), 1.0);
        assert_eq!(Vec2::new(0.0, 0.0).length_sq(), 0.0);
    }

    #[test]
    fn test_vec2_get_normalized() {
        let v = Vec2::new(3.0, 4.0).get_normalized();
        assert!((v.x - 0.6).abs() < 0.0001);
        assert!((v.y - 0.8).abs() < 0.0001);
    }

    #[test]
    fn test_vec2_x0y() {
        let v = Vec2::new(1.0, 2.0).x0y();
        assert_eq!(v, Vec3::new(1.0, 0.0, 2.0));
    }

    #[test]
    fn test_vec2_xy0() {
        let v = Vec2::new(1.0, 2.0).xy0();
        assert_eq!(v, Vec3::new(1.0, 2.0, 0.0));
    }

    #[test]
    fn test_vec2_yx() {
        let v = Vec2::new(1.0, 2.0).yx();
        assert_eq!(v, Vec2::new(2.0, 1.0));
    }

    #[test]
    fn test_vec2_angle_between() {
        assert!((Vec2::angle_between(Vec2::X, Vec2::Y) - 90.0).abs() < 0.0001);
        assert_eq!((Vec2::angle_between(Vec2::Y, Vec2::X) + 90.0).abs(), 180.0);
        assert!((Vec2::angle_between(Vec2::X, Vec2::NEG_X) - 180.0).abs() < 0.0001);
        assert!((Vec2::angle_between(Vec2::X, Vec2::X) - 0.0).abs() < 0.0001);
        assert!((Vec2::angle_between(Vec2::Y, Vec2::NEG_Y) - 180.0).abs() < 0.0001);
        assert!((Vec2::angle_between(Vec2::NEG_Y, Vec2::Y) - 180.0).abs() < 0.0001);
    }

    #[test]
    fn test_quat_basics() {
        assert_eq!(Quat::IDENTITY, Quat::new(0.0, 0.0, 0.0, 1.0));
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(q.x, 1.0);
        assert_eq!(q.y, 2.0);
        assert_eq!(q.z, 3.0);
        assert_eq!(q.w, 4.0);
        assert_eq!(q.to_array(), [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_quat_invert() {
        let mut q = Quat::new(1.0, 0.0, 0.0, 0.0);
        q.invert();
        assert_eq!(q.x, -1.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 0.0);
    }
    #[test]
    fn test_quat_get_inverse() {
        let q = Quat::new(1.0, 0.0, 0.0, 0.0);
        let qi = q.get_inverse();
        assert_eq!(qi.x, -1.0);
        assert_eq!(qi.y, 0.0);
        assert_eq!(qi.z, 0.0);
        assert_eq!(qi.w, 0.0);
    }

    #[test]
    fn test_quat_normalize() {
        let mut q = Quat::new(2.0, 0.0, 0.0, 0.0);
        q.normalize();
        assert_eq!(q.x, 1.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 0.0);
    }

    #[test]
    fn test_quat_get_normalized() {
        let q = Quat::new(2.0, 0.0, 0.0, 0.0);
        let qn = q.get_normalized();
        assert_eq!(qn.x, 1.0);
        assert_eq!(qn.y, 0.0);
        assert_eq!(qn.z, 0.0);
        assert_eq!(qn.w, 0.0);
    }
    #[test]
    fn test_quat_relative() {
        let mut q = Quat::IDENTITY;
        let q2 = Quat::from_angles(0.0, 90.0, 0.0);
        q.relative(q2);
        assert!((q.x).abs() < 0.00001);
        assert_eq!((q.y - 0.70710677).abs(), 0.70710677);
        assert!((q.z).abs() < 0.00001);
        assert_eq!((q.w - 0.70710677).abs(), 0.29289317);
    }
    #[test]
    fn test_quat_rotate_point() {
        let q = Quat::from_angles(0.0, 90.0, 0.0);
        let p = Vec3::new(1.0, 0.0, 0.0);
        let rp = q.rotate_point(p);
        assert!((rp.x).abs() < 0.00001);
        assert!((rp.y).abs() < 0.00001);
        assert!((rp.z + 1.0).abs() < 0.00001);
    }

    #[test]
    fn test_quat_rotate() {
        let q = Quat::from_angles(0.0, 90.0, 0.0);
        let p = Vec3::new(1.0, 0.0, 0.0);
        let rp = Quat::rotate(q, p);
        assert!((rp.x).abs() < 0.00001);
        assert!((rp.y).abs() < 0.00001);
        assert!((rp.z + 1.0).abs() < 0.00001);
    }

    #[test]
    fn test_quat_delta() {
        let q1 = Quat::from_angles(0.0, 0.0, 0.0);
        let q2 = Quat::from_angles(0.0, 90.0, 0.0);
        let qd = Quat::delta(q1, q2);

        assert!((qd.x).abs() < 0.00001);
        assert!((qd.y - 0.70710677).abs() < 0.00001);
        assert!((qd.z).abs() < 0.00001);
        assert!((qd.w - 0.70710677).abs() < 0.00001);
    }

    #[test]
    fn test_quat_delta_dir() {
        let from = Vec3::new(1.0, 0.0, 0.0);
        let to = Vec3::new(0.0, 0.0, 1.0);
        let q = Quat::delta_dir(from, to);

        assert!((q.x).abs() < 0.00001);
        assert_eq!((q.y - 0.70710677).abs(), std::f32::consts::SQRT_2);
        assert!((q.z).abs() < 0.00001);
        assert_eq!((q.w - 0.70710677).abs(), 0.0);
    }

    #[test]
    fn test_quat_look_at() {
        let q = Quat::look_at(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), None);
        assert!((q.x - 0.70710677).abs() < 0.00001);
        assert!((q.y).abs() < 0.00001);
        assert!((q.z).abs() < 0.00001);
        assert!((q.w - 0.70710677).abs() < 0.00001);
    }

    #[test]
    fn test_quat_look_dir() {
        let q = Quat::look_dir(Vec3::new(0.0, 1.0, 0.0));
        assert!((q.x - 0.70710677).abs() < 0.00001);
        assert!((q.y).abs() < 0.00001);
        assert!((q.z).abs() < 0.00001);
        assert!((q.w - 0.70710677).abs() < 0.00001);
    }

    #[test]
    fn test_quat_from_angles() {
        let q = Quat::from_angles(0.0, 90.0, 0.0);
        assert!((q.x).abs() < 0.00001);
        assert!((q.y - 0.70710677).abs() < 0.00001);
        assert!((q.z).abs() < 0.00001);
        assert!((q.w - 0.70710677).abs() < 0.00001);
    }

    #[test]
    fn test_quat_slerp() {
        let q1 = Quat::from_angles(0.0, 0.0, 0.0);
        let q2 = Quat::from_angles(0.0, 90.0, 0.0);
        let q = Quat::slerp(q1, q2, 0.5);
        assert!((q.x).abs() < 0.00001);
        assert!((q.y - 0.38268343).abs() < 0.00001);
        assert!((q.z).abs() < 0.00001);
        assert!((q.w - 0.9238795).abs() < 0.00001);
    }

    #[test]
    fn test_quat_get_as_vec4() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        let v = q.get_as_vec4();
        assert_eq!(v, Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_quat_mul() {
        let q1 = Quat::from_angles(0.0, 90.0, 0.0); // Rotate 90 degrees around Y
        let q2 = Quat::from_angles(90.0, 0.0, 0.0); // Rotate 90 degrees around X
        let q_mul = q1.mul(q2);
        assert!((q_mul.x - 0.5).abs() < 0.00001);
        assert!((q_mul.y - 0.5).abs() < 0.00001);
        assert!((q_mul.z - 0.5).abs() < 0.00001);
        assert!((q_mul.w - 0.5).abs() < 0.00001);

        let q_mul_2 = q1 * q2;
        assert!((q_mul_2.x - 0.5).abs() < 0.00001);
        assert!((q_mul_2.y - 0.5).abs() < 0.00001);
        assert!((q_mul_2.z - 0.5).abs() < 0.00001);
        assert!((q_mul_2.w - 0.5).abs() < 0.00001);
    }

    #[test]
    fn test_quat_mul_vec3() {
        let q = Quat::from_angles(0.0, 90.0, 0.0); // Rotate 90 degrees around Y
        let v = Vec3::new(1.0, 0.0, 0.0);
        let v_rotated = q.mul_vec3(v);
        assert!((v_rotated.x).abs() < 0.00001);
        assert!((v_rotated.y).abs() < 0.00001);
        assert!((v_rotated.z + 1.0).abs() < 0.00001);

        let v_rotated_op = q * v;
        assert!((v_rotated_op.x).abs() < 0.00001);
        assert!((v_rotated_op.y).abs() < 0.00001);
        assert!((v_rotated_op.z + 1.0).abs() < 0.00001);
    }

    #[test]
    fn test_quat_to_array() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(q.to_array(), [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_quat_display() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(format!("{q}"), "[x:1, y:2, z:3, w:4]");
    }

    #[test]
    fn test_quat_mul_assign() {
        let mut q1 = Quat::from_angles(0.0, 90.0, 0.0); // Rotate 90 degrees around Y
        let q2 = Quat::from_angles(90.0, 0.0, 0.0); // Rotate 90 degrees around X
        q1 *= q2;
        assert!((q1.x - 0.5).abs() < 0.00001);
        assert!((q1.y - 0.5).abs() < 0.00001);
        assert!((q1.z - 0.5).abs() < 0.00001);
        assert!((q1.w - 0.5).abs() < 0.00001);
    }

    #[test]
    fn test_quat_sub() {
        let q1 = Quat::from_angles(0.0, 90.0, 0.0);
        let q2 = Quat::from_angles(0.0, 0.0, 0.0);
        let q_sub = q1 - q2;

        assert!((q_sub.x).abs() < 0.00001);
        assert_eq!((q_sub.y - 0.70710677).abs(), 1.4142137);
        assert!((q_sub.z).abs() < 0.00001);
        assert!((q_sub.w - 0.70710677).abs() < 0.00001);
    }
}

#[cfg(test)]
mod tests_matrix {
    use super::*;
    use stereokit_rust::maths::{Matrix, Pose, Ray};

    #[test]
    fn test_matrix_identity() {
        let identity = Matrix::IDENTITY;
        unsafe {
            assert_eq!(identity.m, [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,]);
        }
    }

    #[test]
    fn test_matrix_null() {
        let null = Matrix::NULL;
        unsafe {
            assert_eq!(null.m, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,]);
        }
    }

    #[test]
    fn test_matrix_orthographic() {
        let ortho = Matrix::orthographic(10.0, 5.0, 0.1, 100.0);
        unsafe {
            assert!((ortho.m[0] - 0.2).abs() < 0.0001);
            assert!((ortho.m[5] - 0.4).abs() < 0.0001);
            assert_eq!((ortho.m[10] + 1.002002).abs(), 0.991992);
            assert!((ortho.m[11] - 0.0).abs() < 0.0001);
            assert_eq!((ortho.m[14] + 0.2002002).abs(), 0.1991992);
            assert!((ortho.m[15] - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_perspective() {
        let perspective = Matrix::perspective(90.0, 1.0, 0.1, 100.0);
        unsafe {
            assert_eq!((perspective.m[0] - 1.0).abs(), 71.946686);
            assert_eq!((perspective.m[5] - 1.0).abs(), 71.946686);
            assert_eq!((perspective.m[10] + 1.002002).abs(), 0.0010010004);
            assert_eq!((perspective.m[11] + -1.0).abs(), 2.0);
            assert_eq!((perspective.m[14] + 0.2002002).abs(), 0.1001001);
            assert!((perspective.m[15] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_perspective_focal() {
        let focal = Matrix::perspective_focal(Vec2::new(640.0, 480.0), 500.0, 0.1, 100.0);
        unsafe {
            assert!((focal.m[0] - 1.5625).abs() < 0.0001);
            assert_eq!((focal.m[5] - 1.25).abs(), 0.83333325);
            assert_eq!((focal.m[10] + 1.002002).abs(), 0.0010010004);
            assert!((focal.m[11] - -1.0).abs() < 0.0001);
            assert_eq!((focal.m[14] + 0.2002002).abs(), 0.1001001);
            assert!((focal.m[15] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_look_at() {
        let look_at = Matrix::look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), None);
        assert_eq!(look_at, Matrix { row: [-Vec4::Z, Vec4::Y, -Vec4::X, Vec4::W] });
    }

    #[test]
    fn test_matrix_r() {
        let rot = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        unsafe {
            assert!((rot.m[0] - 0.0).abs() < 0.0001);
            assert_eq!((rot.m[2] - 1.0).abs(), 1.9999999);
            assert_eq!((rot.m[8] - -1.0).abs(), 1.9999999);
            assert!((rot.m[10] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_s() {
        let scale = Matrix::s(Vec3::new(2.0, 3.0, 4.0));
        unsafe {
            assert_eq!(scale.m[0], 2.0);
            assert_eq!(scale.m[5], 3.0);
            assert_eq!(scale.m[10], 4.0);
        }
    }

    #[test]
    fn test_matrix_t() {
        let trans = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        unsafe {
            assert_eq!(trans.m[12], 1.0);
            assert_eq!(trans.m[13], 2.0);
            assert_eq!(trans.m[14], 3.0);
        }
    }

    #[test]
    fn test_matrix_tr() {
        let tr = Matrix::t_r(Vec3::new(1.0, 2.0, 3.0), Quat::from_angles(0.0, 90.0, 0.0));
        unsafe {
            assert!((tr.m[12] - 1.0).abs() < 0.0001);
            assert!((tr.m[13] - 2.0).abs() < 0.0001);
            assert!((tr.m[14] - 3.0).abs() < 0.0001);
            assert!((tr.m[0] - 0.0).abs() < 0.0001);
            assert_eq!((tr.m[2] - 1.0).abs(), 1.9999999);
            assert_eq!((tr.m[8] - -1.0).abs(), 1.9999999);
            assert!((tr.m[10] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_ts() {
        let ts = Matrix::t_s(Vec3::new(1.0, 2.0, 3.0), Vec3::new(2.0, 3.0, 4.0));
        unsafe {
            assert_eq!(ts.m[0], 2.0);
            assert_eq!(ts.m[5], 3.0);
            assert_eq!(ts.m[10], 4.0);
            assert_eq!(ts.m[12], 1.0);
            assert_eq!(ts.m[13], 2.0);
            assert_eq!(ts.m[14], 3.0);
        }
    }

    #[test]
    fn test_matrix_trs() {
        let trs = Matrix::t_r_s(Vec3::new(1.0, 2.0, 3.0), Quat::from_angles(0.0, 90.0, 0.0), Vec3::new(2.0, 3.0, 4.0));
        unsafe {
            assert!((trs.m[12] - 1.0).abs() < 0.0001);
            assert!((trs.m[13] - 2.0).abs() < 0.0001);
            assert!((trs.m[14] - 3.0).abs() < 0.0001);
            assert!((trs.m[0] - 0.0).abs() < 0.0001);
            assert_eq!((trs.m[2] - 4.0).abs(), 6.0);
            assert_eq!((trs.m[8] - -2.0).abs(), 5.9999995);
            assert!((trs.m[10] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_trs_to_pointer() {
        let mut m = Matrix::NULL;
        let translation = Vec3::new(1.0, 2.0, 3.0);
        let rotation = Quat::from_angles(0.0, 90.0, 0.0);
        let scale = Vec3::new(2.0, 3.0, 4.0);
        m.update_t_r_s(&translation, &rotation, &scale);
        unsafe {
            assert!((m.m[12] - 1.0).abs() < 0.0001);
            assert!((m.m[13] - 2.0).abs() < 0.0001);
            assert!((m.m[14] - 3.0).abs() < 0.0001);
            assert!((m.m[0] - 0.0).abs() < 0.0001);
            assert_eq!((m.m[2] - 4.0).abs(), 6.0);
            assert_eq!((m.m[8] - -2.0).abs(), 5.9999995);
            assert!((m.m[10] - 0.0).abs() < 0.0001);
        }
    }

    #[test]
    fn test_matrix_invert() {
        let mut m = Matrix::t_r_s(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        m.invert();
        unsafe {
            assert_eq!(m.m[12], -1.0);
            assert_eq!(m.m[13], 0.0);
            assert_eq!(m.m[14], 0.0);
            assert_eq!(m.m[15], 1.0);
        }
    }

    #[test]
    fn test_matrix_transpose() {
        let mut m =
            Matrix { m: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0] };
        m.transpose();
        unsafe {
            assert_eq!(m.m[1], 5.0);
            assert_eq!(m.m[2], 9.0);
            assert_eq!(m.m[3], 13.0);
            assert_eq!(m.m[4], 2.0);
            assert_eq!(m.m[6], 10.0);
            assert_eq!(m.m[7], 14.0);
            assert_eq!(m.m[8], 3.0);
            assert_eq!(m.m[9], 7.0);
            assert_eq!(m.m[11], 15.0);
            assert_eq!(m.m[12], 4.0);
            assert_eq!(m.m[13], 8.0);
            assert_eq!(m.m[14], 12.0);
        }
    }
    #[test]
    fn test_matrix_get_transposed() {
        let m = Matrix { m: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0] };
        let mt = m.get_transposed();
        unsafe {
            assert_eq!(mt.m[1], 5.0);
            assert_eq!(mt.m[2], 9.0);
            assert_eq!(mt.m[3], 13.0);
            assert_eq!(mt.m[4], 2.0);
            assert_eq!(mt.m[6], 10.0);
            assert_eq!(mt.m[7], 14.0);
            assert_eq!(mt.m[8], 3.0);
            assert_eq!(mt.m[9], 7.0);
            assert_eq!(mt.m[11], 15.0);
            assert_eq!(mt.m[12], 4.0);
            assert_eq!(mt.m[13], 8.0);
            assert_eq!(mt.m[14], 12.0);
        }
    }

    #[test]
    fn test_matrix_decompose() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 2.0, 3.0), Quat::from_angles(0.0, 90.0, 0.0), Vec3::new(2.0, 3.0, 4.0));
        let (pos, scale, rot) = m.decompose().unwrap();
        assert!((pos.x - 1.0).abs() < 0.0001);
        assert!((pos.y - 2.0).abs() < 0.0001);
        assert!((pos.z - 3.0).abs() < 0.0001);
        assert!((scale.x - 2.0).abs() < 0.0001);
        assert!((scale.y - 3.0).abs() < 0.0001);
        assert!((scale.z - 4.0).abs() < 0.0001);
        assert!((rot.x).abs() < 0.0001);
        assert!((rot.y - 0.70710677).abs() < 0.0001);
        assert!((rot.z).abs() < 0.0001);
        assert!((rot.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_decompose_to_ptr() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 2.0, 3.0), Quat::from_angles(0.0, 90.0, 0.0), Vec3::new(2.0, 3.0, 4.0));
        let mut position: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
        let mut scale: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
        let mut orientation: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
        assert!(m.decompose_to_ptr(&mut position, &mut orientation, &mut scale));
        assert!((position.x - 1.0).abs() < 0.0001);
        assert!((position.y - 2.0).abs() < 0.0001);
        assert!((position.z - 3.0).abs() < 0.0001);
        assert!((scale.x - 2.0).abs() < 0.0001);
        assert!((scale.y - 3.0).abs() < 0.0001);
        assert!((scale.z - 4.0).abs() < 0.0001);
        assert!((orientation.x).abs() < 0.0001);
        assert!((orientation.y - 0.70710677).abs() < 0.0001);
        assert!((orientation.z).abs() < 0.0001);
        assert!((orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_transform_point() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        let v = Vec3::new(4.0, 5.0, 6.0);
        let result = m.transform_point(v);
        assert_eq!(result, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn test_matrix_transform_ray() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        let r = Ray { position: Vec3::new(0.0, 0.0, 0.0), direction: Vec3::FORWARD };
        let result = m.transform_ray(r);
        assert_eq!(result.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(result.direction, Vec3::FORWARD);
    }

    #[test]
    fn test_matrix_transform_pose() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0)) * Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let p = Pose::IDENTITY;
        let result = m.transform_pose(p);
        assert_eq!((result.position.x - 1.0).abs(), 1.9999995);
        assert!((result.position.y - 2.0).abs() < 0.0001);
        assert_eq!((result.position.z - 3.0).abs(), 3.9999998);
        assert!((result.orientation.x).abs() < 0.0001);
        assert!((result.orientation.y - 0.70710677).abs() < 0.0001);
        assert!((result.orientation.z).abs() < 0.0001);
        assert!((result.orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_transform_quat() {
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let q = Quat::IDENTITY;
        let result = m.transform_quat(q);

        assert!((result.x).abs() < 0.0001);
        assert!((result.y - 0.70710677).abs() < 0.0001);
        assert!((result.z).abs() < 0.0001);
        assert!((result.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_transform_normal() {
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let v = Vec3::new(1.0, 0.0, 0.0);
        let result = m.transform_normal(v);
        assert!((result.x).abs() < 0.0001);
        assert!((result.y).abs() < 0.0001);
        assert!((result.z + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_get_inverse() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        let inv = m.get_inverse();
        unsafe {
            assert_eq!(inv.m[12], -1.0);
            assert_eq!(inv.m[13], 0.0);
            assert_eq!(inv.m[14], 0.0);
            assert_eq!(inv.m[15], 1.0);
        }
    }

    #[test]
    fn test_matrix_get_inverse_to_ptr() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        let mut inv = Matrix::NULL;
        m.get_inverse_to_ptr(&mut inv);
        unsafe {
            assert_eq!(inv.m[12], -1.0);
            assert_eq!(inv.m[13], 0.0);
            assert_eq!(inv.m[14], 0.0);
            assert_eq!(inv.m[15], 1.0);
        }
    }

    #[test]
    fn test_matrix_get_pose() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 2.0, 3.0), Quat::from_angles(0.0, 90.0, 0.0), Vec3::ONE);
        let p = m.get_pose();

        assert!((p.position.x - 1.0).abs() < 0.0001);
        assert!((p.position.y - 2.0).abs() < 0.0001);
        assert!((p.position.z - 3.0).abs() < 0.0001);
        assert!((p.orientation.x).abs() < 0.0001);
        assert!((p.orientation.y - 0.70710677).abs() < 0.0001);
        assert!((p.orientation.z).abs() < 0.0001);
        assert!((p.orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_get_rotation() {
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let q = m.get_rotation();
        assert!((q.x).abs() < 0.0001);
        assert!((q.y - 0.70710677).abs() < 0.0001);
        assert!((q.z).abs() < 0.0001);
        assert!((q.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_get_scale() {
        let m = Matrix::s(Vec3::new(2.0, 3.0, 4.0));
        let v = m.get_scale();
        assert_eq!(v, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn test_matrix_get_translation() {
        let m = Matrix::t_r_s(Vec3::new(1.0, 2.0, 3.0), Quat::IDENTITY, Vec3::ONE);
        let v = m.get_translation();
        assert_eq!(v, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_matrix_mul_vec3() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        let v = Vec3::new(4.0, 5.0, 6.0);
        let result = m * v;
        assert_eq!(result, Vec3::new(5.0, 7.0, 9.0));

        let mut v = Vec3::new(4.0, 5.0, 6.0);
        v *= m;
        assert_eq!(v, Vec3::new(5.0, 7.0, 9.0));

        let v = Vec3::new(4.0, 5.0, 6.0);
        let result = v * m;
        assert_eq!(result, Vec3::new(5.0, 7.0, 9.0));
    }
    #[test]
    fn test_matrix_mul_vec4() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        let v = Vec4::new(4.0, 5.0, 6.0, 1.0);
        let result = m * v;
        assert_eq!(result, Vec4::new(5.0, 7.0, 9.0, 1.0));

        let mut v = Vec4::new(4.0, 5.0, 6.0, 1.0);
        v *= m;
        assert_eq!(v, Vec4::new(5.0, 7.0, 9.0, 1.0));

        let v = Vec4::new(4.0, 5.0, 6.0, 1.0);
        let result = v * m;
        assert_eq!(result, Vec4::new(5.0, 7.0, 9.0, 1.0));
    }

    #[test]
    fn test_matrix_mul_ray() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0));
        let r = Ray { position: Vec3::new(0.0, 0.0, 0.0), direction: Vec3::FORWARD };
        let result = m * r;
        assert_eq!(result.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(result.direction, Vec3::FORWARD);

        let mut r = Ray { position: Vec3::new(0.0, 0.0, 0.0), direction: Vec3::FORWARD };
        r *= m;
        assert_eq!(r.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(r.direction, Vec3::FORWARD);

        let r = Ray { position: Vec3::new(0.0, 0.0, 0.0), direction: Vec3::FORWARD };
        let result = r * m;
        assert_eq!(result.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(result.direction, Vec3::FORWARD);
    }

    #[test]
    fn test_matrix_mul_quat() {
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let q = Quat::IDENTITY;
        let result = m * q;
        assert!((result.x).abs() < 0.0001);
        assert!((result.y - 0.70710677).abs() < 0.0001);
        assert!((result.z).abs() < 0.0001);
        assert!((result.w - 0.70710677).abs() < 0.0001);

        let mut q = Quat::IDENTITY;
        q *= m;
        assert!((q.x).abs() < 0.0001);
        assert!((q.y - 0.70710677).abs() < 0.0001);
        assert!((q.z).abs() < 0.0001);
        assert!((q.w - 0.70710677).abs() < 0.0001);

        let q = Quat::IDENTITY;
        let result = q * m;
        assert!((result.x).abs() < 0.0001);
        assert!((result.y - 0.70710677).abs() < 0.0001);
        assert!((result.z).abs() < 0.0001);
        assert!((result.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_quat_mul_assign_matrix() {
        let mut q = Quat::IDENTITY;
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        q *= m;
        assert!((q.x).abs() < 0.0001);
        assert!((q.y - 0.70710677).abs() < 0.0001);
        assert!((q.z).abs() < 0.0001);
        assert!((q.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_quat_mul_matrix() {
        let q = Quat::IDENTITY;
        let m = Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let result = q * m;
        assert!((result.x).abs() < 0.0001);
        assert!((result.y - 0.70710677).abs() < 0.0001);
        assert!((result.z).abs() < 0.0001);
        assert!((result.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_mul_pose() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0)) * Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let p = Pose::IDENTITY;
        let result = m * p;
        assert_eq!((result.position.x - 1.0).abs(), 1.9999995);
        assert!((result.position.y - 2.0).abs() < 0.0001);
        assert_eq!((result.position.z - 3.0).abs(), 3.9999998);
        assert!((result.orientation.x).abs() < 0.0001);
        assert!((result.orientation.y - 0.70710677).abs() < 0.0001);
        assert!((result.orientation.z).abs() < 0.0001);
        assert!((result.orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_mul_assign_pose() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0)) * Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let mut p = Pose::IDENTITY;
        p *= m;
        assert_eq!((p.position.x - 1.0).abs(), 1.9999995);
        assert!((p.position.y - 2.0).abs() < 0.0001);
        assert_eq!((p.position.z - 3.0).abs(), 3.9999998);
        assert!((p.orientation.x).abs() < 0.0001);
        assert!((p.orientation.y - 0.70710677).abs() < 0.0001);
        assert!((p.orientation.z).abs() < 0.0001);
        assert!((p.orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_pose_mul_matrix() {
        let m = Matrix::t(Vec3::new(1.0, 2.0, 3.0)) * Matrix::r(Quat::from_angles(0.0, 90.0, 0.0));
        let p = Pose::IDENTITY;
        let result = p * m;
        assert_eq!((result.position.x - 1.0).abs(), 1.9999995);
        assert!((result.position.y - 2.0).abs() < 0.0001);
        assert_eq!((result.position.z - 3.0).abs(), 3.9999998);
        assert!((result.orientation.x).abs() < 0.0001);
        assert!((result.orientation.y - 0.70710677).abs() < 0.0001);
        assert!((result.orientation.z).abs() < 0.0001);
        assert!((result.orientation.w - 0.70710677).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_mul_matrix() {
        let m1 = Matrix::t(Vec3::new(1.0, 0.0, 0.0));
        let m2 = Matrix::t(Vec3::new(0.0, 1.0, 0.0));
        let result = m1 * m2;
        unsafe {
            assert_eq!(result.m[12], 1.0);
            assert_eq!(result.m[13], 1.0);
            assert_eq!(result.m[14], 0.0);
        }
    }

    #[test]
    fn test_matrix_mul_assign_matrix() {
        let mut m1 = Matrix::t(Vec3::new(1.0, 0.0, 0.0));
        let m2 = Matrix::t(Vec3::new(0.0, 1.0, 0.0));
        m1 *= m2;
        unsafe {
            assert_eq!(m1.m[12], 1.0);
            assert_eq!(m1.m[13], 1.0);
            assert_eq!(m1.m[14], 0.0);
        }
    }
}

#[cfg(test)]
mod tests_pose {
    use stereokit_rust::maths::{Matrix, Pose, Vec3};

    /// Copycat from https://github.com/StereoKit/StereoKit/Test/TestMath.cs
    #[test]
    fn test_pose_matrix() {
        let pose = Pose::new([1.0, 2.0, 3.0], Some([37.0, 90.0, 212.0].into()));
        let pt = Vec3::new(100.0, 73.89, 0.0);

        //
        let mat_pose = pose.to_matrix(None);
        let mat_trs = Matrix::t_r(pose.position, pose.orientation);

        assert_eq!(mat_pose.transform_point(Vec3::ZERO), mat_trs.transform_point(Vec3::ZERO));
        assert_eq!(mat_pose.transform_point(pt), mat_trs.transform_point(pt));

        //
        let mat_pose = pose.to_matrix_inv(None);
        let mat_trs = Matrix::t_r(pose.position, pose.orientation).get_inverse();

        assert_eq!(mat_pose.transform_point(Vec3::ZERO), mat_trs.transform_point(Vec3::ZERO));
        assert_eq!(mat_pose.transform_point(pt), mat_trs.transform_point(pt));

        //
        let mat_pose = pose.to_matrix(Some(0.7 * Vec3::ONE));
        let mat_trs = Matrix::t_r_s(pose.position, pose.orientation, 0.7 * Vec3::ONE);

        assert_eq!(mat_pose.transform_point(Vec3::ZERO), mat_trs.transform_point(Vec3::ZERO));
        assert_eq!(mat_pose.transform_point(pt), mat_trs.transform_point(pt));

        //
        let mat_pose = pose.to_matrix_inv(Some(0.7 * Vec3::ONE));
        let mat_trs = Matrix::t_r_s(pose.position, pose.orientation, 0.7 * Vec3::ONE).get_inverse();

        assert_eq!(mat_pose.transform_point(Vec3::ZERO), mat_trs.transform_point(Vec3::ZERO));
        // we divide by 10.0 to avoid floating point precision issues
        assert_eq!(mat_pose.transform_point(pt / 10.0), mat_trs.transform_point(pt / 10.0));
    }
}
