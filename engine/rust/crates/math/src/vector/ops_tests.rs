//! Tests for the vector operations of Phase 1.8 (`dot`, `cross`, `length`,
//! `normalize`, projections, `reflect`, `refract`, `angle_between`,
//! component-wise `min`/`max`/`clamp`).
//!
//! Float tests are instantiated once per float family (`Vec`, `DVec`) and
//! integer tests once per integer family (`IVec`, `UVec`), each in its own
//! module. Values are chosen so most results are exact; the rest are compared
//! with a small tolerance.

use super::*;
use crate::Scalar;

/// Tests shared by `Vec*` and `DVec*`.
macro_rules! float_tests {
    ($module:ident, $scalar:ty, $v2:ident, $v3:ident, $v4:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-5;
            const PI: $scalar = <$scalar as Scalar>::PI;

            fn close(a: $scalar, b: $scalar) -> bool {
                (a - b).abs() <= TOL
            }

            fn close_all<const N: usize>(a: [$scalar; N], b: [$scalar; N]) -> bool {
                a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y))
            }

            macro_rules! assert_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a, $b);
                    assert!(
                        close_all(a.to_array(), b.to_array()),
                        "{:?} is not close to {:?}",
                        a,
                        b
                    );
                }};
            }

            #[test]
            fn dot_sums_the_component_products() {
                assert_eq!($v2::new(1.0, 2.0).dot($v2::new(3.0, 4.0)), 11.0);
                assert_eq!($v3::new(1.0, 2.0, 3.0).dot($v3::new(4.0, -5.0, 6.0)), 12.0);
                assert_eq!($v4::new(1.0, 2.0, 3.0, 4.0).dot($v4::ONE), 10.0);
                assert_eq!($v3::X.dot($v3::Y), 0.0);
            }

            #[test]
            fn dot_is_commutative() {
                let (a, b) = ($v3::new(1.5, -2.0, 3.0), $v3::new(-4.0, 0.5, 2.0));
                assert_eq!(a.dot(b), b.dot(a));
            }

            #[test]
            fn length_and_length_sq() {
                assert_eq!($v2::new(3.0, 4.0).length_sq(), 25.0);
                assert_eq!($v2::new(3.0, 4.0).length(), 5.0);
                assert_eq!($v3::new(2.0, 3.0, 6.0).length_sq(), 49.0);
                assert_eq!($v3::new(2.0, 3.0, 6.0).length(), 7.0);
                assert_eq!($v4::ONE.length_sq(), 4.0);
                assert_eq!($v4::ONE.length(), 2.0);
                assert_eq!($v3::ZERO.length(), 0.0);
            }

            #[test]
            fn distance_and_distance_sq() {
                let (a, b) = ($v2::new(1.0, 1.0), $v2::new(4.0, 5.0));
                assert_eq!(a.distance_sq(b), 25.0);
                assert_eq!(a.distance(b), 5.0);
                assert_eq!(b.distance(a), 5.0);
                assert_eq!($v3::ONE.distance($v3::ONE), 0.0);
                assert_eq!($v4::ZERO.distance_sq($v4::ONE), 4.0);
            }

            #[test]
            fn normalize_gives_unit_length_in_the_same_direction() {
                assert_close!($v2::new(3.0, 4.0).normalize(), $v2::new(0.6, 0.8));
                assert_close!($v3::new(0.0, 0.0, 5.0).normalize(), $v3::Z);
                assert_close!($v3::new(-2.0, 0.0, 0.0).normalize(), -$v3::X);
                let n = $v4::new(1.0, 2.0, 3.0, 4.0).normalize();
                assert!(close(n.length(), 1.0), "{n:?}");
            }

            #[test]
            fn normalize_of_zero_is_nan() {
                let n = $v3::ZERO.normalize();
                assert!(n.x.is_nan() && n.y.is_nan() && n.z.is_nan(), "{n:?}");
            }

            #[test]
            fn try_normalize_rejects_vectors_without_a_direction() {
                assert_eq!($v3::ZERO.try_normalize(), None);
                assert_eq!($v2::new(<$scalar>::INFINITY, 0.0).try_normalize(), None);
                assert_eq!($v2::new(<$scalar>::NAN, 1.0).try_normalize(), None);
                assert_eq!($v3::new(0.0, 3.0, 0.0).try_normalize(), Some($v3::Y));
                assert_close!(
                    $v4::new(0.0, 0.0, 0.0, -9.0).try_normalize().unwrap(),
                    -$v4::W
                );
            }

            #[test]
            fn normalize_or_zero_falls_back_to_zero() {
                assert_eq!($v3::ZERO.normalize_or_zero(), $v3::ZERO);
                assert_eq!($v2::new(<$scalar>::NAN, 0.0).normalize_or_zero(), $v2::ZERO);
                assert_eq!($v3::X.normalize_or_zero(), $v3::X);
                assert_close!($v2::new(0.0, 7.0).normalize_or_zero(), $v2::Y);
            }

            #[test]
            fn projection_and_rejection_split_a_vector() {
                let v = $v3::new(3.0, 4.0, 0.0);
                // `onto` need not be normalized.
                let onto = $v3::new(2.0, 0.0, 0.0);
                assert_eq!(v.project_onto(onto), $v3::new(3.0, 0.0, 0.0));
                assert_eq!(v.reject_from(onto), $v3::new(0.0, 4.0, 0.0));
            }

            #[test]
            fn projection_plus_rejection_is_the_original_and_rejection_is_perpendicular() {
                let v = $v3::new(1.0, 2.0, 3.0);
                let w = $v3::new(4.0, -1.0, 2.0);
                assert_close!(v.project_onto(w) + v.reject_from(w), v);
                assert!(close(v.reject_from(w).dot(w), 0.0));
                // The projection is parallel to `w`: it has no part off `w`.
                assert!(close(v.project_onto(w).reject_from(w).length(), 0.0));
                // Works in 2 and 4 dimensions too.
                let (a, b) = ($v2::new(2.0, 2.0), $v2::new(0.0, 5.0));
                assert_eq!(a.project_onto(b), $v2::new(0.0, 2.0));
                assert_eq!(a.reject_from(b), $v2::new(2.0, 0.0));
                let c = $v4::new(1.0, 2.0, 3.0, 4.0);
                assert_eq!(c.project_onto($v4::W), $v4::new(0.0, 0.0, 0.0, 4.0));
            }

            #[test]
            fn projecting_onto_zero_is_nan() {
                let p = $v2::ONE.project_onto($v2::ZERO);
                assert!(p.x.is_nan() && p.y.is_nan());
                let r = $v2::ONE.reject_from($v2::ZERO);
                assert!(r.x.is_nan() && r.y.is_nan());
            }

            #[test]
            fn reflect_mirrors_about_the_normal() {
                assert_eq!(
                    $v3::new(1.0, -1.0, 0.0).reflect($v3::Y),
                    $v3::new(1.0, 1.0, 0.0)
                );
                assert_eq!(
                    $v3::new(0.0, -2.0, 0.0).reflect($v3::Y),
                    $v3::new(0.0, 2.0, 0.0)
                );
                // Moving along the surface is unchanged.
                assert_eq!(
                    $v3::new(5.0, 0.0, -3.0).reflect($v3::Y),
                    $v3::new(5.0, 0.0, -3.0)
                );
                assert_eq!(
                    $v4::new(1.0, -1.0, 0.0, 5.0).reflect($v4::Y),
                    $v4::new(1.0, 1.0, 0.0, 5.0)
                );
                assert_eq!($v2::new(2.0, -3.0).reflect($v2::Y), $v2::new(2.0, 3.0));
            }

            #[test]
            fn reflect_keeps_the_length_and_is_its_own_inverse() {
                let n = $v3::new(0.6, 0.8, 0.0);
                let v = $v3::new(1.0, 2.0, 3.0);
                let r = v.reflect(n);
                assert!(close(r.length(), v.length()));
                assert_close!(r.reflect(n), v);
            }

            #[test]
            fn refract_with_equal_indices_does_not_bend_the_ray() {
                let incident = $v3::new(0.6, -0.8, 0.0);
                assert_close!(incident.refract($v3::Y, 1.0).unwrap(), incident);
                let incident = $v2::new(0.6, -0.8);
                assert_close!(incident.refract($v2::Y, 1.0).unwrap(), incident);
                let incident = $v4::new(0.6, -0.8, 0.0, 0.0);
                assert_close!(incident.refract($v4::Y, 1.0).unwrap(), incident);
            }

            #[test]
            fn refract_straight_through_keeps_the_direction() {
                assert_close!((-$v3::Y).refract($v3::Y, 0.5).unwrap(), -$v3::Y);
            }

            #[test]
            fn refract_obeys_snells_law() {
                // Air (1.0) into glass (1.5), 45 degrees from the normal.
                let h = <$scalar as Scalar>::sqrt(0.5);
                let eta = 1.0 / 1.5;
                let refracted = $v3::new(h, -h, 0.0).refract($v3::Y, eta).unwrap();
                assert!(close(refracted.length(), 1.0), "{refracted:?}");
                // sin(theta_t) = eta * sin(theta_i), and it bends toward the normal.
                assert!(close(refracted.x, eta * h), "{refracted:?}");
                assert!(refracted.y < 0.0 && refracted.x < h);
            }

            #[test]
            fn refract_is_none_on_total_internal_reflection() {
                // Glass (1.5) into air (1.0), 60 degrees from the normal.
                let incident = $v3::new(0.866_025_4, -0.5, 0.0);
                assert_eq!(incident.refract($v3::Y, 1.5), None);
                // Below the critical angle the same surface does refract.
                let shallow = $v3::new(0.5, -0.866_025_4, 0.0);
                assert!(shallow.refract($v3::Y, 1.5).is_some());
            }

            #[test]
            fn angle_between_known_angles() {
                assert!(close($v2::X.angle_between($v2::Y).value(), PI / 2.0));
                assert!(close($v3::X.angle_between($v3::Z).value(), PI / 2.0));
                assert!(close($v4::W.angle_between($v4::X).value(), PI / 2.0));
                assert_eq!($v3::X.angle_between($v3::X).value(), 0.0);
                assert!(close($v3::X.angle_between(-$v3::X).value(), PI));
                assert!(close(
                    $v2::X.angle_between($v2::new(1.0, 1.0)).value(),
                    PI / 4.0
                ));
            }

            #[test]
            fn angle_between_ignores_length_and_is_symmetric() {
                let (a, b) = ($v2::new(5.0, 0.0), $v2::new(0.0, 3.0));
                assert!(close(a.angle_between(b).value(), PI / 2.0));
                let (c, d) = ($v3::new(1.0, 2.0, 3.0), $v3::new(-2.0, 1.0, 4.0));
                assert!(close(
                    c.angle_between(d).value(),
                    d.angle_between(c).value()
                ));
            }

            #[test]
            fn angle_between_is_precise_for_tiny_angles() {
                // `acos(dot)` would round this to either 0 or ~4.9e-4.
                let angle = $v2::X.angle_between($v2::new(1.0, 1e-4)).value();
                assert!((angle - 1e-4).abs() < 1e-8, "{angle}");
                let nearly_opposite = $v2::X.angle_between($v2::new(-1.0, 1e-4)).value();
                assert!(
                    (PI - nearly_opposite - 1e-4).abs() < 1e-6,
                    "{nearly_opposite}"
                );
            }

            #[test]
            fn angle_between_with_a_zero_vector_is_nan() {
                assert!($v3::ZERO.angle_between($v3::X).value().is_nan());
                assert!($v3::X.angle_between($v3::ZERO).value().is_nan());
            }

            #[test]
            fn min_max_clamp_are_component_wise() {
                let (a, b) = ($v3::new(1.0, 5.0, 3.0), $v3::new(2.0, 4.0, 6.0));
                assert_eq!(a.min(b), $v3::new(1.0, 4.0, 3.0));
                assert_eq!(a.max(b), $v3::new(2.0, 5.0, 6.0));
                assert_eq!(
                    $v3::new(-5.0, 0.5, 5.0).clamp($v3::ZERO, $v3::ONE),
                    $v3::new(0.0, 0.5, 1.0)
                );
                assert_eq!(
                    $v2::new(9.0, -9.0).clamp($v2::splat(-1.0), $v2::splat(1.0)),
                    $v2::new(1.0, -1.0)
                );
                assert_eq!($v4::ONE.min($v4::ZERO), $v4::ZERO);
                assert_eq!($v4::ONE.max($v4::ZERO), $v4::ONE);
            }

            #[test]
            fn min_and_max_ignore_nan_like_the_scalar_functions() {
                let with_nan = $v2::new(<$scalar>::NAN, 1.0);
                let other = $v2::new(2.0, 0.0);
                assert_eq!(with_nan.min(other), $v2::new(2.0, 0.0));
                assert_eq!(with_nan.max(other), $v2::new(2.0, 1.0));
            }

            #[test]
            #[should_panic]
            fn clamp_panics_when_lo_exceeds_hi() {
                let _ = $v2::ONE.clamp($v2::splat(2.0), $v2::splat(1.0));
            }

            #[test]
            fn cross_follows_the_right_hand_rule() {
                assert_eq!($v3::X.cross($v3::Y), $v3::Z);
                assert_eq!($v3::Y.cross($v3::Z), $v3::X);
                assert_eq!($v3::Z.cross($v3::X), $v3::Y);
                let (a, b) = ($v3::new(1.0, 2.0, 3.0), $v3::new(4.0, 5.0, 6.0));
                assert_eq!(a.cross(b), $v3::new(-3.0, 6.0, -3.0));
            }

            #[test]
            fn cross_is_anticommutative_perpendicular_and_zero_when_parallel() {
                let (a, b) = ($v3::new(1.0, 2.0, 3.0), $v3::new(4.0, 5.0, 6.0));
                let c = a.cross(b);
                assert_eq!(c, -b.cross(a));
                assert_eq!(c.dot(a), 0.0);
                assert_eq!(c.dot(b), 0.0);
                assert_eq!(a.cross(a), $v3::ZERO);
                assert_eq!(a.cross(a * 2.0), $v3::ZERO);
                // |a x b|^2 = |a|^2 |b|^2 - (a . b)^2
                assert_eq!(
                    c.length_sq(),
                    a.length_sq() * b.length_sq() - a.dot(b) * a.dot(b)
                );
            }

            #[test]
            fn const_operations_work_in_const_contexts() {
                const A: $v3 = $v3::new(1.0, 2.0, 3.0);
                const B: $v3 = $v3::new(4.0, 5.0, 6.0);
                const DOT: $scalar = A.dot(B);
                const LENGTH_SQ: $scalar = A.length_sq();
                const DISTANCE_SQ: $scalar = A.distance_sq(B);
                const CROSS: $v3 = A.cross(B);
                const MIN: $v3 = A.min(B);
                const MAX: $v3 = A.max(B);
                const CLAMP: $v3 = B.clamp(A, A);
                assert_eq!(DOT, 32.0);
                assert_eq!(LENGTH_SQ, 14.0);
                assert_eq!(DISTANCE_SQ, 27.0);
                assert_eq!(CROSS, $v3::new(-3.0, 6.0, -3.0));
                assert_eq!((MIN, MAX), (A, B));
                assert_eq!(CLAMP, A);
            }
        }
    };
}

