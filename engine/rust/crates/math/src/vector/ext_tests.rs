//! Tests for the vector additions of Phase 2.2: element reductions, rounding,
//! `lerp`, resizing (`extend`/`truncate`/`xy`/`xyz`), conversions between the
//! families (`as_dvec3`, ...), approximate comparison, the 2D and 3D helpers,
//! `abs`/`signum`, the integer `rem_euclid`/`div_euclid`/`abs_diff`,
//! `Display`, and the component-wise `*` and `/` between two vectors.
//!
//! Float tests are instantiated once per float family (`Vec`, `DVec`) and the
//! integer ones are written out for `IVec` and `UVec`. Values are chosen so
//! most results are exact; the rest are compared with a small tolerance.

use super::*;
use crate::{Degrees, Radians};

/// Asserts that formatting `$args` gives exactly `$expected`. `format!` needs
/// `alloc`, so this renders through a fixed buffer.
macro_rules! assert_rendered {
    ($expected:expr, $($args:tt)+) => {{
        use core::fmt::Write;
        struct Sink([u8; 128], usize);
        impl Write for Sink {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0[self.1..self.1 + s.len()].copy_from_slice(s.as_bytes());
                self.1 += s.len();
                Ok(())
            }
        }
        let mut sink = Sink([0; 128], 0);
        write!(sink, $($args)+).unwrap();
        assert_eq!(core::str::from_utf8(&sink.0[..sink.1]).unwrap(), $expected);
    }};
}

