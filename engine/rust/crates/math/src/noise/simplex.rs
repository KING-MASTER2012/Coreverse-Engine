//! Simplex noise in 2D and 3D.

use super::{NoiseScalar, grad2, grad3, lit};
use crate::random::{hash2, hash3};

/// The 2D skew factor `(sqrt(3) - 1) / 2`, which maps the square lattice
/// onto the grid of triangles that tile the plane.
const SKEW_2D: f64 = 0.366_025_403_784_438_6;

/// The 2D unskew factor `(3 - sqrt(3)) / 6`, the inverse of [`SKEW_2D`].
const UNSKEW_2D: f64 = 0.211_324_865_405_187_1;

/// The 3D skew factor `1 / 3`.
const SKEW_3D: f64 = 1.0 / 3.0;

/// The 3D unskew factor `1 / 6`.
const UNSKEW_3D: f64 = 1.0 / 6.0;

/// The factor that stretches the raw 2D simplex value to fill `-1..=1`. The
/// largest raw value is `0.0100802047`, found by maximising the sum of the
/// three corner terms over the position with each corner taking its best
/// gradient (the corners choose gradients independently); the exact factor
/// is `99.2043`, rounded down to leave a margin.
pub(super) const SIMPLEX_2D_SCALE: f64 = 99.2;

/// The factor that stretches the raw 3D simplex value to fill `-1..=1`. The
/// largest raw value is `0.0216268`, found like the 2D one; the exact factor
/// is `46.2390`, rounded down to leave a margin.
pub(super) const SIMPLEX_3D_SCALE: f64 = 46.2;

/// The contribution of one corner of a 2D simplex: a gradient dot product
/// faded out by `(0.5 - r^2)^4` within radius `sqrt(0.5)` of the corner.
#[inline]
fn corner_2d<S: NoiseScalar>(seed: u32, i: i32, j: i32, x: S, y: S) -> S {
    let t = lit::<S>(0.5) - x * x - y * y;
    if t > S::ZERO {
        let t2 = t * t;
        t2 * t2 * grad2(hash2(seed, i, j), x, y)
    } else {
        S::ZERO
    }
}

/// The contribution of one corner of a 3D simplex: a gradient dot product
/// faded out by `(0.6 - r^2)^4` within radius `sqrt(0.6)` of the corner.
#[inline]
fn corner_3d<S: NoiseScalar>(seed: u32, i: i32, j: i32, k: i32, x: S, y: S, z: S) -> S {
    let t = lit::<S>(0.6) - x * x - y * y - z * z;
    if t > S::ZERO {
        let t2 = t * t;
        t2 * t2 * grad3(hash3(seed, i, j, k), x, y, z)
    } else {
        S::ZERO
    }
}

/// 2D simplex noise before it is scaled to `-1..=1`.
pub(super) fn simplex_2d_raw<S: NoiseScalar>(x: S, y: S, seed: u32) -> S {
    let (skew, unskew) = (lit::<S>(SKEW_2D), lit::<S>(UNSKEW_2D));
    // The skewed cell `(fi, fj)` that contains the point.
    let s = (x + y) * skew;
    let (fi, fj) = ((x + s).floor(), (y + s).floor());
    let (i, j) = (fi.floor_to_i32(), fj.floor_to_i32());
    // The offsets from the cell's first corner, in unskewed space.
    let t = (fi + fj) * unskew;
    let (x0, y0) = (x - (fi - t), y - (fj - t));
    // The second corner is one step along the longer axis.
    let (i1, j1, s1x, s1y) = if x0 > y0 {
        (1, 0, S::ONE, S::ZERO)
    } else {
        (0, 1, S::ZERO, S::ONE)
    };
    let (x1, y1) = (x0 - s1x + unskew, y0 - s1y + unskew);
    let (x2, y2) = (x0 - S::ONE + unskew + unskew, y0 - S::ONE + unskew + unskew);
    corner_2d(seed, i, j, x0, y0)
        + corner_2d(seed, i.wrapping_add(i1), j.wrapping_add(j1), x1, y1)
        + corner_2d(seed, i.wrapping_add(1), j.wrapping_add(1), x2, y2)
}

