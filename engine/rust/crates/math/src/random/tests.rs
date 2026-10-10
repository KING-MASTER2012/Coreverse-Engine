//! Tests for the random numbers of Phase 2.8: the reference outputs of both
//! generators, the sampling methods (ranges, floats, unit vectors, shuffles)
//! and the stateless hashes.
//!
//! The statistical checks run on fixed seeds, so they are deterministic: the
//! tolerances are several standard deviations wide, which makes a failure a
//! real change of behaviour, never bad luck.

use super::*;

/// A generator that replays a fixed list of 32-bit values and then repeats the
/// last one, to drive the sampling methods through edge cases.
struct Script<'a> {
    /// The values to hand out.
    values: &'a [u32],
    /// How many have been handed out.
    index: usize,
}

impl<'a> Script<'a> {
    /// A generator that hands out `values` in order.
    fn new(values: &'a [u32]) -> Self {
        Self { values, index: 0 }
    }
}

impl Rng for Script<'_> {
    fn next_u32(&mut self) -> u32 {
        let value = self.values[self.index.min(self.values.len() - 1)];
        self.index += 1;
        value
    }
}

#[test]
fn splitmix64_matches_the_reference_sequence() {
    // The published outputs of splitmix64.c for the seed 0.
    let mut rng = SplitMix64::new(0);
    assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);
    assert_eq!(rng.next_u64(), 0xF88B_B8A8_724C_81EC);
}

#[test]
fn splitmix64_state_resumes_the_sequence() {
    let mut a = SplitMix64::new(99);
    a.next_u64();
    a.next_u64();
    let mut b = SplitMix64::new(a.state());
    assert_eq!(a.next_u64(), b.next_u64());
    // `next_u32` is the high half of the next 64 bits.
    let mut c = SplitMix64::new(5);
    let mut d = SplitMix64::new(5);
    assert_eq!(u64::from(c.next_u32()), d.next_u64() >> 32);
}

#[test]
fn pcg32_matches_the_reference_sequence() {
    // The output of the reference `pcg32-demo` after `pcg32_srandom(42, 54)`.
    let mut rng = Pcg32::from_state_and_stream(42, 54);
    let expected = [
        0xA15C_02B7_u32,
        0x7B47_F409,
        0xBA1D_3330,
        0x83D2_F293,
        0xBFA4_784B,
        0xCBED_606E,
    ];
    for value in expected {
        assert_eq!(rng.next_u32(), value);
    }
}

#[test]
fn pcg32_seeding_is_deterministic_and_seed_sensitive() {
    let mut a = Pcg32::new(1234);
    let mut b = Pcg32::new(1234);
    let first: [u32; 8] = core::array::from_fn(|_| a.next_u32());
    let again: [u32; 8] = core::array::from_fn(|_| b.next_u32());
    assert_eq!(first, again);
    assert_eq!(Pcg32::new(1234), Pcg32::new(1234));
    // Consecutive seeds give unrelated sequences.
    for seed in 0..20_u64 {
        let mut x = Pcg32::new(seed);
        let mut y = Pcg32::new(seed + 1);
        let xs: [u32; 4] = core::array::from_fn(|_| x.next_u32());
        let ys: [u32; 4] = core::array::from_fn(|_| y.next_u32());
        assert!(xs.iter().zip(&ys).all(|(p, q)| p != q), "{seed}");
    }
}

#[test]
fn pcg32_streams_are_independent() {
    let mut a = Pcg32::from_state_and_stream(7, 1);
    let mut b = Pcg32::from_state_and_stream(7, 2);
    let xs: [u32; 8] = core::array::from_fn(|_| a.next_u32());
    let ys: [u32; 8] = core::array::from_fn(|_| b.next_u32());
    assert_ne!(xs, ys);
    // Only the low 63 bits of the stream matter: the top bit is shifted out.
    assert_eq!(
        Pcg32::from_state_and_stream(7, 1),
        Pcg32::from_state_and_stream(7, 1 | (1 << 63))
    );
}

#[test]
fn pcg32_next_u64_is_two_draws_high_half_first() {
    let mut a = Pcg32::new(3);
    let mut b = Pcg32::new(3);
    let high = u64::from(a.next_u32());
    let low = u64::from(a.next_u32());
    assert_eq!(b.next_u64(), (high << 32) | low);
}

