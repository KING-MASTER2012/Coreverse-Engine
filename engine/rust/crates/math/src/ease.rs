//! Easing curves: the classic Penner family (as catalogued on
//! <https://easings.net>) as plain functions `t -> value`.
//!
//! Every curve maps progress `t` in `[0, 1]` to a value that starts at `0`
//! and ends at exactly `1` (the endpoints are exact, not just close). Each
//! family comes in three flavours:
//!
//! - `*_in` starts slowly and speeds up,
//! - `*_out` starts fast and slows down, and is the mirror image of `_in`
//!   (`out(t) == 1 - in(1 - t)`, except for `elastic`),
//! - `*_in_out` does `_in` for the first half and `_out` for the second.
//!
//! | family    | shape                                                       |
//! |-----------|-------------------------------------------------------------|
//! | `quad`, `cubic`, `quart`, `quint` | `t²` .. `t⁵`; higher = sharper       |
//! | `sine`    | quarter / half cosine; the gentlest                         |
//! | `expo`    | exponential; very sharp                                     |
//! | `circ`    | quarter circle; sharp finish                                |
//! | `back`    | overshoots (dips below `0` / rises above `1`) then settles  |
//! | `elastic` | damped spring oscillation around the target                 |
//! | `bounce`  | bounces off the target like a dropped ball                  |
//!
//! plus [`linear`].
//!
//! # Inputs outside `[0, 1]`
//!
//! `t` is **not** clamped, because a clamp is one `.clamp(0.0, 1.0)` away and
//! not every caller wants it. What happens outside the range depends on the
//! family: the polynomial, `sine`, `back` and `expo` curves just continue
//! along their formula, `elastic` and `bounce` keep their usual shape
//! formulas (they do not repeat), and `circ` turns `NaN` once the square
//! root in its formula goes negative ([`circ_in`] beyond `±1`, [`circ_out`]
//! outside `[0, 2]`, [`circ_in_out`] outside `[-0.5, 2]`). A `NaN` `t`
//! gives `NaN`.
//!
//! ```
//! use math::ease::{cubic_in_out, quad_out};
//!
//! assert_eq!(quad_out(0.0_f32), 0.0);
//! assert_eq!(quad_out(0.5_f32), 0.75);
//! assert_eq!(cubic_in_out(0.5_f64), 0.5);
//! assert_eq!(cubic_in_out(1.0_f64), 1.0);
//!
//! // Ease between two values: feed the curve's output to `lerp`.
//! let x = math::interp::lerp(10.0_f32, 20.0, quad_out(0.5));
//! assert_eq!(x, 17.5);
//! ```
//!
//! All functions are generic over [`Scalar`], so they work for `f32` and
//! `f64`. They are not `const fn` (generic code cannot be, and several use
//! transcendental functions).

use crate::Scalar;

/// `x^n` by repeated multiplication, for the small constant `n`s below (the
/// compiler unrolls it).
#[inline]
fn int_pow<T: Scalar>(x: T, n: u32) -> T {
    let mut result = T::ONE;
    let mut i = 0;
    while i < n {
        result = result * x;
        i += 1;
    }
    result
}

/// A float constant of the right type.
#[inline]
fn k<T: Scalar>(value: f64) -> T {
    T::from_f64(value)
}

/// No easing: returns `t` unchanged. Handy as the "no curve" choice in code
/// that picks a curve at runtime.
#[inline]
pub fn linear<T: Scalar>(t: T) -> T {
    t
}

/// Generates the `_in` / `_out` / `_in_out` triple of a power-law curve
/// `t^n`. `$scale` is `2^(n - 1)`, the factor that makes the in-out curve
/// meet at `(0.5, 0.5)`.
macro_rules! power_curves {
    ($($family:literal $in_:ident $out:ident $in_out:ident $n:literal $scale:literal;)+) => {
        $(
            #[doc = concat!("Ease-in ", $family, ": `t^", stringify!($n), "`. Starts slowly.")]
            #[inline]
            pub fn $in_<T: Scalar>(t: T) -> T {
                int_pow(t, $n)
            }

            #[doc = concat!("Ease-out ", $family, ": `1 - (1 - t)^", stringify!($n), "`. Ends slowly.")]
            #[inline]
            pub fn $out<T: Scalar>(t: T) -> T {
                T::ONE - int_pow(T::ONE - t, $n)
            }

            #[doc = concat!("Ease-in-out ", $family, ": slow at both ends, fastest in the middle.")]
            #[inline]
            pub fn $in_out<T: Scalar>(t: T) -> T {
                if t < k(0.5) {
                    k::<T>($scale) * int_pow(t, $n)
                } else {
                    T::ONE - int_pow(k::<T>(2.0) - k::<T>(2.0) * t, $n) / k(2.0)
                }
            }
        )+
    };
}