/// 3D simplex noise before it is scaled to `-1..=1`.
pub(super) fn simplex_3d_raw<S: NoiseScalar>(x: S, y: S, z: S, seed: u32) -> S {
    let (skew, unskew) = (lit::<S>(SKEW_3D), lit::<S>(UNSKEW_3D));
    let s = (x + y + z) * skew;
    let (fi, fj, fk) = ((x + s).floor(), (y + s).floor(), (z + s).floor());
    let (i, j, k) = (fi.floor_to_i32(), fj.floor_to_i32(), fk.floor_to_i32());
    let t = (fi + fj + fk) * unskew;
    let (x0, y0, z0) = (x - (fi - t), y - (fj - t), z - (fk - t));
    // The order of the offsets picks which of the six tetrahedra of the cube
    // the point is in, and so the path of corners from the first to the last.
    let ((i1, j1, k1), (i2, j2, k2)) = if x0 >= y0 {
        if y0 >= z0 {
            ((1, 0, 0), (1, 1, 0))
        } else if x0 >= z0 {
            ((1, 0, 0), (1, 0, 1))
        } else {
            ((0, 0, 1), (1, 0, 1))
        }
    } else if y0 < z0 {
        ((0, 0, 1), (0, 1, 1))
    } else if x0 < z0 {
        ((0, 1, 0), (0, 1, 1))
    } else {
        ((0, 1, 0), (1, 1, 0))
    };
    /// A 0/1 corner offset as the scalar.
    fn step<S: NoiseScalar>(bit: i32) -> S {
        if bit == 0 { S::ZERO } else { S::ONE }
    }
    let two = unskew + unskew;
    let three = two + unskew;
    let (x1, y1, z1) = (
        x0 - step::<S>(i1) + unskew,
        y0 - step::<S>(j1) + unskew,
        z0 - step::<S>(k1) + unskew,
    );
    let (x2, y2, z2) = (
        x0 - step::<S>(i2) + two,
        y0 - step::<S>(j2) + two,
        z0 - step::<S>(k2) + two,
    );
    let (x3, y3, z3) = (
        x0 - S::ONE + three,
        y0 - S::ONE + three,
        z0 - S::ONE + three,
    );
    corner_3d(seed, i, j, k, x0, y0, z0)
        + corner_3d(
            seed,
            i.wrapping_add(i1),
            j.wrapping_add(j1),
            k.wrapping_add(k1),
            x1,
            y1,
            z1,
        )
        + corner_3d(
            seed,
            i.wrapping_add(i2),
            j.wrapping_add(j2),
            k.wrapping_add(k2),
            x2,
            y2,
            z2,
        )
        + corner_3d(
            seed,
            i.wrapping_add(1),
            j.wrapping_add(1),
            k.wrapping_add(1),
            x3,
            y3,
            z3,
        )
}

/// 2D simplex noise at `(x, y)`, in `-1..=1`.
///
/// Simplex noise samples the three corners of the triangle containing the
/// point instead of the four of a square, which is cheaper and, because the
/// triangular grid is more isotropic, free of the faint axis-aligned
/// patterns of Perlin noise. Unlike [`perlin_2d`](super::perlin_2d) it is
/// not zero at the integer lattice points.
///
/// ```
/// use math::noise::simplex_2d;
///
/// let v = simplex_2d(0.37_f32, 1.62, 7);
/// assert!((-1.0..=1.0).contains(&v));
/// assert_eq!(v, simplex_2d(0.37_f32, 1.62, 7)); // deterministic
/// ```
#[inline]
pub fn simplex_2d<S: NoiseScalar>(x: S, y: S, seed: u32) -> S {
    simplex_2d_raw(x, y, seed) * lit::<S>(SIMPLEX_2D_SCALE)
}

/// 3D simplex noise at `(x, y, z)`, in `-1..=1`: the four corners of the
/// tetrahedron containing the point. See [`simplex_2d`].
///
/// ```
/// use math::noise::simplex_3d;
///
/// assert!((-1.0..=1.0).contains(&simplex_3d(0.4_f64, 5.5, -2.2, 9)));
/// ```
#[inline]
pub fn simplex_3d<S: NoiseScalar>(x: S, y: S, z: S, seed: u32) -> S {
    simplex_3d_raw(x, y, z, seed) * lit::<S>(SIMPLEX_3D_SCALE)
}
