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
