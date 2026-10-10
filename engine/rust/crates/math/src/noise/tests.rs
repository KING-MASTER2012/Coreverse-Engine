//! Tests for the noise of Phase 2.8: range, determinism, smoothness and
//! statistics of the Perlin and simplex functions, and the fractal
//! combinators.
//!
//! Random sample points come from a fixed-seed generator, so the statistical
//! checks are deterministic; their tolerances are far wider than the
//! variation between seeds.

use core::cell::Cell;

use super::fractal::*;
use super::perlin::*;
use super::simplex::*;
use super::*;
use crate::{Pcg32, Rng};

/// The number of random points each statistical test samples.
const SAMPLES: u32 = 100_000;

/// A noise function of three coordinates and a seed (the 2D ones ignore `z`),
/// with the name and the largest slope (change per unit distance) it may
/// have, for the tests that run over all four.
struct Case {
    /// The function's name, for failure messages.
    name: &'static str,
    /// The noise function in `f64`.
    f: fn(f64, f64, f64, u32) -> f64,
    /// The same function in `f32`.
    f32: fn(f32, f32, f32, u32) -> f32,
    /// An upper bound on the function's slope, a little above the largest
    /// measured one.
    max_slope: f64,
    /// Whether the function is zero at integer lattice points.
    zero_on_lattice: bool,
}

/// The four noise functions.
fn cases() -> [Case; 4] {
    [
        Case {
            name: "perlin_2d",
            f: |x, y, _, s| perlin_2d(x, y, s),
            f32: |x, y, _, s| perlin_2d(x, y, s),
            max_slope: 4.0,
            zero_on_lattice: true,
        },
        Case {
            name: "perlin_3d",
            f: perlin_3d,
            f32: perlin_3d,
            max_slope: 4.0,
            zero_on_lattice: true,
        },
        Case {
            name: "simplex_2d",
            f: |x, y, _, s| simplex_2d(x, y, s),
            f32: |x, y, _, s| simplex_2d(x, y, s),
            max_slope: 10.0,
            zero_on_lattice: false,
        },
        Case {
            name: "simplex_3d",
            f: simplex_3d,
            f32: simplex_3d,
            max_slope: 40.0,
            zero_on_lattice: false,
        },
    ]
}

/// A coordinate in `-range..range`.
fn coord(rng: &mut Pcg32, range: f64) -> f64 {
    rng.range_f64(-range, range)
}

#[test]
fn floor_to_i32_rounds_down_and_never_panics() {
    assert_eq!(2.9_f32.floor_to_i32(), 2);
    assert_eq!((-0.1_f32).floor_to_i32(), -1);
    assert_eq!((-3.0_f64).floor_to_i32(), -3);
    assert_eq!(3.0_f64.floor_to_i32(), 3);
    assert_eq!(1e20_f32.floor_to_i32(), i32::MAX);
    assert_eq!((-1e20_f64).floor_to_i32(), i32::MIN);
    assert_eq!(f32::NAN.floor_to_i32(), 0);
    assert_eq!(f64::INFINITY.floor_to_i32(), i32::MAX);
}

#[test]
fn values_stay_in_the_unit_range() {
    let mut rng = Pcg32::new(1);
    for case in cases() {
        for i in 0..SAMPLES {
            let (x, y, z) = (
                coord(&mut rng, 200.0),
                coord(&mut rng, 200.0),
                coord(&mut rng, 200.0),
            );
            let v = (case.f)(x, y, z, i % 97);
            assert!((-1.0..=1.0).contains(&v), "{} gave {v}", case.name);
            let w = (case.f32)(x as f32, y as f32, z as f32, i % 97);
            assert!((-1.0..=1.0).contains(&w), "{} f32 gave {w}", case.name);
        }
    }
}

#[test]
fn the_scale_fills_the_range_without_exceeding_it() {
    // 2D Perlin reaches its maximum at a cell centre when all four corner
    // gradients point at it; some seed out of many has that arrangement.
    let best = (0..200_000_u32)
        .map(|seed| perlin_2d(0.5_f64, 0.5, seed).abs())
        .fold(0.0, f64::max);
    assert!((0.99..=1.0).contains(&best), "{best}");
    // The others come close on random points (and never pass 1, above).
    let mut rng = Pcg32::new(2);
    for (case, floor) in cases().into_iter().zip([0.0, 0.9, 0.99, 0.98]) {
        let largest = (0..400_000)
            .map(|i| {
                let (x, y, z) = (
                    coord(&mut rng, 4.0),
                    coord(&mut rng, 4.0),
                    coord(&mut rng, 4.0),
                );
                (case.f)(x, y, z, i % 256).abs()
            })
            .fold(0.0, f64::max);
        assert!(largest >= floor, "{} only reached {largest}", case.name);
    }
}