#[test]
fn clones_replay_the_sequence() {
    let mut a = Pcg32::new(11);
    a.next_u32();
    let mut b = a.clone();
    assert_eq!(a, b);
    for _ in 0..10 {
        assert_eq!(a.next_u32(), b.next_u32());
    }
}

#[test]
fn generators_can_run_in_const_contexts() {
    const FIRST: u32 = {
        let mut rng = Pcg32::new(7);
        rng.next_u32()
    };
    const TABLE: [u64; 3] = {
        let mut rng = SplitMix64::new(0);
        [rng.next_u64(), rng.next_u64(), rng.next_u64()]
    };
    assert_eq!(FIRST, Pcg32::new(7).next_u32());
    assert_eq!(TABLE[0], 0xE220_A839_7B1D_CDAF);
    assert_eq!(TABLE[2], 0x06C4_5D18_8009_454F);
}

#[test]
fn the_default_u64_is_two_u32_draws_high_half_first() {
    let mut rng = Script::new(&[0xAAAA_BBBB, 0xCCCC_DDDD]);
    assert_eq!(rng.next_u64(), 0xAAAA_BBBB_CCCC_DDDD);
}

#[test]
fn floats_are_in_the_half_open_unit_interval() {
    let mut low = Script::new(&[0]);
    assert_eq!(low.next_f32(), 0.0);
    assert_eq!(low.next_f64(), 0.0);
    let mut high = Script::new(&[u32::MAX]);
    let f = high.next_f32();
    assert!(f < 1.0 && f > 0.999_999);
    assert_eq!(f, 1.0 - 1.0 / 16_777_216.0);
    let d = high.next_f64();
    assert!(d < 1.0 && d > 0.999_999_999_999);
    assert_eq!(d, 1.0 - 1.0 / 9_007_199_254_740_992.0);
}

#[test]
fn floats_have_the_right_mean_and_spread() {
    let mut rng = Pcg32::new(1);
    let (mut sum, mut sum_sq) = (0.0_f64, 0.0_f64);
    let n = 100_000;
    for _ in 0..n {
        let x = f64::from(rng.next_f32());
        sum += x;
        sum_sq += x * x;
    }
    let mean = sum / f64::from(n);
    let variance = sum_sq / f64::from(n) - mean * mean;
    assert!((mean - 0.5).abs() < 0.005, "{mean}");
    assert!((variance - 1.0 / 12.0).abs() < 0.002, "{variance}");
    let mean64 = (0..n).map(|_| rng.next_f64()).sum::<f64>() / f64::from(n);
    assert!((mean64 - 0.5).abs() < 0.005, "{mean64}");
}

#[test]
fn bools_and_chances_have_the_right_frequency() {
    let mut rng = Pcg32::new(2);
    let n = 40_000;
    let heads = (0..n).filter(|_| rng.next_bool()).count();
    assert!((heads as i32 - n / 2).abs() < 800, "{heads}");
    let hits = (0..n).filter(|_| rng.chance(0.25)).count();
    assert!((hits as i32 - n / 4).abs() < 700, "{hits}");
    assert!((0..100).all(|_| !rng.chance(0.0)));
    assert!((0..100).all(|_| rng.chance(1.0)));
    assert!((0..100).all(|_| !rng.chance(f64::NAN)));
    assert!((0..100).all(|_| !rng.chance(-1.0)));
}

#[test]
fn below_is_uniform() {
    let mut rng = Pcg32::new(3);
    let mut counts = [0_u32; 6];
    for _ in 0..60_000 {
        counts[rng.below_u32(6) as usize] += 1;
    }
    // 10 000 expected per face, a standard deviation of about 91.
    assert!(
        counts.iter().all(|&c| c.abs_diff(10_000) < 500),
        "{counts:?}"
    );
    let mut counts = [0_u32; 7];
    for _ in 0..70_000 {
        counts[rng.below_u64(7) as usize] += 1;
    }
    assert!(
        counts.iter().all(|&c| c.abs_diff(10_000) < 500),
        "{counts:?}"
    );
}