power_curves! {
    "quadratic" quad_in quad_out quad_in_out 2 2.0;
    "cubic" cubic_in cubic_out cubic_in_out 3 4.0;
    "quartic" quart_in quart_out quart_in_out 4 8.0;
    "quintic" quint_in quint_out quint_in_out 5 16.0;
}

/// Ease-in sine: `1 - cos(t * π / 2)`. The gentlest in-curve.
///
/// Evaluated as `1 - sin((1 - t) * π / 2)` (the same curve), because that
/// form hits `0` and `1` exactly where `cos(π / 2)` is not exactly `0`.
#[inline]
pub fn sine_in<T: Scalar>(t: T) -> T {
    T::ONE - ((T::ONE - t) * T::PI / k(2.0)).sin()
}

/// Ease-out sine: `sin(t * π / 2)`.
#[inline]
pub fn sine_out<T: Scalar>(t: T) -> T {
    (t * T::PI / k(2.0)).sin()
}

/// Ease-in-out sine: `-(cos(π t) - 1) / 2`.
#[inline]
pub fn sine_in_out<T: Scalar>(t: T) -> T {
    (T::ZERO - ((t * T::PI).cos() - T::ONE)) / k(2.0)
}

/// Ease-in exponential: `2^(10 t - 10)`, with the endpoint `t = 0` pinned
/// to exactly `0` (the raw formula only reaches about `0.001`).
#[inline]
pub fn expo_in<T: Scalar>(t: T) -> T {
    if t == T::ZERO {
        T::ZERO
    } else {
        k::<T>(2.0).powf(k::<T>(10.0) * t - k(10.0))
    }
}

/// Ease-out exponential: `1 - 2^(-10 t)`, with `t = 1` pinned to exactly
/// `1`.
#[inline]
pub fn expo_out<T: Scalar>(t: T) -> T {
    if t == T::ONE {
        T::ONE
    } else {
        T::ONE - k::<T>(2.0).powf(k::<T>(-10.0) * t)
    }
}

/// Ease-in-out exponential, with both endpoints pinned exactly.
#[inline]
pub fn expo_in_out<T: Scalar>(t: T) -> T {
    if t == T::ZERO {
        T::ZERO
    } else if t == T::ONE {
        T::ONE
    } else if t < k(0.5) {
        k::<T>(2.0).powf(k::<T>(20.0) * t - k(10.0)) / k(2.0)
    } else {
        (k::<T>(2.0) - k::<T>(2.0).powf(k::<T>(-20.0) * t + k(10.0))) / k(2.0)
    }
}

/// Ease-in circular: `1 - sqrt(1 - t²)`. `NaN` beyond `±1`.
#[inline]
pub fn circ_in<T: Scalar>(t: T) -> T {
    T::ONE - (T::ONE - t * t).sqrt()
}

/// Ease-out circular: `sqrt(1 - (t - 1)²)`. `NaN` outside `[0, 2]`.
#[inline]
pub fn circ_out<T: Scalar>(t: T) -> T {
    let u = t - T::ONE;
    (T::ONE - u * u).sqrt()
}

/// Ease-in-out circular. `NaN` outside `[-0.5, 2]`.
#[inline]
pub fn circ_in_out<T: Scalar>(t: T) -> T {
    if t < k(0.5) {
        let u = k::<T>(2.0) * t;
        (T::ONE - (T::ONE - u * u).sqrt()) / k(2.0)
    } else {
        let u = k::<T>(2.0) - k::<T>(2.0) * t;
        ((T::ONE - u * u).sqrt() + T::ONE) / k(2.0)
    }
}

/// The classic overshoot amount (about 10%) of the `back` curves.
const BACK_OVERSHOOT: f64 = 1.70158;

/// Ease-in back: pulls back below `0` (by about 10%) before accelerating
/// towards `1`.
///
/// The textbook `(c1 + 1) t³ - c1 t²`, factored as `t² (t + c1 (t - 1))` so
/// that `t = 1` gives exactly `1` instead of `(c1 + 1) - c1`, which rounds.
#[inline]
pub fn back_in<T: Scalar>(t: T) -> T {
    let c1 = k::<T>(BACK_OVERSHOOT);
    t * t * (t + c1 * (t - T::ONE))
}

