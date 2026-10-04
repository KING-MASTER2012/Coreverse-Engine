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
//! Phase 1.4 (`interp`) did not add methods, only the `Add`/`Div`
//! supertraits it needs (`lerp` is `a + (b - a) * t`, `inverse_lerp` divides).
//!
//! Phase 1.5 adds the transcendental surface `trig` is built on:
//! [`Scalar::sin`]/[`Scalar::cos`]/[`Scalar::tan`]/[`Scalar::asin`]/
//! [`Scalar::acos`]/[`Scalar::atan2`]/[`Scalar::sqrt`]/[`Scalar::powf`]
//! and the [`Scalar::NAN`] constant. Like `floor` and friends they exist on
//! `f32`/`f64` only with `std`, so they go through the same private `sys`
//! backend below. The public, typed API for them lives in `crate::trig`.
//!
//! Phase 2.1 adds [`Scalar::fract`]/[`Scalar::trunc`]/[`Scalar::exp`]/
//! [`Scalar::ln`]/[`Scalar::log2`]/[`Scalar::log10`]/[`Scalar::atan`]/
//! [`Scalar::hypot`]/[`Scalar::cbrt`]/[`Scalar::mul_add`]/
//! [`Scalar::copysign`]/[`Scalar::is_finite`]/[`Scalar::is_nan`], plus
//! [`Scalar::from_f64`] so generic code (the `ease` curves, `smooth_damp`)
//! can write its float constants once.
//!
//! Every method here (like [`Scalar::abs`] before it) is a thin,
//! `#[inline]` wrapper. `abs`/`min`/`max`/`clamp`/`signum` exist on the
//! primitive `f32`/`f64` types in `core`, so they need no backend.
//! `floor`/`ceil`/`round`/`rem_euclid` do not - they only exist with
//! `std` - so those four go through the private `sys` module below, which
//! calls the primitive methods under the `std` feature and the `libm`
//! crate otherwise.

use core::fmt::Debug;
use core::ops::{Add, Div, Mul, Sub};

/// Backend for the operations `core` lacks (rounding, Euclidean remainder
/// and everything transcendental): primitive methods under `std`, the
/// `libm` crate otherwise.
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
    #[inline]
    pub(super) fn sin_f32(x: f32) -> f32 {
        x.sin()
    }
    #[inline]
    pub(super) fn cos_f32(x: f32) -> f32 {
        x.cos()
    }
    #[inline]
    pub(super) fn tan_f32(x: f32) -> f32 {
        x.tan()
    }
    #[inline]
    pub(super) fn asin_f32(x: f32) -> f32 {
        x.asin()
    }
    #[inline]
    pub(super) fn acos_f32(x: f32) -> f32 {
        x.acos()
    }
    #[inline]
    pub(super) fn sqrt_f32(x: f32) -> f32 {
        x.sqrt()
    }
    #[inline]
    pub(super) fn atan2_f32(y: f32, x: f32) -> f32 {
        y.atan2(x)
    }
    #[inline]
    pub(super) fn powf_f32(x: f32, y: f32) -> f32 {
        x.powf(y)
    }
    #[inline]
    pub(super) fn sin_f64(x: f64) -> f64 {
        x.sin()
    }
    #[inline]
    pub(super) fn cos_f64(x: f64) -> f64 {
        x.cos()
    }
    #[inline]
    pub(super) fn tan_f64(x: f64) -> f64 {
        x.tan()
    }
    #[inline]
    pub(super) fn asin_f64(x: f64) -> f64 {
        x.asin()
    }
    #[inline]
    pub(super) fn acos_f64(x: f64) -> f64 {
        x.acos()
    }
    #[inline]
    pub(super) fn sqrt_f64(x: f64) -> f64 {
        x.sqrt()
    }
    #[inline]
    pub(super) fn atan2_f64(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
    #[inline]
    pub(super) fn powf_f64(x: f64, y: f64) -> f64 {
        x.powf(y)
    }

    /// Generates `fn name(args..) -> ty` forwarding to a primitive method.
    macro_rules! forward {
        ($($name:ident($($arg:ident),+): $ty:ty => $path:path;)+) => {
            $(
                #[inline]
                pub(super) fn $name($($arg: $ty),+) -> $ty {
                    $path($($arg),+)
                }
            )+
        };
    }

    forward! {
        trunc_f32(x): f32 => f32::trunc;
        exp_f32(x): f32 => f32::exp;
        ln_f32(x): f32 => f32::ln;
        log2_f32(x): f32 => f32::log2;
        log10_f32(x): f32 => f32::log10;
        atan_f32(x): f32 => f32::atan;
        cbrt_f32(x): f32 => f32::cbrt;
        hypot_f32(x, y): f32 => f32::hypot;
        copysign_f32(x, y): f32 => f32::copysign;
        mul_add_f32(x, a, b): f32 => f32::mul_add;
        trunc_f64(x): f64 => f64::trunc;
        exp_f64(x): f64 => f64::exp;
        ln_f64(x): f64 => f64::ln;
        log2_f64(x): f64 => f64::log2;
        log10_f64(x): f64 => f64::log10;
        atan_f64(x): f64 => f64::atan;
        cbrt_f64(x): f64 => f64::cbrt;
        hypot_f64(x, y): f64 => f64::hypot;
        copysign_f64(x, y): f64 => f64::copysign;
        mul_add_f64(x, a, b): f64 => f64::mul_add;
    }
}

