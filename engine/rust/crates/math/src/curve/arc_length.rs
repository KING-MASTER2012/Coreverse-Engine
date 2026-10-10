//! Arc length: numerical integration of a curve's length, and the
//! [`ArcLengthTable`] that converts between a curve's parameter and the
//! distance travelled along it (for constant-speed animation).

use super::{CurvePoint, lit};
use crate::scalar::Scalar;

/// The nodes and weights `(x, w)` of the 5-point Gauss-Legendre rule on
/// `-1..=1`; it integrates polynomials up to degree 9 exactly.
const GAUSS_LEGENDRE_5: [(f64, f64); 5] = [
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.0, 0.568_888_888_888_888_9),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// The length of the curve `curve(t)` for `t` from `t0` to `t1`, approximated
/// by the polyline through `segments` evenly spaced points.
///
/// The polyline is always a little shorter than the curve (the error shrinks
/// with the square of `segments`); use [`arc_length_gauss`] when the
/// derivative is known for much higher accuracy per evaluation. `t0` and `t1`
/// may be given in either order, and `segments == 0` counts as `1`.
///
/// ```
/// use math::Vec2;
/// use math::curve::arc_length_polyline;
///
/// // A unit circle's quarter, 1000 chords: pi / 2 within a millionth.
/// let quarter = |t: f64| math::DVec2::new(t.cos(), t.sin());
/// # let _ = Vec2::ZERO;
/// let length = arc_length_polyline(quarter, 0.0, core::f64::consts::FRAC_PI_2, 1000);
/// assert!((length - core::f64::consts::FRAC_PI_2).abs() < 1e-6);
/// ```
pub fn arc_length_polyline<S: Scalar, P: CurvePoint<S>>(
    curve: impl Fn(S) -> P,
    t0: S,
    t1: S,
    segments: u32,
) -> S {
    let (a, b) = if t0 <= t1 { (t0, t1) } else { (t1, t0) };
    let segments = segments.max(1);
    let count = lit::<S>(f64::from(segments));
    let mut previous = curve(a);
    let mut total = S::ZERO;
    for i in 1..=segments {
        // The last point is exactly `b`, whatever the rounding of the steps.
        let t = if i == segments {
            b
        } else {
            a + (b - a) * (lit::<S>(f64::from(i)) / count)
        };
        let point = curve(t);
        total = total + (point - previous).length();
        previous = point;
    }
    total
}

/// The length of a curve for `t` from `t0` to `t1`, given its *derivative*
/// (velocity) `derivative(t)`: the integral of the speed `|derivative(t)|`,
/// computed with the 5-point Gauss-Legendre rule on `segments` equal panels.
///
/// For a smooth curve this is exact to the precision of `S` with a handful of
/// panels (8 is plenty for a Bézier segment; ask for more when the speed
/// changes abruptly). `t0` and `t1` may be given in either order, and
/// `segments == 0` counts as `1`. Each panel costs five derivative
/// evaluations.
///
/// ```
/// use math::curve::{arc_length_gauss, cubic_bezier_derivative};
///
/// // A straight cubic Bézier from 0 to 3 has length 3, however its control
/// // points are spaced.
/// let speed = |t: f64| cubic_bezier_derivative(0.0, 0.0, 1.0, 3.0, t);
/// let length = arc_length_gauss(speed, 0.0, 1.0, 4);
/// assert!((length - 3.0).abs() < 1e-12);
/// ```
pub fn arc_length_gauss<S: Scalar, P: CurvePoint<S>>(
    derivative: impl Fn(S) -> P,
    t0: S,
    t1: S,
    segments: u32,
) -> S {
    let (a, b) = if t0 <= t1 { (t0, t1) } else { (t1, t0) };
    let segments = segments.max(1);
    let panel = (b - a) / lit::<S>(f64::from(segments));
    let half = panel * lit::<S>(0.5);
    let mut total = S::ZERO;
    for i in 0..segments {
        let mid = a + panel * (lit::<S>(f64::from(i)) + lit::<S>(0.5));
        let mut sum = S::ZERO;
        for (x, w) in GAUSS_LEGENDRE_5 {
            sum = sum + lit::<S>(w) * derivative(mid + half * lit::<S>(x)).length();
        }
        total = total + sum * half;
    }
    total
}

/// A table of a curve's cumulative arc length at `N` evenly spaced parameters,
/// which maps the distance travelled along the curve to the parameter `t`
/// that reaches it (and back). Moving `t` by equal steps moves faster or
/// slower along the curve depending on how its control points are spaced;
/// stepping the *distance* by equal amounts and converting with
/// [`ArcLengthTable::t_at_length`] gives constant speed.
///
/// The table holds `N` samples inline (no heap) and `N` must be at least `2`
/// (checked at compile time). Between samples the length is interpolated
/// linearly, so more samples mean a more accurate mapping; `64` to `256` is
/// typical.
///
/// ```
/// use math::Vec2;
/// use math::curve::CubicBezier;
///
/// // The control points bunch up near the start, so `t` runs fast there.
/// let curve = CubicBezier::new(
///     Vec2::new(0.0, 0.0),
///     Vec2::new(0.1, 0.0),
///     Vec2::new(0.2, 0.0),
///     Vec2::new(4.0, 0.0),
/// );
/// let table = curve.arc_length_table::<f32, 128>();
/// assert!((table.total_length() - 4.0).abs() < 1e-3);
///
/// // Half the distance, which the curve reaches long after `t == 0.5`.
/// let t = table.t_at_fraction(0.5);
/// assert!((curve.point(t).x - 2.0).abs() < 1e-2);
/// assert!(t > 0.7);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcLengthTable<S, const N: usize> {
    /// The parameter of the first sample.
    t0: S,
    /// The parameter of the last sample.
    t1: S,
    /// The cumulative length at each sample; non-decreasing, starting at `0`.
    lengths: [S; N],
}

