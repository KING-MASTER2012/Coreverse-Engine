//! Deterministic, dependency-free random numbers (Phase 2.8): the
//! [`Pcg32`] and [`SplitMix64`] generators, the [`Rng`] trait with sampling
//! of ranges, floats, booleans, unit vectors and shuffles on top of them,
//! and stateless integer hashing ([`mix32`], [`hash2`], [`hash3`]).
//!
//! # Determinism
//!
//! Everything here is reproducible: the same seed gives the same numbers on
//! every platform, build and run, and the outputs of a given version are part
//! of this crate's contract. That is what lets a game replay a recording,
//! rebuild a procedural level from its seed or keep a lock-step simulation in
//! sync. The generators use only integer arithmetic; floats are made by exact
//! conversions and the vector samplers use rejection sampling and a single
//! square root (which IEEE 754 defines exactly) instead of `sin`/`cos`, whose
//! last digit differs between platforms.
//!
//! The module is `no_std` and allocation-free, and has no dependencies.
//!
//! ```
//! use math::{Pcg32, Rng};
//!
//! let mut rng = Pcg32::new(2024);
//! let roll = rng.range_u32(1, 7); // a die: 1..=6
//! assert!((1..=6).contains(&roll));
//!
//! let heading = rng.unit_vec2(); // a random direction
//! assert!((heading.length() - 1.0).abs() < 1e-5);
//!
//! let mut deck = [1, 2, 3, 4, 5];
//! rng.shuffle(&mut deck);
//! deck.sort();
//! assert_eq!(deck, [1, 2, 3, 4, 5]); // a shuffle is a permutation
//! ```
//!
//! # Which generator?
//!
//! - [`Pcg32`] is the general-purpose one: small, fast and statistically
//!   strong, with selectable streams for independent sequences.
//! - [`SplitMix64`] is for seeding and for deriving seeds (`SplitMix64::new(
//!   n).next_u64()` turns a counter into a well-mixed 64-bit value).
//!
//! Neither is cryptographically secure: never use them for secrets, tokens or
//! anything an attacker benefits from predicting.
//!
//! # Stateless hashing
//!
//! For randomness that must depend only on *where* something is, not on how
//! many numbers were drawn before (placing a tree per grid cell, jittering a
//! pixel), generators are the wrong tool, since the result would depend on the
//! order of evaluation. [`hash2`] and [`hash3`] turn integer coordinates and a
//! seed straight into well-mixed bits, and [`hash_to_f32`] turns those into a
//! float in `0..1`.

mod generators;

pub use generators::{Pcg32, SplitMix64};

use crate::{DVec2, DVec3, Scalar, Vec2, Vec3};

/// Scrambles the 32 bits of `x` so that every output bit depends on every
/// input bit (the "lowbias32" finalizer by Chris Wellons).
///
/// It is a bijection (distinct inputs give distinct outputs), maps `0` to
/// `0`, and is a `const fn`.
///
/// ```
/// use math::random::mix32;
///
/// assert_eq!(mix32(0), 0);
/// assert_ne!(mix32(1), mix32(2));
/// const SCRAMBLED: u32 = mix32(12345);
/// assert_eq!(SCRAMBLED, mix32(12345));
/// ```
#[inline]
pub const fn mix32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^ (x >> 16)
}

/// Scrambles the 64 bits of `x` so that every output bit depends on every
/// input bit (the finalizer of SplitMix64, the same as MurmurHash3's
/// `fmix64` with different constants).
///
/// It is a bijection (distinct inputs give distinct outputs), maps `0` to
/// `0`, and is a `const fn`.
#[inline]
pub const fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A hash of the 2D integer coordinates `(x, y)` under `seed`: well-mixed
/// 32 bits that depend on all three inputs, with no visible pattern between
/// neighbouring cells.
///
/// Deterministic and stateless, so it is the right source of per-cell
/// randomness (and the lattice hash of this crate's noise functions).
/// `hash2(s, x, y) != hash2(s, y, x)` in general.
///
/// ```
/// use math::random::{hash2, hash_to_f32};
///
/// // A reproducible pseudo-random value per grid cell.
/// let jitter = hash_to_f32(hash2(42, 10, -3));
/// assert!((0.0..1.0).contains(&jitter));
/// assert_eq!(hash2(42, 10, -3), hash2(42, 10, -3));
/// ```
#[inline]
pub const fn hash2(seed: u32, x: i32, y: i32) -> u32 {
    let mut h = mix32(seed ^ 0x9E37_79B9);
    h = mix32(h ^ x as u32);
    mix32(h ^ y as u32)
}