/// `libm::round*` rounds ties away from zero, same as `f32`/`f64::round`.
/// `rem_euclid` is rebuilt from `fmod` exactly the way std defines it.
/// `libm` names the `f32` versions with an `f` suffix (`sinf`, `powf`...).
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
    #[inline]
    pub(super) fn sin_f32(x: f32) -> f32 {
        libm::sinf(x)
    }
    #[inline]
    pub(super) fn cos_f32(x: f32) -> f32 {
        libm::cosf(x)
    }
    #[inline]
    pub(super) fn tan_f32(x: f32) -> f32 {
        libm::tanf(x)
    }
    #[inline]
    pub(super) fn asin_f32(x: f32) -> f32 {
        libm::asinf(x)
    }
    #[inline]
    pub(super) fn acos_f32(x: f32) -> f32 {
        libm::acosf(x)
    }
    #[inline]
    pub(super) fn sqrt_f32(x: f32) -> f32 {
        libm::sqrtf(x)
    }
    #[inline]
    pub(super) fn atan2_f32(y: f32, x: f32) -> f32 {
        libm::atan2f(y, x)
    }
    #[inline]
    pub(super) fn powf_f32(x: f32, y: f32) -> f32 {
        libm::powf(x, y)
    }
    #[inline]
    pub(super) fn sin_f64(x: f64) -> f64 {
        libm::sin(x)
    }
    #[inline]
    pub(super) fn cos_f64(x: f64) -> f64 {
        libm::cos(x)
    }
    #[inline]
    pub(super) fn tan_f64(x: f64) -> f64 {
        libm::tan(x)
    }
    #[inline]
    pub(super) fn asin_f64(x: f64) -> f64 {
        libm::asin(x)
    }
    #[inline]
    pub(super) fn acos_f64(x: f64) -> f64 {
        libm::acos(x)
    }
    #[inline]
    pub(super) fn sqrt_f64(x: f64) -> f64 {
        libm::sqrt(x)
    }
    #[inline]
    pub(super) fn atan2_f64(y: f64, x: f64) -> f64 {
        libm::atan2(y, x)
    }
    #[inline]
    pub(super) fn powf_f64(x: f64, y: f64) -> f64 {
        libm::pow(x, y)
    }

    /// Generates `fn name(args..) -> ty` forwarding to a `libm` function.
    macro_rules! forward {
        ($($name:ident($($arg:ident),+): $ty:ty => $path:path;)+) => {
            $(
                #[inline]
                pub(super) fn $name($($arg: $ty),+) -> $ty {
                    $path($($arg),+)
                }
            )+
        };
    }

    forward! {
        trunc_f32(x): f32 => libm::truncf;
        exp_f32(x): f32 => libm::expf;
        ln_f32(x): f32 => libm::logf;
        log2_f32(x): f32 => libm::log2f;
        log10_f32(x): f32 => libm::log10f;
        atan_f32(x): f32 => libm::atanf;
        cbrt_f32(x): f32 => libm::cbrtf;
        hypot_f32(x, y): f32 => libm::hypotf;
        copysign_f32(x, y): f32 => libm::copysignf;
        mul_add_f32(x, a, b): f32 => libm::fmaf;
        trunc_f64(x): f64 => libm::trunc;
        exp_f64(x): f64 => libm::exp;
        ln_f64(x): f64 => libm::log;
        log2_f64(x): f64 => libm::log2;
        log10_f64(x): f64 => libm::log10;
        atan_f64(x): f64 => libm::atan;
        cbrt_f64(x): f64 => libm::cbrt;
        hypot_f64(x, y): f64 => libm::hypot;
        copysign_f64(x, y): f64 => libm::copysign;
        mul_add_f64(x, a, b): f64 => libm::fma;
    }
}

