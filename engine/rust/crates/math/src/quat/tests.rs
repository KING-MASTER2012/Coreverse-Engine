//! Tests for the quaternions of Phase 1.10.
//!
//! Instantiated once per float family (`Quat` over `f32`, `DQuat` over
//! `f64`). Rotations are compared as rotations, not as component lists:
//! `q` and `-q` are the same rotation, and most conversions may return
//! either sign.

use super::*;
use crate::angle::Degrees;

macro_rules! quat_tests {
    (
        $module:ident, $scalar:ty,
        $quat:ident, $v3:ident, $v4:ident, $m3:ident, $m4:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-5;
            const PI: $scalar = <$scalar as Scalar>::PI;

            fn close(a: $scalar, b: $scalar) -> bool {
                (a - b).abs() <= TOL
            }

            fn quat_close(a: $quat, b: $quat) -> bool {
                a.to_array()
                    .iter()
                    .zip(b.to_array())
                    .all(|(x, y)| close(*x, y))
            }

            /// Whether two quaternions are the same rotation (`q` or `-q`).
            fn same_rotation(a: $quat, b: $quat) -> bool {
                quat_close(a, b) || quat_close(a, -b)
            }

            fn rad(value: $scalar) -> Radians<$scalar> {
                Radians::new(value)
            }

            fn vec_close(a: $v3, b: $v3) -> bool {
                close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z)
            }

            fn mat_close(a: $m3, b: $m3) -> bool {
                a.to_cols_array()
                    .iter()
                    .zip(b.to_cols_array())
                    .all(|(x, y)| close(*x, y))
            }

            macro_rules! assert_same_rotation {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a, $b);
                    assert!(
                        same_rotation(a, b),
                        "{:?} is not the rotation of {:?}",
                        a,
                        b
                    );
                }};
            }

            macro_rules! assert_vec_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a, $b);
                    assert!(vec_close(a, b), "{:?} is not close to {:?}", a, b);
                }};
            }

            /// A deterministic stream of values in `-1.0..=1.0`.
            struct Stream(u32);

            impl Stream {
                fn sample(&mut self) -> $scalar {
                    self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let high = (self.0 >> 16) as u16;
                    <$scalar>::from(high) / 32767.5 - 1.0
                }

                fn vector(&mut self) -> $v3 {
                    $v3::new(self.sample(), self.sample(), self.sample())
                }

                /// A random rotation, as a unit quaternion.
                fn rotation(&mut self) -> $quat {
                    loop {
                        let q =
                            $quat::new(self.sample(), self.sample(), self.sample(), self.sample());
                        if q.length() > 0.2 {
                            return q.normalize();
                        }
                    }
                }
            }

            fn quarter_turn_z() -> $quat {
                $quat::from_rotation_z(rad(PI / 2.0))
            }

            #[test]
            fn identity_default_and_constructors() {
                assert_eq!($quat::IDENTITY.to_array(), [0.0, 0.0, 0.0, 1.0]);
                assert_eq!($quat::default(), $quat::IDENTITY);
                let q = $quat::new(1.0, 2.0, 3.0, 4.0);
                assert_eq!((q.x, q.y, q.z, q.w), (1.0, 2.0, 3.0, 4.0));
                assert_eq!(q.to_array(), [1.0, 2.0, 3.0, 4.0]);
                assert_eq!($quat::from_array([1.0, 2.0, 3.0, 4.0]), q);
                assert_eq!($quat::from([1.0, 2.0, 3.0, 4.0]), q);
                let array: [$scalar; 4] = q.into();
                assert_eq!(array, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(q.xyz(), $v3::new(1.0, 2.0, 3.0));
            }

            #[test]
            fn the_memory_layout_is_four_scalars_without_padding() {
                assert_eq!(
                    core::mem::size_of::<$quat>(),
                    4 * core::mem::size_of::<$scalar>()
                );
            }

            #[test]
            fn axis_angle_rotations_are_right_handed() {
                // A quarter turn counter-clockwise: X -> Y -> -X, and so on.
                assert_vec_close!(quarter_turn_z() * $v3::X, $v3::Y);
                assert_vec_close!(quarter_turn_z() * $v3::Y, -$v3::X);
                assert_vec_close!($quat::from_rotation_x(rad(PI / 2.0)) * $v3::Y, $v3::Z);
                assert_vec_close!($quat::from_rotation_y(rad(PI / 2.0)) * $v3::Z, $v3::X);
                // The axis itself is untouched.
                assert_vec_close!(quarter_turn_z() * $v3::Z, $v3::Z);
            }

            #[test]
            fn from_axis_angle_accepts_radians_and_degrees() {
                let from_radians = $quat::from_axis_angle($v3::Z, Radians::new(PI / 2.0));
                let from_degrees = $quat::from_axis_angle($v3::Z, Degrees::new(90.0));
                let from_scalar_rotation = $quat::from_rotation_z(Degrees::new(90.0));
                assert!(quat_close(from_radians, from_degrees));
                assert!(quat_close(from_radians, from_scalar_rotation));
                assert!(quat_close(from_radians, quarter_turn_z()));
            }

            #[test]
            fn from_axis_angle_builds_the_documented_components() {
                let q = $quat::from_axis_angle($v3::Y, rad(PI));
                assert!(close(q.x, 0.0) && close(q.y, 1.0) && close(q.z, 0.0) && close(q.w, 0.0));
                let axis = $v3::new(1.0, 2.0, 2.0) / 3.0;
                let q = $quat::from_axis_angle(axis, rad(1.0));
                let half: $scalar = 0.5;
                let (sin, cos) = (Scalar::sin(half), Scalar::cos(half));
                assert!(close(q.x, axis.x * sin) && close(q.w, cos));
                assert!(close(q.length(), 1.0));
                assert_eq!($quat::from_axis_angle($v3::Z, rad(0.0)), $quat::IDENTITY);
            }

            #[test]
            fn rotating_a_vector_keeps_its_length() {
                let mut stream = Stream(7);
                for _ in 0..500 {
                    let (q, v) = (stream.rotation(), stream.vector());
                    assert!(close((q * v).length(), v.length()), "{q:?} {v:?}");
                }
            }

            #[test]
            fn a_half_turn_flips_the_perpendicular_components() {
                let half_turn = $quat::from_rotation_y(rad(PI));
                assert_vec_close!(
                    half_turn * $v3::new(1.0, 2.0, 3.0),
                    $v3::new(-1.0, 2.0, -3.0)
                );
                assert_eq!(
                    $quat::IDENTITY * $v3::new(1.0, 2.0, 3.0),
                    $v3::new(1.0, 2.0, 3.0)
                );
            }

            #[test]
            fn a_full_turn_is_minus_identity_and_still_the_identity_rotation() {
                let full = $quat::from_rotation_x(rad(2.0 * PI));
                assert!(quat_close(full, -$quat::IDENTITY));
                assert_vec_close!(full * $v3::new(1.0, 2.0, 3.0), $v3::new(1.0, 2.0, 3.0));
            }

            #[test]
            fn hamilton_products_of_the_basis_quaternions() {
                let (i, j, k) = (
                    $quat::new(1.0, 0.0, 0.0, 0.0),
                    $quat::new(0.0, 1.0, 0.0, 0.0),
                    $quat::new(0.0, 0.0, 1.0, 0.0),
                );
                assert_eq!(i * j, k);
                assert_eq!(j * k, i);
                assert_eq!(k * i, j);
                assert_eq!(j * i, -k);
                assert_eq!(i * i, $quat::new(0.0, 0.0, 0.0, -1.0));
                assert_eq!(j * j, $quat::new(0.0, 0.0, 0.0, -1.0));
                assert_eq!(k * k, $quat::new(0.0, 0.0, 0.0, -1.0));
            }

            #[test]
            fn a_product_applies_the_right_factor_first() {
                let about_z = quarter_turn_z();
                let about_x = $quat::from_rotation_x(rad(PI / 2.0));
                // Y -> Z by the X turn, which the Z turn leaves alone.
                assert_vec_close!((about_z * about_x) * $v3::Y, $v3::Z);
                // Y -> -X by the Z turn, which the X turn leaves alone.
                assert_vec_close!((about_x * about_z) * $v3::Y, -$v3::X);
                assert!(!same_rotation(about_z * about_x, about_x * about_z));
            }

            #[test]
            fn composition_equals_applying_one_after_the_other() {
                let mut stream = Stream(11);
                for _ in 0..200 {
                    let (a, b, v) = (stream.rotation(), stream.rotation(), stream.vector());
                    assert_vec_close!((a * b) * v, a * (b * v));
                }
            }

            #[test]
            fn products_are_associative_and_identity_is_neutral() {
                let mut stream = Stream(13);
                for _ in 0..100 {
                    let (a, b, c) = (stream.rotation(), stream.rotation(), stream.rotation());
                    assert_same_rotation!((a * b) * c, a * (b * c));
                    assert_eq!($quat::IDENTITY * a, a);
                    assert_eq!(a * $quat::IDENTITY, a);
                }
                let mut q = quarter_turn_z();
                q *= quarter_turn_z();
                assert_same_rotation!(q, $quat::from_rotation_z(rad(PI)));
            }

            #[test]
            fn conjugate_undoes_a_unit_rotation() {
                let mut stream = Stream(17);
                for _ in 0..200 {
                    let (q, v) = (stream.rotation(), stream.vector());
                    assert_vec_close!(q.conjugate() * (q * v), v);
                    assert!(quat_close(q * q.conjugate(), $quat::IDENTITY));
                }
                assert_eq!(
                    $quat::new(1.0, 2.0, 3.0, 4.0).conjugate(),
                    $quat::new(-1.0, -2.0, -3.0, 4.0)
                );
            }

            #[test]
            fn inverse_works_for_non_unit_quaternions_too() {
                let q = $quat::new(1.0, -2.0, 0.5, 3.0);
                assert!(quat_close(q * q.inverse(), $quat::IDENTITY));
                assert!(quat_close(q.inverse() * q, $quat::IDENTITY));
                assert_eq!(
                    $quat::new(0.0, 0.0, 0.0, 2.0).inverse(),
                    $quat::new(0.0, 0.0, 0.0, 0.5)
                );
                // For unit quaternions it is the conjugate.
                let unit = quarter_turn_z();
                assert!(quat_close(unit.inverse(), unit.conjugate()));
                let zero = $quat::new(0.0, 0.0, 0.0, 0.0).inverse();
                assert!(!zero.w.is_finite());
            }

            #[test]
            fn dot_and_length() {
                let (a, b) = (
                    $quat::new(1.0, 2.0, 3.0, 4.0),
                    $quat::new(-1.0, 0.5, 2.0, 1.0),
                );
                assert_eq!(a.dot(b), 10.0);
                assert_eq!(a.length_sq(), 30.0);
                assert_eq!($quat::new(0.0, 0.0, 3.0, 4.0).length(), 5.0);
                assert_eq!($quat::IDENTITY.length(), 1.0);
            }

            #[test]
            fn normalize_and_its_fallbacks() {
                let n = $quat::new(0.0, 0.0, 3.0, 4.0).normalize();
                assert!(quat_close(n, $quat::new(0.0, 0.0, 0.6, 0.8)));
                let zero = $quat::new(0.0, 0.0, 0.0, 0.0);
                assert!(zero.normalize().w.is_nan());
                assert_eq!(zero.try_normalize(), None);
                assert_eq!(zero.normalize_or_identity(), $quat::IDENTITY);
                let nan = $quat::new(<$scalar>::NAN, 0.0, 0.0, 1.0);
                assert_eq!(nan.try_normalize(), None);
                assert_eq!(nan.normalize_or_identity(), $quat::IDENTITY);
                let infinite = $quat::new(<$scalar>::INFINITY, 0.0, 0.0, 1.0);
                assert_eq!(infinite.try_normalize(), None);
                assert!(quat_close(
                    $quat::new(0.0, 5.0, 0.0, 0.0).try_normalize().unwrap(),
                    $quat::new(0.0, 1.0, 0.0, 0.0)
                ));
            }

            #[test]
            fn component_wise_operators() {
                let (a, b) = (
                    $quat::new(1.0, 2.0, 3.0, 4.0),
                    $quat::new(4.0, 3.0, 2.0, 1.0),
                );
                assert_eq!(a + b, $quat::new(5.0, 5.0, 5.0, 5.0));
                assert_eq!(a - b, $quat::new(-3.0, -1.0, 1.0, 3.0));
                assert_eq!(-a, $quat::new(-1.0, -2.0, -3.0, -4.0));
                assert_eq!(a * 2.0, $quat::new(2.0, 4.0, 6.0, 8.0));
                assert_eq!(2.0 * a, a * 2.0);
                assert_eq!(a / 2.0, $quat::new(0.5, 1.0, 1.5, 2.0));
            }

            #[test]
            fn axis_angle_round_trips() {
                let axis = $v3::new(1.0, 2.0, 2.0) / 3.0;
                let (back_axis, angle) = $quat::from_axis_angle(axis, rad(1.0)).to_axis_angle();
                assert_vec_close!(back_axis, axis);
                assert!(close(angle.value(), 1.0));
                let mut stream = Stream(19);
                for _ in 0..200 {
                    let q = stream.rotation();
                    let (axis, angle) = q.to_axis_angle();
                    assert!((0.0..=PI + TOL).contains(&angle.value()), "{angle:?}");
                    assert!(close(axis.length(), 1.0));
                    assert_same_rotation!($quat::from_axis_angle(axis, angle), q);
                }
            }

            #[test]
            fn to_axis_angle_reports_the_shorter_rotation() {
                // 270 degrees about Z is 90 degrees about -Z.
                let (axis, angle) = $quat::from_rotation_z(Degrees::new(270.0)).to_axis_angle();
                assert_vec_close!(axis, -$v3::Z);
                assert!(close(angle.value(), PI / 2.0));
                // `q` and `-q` describe the same thing.
                let (axis, angle) = (-quarter_turn_z()).to_axis_angle();
                assert_vec_close!(axis, $v3::Z);
                assert!(close(angle.value(), PI / 2.0));
            }

            #[test]
            fn to_axis_angle_of_the_identity_is_x_and_zero() {
                let (axis, angle) = $quat::IDENTITY.to_axis_angle();
                assert_eq!(axis, $v3::X);
                assert_eq!(angle.value(), 0.0);
                let (axis, angle) = (-$quat::IDENTITY).to_axis_angle();
                assert_eq!((axis, angle.value()), ($v3::X, 0.0));
            }

            #[test]
            fn nlerp_hits_the_endpoints_and_takes_the_short_way() {
                let (a, b) = ($quat::IDENTITY, quarter_turn_z());
                assert!(quat_close(a.nlerp(b, 0.0), a));
                assert!(quat_close(a.nlerp(b, 1.0), b));
                // About a shared axis the middle is exactly the half angle.
                assert_same_rotation!(a.nlerp(b, 0.5), $quat::from_rotation_z(rad(PI / 4.0)));
                // `-b` is the same rotation: same path.
                assert_same_rotation!(a.nlerp(-b, 0.5), $quat::from_rotation_z(rad(PI / 4.0)));
                assert!(close(a.nlerp(b, 0.3).length(), 1.0));
            }

            #[test]
            fn slerp_hits_the_endpoints() {
                let (a, b) = (
                    $quat::from_rotation_x(rad(0.3)),
                    $quat::from_rotation_y(rad(1.2)),
                );
                assert_eq!(a.slerp(b, 0.0), a);
                assert_same_rotation!(a.slerp(b, 1.0), b);
            }

            #[test]
            fn slerp_turns_at_constant_speed() {
                let target = $quat::from_rotation_z(Degrees::new(120.0));
                for step in 0_u8..=12 {
                    let t = <$scalar>::from(step) / 12.0;
                    let expected = $quat::from_rotation_z(Degrees::new(120.0 * t));
                    assert_same_rotation!($quat::IDENTITY.slerp(target, t), expected);
                }
                // `nlerp` is not uniform: a third of the way is not 40 degrees.
                let third = $quat::IDENTITY.nlerp(target, 1.0 / 3.0);
                assert!(!same_rotation(
                    third,
                    $quat::from_rotation_z(Degrees::new(40.0))
                ));
            }

            #[test]
            fn slerp_takes_the_shorter_arc() {
                let target = quarter_turn_z();
                assert_same_rotation!(
                    $quat::IDENTITY.slerp(-target, 0.5),
                    $quat::from_rotation_z(rad(PI / 4.0))
                );
                assert_same_rotation!(
                    $quat::IDENTITY.slerp(target, 0.5),
                    $quat::from_rotation_z(rad(PI / 4.0))
                );
            }

            #[test]
            fn slerp_extrapolates_along_the_same_arc() {
                assert_same_rotation!(
                    $quat::IDENTITY.slerp(quarter_turn_z(), 2.0),
                    $quat::from_rotation_z(rad(PI))
                );
                assert_same_rotation!(
                    quarter_turn_z().slerp($quat::IDENTITY, 2.0),
                    $quat::from_rotation_z(rad(-PI / 2.0))
                );
            }

            #[test]
            fn slerp_between_nearly_opposite_rotations_takes_the_shorter_side() {
                // 179 degrees: the short way is counter-clockwise.
                let almost = $quat::from_rotation_z(Degrees::new(179.0));
                assert_same_rotation!(
                    $quat::IDENTITY.slerp(almost, 0.5),
                    $quat::from_rotation_z(Degrees::new(89.5))
                );
                // 181 degrees is -179 the short way, so it turns clockwise.
                let past = $quat::from_rotation_z(Degrees::new(181.0));
                assert_same_rotation!(
                    $quat::IDENTITY.slerp(past, 0.5),
                    $quat::from_rotation_z(Degrees::new(-89.5))
                );
            }

            #[test]
            fn slerp_of_nearly_equal_rotations_is_stable() {
                let a = $quat::from_rotation_y(rad(0.7));
                // Identical: nothing to interpolate, and no division by zero.
                for t in [0.0, 0.3, 1.0] {
                    let result = a.slerp(a, t);
                    assert!(quat_close(result, a), "{result:?}");
                }
                // A tiny angle, below the threshold of the fallback.
                let b = $quat::from_rotation_y(rad(0.7 + 1e-4));
                let middle = a.slerp(b, 0.5);
                assert!(close(middle.length(), 1.0));
                assert_same_rotation!(middle, $quat::from_rotation_y(rad(0.7 + 5e-5)));
                // Antipodal representations of one rotation.
                assert_same_rotation!(a.slerp(-a, 0.4), a);
            }

            #[test]
            fn slerp_results_stay_normalized() {
                let mut stream = Stream(23);
                for _ in 0..200 {
                    let (a, b) = (stream.rotation(), stream.rotation());
                    let t = (stream.sample() + 1.0) / 2.0;
                    assert!(close(a.slerp(b, t).length(), 1.0));
                    assert!(close(a.nlerp(b, t).length(), 1.0));
                }
            }

            #[test]
            fn slerp_with_nan_input_gives_nan() {
                let nan = $quat::new(<$scalar>::NAN, 0.0, 0.0, 1.0);
                assert!($quat::IDENTITY.slerp(nan, 0.5).w.is_nan());
            }

            #[test]
            fn to_mat3_of_known_rotations() {
                assert_eq!($quat::IDENTITY.to_mat3(), $m3::IDENTITY);
                let expected = $m3::from_cols($v3::Y, -$v3::X, $v3::Z);
                assert!(mat_close(quarter_turn_z().to_mat3(), expected));
                let half_turn = $quat::from_rotation_x(rad(PI)).to_mat3();
                assert!(mat_close(
                    half_turn,
                    $m3::from_cols($v3::X, -$v3::Y, -$v3::Z)
                ));
            }

            #[test]
            fn the_matrix_rotates_like_the_quaternion() {
                let mut stream = Stream(29);
                for _ in 0..300 {
                    let (q, v) = (stream.rotation(), stream.vector());
                    assert_vec_close!(q.to_mat3() * v, q * v);
                }
            }

            #[test]
            fn the_matrix_is_a_proper_rotation() {
                let mut stream = Stream(31);
                for _ in 0..100 {
                    let m = stream.rotation().to_mat3();
                    assert!(close(m.determinant(), 1.0));
                    assert!(mat_close(m * m.transpose(), $m3::IDENTITY));
                }
            }

            #[test]
            fn matrix_composition_matches_quaternion_composition() {
                let mut stream = Stream(37);
                for _ in 0..100 {
                    let (a, b) = (stream.rotation(), stream.rotation());
                    assert!(mat_close((a * b).to_mat3(), a.to_mat3() * b.to_mat3()));
                }
            }

            #[test]
            fn to_mat4_embeds_the_rotation_without_translation() {
                let q = $quat::from_axis_angle($v3::new(1.0, 2.0, 2.0) / 3.0, rad(0.9));
                let (m3, m4) = (q.to_mat3(), q.to_mat4());
                for column in 0..3 {
                    assert_eq!(m4.col(column).x, m3.col(column).x);
                    assert_eq!(m4.col(column).y, m3.col(column).y);
                    assert_eq!(m4.col(column).z, m3.col(column).z);
                    assert_eq!(m4.col(column).w, 0.0);
                }
                assert_eq!(m4.w_axis, $v4::W);
                assert_eq!($quat::IDENTITY.to_mat4(), $m4::IDENTITY);
                let v = $v4::new(1.0, 2.0, 3.0, 1.0);
                let rotated = m4 * v;
                assert_vec_close!(
                    $v3::new(rotated.x, rotated.y, rotated.z),
                    q * $v3::new(1.0, 2.0, 3.0)
                );
                assert_eq!(rotated.w, 1.0);
            }

            #[test]
            fn from_mat3_inverts_to_mat3() {
                let mut stream = Stream(41);
                for _ in 0..500 {
                    let q = stream.rotation();
                    let back = $quat::from_mat3(q.to_mat3());
                    assert_same_rotation!(back, q);
                    assert!(close(back.length(), 1.0));
                }
            }

            #[test]
            fn from_mat3_handles_every_branch_and_half_turns() {
                // Half turns about each axis make the trace -1, and each
                // axis picks a different branch of the algorithm.
                for axis in [$v3::X, $v3::Y, $v3::Z] {
                    let q = $quat::from_axis_angle(axis, rad(PI));
                    assert_same_rotation!($quat::from_mat3(q.to_mat3()), q);
                }
                // Rotations just past a half turn, where the sign of the
                // off-diagonal terms flips.
                for axis in [$v3::X, $v3::Y, $v3::Z] {
                    for angle in [PI - 0.01, PI + 0.01, 2.0, 4.0] {
                        let q = $quat::from_axis_angle(axis, rad(angle));
                        assert_same_rotation!($quat::from_mat3(q.to_mat3()), q);
                    }
                }
                assert_eq!($quat::from_mat3($m3::IDENTITY), $quat::IDENTITY);
                assert_same_rotation!(
                    $quat::from_mat3($m3::from_cols($v3::Y, -$v3::X, $v3::Z)),
                    quarter_turn_z()
                );
            }

            #[test]
            fn from_mat3_of_a_product_is_the_product_of_the_rotations() {
                let mut stream = Stream(43);
                for _ in 0..100 {
                    let (a, b) = (stream.rotation(), stream.rotation());
                    let m = a.to_mat3() * b.to_mat3();
                    assert_same_rotation!($quat::from_mat3(m), a * b);
                }
            }

            #[test]
            fn from_mat3_tolerates_slightly_inexact_matrices() {
                let mut m = quarter_turn_z().to_mat3();
                m.x_axis.x += 1e-4;
                m.y_axis.y -= 1e-4;
                let q = $quat::from_mat3(m);
                assert!(close(q.length(), 1.0));
                assert!((q.dot(quarter_turn_z()).abs() - 1.0).abs() < 1e-3);
            }

            #[test]
            fn from_mat4_ignores_the_translation() {
                let q = $quat::from_axis_angle($v3::new(0.0, 0.6, 0.8), rad(2.0));
                let mut rigid = q.to_mat4();
                rigid.w_axis = $v4::new(5.0, 6.0, 7.0, 1.0);
                assert_same_rotation!($quat::from_mat4(rigid), q);
                assert_eq!($quat::from_mat4($m4::IDENTITY), $quat::IDENTITY);
            }

            #[test]
            fn const_operations_work_in_const_contexts() {
                const Q: $quat = $quat::new(0.0, 0.0, 0.6, 0.8);
                const CONJUGATE: $quat = Q.conjugate();
                const INVERSE: $quat = Q.inverse();
                const PRODUCT: $quat = Q.mul_quat(Q);
                const DOT: $scalar = Q.dot(Q);
                const LENGTH_SQ: $scalar = Q.length_sq();
                const XYZ: $v3 = Q.xyz();
                const ROTATED: $v3 = Q.mul_vec3($v3::X);
                const MATRIX: $m3 = Q.to_mat3();
                const MATRIX4: $m4 = Q.to_mat4();
                const ARRAY: [$scalar; 4] = Q.to_array();
                const FROM_ARRAY: $quat = $quat::from_array(ARRAY);
                assert_eq!(CONJUGATE, Q.conjugate());
                assert_eq!(INVERSE, Q.inverse());
                assert_eq!(PRODUCT, Q * Q);
                assert!(close(DOT, 1.0) && close(LENGTH_SQ, 1.0));
                assert_eq!(XYZ, $v3::new(0.0, 0.0, 0.6));
                assert_eq!(ROTATED, Q * $v3::X);
                assert_eq!(MATRIX, Q.to_mat3());
                assert_eq!(MATRIX4, Q.to_mat4());
                assert_eq!(FROM_ARRAY, Q);
            }
        }
    };
}

quat_tests!(quat_f32, f32, Quat, Vec3, Vec4, Mat3, Mat4);
quat_tests!(dquat_f64, f64, DQuat, DVec3, DVec4, DMat3, DMat4);
