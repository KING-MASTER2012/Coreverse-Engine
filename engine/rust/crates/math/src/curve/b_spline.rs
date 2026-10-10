//! B-splines: the uniform cubic segment ([`cubic_b_spline`]) and general
//! B-splines of any degree over a knot vector ([`BSpline`], evaluated with
//! de Boor's algorithm).

use core::cmp::Ordering;

use super::{CurvePoint, lit};
use crate::scalar::Scalar;

/// The highest degree [`BSpline`] supports. de Boor's algorithm works on a
/// small stack buffer of `MAX_B_SPLINE_DEGREE + 1` points (there is no heap),
/// and degrees above 3 or 4 are rare anyway.
pub const MAX_B_SPLINE_DEGREE: usize = 15;

/// The point at `t` (`0..=1`) on the uniform cubic B-spline segment defined by
/// the four consecutive control points `p0`..`p3`.
///
/// A B-spline *approximates* its control polygon instead of passing through
/// it: the segment starts at `(p0 + 4 p1 + p2) / 6` and ends at
/// `(p1 + 4 p2 + p3) / 6`. In return it is `C2` continuous: chaining the
/// segments `(p0..p3)`, `(p1..p4)`, ... gives a curve with continuous
/// position, tangent *and* curvature, which makes it a good choice for
/// camera motion and smooth paths.
///
/// ```
/// use math::curve::cubic_b_spline;
///
/// // Constant control points give a constant curve, and a line a line.
/// assert!((cubic_b_spline(2.0, 2.0, 2.0, 2.0, 0.3_f32) - 2.0).abs() < 1e-6);
/// assert!((cubic_b_spline(0.0, 1.0, 2.0, 3.0, 0.0_f32) - 1.0).abs() < 1e-6);
/// assert!((cubic_b_spline(0.0, 1.0, 2.0, 3.0, 1.0_f32) - 2.0).abs() < 1e-6);
/// ```
#[inline]
pub fn cubic_b_spline<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, p3: P, t: S) -> P {
    let three = lit::<S>(3.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let u = S::ONE - t;
    // The four uniform cubic B-spline basis functions, times six.
    let b0 = u * u * u;
    let b1 = three * t3 - lit::<S>(6.0) * t2 + lit::<S>(4.0);
    let b2 = three * t2 + three * t + S::ONE - three * t3;
    let b3 = t3;
    (p0 * b0 + p1 * b1 + p2 * b2 + p3 * b3) * (S::ONE / lit::<S>(6.0))
}

/// The first derivative (the velocity) of [`cubic_b_spline`] at `t`.
#[inline]
pub fn cubic_b_spline_derivative<S: Scalar, P: CurvePoint<S>>(
    p0: P,
    p1: P,
    p2: P,
    p3: P,
    t: S,
) -> P {
    let three = lit::<S>(3.0);
    let nine = lit::<S>(9.0);
    let t2 = t * t;
    let u = S::ONE - t;
    // The derivatives of the four basis functions, times six.
    let d0 = S::ZERO - three * u * u;
    let d1 = nine * t2 - lit::<S>(12.0) * t;
    let d2 = three + lit::<S>(6.0) * t - nine * t2;
    let d3 = three * t2;
    (p0 * d0 + p1 * d1 + p2 * d2 + p3 * d3) * (S::ONE / lit::<S>(6.0))
}

/// Fills `knots` with the *clamped uniform* knot vector for a B-spline of
/// `degree` with `control_count` control points, and returns `true`. Such a
/// spline starts at its first and ends at its last control point.
///
/// The vector has `control_count + degree + 1` knots: `degree + 1` zeros,
/// then `1, 2, ...`, then `degree + 1` copies of `control_count - degree`.
/// The spline's domain is therefore `0..=control_count - degree`. Returns
/// `false`, leaving `knots` untouched, if `degree` is `0`,
/// `control_count <= degree` or `knots` has the wrong length.
///
/// ```
/// use math::curve::fill_clamped_uniform_knots;
///
/// let mut knots = [0.0_f32; 8];
/// assert!(fill_clamped_uniform_knots(&mut knots, 5, 2));
/// assert_eq!(knots, [0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0]);
/// ```
pub fn fill_clamped_uniform_knots<S: Scalar>(
    knots: &mut [S],
    control_count: usize,
    degree: usize,
) -> bool {
    if degree == 0 || control_count <= degree || knots.len() != control_count + degree + 1 {
        return false;
    }
    for (i, knot) in knots.iter_mut().enumerate() {
        let value = if i <= degree {
            0
        } else if i >= control_count {
            control_count - degree
        } else {
            i - degree
        };
        *knot = S::from_f64(value as f64);
    }
    true
}

/// A B-spline of any degree: a curve of `degree`-th degree polynomial pieces
/// shaped by control points and a knot vector, evaluated with de Boor's
/// algorithm. It borrows both slices.
///
/// With `n` control points a degree-`p` spline needs `n + p + 1` knots, in
/// non-decreasing order, and is defined on `knots[p]..=knots[n]`. The usual
/// choice is a clamped knot vector (see [`fill_clamped_uniform_knots`]), for
/// which the spline starts at the first and ends at the last control point;
/// with `p + 1` control points it *is* the Bézier curve of those points.
///
/// ```
/// use math::Vec2;
/// use math::curve::{BSpline, fill_clamped_uniform_knots};
///
/// let points = [
///     Vec2::new(0.0, 0.0),
///     Vec2::new(1.0, 2.0),
///     Vec2::new(3.0, 2.0),
///     Vec2::new(4.0, 0.0),
///     Vec2::new(6.0, 1.0),
/// ];
/// let mut knots = [0.0_f32; 9];
/// assert!(fill_clamped_uniform_knots(&mut knots, points.len(), 3));
///
/// let spline = BSpline::new(3, &knots, &points).unwrap();
/// assert_eq!(spline.domain(), (0.0, 2.0));
/// assert_eq!(spline.point(0.0), Some(points[0]));
/// assert_eq!(spline.point(2.0), Some(points[4]));
/// assert_eq!(spline.point(2.5), None); // outside the domain
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BSpline<'a, P, S> {
    /// The degree `p` of the polynomial pieces (`1` is a polyline, `2` is
    /// quadratic, `3` cubic).
    degree: usize,
    /// The knot vector, `control_points.len() + degree + 1` non-decreasing
    /// values.
    knots: &'a [S],
    /// The control points.
    control_points: &'a [P],
}

