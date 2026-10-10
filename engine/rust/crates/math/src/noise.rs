//! Coherent noise for procedural content (Phase 2.8): gradient ([`perlin_2d`],
//! [`perlin_3d`]) and simplex ([`simplex_2d`], [`simplex_3d`]) noise, and the
//! fractal combinators [`fbm_2d`], [`fbm_3d`], [`ridged_2d`] and
//! [`ridged_3d`] that stack octaves of them.
//!
//! Noise is a smooth random function: nearby inputs give nearby outputs, but
//! the overall pattern has no repetition you can see. It shapes terrain,
//! clouds, wood grain, camera shake, wind and any parameter that should
//! wander instead of jump.
//!
//! ```
//! use math::noise::{fbm_2d, perlin_2d};
//!
//! let seed: u32 = 1234;
//! // A height field: three octaves of Perlin noise, each twice as detailed
//! // and half as strong as the last.
//! let height = |x: f32, z: f32| {
//!     fbm_2d(x * 0.1, z * 0.1, 3, 2.0, 0.5, |x, y, octave| {
//!         perlin_2d(x, y, seed.wrapping_add(octave))
//!     })
//! };
//! let h = height(10.0, 20.0);
//! assert!((-1.0..=1.0).contains(&h));
//! ```
//!
//! # Properties
//!
//! - **Deterministic.** The same `(x, y[, z], seed)` gives the same value on
//!   every platform and build; there is no state and no permutation table to
//!   build. The lattice gradients come from [`hash2`](crate::random::hash2)
//!   and [`hash3`](crate::random::hash3), so a different `seed` is a different
//!   field.
//! - **Range.** Every function returns values in `-1..=1` (the fractal ones
//!   too, except [`ridged_2d`]/[`ridged_3d`] which return `0..=1`), and
//!   typically fills most of it. The mean is `0`.
//! - **Scale.** The lattice has unit spacing: features are about one unit
//!   wide, so multiply the coordinates by a frequency (`x * 0.05`) to make
//!   them larger or smaller. Coordinates can be negative or large; precision
//!   degrades slowly with the magnitude, as with any float coordinate (in
//!   `f32`, keep them below about `10^5`).
//! - **Generic over `f32` and `f64`** through [`NoiseScalar`].
//! - **`no_std` and allocation-free**; no function panics, and a `NaN` or
//!   infinite coordinate yields a finite or `NaN` value, never a crash.
//!
//! # Perlin or simplex?
//!
//! | | Perlin (gradient) | Simplex |
//! | --- | --- | --- |
//! | Cost | `2^n` lattice corners | `n + 1` corners, fewer at high dimension |
//! | Look | slight axis-aligned grid bias | isotropic, no visible grid |
//! | Zero at lattice points | yes (the grid shows in flat spots) | no |
//!
//! Both work for most purposes; simplex looks more natural when the grid
//! alignment of Perlin noise bothers you.

mod fractal;
mod perlin;
mod simplex;

use crate::scalar::Scalar;

pub use fractal::{fbm_2d, fbm_3d, ridged_2d, ridged_3d};
pub use perlin::{perlin_2d, perlin_3d};
pub use simplex::{simplex_2d, simplex_3d};

/// A float type the noise functions work on: [`Scalar`] plus the conversion to
/// a lattice index. Implemented for `f32` and `f64`.
pub trait NoiseScalar: Scalar {
    /// The integer part of `self` rounded down, as the index of a lattice
    /// cell. Values beyond the `i32` range saturate and `NaN` gives `0`, so
    /// the noise functions never panic on bad input.
    fn floor_to_i32(self) -> i32;
}

impl NoiseScalar for f32 {
    #[inline]
    fn floor_to_i32(self) -> i32 {
        Scalar::floor(self) as i32
    }
}

impl NoiseScalar for f64 {
    #[inline]
    fn floor_to_i32(self) -> i32 {
        Scalar::floor(self) as i32
    }
}

/// The constant `value` as the scalar `S`.
#[inline]
fn lit<S: Scalar>(value: f64) -> S {
    S::from_f64(value)
}

/// Perlin's quintic fade curve `6t^5 - 15t^4 + 10t^3`: `0` at `t == 0`, `1`
/// at `t == 1`, with zero first and second derivatives at both, so the noise
/// has no visible creases at lattice boundaries.
#[inline]
fn fade<S: Scalar>(t: S) -> S {
    t * t * t * (t * (t * lit::<S>(6.0) - lit::<S>(15.0)) + lit::<S>(10.0))
}

/// The linear interpolation `a + (b - a) * t`.
#[inline]
fn lerp<S: Scalar>(a: S, b: S, t: S) -> S {
    a + (b - a) * t
}

/// The dot product of the offset `(x, y)` with one of 8 unit gradients picked
/// by the low three bits of `hash`: the four axis directions and the four
/// diagonals.
#[inline]
fn grad2<S: Scalar>(hash: u32, x: S, y: S) -> S {
    let diagonal = lit::<S>(core::f64::consts::FRAC_1_SQRT_2);
    match hash & 7 {
        0 => x,
        1 => S::ZERO - x,
        2 => y,
        3 => S::ZERO - y,
        4 => (x + y) * diagonal,
        5 => (x - y) * diagonal,
        6 => (y - x) * diagonal,
        _ => (S::ZERO - x - y) * diagonal,
    }
}

/// The dot product of the offset `(x, y, z)` with one of the 12 unit
/// gradients that point from the centre of a cube to the middle of its edges
/// (`(+-1, +-1, 0)`, `(+-1, 0, +-1)`, `(0, +-1, +-1)`, normalised), picked by
/// the low four bits of `hash` (four of the sixteen values repeat an edge,
/// as in Perlin's reference implementation).
#[inline]
fn grad3<S: Scalar>(hash: u32, x: S, y: S, z: S) -> S {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    let u = if h & 1 == 0 { u } else { S::ZERO - u };
    let v = if h & 2 == 0 { v } else { S::ZERO - v };
    (u + v) * lit::<S>(core::f64::consts::FRAC_1_SQRT_2)
}

#[cfg(test)]
mod tests;
