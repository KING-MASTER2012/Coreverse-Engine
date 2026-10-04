//! Trigonometry, roots, powers and logarithms: [`sin`], [`cos`], [`tan`],
//! [`cot`], [`asin`], [`acos`], [`atan`], [`atan2`], [`sqrt`], [`cbrt`],
//! [`nth_root`], [`hypot`], [`pow`], [`exp`], [`ln`], [`log2`] and
//! [`log10`].
//!
//! Everything is generic over [`Scalar`], so it works for `f32` and `f64`.
//! The functions are backed by `std` by default and by the `libm` crate
//! under the `no_std` feature (see the crate-level docs); the results agree
//! to within normal floating-point error but are not guaranteed bit-for-bit
//! identical between the two backends.
//!
//! # Typed angles
//!
//! The angle-taking functions accept anything convertible into
//! [`Radians`], so both angle types work directly, and the inverse
//! functions return [`Radians`] rather than a bare number:
//!
//! ```
//! use math::angle::{Degrees, Radians};
//! use math::trig::{acos, sin};
//!
//! let a = sin(Degrees(30.0_f64));
//! assert!((a - 0.5).abs() < 1e-12);
//!
//! let b = sin(Radians(core::f64::consts::FRAC_PI_2));
//! assert!((b - 1.0).abs() < 1e-12);
//!
//! let angle = acos(0.5_f64); // Radians<f64>
//! assert!((Degrees::from(angle).0 - 60.0).abs() < 1e-9);
//! ```
//!
//! For code that already works in raw radians, [`raw`] has the same
//! functions taking and returning bare numbers.
//!
//! # Out-of-domain input
//!
//! Nothing here panics. Input with no real answer returns `NaN`, matching
//! `f32`/`f64`: [`asin`]/[`acos`] outside `[-1, 1]`, [`sqrt`] of a negative
//! number, and [`nth_root`] of a negative number for an even degree. The
//! logarithms are `NaN` for negative input and `-inf` for zero.
//! [`cot`] is `±inf` where `sin` is zero.

use crate::Scalar;
use crate::angle::Radians;

/// The sine of `angle`.
#[inline]
pub fn sin<T: Scalar>(angle: impl Into<Radians<T>>) -> T {
    raw::sin(angle.into().0)
}

/// The cosine of `angle`.
#[inline]
pub fn cos<T: Scalar>(angle: impl Into<Radians<T>>) -> T {
    raw::cos(angle.into().0)
}

/// The tangent of `angle`.
///
/// Grows without bound near odd multiples of a quarter turn, but since
/// π is not exactly representable it never returns `±inf` there.
#[inline]
pub fn tan<T: Scalar>(angle: impl Into<Radians<T>>) -> T {
    raw::tan(angle.into().0)
}

/// The cotangent of `angle`, `cos / sin`.
///
/// `±inf` where the sine is exactly zero (at `0`, and `-0.0` gives `-inf`).
/// Computed as `cos / sin` rather than `1 / tan`, which is more accurate
/// near multiples of a half turn.
#[inline]
pub fn cot<T: Scalar>(angle: impl Into<Radians<T>>) -> T {
    raw::cot(angle.into().0)
}

/// The arcsine of `x`, as an angle in `[-π/2, π/2]`. `NaN` if `x` is
/// outside `[-1, 1]`.
#[inline]
pub fn asin<T: Scalar>(x: T) -> Radians<T> {
    Radians(raw::asin(x))
}

/// The arccosine of `x`, as an angle in `[0, π]`. `NaN` if `x` is outside
/// `[-1, 1]`.
#[inline]
pub fn acos<T: Scalar>(x: T) -> Radians<T> {
    Radians(raw::acos(x))
}

/// The four-quadrant arctangent: the angle of the point `(x, y)` measured
/// counterclockwise from the positive `x` axis, in `[-π, π]`.
///
/// Note the argument order: `y` first, like `f32::atan2` and the C
/// function of the same name. `atan2(0, 0)` is `0`.
#[inline]
pub fn atan2<T: Scalar>(y: T, x: T) -> Radians<T> {
    Radians(raw::atan2(y, x))
}

/// The arctangent of `x`, as an angle in `[-π/2, π/2]`.
///
/// Prefer [`atan2`] when you have both coordinates: it knows the quadrant,
/// and has no division that could be by zero.
#[inline]
pub fn atan<T: Scalar>(x: T) -> Radians<T> {
    Radians(raw::atan(x))
}

