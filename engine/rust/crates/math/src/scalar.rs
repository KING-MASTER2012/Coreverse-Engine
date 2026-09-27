//! The internal `Scalar` trait: the minimal common surface `math`'s
//! generic functions are written against, so the same code works for both
//! `f32` and `f64` without duplicating it per type.
//!
//! This trait grows across Phase 1 rather than being finished in one go:
//! this file adds only what [`crate::consts`] and [`crate::approx`] need
//! right now (the constants below, plus [`Scalar::abs`]); Phase 1.3 adds
//! `min`/`max`/`clamp`/`sign`/`signum`/`floor`/`ceil`/`round`/`modulo` to
//! this same trait rather than introducing a second one.

use core::fmt::Debug;
use core::ops::{Mul, Sub};

/// A floating-point scalar (`f32` or `f64`) that `math`'s generic
/// functions are written against.
///
/// Not meant to be implemented for anything outside this crate: the
/// constants below (`PI`, `DEG_TO_RAD`, ...) are only meaningful for
/// floating-point angles and tolerances, unlike the separate integer
/// trait `int_math` introduces in Phase 1.6.
pub trait Scalar:
    Copy + PartialEq + PartialOrd + Debug + Sub<Output = Self> + Mul<Output = Self>
{
    /// Additive identity, `0`.
    const ZERO: Self;
    /// Multiplicative identity, `1`.
    const ONE: Self;
    /// π.
    const PI: Self;
    /// 2π (a full turn in radians).
    const TAU: Self;
    /// Euler's number.
    const E: Self;
    /// Multiply a value in degrees by this to get radians (`PI / 180`).
    const DEG_TO_RAD: Self;
    /// Multiply a value in radians by this to get degrees (`180 / PI`).
    const RAD_TO_DEG: Self;
    /// This type's machine epsilon - the default tolerance
    /// [`crate::approx::approx_eq`]/[`crate::approx::approx_zero`] compare
    /// against. Reach for [`crate::approx::approx_eq_eps`] or
    /// [`crate::approx::approx_eq_rel`] with a looser tolerance once a
    /// comparison has accumulated more than a couple of operations' worth
    /// of rounding error.
    const DEFAULT_EPSILON: Self;

    /// The absolute value.
    fn abs(self) -> Self;
}

impl Scalar for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const PI: Self = core::f32::consts::PI;
    const TAU: Self = core::f32::consts::TAU;
    const E: Self = core::f32::consts::E;
    const DEG_TO_RAD: Self = Self::PI / 180.0;
    const RAD_TO_DEG: Self = 180.0 / Self::PI;
    const DEFAULT_EPSILON: Self = f32::EPSILON;

    fn abs(self) -> Self {
        f32::abs(self)
    }
}

impl Scalar for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const PI: Self = core::f64::consts::PI;
    const TAU: Self = core::f64::consts::TAU;
    const E: Self = core::f64::consts::E;
    const DEG_TO_RAD: Self = Self::PI / 180.0;
    const RAD_TO_DEG: Self = 180.0 / Self::PI;
    const DEFAULT_EPSILON: Self = f64::EPSILON;

    fn abs(self) -> Self {
        f64::abs(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_deg_rad_round_trip() {
        let deg = 180.0_f32;
        let rad = deg * f32::DEG_TO_RAD;
        assert!((rad - core::f32::consts::PI).abs() < 1e-6);
        assert!((rad * f32::RAD_TO_DEG - deg).abs() < 1e-4);
    }

    #[test]
    fn f64_deg_rad_round_trip() {
        let deg = 180.0_f64;
        let rad = deg * f64::DEG_TO_RAD;
        assert!((rad - core::f64::consts::PI).abs() < 1e-12);
        assert!((rad * f64::RAD_TO_DEG - deg).abs() < 1e-9);
    }

    #[test]
    fn abs_matches_std() {
        assert_eq!(Scalar::abs(-3.5_f32), 3.5_f32);
        assert_eq!(Scalar::abs(-2.25_f64), 2.25_f64);
    }

    #[test]
    fn tau_is_two_pi() {
        assert_eq!(f32::TAU, f32::PI * 2.0);
        assert_eq!(f64::TAU, f64::PI * 2.0);
    }
}