#[test]
fn noise_has_zero_mean_and_a_healthy_spread() {
    let mut rng = Pcg32::new(3);
    for case in cases() {
        let (mut sum, mut square) = (0.0, 0.0);
        for i in 0..SAMPLES {
            let (x, y, z) = (
                coord(&mut rng, 100.0),
                coord(&mut rng, 100.0),
                coord(&mut rng, 100.0),
            );
            let v = (case.f)(x, y, z, i % 64);
            sum += v;
            square += v * v;
        }
        let mean = sum / f64::from(SAMPLES);
        let deviation = (square / f64::from(SAMPLES) - mean * mean).sqrt();
        assert!(mean.abs() < 0.02, "{} mean {mean}", case.name);
        // Neither flat nor saturated.
        assert!(
            (0.2..0.7).contains(&deviation),
            "{} deviation {deviation}",
            case.name
        );
    }
}

#[test]
fn perlin_is_zero_on_the_lattice_and_simplex_is_not() {
    for case in cases() {
        let mut all_zero = true;
        for x in -4..4 {
            for y in -4..4 {
                for z in -4..4 {
                    let v = (case.f)(f64::from(x), f64::from(y), f64::from(z), 11);
                    all_zero &= v == 0.0;
                }
            }
        }
        assert_eq!(all_zero, case.zero_on_lattice, "{}", case.name);
    }
}

#[test]
fn noise_is_deterministic_and_depends_on_the_seed() {
    let mut rng = Pcg32::new(4);
    for case in cases() {
        let mut differing = 0;
        let n = 500;
        for _ in 0..n {
            let (x, y, z) = (
                coord(&mut rng, 50.0),
                coord(&mut rng, 50.0),
                coord(&mut rng, 50.0),
            );
            assert_eq!((case.f)(x, y, z, 5), (case.f)(x, y, z, 5), "{}", case.name);
            if (case.f)(x, y, z, 5) != (case.f)(x, y, z, 6) {
                differing += 1;
            }
        }
        assert!(differing > n * 9 / 10, "{} {differing}", case.name);
    }
}

#[test]
fn noise_is_continuous() {
    let mut rng = Pcg32::new(5);
    let step = 1e-4;
    for case in cases() {
        for i in 0..20_000 {
            let (x, y, z) = (
                coord(&mut rng, 30.0),
                coord(&mut rng, 30.0),
                coord(&mut rng, 30.0),
            );
            let v = (case.f)(x, y, z, i % 16);
            for (dx, dy, dz) in [(step, 0.0, 0.0), (0.0, step, 0.0), (0.0, 0.0, step)] {
                let w = (case.f)(x + dx, y + dy, z + dz, i % 16);
                let slope = (w - v).abs() / step;
                assert!(slope < case.max_slope, "{} slope {slope}", case.name);
            }
        }
    }
}

#[test]
fn noise_has_a_continuous_slope_across_cell_boundaries() {
    // The one-sided slopes at either side of a lattice plane agree: the fade
    // curve (Perlin) and the radial falloff (simplex) leave no crease.
    // A step small enough that the slope's own change over it (the curvature,
    // up to a few hundred for 3D simplex, times the step) stays below the bound.
    let h = 1e-5;
    for case in cases() {
        for seed in 0..40 {
            for boundary in [1.0_f64, -2.0, 7.0] {
                let (y, z) = (0.37, 0.61);
                let before = ((case.f)(boundary - h, y, z, seed)
                    - (case.f)(boundary - 2.0 * h, y, z, seed))
                    / h;
                let after = ((case.f)(boundary + 2.0 * h, y, z, seed)
                    - (case.f)(boundary + h, y, z, seed))
                    / h;
                assert!(
                    (before - after).abs() < 0.1,
                    "{} {before} {after}",
                    case.name
                );
            }
        }
    }
}

