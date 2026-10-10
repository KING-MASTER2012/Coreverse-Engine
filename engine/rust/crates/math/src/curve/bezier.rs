//! Quadratic and cubic Bézier curves: evaluation, derivatives, splitting and
//! the [`QuadraticBezier`]/[`CubicBezier`] structs that bundle them.

use super::arc_length::{ArcLengthTable, arc_length_gauss};
use super::hermite::{bezier_to_hermite, hermite_to_bezier};
use super::{CurvePoint, lerp_point, lit};
use crate::scalar::Scalar;

/// How many Gauss-Legendre panels [`QuadraticBezier::length`] and
/// [`CubicBezier::length`] use. Each panel evaluates the derivative at five
/// points, which is far more than enough for a Bézier segment.
const LENGTH_PANELS: u32 = 8;

/// The point at `t` on the quadratic Bézier curve with control points `p0`,
/// `p1` and `p2`: `(1 - t)^2 p0 + 2 (1 - t) t p1 + t^2 p2`.
///
/// The curve starts at `p0`, ends at `p2` and is tangent to `p1 - p0` and
/// `p2 - p1` there; `p1` itself is not on the curve. The result is exactly
/// `p0` at `t == 0` and exactly `p2` at `t == 1`.
///
/// ```
/// use math::Vec2;
/// use math::curve::quadratic_bezier;
///
/// let (p0, p1, p2) = (Vec2::new(0.0, 0.0), Vec2::new(1.0, 2.0), Vec2::new(2.0, 0.0));
/// assert_eq!(quadratic_bezier(p0, p1, p2, 0.0), p0);
/// assert_eq!(quadratic_bezier(p0, p1, p2, 0.5), Vec2::new(1.0, 1.0));
/// assert_eq!(quadratic_bezier(p0, p1, p2, 1.0), p2);
/// ```
#[inline]
pub fn quadratic_bezier<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, t: S) -> P {
    let u = S::ONE - t;
    p0 * (u * u) + p1 * (lit::<S>(2.0) * u * t) + p2 * (t * t)
}

/// The first derivative (the velocity) of [`quadratic_bezier`] at `t`:
/// `2 ((1 - t) (p1 - p0) + t (p2 - p1))`.
#[inline]
pub fn quadratic_bezier_derivative<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, t: S) -> P {
    let u = S::ONE - t;
    ((p1 - p0) * u + (p2 - p1) * t) * lit::<S>(2.0)
}

/// The second derivative (the acceleration) of [`quadratic_bezier`]:
/// `2 (p0 - 2 p1 + p2)`. A quadratic curve's acceleration is constant, so
/// there is no `t`.
#[inline]
pub fn quadratic_bezier_second_derivative<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P) -> P {
    let two = lit::<S>(2.0);
    (p0 - p1 * two + p2) * two
}

/// Splits the quadratic Bézier curve at `t` into two quadratic curves (de
/// Casteljau's algorithm), returned as their control points.
///
/// The left curve covers the original's `0..=t`, the right one `t..=1`; they
/// share their middle point, which is the original curve's point at `t`.
///
/// ```
/// use math::Vec2;
/// use math::curve::{quadratic_bezier, quadratic_bezier_split};
///
/// let (p0, p1, p2) = (Vec2::new(0.0, 0.0), Vec2::new(1.0, 2.0), Vec2::new(2.0, 0.0));
/// let ([a0, a1, a2], [b0, b1, b2]) = quadratic_bezier_split(p0, p1, p2, 0.5);
/// assert_eq!(a2, b0);
/// assert_eq!(a2, quadratic_bezier(p0, p1, p2, 0.5));
/// assert_eq!((a0, b2), (p0, p2));
/// # let _ = (a1, b1);
/// ```
#[inline]
pub fn quadratic_bezier_split<S: Scalar, P: CurvePoint<S>>(
    p0: P,
    p1: P,
    p2: P,
    t: S,
) -> ([P; 3], [P; 3]) {
    let a = lerp_point(p0, p1, t);
    let b = lerp_point(p1, p2, t);
    let mid = lerp_point(a, b, t);
    ([p0, a, mid], [mid, b, p2])
}

