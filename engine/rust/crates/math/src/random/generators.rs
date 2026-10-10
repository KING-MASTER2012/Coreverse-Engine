//! The two generators of the crate: [`SplitMix64`] and [`Pcg32`], both
//! `const`-constructible, allocation-free and fully deterministic.

use super::{Rng, mix64};

/// The "golden gamma" SplitMix64 adds to its state on every step: `2^64`
/// divided by the golden ratio, rounded to an odd number.
const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// The multiplier of the 64-bit linear congruential generator inside PCG.
const PCG_MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// SplitMix64: a tiny, very fast generator with 64 bits of state that passes
/// the usual statistical test suites. Every seed (including `0`) is valid and
/// gives a different, full-period sequence.
///
/// It is the right tool for *seeding*: turning one `u64` into well-mixed
/// state for other generators (that is how [`Pcg32::new`] works), or deriving
/// independent seeds from a counter. As a general-purpose generator prefer
/// [`Pcg32`], which has better statistical quality per output bit.
///
/// The output for a given seed is part of this crate's contract and will not
/// change:
///
/// ```
/// use math::SplitMix64;
///
/// let mut rng = SplitMix64::new(0);
/// assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
/// assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
/// ```
///
/// It is deliberately not `Copy`: copying a generator by accident replays the
/// same numbers twice. Use [`Clone`] when a second copy of the state is really
/// wanted.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitMix64 {
    /// The whole state: a counter that is advanced by [`SPLITMIX_GAMMA`] and
    /// scrambled on output.
    state: u64,
}

impl SplitMix64 {
    /// A generator seeded with `seed`. Any value is a good seed, small
    /// consecutive ones (`0, 1, 2, ...`) included.
    #[inline]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The current state, which can be passed to [`SplitMix64::new`] to resume
    /// the sequence from this point.
    #[inline]
    pub const fn state(&self) -> u64 {
        self.state
    }

    /// The next 64 random bits. A `const fn`, so a generator can be run at
    /// compile time (to build a lookup table, say).
    #[inline]
    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_GAMMA);
        mix64(self.state)
    }

    /// The next 32 random bits: the upper half of [`SplitMix64::next_u64`],
    /// whose bits are the best mixed.
    #[inline]
    pub const fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

impl Rng for SplitMix64 {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        SplitMix64::next_u32(self)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        SplitMix64::next_u64(self)
    }
}

/// PCG32 (`PCG-XSH-RR 64/32`, by Melissa O'Neill): a 64-bit linear
/// congruential generator whose state is permuted into 32 well-mixed output
/// bits. Small (16 bytes), fast, statistically strong and, unlike most
/// generators, equipped with `2^63` independent *streams*.
///
/// Use [`Pcg32::new`] to seed it from a single number, or
/// [`Pcg32::from_state_and_stream`] for the exact initialisation of the
/// reference implementation (`pcg32_srandom_r`), which makes the output
/// identical to every other PCG32 implementation:
///
/// ```
/// use math::{Pcg32, Rng};
///
/// let mut rng = Pcg32::from_state_and_stream(42, 54);
/// assert_eq!(rng.next_u32(), 0xA15C_02B7);
/// assert_eq!(rng.next_u32(), 0x7B47_F409);
/// assert_eq!(rng.next_u32(), 0xBA1D_3330);
/// ```
///
/// It is deliberately not `Copy`: copying a generator by accident replays the
/// same numbers twice. Use [`Clone`] when a second copy of the state is really
/// wanted.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pcg32 {
    /// The 64-bit LCG state.
    state: u64,
    /// The stream selector (the LCG increment); always odd.
    inc: u64,
}

impl Pcg32 {
    /// A generator seeded with `seed`. Both the starting state and the
    /// stream are derived from the seed (through [`SplitMix64`]), so
    /// different seeds, even consecutive ones, give unrelated sequences.
    ///
    /// ```
    /// use math::{Pcg32, Rng};
    ///
    /// let mut a = Pcg32::new(7);
    /// let mut b = Pcg32::new(7);
    /// assert_eq!(a.next_u32(), b.next_u32()); // same seed, same numbers
    /// assert_ne!(Pcg32::new(7).next_u32(), Pcg32::new(8).next_u32());
    /// ```
    #[inline]
    pub const fn new(seed: u64) -> Self {
        let mut seeder = SplitMix64::new(seed);
        let state = seeder.next_u64();
        let stream = seeder.next_u64();
        Self::from_state_and_stream(state, stream)
    }

    /// A generator with an explicit starting state `init_state` on the
    /// stream `init_stream`, initialised exactly like the reference
    /// implementation's `pcg32_srandom_r(rng, initstate, initseq)`.
    ///
    /// Generators with the same state on different streams produce unrelated
    /// sequences, which is how to give each entity or thread its own
    /// generator from one base seed. Only the low 63 bits of `init_stream`
    /// matter.
    #[inline]
    pub const fn from_state_and_stream(init_state: u64, init_stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (init_stream << 1) | 1,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(init_state);
        rng.step();
        rng
    }

    /// Advances the LCG by one step and returns the state it had before.
    #[inline]
    const fn step(&mut self) -> u64 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG_MULTIPLIER).wrapping_add(self.inc);
        old
    }

    /// The next 32 random bits. A `const fn`, so a generator can be run at
    /// compile time.
    #[inline]
    pub const fn next_u32(&mut self) -> u32 {
        let old = self.step();
        // XSH-RR: xorshift the high bits down, then rotate by the top 5 bits.
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rotation = (old >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    /// The next 64 random bits: two outputs, the first as the high half.
    #[inline]
    pub const fn next_u64(&mut self) -> u64 {
        let high = self.next_u32() as u64;
        let low = self.next_u32() as u64;
        (high << 32) | low
    }
}

impl Rng for Pcg32 {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        Pcg32::next_u32(self)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        Pcg32::next_u64(self)
    }
}