/// Tests shared by `Vec*` and `DVec*`.
macro_rules! float_tests {
    ($module:ident, $scalar:ty, $v2:ident, $v3:ident, $v4:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-5;
            const PI: $scalar = <$scalar as crate::Scalar>::PI;
            const NAN: $scalar = <$scalar>::NAN;
            const INF: $scalar = <$scalar>::INFINITY;

            fn close(a: $scalar, b: $scalar) -> bool {
                (a - b).abs() <= TOL
            }

            macro_rules! assert_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a.to_array(), $b.to_array());
                    assert!(
                        a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y)),
                        "{:?} is not close to {:?}",
                        a,
                        b
                    );
                }};
            }

            #[test]
            fn element_reductions() {
                let v = $v4::new(3.0, -1.0, 4.0, 2.0);
                assert_eq!(v.min_element(), -1.0);
                assert_eq!(v.max_element(), 4.0);
                assert_eq!(v.element_sum(), 8.0);
                assert_eq!(v.element_product(), -24.0);

                assert_eq!($v2::new(2.0, 5.0).min_element(), 2.0);
                assert_eq!($v2::new(5.0, 2.0).min_element(), 2.0);
                assert_eq!($v2::new(2.0, 5.0).max_element(), 5.0);
                assert_eq!($v2::new(5.0, 2.0).max_element(), 5.0);
                assert_eq!($v3::splat(7.0).element_sum(), 21.0);
                assert_eq!($v3::new(2.0, 3.0, 4.0).element_product(), 24.0);
            }

            #[test]
            fn min_and_max_element_ignore_nan() {
                for v in [
                    $v3::new(NAN, 2.0, 1.0),
                    $v3::new(2.0, NAN, 1.0),
                    $v3::new(2.0, 1.0, NAN),
                ] {
                    assert_eq!(v.min_element(), 1.0, "{v:?}");
                    assert_eq!(v.max_element(), 2.0, "{v:?}");
                }
            }

            #[test]
            fn abs_drops_the_sign() {
                assert_eq!($v3::new(-1.5, 2.0, -0.0).abs(), $v3::new(1.5, 2.0, 0.0));
                // Even negative zero loses its sign.
                assert!($v2::new(-0.0, 3.0).abs().x.is_sign_positive());
            }

            #[test]
            fn signum_is_ieee_and_never_zero() {
                let s = $v4::new(-3.0, 0.0, -0.0, 8.0).signum();
                assert_eq!(s.to_array(), [-1.0, 1.0, -1.0, 1.0]);
                assert!($v2::new(NAN, 1.0).signum().x.is_nan());
            }

            #[test]
            fn rounding_functions() {
                let v = $v4::new(-2.5, -1.2, 1.2, 2.5);
                assert_eq!(v.floor().to_array(), [-3.0, -2.0, 1.0, 2.0]);
                assert_eq!(v.ceil().to_array(), [-2.0, -1.0, 2.0, 3.0]);
                // Ties round away from zero.
                assert_eq!(v.round().to_array(), [-3.0, -1.0, 1.0, 3.0]);
                assert_eq!(v.trunc().to_array(), [-2.0, -1.0, 1.0, 2.0]);
                assert_close!(v.fract(), $v4::new(-0.5, -0.2, 0.2, 0.5));
            }

            #[test]
            fn trunc_plus_fract_is_the_original() {
                for v in [
                    $v3::new(3.75, -3.75, 0.0),
                    $v3::new(-0.25, 100.5, -7.0),
                    $v3::new(1e-3, -1e-3, 12345.678),
                ] {
                    assert_close!(v.trunc() + v.fract(), v);
                }
                assert_eq!($v2::new(-3.75, 3.75).fract().to_array(), [-0.75, 0.75]);
            }

            #[test]
            fn rounding_keeps_integers_and_special_values() {
                let whole = $v3::new(-4.0, 0.0, 9.0);
                assert_eq!(whole.floor(), whole);
                assert_eq!(whole.ceil(), whole);
                assert_eq!(whole.round(), whole);
                assert_eq!(whole.trunc(), whole);
                assert_eq!(whole.fract(), $v3::ZERO);
                assert!($v2::new(NAN, 1.0).floor().x.is_nan());
                assert_eq!($v2::new(INF, -INF).round().to_array(), [INF, -INF]);
            }

            #[test]
            fn lerp_interpolates_and_extrapolates() {
                let a = $v3::new(0.0, 10.0, -4.0);
                let b = $v3::new(10.0, 20.0, 4.0);
                assert_eq!(a.lerp(b, 0.0), a);
                assert_eq!(a.lerp(b, 1.0), b);
                assert_eq!(a.lerp(b, 0.5), $v3::new(5.0, 15.0, 0.0));
                assert_eq!(a.lerp(b, 2.0), $v3::new(20.0, 30.0, 12.0));
                assert_eq!(a.lerp(b, -1.0), $v3::new(-10.0, 0.0, -12.0));
            }

            #[test]
            fn lerp_matches_the_scalar_function_per_component() {
                let a = $v4::new(-1.0, 0.5, 3.0, 100.0);
                let b = $v4::new(7.0, -0.5, 3.0, -50.0);
                for t in [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5] {
                    let v = a.lerp(b, t);
                    for i in 0..4 {
                        assert_eq!(v[i], crate::interp::lerp(a[i], b[i], t), "t = {t}, i = {i}");
                    }
                }
            }

            #[test]
            fn finite_and_nan_checks() {
                assert!($v3::new(1.0, 2.0, 3.0).is_finite());
                assert!(!$v3::new(1.0, INF, 3.0).is_finite());
                assert!(!$v3::new(1.0, 2.0, -INF).is_finite());
                assert!(!$v3::new(NAN, 2.0, 3.0).is_finite());

                assert!($v3::new(1.0, NAN, 3.0).is_nan());
                assert!($v2::new(NAN, NAN).is_nan());
                assert!(!$v3::new(1.0, INF, 3.0).is_nan());
                assert!(!$v4::ONE.is_nan());
            }

            #[test]
            fn approx_eq_uses_the_default_epsilon() {
                let a = $v3::new(1.0, 2.0, 3.0);
                assert!(a.approx_eq(a));
                assert!(a.approx_eq(a + $v3::splat(<$scalar>::EPSILON / 2.0)));
                assert!(!a.approx_eq($v3::new(1.0, 2.0, 3.1)));
                assert!(!a.approx_eq($v3::new(1.1, 2.0, 3.0)));
                assert!(!$v2::new(NAN, 1.0).approx_eq($v2::new(NAN, 1.0)));
            }

            #[test]
            fn approx_eq_eps_is_absolute() {
                let a = $v3::new(1.0, 2.0, 3.0);
                assert!(a.approx_eq_eps($v3::new(1.05, 1.95, 3.0), 0.1));
                assert!(!a.approx_eq_eps($v3::new(1.0, 2.0, 3.2), 0.1));
                assert!(a.approx_eq_eps($v3::new(1.0, 2.0, 3.2), 0.25));
            }

            #[test]
            fn approx_eq_rel_scales_with_magnitude() {
                let a = $v2::new(1000.0, 1.0);
                assert!(a.approx_eq_rel($v2::new(1000.5, 1.0005), 0.001));
                assert!(!a.approx_eq_rel($v2::new(1002.0, 1.0), 0.001));
                assert!(!a.approx_eq_rel($v2::new(1000.0, 1.01), 0.001));
                assert!($v2::ZERO.approx_eq_rel($v2::ZERO, 0.001));
            }

            #[test]
            fn extend_and_truncate() {
                assert_eq!($v2::new(1.0, 2.0).extend(3.0), $v3::new(1.0, 2.0, 3.0));
                assert_eq!(
                    $v3::new(1.0, 2.0, 3.0).extend(4.0),
                    $v4::new(1.0, 2.0, 3.0, 4.0)
                );
                assert_eq!(
                    $v4::new(1.0, 2.0, 3.0, 4.0).truncate(),
                    $v3::new(1.0, 2.0, 3.0)
                );
                assert_eq!($v3::new(1.0, 2.0, 3.0).truncate(), $v2::new(1.0, 2.0));

                let v = $v3::new(-1.0, 5.0, 0.5);
                assert_eq!(v.extend(9.0).truncate(), v);
                let p = $v3::new(1.0, 2.0, 3.0).extend(1.0);
                assert_eq!(p.w, 1.0);
            }

            #[test]
            fn swizzles() {
                let v4 = $v4::new(1.0, 2.0, 3.0, 4.0);
                assert_eq!(v4.xyz(), $v3::new(1.0, 2.0, 3.0));
                assert_eq!(v4.xy(), $v2::new(1.0, 2.0));
                assert_eq!($v3::new(1.0, 2.0, 3.0).xy(), $v2::new(1.0, 2.0));
                assert_eq!(v4.xyz(), v4.truncate());
            }

            #[test]
            fn perp_is_a_quarter_turn() {
                assert_eq!($v2::new(1.0, 2.0).perp(), $v2::new(-2.0, 1.0));
                assert_eq!($v2::X.perp(), $v2::Y);
                assert_eq!($v2::Y.perp(), -$v2::X);
                let v = $v2::new(3.0, -4.0);
                assert_eq!(v.perp().perp(), -v);
                assert_eq!(v.perp().dot(v), 0.0);
                assert_eq!(v.perp().length(), v.length());
            }

            #[test]
            fn perp_dot_is_the_2d_cross_product() {
                assert_eq!($v2::X.perp_dot($v2::Y), 1.0);
                assert_eq!($v2::Y.perp_dot($v2::X), -1.0);
                assert_eq!($v2::new(2.0, 0.0).perp_dot($v2::new(0.0, 3.0)), 6.0);
                assert_eq!($v2::new(1.0, 2.0).perp_dot($v2::new(2.0, 4.0)), 0.0);
                let (a, b) = ($v2::new(1.5, -2.0), $v2::new(0.5, 4.0));
                assert_eq!(a.perp_dot(b), a.perp().dot(b));
                assert_eq!(a.perp_dot(b), -b.perp_dot(a));
            }

            #[test]
            fn from_angle_is_cos_sin() {
                assert_close!($v2::from_angle(Radians::<$scalar>::new(0.0)), $v2::X);
                assert_close!($v2::from_angle(Degrees::<$scalar>::new(90.0)), $v2::Y);
                assert_close!($v2::from_angle(Radians::<$scalar>::new(PI)), -$v2::X);
                assert_close!($v2::from_angle(Degrees::<$scalar>::new(-90.0)), -$v2::Y);
                for degrees in [0.0, 17.0, 90.0, 135.0, 200.0, 359.0, -45.0] {
                    let v = $v2::from_angle(Degrees::<$scalar>::new(degrees));
                    assert!(close(v.length(), 1.0), "{degrees}: {v:?}");
                }
            }

            #[test]
            fn rotate_turns_counter_clockwise() {
                assert_close!($v2::X.rotate(Degrees::<$scalar>::new(90.0)), $v2::Y);
                assert_close!($v2::Y.rotate(Degrees::<$scalar>::new(90.0)), -$v2::X);
                assert_close!($v2::X.rotate(Degrees::<$scalar>::new(-90.0)), -$v2::Y);
                assert_close!(
                    $v2::new(1.0, 2.0).rotate(Radians::<$scalar>::new(PI)),
                    $v2::new(-1.0, -2.0)
                );
                assert_close!(
                    $v2::new(1.0, 2.0).rotate(Radians::<$scalar>::new(0.0)),
                    $v2::new(1.0, 2.0)
                );
            }

            #[test]
            fn rotate_matches_from_angle_and_preserves_length() {
                let angle = Degrees::<$scalar>::new(37.0);
                assert_close!($v2::X.rotate(angle), $v2::from_angle(angle));
                let v = $v2::new(3.0, -4.0);
                assert!(close(v.rotate(angle).length(), 5.0));
                // Rotating back undoes it.
                assert_close!(v.rotate(angle).rotate(Degrees::<$scalar>::new(-37.0)), v);
                // The angle between the two is the rotation angle.
                let turned = v.rotate(angle).angle_between(v).value();
                assert!(
                    close(turned, Radians::<$scalar>::from(angle).value()),
                    "{turned}"
                );
            }

            #[test]
            fn any_orthogonal_is_perpendicular() {
                let samples = [
                    $v3::X,
                    $v3::Y,
                    $v3::Z,
                    -$v3::X,
                    -$v3::Y,
                    -$v3::Z,
                    $v3::new(1.0, 2.0, 3.0),
                    $v3::new(-3.0, 2.0, -1.0),
                    $v3::new(5.0, 5.0, 5.0),
                    $v3::new(0.0, 0.0, 1e-3),
                    $v3::new(1e3, -1e3, 0.5),
                ];
                for v in samples {
                    let o = v.any_orthogonal();
                    assert_eq!(v.dot(o), 0.0, "{v:?} -> {o:?}");
                    assert!(o.length_sq() > 0.0, "{v:?} -> {o:?}");
                    assert!(o.length_sq() <= v.length_sq(), "{v:?} -> {o:?}");
                }
                assert_eq!($v3::ZERO.any_orthogonal(), $v3::ZERO);
            }

            #[test]
            fn any_orthogonal_known_values() {
                assert_eq!(
                    $v3::new(1.0, 2.0, 0.0).any_orthogonal(),
                    $v3::new(-2.0, 1.0, 0.0)
                );
                assert_eq!(
                    $v3::new(0.0, 2.0, 1.0).any_orthogonal(),
                    $v3::new(0.0, -1.0, 2.0)
                );
            }

            #[test]
            fn orthonormal_basis_is_right_handed_and_orthonormal() {
                let directions = [
                    $v3::X,
                    $v3::Y,
                    $v3::Z,
                    -$v3::X,
                    -$v3::Y,
                    -$v3::Z,
                    $v3::new(1.0, 0.0, -0.0),
                    $v3::new(1.0, 2.0, 3.0),
                    $v3::new(-1.0, 2.0, -3.0),
                    $v3::new(0.1, -0.2, -0.97),
                    $v3::new(0.0, 1.0, -1e-4),
                    $v3::new(3.0, -4.0, 1e-4),
                ];
                for direction in directions {
                    let n = direction.normalize();
                    let (t, b) = n.orthonormal_basis();
                    let frame = (n, t, b);
                    assert!(close(t.length(), 1.0), "{frame:?}");
                    assert!(close(b.length(), 1.0), "{frame:?}");
                    assert!(t.dot(n).abs() <= TOL, "{frame:?}");
                    assert!(b.dot(n).abs() <= TOL, "{frame:?}");
                    assert!(t.dot(b).abs() <= TOL, "{frame:?}");
                    assert_close!(t.cross(b), n);
                }
            }

            #[test]
            fn orthonormal_basis_of_the_axes() {
                assert_eq!($v3::Z.orthonormal_basis(), ($v3::X, $v3::Y));
                // `-Z` is the case that breaks the naive constructions.
                let (t, b) = (-$v3::Z).orthonormal_basis();
                assert_close!(t, $v3::X);
                assert_close!(b, -$v3::Y);
                assert_close!(t.cross(b), -$v3::Z);
            }

            #[test]
            fn component_wise_vector_multiplication_and_division() {
                let a = $v3::new(2.0, 4.0, 6.0);
                let b = $v3::new(1.0, 2.0, 3.0);
                assert_eq!(a * b, $v3::new(2.0, 8.0, 18.0));
                assert_eq!(a / b, $v3::splat(2.0));
                assert_eq!(a * $v3::ONE, a);
                assert_eq!(a / $v3::ONE, a);

                let mut m = a;
                m *= b;
                assert_eq!(m, $v3::new(2.0, 8.0, 18.0));
                m /= b;
                assert_eq!(m, a);

                // The scalar forms are unaffected.
                assert_eq!(a * 2.0, $v3::new(4.0, 8.0, 12.0));
                assert_eq!(a / 2.0, b);
                assert_eq!(2.0 * a, a * 2.0);
            }

            #[test]
            fn component_wise_division_by_zero_is_infinite() {
                let v = $v2::new(1.0, -1.0) / $v2::ZERO;
                assert_eq!(v.to_array(), [INF, -INF]);
                assert!(($v2::ZERO / $v2::ZERO).is_nan());
            }

            #[test]
            fn display_formats_as_a_bracketed_list() {
                assert_rendered!("[1.5, -2, 0.25]", "{}", $v3::new(1.5, -2.0, 0.25));
                assert_rendered!("[0, 1]", "{}", $v2::new(0.0, 1.0));
                assert_rendered!("[1, 2, 3, 4]", "{}", $v4::new(1.0, 2.0, 3.0, 4.0));
            }

            #[test]
            fn display_forwards_the_formatter_options_to_every_component() {
                assert_rendered!("[1.00, 2.50]", "{:.2}", $v2::new(1.0, 2.5));
                assert_rendered!("[   1,    2,    3]", "{:4}", $v3::new(1.0, 2.0, 3.0));
                assert_rendered!("[+1, -2]", "{:+}", $v2::new(1.0, -2.0));
            }

            #[test]
            fn const_functions_work_in_const_context() {
                const ABS: $v3 = $v3::new(-1.0, 2.0, -3.0).abs();
                const SIGNUM: $v2 = $v2::new(-4.0, 4.0).signum();
                const LERP: $v3 = $v3::ZERO.lerp($v3::ONE, 0.5);
                const EXTENDED: $v4 = $v3::ONE.extend(2.0);
                const TRUNCATED: $v2 = $v4::ONE.truncate().truncate();
                const SWIZZLE: $v3 = $v4::ONE.xyz();
                const SUM: $scalar = $v4::ONE.element_sum();
                const PRODUCT: $scalar = $v3::splat(2.0).element_product();
                const MIN: $scalar = $v3::new(3.0, 1.0, 2.0).min_element();
                const MAX: $scalar = $v3::new(3.0, 1.0, 2.0).max_element();
                const PERP: $v2 = $v2::X.perp();
                const PERP_DOT: $scalar = $v2::X.perp_dot($v2::Y);
                const ORTHOGONAL: $v3 = $v3::new(0.0, 0.0, 2.0).any_orthogonal();
                const FINITE: bool = $v3::ONE.is_finite();
                const NANS: bool = $v3::ONE.is_nan();
                assert_eq!(ABS, $v3::new(1.0, 2.0, 3.0));
                assert_eq!(SIGNUM, $v2::new(-1.0, 1.0));
                assert_eq!(LERP, $v3::splat(0.5));
                assert_eq!(EXTENDED, $v4::new(1.0, 1.0, 1.0, 2.0));
                assert_eq!(TRUNCATED, $v2::ONE);
                assert_eq!(SWIZZLE, $v3::ONE);
                assert_eq!((SUM, PRODUCT, MIN, MAX), (4.0, 8.0, 1.0, 3.0));
                assert_eq!((PERP, PERP_DOT), ($v2::Y, 1.0));
                assert_eq!(ORTHOGONAL, $v3::new(0.0, -2.0, 0.0));
                assert!(FINITE && !NANS);
            }
        }
    };
}

