//! Interpolation helpers: [`lerp`](crate::interp::lerp),
//! [`inverse_lerp`](crate::interp::inverse_lerp),
//! [`remap`](crate::interp::remap),
//! [`smoothstep`](crate::interp::smoothstep),
//! [`step`](crate::interp::step) and [`delta`](crate::interp::delta).
//!
//! Every function here is written once, generically, against [`Scalar`],
//! so it works for both `f32` and `f64`.
//!
//! # `const fn` variants
//!
//! Trait methods cannot be `const` on stable Rust, so the generic
//! functions above cannot be `const fn` either. For use in `const` and
//! `static` items, [`f32`](crate::interp::f32) and
//! [`f64`](crate::interp::f64) hold the same six functions as
//! concrete `const fn`s (`math::interp::f32::lerp(..)`), generated from a
//! single definition so the two families cannot drift apart. The tests in
//! this file check that they agree with the generic versions.
//!
//! None of these functions clamp their inputs unless their own
//! documentation says so (`smoothstep` does): `lerp` extrapolates for
//! `t` outside `[0, 1]`, and `inverse_lerp`/`remap` likewise return values
//! outside `[0, 1]` for inputs outside the source range.

use crate::Scalar;

/// Linear interpolation from `a` (at `t = 0`) to `b` (at `t = 1`), computed
/// as `a + (b - a) * t`.
///
/// Does not clamp: `t` outside `[0, 1]` extrapolates along the same line.
/// This form is exact at `t = 0` but can miss `b` by a rounding error at
/// `t = 1`; it is used over `a * (1 - t) + b * t` because it needs one
/// multiplication instead of two.
#[inline]
pub fn lerp<T: Scalar>(a: T, b: T, t: T) -> T {
    a + (b - a) * t
}

/// The `t` for which [`lerp`]`(a, b, t) == v`: how far `v` is from `a`
/// towards `b`, as a fraction (`0` at `a`, `1` at `b`).
///
/// Does not clamp. If `a == b` there is no meaningful answer, so this
/// returns `0` rather than dividing by zero.
#[inline]
pub fn inverse_lerp<T: Scalar>(a: T, b: T, v: T) -> T {
    if a == b { T::ZERO } else { (v - a) / (b - a) }
}

/// Maps `v` from the range `[in_min, in_max]` to the range
/// `[out_min, out_max]` (`inverse_lerp` followed by `lerp`).
///
/// Does not clamp, and either range may be reversed (`min > max`). If
/// `in_min == in_max` the result is `out_min` (see [`inverse_lerp`]).
#[inline]
pub fn remap<T: Scalar>(v: T, in_min: T, in_max: T, out_min: T, out_max: T) -> T {
    lerp(out_min, out_max, inverse_lerp(in_min, in_max, v))
}

/// A step function: `0` if `x < edge`, otherwise `1`.
///
/// Same convention as GLSL's `step(edge, x)`, except a `NaN` `x` gives `0`
/// (the comparison is written as `x >= edge`).
#[inline]
pub fn step<T: Scalar>(edge: T, x: T) -> T {
    if x >= edge { T::ONE } else { T::ZERO }
}

/// Smooth Hermite interpolation between `0` (at `x <= edge0`) and `1` (at
/// `x >= edge1`): `t * t * (3 - 2 * t)` with `t` clamped to `[0, 1]`.
///
/// Unlike GLSL's `smoothstep`, whose result is undefined in these cases,
/// this is defined for reversed edges (`edge0 > edge1`, giving a curve
/// that falls from `1` to `0`) and for `edge0 == edge1`, where it acts as
/// [`step`]`(edge0, x)`.
#[inline]
pub fn smoothstep<T: Scalar>(edge0: T, edge1: T, x: T) -> T {
    if edge0 == edge1 {
        return step(edge0, x);
    }
    let t = Scalar::clamp(inverse_lerp(edge0, edge1, x), T::ZERO, T::ONE);
    let two = T::ONE + T::ONE;
    let three = two + T::ONE;
    t * t * (three - two * t)
}

/// The signed difference `to - from`: how far, and in which direction, you
/// must move to get from `from` to `to`.
#[inline]
pub fn delta<T: Scalar>(from: T, to: T) -> T {
    to - from
}