/// The square root of `x`. `NaN` if `x` is negative.
#[inline]
pub fn sqrt<T: Scalar>(x: T) -> T {
    x.sqrt()
}

/// `x` raised to the power `exp`.
///
/// Follows `f32::powf`: `pow(x, 0)` is `1` for any `x`, including `NaN`,
/// and a negative `x` with a non-integer `exp` is `NaN`.
#[inline]
pub fn pow<T: Scalar>(x: T, exp: T) -> T {
    x.powf(exp)
}

/// The `n`-th root of `x`, so that `pow(nth_root(x, n), n) == x` up to
/// rounding.
///
/// `n` is a `T` rather than an integer so this needs no numeric
/// conversions, but it is meant to be a whole number:
/// - `n == 0` is `NaN` (there is no zeroth root).
/// - A negative `x` has a real root only for an *odd* whole `n`, and that
///   root is negative: `nth_root(-8, 3) == -2`. An even or fractional `n`
///   gives `NaN`, like [`sqrt`] does.
/// - A negative `n` is the reciprocal: `nth_root(8, -3) == 0.5`.
/// - A fractional `n` is allowed for non-negative `x` and behaves as
///   `pow(x, 1 / n)`.
///
/// The result is computed as `pow(x, 1 / n)`, so for exact powers like
/// `nth_root(27, 3)` it can be off from the exact answer by a rounding
/// error; compare with [`crate::approx::approx_eq_eps`] rather than `==`.
/// `n == 2` uses [`sqrt`] and is exact wherever `sqrt` is.
#[inline]
pub fn nth_root<T: Scalar>(x: T, n: T) -> T {
    let two = T::ONE + T::ONE;
    if n == T::ZERO {
        return T::NAN;
    }
    if n == two {
        return x.sqrt();
    }
    let inverse = T::ONE / n;
    if x >= T::ZERO {
        x.powf(inverse)
    } else if x < T::ZERO && n == n.floor() && n.modulo(two) == T::ONE {
        // Odd whole degree: the real root of a negative number is the
        // negation of the root of its magnitude.
        T::ZERO - (T::ZERO - x).powf(inverse)
    } else {
        // Even or fractional degree of a negative number, or `x` is NaN.
        T::NAN
    }
}

/// The real cube root of `x`. Defined for negative numbers too
/// (`cbrt(-8) == -2`), and exact for perfect cubes on common platforms,
/// unlike `pow(x, 1.0 / 3.0)`. See [`nth_root`] for other degrees.
#[inline]
pub fn cbrt<T: Scalar>(x: T) -> T {
    x.cbrt()
}

/// The length of the hypotenuse of a right triangle with legs `x` and `y`,
/// `sqrt(x² + y²)`, computed without intermediate overflow or underflow
/// (so `hypot(1e30_f32, 1e30)` is finite where `sqrt(x * x + y * y)` is
/// `inf`). Slower than the plain formula; use it when the inputs can be
/// extreme.
#[inline]
pub fn hypot<T: Scalar>(x: T, y: T) -> T {
    x.hypot(y)
}

/// `e` raised to the power `x`. `exp(0) == 1`; large `x` overflows to
/// `inf`, very negative `x` underflows to `0`.
#[inline]
pub fn exp<T: Scalar>(x: T) -> T {
    x.exp()
}

/// The natural (base `e`) logarithm of `x`. `NaN` for negative `x`, `-inf`
/// for zero.
#[inline]
pub fn ln<T: Scalar>(x: T) -> T {
    x.ln()
}

/// The base-2 logarithm of `x`. `NaN` for negative `x`, `-inf` for zero.
#[inline]
pub fn log2<T: Scalar>(x: T) -> T {
    x.log2()
}

/// The base-10 logarithm of `x`. `NaN` for negative `x`, `-inf` for zero.
#[inline]
pub fn log10<T: Scalar>(x: T) -> T {
    x.log10()
}

/// The same angle functions as the parent module, on bare numbers in
/// **radians** instead of [`Radians`].
///
/// Use these when the surrounding code already works with raw floats and
/// wrapping every value in [`Radians`] would only add noise. [`sqrt`],
/// [`pow`], [`nth_root`], [`exp`], [`ln`] and the other roots and
/// logarithms are not angle functions and have no counterpart here.
pub mod raw {
    use crate::Scalar;