float_tests!(vec_f32, f32, Vec2, Vec3, Vec4);
float_tests!(dvec_f64, f64, DVec2, DVec3, DVec4);

mod ivec_i32 {
    use super::*;

    #[test]
    fn element_reductions() {
        let v = IVec4::new(3, -1, 4, 2);
        assert_eq!(v.min_element(), -1);
        assert_eq!(v.max_element(), 4);
        assert_eq!(v.element_sum(), 8);
        assert_eq!(v.element_product(), -24);
        assert_eq!(IVec2::new(-2, -5).min_element(), -5);
        assert_eq!(IVec2::new(-5, -2).max_element(), -2);
        assert_eq!(IVec3::new(i32::MAX, 0, i32::MIN).min_element(), i32::MIN);
        assert_eq!(IVec3::new(i32::MAX, 0, i32::MIN).max_element(), i32::MAX);
    }

    #[test]
    fn abs_and_signum() {
        assert_eq!(IVec3::new(-5, 0, 7).abs(), IVec3::new(5, 0, 7));
        assert_eq!(
            IVec4::new(-5, 0, 7, i32::MIN + 1).signum(),
            IVec4::new(-1, 0, 1, -1)
        );
        assert_eq!(
            IVec2::new(i32::MIN + 1, i32::MAX).abs(),
            IVec2::splat(i32::MAX)
        );
    }