/// Ease-out back: overshoots past `1` (by about 10%) and settles back.
/// The exact mirror of [`back_in`], so both endpoints are exact.
#[inline]
pub fn back_out<T: Scalar>(t: T) -> T {
    T::ONE - back_in(T::ONE - t)
}

/// Ease-in-out back: pulls back, then overshoots, then settles (a softer
/// overshoot than the one-sided curves, about 5% each way).
#[inline]
pub fn back_in_out<T: Scalar>(t: T) -> T {
    let c2 = k::<T>(BACK_OVERSHOOT * 1.525);
    if t < k(0.5) {
        let u = k::<T>(2.0) * t;
        (u * u * ((c2 + T::ONE) * u - c2)) / k(2.0)
    } else {
        let u = k::<T>(2.0) * t - k(2.0);
        (u * u * ((c2 + T::ONE) * u + c2) + k(2.0)) / k(2.0)
    }
}

/// Ease-in elastic: a growing spring oscillation that snaps to `1`. Has
/// exact endpoints.
#[inline]
pub fn elastic_in<T: Scalar>(t: T) -> T {
    if t == T::ZERO {
        T::ZERO
    } else if t == T::ONE {
        T::ONE
    } else {
        let c4 = T::TAU / k(3.0);
        T::ZERO
            - k::<T>(2.0).powf(k::<T>(10.0) * t - k(10.0)) * ((t * k(10.0) - k(10.75)) * c4).sin()
    }
}

/// Ease-out elastic: shoots to `1`, then rings around it with decaying
/// amplitude. Not the exact mirror of [`elastic_in`] (the phase is the
/// standard one from easings.net).
#[inline]
pub fn elastic_out<T: Scalar>(t: T) -> T {
    if t == T::ZERO {
        T::ZERO
    } else if t == T::ONE {
        T::ONE
    } else {
        let c4 = T::TAU / k(3.0);
        k::<T>(2.0).powf(k::<T>(-10.0) * t) * ((t * k(10.0) - k(0.75)) * c4).sin() + T::ONE
    }
}

/// Ease-in-out elastic: oscillates in at the start, rings out at the end.
#[inline]
pub fn elastic_in_out<T: Scalar>(t: T) -> T {
    if t == T::ZERO {
        T::ZERO
    } else if t == T::ONE {
        T::ONE
    } else {
        let c5 = T::TAU / k(4.5);
        let wave = ((t * k(20.0) - k(11.125)) * c5).sin();
        if t < k(0.5) {
            T::ZERO - k::<T>(2.0).powf(k::<T>(20.0) * t - k(10.0)) * wave / k(2.0)
        } else {
            k::<T>(2.0).powf(k::<T>(-20.0) * t + k(10.0)) * wave / k(2.0) + T::ONE
        }
    }
}

/// Ease-out bounce: like a ball dropped onto the target, bouncing back up
/// with decreasing height; four arcs, the first reaching `1` at `t = 1/2.75`.
#[inline]
pub fn bounce_out<T: Scalar>(t: T) -> T {
    let n1 = k::<T>(7.5625);
    let d1 = k::<T>(2.75);
    if t < T::ONE / d1 {
        n1 * t * t
    } else if t < k::<T>(2.0) / d1 {
        let u = t - k::<T>(1.5) / d1;
        n1 * u * u + k(0.75)
    } else if t < k::<T>(2.5) / d1 {
        let u = t - k::<T>(2.25) / d1;
        n1 * u * u + k(0.9375)
    } else {
        let u = t - k::<T>(2.625) / d1;
        n1 * u * u + k(0.984375)
    }
}

/// Ease-in bounce: `1 - bounce_out(1 - t)`; bounces away from `0` with
/// increasing height.
#[inline]
pub fn bounce_in<T: Scalar>(t: T) -> T {
    T::ONE - bounce_out(T::ONE - t)
}