float_tests!(vec_f32, f32, Vec2, Vec3, Vec4);
float_tests!(dvec_f64, f64, DVec2, DVec3, DVec4);

/// Tests shared by `IVec*` and `UVec*`, using non-negative values only.
macro_rules! integer_tests {
    ($module:ident, $scalar:ty, $v2:ident, $v3:ident, $v4:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn dot_sums_the_component_products() {
                assert_eq!($v2::new(1, 2).dot($v2::new(3, 4)), 11);
                assert_eq!($v3::new(1, 2, 3).dot($v3::new(4, 5, 6)), 32);
                assert_eq!($v4::new(1, 2, 3, 4).dot($v4::ONE), 10);
                assert_eq!($v3::X.dot($v3::Y), 0);
            }

            #[test]
            fn length_sq_is_the_dot_product_with_itself() {
                assert_eq!($v2::new(3, 4).length_sq(), 25);
                assert_eq!($v3::new(2, 3, 6).length_sq(), 49);
                assert_eq!($v4::ONE.length_sq(), 4);
                assert_eq!($v3::ZERO.length_sq(), 0);
            }

            #[test]
            fn distance_sq_is_symmetric_and_never_underflows() {
                let (a, b) = ($v2::new(1, 1), $v2::new(4, 5));
                assert_eq!(a.distance_sq(b), 25);
                assert_eq!(b.distance_sq(a), 25);
                // Mixed directions: one component grows, the other shrinks.
                assert_eq!($v2::new(5, 1).distance_sq($v2::new(1, 4)), 25);
                assert_eq!($v2::new(1, 4).distance_sq($v2::new(5, 1)), 25);
                assert_eq!($v3::ONE.distance_sq($v3::ONE), 0);
                assert_eq!($v4::ZERO.distance_sq($v4::ONE), 4);
            }

            #[test]
            fn min_max_clamp_are_component_wise() {
                let (a, b) = ($v3::new(1, 5, 3), $v3::new(2, 4, 6));
                assert_eq!(a.min(b), $v3::new(1, 4, 3));
                assert_eq!(a.max(b), $v3::new(2, 5, 6));
                assert_eq!(
                    $v3::new(0, 5, 9).clamp($v3::splat(1), $v3::splat(6)),
                    $v3::new(1, 5, 6)
                );
                assert_eq!(
                    $v2::new(3, 3).clamp($v2::new(3, 4), $v2::new(3, 4)),
                    $v2::new(3, 4)
                );
                assert_eq!($v4::ONE.min($v4::ZERO), $v4::ZERO);
                assert_eq!($v4::ONE.max($v4::ZERO), $v4::ONE);
            }

            #[test]
            #[should_panic(expected = "clamp: every component of `lo`")]
            fn clamp_panics_when_lo_exceeds_hi() {
                let _ = $v2::ONE.clamp($v2::splat(2), $v2::splat(1));
            }

            #[test]
            #[should_panic(expected = "clamp: every component of `lo`")]
            fn clamp_panics_when_only_one_component_is_inverted() {
                let _ = $v3::ONE.clamp($v3::new(0, 0, 5), $v3::new(9, 9, 4));
            }

            #[cfg(debug_assertions)]
            #[test]
            #[should_panic(expected = "attempt to multiply with overflow")]
            fn dot_overflow_panics_in_debug_builds() {
                let _ = $v2::splat(<$scalar>::MAX).dot($v2::splat(2));
            }

            #[test]
            fn const_operations_work_in_const_contexts() {
                const A: $v3 = $v3::new(1, 2, 3);
                const B: $v3 = $v3::new(4, 5, 6);
                const DOT: $scalar = A.dot(B);
                const LENGTH_SQ: $scalar = A.length_sq();
                const DISTANCE_SQ: $scalar = A.distance_sq(B);
                const MIN: $v3 = A.min(B);
                const MAX: $v3 = A.max(B);
                const CLAMP: $v3 = B.clamp(A, A);
                assert_eq!((DOT, LENGTH_SQ, DISTANCE_SQ), (32, 14, 27));
                assert_eq!((MIN, MAX), (A, B));
                assert_eq!(CLAMP, A);
            }
        }
    };
}