/// A floating-point scalar (`f32` or `f64`) that `math`'s generic
/// functions are written against.
///
/// Not meant to be implemented for anything outside this crate: the
/// constants below (`PI`, `DEG_TO_RAD`, ...) are only meaningful for
/// floating-point angles and tolerances, unlike the separate integer
/// trait [`crate::Integer`].
pub trait Scalar:
Copy
+ PartialEq
+ PartialOrd
+ Debug
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
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
    /// Not a number. What the domain-restricted functions in `crate::trig`
    /// (`asin(2)`, `nth_root(-4, 2)`, ...) return for input they have no
    /// real answer for.
    const NAN: Self;
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

    /// The sine of `self`, in radians.
    fn sin(self) -> Self;

    /// The cosine of `self`, in radians.
    fn cos(self) -> Self;

    /// The tangent of `self`, in radians.
    fn tan(self) -> Self;

    /// The arcsine of `self`, in radians within `[-PI/2, PI/2]`. `NaN` if
    /// `self` is outside `[-1, 1]`.
    fn asin(self) -> Self;

    /// The arccosine of `self`, in radians within `[0, PI]`. `NaN` if
    /// `self` is outside `[-1, 1]`.
    fn acos(self) -> Self;

    /// The four-quadrant arctangent of `self` (the `y` coordinate) and
    /// `x`, in radians within `[-PI, PI]`. The angle of the point `(x, y)`
    /// measured counterclockwise from the positive `x` axis.
    fn atan2(self, x: Self) -> Self;

    /// The square root of `self`. `NaN` if `self` is negative.
    fn sqrt(self) -> Self;

    /// `self` raised to the (floating-point) power `exp`.
    fn powf(self, exp: Self) -> Self;

    /// `self` with its fractional part removed (rounded towards zero):
    /// `3.7 -> 3.0`, `-3.7 -> -3.0`. Differs from [`Scalar::floor`] for
    /// negative numbers.
    fn trunc(self) -> Self;

    /// The fractional part of `self`, `self - self.trunc()`. Keeps `self`'s
    /// sign (`-3.75 -> -0.75`), so it is **not** the GLSL `fract`; for that
    /// use `self - self.floor()`.
    fn fract(self) -> Self;

    /// `e` raised to the power `self`.
    fn exp(self) -> Self;

    /// The natural logarithm of `self`. `NaN` if `self` is negative,
    /// `-inf` for zero.
    fn ln(self) -> Self;

    /// The base-2 logarithm of `self`. `NaN` if `self` is negative, `-inf`
    /// for zero.
    fn log2(self) -> Self;

    /// The base-10 logarithm of `self`. `NaN` if `self` is negative,
    /// `-inf` for zero.
    fn log10(self) -> Self;

    /// The arctangent of `self`, in radians within `[-PI/2, PI/2]`. Use
    /// [`Scalar::atan2`] when you have both coordinates.
    fn atan(self) -> Self;

    /// The length of the hypotenuse of a right triangle with legs `self`
    /// and `other`, `sqrt(self² + other²)`, without intermediate overflow
    /// or underflow.
    fn hypot(self, other: Self) -> Self;

    /// The real cube root of `self`. Unlike `powf(1/3)` it is defined for
    /// negative numbers (`cbrt(-8) == -2`).
    fn cbrt(self) -> Self;

    /// The fused multiply-add `self * a + b` with a single rounding.
    ///
    /// Exact, but only fast where the target has a hardware FMA
    /// instruction: elsewhere (and under `no_std`) it falls back to a
    /// software implementation that is much slower than a plain `self * a
    /// + b`. Reach for it for accuracy, not as a blanket speed-up.
    fn mul_add(self, a: Self, b: Self) -> Self;

    /// `self`'s magnitude with `sign`'s sign bit (so `-0.0` counts as
    /// negative, and a `NaN` `sign` is whatever its sign bit says).
    fn copysign(self, sign: Self) -> Self;

    /// `true` if `self` is neither infinite nor `NaN`.
    fn is_finite(self) -> bool;

    /// `true` if `self` is `NaN`.
    fn is_nan(self) -> bool;

    /// Converts a `f64` constant to this type, rounding to nearest for
    /// `f32` (out-of-range values become `inf`). Exists so generic code can
    /// write its float constants (`0.48`, `1.70158`) once instead of
    /// building them out of `ONE`s.
    fn from_f64(value: f64) -> Self;
}

