//! Interpolation helpers: [`lerp`](crate::interp::lerp),
//! [`inverse_lerp`](crate::interp::inverse_lerp),
//! [`remap`](crate::interp::remap),
//! [`smoothstep`](crate::interp::smoothstep),
//! [`step`](crate::interp::step) and [`delta`](crate::interp::delta), plus
//! (Phase 2.1) [`smootherstep`](crate::interp::smootherstep),
//! [`move_towards`](crate::interp::move_towards),
//! [`repeat`](crate::interp::repeat),
//! [`ping_pong`](crate::interp::ping_pong) and the critically damped
//! [`smooth_damp`](crate::interp::smooth_damp). Easing curves live in
//! [`crate::ease`].
//!
//! Every function here is written once, generically, against [`Scalar`],
//! so it works for both `f32` and `f64`.
//!
//! # `const fn` variants
//!
//! Trait methods cannot be `const` on stable Rust, so the generic
//! functions above cannot be `const fn` either. For use in `const` and
//! `static` items, [`f32`](crate::interp::f32) and
//! [`f64`](crate::interp::f64) hold the same functions as
//! concrete `const fn`s (`math::interp::f32::lerp(..)`), generated from a
//! single definition so the two families cannot drift apart. The tests in
//! this file check that they agree with the generic versions. Only the
//! functions that need no float method that is not `const` have one:
//! the original six plus `smootherstep` and `move_towards`. `repeat`,
//! `ping_pong` and `smooth_damp` (Euclidean remainder, state) are generic
//! only.
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

/// Smoother Hermite interpolation between `0` (at `x <= edge0`) and `1`
/// (at `x >= edge1`): Perlin's `6t⁵ - 15t⁴ + 10t³` with `t` clamped to
/// `[0, 1]`.
///
/// Like [`smoothstep`] but with zero first *and second* derivative at both
/// edges, so chained transitions have no visible acceleration jump. Reversed
/// and equal edges behave exactly as documented on [`smoothstep`].
#[inline]
pub fn smootherstep<T: Scalar>(edge0: T, edge1: T, x: T) -> T {
    if edge0 == edge1 {
        return step(edge0, x);
    }
    let t = Scalar::clamp(inverse_lerp(edge0, edge1, x), T::ZERO, T::ONE);
    t * t * t * (t * (t * T::from_f64(6.0) - T::from_f64(15.0)) + T::from_f64(10.0))
}

/// Moves `current` towards `target` by at most `max_delta` and never past
/// it: the constant-speed counterpart of [`lerp`]'s fractions.
///
/// If `target` is within `max_delta` it is returned exactly. A **negative**
/// `max_delta` pushes `current` *away* from `target` (Unity's
/// `Mathf.MoveTowards` does the same), which is occasionally useful and
/// otherwise a bug to watch for. A `NaN` input gives `NaN`.
#[inline]
pub fn move_towards<T: Scalar>(current: T, target: T, max_delta: T) -> T {
    let diff = target - current;
    if diff.abs() <= max_delta {
        target
    } else {
        current + diff.sign() * max_delta
    }
}

/// Wraps `t` into `[0, length)`: the Euclidean remainder, so a negative
/// `t` wraps up from `length` instead of staying negative like `%`.
///
/// `length` should be positive. A negative one wraps into `[0, |length|)`,
/// zero (or `NaN`) gives `NaN`. The result is never `length` itself, even
/// when a tiny negative `t` rounds up to it.
#[inline]
pub fn repeat<T: Scalar>(t: T, length: T) -> T {
    let wrapped = t.modulo(length);
    if wrapped >= length.abs() {
        T::ZERO
    } else {
        wrapped
    }
}

/// Bounces `t` back and forth between `0` and `length`: a triangle wave
/// with period `2 * length` (`0 -> length -> 0 -> length ...`).
///
/// Useful for looping an oscillating value. Works for negative `t`;
/// `length` should be positive (see [`repeat`]).
#[inline]
pub fn ping_pong<T: Scalar>(t: T, length: T) -> T {
    let wrapped = repeat(t, length * T::from_f64(2.0));
    length - (wrapped - length).abs()
}

