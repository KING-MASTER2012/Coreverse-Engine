//! Parametric curves for animation, paths and procedural geometry (Phase
//! 2.7): Bézier curves ([`quadratic_bezier`], [`cubic_bezier`]), [`hermite()`]
//! interpolation, [`catmull_rom()`] splines, B-splines ([`cubic_b_spline`],
//! [`BSpline`]) and arc-length tools ([`arc_length_gauss`],
//! [`ArcLengthTable`]).
//!
//! Everything is `no_std`-compatible and allocation-free: curves with a
//! variable number of control points ([`catmull_rom_spline`], [`BSpline`])
//! borrow slices, and [`ArcLengthTable`] stores its samples in a const-sized
//! array.
//!
//! # Points
//!
//! Every function is generic over a [`CurvePoint`]: a type that can be added,
//! subtracted and scaled by a scalar, and has a length. That covers `f32` and
//! `f64` (a curve through numbers, e.g. a volume envelope), `Vec2`, `Vec3`
//! and `Vec4`, and their `f64` counterparts `DVec2`, `DVec3` and `DVec4`. You
//! can implement the trait for your own types (a colour, say) too.
//!
//! ```
//! use math::Vec3;
//! use math::curve::{cubic_bezier, hermite};
//!
//! let p0 = Vec3::new(0.0, 0.0, 0.0);
//! let p3 = Vec3::new(4.0, 0.0, 0.0);
//! let mid = cubic_bezier(p0, Vec3::new(1.0, 2.0, 0.0), Vec3::new(3.0, 2.0, 0.0), p3, 0.5);
//! assert_eq!(mid, Vec3::new(2.0, 1.5, 0.0));
//!
//! // A plain `f32` is a point too.
//! assert_eq!(hermite(0.0, 0.0, 1.0, 0.0, 0.5_f32), 0.5);
//! ```
//!
//! # Conventions
//!
//! - **The parameter `t`** runs from `0` (start) to `1` (end) on every
//!   single-segment curve. It is not clamped: values outside `0..=1`
//!   extrapolate the polynomial.
//! - **End points are exact.** `t == 0` and `t == 1` return the start and end
//!   point without rounding error (for the interpolating curves; a B-spline
//!   does not pass through its control points).
//! - **Derivatives** are taken with respect to `t`, so they are velocities
//!   (the curve's speed depends on how the control points are spaced), not
//!   unit tangents. Divide by [`CurvePoint::length`] for a direction.
//! - **Arc length** is not proportional to `t`. To move at constant speed,
//!   measure the curve with [`ArcLengthTable`] and map distance to `t`.
//! - **Invalid input never panics.** Functions that can be given inputs
//!   with no answer (a spline with too few points, a parameter outside the
//!   domain) return `None`.
//!
//! # Which curve?
//!
//! | Curve | Passes through | Continuity | Use it for |
//! | --- | --- | --- | --- |
//! | Bézier | first and last control point | the segment | UI easing paths, vector shapes |
//! | Hermite | both end points, with given tangents | the segment | keyframes with explicit velocities |
//! | Catmull-Rom | every point | `C1` | camera/animation paths through waypoints |
//! | B-spline | none (it smooths) | `C2` for cubic | smooth, wiggle-free motion |

mod arc_length;
mod b_spline;
mod bezier;
mod catmull_rom;
mod hermite;

use core::ops::{Add, Mul, Sub};

use crate::scalar::Scalar;

pub use arc_length::{ArcLengthTable, arc_length_gauss, arc_length_polyline};
pub use b_spline::{
    BSpline, MAX_B_SPLINE_DEGREE, cubic_b_spline, cubic_b_spline_derivative,
    fill_clamped_uniform_knots,
};
pub use bezier::{
    CubicBezier, QuadraticBezier, cubic_bezier, cubic_bezier_derivative,
    cubic_bezier_second_derivative, cubic_bezier_split, quadratic_bezier,
    quadratic_bezier_derivative, quadratic_bezier_second_derivative, quadratic_bezier_split,
};
pub use catmull_rom::{catmull_rom, catmull_rom_alpha, catmull_rom_derivative, catmull_rom_spline};
pub use hermite::{bezier_to_hermite, hermite, hermite_derivative, hermite_to_bezier};

/// A point (or value) a curve can pass through: something that can be added,
/// subtracted and scaled by the scalar `S`, and that has a length.
///
/// Implemented for `f32`, `f64` and every float vector (`Vec2`..`Vec4`,
/// `DVec2`..`DVec4`). The scalar is a type parameter, not an associated type,
/// so that `Vec3: CurvePoint<f32>` and `DVec3: CurvePoint<f64>` are separate,
/// unambiguous facts; the curve functions infer `S` from the point type.
///
/// Implement it for your own types to run any curve through them:
///
/// ```
/// use core::ops::{Add, Mul, Sub};
/// use math::curve::{CurvePoint, quadratic_bezier};
///
/// /// A grey level, as a curve point.
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// struct Grey(f32);
///
/// impl Add for Grey { type Output = Grey; fn add(self, r: Grey) -> Grey { Grey(self.0 + r.0) } }
/// impl Sub for Grey { type Output = Grey; fn sub(self, r: Grey) -> Grey { Grey(self.0 - r.0) } }
/// impl Mul<f32> for Grey { type Output = Grey; fn mul(self, s: f32) -> Grey { Grey(self.0 * s) } }
/// impl CurvePoint<f32> for Grey { fn length(self) -> f32 { self.0.abs() } }
///
/// let mid = quadratic_bezier(Grey(0.0), Grey(1.0), Grey(0.0), 0.5_f32);
/// assert_eq!(mid, Grey(0.5));
/// ```
pub trait CurvePoint<S: Scalar>:
    Copy + Add<Output = Self> + Sub<Output = Self> + Mul<S, Output = Self>
{
    /// The Euclidean length of `self`, seen as a vector from the origin. For a
    /// scalar this is its absolute value. Arc-length functions measure a
    /// curve by the length of differences between its points.
    fn length(self) -> S;
}

impl CurvePoint<f32> for f32 {
    #[inline]
    fn length(self) -> f32 {
        Scalar::abs(self)
    }
}

impl CurvePoint<f64> for f64 {
    #[inline]
    fn length(self) -> f64 {
        Scalar::abs(self)
    }
}

/// Implements [`CurvePoint`] for float vector types: `$vector => $scalar`.
macro_rules! curve_point_for_vectors {
    ($($vector:ty => $scalar:ty),+ $(,)?) => {
        $(
            impl CurvePoint<$scalar> for $vector {
                #[inline]
                fn length(self) -> $scalar {
                    <$vector>::length(self)
                }
            }
        )+
    };
}

curve_point_for_vectors! {
    crate::Vec2 => f32,
    crate::Vec3 => f32,
    crate::Vec4 => f32,
    crate::DVec2 => f64,
    crate::DVec3 => f64,
    crate::DVec4 => f64,
}

/// The constant `value` as the scalar `S`: stands in for the float literals
/// generic code cannot write.
#[inline]
fn lit<S: Scalar>(value: f64) -> S {
    S::from_f64(value)
}

/// The point `a + (b - a) * t`, the linear interpolation between two points.
#[inline]
fn lerp_point<S: Scalar, P: CurvePoint<S>>(a: P, b: P, t: S) -> P {
    a + (b - a) * t
}

#[cfg(test)]
mod tests;