impl Scalar for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const PI: Self = core::f32::consts::PI;
    const TAU: Self = core::f32::consts::TAU;
    const E: Self = core::f32::consts::E;
    const DEG_TO_RAD: Self = Self::PI / 180.0;
    const RAD_TO_DEG: Self = 180.0 / Self::PI;
    const NAN: Self = f32::NAN;
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

    #[inline]
    fn sin(self) -> Self {
        sys::sin_f32(self)
    }

    #[inline]
    fn cos(self) -> Self {
        sys::cos_f32(self)
    }

    #[inline]
    fn tan(self) -> Self {
        sys::tan_f32(self)
    }

    #[inline]
    fn asin(self) -> Self {
        sys::asin_f32(self)
    }

    #[inline]
    fn acos(self) -> Self {
        sys::acos_f32(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        sys::sqrt_f32(self)
    }

    #[inline]
    fn atan2(self, x: Self) -> Self {
        sys::atan2_f32(self, x)
    }

    #[inline]
    fn powf(self, exp: Self) -> Self {
        sys::powf_f32(self, exp)
    }

    #[inline]
    fn trunc(self) -> Self {
        sys::trunc_f32(self)
    }

    #[inline]
    fn fract(self) -> Self {
        self - sys::trunc_f32(self)
    }

    #[inline]
    fn exp(self) -> Self {
        sys::exp_f32(self)
    }

    #[inline]
    fn ln(self) -> Self {
        sys::ln_f32(self)
    }

    #[inline]
    fn log2(self) -> Self {
        sys::log2_f32(self)
    }

    #[inline]
    fn log10(self) -> Self {
        sys::log10_f32(self)
    }

    #[inline]
    fn atan(self) -> Self {
        sys::atan_f32(self)
    }

    #[inline]
    fn hypot(self, other: Self) -> Self {
        sys::hypot_f32(self, other)
    }

    #[inline]
    fn cbrt(self) -> Self {
        sys::cbrt_f32(self)
    }

    #[inline]
    fn mul_add(self, a: Self, b: Self) -> Self {
        sys::mul_add_f32(self, a, b)
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        sys::copysign_f32(self, sign)
    }

    #[inline]
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }

    #[inline]
    fn is_nan(self) -> bool {
        f32::is_nan(self)
    }

    #[inline]
    fn from_f64(value: f64) -> Self {
        value as f32
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
    const NAN: Self = f64::NAN;
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

    #[inline]
    fn sin(self) -> Self {
        sys::sin_f64(self)
    }

    #[inline]
    fn cos(self) -> Self {
        sys::cos_f64(self)
    }

    #[inline]
    fn tan(self) -> Self {
        sys::tan_f64(self)
    }

    #[inline]
    fn asin(self) -> Self {
        sys::asin_f64(self)
    }

    #[inline]
    fn acos(self) -> Self {
        sys::acos_f64(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        sys::sqrt_f64(self)
    }

    #[inline]
    fn atan2(self, x: Self) -> Self {
        sys::atan2_f64(self, x)
    }

    #[inline]
    fn powf(self, exp: Self) -> Self {
        sys::powf_f64(self, exp)
    }

    #[inline]
    fn trunc(self) -> Self {
        sys::trunc_f64(self)
    }

    #[inline]
    fn fract(self) -> Self {
        self - sys::trunc_f64(self)
    }

    #[inline]
    fn exp(self) -> Self {
        sys::exp_f64(self)
    }

    #[inline]
    fn ln(self) -> Self {
        sys::ln_f64(self)
    }

    #[inline]
    fn log2(self) -> Self {
        sys::log2_f64(self)
    }

    #[inline]
    fn log10(self) -> Self {
        sys::log10_f64(self)
    }

    #[inline]
    fn atan(self) -> Self {
        sys::atan_f64(self)
    }

    #[inline]
    fn hypot(self, other: Self) -> Self {
        sys::hypot_f64(self, other)
    }

    #[inline]
    fn cbrt(self) -> Self {
        sys::cbrt_f64(self)
    }

    #[inline]
    fn mul_add(self, a: Self, b: Self) -> Self {
        sys::mul_add_f64(self, a, b)
    }

    #[inline]
    fn copysign(self, sign: Self) -> Self {
        sys::copysign_f64(self, sign)
    }

    #[inline]
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }

    #[inline]
    fn is_nan(self) -> bool {
        f64::is_nan(self)
    }

    #[inline]
    fn from_f64(value: f64) -> Self {
        value
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
    fn nan_constant_is_nan() {
        assert!(f32::NAN.is_nan());
        assert!(<f64 as Scalar>::NAN.is_nan());
    }

    #[test]
    fn transcendental_methods_match_known_values() {
        assert!((Scalar::sin(f32::PI / 2.0) - 1.0).abs() < 1e-6);
        assert!((Scalar::cos(0.0_f64) - 1.0).abs() < 1e-12);
        assert!((Scalar::tan(f64::PI / 4.0) - 1.0).abs() < 1e-12);
        assert!((Scalar::asin(1.0_f32) - f32::PI / 2.0).abs() < 1e-6);
        assert!((Scalar::acos(1.0_f64)).abs() < 1e-12);
        assert!((Scalar::atan2(1.0_f64, 1.0) - f64::PI / 4.0).abs() < 1e-12);
        assert_eq!(Scalar::sqrt(16.0_f32), 4.0_f32);
        assert_eq!(Scalar::powf(2.0_f64, 8.0), 256.0_f64);
        assert!(Scalar::sqrt(-1.0_f32).is_nan());
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

    #[test]
    fn trunc_and_fract() {
        assert_eq!(Scalar::trunc(3.7_f32), 3.0);
        assert_eq!(Scalar::trunc(-3.7_f32), -3.0);
        assert_eq!(Scalar::trunc(-3.7_f64), -3.0);
        assert_eq!(Scalar::fract(3.75_f32), 0.75);
        // Keeps the sign of the input, unlike GLSL's fract.
        assert_eq!(Scalar::fract(-3.75_f64), -0.75);
        assert_eq!(Scalar::fract(5.0_f64), 0.0);
        assert!(Scalar::fract(f32::INFINITY).is_nan());
    }

    #[test]
    fn exp_and_logs_match_known_values() {
        assert_eq!(Scalar::exp(0.0_f32), 1.0);
        assert!((Scalar::exp(1.0_f64) - f64::E).abs() < 1e-12);
        assert!((Scalar::ln(f32::E) - 1.0).abs() < 1e-6);
        assert_eq!(Scalar::ln(1.0_f64), 0.0);
        assert_eq!(Scalar::log2(8.0_f32), 3.0);
        assert_eq!(Scalar::log2(0.25_f64), -2.0);
        assert!((Scalar::log10(1000.0_f64) - 3.0).abs() < 1e-12);
        assert!((Scalar::log10(100.0_f32) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn logs_out_of_domain() {
        assert!(Scalar::ln(-1.0_f32).is_nan());
        assert!(Scalar::log2(-1.0_f64).is_nan());
        assert!(Scalar::log10(-1.0_f32).is_nan());
        assert_eq!(Scalar::ln(0.0_f32), f32::NEG_INFINITY);
        assert_eq!(Scalar::log2(0.0_f64), f64::NEG_INFINITY);
    }

    #[test]
    fn atan_hypot_cbrt() {
        assert_eq!(Scalar::atan(0.0_f32), 0.0);
        assert!((Scalar::atan(1.0_f64) - f64::PI / 4.0).abs() < 1e-12);
        assert!((Scalar::atan(f32::INFINITY) - f32::PI / 2.0).abs() < 1e-6);
        assert_eq!(Scalar::hypot(3.0_f32, 4.0), 5.0);
        assert_eq!(Scalar::hypot(-3.0_f64, 4.0), 5.0);
        // The point of hypot: no intermediate overflow.
        assert_eq!(
            Scalar::hypot(1.0e30_f32, 1.0e30),
            1.0e30 * core::f32::consts::SQRT_2
        );
        assert_eq!(Scalar::cbrt(27.0_f32), 3.0);
        assert_eq!(Scalar::cbrt(-8.0_f64), -2.0);
    }

    #[test]
    fn mul_add_is_a_fused_multiply_add() {
        assert_eq!(Scalar::mul_add(2.0_f32, 3.0, 4.0), 10.0);
        assert_eq!(Scalar::mul_add(-2.0_f64, 3.0, 1.0), -5.0);
        // A single rounding: (1 + e)(1 - e) - 1 is exactly -e*e only when fused.
        let e = f64::EPSILON;
        assert_eq!(Scalar::mul_add(1.0 + e, 1.0 - e, -1.0), -e * e);
    }

    #[test]
    fn copysign_uses_the_sign_bit() {
        assert_eq!(Scalar::copysign(3.0_f32, -1.0), -3.0);
        assert_eq!(Scalar::copysign(-3.0_f64, 1.0), 3.0);
        assert_eq!(Scalar::copysign(3.0_f32, -0.0), -3.0);
        assert_eq!(
            Scalar::copysign(0.0_f64, -1.0).to_bits(),
            (-0.0_f64).to_bits()
        );
    }

    #[test]
    fn finite_and_nan_predicates() {
        assert!(Scalar::is_finite(1.0_f32));
        assert!(!Scalar::is_finite(f32::INFINITY));
        assert!(!Scalar::is_finite(f64::NAN));
        assert!(Scalar::is_nan(f64::NAN));
        assert!(!Scalar::is_nan(f32::INFINITY));
        assert!(!Scalar::is_nan(0.0_f32));
    }

    #[test]
    fn from_f64_converts_constants() {
        assert_eq!(<f64 as Scalar>::from_f64(0.48), 0.48);
        assert_eq!(<f32 as Scalar>::from_f64(0.5), 0.5);
        assert_eq!(<f32 as Scalar>::from_f64(0.1), 0.1_f32);
        assert_eq!(<f32 as Scalar>::from_f64(f64::INFINITY), f32::INFINITY);
    }
}
