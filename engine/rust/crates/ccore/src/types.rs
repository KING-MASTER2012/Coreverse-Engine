use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// A size in bytes, with convenience constructors and human-readable
/// formatting (e.g. `ByteSize::from_bytes(1536)` displays as `"1.50 KiB"`).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ByteSize(u64);

impl ByteSize {
    /// Function that returns self from bytes
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }
    /// Function that returns self from kilobytes
    pub const fn from_kib(kib: u64) -> Self {
        Self(kib * 1024)
    }
    /// Function that returns self from megabytes
    pub const fn from_mib(mib: u64) -> Self {
        Self(mib * 1024 * 1024)
    }
    /// Function that returns self from megabytes
    pub const fn from_gib(gib: u64) -> Self {
        Self(gib * 1024 * 1024 * 1024)
    }
    /// Function that returns bytes
    pub const fn as_bytes(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut value = self.0 as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            write!(f, "{} {}", self.0, UNITS[unit])
        } else {
            write!(f, "{value:.2} {}", UNITS[unit])
        }
    }
}

impl From<u64> for ByteSize {
    fn from(bytes: u64) -> Self {
        Self(bytes)
    }
}

/// A frame counter, incremented once per simulated/rendered frame.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct FrameNumber(u64);

impl FrameNumber {
    /// Function that creates new FrameNumber
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Function that returns value
    pub const fn value(&self) -> u64 {
        self.0
    }
    /// The next function
    #[must_use]
    pub const fn next(&self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

impl fmt::Display for FrameNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame {}", self.0)
    }
}

/// Milliseconds since the Unix epoch.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Timestamp(u64);

impl Timestamp {
    /// The now function
    pub fn now() -> Self {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self(millis)
    }
    /// Function that add millis into self
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }
    /// Function that returns millis
    pub const fn as_millis(&self) -> u64 {
        self.0
    }
}

/// An axis-aligned rectangle, generic over its coordinate/size type.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Rect<T> {
    /// x
    pub x: T,
    /// y
    pub y: T,
    /// Width
    pub width: T,
    /// Height
    pub height: T,
}

impl<T> Rect<T> {
    /// Function that creates new Rect
    pub const fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// An 8-bit-per-channel RGBA color.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Color {
    /// Red
    pub r: u8,
    /// Green
    pub g: u8,
    /// Blue
    pub b: u8,
    /// Alpha
    pub a: u8,
}

impl Color {
    /// Color from RGB
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    /// Color from RGBA
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    /// The white
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// The black
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// The transparent
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_size_formats_human_readable() {
        assert_eq!(ByteSize::from_bytes(512).to_string(), "512 B");
        assert_eq!(ByteSize::from_bytes(1536).to_string(), "1.50 KiB");
        assert_eq!(ByteSize::from_mib(2).to_string(), "2.00 MiB");
    }

    #[test]
    fn frame_number_increments() {
        let f = FrameNumber::new(9);
        assert_eq!(f.next().value(), 10);
    }
}