/// Generates the `const fn` versions of this module's six functions for
/// one concrete float type. Invoked once inside each of [`f32`]/[`f64`].
macro_rules! const_interp {
    ($t:ty) => {
        /// `const fn` version of [`super::lerp`].
        #[inline]
        pub const fn lerp(a: $t, b: $t, t: $t) -> $t {
            a + (b - a) * t
        }

        /// `const fn` version of [`super::inverse_lerp`].
        #[inline]
        pub const fn inverse_lerp(a: $t, b: $t, v: $t) -> $t {
            if a == b { 0.0 } else { (v - a) / (b - a) }
        }

        /// `const fn` version of [`super::remap`].
        #[inline]
        pub const fn remap(v: $t, in_min: $t, in_max: $t, out_min: $t, out_max: $t) -> $t {
            lerp(out_min, out_max, inverse_lerp(in_min, in_max, v))
        }

        /// `const fn` version of [`super::step`].
        #[inline]
        pub const fn step(edge: $t, x: $t) -> $t {
            if x >= edge { 1.0 } else { 0.0 }
        }

        /// `const fn` version of [`super::smoothstep`].
        #[inline]
        pub const fn smoothstep(edge0: $t, edge1: $t, x: $t) -> $t {
            if edge0 == edge1 {
                return step(edge0, x);
            }
            let raw = inverse_lerp(edge0, edge1, x);
            // Manual clamp: `clamp` is not `const`. A `NaN` falls through
            // both comparisons unchanged, same as the generic version.
            let t = if raw < 0.0 {
                0.0
            } else if raw > 1.0 {
                1.0
            } else {
                raw
            };
            t * t * (3.0 - 2.0 * t)
        }

        /// `const fn` version of [`super::delta`].
        #[inline]
        pub const fn delta(from: $t, to: $t) -> $t {
            to - from
        }
    };
}

/// `const fn` versions of this module's functions for `f32`.
pub mod f32 {
    const_interp!(f32);
}