#[test]
fn simplex_has_no_crease_across_its_triangle_boundaries() {
    // The diagonal x == y of the skewed grid, for the 2D function: the one-
    // sided slopes along the normal agree.
    let h = 1e-3;
    for seed in 0..40 {
        for t in [0.2_f64, 0.45, 0.8] {
            let slope = |a: f64, b: f64| {
                (simplex_2d(t + a * h, t - a * h, seed) - simplex_2d(t + b * h, t - b * h, seed))
                    / ((a - b) * h)
            };
            assert!((slope(-1.0, -2.0) - slope(2.0, 1.0)).abs() < 0.5);
        }
    }
}

#[test]
fn coordinates_can_be_negative_or_huge_without_failing() {
    let mut rng = Pcg32::new(6);
    for case in cases() {
        for _ in 0..2_000 {
            for range in [1e3, 1e4, 1e6] {
                let (x, y, z) = (
                    coord(&mut rng, range),
                    coord(&mut rng, range),
                    coord(&mut rng, range),
                );
                let v = (case.f)(x, y, z, 3);
                assert!((-1.0..=1.0).contains(&v), "{} {v}", case.name);
            }
        }
        // Garbage coordinates must not panic; the value is `NaN` or in range.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300] {
            let v = (case.f)(bad, bad, bad, 0);
            assert!(v.is_nan() || (-1.0..=1.0).contains(&v), "{} {v}", case.name);
            let w = (case.f32)(bad as f32, 1.0, bad as f32, 0);
            assert!(w.is_nan() || (-1.0..=1.0).contains(&w), "{} {w}", case.name);
        }
    }
}

#[test]
fn f32_and_f64_agree() {
    let mut rng = Pcg32::new(7);
    for case in cases() {
        for i in 0..2_000 {
            let (x, y, z) = (
                coord(&mut rng, 20.0),
                coord(&mut rng, 20.0),
                coord(&mut rng, 20.0),
            );
            let (x32, y32, z32) = (x as f32, y as f32, z as f32);
            // Compare at the points the `f32` actually rounded to.
            let wide = (case.f)(f64::from(x32), f64::from(y32), f64::from(z32), i % 8);
            let narrow = f64::from((case.f32)(x32, y32, z32, i % 8));
            assert!(
                (wide - narrow).abs() < 1e-3,
                "{} {wide} {narrow}",
                case.name
            );
        }
    }
}

#[test]
fn the_2d_functions_are_the_xy_plane_of_nothing_in_particular() {
    // 2D and 3D noise are different fields, not slices of one another.
    assert_ne!(perlin_2d(0.4_f32, 0.7, 1), perlin_3d(0.4_f32, 0.7, 0.0, 1));
    assert_ne!(
        simplex_2d(0.4_f32, 0.7, 1),
        simplex_3d(0.4_f32, 0.7, 0.0, 1)
    );
}

#[test]
fn fbm_with_no_octaves_is_zero() {
    let one = |_: f32, _: f32, _: u32| 1.0_f32;
    assert_eq!(fbm_2d(0.3, 0.4, 0, 2.0, 0.5, one), 0.0);
    assert_eq!(ridged_2d(0.3, 0.4, 0, 2.0, 0.5, one), 0.0);
    let one3 = |_: f64, _: f64, _: f64, _: u32| 1.0_f64;
    assert_eq!(fbm_3d(0.3, 0.4, 0.5, 0, 2.0, 0.5, one3), 0.0);
    assert_eq!(ridged_3d(0.3, 0.4, 0.5, 0, 2.0, 0.5, one3), 0.0);
}

#[test]
fn a_single_octave_is_the_plain_noise() {
    let f = |x: f64, y: f64, o: u32| perlin_2d(x, y, 9 + o);
    assert_eq!(fbm_2d(1.3, 4.1, 1, 2.0, 0.5, f), perlin_2d(1.3_f64, 4.1, 9));
    let g = |x: f64, y: f64, z: f64, o: u32| simplex_3d(x, y, z, 9 + o);
    assert_eq!(
        fbm_3d(1.3, 4.1, 0.2, 1, 2.0, 0.5, g),
        simplex_3d(1.3_f64, 4.1, 0.2, 9)
    );
}

#[test]
fn fbm_normalises_by_the_total_amplitude() {
    // A constant layer keeps its value however many octaves and whatever gain.
    for (octaves, gain) in [(1, 0.5), (4, 0.5), (8, 0.7), (3, 1.0), (5, 0.0)] {
        let v = fbm_2d(0.1_f64, 0.2, octaves, 2.0, gain, |_, _, _| 0.75);
        assert!((v - 0.75).abs() < 1e-12, "{octaves} {gain} {v}");
        let w = fbm_3d(
            0.1_f32,
            0.2,
            0.3,
            octaves,
            2.0,
            gain as f32,
            |_, _, _, _| -0.5,
        );
        assert!((w + 0.5).abs() < 1e-6);
    }
    // A zero gain leaves only the first octave.
    let f = |x: f64, y: f64, o: u32| perlin_2d(x, y, 1 + o);
    assert_eq!(fbm_2d(1.3, 2.7, 6, 2.0, 0.0, f), perlin_2d(1.3_f64, 2.7, 1));
}

