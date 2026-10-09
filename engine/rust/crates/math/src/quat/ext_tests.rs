//! Tests for the Phase 2.4 quaternion functions other than the Euler
//! conversions: rotation vectors, arcs, look rotations, angles between
//! rotations, `exp`/`ln`/`pow`, swing-twist and the general helpers.

use super::*;
use crate::angle::Degrees;
use crate::test_support::{text, text_with_precision};

macro_rules! ext_tests {
    ($module:ident, $scalar:ty, $quat:ident, $other:ident, $as_other:ident, $as_back:ident, $v3:ident, $m4:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;
            const PI: $scalar = <$scalar as Scalar>::PI;

            fn rad(value: $scalar) -> Radians<$scalar> {
                Radians::new(value)
            }

            fn same_rotation(a: $quat, b: $quat) -> bool {
                a.angle_between(b).value() <= TOL
            }

            fn vec_close(a: $v3, b: $v3) -> bool {
                (a - b).length() <= TOL
            }

            fn next(state: &mut u64) -> $scalar {
                *state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((*state >> 40) as $scalar) / ((1u64 << 24) as $scalar)
            }

            fn random_direction(state: &mut u64) -> $v3 {
                $v3::new(next(state) - 0.5, next(state) - 0.5, next(state) - 0.5).normalize()
            }

            fn random_rotation(state: &mut u64) -> $quat {
                let axis = random_direction(state);
                $quat::from_axis_angle(axis, rad(next(state) * 2.0 * PI))
            }

            // ---- rotation vectors --------------------------------------

            #[test]
            fn scaled_axis_is_axis_times_angle() {
                let q = $quat::from_scaled_axis($v3::new(0.0, 0.0, PI / 2.0));
                assert!(same_rotation(q, $quat::from_rotation_z(rad(PI / 2.0))));
                let axis = $v3::new(1.0, 2.0, -2.0).normalize();
                let q = $quat::from_scaled_axis(axis * 0.8);
                assert!(same_rotation(q, $quat::from_axis_angle(axis, rad(0.8))));
            }

            #[test]
            fn scaled_axis_zero_and_tiny_vectors() {
                assert_eq!($quat::from_scaled_axis($v3::ZERO), $quat::IDENTITY);
                let tiny = $v3::new(1e-20, 0.0, 0.0);
                let q = $quat::from_scaled_axis(tiny);
                assert!(q.is_finite() && same_rotation(q, $quat::IDENTITY));
                assert_eq!($quat::IDENTITY.to_scaled_axis(), $v3::ZERO);
            }

            #[test]
            fn scaled_axis_round_trips_up_to_a_half_turn() {
                let mut state = 7;
                for _ in 0..100 {
                    let axis = random_direction(&mut state);
                    let v = axis * (next(&mut state) * (PI - 0.01));
                    let back = $quat::from_scaled_axis(v).to_scaled_axis();
                    assert!(vec_close(v, back), "{v} {back}");
                }
            }

            #[test]
            fn to_scaled_axis_takes_the_shorter_way() {
                let q = $quat::from_rotation_z(rad(1.5 * PI));
                let v = q.to_scaled_axis();
                assert!((v.length() - PI / 2.0).abs() < TOL);
                assert!(v.z < 0.0);
                assert!(vec_close((-q).to_scaled_axis(), v));
            }

            // ---- arcs and look rotations -----------------------------------

            #[test]
            fn rotation_arc_turns_from_into_to() {
                let mut state = 99;
                for _ in 0..200 {
                    let (from, to) = (random_direction(&mut state), random_direction(&mut state));
                    let q = $quat::from_rotation_arc(from, to);
                    assert!(q.is_normalized());
                    assert!(vec_close(q * from, to), "{from} {to}");
                }
            }

            #[test]
            fn rotation_arc_is_the_shortest_one() {
                let (from, to) = ($v3::X, $v3::new(1.0, 1.0, 0.0).normalize());
                let q = $quat::from_rotation_arc(from, to);
                assert!((q.angle_between($quat::IDENTITY).value() - PI / 4.0).abs() < TOL);
            }

            #[test]
            fn rotation_arc_of_equal_directions_is_the_identity() {
                assert_eq!($quat::from_rotation_arc($v3::Y, $v3::Y), $quat::IDENTITY);
            }

            #[test]
            fn rotation_arc_of_opposite_directions_is_a_half_turn() {
                for from in [
                    $v3::X,
                    $v3::Y,
                    $v3::Z,
                    $v3::new(1.0, 2.0, 3.0).normalize(),
                    -$v3::Z,
                ] {
                    let q = $quat::from_rotation_arc(from, -from);
                    assert!(q.is_normalized() && q.is_finite());
                    assert!(vec_close(q * from, -from), "{from}");
                    assert!((q.angle_between($quat::IDENTITY).value() - PI).abs() < TOL);
                }
            }

            #[test]
            fn rotation_arc_of_nearly_opposite_directions_stays_finite() {
                let from = $v3::X;
                let to = $v3::new(-1.0, 1e-4, 0.0).normalize();
                let q = $quat::from_rotation_arc(from, to);
                assert!(q.is_finite() && q.is_normalized());
                assert!(vec_close(q * from, to));
            }

            #[test]
            fn look_rotation_rh_looks_down_minus_z() {
                let mut state = 5;
                for _ in 0..100 {
                    let forward = random_direction(&mut state) * 3.0;
                    let up = $v3::Y;
                    if forward.normalize().cross(up).length() < 0.05 {
                        continue;
                    }
                    let q = $quat::look_rotation_rh(forward, up);
                    assert!(q.is_normalized());
                    assert!(vec_close(q * -$v3::Z, forward.normalize()));
                    // The local up axis leans towards the up hint.
                    assert!((q * $v3::Y).dot(up) > 0.0);
                    // ... and is perpendicular to the view direction.
                    assert!((q * $v3::Y).dot(forward.normalize()).abs() < TOL);
                }
            }

            #[test]
            fn look_rotation_lh_looks_down_plus_z() {
                let forward = $v3::new(1.0, 0.5, -2.0);
                let q = $quat::look_rotation_lh(forward, $v3::Y);
                assert!(vec_close(q * $v3::Z, forward.normalize()));
                assert!((q * $v3::Y).dot($v3::Y) > 0.0);
            }

            #[test]
            fn look_rotation_default_forward_is_the_identity() {
                assert!(same_rotation(
                    $quat::look_rotation_rh(-$v3::Z, $v3::Y),
                    $quat::IDENTITY
                ));
                assert!(same_rotation(
                    $quat::look_rotation_lh($v3::Z, $v3::Y),
                    $quat::IDENTITY
                ));
            }

            #[test]
            fn look_rotation_is_the_inverse_of_the_view_matrix() {
                let (forward, up) = ($v3::new(0.3, -0.4, -1.0), $v3::new(0.1, 1.0, 0.0));
                let view = $m4::look_to_rh($v3::ZERO, forward, up);
                let from_quat = $quat::look_rotation_rh(forward, up).conjugate().to_mat4();
                assert!(view.approx_eq_eps(from_quat, TOL));
                let view = $m4::look_to_lh($v3::ZERO, forward, up);
                let from_quat = $quat::look_rotation_lh(forward, up).conjugate().to_mat4();
                assert!(view.approx_eq_eps(from_quat, TOL));
            }

            // ---- angles between rotations --------------------------------

            #[test]
            fn angle_between_known_rotations() {
                let a = $quat::IDENTITY;
                let b = $quat::from_rotation_x(Degrees::new(90.0));
                assert!((a.angle_between(b).value() - PI / 2.0).abs() < TOL);
                assert!((b.angle_between(a).value() - PI / 2.0).abs() < TOL);
                assert!(a.angle_between(a).value().abs() < TOL);
            }

            #[test]
            fn angle_between_q_and_minus_q_is_zero() {
                let q = $quat::from_rotation_y(rad(0.9));
                assert!(q.angle_between(-q).value().abs() < TOL);
            }

            #[test]
            fn angle_between_is_the_shorter_way_around() {
                let q = $quat::from_rotation_z(rad(1.5 * PI));
                assert!((q.angle_between($quat::IDENTITY).value() - PI / 2.0).abs() < TOL);
                let half = $quat::from_rotation_z(rad(PI));
                assert!((half.angle_between($quat::IDENTITY).value() - PI).abs() < TOL);
            }

            #[test]
            fn angle_between_does_not_need_unit_quaternions() {
                let a = $quat::from_rotation_x(rad(0.3)) * 2.5;
                let b = $quat::from_rotation_x(rad(1.1)) * 0.4;
                assert!((a.angle_between(b).value() - 0.8).abs() < TOL);
            }

            #[test]
            fn is_normalized_checks_the_length() {
                assert!($quat::IDENTITY.is_normalized());
                assert!($quat::from_rotation_x(rad(2.0)).is_normalized());
                assert!(!($quat::IDENTITY * 1.01).is_normalized());
                assert!(!$quat::new(0.0, 0.0, 0.0, 0.0).is_normalized());
                assert!(!$quat::new(<$scalar>::NAN, 0.0, 0.0, 1.0).is_normalized());
            }

            #[test]
            fn rotate_towards_moves_by_at_most_the_step() {
                let from = $quat::from_rotation_z(rad(0.2));
                let to = $quat::from_rotation_z(rad(1.4));
                let step = from.rotate_towards(to, rad(0.5));
                assert!((step.angle_between(from).value() - 0.5).abs() < TOL);
                assert!((step.angle_between(to).value() - 0.7).abs() < TOL);
                assert!(same_rotation(step, $quat::from_rotation_z(rad(0.7))));
            }

            #[test]
            fn rotate_towards_does_not_overshoot() {
                let from = $quat::IDENTITY;
                let to = $quat::from_rotation_x(rad(0.4));
                assert_eq!(from.rotate_towards(to, rad(0.4)), to);
                assert_eq!(from.rotate_towards(to, rad(3.0)), to);
                assert_eq!(from.rotate_towards(from, rad(1.0)), from);
            }

            #[test]
            fn rotate_towards_with_zero_or_negative_step_stays_put() {
                let from = $quat::from_rotation_y(rad(0.3));
                let to = $quat::from_rotation_y(rad(1.0));
                assert!(same_rotation(from.rotate_towards(to, rad(0.0)), from));
                assert!(same_rotation(from.rotate_towards(to, rad(-0.5)), from));
            }

            #[test]
            fn rotate_towards_takes_the_shorter_arc() {
                let from = $quat::from_rotation_z(rad(0.1));
                let to = $quat::from_rotation_z(rad(2.0 * PI - 0.1));
                let step = from.rotate_towards(to, rad(0.05));
                assert!(same_rotation(step, $quat::from_rotation_z(rad(0.05))));
            }

            // ---- exp, ln, pow --------------------------------------------

            #[test]
            fn exp_and_ln_of_the_neutral_elements() {
                assert!(same_rotation(
                    $quat::new(0.0, 0.0, 0.0, 0.0).exp(),
                    $quat::IDENTITY
                ));
                let zero = $quat::IDENTITY.ln();
                assert!(zero.approx_eq_eps($quat::new(0.0, 0.0, 0.0, 0.0), TOL));
            }

            #[test]
            fn ln_of_a_unit_rotation_is_half_the_rotation_vector() {
                let axis = $v3::new(1.0, -2.0, 2.0).normalize();
                let q = $quat::from_axis_angle(axis, rad(1.2));
                let ln = q.ln();
                assert!(ln.w.abs() < TOL);
                assert!(vec_close(ln.xyz(), axis * 0.6));
            }

            #[test]
            fn exp_inverts_ln() {
                let mut state = 3;
                for _ in 0..100 {
                    let q = random_rotation(&mut state);
                    assert!(q.ln().exp().approx_eq_eps(q, TOL), "{q}");
                    let scaled = q * (0.3 + 2.0 * next(&mut state));
                    assert!(
                        scaled.ln().exp().approx_eq_eps(scaled, 10.0 * TOL),
                        "{scaled}"
                    );
                }
            }

            #[test]
            fn ln_of_a_negative_real_uses_the_x_axis() {
                let ln = $quat::new(0.0, 0.0, 0.0, -1.0).ln();
                assert!((ln.x - PI).abs() < TOL && ln.y == 0.0 && ln.z == 0.0);
                assert!(ln.w.abs() < TOL);
            }

            #[test]
            fn pow_scales_the_rotation() {
                let q = $quat::from_rotation_z(Degrees::new(40.0));
                assert!(same_rotation(
                    q.pow(0.5),
                    $quat::from_rotation_z(Degrees::new(20.0))
                ));
                assert!(same_rotation(q.pow(2.0), q * q));
                assert!(same_rotation(q.pow(0.0), $quat::IDENTITY));
                assert!(same_rotation(q.pow(1.0), q));
                assert!(same_rotation(q.pow(-1.0), q.conjugate()));
                assert!(same_rotation(q.pow(0.5) * q.pow(0.5), q));
            }

            // ---- swing-twist ---------------------------------------------

            #[test]
            fn swing_times_twist_is_the_rotation() {
                let mut state = 21;
                for _ in 0..100 {
                    let q = random_rotation(&mut state);
                    let axis = random_direction(&mut state);
                    let (swing, twist) = q.to_swing_twist(axis);
                    assert!(swing.is_normalized() && twist.is_normalized());
                    assert!(same_rotation(swing * twist, q));
                    // The twist turns about the axis only, so it leaves it alone.
                    assert!(vec_close(twist * axis, axis));
                }
            }

            #[test]
            fn a_pure_twist_has_no_swing() {
                let q = $quat::from_rotation_y(rad(1.1));
                let (swing, twist) = q.to_swing_twist($v3::Y);
                assert!(same_rotation(swing, $quat::IDENTITY));
                assert!(same_rotation(twist, q));
            }

            #[test]
            fn a_pure_swing_has_no_twist() {
                let q = $quat::from_rotation_x(rad(0.8));
                let (swing, twist) = q.to_swing_twist($v3::Y);
                assert!(same_rotation(twist, $quat::IDENTITY));
                assert!(same_rotation(swing, q));
            }

            #[test]
            fn a_half_turn_swing_has_an_undefined_twist() {
                let q = $quat::from_rotation_x(rad(PI));
                let (swing, twist) = q.to_swing_twist($v3::Y);
                assert!(twist.is_finite() && same_rotation(twist, $quat::IDENTITY));
                assert!(same_rotation(swing * twist, q));
            }

            // ---- general helpers -----------------------------------------

            #[test]
            fn finite_and_nan() {
                assert!($quat::IDENTITY.is_finite() && !$quat::IDENTITY.is_nan());
                assert!(!$quat::new(<$scalar>::INFINITY, 0.0, 0.0, 1.0).is_finite());
                assert!($quat::new(0.0, <$scalar>::NAN, 0.0, 1.0).is_nan());
                assert!(!$quat::new(0.0, <$scalar>::NAN, 0.0, 1.0).is_finite());
            }

            #[test]
            fn approximate_equality_compares_components() {
                let q = $quat::from_rotation_x(rad(0.5));
                assert!(q.approx_eq(q));
                assert!(q.approx_eq_eps(q * (1.0 + 1e-6), 1e-4));
                assert!(q.approx_eq_rel(q * (1.0 + 1e-6), 1e-4));
                assert!(!q.approx_eq_eps($quat::IDENTITY, 1e-4));
                assert!(!q.approx_eq(-q));
                assert!(!$quat::new(<$scalar>::NAN, 0.0, 0.0, 1.0).approx_eq($quat::IDENTITY));
            }

            #[test]
            fn display_lists_the_components() {
                assert_eq!(text(&$quat::IDENTITY).as_str(), "[0, 0, 0, 1]");
                assert_eq!(
                    text(&$quat::new(1.0, 2.5, -3.0, 0.0)).as_str(),
                    "[1, 2.5, -3, 0]"
                );
                assert_eq!(
                    text_with_precision(&$quat::new(1.0, 0.5, 0.25, 0.0), 2).as_str(),
                    "[1.00, 0.50, 0.25, 0.00]"
                );
            }

            #[test]
            fn conversion_between_the_families() {
                let q = $quat::new(0.5, -0.25, 0.75, 1.0);
                let other: $other = q.$as_other();
                assert_eq!(other.to_array(), [0.5, -0.25, 0.75, 1.0]);
                assert_eq!(other.$as_back(), q);
            }
        }
    };
}

ext_tests!(quat_f32, f32, Quat, DQuat, as_dquat, as_quat, Vec3, Mat4);
ext_tests!(quat_f64, f64, DQuat, Quat, as_quat, as_dquat, DVec3, DMat4);