impl<S: Scalar, const N: usize> ArcLengthTable<S, N> {
    /// The parameter of sample `index` (`0..N`): the samples are evenly
    /// spaced from `t0` to `t1`, ending exactly on `t1`.
    #[inline]
    fn sample_t(&self, index: usize) -> S {
        if index + 1 == N {
            self.t1
        } else {
            let fraction = lit::<S>(index as f64) / lit::<S>((N - 1) as f64);
            self.t0 + (self.t1 - self.t0) * fraction
        }
    }

    /// Measures the curve `curve(t)` for `t` in `t0..=t1` by the polyline
    /// through `N` evenly spaced points. The chords are slightly shorter than
    /// the curve, so the table underestimates by an amount that shrinks with
    /// the square of `N`; [`ArcLengthTable::from_derivative`] is more accurate
    /// when the derivative is available. `t0` should be below `t1`.
    pub fn new<P: CurvePoint<S>>(curve: impl Fn(S) -> P, t0: S, t1: S) -> Self {
        const { assert!(N >= 2, "an ArcLengthTable needs at least 2 samples") };
        let mut table = Self {
            t0,
            t1,
            lengths: [S::ZERO; N],
        };
        let mut previous = curve(t0);
        for i in 1..N {
            let point = curve(table.sample_t(i));
            table.lengths[i] = table.lengths[i - 1] + (point - previous).length();
            previous = point;
        }
        table
    }

    /// Measures the curve for `t` in `t0..=t1` from its *derivative*: the
    /// length between neighbouring samples is integrated with one
    /// Gauss-Legendre panel (see [`arc_length_gauss`]), so the table is
    /// accurate to the precision of `S` for smooth curves, whatever `N`.
    /// `t0` should be below `t1`.
    pub fn from_derivative<P: CurvePoint<S>>(derivative: impl Fn(S) -> P, t0: S, t1: S) -> Self {
        const { assert!(N >= 2, "an ArcLengthTable needs at least 2 samples") };
        let mut table = Self {
            t0,
            t1,
            lengths: [S::ZERO; N],
        };
        for i in 1..N {
            let (a, b) = (table.sample_t(i - 1), table.sample_t(i));
            table.lengths[i] = table.lengths[i - 1] + arc_length_gauss(&derivative, a, b, 1);
        }
        table
    }

    /// The parameter range `(t0, t1)` the table covers.
    #[inline]
    pub const fn domain(&self) -> (S, S) {
        (self.t0, self.t1)
    }

    /// The cumulative length at each of the `N` samples (the first is `0`).
    #[inline]
    pub const fn lengths(&self) -> &[S; N] {
        &self.lengths
    }

    /// The length of the whole curve.
    #[inline]
    pub const fn total_length(&self) -> S {
        self.lengths[N - 1]
    }

    /// The distance travelled along the curve when the parameter is `t`.
    /// `t` is clamped to the table's domain, so this is `0` at or before `t0`
    /// and [`ArcLengthTable::total_length`] at or after `t1`.
    pub fn length_at(&self, t: S) -> S {
        if t <= self.t0 {
            return S::ZERO;
        }
        if t >= self.t1 {
            return self.total_length();
        }
        // The last sample at or before `t`, by bisection (the scalar has no
        // conversion to an index). `t` is strictly inside the domain, so the
        // answer is one of `0..=N - 2`.
        let (mut lo, mut hi) = (0_usize, N - 2);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.sample_t(mid) <= t {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let (ta, tb) = (self.sample_t(lo), self.sample_t(lo + 1));
        let fraction = (t - ta) / (tb - ta);
        lerp_scalar(self.lengths[lo], self.lengths[lo + 1], fraction)
    }

    /// The parameter `t` at which the curve has travelled the distance
    /// `length`: the inverse of [`ArcLengthTable::length_at`]. `length` is
    /// clamped to `0..=total_length()`. If the curve has no length at all
    /// (every point equal) the result is `t0`.
    pub fn t_at_length(&self, length: S) -> S {
        let total = self.total_length();
        if total > S::ZERO {
            if length <= S::ZERO {
                return self.t0;
            }
        } else {
            // No length at all (or a `NaN` one): there is no distance to follow.
            return self.t0;
        }
        if length >= total {
            return self.t1;
        }
        // The first sample strictly beyond `length`; the one before it is at or
        // below. Skips stretches where the length does not grow. Both exist
        // because `0 < length < total`.
        let beyond = self.lengths.partition_point(|&sample| sample <= length);
        let (la, lb) = (self.lengths[beyond - 1], self.lengths[beyond]);
        let fraction = (length - la) / (lb - la);
        lerp_scalar(self.sample_t(beyond - 1), self.sample_t(beyond), fraction)
    }

    /// The parameter `t` at which the curve has travelled the given
    /// `fraction` (`0..=1`, clamped) of its total length: the constant-speed
    /// replacement for `t = fraction`. If the curve has no length at all the
    /// result is the plain linear `t0 + (t1 - t0) * fraction`.
    pub fn t_at_fraction(&self, fraction: S) -> S {
        let fraction = if fraction > S::ONE { S::ONE } else { fraction };
        let fraction = if fraction >= S::ZERO {
            fraction
        } else {
            S::ZERO
        };
        let total = self.total_length();
        if total > S::ZERO {
            self.t_at_length(total * fraction)
        } else {
            lerp_scalar(self.t0, self.t1, fraction)
        }
    }
}

/// The linear interpolation `a + (b - a) * t` of two scalars.
#[inline]
fn lerp_scalar<S: Scalar>(a: S, b: S, t: S) -> S {
    a + (b - a) * t
}