    #[test]
    fn rem_euclid_is_never_negative() {
        let v = IVec3::new(-7, 7, -1);
        assert_eq!(v.rem_euclid(IVec3::new(3, 3, 5)), IVec3::new(2, 1, 4));
        // A negative divisor still gives a result in `0..|rhs|`.
        assert_eq!(v.rem_euclid(IVec3::new(-3, -3, -5)), IVec3::new(2, 1, 4));
        // `%` keeps the sign instead.
        assert_eq!(-7 % 3, -1);
    }

    #[test]
    fn div_euclid_rounds_down_for_a_positive_divisor() {
        let v = IVec3::new(-7, 7, -1);
        assert_eq!(v.div_euclid(IVec3::new(3, 3, 5)), IVec3::new(-3, 2, -1));
        assert_eq!(v.div_euclid(IVec3::new(-3, -3, -5)), IVec3::new(3, -2, 1));
    }

    #[test]
    fn euclidean_division_identity() {
        for a in -9..=9 {
            for b in [-4, -3, -1, 1, 2, 5] {
                let (a, b) = (IVec2::splat(a), IVec2::splat(b));
                assert_eq!(a, a.div_euclid(b) * b + a.rem_euclid(b), "{a} / {b}");
                let r = a.rem_euclid(b);
                assert!(r.x >= 0 && r.x < b.x.abs(), "{a} % {b} = {r}");
            }
        }
    }