/// The point at `t` on the cubic Bézier curve with control points `p0`..`p3`:
/// `(1 - t)^3 p0 + 3 (1 - t)^2 t p1 + 3 (1 - t) t^2 p2 + t^3 p3`.
///
/// The curve starts at `p0`, ends at `p3` and leaves/arrives in the
/// direction of `p1 - p0` and `p3 - p2`. The result is exactly `p0` at
/// `t == 0` and exactly `p3` at `t == 1`.
///
/// ```
/// use math::Vec2;
/// use math::curve::cubic_bezier;
///
/// let p = [
///     Vec2::new(0.0, 0.0),
///     Vec2::new(0.0, 1.0),
///     Vec2::new(1.0, 1.0),
///     Vec2::new(1.0, 0.0),
/// ];
/// assert_eq!(cubic_bezier(p[0], p[1], p[2], p[3], 0.0), p[0]);
/// assert_eq!(cubic_bezier(p[0], p[1], p[2], p[3], 0.5), Vec2::new(0.5, 0.75));
/// assert_eq!(cubic_bezier(p[0], p[1], p[2], p[3], 1.0), p[3]);
/// ```
#[inline]
pub fn cubic_bezier<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, p3: P, t: S) -> P {
    let u = S::ONE - t;
    let three = lit::<S>(3.0);
    p0 * (u * u * u) + p1 * (three * u * u * t) + p2 * (three * u * t * t) + p3 * (t * t * t)
}

/// The first derivative (the velocity) of [`cubic_bezier`] at `t`:
/// `3 ((1 - t)^2 (p1 - p0) + 2 (1 - t) t (p2 - p1) + t^2 (p3 - p2))`.
#[inline]
pub fn cubic_bezier_derivative<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, p3: P, t: S) -> P {
    let u = S::ONE - t;
    ((p1 - p0) * (u * u) + (p2 - p1) * (lit::<S>(2.0) * u * t) + (p3 - p2) * (t * t))
        * lit::<S>(3.0)
}

/// The second derivative (the acceleration) of [`cubic_bezier`] at `t`:
/// `6 ((1 - t) (p0 - 2 p1 + p2) + t (p1 - 2 p2 + p3))`.
#[inline]
pub fn cubic_bezier_second_derivative<S: Scalar, P: CurvePoint<S>>(
    p0: P,
    p1: P,
    p2: P,
    p3: P,
    t: S,
) -> P {
    let u = S::ONE - t;
    let two = lit::<S>(2.0);
    ((p0 - p1 * two + p2) * u + (p1 - p2 * two + p3) * t) * lit::<S>(6.0)
}

/// Splits the cubic Bézier curve at `t` into two cubic curves (de Casteljau's
/// algorithm), returned as their control points.
///
/// The left curve covers the original's `0..=t`, the right one `t..=1`; they
/// share their middle point, which is the original curve's point at `t`.
#[inline]
pub fn cubic_bezier_split<S: Scalar, P: CurvePoint<S>>(
    p0: P,
    p1: P,
    p2: P,
    p3: P,
    t: S,
) -> ([P; 4], [P; 4]) {
    let a = lerp_point(p0, p1, t);
    let b = lerp_point(p1, p2, t);
    let c = lerp_point(p2, p3, t);
    let d = lerp_point(a, b, t);
    let e = lerp_point(b, c, t);
    let mid = lerp_point(d, e, t);
    ([p0, a, d, mid], [mid, e, c, p3])
}