/// Smoothly moves `current` towards `target` like a critically damped
/// spring, with no overshoot. Call it once per frame, feeding the return
/// value back in as `current`.
///
/// - `velocity` is the spring's state: keep it between calls (start from
///   `T::ZERO`); the function updates it.
/// - `smooth_time` is roughly the time it takes to reach the target. Values
///   below `0.0001` are raised to it.
/// - `delta_time` is the frame time, in the same unit as `smooth_time`.
///
/// This is the Game Programming Gems 4 / Unity `Mathf.SmoothDamp`
/// formulation (a cheap polynomial in place of `exp`), so it needs no
/// transcendental functions and is stable for any `delta_time`. See
/// [`smooth_damp_clamped`] to cap the speed.
///
/// ```
/// use math::interp::smooth_damp;
///
/// let (mut x, mut v) = (0.0_f32, 0.0_f32);
/// for _ in 0..600 {
///     x = smooth_damp(x, 10.0, &mut v, 0.25, 1.0 / 60.0);
/// }
/// assert!((x - 10.0).abs() < 1e-3);
/// ```
#[inline]
pub fn smooth_damp<T: Scalar>(
    current: T,
    target: T,
    velocity: &mut T,
    smooth_time: T,
    delta_time: T,
) -> T {
    smooth_damp_clamped(
        current,
        target,
        velocity,
        smooth_time,
        T::from_f64(f64::INFINITY),
        delta_time,
    )
}

/// [`smooth_damp`] with a speed limit: the spring never asks for more than
/// `max_speed` (its absolute value is used) units per unit of time.
#[inline]
pub fn smooth_damp_clamped<T: Scalar>(
    current: T,
    target: T,
    velocity: &mut T,
    smooth_time: T,
    max_speed: T,
    delta_time: T,
) -> T {
    let smooth_time = smooth_time.max(T::from_f64(1.0e-4));
    let omega = T::from_f64(2.0) / smooth_time;
    let x = omega * delta_time;
    let decay = T::ONE / (T::ONE + x + T::from_f64(0.48) * x * x + T::from_f64(0.235) * x * x * x);

    // Limit the distance the spring is allowed to cover in one smooth_time.
    let max_change = max_speed.abs() * smooth_time;
    let change = (current - target).max(T::ZERO - max_change).min(max_change);
    let limited_target = current - change;

    let temp = (*velocity + omega * change) * delta_time;
    *velocity = (*velocity - omega * temp) * decay;
    let mut output = limited_target + (change + temp) * decay;

    // Don't overshoot the real target.
    if (target - current > T::ZERO) == (output > target) {
        output = target;
        *velocity = T::ZERO;
    }
    output
}