    #[test]
    #[should_panic]
    fn rem_euclid_by_zero_panics() {
        let _ = IVec2::ONE.rem_euclid(IVec2::new(1, 0));
    }

    #[test]
    #[should_panic]
    fn div_euclid_by_zero_panics() {
        let _ = IVec2::ONE.div_euclid(IVec2::new(0, 1));
    }

    #[test]
    fn abs_diff_returns_the_unsigned_family() {
        let d: UVec2 = IVec2::new(-5, 3).abs_diff(IVec2::new(4, -2));
        assert_eq!(d, UVec2::new(9, 5));
        // Symmetric.
        assert_eq!(IVec2::new(4, -2).abs_diff(IVec2::new(-5, 3)), d);
        // The full range fits in the unsigned type.
        let extreme = IVec2::new(i32::MIN, i32::MAX).abs_diff(IVec2::new(i32::MAX, i32::MIN));
        assert_eq!(extreme, UVec2::splat(u32::MAX));
        assert_eq!(IVec3::ONE.abs_diff(IVec3::ONE), UVec3::ZERO);
        let _: UVec4 = IVec4::ONE.abs_diff(IVec4::ZERO);
    }

    #[test]
    fn extend_truncate_and_swizzles() {
        assert_eq!(IVec2::new(1, 2).extend(3), IVec3::new(1, 2, 3));
        assert_eq!(IVec3::new(1, 2, 3).extend(4), IVec4::new(1, 2, 3, 4));
        assert_eq!(IVec4::new(1, 2, 3, 4).truncate(), IVec3::new(1, 2, 3));
        assert_eq!(IVec3::new(1, 2, 3).truncate(), IVec2::new(1, 2));
        assert_eq!(IVec4::new(1, 2, 3, 4).xyz(), IVec3::new(1, 2, 3));
        assert_eq!(IVec4::new(1, 2, 3, 4).xy(), IVec2::new(1, 2));
        assert_eq!(IVec3::new(1, 2, 3).xy(), IVec2::new(1, 2));
    }

