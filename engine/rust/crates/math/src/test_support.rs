//! Helpers shared by the unit tests. Compiled for tests only.

use core::fmt::{Display, Write};

/// Text formatted into a fixed buffer, so `Display` output can be checked
/// without `alloc` (the `no_std` test build has neither `format!` nor
/// `to_string`).
pub(crate) struct Text {
    buffer: [u8; 128],
    length: usize,
}

impl Text {
    /// The formatted text.
    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.length]).unwrap()
    }
}

impl Write for Text {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.length + s.len();
        self.buffer
            .get_mut(self.length..end)
            .ok_or(core::fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.length = end;
        Ok(())
    }
}

/// `value` formatted with `{}`.
pub(crate) fn text(value: &impl Display) -> Text {
    let mut out = Text {
        buffer: [0; 128],
        length: 0,
    };
    write!(out, "{value}").unwrap();
    out
}

/// `value` formatted with `{:.precision$}`.
pub(crate) fn text_with_precision(value: &impl Display, precision: usize) -> Text {
    let mut out = Text {
        buffer: [0; 128],
        length: 0,
    };
    write!(out, "{value:.precision$}").unwrap();
    out
}

/// A small deterministic pseudo random number generator (an LCG), so the
/// randomized tests are reproducible.
pub(crate) struct Rng(u64);

impl Rng {
    /// A generator that always produces the same sequence for `seed`.
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next number, uniformly in `[0, 1)`.
    pub(crate) fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f64) / ((1u64 << 24) as f64)
    }

    /// The next number, uniformly in `[low, high)`.
    pub(crate) fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }
}