impl<'a, P, S: Scalar> BSpline<'a, P, S> {
    /// The B-spline of `degree` through `control_points` over `knots`, or
    /// `None` if the input is not a valid spline:
    ///
    /// - `degree` is `0` or above [`MAX_B_SPLINE_DEGREE`], or there are not
    ///   more control points than `degree`;
    /// - `knots` does not hold exactly one more than the number of control
    ///   points plus the degree values;
    /// - the knots decrease, or one is `NaN`;
    /// - the domain, from the knot at index `degree` to the knot at index
    ///   `control_points.len()`, is empty.
    pub fn new(degree: usize, knots: &'a [S], control_points: &'a [P]) -> Option<Self> {
        let count = control_points.len();
        if degree == 0 || degree > MAX_B_SPLINE_DEGREE || count <= degree {
            return None;
        }
        if knots.len() != count + degree + 1 {
            return None;
        }
        // Every neighbouring pair must be ordered; a `NaN` knot has no order, so
        // it is rejected here too.
        let ordered = knots.windows(2).all(|pair| {
            matches!(
                pair[0].partial_cmp(&pair[1]),
                Some(Ordering::Less | Ordering::Equal)
            )
        });
        if ordered && knots[degree] < knots[count] {
            Some(Self {
                degree,
                knots,
                control_points,
            })
        } else {
            None
        }
    }

    /// The degree of the polynomial pieces.
    #[inline]
    pub const fn degree(&self) -> usize {
        self.degree
    }

    /// The knot vector.
    #[inline]
    pub const fn knots(&self) -> &'a [S] {
        self.knots
    }

    /// The control points.
    #[inline]
    pub const fn control_points(&self) -> &'a [P] {
        self.control_points
    }

    /// The parameter range `(start, end)` the spline is defined on.
    #[inline]
    pub fn domain(&self) -> (S, S) {
        (
            self.knots[self.degree],
            self.knots[self.control_points.len()],
        )
    }
}

impl<P: Copy, S: Scalar> BSpline<'_, P, S> {
    /// The point at `t`, or `None` if `t` is outside [`BSpline::domain`] (or
    /// `NaN`).
    pub fn point(&self, t: S) -> Option<P>
    where
        P: CurvePoint<S>,
    {
        let (lo, hi) = self.domain();
        // Written to reject `NaN` too.
        if !(t >= lo && t <= hi) {
            return None;
        }
        let degree = self.degree;
        let count = self.control_points.len();
        let knots = self.knots;
        // The knot span `knots[span] <= t < knots[span + 1]`. At the very end
        // of the domain no span satisfies that, so take the last non-empty
        // one instead (its polynomial is the one that reaches `hi`).
        let candidates = &knots[..=count];
        let span = if t < hi {
            candidates.partition_point(|&knot| knot <= t) - 1
        } else {
            candidates.partition_point(|&knot| knot < t) - 1
        };
        // de Boor's algorithm on the `degree + 1` control points that affect
        // this span.
        let mut d = [self.control_points[0]; MAX_B_SPLINE_DEGREE + 1];
        d[..=degree].copy_from_slice(&self.control_points[span - degree..=span]);
        for round in 1..=degree {
            for j in (round..=degree).rev() {
                let left = knots[j + span - degree];
                let right = knots[j + 1 + span - round];
                let width = right - left;
                let weight = if width > S::ZERO {
                    (t - left) / width
                } else {
                    S::ZERO
                };
                d[j] = d[j - 1] + (d[j] - d[j - 1]) * weight;
            }
        }
        Some(d[degree])
    }

    /// The point at `t` clamped to [`BSpline::domain`], so it never fails: a
    /// parameter before the start gives the start, one after the end the end.
    /// A `NaN` parameter gives the start.
    pub fn point_clamped(&self, t: S) -> P
    where
        P: CurvePoint<S>,
    {
        let (lo, hi) = self.domain();
        let t = if t > hi { hi } else { t };
        let t = if t >= lo { t } else { lo };
        // `t` is in the domain now, so `point` has an answer.
        self.point(t).unwrap_or(self.control_points[0])
    }
}