    #[test]
    fn component_wise_vector_multiplication_and_division() {
        let a = IVec3::new(2, -3, 4);
        let b = IVec3::new(5, 6, -7);
        assert_eq!(a * b, IVec3::new(10, -18, -28));
        assert_eq!(IVec3::new(10, -18, -28) / b, a);
        let mut m = a;
        m *= b;
        assert_eq!(m, IVec3::new(10, -18, -28));
        m /= b;
        assert_eq!(m, a);
        assert_eq!(a * 2, IVec3::new(4, -6, 8));
        // Integer division truncates towards zero.
        assert_eq!(IVec2::new(7, -7) / IVec2::splat(2), IVec2::new(3, -3));
    }

    #[test]
    #[should_panic]
    fn component_wise_division_by_a_zero_component_panics() {
        let _ = IVec2::ONE / IVec2::new(1, 0);
    }

    #[test]
    fn display_formats_as_a_bracketed_list() {
        assert_rendered!("[1, -2, 3]", "{}", IVec3::new(1, -2, 3));
        assert_rendered!("[0, 0]", "{}", IVec2::ZERO);
        assert_rendered!("[   1,   -2,    3,    4]", "{:4}", IVec4::new(1, -2, 3, 4));
        assert_rendered!("[001, 002]", "{:03}", IVec2::new(1, 2));
    }