/// A hash of the 3D integer coordinates `(x, y, z)` under `seed`; the 3D
/// counterpart of [`hash2`].
#[inline]
pub const fn hash3(seed: u32, x: i32, y: i32, z: i32) -> u32 {
    let mut h = mix32(seed ^ 0x9E37_79B9);
    h = mix32(h ^ x as u32);
    h = mix32(h ^ y as u32);
    mix32(h ^ z as u32)
}

/// The float in `0..1` made from the top 24 bits of `bits`: every `f32`
/// multiple of `2^-24` below `1` is produced with equal probability by
/// uniform input, and `1.0` never is.
#[inline]
pub const fn hash_to_f32(bits: u32) -> f32 {
    (bits >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// A source of random bits and the sampling methods built on them.
///
/// Implementors only provide [`Rng::next_u32`]; everything else has a default
/// implementation (override [`Rng::next_u64`] when 64 bits come cheaper than
/// two 32-bit draws). It is implemented by [`Pcg32`] and [`SplitMix64`], and
/// for `&mut R` so a generator can be passed down by reference.
///
/// No method panics. Where a request has no valid answer (an empty range, a
/// bound of zero, an empty slice) the method returns its documented neutral
/// value instead.
pub trait Rng {
    /// The next 32 random bits.
    fn next_u32(&mut self) -> u32;

    /// The next 64 random bits: by default two [`Rng::next_u32`] draws, the
    /// first as the high half.
    fn next_u64(&mut self) -> u64 {
        let high = u64::from(self.next_u32());
        let low = u64::from(self.next_u32());
        (high << 32) | low
    }

    /// A fair coin flip.
    #[inline]
    fn next_bool(&mut self) -> bool {
        // The top bit: the best mixed one for many generators.
        self.next_u32() >> 31 != 0
    }

    /// `true` with the given `probability`: always for `1` or more, never for
    /// `0` or less (or `NaN`).
    #[inline]
    fn chance(&mut self, probability: f64) -> bool {
        self.next_f64() < probability
    }

    /// A float in `0..1` (`1.0` excluded) with 24 random bits, every multiple
    /// of `2^-24` equally likely.
    #[inline]
    fn next_f32(&mut self) -> f32 {
        hash_to_f32(self.next_u32())
    }

    /// A float in `0..1` (`1.0` excluded) with 53 random bits, every multiple
    /// of `2^-53` equally likely.
    #[inline]
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// A uniformly distributed integer in `0..bound`, or `0` if `bound` is
    /// `0` or `1`.
    ///
    /// Free of the modulo bias of `next_u32() % bound`: it uses Lemire's
    /// multiply-and-reject method, which almost never divides and rejects a
    /// draw with probability below `bound / 2^32`.
    fn below_u32(&mut self, bound: u32) -> u32 {
        let mut product = u64::from(self.next_u32()) * u64::from(bound);
        let mut low = product as u32;
        if low < bound {
            // Draws below `2^32 mod bound` would over-represent some results.
            let threshold = bound.wrapping_neg() % bound;
            while low < threshold {
                product = u64::from(self.next_u32()) * u64::from(bound);
                low = product as u32;
            }
        }
        (product >> 32) as u32
    }

    /// A uniformly distributed integer in `0..bound`, or `0` if `bound` is
    /// `0` or `1`. Unbiased like [`Rng::below_u32`].
    fn below_u64(&mut self, bound: u64) -> u64 {
        let mut product = u128::from(self.next_u64()) * u128::from(bound);
        let mut low = product as u64;
        if low < bound {
            let threshold = bound.wrapping_neg() % bound;
            while low < threshold {
                product = u128::from(self.next_u64()) * u128::from(bound);
                low = product as u64;
            }
        }
        (product >> 64) as u64
    }

    /// A uniformly distributed integer in `lo..hi` (`hi` excluded), or `lo`
    /// if the range is empty (`hi <= lo`).
    #[inline]
    fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            lo
        } else {
            lo + self.below_u32(hi - lo)
        }
    }

    /// A uniformly distributed integer in `lo..hi` (`hi` excluded), or `lo`
    /// if the range is empty (`hi <= lo`). Works across the whole `i32`
    /// range, `i32::MIN..i32::MAX` included.
    #[inline]
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            lo
        } else {
            // The width fits in a `u32` even when `hi - lo` overflows an `i32`.
            let width = (hi as u32).wrapping_sub(lo as u32);
            lo.wrapping_add(self.below_u32(width) as i32)
        }
    }

    /// A uniformly distributed integer in `lo..hi` (`hi` excluded), or `lo`
    /// if the range is empty (`hi <= lo`).
    #[inline]
    fn range_u64(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            lo
        } else {
            lo + self.below_u64(hi - lo)
        }
    }

    /// A uniformly distributed integer in `lo..hi` (`hi` excluded), or `lo`
    /// if the range is empty (`hi <= lo`). Works across the whole `i64`
    /// range.
    #[inline]
    fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            lo
        } else {
            let width = (hi as u64).wrapping_sub(lo as u64);
            lo.wrapping_add(self.below_u64(width) as i64)
        }
    }

    /// A uniformly distributed index in `lo..hi` (`hi` excluded), or `lo` if
    /// the range is empty (`hi <= lo`).
    #[inline]
    fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.range_u64(lo as u64, hi as u64) as usize
    }

    /// A uniformly distributed float in `lo..hi` (`hi` excluded), or `lo` if
    /// the range is empty or either bound is `NaN`. The width `hi - lo` must
    /// be finite.
    #[inline]
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        if lo < hi {
            let value = lo + (hi - lo) * self.next_f32();
            // Rounding can land exactly on `hi`; keep the range half-open.
            if value < hi { value } else { hi.next_down() }
        } else {
            // An empty range, or a `NaN` bound.
            lo
        }
    }

    /// A uniformly distributed float in `lo..hi` (`hi` excluded), or `lo` if
    /// the range is empty or either bound is `NaN`. The width `hi - lo` must
    /// be finite.
    #[inline]
    fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        if lo < hi {
            let value = lo + (hi - lo) * self.next_f64();
            if value < hi { value } else { hi.next_down() }
        } else {
            lo
        }
    }

    /// A point strictly inside the unit disk (`length() < 1`), uniformly
    /// distributed over its area, by rejection sampling.
    fn vec2_in_unit_disk(&mut self) -> Vec2 {
        loop {
            let x = self.next_f32() * 2.0 - 1.0;
            let y = self.next_f32() * 2.0 - 1.0;
            if x * x + y * y < 1.0 {
                return Vec2::new(x, y);
            }
        }
    }

    /// A point strictly inside the unit disk (`length() < 1`), uniformly
    /// distributed over its area, by rejection sampling.
    fn dvec2_in_unit_disk(&mut self) -> DVec2 {
        loop {
            let x = self.next_f64() * 2.0 - 1.0;
            let y = self.next_f64() * 2.0 - 1.0;
            if x * x + y * y < 1.0 {
                return DVec2::new(x, y);
            }
        }
    }

    /// A point strictly inside the unit ball (`length() < 1`), uniformly
    /// distributed over its volume, by rejection sampling.
    fn vec3_in_unit_ball(&mut self) -> Vec3 {
        loop {
            let x = self.next_f32() * 2.0 - 1.0;
            let y = self.next_f32() * 2.0 - 1.0;
            let z = self.next_f32() * 2.0 - 1.0;
            if x * x + y * y + z * z < 1.0 {
                return Vec3::new(x, y, z);
            }
        }
    }

    /// A point strictly inside the unit ball (`length() < 1`), uniformly
    /// distributed over its volume, by rejection sampling.
    fn dvec3_in_unit_ball(&mut self) -> DVec3 {
        loop {
            let x = self.next_f64() * 2.0 - 1.0;
            let y = self.next_f64() * 2.0 - 1.0;
            let z = self.next_f64() * 2.0 - 1.0;
            if x * x + y * y + z * z < 1.0 {
                return DVec3::new(x, y, z);
            }
        }
    }

    /// A unit vector with a uniformly distributed direction (a point on the
    /// unit circle).
    ///
    /// Uses no `sin`/`cos`: a point `(x, y)` of the disk is squared as a
    /// complex number, `(x^2 - y^2, 2xy) / (x^2 + y^2)`, which doubles its
    /// angle and so keeps it uniform.
    fn unit_vec2(&mut self) -> Vec2 {
        loop {
            let x = self.next_f32() * 2.0 - 1.0;
            let y = self.next_f32() * 2.0 - 1.0;
            let squared = x * x + y * y;
            // Points near the centre have no reliable direction.
            if (1e-6..1.0).contains(&squared) {
                return Vec2::new((x * x - y * y) / squared, 2.0 * x * y / squared);
            }
        }
    }

    /// A unit vector with a uniformly distributed direction (a point on the
    /// unit circle). See [`Rng::unit_vec2`].
    fn unit_dvec2(&mut self) -> DVec2 {
        loop {
            let x = self.next_f64() * 2.0 - 1.0;
            let y = self.next_f64() * 2.0 - 1.0;
            let squared = x * x + y * y;
            if (1e-12..1.0).contains(&squared) {
                return DVec2::new((x * x - y * y) / squared, 2.0 * x * y / squared);
            }
        }
    }

    /// A unit vector with a uniformly distributed direction (a point on the
    /// unit sphere), by Marsaglia's method: a point `(a, b)` of the disk with
    /// `s = a^2 + b^2` gives `(2a sqrt(1 - s), 2b sqrt(1 - s), 1 - 2s)`. Uses
    /// a single square root and no `sin`/`cos`.
    fn unit_vec3(&mut self) -> Vec3 {
        loop {
            let a = self.next_f32() * 2.0 - 1.0;
            let b = self.next_f32() * 2.0 - 1.0;
            let s = a * a + b * b;
            if s < 1.0 {
                let r = 2.0 * Scalar::sqrt(1.0 - s);
                return Vec3::new(a * r, b * r, 1.0 - 2.0 * s);
            }
        }
    }

    /// A unit vector with a uniformly distributed direction (a point on the
    /// unit sphere). See [`Rng::unit_vec3`].
    fn unit_dvec3(&mut self) -> DVec3 {
        loop {
            let a = self.next_f64() * 2.0 - 1.0;
            let b = self.next_f64() * 2.0 - 1.0;
            let s = a * a + b * b;
            if s < 1.0 {
                let r = 2.0 * Scalar::sqrt(1.0 - s);
                return DVec3::new(a * r, b * r, 1.0 - 2.0 * s);
            }
        }
    }

    /// Shuffles `items` in place: every permutation is equally likely
    /// (Fisher-Yates).
    fn shuffle<T>(&mut self, items: &mut [T])
    where
        Self: Sized,
    {
        for i in (1..items.len()).rev() {
            let j = self.below_u64(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }

    /// A uniformly chosen element of `items`, or `None` if it is empty.
    fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T>
    where
        Self: Sized,
    {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below_u64(items.len() as u64) as usize])
        }
    }
}

impl<R: Rng + ?Sized> Rng for &mut R {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        (**self).next_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        (**self).next_u64()
    }
}

#[cfg(test)]
mod tests;
