//! The internal `Scalar` trait: the minimal common surface `math`'s
//! generic functions are written against, so the same code works for both
//! `f32` and `f64` without duplicating it per type.
//!
//! This trait grows across Phase 1 rather than being finished in one go.
//! Phase 1.2 added the constants and [`Scalar::abs`]; this phase (1.3)
//! adds [`Scalar::min`]/[`Scalar::max`]/[`Scalar::clamp`]/[`Scalar::sign`]/
//! [`Scalar::signum`]/[`Scalar::floor`]/[`Scalar::ceil`]/[`Scalar::round`]/
//! [`Scalar::modulo`] to this same trait.
//!
//! Every method here (like [`Scalar::abs`] before it) is a thin,
//! `#[inline]` wrapper. `abs`/`min`/`max`/`clamp`/`signum` exist on the
//! primitive `f32`/`f64` types in `core`, so they need no backend.
//! `floor`/`ceil`/`round`/`rem_euclid` do not - they only exist with
//! `std` - so those four go through the private `sys` module below, which
//! calls the primitive methods under the `std` feature and the `libm`
//! crate otherwise (the same split Phase 1.5's `trig` module uses).

use core::fmt::Debug;
use core::ops::{Mul, Sub};

/// Backend for the four operations `core` lacks: primitive methods under
/// `std`, the `libm` crate otherwise.
#[cfg(feature = "std")]
mod sys {
    pub(super) fn floor_f32(x: f32) -> f32 {
        x.floor()
    }
    pub(super) fn ceil_f32(x: f32) -> f32 {
        x.ceil()
    }
    pub(super) fn round_f32(x: f32) -> f32 {
        x.round()
    }
    pub(super) fn rem_euclid_f32(x: f32, y: f32) -> f32 {
        x.rem_euclid(y)
    }
    pub(super) fn floor_f64(x: f64) -> f64 {
        x.floor()
    }
    pub(super) fn ceil_f64(x: f64) -> f64 {
        x.ceil()
    }
    pub(super) fn round_f64(x: f64) -> f64 {
        x.round()
    }
    pub(super) fn rem_euclid_f64(x: f64, y: f64) -> f64 {
        x.rem_euclid(y)
    }
}