    #[test]
    fn const_functions_work_in_const_context() {
        const ABS: IVec2 = IVec2::new(-1, 2).abs();
        const SIGNUM: IVec2 = IVec2::new(-9, 0).signum();
        const REM: IVec2 = IVec2::new(-1, 5).rem_euclid(IVec2::splat(3));
        const DIV: IVec2 = IVec2::new(-1, 5).div_euclid(IVec2::splat(3));
        const DIFF: UVec2 = IVec2::new(-1, 5).abs_diff(IVec2::new(1, 1));
        const EXTENDED: IVec3 = IVec2::ONE.extend(2);
        const SUM: i32 = IVec4::ONE.element_sum();
        const MIN: i32 = IVec3::new(3, 1, 2).min_element();
        const MAX: i32 = IVec3::new(3, 1, 2).max_element();
        assert_eq!(ABS, IVec2::new(1, 2));
        assert_eq!(SIGNUM, IVec2::new(-1, 0));
        assert_eq!(REM, IVec2::new(2, 2));
        assert_eq!(DIV, IVec2::new(-1, 1));
        assert_eq!(DIFF, UVec2::new(2, 4));
        assert_eq!(EXTENDED, IVec3::new(1, 1, 2));
        assert_eq!((SUM, MIN, MAX), (4, 1, 3));
    }
}

mod uvec_u32 {
    use super::*;

    #[test]
    fn element_reductions() {
        let v = UVec4::new(3, 1, 4, 2);
        assert_eq!(v.min_element(), 1);
        assert_eq!(v.max_element(), 4);
        assert_eq!(v.element_sum(), 10);
        assert_eq!(v.element_product(), 24);
        assert_eq!(UVec2::new(0, u32::MAX).min_element(), 0);
        assert_eq!(UVec2::new(0, u32::MAX).max_element(), u32::MAX);
    }

    #[test]
    fn abs_diff_returns_the_same_type_and_never_underflows() {
        let d: UVec3 = UVec3::new(3, 10, 0).abs_diff(UVec3::new(7, 4, u32::MAX));
        assert_eq!(d, UVec3::new(4, 6, u32::MAX));
        assert_eq!(
            UVec2::new(7, 4).abs_diff(UVec2::new(3, 10)),
            UVec2::new(4, 6)
        );
        assert_eq!(UVec4::ONE.abs_diff(UVec4::ONE), UVec4::ZERO);
    }

    #[test]
    fn rem_euclid_and_div_euclid_match_the_plain_operators() {
        let (a, b) = (UVec3::new(7, 9, 0), UVec3::new(3, 9, 5));
        assert_eq!(a.rem_euclid(b), UVec3::new(1, 0, 0));
        assert_eq!(a.div_euclid(b), UVec3::new(2, 1, 0));
    }

    #[test]
    #[should_panic]
    fn rem_euclid_by_zero_panics() {
        let _ = UVec2::ONE.rem_euclid(UVec2::new(1, 0));
    }

    #[test]
    fn extend_truncate_and_swizzles() {
        assert_eq!(UVec2::new(1, 2).extend(3), UVec3::new(1, 2, 3));
        assert_eq!(UVec3::new(1, 2, 3).extend(4), UVec4::new(1, 2, 3, 4));
        assert_eq!(UVec4::new(1, 2, 3, 4).truncate(), UVec3::new(1, 2, 3));
        assert_eq!(UVec4::new(1, 2, 3, 4).xyz(), UVec3::new(1, 2, 3));
        assert_eq!(UVec3::new(1, 2, 3).xy(), UVec2::new(1, 2));
    }

    #[test]
    fn component_wise_vector_multiplication_and_division() {
        let a = UVec3::new(2, 3, 4);
        let b = UVec3::new(5, 6, 7);
        assert_eq!(a * b, UVec3::new(10, 18, 28));
        assert_eq!(UVec3::new(10, 18, 28) / b, a);
        let mut m = a;
        m *= b;
        m /= UVec3::new(1, 2, 1);
        assert_eq!(m, UVec3::new(10, 9, 28));
        assert_eq!(a * 2, UVec3::new(4, 6, 8));
    }

    #[test]
    fn display_formats_as_a_bracketed_list() {
        assert_rendered!("[1, 2, 3]", "{}", UVec3::new(1, 2, 3));
        assert_rendered!("[4294967295, 0]", "{}", UVec2::new(u32::MAX, 0));
    }

    #[test]
    fn const_functions_work_in_const_context() {
        const DIFF: UVec2 = UVec2::new(1, 9).abs_diff(UVec2::new(4, 2));
        const REM: UVec2 = UVec2::new(7, 9).rem_euclid(UVec2::splat(4));
        const DIV: UVec2 = UVec2::new(7, 9).div_euclid(UVec2::splat(4));
        const TRUNCATED: UVec2 = UVec3::ONE.xy();
        const SUM: u32 = UVec3::splat(2).element_sum();
        const PRODUCT: u32 = UVec3::splat(2).element_product();
        assert_eq!(DIFF, UVec2::new(3, 7));
        assert_eq!(REM, UVec2::new(3, 1));
        assert_eq!(DIV, UVec2::new(1, 2));
        assert_eq!(TRUNCATED, UVec2::ONE);
        assert_eq!((SUM, PRODUCT), (6, 8));
    }
}