#[test]
fn fbm_calls_each_octave_with_its_frequency_and_index() {
    let calls = Cell::new(0_u32);
    let result = fbm_2d(1.0_f64, 2.0, 4, 3.0, 0.5, |x, y, octave| {
        // Octave `k` is sampled at `3^k` times the coordinates.
        let frequency = 3.0_f64.powi(octave as i32);
        assert_eq!((x, y), (frequency, 2.0 * frequency));
        assert_eq!(octave, calls.get());
        calls.set(calls.get() + 1);
        0.0
    });
    assert_eq!(result, 0.0);
    assert_eq!(calls.get(), 4);
    // The 3D version scales all three coordinates.
    let seen = Cell::new(0_u32);
    fbm_3d(1.0_f64, 2.0, 3.0, 3, 2.0, 0.5, |x, y, z, octave| {
        let f = f64::from(1_u32 << octave);
        assert_eq!((x, y, z), (f, 2.0 * f, 3.0 * f));
        seen.set(seen.get() + 1);
        0.0
    });
    assert_eq!(seen.get(), 3);
}

#[test]
fn fbm_stays_in_range_and_adds_detail() {
    let mut rng = Pcg32::new(8);
    let (mut rough_one, mut rough_many) = (0.0, 0.0);
    for _ in 0..5_000 {
        let (x, y) = (coord(&mut rng, 50.0), coord(&mut rng, 50.0));
        let noise = |x: f64, y: f64, o: u32| simplex_2d(x, y, 7 + o);
        let many = fbm_2d(x, y, 6, 2.0, 0.9, noise);
        assert!((-1.0..=1.0).contains(&many));
        // More octaves make the surface rougher at a fine scale.
        let step = 0.01;
        let one = fbm_2d(x, y, 1, 2.0, 0.9, noise);
        rough_one += (fbm_2d(x + step, y, 1, 2.0, 0.9, noise) - one).abs();
        rough_many += (fbm_2d(x + step, y, 6, 2.0, 0.9, noise) - many).abs();
    }
    assert!(rough_many > rough_one * 1.3, "{rough_one} {rough_many}");
}

#[test]
fn ridged_noise_is_in_zero_to_one() {
    let mut rng = Pcg32::new(9);
    for _ in 0..5_000 {
        let (x, y, z) = (
            coord(&mut rng, 50.0),
            coord(&mut rng, 50.0),
            coord(&mut rng, 50.0),
        );
        let r2 = ridged_2d(x, y, 5, 2.0, 0.5, |x, y, o| perlin_2d(x, y, 2 + o));
        assert!((0.0..=1.0).contains(&r2), "{r2}");
        let r3 = ridged_3d(x, y, z, 5, 2.0, 0.5, |x, y, z, o| {
            simplex_3d(x, y, z, 2 + o)
        });
        assert!((0.0..=1.0).contains(&r3), "{r3}");
    }
    // The crease: no noise gives a full ridge, extreme noise gives none.
    assert_eq!(ridged_2d(0.0_f32, 0.0, 3, 2.0, 0.5, |_, _, _| 0.0), 1.0);
    assert_eq!(ridged_2d(0.0_f32, 0.0, 3, 2.0, 0.5, |_, _, _| 1.0), 0.0);
    assert_eq!(ridged_2d(0.0_f32, 0.0, 3, 2.0, 0.5, |_, _, _| -1.0), 0.0);
}

#[test]
fn octaves_are_independent_layers() {
    // Mixing the octave into the seed gives different layers; sampling one
    // field at several scales (ignoring the octave) correlates them, so the
    // two stacks must differ.
    let (x, y) = (3.37_f64, 8.91);
    let independent = fbm_2d(x, y, 4, 2.0, 0.5, |x, y, o| perlin_2d(x, y, 5 + o));
    let correlated = fbm_2d(x, y, 4, 2.0, 0.5, |x, y, _| perlin_2d(x, y, 5));
    assert_ne!(independent, correlated);
}