    /// The sine of `radians`.
    #[inline]
    pub fn sin<T: Scalar>(radians: T) -> T {
        radians.sin()
    }

    /// The cosine of `radians`.
    #[inline]
    pub fn cos<T: Scalar>(radians: T) -> T {
        radians.cos()
    }

    /// The tangent of `radians`.
    #[inline]
    pub fn tan<T: Scalar>(radians: T) -> T {
        radians.tan()
    }

    /// The cotangent of `radians`, `cos / sin`. `±inf` where the sine is
    /// exactly zero.
    #[inline]
    pub fn cot<T: Scalar>(radians: T) -> T {
        radians.cos() / radians.sin()
    }

    /// The arcsine of `x`, in radians within `[-π/2, π/2]`. `NaN` outside
    /// `[-1, 1]`.
    #[inline]
    pub fn asin<T: Scalar>(x: T) -> T {
        x.asin()
    }

    /// The arccosine of `x`, in radians within `[0, π]`. `NaN` outside
    /// `[-1, 1]`.
    #[inline]
    pub fn acos<T: Scalar>(x: T) -> T {
        x.acos()
    }

    /// The four-quadrant arctangent of `y` and `x`, in radians within
    /// `[-π, π]`. `y` comes first.
    #[inline]
    pub fn atan2<T: Scalar>(y: T, x: T) -> T {
        y.atan2(x)
    }

