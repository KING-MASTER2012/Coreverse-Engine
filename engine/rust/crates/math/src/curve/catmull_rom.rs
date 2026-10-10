//! Catmull-Rom splines: smooth `C1` curves that pass through every control
//! point, in a uniform and a centripetal/chordal (Barry-Goldman) flavour.

use super::hermite::{hermite, hermite_derivative};
use super::{CurvePoint, lit};
use crate::scalar::Scalar;

/// The point at `t` on the uniform Catmull-Rom segment between `p1` (at
/// `t == 0`) and `p2` (at `t == 1`); `p0` and `p3` are the neighbouring
/// points that shape the tangents there: `(p2 - p0) / 2` at `p1` and
/// `(p3 - p1) / 2` at `p2`.
///
/// Chaining segments `(p0, p1, p2, p3)`, `(p1, p2, p3, p4)`, ... gives a
/// curve through all the points with a continuous tangent. The result is
/// exactly `p1` at `t == 0` and exactly `p2` at `t == 1`. For a whole chain
/// see [`catmull_rom_spline`].
///
/// ```
/// use math::curve::catmull_rom;
///
/// // Equally spaced points on a line stay on the line, at constant speed.
/// let t = 0.25_f32;
/// assert_eq!(catmull_rom(0.0, 1.0, 2.0, 3.0, t), 1.25);
/// ```
#[inline]
pub fn catmull_rom<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, p3: P, t: S) -> P {
    let half = lit::<S>(0.5);
    hermite(p1, (p2 - p0) * half, p2, (p3 - p1) * half, t)
}

/// The first derivative (the velocity) of [`catmull_rom`] at `t`.
#[inline]
pub fn catmull_rom_derivative<S: Scalar, P: CurvePoint<S>>(p0: P, p1: P, p2: P, p3: P, t: S) -> P {
    let half = lit::<S>(0.5);
    hermite_derivative(p1, (p2 - p0) * half, p2, (p3 - p1) * half, t)
}

/// The spacing between two knots of the Barry-Goldman recursion: the
/// distance between the points raised to `alpha`, or `1` when the points
/// coincide (a zero spacing would divide by zero; since the points are equal
/// any positive spacing gives the same curve).
#[inline]
fn knot_step<S: Scalar, P: CurvePoint<S>>(a: P, b: P, alpha: S) -> S {
    let step = (b - a).length().powf(alpha);
    if step > S::ZERO { step } else { S::ONE }
}

/// The point `(t_hi - t) / (t_hi - t_lo) * a + (t - t_lo) / (t_hi - t_lo) * b`:
/// the linear blend of `a` and `b` over the knot interval `t_lo..t_hi`.
#[inline]
fn blend<S: Scalar, P: CurvePoint<S>>(a: P, b: P, t_lo: S, t_hi: S, t: S) -> P {
    let span = t_hi - t_lo;
    a * ((t_hi - t) / span) + b * ((t - t_lo) / span)
}

/// The point at `t` on the Catmull-Rom segment between `p1` and `p2`, with the
/// knot spacing chosen by `alpha` (Barry-Goldman's pyramid formulation).
///
/// - `alpha == 0` is the **uniform** spline, the same curve as
///   [`catmull_rom`].
/// - `alpha == 0.5` is the **centripetal** spline: it never forms cusps or
///   self-intersections inside a segment and does not overshoot, which makes
///   it the best default for paths through unevenly spaced waypoints.
/// - `alpha == 1` is the **chordal** spline.
///
/// Coincident points are handled (the segment simply degenerates, with no
/// `NaN`). The result is exactly `p1` at `t == 0` and exactly `p2` at
/// `t == 1`.
///
/// ```
/// use math::Vec2;
/// use math::curve::catmull_rom_alpha;
///
/// let p = [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(5.0, 1.0)];
/// let mid = catmull_rom_alpha(p[0], p[1], p[2], p[3], 0.5, 0.5);
/// assert!(mid.y > 0.0 && mid.y < 1.0);
/// ```
pub fn catmull_rom_alpha<S: Scalar, P: CurvePoint<S>>(
    p0: P,
    p1: P,
    p2: P,
    p3: P,
    alpha: S,
    t: S,
) -> P {
    if t == S::ZERO {
        return p1;
    }
    if t == S::ONE {
        return p2;
    }
    let t0 = S::ZERO;
    let t1 = t0 + knot_step(p0, p1, alpha);
    let t2 = t1 + knot_step(p1, p2, alpha);
    let t3 = t2 + knot_step(p2, p3, alpha);
    // The parameter, moved onto the knot interval of the middle segment.
    let u = t1 + (t2 - t1) * t;
    let a1 = blend(p0, p1, t0, t1, u);
    let a2 = blend(p1, p2, t1, t2, u);
    let a3 = blend(p2, p3, t2, t3, u);
    let b1 = blend(a1, a2, t0, t2, u);
    let b2 = blend(a2, a3, t1, t3, u);
    blend(b1, b2, t1, t2, u)
}

/// The point at `t` on the Catmull-Rom spline through all of `points`, or
/// `None` if there are fewer than two points or `t` is outside
/// `0..=points.len() - 1` (or `NaN`).
///
/// The integer part of `t` selects the segment and the fractional part is the
/// local parameter, so the spline is at `points[i]` exactly when `t == i`.
/// `alpha` picks the flavour as in [`catmull_rom_alpha`] (`0` uniform, `0.5`
/// centripetal). The two ends have no neighbour to take a tangent from, so
/// the missing point is mirrored: the spline starts and ends heading
/// straight towards the next (or from the previous) point.
///
/// ```
/// use math::Vec2;
/// use math::curve::catmull_rom_spline;
///
/// let path = [Vec2::new(0.0, 0.0), Vec2::new(2.0, 1.0), Vec2::new(4.0, 0.0)];
/// assert_eq!(catmull_rom_spline(&path, 0.5, 1.0), Some(path[1]));
/// assert_eq!(catmull_rom_spline(&path, 0.5, 2.0), Some(path[2]));
/// assert_eq!(catmull_rom_spline(&path, 0.5, 2.5), None);
/// ```
pub fn catmull_rom_spline<S: Scalar, P: CurvePoint<S>>(points: &[P], alpha: S, t: S) -> Option<P> {
    let count = points.len();
    if count < 2 {
        return None;
    }
    let last = S::from_f64((count - 1) as f64);
    // Written to reject `NaN` too.
    if !(t >= S::ZERO && t <= last) {
        return None;
    }
    // The segment is the largest index `i <= count - 2` with `i <= t`, found by
    // bisection (the scalar has no conversion to an index).
    let (mut lo, mut hi) = (0_usize, count - 2);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if S::from_f64(mid as f64) <= t {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let local = t - S::from_f64(lo as f64);
    let p1 = points[lo];
    let p2 = points[lo + 1];
    // Mirror the missing neighbour at either end: `2 * p1 - p2`.
    let p0 = if lo == 0 {
        p1 + (p1 - p2)
    } else {
        points[lo - 1]
    };
    let p3 = if lo + 2 < count {
        points[lo + 2]
    } else {
        p2 + (p2 - p1)
    };
    Some(catmull_rom_alpha(p0, p1, p2, p3, alpha, local))
}