/// A quadratic Bézier curve: its three control points, with the evaluation,
/// derivative, splitting and length functions as methods.
///
/// ```
/// use math::Vec2;
/// use math::curve::QuadraticBezier;
///
/// let curve = QuadraticBezier::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(2.0, 0.0));
/// assert_eq!(curve.point(0.5), Vec2::new(1.0, 0.5));
/// // sqrt(2) + asinh(1): the arc length of this parabola.
/// assert!((curve.length() - 2.295_587).abs() < 1e-4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QuadraticBezier<P> {
    /// The start of the curve (the curve passes through it at `t == 0`).
    pub p0: P,
    /// The control point, which shapes the curve but is not on it.
    pub p1: P,
    /// The end of the curve (the curve passes through it at `t == 1`).
    pub p2: P,
}

impl<P> QuadraticBezier<P> {
    /// The curve with control points `p0`, `p1` and `p2`.
    #[inline]
    pub const fn new(p0: P, p1: P, p2: P) -> Self {
        Self { p0, p1, p2 }
    }
}

impl<P: Copy> QuadraticBezier<P> {
    /// The control points as an array `[p0, p1, p2]`.
    #[inline]
    pub const fn to_array(self) -> [P; 3] {
        [self.p0, self.p1, self.p2]
    }

    /// The point at `t`. See [`quadratic_bezier`].
    #[inline]
    pub fn point<S: Scalar>(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        quadratic_bezier(self.p0, self.p1, self.p2, t)
    }

    /// The first derivative at `t`. See [`quadratic_bezier_derivative`].
    #[inline]
    pub fn derivative<S: Scalar>(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        quadratic_bezier_derivative(self.p0, self.p1, self.p2, t)
    }

    /// The (constant) second derivative. See
    /// [`quadratic_bezier_second_derivative`].
    #[inline]
    pub fn second_derivative<S: Scalar>(&self) -> P
    where
        P: CurvePoint<S>,
    {
        quadratic_bezier_second_derivative(self.p0, self.p1, self.p2)
    }

    /// Splits the curve at `t` into the curves covering `0..=t` and `t..=1`.
    /// See [`quadratic_bezier_split`].
    #[inline]
    pub fn split<S: Scalar>(&self, t: S) -> (Self, Self)
    where
        P: CurvePoint<S>,
    {
        let ([a0, a1, a2], [b0, b1, b2]) = quadratic_bezier_split(self.p0, self.p1, self.p2, t);
        (Self::new(a0, a1, a2), Self::new(b0, b1, b2))
    }

    /// The arc length of the whole curve, by Gauss-Legendre quadrature of its
    /// speed. Accurate to about the precision of `S` for any ordinary curve.
    #[inline]
    pub fn length<S: Scalar>(&self) -> S
    where
        P: CurvePoint<S>,
    {
        self.length_between(S::ZERO, S::ONE)
    }

    /// The arc length of the part of the curve between the parameters `t0`
    /// and `t1` (in either order).
    #[inline]
    pub fn length_between<S: Scalar>(&self, t0: S, t1: S) -> S
    where
        P: CurvePoint<S>,
    {
        arc_length_gauss(|t| self.derivative(t), t0, t1, LENGTH_PANELS)
    }

    /// A table of the curve's cumulative arc length at `N` evenly spaced
    /// parameters, for constant-speed movement along it. See
    /// [`ArcLengthTable`].
    #[inline]
    pub fn arc_length_table<S: Scalar, const N: usize>(&self) -> ArcLengthTable<S, N>
    where
        P: CurvePoint<S>,
    {
        ArcLengthTable::from_derivative(|t| self.derivative(t), S::ZERO, S::ONE)
    }
}

/// A cubic Bézier curve: its four control points, with the evaluation,
/// derivative, splitting and length functions as methods.
///
/// ```
/// use math::Vec2;
/// use math::curve::CubicBezier;
///
/// let curve = CubicBezier::new(
///     Vec2::new(0.0, 0.0),
///     Vec2::new(0.0, 1.0),
///     Vec2::new(1.0, 1.0),
///     Vec2::new(1.0, 0.0),
/// );
/// let (left, right) = curve.split(0.5);
/// assert_eq!(left.p3, curve.point(0.5));
/// assert_eq!(left.p3, right.p0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CubicBezier<P> {
    /// The start of the curve (the curve passes through it at `t == 0`).
    pub p0: P,
    /// The first control point, which sets the direction leaving `p0`.
    pub p1: P,
    /// The second control point, which sets the direction arriving at `p3`.
    pub p2: P,
    /// The end of the curve (the curve passes through it at `t == 1`).
    pub p3: P,
}