    /// The arctangent of `x`, in radians within `[-π/2, π/2]`.
    #[inline]
    pub fn atan<T: Scalar>(x: T) -> T {
        x.atan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::Degrees;
    use crate::approx::approx_eq_eps;

    const EPS64: f64 = 1e-12;
    const EPS32: f32 = 1e-5;

    fn close64(a: f64, b: f64) -> bool {
        approx_eq_eps(a, b, EPS64)
    }

    fn close32(a: f32, b: f32) -> bool {
        approx_eq_eps(a, b, EPS32)
    }

    #[test]
    fn sin_cos_tan_known_values() {
        assert!(close64(sin(Radians(0.0_f64)), 0.0));
        assert!(close64(sin(Radians(core::f64::consts::FRAC_PI_2)), 1.0));
        assert!(close64(sin(Radians(core::f64::consts::PI)), 0.0));
        assert!(close64(cos(Radians(0.0_f64)), 1.0));
        assert!(close64(cos(Radians(core::f64::consts::PI)), -1.0));
        assert!(close64(tan(Radians(core::f64::consts::FRAC_PI_4)), 1.0));
        assert!(close32(sin(Radians(core::f32::consts::FRAC_PI_6)), 0.5));
        assert!(close32(cos(Radians(core::f32::consts::FRAC_PI_3)), 0.5));
    }

    #[test]
    fn degrees_are_accepted_directly() {
        assert!(close64(sin(Degrees(30.0_f64)), 0.5));
        assert!(close64(cos(Degrees(60.0_f64)), 0.5));
        assert!(close64(tan(Degrees(45.0_f64)), 1.0));
        assert!(close32(sin(Degrees(90.0_f32)), 1.0));
        assert!(close64(cot(Degrees(45.0_f64)), 1.0));
    }

    #[test]
    fn pythagorean_identity() {
        for deg in [-270.0_f64, -90.0, 0.0, 17.0, 45.0, 123.0, 720.0] {
            let s = sin(Degrees(deg));
            let c = cos(Degrees(deg));
            assert!(close64(s * s + c * c, 1.0), "{deg} degrees");
        }
    }

    #[test]
    fn cot_is_reciprocal_of_tan() {
        for rad in [0.3_f64, 0.7, 1.0, 2.0, -1.2] {
            assert!(close64(cot(Radians(rad)) * tan(Radians(rad)), 1.0));
        }
    }

    #[test]
    fn cot_at_zero_is_infinite() {
        assert_eq!(cot(Radians(0.0_f32)), f32::INFINITY);
        assert_eq!(cot(Radians(-0.0_f32)), f32::NEG_INFINITY);
        assert_eq!(cot(Radians(0.0_f64)), f64::INFINITY);
        assert!(close64(cot(Radians(core::f64::consts::FRAC_PI_2)), 0.0));
    }

    #[test]
    fn inverse_functions_return_radians() {
        let a: Radians<f64> = asin(1.0);
        assert!(close64(a.0, core::f64::consts::FRAC_PI_2));
        let b: Radians<f64> = acos(0.5);
        assert!(close64(b.0, core::f64::consts::FRAC_PI_3));
        let c: Radians<f32> = asin(0.0);
        assert!(close32(c.0, 0.0));
        let d: Radians<f32> = acos(-1.0);
        assert!(close32(d.0, core::f32::consts::PI));
    }

    #[test]
    fn inverse_functions_round_trip() {
        for x in [-1.0_f64, -0.6, 0.0, 0.25, 0.9, 1.0] {
            assert!(close64(sin(asin(x)), x));
            assert!(close64(cos(acos(x)), x));
        }
    }

    #[test]
    fn inverse_functions_are_nan_out_of_domain() {
        assert!(asin(1.5_f32).0.is_nan());
        assert!(asin(-1.5_f64).0.is_nan());
        assert!(acos(2.0_f32).0.is_nan());
        assert!(acos(-1.0001_f64).0.is_nan());
    }

    #[test]
    fn atan2_covers_all_quadrants() {
        let q = |y: f64, x: f64| atan2(y, x).0;
        assert!(close64(q(0.0, 1.0), 0.0));
        assert!(close64(q(1.0, 1.0), core::f64::consts::FRAC_PI_4));
        assert!(close64(q(1.0, 0.0), core::f64::consts::FRAC_PI_2));
        assert!(close64(q(1.0, -1.0), 3.0 * core::f64::consts::FRAC_PI_4));
        assert!(close64(q(0.0, -1.0), core::f64::consts::PI));
        assert!(close64(q(-1.0, -1.0), -3.0 * core::f64::consts::FRAC_PI_4));
        assert!(close64(q(-1.0, 0.0), -core::f64::consts::FRAC_PI_2));
        assert!(close64(q(-1.0, 1.0), -core::f64::consts::FRAC_PI_4));
        assert_eq!(atan2(0.0_f32, 0.0).0, 0.0);
    }

    #[test]
    fn atan2_argument_order_is_y_then_x() {
        // The point (x = 1, y = 2) is above the x axis: angle > 45 degrees.
        let deg = Degrees::from(atan2(2.0_f64, 1.0)).0;
        assert!(deg > 63.0 && deg < 64.0, "{deg}");
    }

    #[test]
    fn raw_functions_match_typed_ones() {
        for x in [-2.0_f64, -0.5, 0.0, 0.3, 1.0, 2.5] {
            assert_eq!(raw::sin(x), sin(Radians(x)));
            assert_eq!(raw::cos(x), cos(Radians(x)));
            assert_eq!(raw::tan(x), tan(Radians(x)));
            assert_eq!(raw::cot(x), cot(Radians(x)));
        }
        assert_eq!(raw::asin(0.5_f32), asin(0.5_f32).0);
        assert_eq!(raw::acos(0.5_f32), acos(0.5_f32).0);
        assert_eq!(raw::atan2(1.0_f64, -2.0), atan2(1.0_f64, -2.0).0);
    }

    #[test]
    fn sqrt_and_pow() {
        assert_eq!(sqrt(9.0_f32), 3.0);
        assert_eq!(sqrt(2.25_f64), 1.5);
        assert!(sqrt(-1.0_f32).is_nan());
        assert_eq!(pow(2.0_f64, 10.0), 1024.0);
        assert!(close64(pow(9.0_f64, 0.5), 3.0));
        assert_eq!(pow(5.0_f32, 0.0), 1.0);
        assert_eq!(pow(2.0_f32, -1.0), 0.5);
        assert!(pow(-8.0_f64, 0.5).is_nan());
        assert_eq!(pow(-2.0_f64, 3.0), -8.0);
    }

    #[test]
    fn nth_root_positive_input() {
        assert_eq!(nth_root(16.0_f64, 2.0), 4.0);
        assert!(close64(nth_root(27.0_f64, 3.0), 3.0));
        assert!(close64(nth_root(81.0_f64, 4.0), 3.0));
        assert!(close32(nth_root(32.0_f32, 5.0), 2.0));
        assert_eq!(nth_root(7.0_f64, 1.0), 7.0);
        assert_eq!(nth_root(0.0_f64, 3.0), 0.0);
    }

    #[test]
    fn nth_root_of_negative_needs_an_odd_degree() {
        assert!(close64(nth_root(-8.0_f64, 3.0), -2.0));
        assert!(close64(nth_root(-32.0_f64, 5.0), -2.0));
        assert!(close64(nth_root(-7.0_f64, 1.0), -7.0));
        assert!(nth_root(-4.0_f64, 2.0).is_nan());
        assert!(nth_root(-16.0_f64, 4.0).is_nan());
        // Fractional degrees have no real root of a negative number.
        assert!(nth_root(-8.0_f64, 1.5).is_nan());
    }

    #[test]
    fn nth_root_degenerate_degrees() {
        assert!(nth_root(8.0_f64, 0.0).is_nan());
        assert!(close64(nth_root(8.0_f64, -3.0), 0.5));
        assert!(close64(nth_root(-8.0_f64, -3.0), -0.5));
        assert!(nth_root(f64::NAN, 3.0).is_nan());
        assert!(nth_root(8.0_f64, f64::NAN).is_nan());
        assert!(close64(nth_root(4.0_f64, 0.5), 16.0));
    }

    #[test]
    fn nth_root_round_trips_through_pow() {
        for (x, n) in [(2.0_f64, 3.0), (10.0, 4.0), (0.5, 5.0), (-3.0, 3.0)] {
            assert!(approx_eq_eps(pow(nth_root(x, n), n), x, 1e-9), "{x}, {n}");
        }
    }

    #[test]
    fn atan_returns_radians_in_half_range() {
        let a: Radians<f64> = atan(1.0);
        assert!(close64(a.0, core::f64::consts::FRAC_PI_4));
        assert!(close64(atan(-1.0_f64).0, -core::f64::consts::FRAC_PI_4));
        assert!(close32(atan(f32::INFINITY).0, core::f32::consts::FRAC_PI_2));
        assert!(close32(
            atan(f32::NEG_INFINITY).0,
            -core::f32::consts::FRAC_PI_2
        ));
        assert_eq!(atan(0.0_f32).0, 0.0);
        assert_eq!(raw::atan(0.5_f64), atan(0.5_f64).0);
        // tan and atan are inverses on (-pi/2, pi/2).
        for x in [-3.0_f64, -0.5, 0.0, 0.7, 12.0] {
            assert!(approx_eq_eps(tan(atan(x)), x, 1e-9));
        }
    }

    #[test]
    fn cbrt_and_hypot() {
        assert_eq!(cbrt(27.0_f32), 3.0);
        assert_eq!(cbrt(-8.0_f64), -2.0);
        assert_eq!(cbrt(0.0_f64), 0.0);
        let c = cbrt(2.0_f64);
        assert!(close64(c * c * c, 2.0));
        assert_eq!(hypot(3.0_f32, 4.0), 5.0);
        assert_eq!(hypot(-5.0_f64, 12.0), 13.0);
        assert!((sqrt(1.0e30_f32 * 1.0e30 + 1.0e30 * 1.0e30)).is_infinite());
        assert!(hypot(1.0e30_f32, 1.0e30).is_finite());
    }

    #[test]
    fn exp_and_logarithms() {
        assert_eq!(exp(0.0_f32), 1.0);
        assert!(close64(exp(1.0_f64), core::f64::consts::E));
        assert_eq!(exp(f64::NEG_INFINITY), 0.0);
        assert!(exp(1000.0_f64).is_infinite());
        assert_eq!(ln(1.0_f64), 0.0);
        assert!(close64(ln(core::f64::consts::E), 1.0));
        assert_eq!(log2(1024.0_f32), 10.0);
        assert!(close64(log10(0.001_f64), -3.0));
        for x in [0.1_f64, 1.0, 2.5, 40.0] {
            assert!(approx_eq_eps(exp(ln(x)), x, 1e-12 * x.max(1.0)));
            assert!(approx_eq_eps(pow(2.0, log2(x)), x, 1e-12 * x.max(1.0)));
        }
    }

    #[test]
    fn logarithm_domain() {
        assert!(ln(-1.0_f32).is_nan());
        assert!(log2(-0.5_f64).is_nan());
        assert!(log10(-3.0_f32).is_nan());
        assert_eq!(ln(0.0_f64), f64::NEG_INFINITY);
        assert_eq!(log10(0.0_f32), f32::NEG_INFINITY);
    }
}