/// `libm::round*` rounds ties away from zero, same as `f32`/`f64::round`.
/// `rem_euclid` is rebuilt from `fmod` exactly the way std defines it.
#[cfg(not(feature = "std"))]
mod sys {
    pub(super) fn floor_f32(x: f32) -> f32 {
        libm::floorf(x)
    }
    pub(super) fn ceil_f32(x: f32) -> f32 {
        libm::ceilf(x)
    }
    pub(super) fn round_f32(x: f32) -> f32 {
        libm::roundf(x)
    }
    pub(super) fn rem_euclid_f32(x: f32, y: f32) -> f32 {
        let r = libm::fmodf(x, y);
        if r < 0.0 { r + y.abs() } else { r }
    }
    pub(super) fn floor_f64(x: f64) -> f64 {
        libm::floor(x)
    }
    pub(super) fn ceil_f64(x: f64) -> f64 {
        libm::ceil(x)
    }
    pub(super) fn round_f64(x: f64) -> f64 {
        libm::round(x)
    }
    pub(super) fn rem_euclid_f64(x: f64, y: f64) -> f64 {
        let r = libm::fmod(x, y);
        if r < 0.0 { r + y.abs() } else { r }
    }
}

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

    /// The smaller of `self` and `other`. If either is `NaN`, returns the
    /// other (matches `f32`/`f64::min`, unlike the `<`/`>` operators).
    fn min(self, other: Self) -> Self;

    /// The larger of `self` and `other`. If either is `NaN`, returns the
    /// other (matches `f32`/`f64::max`).
    fn max(self, other: Self) -> Self;

    /// Restricts `self` to the inclusive range `[lo, hi]`. Panics in debug
    /// builds (matches `f32`/`f64::clamp`) if `lo > hi`, or if either bound
    /// is `NaN`.
    fn clamp(self, lo: Self, hi: Self) -> Self;

    /// The three-state sign of `self`: `-1` if negative, `1` if positive,
    /// `0` if exactly zero (`+0.0` or `-0.0`). See [`Scalar::signum`] for
    /// the IEEE-754 variant that never returns zero.
    fn sign(self) -> Self;

    /// The IEEE-754 sign of `self`: `1.0` if positive (including `+0.0`),
    /// `-1.0` if negative (including `-0.0`), `NaN` if `self` is `NaN`.
    /// Never returns zero - see [`Scalar::sign`] for the three-state
    /// version that does.
    fn signum(self) -> Self;

    /// The largest integer (as this same float type) less than or equal
    /// to `self`.
    fn floor(self) -> Self;

    /// The smallest integer (as this same float type) greater than or
    /// equal to `self`.
    fn ceil(self) -> Self;

    /// `self` rounded to the nearest integer, ties rounding away from
    /// zero (`2.5 -> 3.0`, `-2.5 -> -3.0`) - matches `f32`/`f64::round`.
    fn round(self) -> Self;

    /// Euclidean remainder of `self / other`: always in `[0, other.abs())`
    /// regardless of either operand's sign, unlike `%`/`Rem`, which keeps
    /// `self`'s sign and can return a negative result.
    fn modulo(self, other: Self) -> Self;
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

    #[inline]
    fn abs(self) -> Self {
        f32::abs(self)
    }

    #[inline]
    fn min(self, other: Self) -> Self {
        f32::min(self, other)
    }

    #[inline]
    fn max(self, other: Self) -> Self {
        f32::max(self, other)
    }

    #[inline]
    fn clamp(self, lo: Self, hi: Self) -> Self {
        f32::clamp(self, lo, hi)
    }

    #[inline]
    fn sign(self) -> Self {
        if self == Self::ZERO {
            Self::ZERO
        } else {
            self.signum()
        }
    }

    #[inline]
    fn signum(self) -> Self {
        f32::signum(self)
    }

    #[inline]
    fn floor(self) -> Self {
        sys::floor_f32(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        sys::ceil_f32(self)
    }

    #[inline]
    fn round(self) -> Self {
        sys::round_f32(self)
    }

    #[inline]
    fn modulo(self, other: Self) -> Self {
        sys::rem_euclid_f32(self, other)
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

    #[inline]
    fn abs(self) -> Self {
        f64::abs(self)
    }

    #[inline]
    fn min(self, other: Self) -> Self {
        f64::min(self, other)
    }

    #[inline]
    fn max(self, other: Self) -> Self {
        f64::max(self, other)
    }

    #[inline]
    fn clamp(self, lo: Self, hi: Self) -> Self {
        f64::clamp(self, lo, hi)
    }

    #[inline]
    fn sign(self) -> Self {
        if self == Self::ZERO {
            Self::ZERO
        } else {
            self.signum()
        }
    }

    #[inline]
    fn signum(self) -> Self {
        f64::signum(self)
    }

    #[inline]
    fn floor(self) -> Self {
        sys::floor_f64(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        sys::ceil_f64(self)
    }

    #[inline]
    fn round(self) -> Self {
        sys::round_f64(self)
    }

    #[inline]
    fn modulo(self, other: Self) -> Self {
        sys::rem_euclid_f64(self, other)
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

    #[test]
    fn min_max() {
        assert_eq!(Scalar::min(3.0_f32, 5.0_f32), 3.0_f32);
        assert_eq!(Scalar::max(3.0_f32, 5.0_f32), 5.0_f32);
        // NaN propagation follows f32/f64::min/max: the non-NaN operand wins.
        assert_eq!(Scalar::min(f32::NAN, 5.0_f32), 5.0_f32);
        assert_eq!(Scalar::max(f32::NAN, 5.0_f32), 5.0_f32);
    }

    #[test]
    fn clamp() {
        assert_eq!(Scalar::clamp(5.0_f32, 0.0, 10.0), 5.0_f32);
        assert_eq!(Scalar::clamp(-5.0_f32, 0.0, 10.0), 0.0_f32);
        assert_eq!(Scalar::clamp(15.0_f32, 0.0, 10.0), 10.0_f32);
    }

    #[test]
    fn sign_is_three_state() {
        assert_eq!(Scalar::sign(5.0_f32), 1.0_f32);
        assert_eq!(Scalar::sign(-5.0_f32), -1.0_f32);
        assert_eq!(Scalar::sign(0.0_f32), 0.0_f32);
        assert_eq!(Scalar::sign(-0.0_f32), 0.0_f32);
    }

    #[test]
    fn signum_is_ieee_and_never_zero() {
        assert_eq!(Scalar::signum(5.0_f32), 1.0_f32);
        assert_eq!(Scalar::signum(-5.0_f32), -1.0_f32);
        assert_eq!(Scalar::signum(0.0_f32), 1.0_f32);
        assert_eq!(Scalar::signum(-0.0_f32), -1.0_f32);
    }

    #[test]
    fn floor_ceil_round() {
        assert_eq!(Scalar::floor(1.7_f32), 1.0_f32);
        assert_eq!(Scalar::floor(-1.2_f32), -2.0_f32);
        assert_eq!(Scalar::ceil(1.2_f32), 2.0_f32);
        assert_eq!(Scalar::ceil(-1.7_f32), -1.0_f32);
        assert_eq!(Scalar::round(2.5_f32), 3.0_f32);
        assert_eq!(Scalar::round(-2.5_f32), -3.0_f32);
    }

    #[test]
    fn modulo_is_euclidean() {
        // Matches `%` when both operands are positive...
        assert_eq!(Scalar::modulo(5.0_f32, 3.0_f32), 2.0_f32);
        // ...but stays non-negative when `self` is negative, unlike `%`.
        assert_eq!(Scalar::modulo(-5.0_f32, 3.0_f32), 1.0_f32);
        assert_eq!(Scalar::modulo(5.0_f64, 3.0_f64), 2.0_f64);
        assert_eq!(Scalar::modulo(-5.0_f64, 3.0_f64), 1.0_f64);
    }
}