impl<P> CubicBezier<P> {
    /// The curve with control points `p0`..`p3`.
    #[inline]
    pub const fn new(p0: P, p1: P, p2: P, p3: P) -> Self {
        Self { p0, p1, p2, p3 }
    }
}

impl<P: Copy> CubicBezier<P> {
    /// The control points as an array `[p0, p1, p2, p3]`.
    #[inline]
    pub const fn to_array(self) -> [P; 4] {
        [self.p0, self.p1, self.p2, self.p3]
    }

    /// The cubic Bézier curve equal to the Hermite curve from `p0` (leaving
    /// with velocity `m0`) to `p1` (arriving with velocity `m1`). See
    /// [`hermite_to_bezier`].
    #[inline]
    pub fn from_hermite<S: Scalar>(p0: P, m0: P, p1: P, m1: P) -> Self
    where
        P: CurvePoint<S>,
    {
        let [b0, b1, b2, b3] = hermite_to_bezier(p0, m0, p1, m1);
        Self::new(b0, b1, b2, b3)
    }

    /// The Hermite form `[p0, m0, p1, m1]` of this curve: its end points and
    /// the velocities there. See [`bezier_to_hermite`].
    #[inline]
    pub fn to_hermite<S: Scalar>(&self) -> [P; 4]
    where
        P: CurvePoint<S>,
    {
        bezier_to_hermite(self.p0, self.p1, self.p2, self.p3)
    }

    /// The point at `t`. See [`cubic_bezier`].
    #[inline]
    pub fn point<S: Scalar>(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        cubic_bezier(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// The first derivative at `t`. See [`cubic_bezier_derivative`].
    #[inline]
    pub fn derivative<S: Scalar>(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        cubic_bezier_derivative(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// The second derivative at `t`. See [`cubic_bezier_second_derivative`].
    #[inline]
    pub fn second_derivative<S: Scalar>(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        cubic_bezier_second_derivative(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// Splits the curve at `t` into the curves covering `0..=t` and `t..=1`.
    /// See [`cubic_bezier_split`].
    #[inline]
    pub fn split<S: Scalar>(&self, t: S) -> (Self, Self)
    where
        P: CurvePoint<S>,
    {
        let ([a0, a1, a2, a3], [b0, b1, b2, b3]) =
            cubic_bezier_split(self.p0, self.p1, self.p2, self.p3, t);
        (Self::new(a0, a1, a2, a3), Self::new(b0, b1, b2, b3))
    }

    /// The arc length of the whole curve, by Gauss-Legendre quadrature of its
    /// speed. Accurate to about the precision of `S` for any ordinary curve.
    #[inline]
    pub fn length<S: Scalar>(&self) -> S
    where
        P: CurvePoint<S>,
    {
        self.length_between(S::ZERO, S::ONE)
    }

    /// The arc length of the part of the curve between the parameters `t0`
    /// and `t1` (in either order).
    #[inline]
    pub fn length_between<S: Scalar>(&self, t0: S, t1: S) -> S
    where
        P: CurvePoint<S>,
    {
        arc_length_gauss(|t| self.derivative(t), t0, t1, LENGTH_PANELS)
    }

    /// A table of the curve's cumulative arc length at `N` evenly spaced
    /// parameters, for constant-speed movement along it. See
    /// [`ArcLengthTable`].
    #[inline]
    pub fn arc_length_table<S: Scalar, const N: usize>(&self) -> ArcLengthTable<S, N>
    where
        P: CurvePoint<S>,
    {
        ArcLengthTable::from_derivative(|t| self.derivative(t), S::ZERO, S::ONE)
    }
}