/// Ease-in-out bounce: bounces away from `0`, then bounces onto `1`.
#[inline]
pub fn bounce_in_out<T: Scalar>(t: T) -> T {
    if t < k(0.5) {
        (T::ONE - bounce_out(T::ONE - k::<T>(2.0) * t)) / k(2.0)
    } else {
        (T::ONE + bounce_out(k::<T>(2.0) * t - T::ONE)) / k(2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Curve = (&'static str, fn(f64) -> f64);
    type Mirrored = (&'static str, fn(f64) -> f64, fn(f64) -> f64);

    /// Every curve, as `f64` function pointers.
    const ALL: [Curve; 31] = [
        ("linear", linear::<f64>),
        ("quad_in", quad_in::<f64>),
        ("quad_out", quad_out::<f64>),
        ("quad_in_out", quad_in_out::<f64>),
        ("cubic_in", cubic_in::<f64>),
        ("cubic_out", cubic_out::<f64>),
        ("cubic_in_out", cubic_in_out::<f64>),
        ("quart_in", quart_in::<f64>),
        ("quart_out", quart_out::<f64>),
        ("quart_in_out", quart_in_out::<f64>),
        ("quint_in", quint_in::<f64>),
        ("quint_out", quint_out::<f64>),
        ("quint_in_out", quint_in_out::<f64>),
        ("sine_in", sine_in::<f64>),
        ("sine_out", sine_out::<f64>),
        ("sine_in_out", sine_in_out::<f64>),
        ("expo_in", expo_in::<f64>),
        ("expo_out", expo_out::<f64>),
        ("expo_in_out", expo_in_out::<f64>),
        ("circ_in", circ_in::<f64>),
        ("circ_out", circ_out::<f64>),
        ("circ_in_out", circ_in_out::<f64>),
        ("back_in", back_in::<f64>),
        ("back_out", back_out::<f64>),
        ("back_in_out", back_in_out::<f64>),
        ("elastic_in", elastic_in::<f64>),
        ("elastic_out", elastic_out::<f64>),
        ("elastic_in_out", elastic_in_out::<f64>),
        ("bounce_in", bounce_in::<f64>),
        ("bounce_out", bounce_out::<f64>),
        ("bounce_in_out", bounce_in_out::<f64>),
    ];

    /// `(in, out)` pairs that must mirror each other exactly.
    const MIRRORED: [Mirrored; 10] = [
        ("quad", quad_in::<f64>, quad_out::<f64>),
        ("cubic", cubic_in::<f64>, cubic_out::<f64>),
        ("quart", quart_in::<f64>, quart_out::<f64>),
        ("quint", quint_in::<f64>, quint_out::<f64>),
        ("sine", sine_in::<f64>, sine_out::<f64>),
        ("expo", expo_in::<f64>, expo_out::<f64>),
        ("circ", circ_in::<f64>, circ_out::<f64>),
        ("back", back_in::<f64>, back_out::<f64>),
        ("bounce", bounce_in::<f64>, bounce_out::<f64>),
        ("linear", linear::<f64>, linear::<f64>),
    ];

    /// The `_in_out` curves (and `linear`), which are symmetric about
    /// `(0.5, 0.5)`.
    const IN_OUT: [Curve; 11] = [
        ("linear", linear::<f64>),
        ("quad_in_out", quad_in_out::<f64>),
        ("cubic_in_out", cubic_in_out::<f64>),
        ("quart_in_out", quart_in_out::<f64>),
        ("quint_in_out", quint_in_out::<f64>),
        ("sine_in_out", sine_in_out::<f64>),
        ("expo_in_out", expo_in_out::<f64>),
        ("circ_in_out", circ_in_out::<f64>),
        ("back_in_out", back_in_out::<f64>),
        ("elastic_in_out", elastic_in_out::<f64>),
        ("bounce_in_out", bounce_in_out::<f64>),
    ];

    /// Curves that never go backwards.
    const MONOTONIC: [&str; 19] = [
        "linear",
        "quad_in",
        "quad_out",
        "quad_in_out",
        "cubic_in",
        "cubic_out",
        "cubic_in_out",
        "quart_in",
        "quart_out",
        "quart_in_out",
        "quint_in",
        "quint_out",
        "quint_in_out",
        "sine_in",
        "sine_out",
        "sine_in_out",
        "expo_in",
        "expo_out",
        "expo_in_out",
    ];

    fn near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    /// 1001 evenly spaced samples of `[0, 1]`.
    fn sweep() -> impl Iterator<Item = f64> {
        (0..=1000).map(|i| f64::from(i) / 1000.0)
    }

    #[test]
    fn endpoints_are_exact() {
        // Exactly 0 and 1 (`assert_eq!`, no tolerance) for every curve.
        for (name, f) in ALL {
            assert_eq!(f(0.0), 0.0, "{name}(0) = {:e}", f(0.0));
            assert_eq!(f(1.0), 1.0, "{name}(1) - 1 = {:e}", f(1.0) - 1.0);
        }
    }

    #[test]
    fn endpoints_are_exact_in_f32() {
        let f32_curves: [fn(f32) -> f32; 31] = [
            linear::<f32>,
            quad_in::<f32>,
            quad_out::<f32>,
            quad_in_out::<f32>,
            cubic_in::<f32>,
            cubic_out::<f32>,
            cubic_in_out::<f32>,
            quart_in::<f32>,
            quart_out::<f32>,
            quart_in_out::<f32>,
            quint_in::<f32>,
            quint_out::<f32>,
            quint_in_out::<f32>,
            sine_in::<f32>,
            sine_out::<f32>,
            sine_in_out::<f32>,
            expo_in::<f32>,
            expo_out::<f32>,
            expo_in_out::<f32>,
            circ_in::<f32>,
            circ_out::<f32>,
            circ_in_out::<f32>,
            back_in::<f32>,
            back_out::<f32>,
            back_in_out::<f32>,
            elastic_in::<f32>,
            elastic_out::<f32>,
            elastic_in_out::<f32>,
            bounce_in::<f32>,
            bounce_out::<f32>,
            bounce_in_out::<f32>,
        ];
        for (i, f) in f32_curves.iter().enumerate() {
            assert_eq!(f(0.0), 0.0, "curve {i}: f(0) = {:e}", f(0.0));
            assert_eq!(f(1.0), 1.0, "curve {i}: f(1) - 1 = {:e}", f(1.0) - 1.0);
        }
    }

    #[test]
    fn in_out_curves_pass_through_the_centre() {
        for (name, f) in IN_OUT {
            assert!(near(f(0.5), 0.5, 1e-12), "{name}(0.5) = {}", f(0.5));
        }
    }

    #[test]
    fn in_out_curves_are_point_symmetric() {
        for (name, f) in IN_OUT {
            for i in 0..=100 {
                let t = f64::from(i) / 100.0;
                assert!(
                    near(f(1.0 - t), 1.0 - f(t), 1e-9),
                    "{name}: f(1 - {t}) = {}, 1 - f({t}) = {}",
                    f(1.0 - t),
                    1.0 - f(t)
                );
            }
        }
    }

    #[test]
    fn out_curves_mirror_their_in_curves() {
        for (name, in_, out) in MIRRORED {
            for i in 0..=100 {
                let t = f64::from(i) / 100.0;
                assert!(
                    near(out(t), 1.0 - in_(1.0 - t), 1e-9),
                    "{name}: out({t}) = {}, 1 - in(1 - {t}) = {}",
                    out(t),
                    1.0 - in_(1.0 - t)
                );
            }
        }
    }

    #[test]
    fn monotonic_curves_never_decrease() {
        for (name, f) in ALL {
            if !MONOTONIC.contains(&name) {
                continue;
            }
            let mut previous = f(0.0);
            for i in 1..=2000 {
                let value = f(f64::from(i) / 2000.0);
                assert!(value >= previous - 1e-12, "{name} decreased at step {i}");
                previous = value;
            }
        }
        // circ is monotonic too, but only defined on [0, 1].
        for f in [circ_in::<f64>, circ_out::<f64>, circ_in_out::<f64>] {
            let mut previous = f(0.0);
            for i in 1..=2000 {
                let value = f(f64::from(i) / 2000.0);
                assert!(value >= previous - 1e-12);
                previous = value;
            }
        }
    }

    #[test]
    fn known_midpoint_values() {
        assert_eq!(quad_in(0.5_f64), 0.25);
        assert_eq!(quad_out(0.5_f64), 0.75);
        assert_eq!(cubic_in(0.5_f64), 0.125);
        assert_eq!(cubic_out(0.5_f64), 0.875);
        assert_eq!(quart_in(0.5_f64), 0.0625);
        assert_eq!(quint_in(0.5_f64), 0.03125);
        assert_eq!(quad_in_out(0.25_f64), 0.125);
        assert_eq!(cubic_in_out(0.25_f64), 0.0625);
        assert!(near(
            sine_in(0.5_f64),
            1.0 - core::f64::consts::FRAC_1_SQRT_2,
            1e-12
        ));
        assert!(near(
            sine_out(0.5_f64),
            core::f64::consts::FRAC_1_SQRT_2,
            1e-12
        ));
        assert!(near(circ_out(0.5_f64), Scalar::sqrt(0.75_f64), 1e-12));
        assert!(near(expo_in(0.5_f64), 0.03125, 1e-12));
        assert!(near(expo_out(0.5_f64), 0.96875, 1e-12));
        assert_eq!(quad_in(0.5_f32), 0.25);
    }

    #[test]
    fn bounce_out_touches_one_at_each_arc_boundary_start() {
        // The first arc ends exactly at 1.0 when t = 1 / 2.75.
        assert!(near(bounce_out(1.0 / 2.75), 1.0, 1e-12));
        // Between bounces it comes back down: the troughs of the later arcs.
        assert!(near(bounce_out(1.5 / 2.75), 0.75, 1e-12));
        assert!(near(bounce_out(2.25 / 2.75), 0.9375, 1e-12));
        assert!(near(bounce_out(2.625 / 2.75), 0.984375, 1e-12));
        // And it is never above 1.
        for i in 0..=1000 {
            assert!(bounce_out(f64::from(i) / 1000.0) <= 1.0 + 1e-12);
        }
    }

    #[test]
    fn back_curves_overshoot() {
        let min = |f: fn(f64) -> f64| sweep().map(f).fold(f64::MAX, f64::min);
        let max = |f: fn(f64) -> f64| sweep().map(f).fold(f64::MIN, f64::max);
        // ~10% for the one-sided curves, ~5% for in-out.
        assert!(near(min(back_in), -0.1, 0.01), "{}", min(back_in));
        assert!(near(max(back_out), 1.1, 0.01), "{}", max(back_out));
        assert!(min(back_in_out) < -0.05 && max(back_in_out) > 1.05);
        assert!(max(back_in) <= 1.0 + 1e-12);
        assert!(min(back_out) >= -1e-12);
    }

    #[test]
    fn elastic_curves_ring_around_the_target() {
        // Rings: overshoots 1 and undershoots 1 on the way.
        assert!(sweep().any(|t| elastic_out(t) > 1.0));
        assert!(sweep().any(|t| elastic_out(t) < 1.0 && t > 0.3));
        assert!(sweep().any(|t| elastic_in(t) < 0.0));
        assert!(sweep().any(|t| elastic_in_out(t) < 0.0));
        assert!(sweep().any(|t| elastic_in_out(t) > 1.0));
    }

    #[test]
    fn circ_is_nan_outside_its_domain() {
        assert!(circ_in(1.5_f64).is_nan());
        assert!(circ_in(-1.5_f64).is_nan());
        assert!(circ_out(-0.5_f64).is_nan());
        assert!(circ_out(2.5_f64).is_nan());
        assert!(circ_in_out(2.5_f32).is_nan());
        assert!(circ_in_out(-1.0_f32).is_nan());
    }

    #[test]
    fn nan_in_gives_nan_out() {
        for (name, f) in ALL {
            assert!(f(f64::NAN).is_nan(), "{name}(NaN) should be NaN");
        }
    }

    #[test]
    fn polynomial_curves_extrapolate_outside_the_unit_range() {
        assert_eq!(quad_in(2.0_f64), 4.0);
        assert_eq!(quad_in(-2.0_f64), 4.0);
        assert_eq!(cubic_in(-1.0_f64), -1.0);
        assert_eq!(linear(7.5_f32), 7.5);
    }

    #[test]
    fn f32_matches_f64() {
        for i in 0..=20_i16 {
            let t = f32::from(i) / 20.0;
            let wide = f64::from(t);
            let pairs = [
                (f64::from(quad_in_out(t)), quad_in_out(wide)),
                (f64::from(sine_in_out(t)), sine_in_out(wide)),
                (f64::from(expo_out(t)), expo_out(wide)),
                (f64::from(circ_in_out(t)), circ_in_out(wide)),
                (f64::from(back_in_out(t)), back_in_out(wide)),
                (f64::from(elastic_out(t)), elastic_out(wide)),
                (f64::from(bounce_in_out(t)), bounce_in_out(wide)),
            ];
            for (n, (narrow, w)) in pairs.into_iter().enumerate() {
                assert!(near(narrow, w, 1e-4), "curve {n}, t = {t}: {narrow} vs {w}");
            }
        }
    }
}