mod conversions {
    use super::*;

    #[test]
    fn float_to_float_widens_exactly_and_narrows_with_rounding() {
        assert_eq!(
            Vec3::new(1.5, -2.25, 0.0).as_dvec3(),
            DVec3::new(1.5, -2.25, 0.0)
        );
        assert_eq!(
            DVec3::new(0.5, 2.0, -8.0).as_vec3(),
            Vec3::new(0.5, 2.0, -8.0)
        );
        assert_eq!(
            DVec2::new(0.1, 1e300).as_vec2(),
            Vec2::new(0.1_f32, f32::INFINITY)
        );
    }

    #[test]
    fn float_to_integer_truncates_and_saturates() {
        assert_eq!(Vec3::new(1.9, -2.9, 3.0).as_ivec3(), IVec3::new(1, -2, 3));
        // Negative values saturate at zero for unsigned.
        assert_eq!(Vec3::new(1.9, -2.9, 3.0).as_uvec3(), UVec3::new(1, 0, 3));
        assert_eq!(
            Vec2::new(f32::NAN, 1e20).as_ivec2(),
            IVec2::new(0, i32::MAX)
        );
        assert_eq!(
            Vec2::new(f32::INFINITY, f32::NEG_INFINITY).as_ivec2(),
            IVec2::new(i32::MAX, i32::MIN)
        );
        assert_eq!(DVec2::new(5e9, -1.0).as_uvec2(), UVec2::new(u32::MAX, 0));
        assert_eq!(
            DVec4::new(1.5, 2.5, -3.5, 4.5).as_ivec4(),
            IVec4::new(1, 2, -3, 4)
        );
    }

    #[test]
    fn integer_to_float() {
        assert_eq!(IVec3::new(-1, 2, 3).as_vec3(), Vec3::new(-1.0, 2.0, 3.0));
        assert_eq!(
            IVec2::new(i32::MIN, 7).as_dvec2(),
            DVec2::new(-2147483648.0, 7.0)
        );
        assert_eq!(
            UVec2::new(u32::MAX, 1).as_dvec2(),
            DVec2::new(4294967295.0, 1.0)
        );
        // `f32` cannot hold every `u32`: the nearest one is used.
        assert_eq!(
            UVec2::new(u32::MAX, 1).as_vec2(),
            Vec2::new(4294967296.0, 1.0)
        );
    }

    #[test]
    fn integer_to_integer_wraps() {
        assert_eq!(IVec3::new(-1, 2, 3).as_uvec3(), UVec3::new(u32::MAX, 2, 3));
        assert_eq!(UVec3::new(u32::MAX, 1, 2).as_ivec3(), IVec3::new(-1, 1, 2));
        assert_eq!(
            UVec2::new(3_000_000_000, 5).as_ivec2(),
            IVec2::new(-1_294_967_296, 5)
        );
    }

    #[test]
    fn round_trips_that_are_exact() {
        let v = Vec4::new(1.0, -2.0, 3.0, 4.0);
        assert_eq!(v.as_dvec4().as_vec4(), v);
        assert_eq!(v.as_ivec4().as_vec4(), v);
        let i = IVec4::new(1, -2, 3, 4);
        assert_eq!(i.as_dvec4().as_ivec4(), i);
        assert_eq!(i.as_uvec4().as_ivec4(), i);
    }

    #[test]
    fn every_size_has_the_conversions() {
        assert_eq!(Vec2::ONE.as_dvec2(), DVec2::ONE);
        assert_eq!(Vec3::ONE.as_dvec3(), DVec3::ONE);
        assert_eq!(Vec4::ONE.as_dvec4(), DVec4::ONE);
        assert_eq!(DVec2::ONE.as_ivec2(), IVec2::ONE);
        assert_eq!(IVec3::ONE.as_uvec3(), UVec3::ONE);
        assert_eq!(UVec4::ONE.as_vec4(), Vec4::ONE);
    }

    #[test]
    fn conversions_work_in_const_context() {
        const WIDE: DVec2 = Vec2::new(1.5, 2.5).as_dvec2();
        const INTS: IVec3 = Vec3::new(1.9, -1.9, 0.5).as_ivec3();
        const UNSIGNED: UVec2 = IVec2::new(3, 4).as_uvec2();
        assert_eq!(WIDE, DVec2::new(1.5, 2.5));
        assert_eq!(INTS, IVec3::new(1, -1, 0));
        assert_eq!(UNSIGNED, UVec2::new(3, 4));
    }
}