integer_tests!(ivec_i32, i32, IVec2, IVec3, IVec4);
integer_tests!(uvec_u32, u32, UVec2, UVec3, UVec4);

/// Behaviour that only exists for the signed integer vectors.
mod signed_only {
    use super::*;

    #[test]
    fn operations_handle_negative_components() {
        assert_eq!(IVec3::new(1, 2, 3).dot(IVec3::new(4, -5, 6)), 12);
        assert_eq!(IVec2::new(-3, -4).length_sq(), 25);
        assert_eq!(IVec2::new(-3, 4).distance_sq(IVec2::new(3, -4)), 100);
        assert_eq!(
            IVec3::new(-1, 5, -3).min(IVec3::new(2, -4, -6)),
            IVec3::new(-1, -4, -6)
        );
        assert_eq!(
            IVec3::new(-1, 5, -3).max(IVec3::new(2, -4, -6)),
            IVec3::new(2, 5, -3)
        );
        assert_eq!(
            IVec3::new(-9, 0, 9).clamp(IVec3::splat(-2), IVec3::splat(2)),
            IVec3::new(-2, 0, 2)
        );
    }

    #[test]
    fn cross_follows_the_right_hand_rule() {
        assert_eq!(IVec3::X.cross(IVec3::Y), IVec3::Z);
        assert_eq!(IVec3::Y.cross(IVec3::Z), IVec3::X);
        assert_eq!(IVec3::Z.cross(IVec3::X), IVec3::Y);
        let (a, b) = (IVec3::new(1, 2, 3), IVec3::new(4, 5, 6));
        assert_eq!(a.cross(b), IVec3::new(-3, 6, -3));
        assert_eq!(a.cross(b), -b.cross(a));
        assert_eq!(a.cross(b).dot(a), 0);
        assert_eq!(a.cross(a), IVec3::ZERO);
    }

    #[test]
    fn const_cross_works_in_const_contexts() {
        const CROSS: IVec3 = IVec3::new(1, 2, 3).cross(IVec3::new(4, 5, 6));
        assert_eq!(CROSS, IVec3::new(-3, 6, -3));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn distance_sq_overflow_panics_in_debug_builds() {
        let _ = IVec2::new(i32::MIN, 0).distance_sq(IVec2::new(i32::MAX, 0));
    }
}