/// `const fn` versions of this module's functions for `f64`.
pub mod f64 {
    const_interp!(f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approx::approx_eq;

    // `super::f32`/`super::f64` are modules here, so the primitive types
    // are spelled out to keep the two apart.
    type F32 = core::primitive::f32;
    type F64 = core::primitive::f64;

    // Compile-time evaluation: proves the `const fn` versions really are.
    const CONST_LERP: F32 = f32::lerp(0.0, 10.0, 0.5);
    const CONST_SMOOTH: F64 = f64::smoothstep(0.0, 1.0, 0.25);

    #[test]
    fn lerp_endpoints_and_midpoint() {
        assert_eq!(lerp(2.0_f32, 6.0, 0.0), 2.0);
        assert_eq!(lerp(2.0_f32, 6.0, 1.0), 6.0);
        assert_eq!(lerp(2.0_f32, 6.0, 0.5), 4.0);
        assert_eq!(lerp(2.0_f64, 6.0, 0.5), 4.0);
    }

    #[test]
    fn lerp_extrapolates() {
        assert_eq!(lerp(0.0_f32, 10.0, 2.0), 20.0);
        assert_eq!(lerp(0.0_f32, 10.0, -1.0), -10.0);
    }

    #[test]
    fn inverse_lerp_basic() {
        assert_eq!(inverse_lerp(0.0_f32, 10.0, 5.0), 0.5);
        assert_eq!(inverse_lerp(0.0_f32, 10.0, 15.0), 1.5);
        assert_eq!(inverse_lerp(10.0_f32, 0.0, 2.5), 0.75);
    }

    #[test]
    fn inverse_lerp_degenerate_range_is_zero() {
        assert_eq!(inverse_lerp(3.0_f32, 3.0, 99.0), 0.0);
        assert_eq!(inverse_lerp(3.0_f64, 3.0, 3.0), 0.0);
    }

    #[test]
    fn lerp_and_inverse_lerp_round_trip() {
        let t = inverse_lerp(-4.0_f64, 12.0, 5.0);
        assert!(approx_eq(lerp(-4.0_f64, 12.0, t), 5.0));
    }

    #[test]
    fn remap_basic_and_reversed() {
        assert_eq!(remap(5.0_f32, 0.0, 10.0, 100.0, 200.0), 150.0);
        // Reversed output range.
        assert_eq!(remap(2.5_f32, 0.0, 10.0, 1.0, 0.0), 0.75);
        // Reversed input range.
        assert_eq!(remap(2.5_f32, 10.0, 0.0, 0.0, 100.0), 75.0);
        // Not clamped.
        assert_eq!(remap(20.0_f32, 0.0, 10.0, 0.0, 1.0), 2.0);
    }

    #[test]
    fn remap_degenerate_input_range_gives_out_min() {
        assert_eq!(remap(7.0_f32, 4.0, 4.0, 10.0, 20.0), 10.0);
    }

    #[test]
    fn step_convention() {
        assert_eq!(step(0.5_f32, 0.25), 0.0);
        assert_eq!(step(0.5_f32, 0.5), 1.0);
        assert_eq!(step(0.5_f32, 0.75), 1.0);
        assert_eq!(step(0.5_f32, F32::NAN), 0.0);
    }

    #[test]
    fn smoothstep_shape() {
        assert_eq!(smoothstep(0.0_f32, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0_f32, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0_f32, 1.0, 0.5), 0.5);
        assert_eq!(smoothstep(0.0_f32, 1.0, 1.0), 1.0);
        assert_eq!(smoothstep(0.0_f32, 1.0, 2.0), 1.0);
        // t = 0.25: 0.0625 * (3 - 0.5)
        assert_eq!(smoothstep(0.0_f32, 1.0, 0.25), 0.15625);
    }

    #[test]
    fn smoothstep_with_offset_and_reversed_edges() {
        assert_eq!(smoothstep(10.0_f32, 20.0, 15.0), 0.5);
        // Reversed edges fall from 1 to 0.
        assert_eq!(smoothstep(1.0_f32, 0.0, 1.0), 0.0);
        assert_eq!(smoothstep(1.0_f32, 0.0, 0.0), 1.0);
        assert_eq!(smoothstep(1.0_f32, 0.0, 0.5), 0.5);
    }

    #[test]
    fn smoothstep_degenerate_edges_act_as_step() {
        assert_eq!(smoothstep(2.0_f32, 2.0, 1.9), 0.0);
        assert_eq!(smoothstep(2.0_f32, 2.0, 2.0), 1.0);
    }

    #[test]
    fn delta_is_directional() {
        assert_eq!(delta(3.0_f32, 8.0), 5.0);
        assert_eq!(delta(8.0_f32, 3.0), -5.0);
        assert_eq!(delta(1.5_f64, 1.5), 0.0);
    }

    #[test]
    fn const_versions_evaluate_at_compile_time() {
        assert_eq!(CONST_LERP, 5.0);
        assert_eq!(CONST_SMOOTH, 0.15625);
    }

    #[test]
    fn const_versions_match_generic_versions() {
        let xs: [F32; 7] = [-2.0, -0.5, 0.0, 0.25, 0.5, 1.0, 3.0];
        for &x in &xs {
            assert_eq!(f32::lerp(1.0, 5.0, x), lerp(1.0, 5.0, x));
            assert_eq!(f32::inverse_lerp(1.0, 5.0, x), inverse_lerp(1.0, 5.0, x));
            assert_eq!(f32::inverse_lerp(2.0, 2.0, x), inverse_lerp(2.0, 2.0, x));
            assert_eq!(
                f32::remap(x, 0.0, 1.0, 10.0, -10.0),
                remap(x, 0.0, 1.0, 10.0, -10.0)
            );
            assert_eq!(f32::step(0.5, x), step(0.5, x));
            assert_eq!(f32::smoothstep(0.0, 1.0, x), smoothstep(0.0, 1.0, x));
            assert_eq!(f32::smoothstep(1.0, 0.0, x), smoothstep(1.0, 0.0, x));
            assert_eq!(f32::smoothstep(0.5, 0.5, x), smoothstep(0.5, 0.5, x));
            assert_eq!(f32::delta(x, 1.0), delta(x, 1.0));
        }
        let ys: [F64; 5] = [-1.0, 0.0, 0.3, 1.0, 4.0];
        for &y in &ys {
            assert_eq!(f64::lerp(-1.0, 1.0, y), lerp(-1.0, 1.0, y));
            assert_eq!(f64::inverse_lerp(-1.0, 1.0, y), inverse_lerp(-1.0, 1.0, y));
            assert_eq!(
                f64::remap(y, 0.0, 1.0, 2.0, 4.0),
                remap(y, 0.0, 1.0, 2.0, 4.0)
            );
            assert_eq!(f64::step(0.3, y), step(0.3, y));
            assert_eq!(f64::smoothstep(0.0, 1.0, y), smoothstep(0.0, 1.0, y));
            assert_eq!(f64::delta(y, 2.0), delta(y, 2.0));
        }
    }
}