#[test]
fn below_handles_degenerate_and_huge_bounds() {
    let mut rng = Pcg32::new(4);
    for _ in 0..100 {
        assert_eq!(rng.below_u32(0), 0);
        assert_eq!(rng.below_u32(1), 0);
        assert_eq!(rng.below_u64(0), 0);
        assert_eq!(rng.below_u64(1), 0);
        assert!(rng.below_u32(u32::MAX) < u32::MAX);
        assert!(rng.below_u64(u64::MAX) < u64::MAX);
        assert!(rng.below_u32(3_000_000_000) < 3_000_000_000);
    }
}

#[test]
fn below_rejects_the_biased_draws() {
    // With `bound = 3` the threshold is `2^32 mod 3 == 1`: the draw `0` has
    // a product with a zero low half and must be rejected and redrawn.
    let mut rng = Script::new(&[0, 0xFFFF_FFFF]);
    assert_eq!(rng.below_u32(3), 2);
    assert_eq!(rng.index, 2);
    // A draw that is accepted is used as it is.
    let mut rng = Script::new(&[0x8000_0000]);
    assert_eq!(rng.below_u32(3), 1);
    assert_eq!(rng.index, 1);
}

#[test]
fn integer_ranges_stay_inside_and_are_half_open() {
    let mut rng = Pcg32::new(5);
    let mut seen = [false; 5];
    for _ in 0..1_000 {
        let v = rng.range_u32(10, 15);
        assert!((10..15).contains(&v));
        seen[(v - 10) as usize] = true;
        assert!((-3..3).contains(&rng.range_i32(-3, 3)));
        assert!((10..15).contains(&rng.range_u64(10, 15)));
        assert!((-3..3).contains(&rng.range_i64(-3, 3)));
        assert!((2..9).contains(&rng.range_usize(2, 9)));
    }
    assert!(seen.iter().all(|&s| s), "every value of the range is drawn");
}

#[test]
fn integer_ranges_handle_the_extremes() {
    let mut rng = Pcg32::new(6);
    for _ in 0..200 {
        // The full width does not fit in an `i32`/`i64`, only in the unsigned type.
        let a = rng.range_i32(i32::MIN, i32::MAX);
        assert!(a < i32::MAX);
        let b = rng.range_i64(i64::MIN, i64::MAX);
        assert!(b < i64::MAX);
        assert!((u32::MAX - 1..u32::MAX).contains(&rng.range_u32(u32::MAX - 1, u32::MAX)));
        assert_eq!(rng.range_i32(i32::MAX - 1, i32::MAX), i32::MAX - 1);
    }
    // Negative and positive halves are both reached.
    let draws: [i32; 64] = core::array::from_fn(|_| rng.range_i32(i32::MIN, i32::MAX));
    assert!(draws.iter().any(|&v| v < 0) && draws.iter().any(|&v| v > 0));
}

#[test]
fn empty_ranges_return_the_lower_bound() {
    let mut rng = Pcg32::new(7);
    assert_eq!(rng.range_u32(5, 5), 5);
    assert_eq!(rng.range_u32(5, 2), 5);
    assert_eq!(rng.range_i32(-1, -1), -1);
    assert_eq!(rng.range_i32(4, -4), 4);
    assert_eq!(rng.range_u64(9, 9), 9);
    assert_eq!(rng.range_i64(3, -3), 3);
    assert_eq!(rng.range_usize(8, 1), 8);
    assert_eq!(rng.range_f32(2.0, 2.0), 2.0);
    assert_eq!(rng.range_f32(2.0, 1.0), 2.0);
}

#[test]
fn float_ranges_are_half_open_and_cover_the_range() {
    let mut rng = Pcg32::new(8);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for _ in 0..10_000 {
        let v = rng.range_f32(-2.0, 6.0);
        assert!((-2.0..6.0).contains(&v));
        lo = lo.min(v);
        hi = hi.max(v);
        let d = rng.range_f64(-2.0, 6.0);
        assert!((-2.0..6.0).contains(&d));
    }
    assert!(lo < -1.9 && hi > 5.9);
    assert_eq!(rng.range_f64(1.0, 1.0), 1.0);
    assert!(rng.range_f64(f64::NAN, 1.0).is_nan());
    assert!(rng.range_f32(f32::NAN, 1.0).is_nan());
    assert_eq!(rng.range_f32(1.0, f32::NAN), 1.0);
}