/// Generates the `const fn` versions of this module's eight functions for
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

        /// `const fn` version of [`super::smootherstep`].
        #[inline]
        pub const fn smootherstep(edge0: $t, edge1: $t, x: $t) -> $t {
            if edge0 == edge1 {
                return step(edge0, x);
            }
            let raw = inverse_lerp(edge0, edge1, x);
            let t = if raw < 0.0 {
                0.0
            } else if raw > 1.0 {
                1.0
            } else {
                raw
            };
            t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
        }

        /// `const fn` version of [`super::move_towards`].
        #[inline]
        pub const fn move_towards(current: $t, target: $t, max_delta: $t) -> $t {
            let diff = target - current;
            let abs = if diff < 0.0 { -diff } else { diff };
            if abs <= max_delta {
                return target;
            }
            // Three-state sign; a NaN `diff` stays NaN like the generic one.
            let sign = if diff == 0.0 {
                0.0
            } else if diff > 0.0 {
                1.0
            } else if diff < 0.0 {
                -1.0
            } else {
                diff
            };
            current + sign * max_delta
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

    #[test]
    fn smootherstep_shape() {
        assert_eq!(smootherstep(0.0_f32, 1.0, -1.0), 0.0);
        assert_eq!(smootherstep(0.0_f32, 1.0, 0.0), 0.0);
        assert_eq!(smootherstep(0.0_f32, 1.0, 0.5), 0.5);
        assert_eq!(smootherstep(0.0_f32, 1.0, 1.0), 1.0);
        assert_eq!(smootherstep(0.0_f32, 1.0, 2.0), 1.0);
        // t = 0.25: 6/1024 - 15/256 + 10/64 = 0.103515625
        assert_eq!(smootherstep(0.0_f64, 1.0, 0.25), 0.103515625);
        // Flatter than smoothstep near the edge.
        assert!(smootherstep(0.0_f32, 1.0, 0.1) < smoothstep(0.0_f32, 1.0, 0.1));
    }

    #[test]
    fn smootherstep_reversed_and_degenerate_edges() {
        assert_eq!(smootherstep(1.0_f32, 0.0, 1.0), 0.0);
        assert_eq!(smootherstep(1.0_f32, 0.0, 0.0), 1.0);
        assert_eq!(smootherstep(1.0_f32, 0.0, 0.5), 0.5);
        assert_eq!(smootherstep(2.0_f32, 2.0, 1.9), 0.0);
        assert_eq!(smootherstep(2.0_f32, 2.0, 2.0), 1.0);
    }

    #[test]
    fn move_towards_steps_and_snaps() {
        assert_eq!(move_towards(0.0_f32, 10.0, 3.0), 3.0);
        assert_eq!(move_towards(10.0_f32, 0.0, 3.0), 7.0);
        // Never overshoots: lands exactly on the target.
        assert_eq!(move_towards(9.0_f32, 10.0, 3.0), 10.0);
        assert_eq!(move_towards(10.0_f64, 10.0, 0.0), 10.0);
        assert_eq!(move_towards(4.0_f64, 4.0, 1.0), 4.0);
        // Zero step stays put.
        assert_eq!(move_towards(1.0_f32, 5.0, 0.0), 1.0);
    }

    #[test]
    fn move_towards_negative_delta_moves_away() {
        assert_eq!(move_towards(0.0_f32, 10.0, -2.0), -2.0);
        assert_eq!(move_towards(5.0_f32, 5.0, -2.0), 5.0);
    }

    #[test]
    fn move_towards_nan_gives_nan() {
        assert!(move_towards(F32::NAN, 1.0, 1.0).is_nan());
        assert!(move_towards(0.0_f32, F32::NAN, 1.0).is_nan());
    }

    #[test]
    fn repeat_wraps_euclidean() {
        assert_eq!(repeat(5.5_f32, 2.0), 1.5);
        assert_eq!(repeat(-0.5_f32, 2.0), 1.5);
        assert_eq!(repeat(-4.0_f64, 2.0), 0.0);
        assert_eq!(repeat(2.0_f64, 2.0), 0.0);
        assert_eq!(repeat(1.25_f32, 2.0), 1.25);
    }

    #[test]
    fn repeat_never_returns_length() {
        // `rem_euclid(-1e-10, 1.0)` rounds up to exactly 1.0 in f32.
        let r = repeat(-1.0e-10_f32, 1.0);
        assert!((0.0..1.0).contains(&r), "got {r}");
        let r = repeat(-1.0e-20_f64, 1.0);
        assert!((0.0..1.0).contains(&r), "got {r}");
    }

    #[test]
    fn repeat_edge_cases() {
        assert!(repeat(1.0_f32, 0.0).is_nan());
        assert!(repeat(F32::INFINITY, 1.0).is_nan());
        // A negative length wraps into [0, |length|).
        assert_eq!(repeat(5.0_f32, -2.0), 1.0);
    }

    #[test]
    fn ping_pong_is_a_triangle_wave() {
        let expected = [0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 1.0];
        for (i, &e) in expected.iter().enumerate() {
            let t = i as F32 * 0.5;
            assert_eq!(ping_pong(t, 1.0), e, "t = {t}");
        }
        // Negative t mirrors.
        assert_eq!(ping_pong(-0.25_f32, 1.0), 0.25);
        assert_eq!(ping_pong(-1.5_f64, 1.0), 0.5);
        // Other lengths.
        assert_eq!(ping_pong(7.0_f64, 5.0), 3.0);
    }

    /// Runs the spring for `steps` frames and returns the final position,
    /// the final velocity and the largest position reached.
    fn run_damp(max_speed: Option<F64>, steps: usize) -> (F64, F64, F64) {
        let (mut x, mut v, mut peak) = (0.0_f64, 0.0_f64, 0.0_f64);
        for _ in 0..steps {
            x = match max_speed {
                Some(m) => smooth_damp_clamped(x, 10.0, &mut v, 0.3, m, 1.0 / 60.0),
                None => smooth_damp(x, 10.0, &mut v, 0.3, 1.0 / 60.0),
            };
            peak = peak.max(x);
        }
        (x, v, peak)
    }

    #[test]
    fn smooth_damp_converges_without_overshoot() {
        let (x, v, peak) = run_damp(None, 1200);
        assert!((x - 10.0).abs() < 1e-6, "x = {x}");
        assert!(v.abs() < 1e-6, "v = {v}");
        assert!(peak <= 10.0, "overshot to {peak}");
    }

    #[test]
    fn smooth_damp_moves_toward_the_target_from_both_sides() {
        let mut v = 0.0_f32;
        assert!(smooth_damp(0.0, 5.0, &mut v, 0.5, 0.016) > 0.0);
        let mut v = 0.0_f32;
        assert!(smooth_damp(0.0, -5.0, &mut v, 0.5, 0.016) < 0.0);
    }

    #[test]
    fn smooth_damp_at_the_target_stays_there() {
        let mut v = 0.0_f64;
        assert_eq!(smooth_damp(3.0, 3.0, &mut v, 0.2, 0.016), 3.0);
        assert_eq!(v, 0.0);
    }

    #[test]
    fn smooth_damp_zero_delta_time_changes_nothing() {
        let mut v = 0.5_f64;
        assert_eq!(smooth_damp(1.0, 9.0, &mut v, 0.3, 0.0), 1.0);
        assert_eq!(v, 0.5);
    }

    #[test]
    fn smooth_damp_clamped_limits_the_speed() {
        // Distance 10, max_speed 2: the spring needs at least 5 s, so after
        // 1 s (60 frames) it cannot have covered more than 2 units.
        let (x, _, _) = run_damp(Some(2.0), 60);
        assert!(x <= 2.0 + 1e-9, "x = {x}");
        // ...but it still gets there eventually, without overshoot.
        let (x, _, peak) = run_damp(Some(2.0), 2400);
        assert!((x - 10.0).abs() < 1e-6, "x = {x}");
        assert!(peak <= 10.0);
        // Negative max speeds use their absolute value.
        assert_eq!(run_damp(Some(-2.0), 60), run_damp(Some(2.0), 60));
    }

    #[test]
    fn smooth_damp_tiny_smooth_time_is_clamped_not_nan() {
        let mut v = 0.0_f32;
        let x = smooth_damp(0.0, 1.0, &mut v, 0.0, 0.016);
        assert!(x.is_finite() && (0.0..=1.0).contains(&x), "x = {x}");
        let mut v = 0.0_f32;
        assert!(smooth_damp(0.0, 1.0, &mut v, -3.0, 0.016).is_finite());
    }

    #[test]
    fn const_phase_2_1_versions() {
        const SMOOTHER: F32 = f32::smootherstep(0.0, 1.0, 0.25);
        const MOVED: F64 = f64::move_towards(0.0, 10.0, 3.0);
        assert_eq!(SMOOTHER, 0.103515625);
        assert_eq!(MOVED, 3.0);
        let xs: [F32; 8] = [-2.0, -0.5, 0.0, 0.25, 0.5, 1.0, 3.0, F32::NAN];
        for &x in &xs {
            let pairs = [
                (f32::smootherstep(0.0, 1.0, x), smootherstep(0.0, 1.0, x)),
                (f32::smootherstep(1.0, 0.0, x), smootherstep(1.0, 0.0, x)),
                (f32::smootherstep(0.5, 0.5, x), smootherstep(0.5, 0.5, x)),
                (f32::move_towards(0.0, x, 0.75), move_towards(0.0, x, 0.75)),
                (f32::move_towards(x, 1.0, 0.75), move_towards(x, 1.0, 0.75)),
                (f32::move_towards(1.0, 1.0, x), move_towards(1.0, 1.0, x)),
            ];
            for (c, g) in pairs {
                assert!(c == g || (c.is_nan() && g.is_nan()), "x = {x}: {c} vs {g}");
            }
        }
        for &y in &[-1.0_f64, 0.0, 0.3, 1.0, 4.0] {
            assert_eq!(f64::smootherstep(0.0, 1.0, y), smootherstep(0.0, 1.0, y));
            assert_eq!(f64::move_towards(y, 2.0, 0.5), move_towards(y, 2.0, 0.5));
        }
    }
}
