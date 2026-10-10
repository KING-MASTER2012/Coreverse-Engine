//! Cubic Hermite interpolation: a curve fixed by its two end points and the
//! velocities there, and its conversion to and from a Bézier curve.

use super::{CurvePoint, lit};
use crate::scalar::Scalar;

/// The point at `t` on the cubic Hermite curve from `p0` to `p1`, leaving `p0`
/// with velocity `m0` and arriving at `p1` with velocity `m1`.
///
/// The velocities are derivatives with respect to `t`: `m0` is
/// `hermite_derivative(.., 0)` and `m1` is `hermite_derivative(.., 1)`. For a
/// straight line from `p0` to `p1` at constant speed both are `p1 - p0`. The
/// result is exactly `p0` at `t == 0` and exactly `p1` at `t == 1`.
///
/// ```
/// use math::curve::hermite;
///
/// // Ease from 0 to 1 with zero velocity at both ends: the smoothstep curve.
/// assert_eq!(hermite(0.0, 0.0, 1.0, 0.0, 0.0_f32), 0.0);
/// assert_eq!(hermite(0.0, 0.0, 1.0, 0.0, 0.5_f32), 0.5);
/// assert_eq!(hermite(0.0, 0.0, 1.0, 0.0, 1.0_f32), 1.0);
/// assert_eq!(hermite(0.0, 0.0, 1.0, 0.0, 0.25_f32), 0.156_25);
/// ```
#[inline]
pub fn hermite<S: Scalar, P: CurvePoint<S>>(p0: P, m0: P, p1: P, m1: P, t: S) -> P {
    let two = lit::<S>(2.0);
    let three = lit::<S>(3.0);
    let t2 = t * t;
    let t3 = t2 * t;
    // The four Hermite basis functions.
    let h00 = two * t3 - three * t2 + S::ONE;
    let h10 = t3 - two * t2 + t;
    let h01 = three * t2 - two * t3;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// The first derivative (the velocity) of [`hermite`] at `t`. It is `m0` at
/// `t == 0` and `m1` at `t == 1`.
#[inline]
pub fn hermite_derivative<S: Scalar, P: CurvePoint<S>>(p0: P, m0: P, p1: P, m1: P, t: S) -> P {
    let two = lit::<S>(2.0);
    let three = lit::<S>(3.0);
    let six = lit::<S>(6.0);
    let t2 = t * t;
    // The derivatives of the four Hermite basis functions.
    let d00 = six * (t2 - t);
    let d10 = three * t2 - two * two * t + S::ONE;
    let d01 = six * (t - t2);
    let d11 = three * t2 - two * t;
    p0 * d00 + m0 * d10 + p1 * d01 + m1 * d11
}

/// The control points `[b0, b1, b2, b3]` of the cubic Bézier curve that traces
/// exactly the same path as the Hermite curve `p0, m0, p1, m1`:
/// `[p0, p0 + m0 / 3, p1 - m1 / 3, p1]`.
///
/// ```
/// use math::curve::{cubic_bezier, hermite, hermite_to_bezier};
///
/// let [b0, b1, b2, b3] = hermite_to_bezier(0.0, 3.0, 1.0, -3.0_f32);
/// assert_eq!([b0, b1, b2, b3], [0.0, 1.0, 2.0, 1.0]);
/// assert_eq!(
///     cubic_bezier(b0, b1, b2, b3, 0.25_f32),
///     hermite(0.0, 3.0, 1.0, -3.0, 0.25_f32)
/// );
/// ```
#[inline]
pub fn hermite_to_bezier<S: Scalar, P: CurvePoint<S>>(p0: P, m0: P, p1: P, m1: P) -> [P; 4] {
    let third = S::ONE / lit::<S>(3.0);
    [p0, p0 + m0 * third, p1 - m1 * third, p1]
}

/// The Hermite form `[p0, m0, p1, m1]` of the cubic Bézier curve with control
/// points `b0`..`b3`: its end points and the velocities there,
/// `[b0, 3 (b1 - b0), b3, 3 (b3 - b2)]`. The inverse of [`hermite_to_bezier`].
#[inline]
pub fn bezier_to_hermite<S: Scalar, P: CurvePoint<S>>(b0: P, b1: P, b2: P, b3: P) -> [P; 4] {
    let three = lit::<S>(3.0);
    [b0, (b1 - b0) * three, b3, (b3 - b2) * three]
}
