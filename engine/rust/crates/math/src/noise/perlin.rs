//! Perlin (improved gradient) noise in 2D and 3D.

use super::{NoiseScalar, fade, grad2, grad3, lerp, lit};
use crate::random::{hash2, hash3};

/// The factor that stretches the raw 2D Perlin value to fill `-1..=1`. The raw
/// value is bounded by `1 / sqrt(2)` (unit gradients, and the bound is reached
/// at a cell's centre when all four corners point at it), so the exact factor
/// is `sqrt(2)`; it is reduced by one part in a million so that rounding
/// error can never push a result past `1`.
pub(super) const PERLIN_2D_SCALE: f64 = core::f64::consts::SQRT_2 * 0.999_999;

/// The factor that stretches the raw 3D Perlin value to fill `-1..=1`. Like
/// the 2D case the raw value is bounded by `1 / sqrt(2)`: at a cell's centre
/// a corner's gradient, which has two non-zero components, can align with
/// two of the three components of the offset. Reduced by one part in a
/// million for the same reason.
pub(super) const PERLIN_3D_SCALE: f64 = core::f64::consts::SQRT_2 * 0.999_999;

/// 2D Perlin noise before it is scaled to `-1..=1`.
pub(super) fn perlin_2d_raw<S: NoiseScalar>(x: S, y: S, seed: u32) -> S {
    let (fx, fy) = (x.floor(), y.floor());
    let (xi, yi) = (fx.floor_to_i32(), fy.floor_to_i32());
    // The position inside the cell, and relative to its far corners.
    let (dx, dy) = (x - fx, y - fy);
    let (ex, ey) = (dx - S::ONE, dy - S::ONE);
    let (x1, y1) = (xi.wrapping_add(1), yi.wrapping_add(1));
    let n00 = grad2(hash2(seed, xi, yi), dx, dy);
    let n10 = grad2(hash2(seed, x1, yi), ex, dy);
    let n01 = grad2(hash2(seed, xi, y1), dx, ey);
    let n11 = grad2(hash2(seed, x1, y1), ex, ey);
    let (u, v) = (fade(dx), fade(dy));
    lerp(lerp(n00, n10, u), lerp(n01, n11, u), v)
}

/// 3D Perlin noise before it is scaled to `-1..=1`.
pub(super) fn perlin_3d_raw<S: NoiseScalar>(x: S, y: S, z: S, seed: u32) -> S {
    let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
    let (xi, yi, zi) = (fx.floor_to_i32(), fy.floor_to_i32(), fz.floor_to_i32());
    let (dx, dy, dz) = (x - fx, y - fy, z - fz);
    let (ex, ey, ez) = (dx - S::ONE, dy - S::ONE, dz - S::ONE);
    let (x1, y1, z1) = (xi.wrapping_add(1), yi.wrapping_add(1), zi.wrapping_add(1));
    let n000 = grad3(hash3(seed, xi, yi, zi), dx, dy, dz);
    let n100 = grad3(hash3(seed, x1, yi, zi), ex, dy, dz);
    let n010 = grad3(hash3(seed, xi, y1, zi), dx, ey, dz);
    let n110 = grad3(hash3(seed, x1, y1, zi), ex, ey, dz);
    let n001 = grad3(hash3(seed, xi, yi, z1), dx, dy, ez);
    let n101 = grad3(hash3(seed, x1, yi, z1), ex, dy, ez);
    let n011 = grad3(hash3(seed, xi, y1, z1), dx, ey, ez);
    let n111 = grad3(hash3(seed, x1, y1, z1), ex, ey, ez);
    let (u, v, w) = (fade(dx), fade(dy), fade(dz));
    let near = lerp(lerp(n000, n100, u), lerp(n010, n110, u), v);
    let far = lerp(lerp(n001, n101, u), lerp(n011, n111, u), v);
    lerp(near, far, w)
}

/// 2D Perlin (gradient) noise at `(x, y)`, in `-1..=1`.
///
/// Every integer lattice point gets a pseudo-random unit gradient from `seed`;
/// the value at a point is the smooth (quintic) blend of the four surrounding
/// corners' contributions. The result is exactly `0` at the lattice points
/// themselves and changes smoothly between them: it is continuous with
/// continuous first and second derivatives.
///
/// ```
/// use math::noise::perlin_2d;
///
/// assert_eq!(perlin_2d(3.0_f32, -2.0, 7), 0.0); // zero on the lattice
/// let v = perlin_2d(0.37_f32, 1.62, 7);
/// assert!((-1.0..=1.0).contains(&v));
/// assert_eq!(v, perlin_2d(0.37_f32, 1.62, 7)); // deterministic
/// ```
#[inline]
pub fn perlin_2d<S: NoiseScalar>(x: S, y: S, seed: u32) -> S {
    perlin_2d_raw(x, y, seed) * lit::<S>(PERLIN_2D_SCALE)
}

/// 3D Perlin (gradient) noise at `(x, y, z)`, in `-1..=1`. The 3D counterpart
/// of [`perlin_2d`], with the 12 edge-midpoint gradients of Perlin's improved
/// noise; exactly `0` at the lattice points.
///
/// ```
/// use math::noise::perlin_3d;
///
/// assert_eq!(perlin_3d(1.0_f64, 2.0, 3.0, 9), 0.0);
/// assert!((-1.0..=1.0).contains(&perlin_3d(0.4_f64, 5.5, -2.2, 9)));
/// ```
#[inline]
pub fn perlin_3d<S: NoiseScalar>(x: S, y: S, z: S, seed: u32) -> S {
    perlin_3d_raw(x, y, z, seed) * lit::<S>(PERLIN_3D_SCALE)
}