#[test]
fn float_ranges_never_return_the_upper_bound() {
    // The largest draw, in a range so narrow that `lo + width * u` rounds to `hi`.
    let mut rng = Script::new(&[u32::MAX]);
    let (lo, hi) = (1.0_f32, 1.0 + f32::EPSILON);
    let v = rng.range_f32(lo, hi);
    assert!(v >= lo && v < hi, "{v}");
    let mut rng = Script::new(&[u32::MAX, u32::MAX]);
    let (lo, hi) = (1.0_f64, 1.0 + f64::EPSILON);
    let v = rng.range_f64(lo, hi);
    assert!(v >= lo && v < hi, "{v}");
}

#[test]
fn unit_vectors_have_unit_length() {
    let mut rng = Pcg32::new(9);
    for _ in 0..5_000 {
        assert!((rng.unit_vec2().length() - 1.0).abs() < 1e-5);
        assert!((rng.unit_vec3().length() - 1.0).abs() < 1e-5);
        assert!((rng.unit_dvec2().length() - 1.0).abs() < 1e-12);
        assert!((rng.unit_dvec3().length() - 1.0).abs() < 1e-12);
    }
}

#[test]
fn unit_vectors_are_uniformly_distributed() {
    let mut rng = Pcg32::new(10);
    let n = 40_000;
    let (mut sum2, mut sum3) = (Vec2::ZERO, Vec3::ZERO);
    let (mut xx, mut zz) = (0.0_f64, 0.0_f64);
    let mut octants = [0_u32; 8];
    for _ in 0..n {
        let a = rng.unit_vec2();
        sum2 += a;
        xx += f64::from(a.x * a.x);
        let b = rng.unit_vec3();
        sum3 += b;
        zz += f64::from(b.z * b.z);
        let octant =
            usize::from(b.x > 0.0) | usize::from(b.y > 0.0) << 1 | usize::from(b.z > 0.0) << 2;
        octants[octant] += 1;
    }
    let n_f = n as f32;
    // No preferred direction: the mean is zero.
    assert!(sum2.length() / n_f < 0.02, "{sum2:?}");
    assert!(sum3.length() / n_f < 0.02, "{sum3:?}");
    // The second moments of a uniform circle and sphere: 1/2 and 1/3.
    assert!((xx / f64::from(n) - 0.5).abs() < 0.01, "{xx}");
    assert!((zz / f64::from(n) - 1.0 / 3.0).abs() < 0.01, "{zz}");
    // All eight octants are equally likely (5000 expected, deviation ~66).
    assert!(
        octants.iter().all(|&c| c.abs_diff(5_000) < 400),
        "{octants:?}"
    );
}

#[test]
fn double_precision_unit_vectors_are_uniformly_distributed() {
    let mut rng = Pcg32::new(11);
    let n = 40_000;
    let (mut sum2, mut sum3) = (DVec2::ZERO, DVec3::ZERO);
    let mut zz = 0.0;
    for _ in 0..n {
        sum2 += rng.unit_dvec2();
        let b = rng.unit_dvec3();
        sum3 += b;
        zz += b.z * b.z;
    }
    assert!(sum2.length() / f64::from(n) < 0.02);
    assert!(sum3.length() / f64::from(n) < 0.02);
    assert!((zz / f64::from(n) - 1.0 / 3.0).abs() < 0.01);
}

#[test]
fn disk_and_ball_samples_are_inside_and_uniform() {
    let mut rng = Pcg32::new(12);
    let n = 40_000;
    let (mut r2_disk, mut r2_ball) = (0.0_f64, 0.0_f64);
    let (mut d2_disk, mut d2_ball) = (0.0_f64, 0.0_f64);
    for _ in 0..n {
        let p = rng.vec2_in_unit_disk();
        assert!(p.length() < 1.0);
        r2_disk += f64::from(p.length_sq());
        let q = rng.vec3_in_unit_ball();
        assert!(q.length() < 1.0);
        r2_ball += f64::from(q.length_sq());
        let p = rng.dvec2_in_unit_disk();
        assert!(p.length() < 1.0);
        d2_disk += p.length_sq();
        let q = rng.dvec3_in_unit_ball();
        assert!(q.length() < 1.0);
        d2_ball += q.length_sq();
    }
    // The mean squared radius of a uniform disk is 1/2 and of a ball 3/5.
    let n = f64::from(n);
    assert!((r2_disk / n - 0.5).abs() < 0.01, "{}", r2_disk / n);
    assert!((r2_ball / n - 0.6).abs() < 0.01, "{}", r2_ball / n);
    assert!((d2_disk / n - 0.5).abs() < 0.01);
    assert!((d2_ball / n - 0.6).abs() < 0.01);
}

