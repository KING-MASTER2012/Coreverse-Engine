//! Fractal noise: octaves of a base noise function stacked at increasing
//! frequency and decreasing amplitude.

use crate::scalar::Scalar;

/// Fractional Brownian motion in 2D: the sum of `octaves` layers of `noise`,
/// each sampled at `lacunarity` times the frequency and weighted by `gain`
/// times the amplitude of the one before, divided by the total amplitude so
/// the result stays in the range of `noise` (`-1..=1` for this module's
/// functions).
///
/// `noise(x, y, octave)` is called once per octave with the scaled
/// coordinates and the octave number (`0, 1, ...`); mix the octave into the
/// seed (`seed.wrapping_add(octave)`) so that the layers are independent
/// instead of copies of one another at different sizes.
///
/// - `lacunarity` is how much finer each octave is; `2.0` is the usual value.
/// - `gain` (also called persistence) is how much weaker each octave is;
///   `0.5` gives the classic `1/f` look, smaller values smooth the result,
///   larger ones make it rougher.
/// - With `octaves == 0` the result is `0`.
///
/// ```
/// use math::noise::{fbm_2d, perlin_2d};
///
/// let seed = 5;
/// let n = fbm_2d(1.3_f32, 4.1, 4, 2.0, 0.5, |x, y, o| perlin_2d(x, y, seed + o));
/// assert!((-1.0..=1.0).contains(&n));
/// // One octave is the plain noise.
/// let one = fbm_2d(1.3_f32, 4.1, 1, 2.0, 0.5, |x, y, o| perlin_2d(x, y, seed + o));
/// assert_eq!(one, perlin_2d(1.3_f32, 4.1, seed));
/// ```
pub fn fbm_2d<S: Scalar>(
    x: S,
    y: S,
    octaves: u32,
    lacunarity: S,
    gain: S,
    noise: impl Fn(S, S, u32) -> S,
) -> S {
    accumulate(octaves, lacunarity, gain, |frequency, octave| {
        noise(x * frequency, y * frequency, octave)
    })
}

/// Fractional Brownian motion in 3D. See [`fbm_2d`].
pub fn fbm_3d<S: Scalar>(
    x: S,
    y: S,
    z: S,
    octaves: u32,
    lacunarity: S,
    gain: S,
    noise: impl Fn(S, S, S, u32) -> S,
) -> S {
    accumulate(octaves, lacunarity, gain, |frequency, octave| {
        noise(x * frequency, y * frequency, z * frequency, octave)
    })
}

/// Ridged multifractal noise in 2D: like [`fbm_2d`], but each octave is
/// folded into a sharp crease, `1 - |noise|`, so the result is in `0..=1` with
/// thin bright ridges where `noise` crosses zero. The shape of mountain
/// ranges, veins and lightning. With `octaves == 0` the result is `0`.
///
/// ```
/// use math::noise::{ridged_2d, simplex_2d};
///
/// let r = ridged_2d(0.9_f32, 2.4, 4, 2.0, 0.5, |x, y, o| simplex_2d(x, y, 3 + o));
/// assert!((0.0..=1.0).contains(&r));
/// ```
pub fn ridged_2d<S: Scalar>(
    x: S,
    y: S,
    octaves: u32,
    lacunarity: S,
    gain: S,
    noise: impl Fn(S, S, u32) -> S,
) -> S {
    accumulate(octaves, lacunarity, gain, |frequency, octave| {
        S::ONE - noise(x * frequency, y * frequency, octave).abs()
    })
}

/// Ridged multifractal noise in 3D. See [`ridged_2d`].
pub fn ridged_3d<S: Scalar>(
    x: S,
    y: S,
    z: S,
    octaves: u32,
    lacunarity: S,
    gain: S,
    noise: impl Fn(S, S, S, u32) -> S,
) -> S {
    accumulate(octaves, lacunarity, gain, |frequency, octave| {
        S::ONE - noise(x * frequency, y * frequency, z * frequency, octave).abs()
    })
}

/// The amplitude-weighted average of `layer(frequency, octave)` over the
/// octaves: the frequency starts at `1` and is multiplied by `lacunarity`
/// each octave, the amplitude starts at `1` and is multiplied by `gain`.
fn accumulate<S: Scalar>(octaves: u32, lacunarity: S, gain: S, layer: impl Fn(S, u32) -> S) -> S {
    let (mut sum, mut total) = (S::ZERO, S::ZERO);
    let (mut frequency, mut amplitude) = (S::ONE, S::ONE);
    for octave in 0..octaves {
        sum = sum + layer(frequency, octave) * amplitude;
        total = total + amplitude;
        frequency = frequency * lacunarity;
        amplitude = amplitude * gain;
    }
    if total > S::ZERO {
        sum / total
    } else {
        S::ZERO
    }
}