#[test]
fn vector_samplers_are_deterministic() {
    let mut a = Pcg32::new(13);
    let mut b = Pcg32::new(13);
    for _ in 0..50 {
        assert_eq!(a.unit_vec3(), b.unit_vec3());
        assert_eq!(a.vec2_in_unit_disk(), b.vec2_in_unit_disk());
        assert_eq!(a.unit_dvec2(), b.unit_dvec2());
    }
}

#[test]
fn shuffle_is_a_uniform_permutation() {
    let mut rng = Pcg32::new(14);
    let mut counts = [0_u32; 6];
    for _ in 0..6_000 {
        let mut items = [0, 1, 2];
        rng.shuffle(&mut items);
        let mut sorted = items;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2]);
        // The index of this permutation among the six.
        let index = match items {
            [0, 1, 2] => 0,
            [0, 2, 1] => 1,
            [1, 0, 2] => 2,
            [1, 2, 0] => 3,
            [2, 0, 1] => 4,
            _ => 5,
        };
        counts[index] += 1;
    }
    // 1000 expected each, a deviation of about 29.
    assert!(
        counts.iter().all(|&c| c.abs_diff(1_000) < 150),
        "{counts:?}"
    );
}

#[test]
fn shuffle_handles_short_slices_and_is_deterministic() {
    let mut rng = Pcg32::new(15);
    let mut empty: [u8; 0] = [];
    rng.shuffle(&mut empty);
    let mut one = [7];
    rng.shuffle(&mut one);
    assert_eq!(one, [7]);
    let mut a = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut b = a;
    Pcg32::new(16).shuffle(&mut a);
    Pcg32::new(16).shuffle(&mut b);
    assert_eq!(a, b);
    assert_ne!(a, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn choose_picks_every_element_and_none_from_empty() {
    let mut rng = Pcg32::new(17);
    let items = ["a", "b", "c", "d"];
    let mut seen = [false; 4];
    for _ in 0..200 {
        let picked = rng.choose(&items).unwrap();
        seen[items.iter().position(|i| i == picked).unwrap()] = true;
    }
    assert!(seen.iter().all(|&s| s));
    let none: [u8; 0] = [];
    assert_eq!(rng.choose(&none), None);
}

#[test]
fn a_mutable_reference_is_a_generator_too() {
    /// Draws through a generic `Rng`, to prove `&mut R` satisfies the bound.
    fn draw(mut rng: impl Rng) -> (u32, u64) {
        (rng.next_u32(), rng.next_u64())
    }
    let mut a = Pcg32::new(18);
    let mut b = Pcg32::new(18);
    let through_reference = draw(&mut a);
    let direct = (b.next_u32(), b.next_u64());
    assert_eq!(through_reference, direct);
    // The generator advanced, so the reference really was used.
    assert_eq!(a, b);
    // And trait objects work for the non-generic methods.
    let object: &mut dyn Rng = &mut Pcg32::new(19);
    assert!(object.next_f32() < 1.0);
    assert!(object.range_u32(0, 10) < 10);
}

#[test]
fn mix_functions_are_scrambling_bijections() {
    assert_eq!(mix32(0), 0);
    assert_eq!(mix64(0), 0);
    // Avalanche: flipping one input bit flips about half of the output bits.
    let mut total32 = 0;
    let mut total64 = 0;
    let mut cases = 0;
    for x in 1..200_u32 {
        for bit in 0..32 {
            total32 += (mix32(x) ^ mix32(x ^ (1 << bit))).count_ones();
            cases += 1;
        }
        for bit in 0..64 {
            let y = u64::from(x) * 0x1_0000_0001;
            total64 += (mix64(y) ^ mix64(y ^ (1 << bit))).count_ones();
        }
    }
    let mean32 = f64::from(total32) / f64::from(cases);
    let mean64 = f64::from(total64) / f64::from(cases * 2);
    assert!((mean32 - 16.0).abs() < 0.5, "{mean32}");
    assert!((mean64 - 32.0).abs() < 0.8, "{mean64}");
    // No collisions among a few thousand consecutive inputs.
    let mut seen = [0_u32; 4096];
    for (i, slot) in seen.iter_mut().enumerate() {
        *slot = mix32(i as u32);
    }
    seen.sort_unstable();
    assert!(seen.windows(2).all(|w| w[0] != w[1]));
}

#[test]
fn coordinate_hashes_depend_on_every_input() {
    assert_eq!(hash2(1, 2, 3), hash2(1, 2, 3));
    assert_ne!(hash2(1, 2, 3), hash2(2, 2, 3), "seed");
    assert_ne!(hash2(1, 2, 3), hash2(1, 3, 3), "x");
    assert_ne!(hash2(1, 2, 3), hash2(1, 2, 4), "y");
    assert_ne!(hash2(1, 2, 3), hash2(1, 3, 2), "order");
    assert_eq!(hash3(1, 2, 3, 4), hash3(1, 2, 3, 4));
    assert_ne!(hash3(1, 2, 3, 4), hash3(2, 2, 3, 4), "seed");
    assert_ne!(hash3(1, 2, 3, 4), hash3(1, 3, 3, 4), "x");
    assert_ne!(hash3(1, 2, 3, 4), hash3(1, 2, 4, 4), "y");
    assert_ne!(hash3(1, 2, 3, 4), hash3(1, 2, 3, 5), "z");
    assert_ne!(hash3(1, 1, 2, 3), hash3(1, 3, 2, 1), "order");
    // Negative coordinates hash like any others.
    assert_ne!(hash2(0, -1, 0), hash2(0, 1, 0));
}

#[test]
fn coordinate_hashes_are_uniform_over_a_grid() {
    // The low three bits (what the noise functions pick gradients with) of
    // 64 x 64 neighbouring cells: 512 expected per value, deviation ~21.
    let mut counts = [0_u32; 8];
    for x in -32..32 {
        for y in -32..32 {
            counts[(hash2(77, x, y) & 7) as usize] += 1;
        }
    }
    assert!(counts.iter().all(|&c| c.abs_diff(512) < 100), "{counts:?}");
    let mut counts = [0_u32; 8];
    for x in -8..8 {
        for y in -8..8 {
            for z in -8..8 {
                counts[(hash3(77, x, y, z) >> 29) as usize] += 1;
            }
        }
    }
    assert!(counts.iter().all(|&c| c.abs_diff(512) < 100), "{counts:?}");
}

#[test]
fn hash_to_f32_covers_the_unit_interval() {
    assert_eq!(hash_to_f32(0), 0.0);
    assert!(hash_to_f32(u32::MAX) < 1.0);
    assert_eq!(hash_to_f32(1 << 31), 0.5);
    const HALF: f32 = hash_to_f32(1 << 31);
    assert_eq!(HALF, 0.5);
    let mean = (0..10_000)
        .map(|i| f64::from(hash_to_f32(hash2(5, i, 0))))
        .sum::<f64>()
        / 10_000.0;
    assert!((mean - 0.5).abs() < 0.02, "{mean}");
}

#[cfg(all(feature = "serde", feature = "std"))]
#[test]
fn generators_serialize_and_resume() {
    let mut rng = Pcg32::new(21);
    rng.next_u32();
    let json = serde_json::to_string(&rng).unwrap();
    let mut back: Pcg32 = serde_json::from_str(&json).unwrap();
    assert_eq!(back, rng);
    for _ in 0..10 {
        assert_eq!(back.next_u32(), rng.next_u32());
    }
    let mut split = SplitMix64::new(3);
    split.next_u64();
    let json = serde_json::to_string(&split).unwrap();
    assert_eq!(json, format!("{{\"state\":{}}}", split.state()));
    let mut back: SplitMix64 = serde_json::from_str(&json).unwrap();
    assert_eq!(back.next_u64(), split.next_u64());
}
