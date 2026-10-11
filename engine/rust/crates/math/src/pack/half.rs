//! IEEE 754 binary16 ("half") floats: bit-exact conversions to and from `f32`
//! and `f64`, the [`F16`] value type, and the half-float vertex formats.

use core::cmp::Ordering;
use core::fmt;

use crate::{Vec2, Vec4};

/// The `binary16` bits of the float whose parts are given, rounded to nearest,
/// ties to even.
///
/// The source float is described by the position of its sign bit already
/// moved into `sign` (`0x8000` or `0`), its biased exponent, its stored
/// mantissa (the bits after the implicit leading one), the width of that
/// mantissa, the exponent bias and the all-ones exponent that marks infinity
/// and `NaN`. One routine serves `f32` and `f64`, so neither goes through the
/// other and rounds twice.
#[inline]
const fn float_to_half(
    sign: u16,
    biased_exponent: u32,
    mantissa: u64,
    mantissa_bits: u32,
    bias: i32,
    special_exponent: u32,
) -> u16 {
    if biased_exponent == special_exponent {
        if mantissa == 0 {
            return sign | 0x7C00;
        }
        // A NaN keeps the top bits of its payload. If those are all zero the
        // result would read as infinity, so set the "quiet" bit instead.
        let payload = (mantissa >> (mantissa_bits - 10)) as u16;
        return sign | 0x7C00 | if payload == 0 { 0x0200 } else { payload };
    }
    // The exponent field a half would have; `1..=30` is a normal half.
    let exponent = biased_exponent as i32 - bias + 15;
    if exponent >= 0x1F {
        return sign | 0x7C00;
    }
    if exponent > 0 {
        let shift = mantissa_bits - 10;
        let kept = (mantissa >> shift) as u16;
        let rest = mantissa & ((1_u64 << shift) - 1);
        let halfway = 1_u64 << (shift - 1);
        let mut bits = ((exponent as u16) << 10) | kept;
        // A carry out of the mantissa correctly bumps the exponent, up to
        // infinity at the top.
        if rest > halfway || (rest == halfway && kept & 1 == 1) {
            bits += 1;
        }
        return sign | bits;
    }
    if biased_exponent == 0 {
        // Zero, or a subnormal of the source type, far below the smallest
        // half subnormal.
        return sign;
    }
    // A subnormal half (or something that rounds to zero or to the smallest
    // normal): the mantissa with its implicit one, shifted down to units of
    // the smallest subnormal, `2^-24`.
    let shift = mantissa_bits as i32 - 9 - exponent;
    if shift > mantissa_bits as i32 + 1 {
        return sign;
    }
    let shift = shift as u32;
    let full = (1_u64 << mantissa_bits) | mantissa;
    let kept = (full >> shift) as u16;
    let rest = full & ((1_u64 << shift) - 1);
    let halfway = 1_u64 << (shift - 1);
    let rounded = if rest > halfway || (rest == halfway && kept & 1 == 1) {
        kept + 1
    } else {
        kept
    };
    sign | rounded
}

/// The IEEE 754 `binary16` bits of `value`, rounded to nearest with ties to
/// even.
///
/// Values beyond the largest half (`65504`, up to a rounding tie at `65520`)
/// become infinity, values below half of the smallest subnormal (`2^-24`)
/// become zero, the sign of zero is kept, and a `NaN` stays a `NaN` (with the
/// top of its payload). A `const fn`.
///
/// ```
/// use math::pack::f32_to_f16_bits;
///
/// assert_eq!(f32_to_f16_bits(1.0), 0x3C00);
/// assert_eq!(f32_to_f16_bits(-2.0), 0xC000);
/// assert_eq!(f32_to_f16_bits(65504.0), 0x7BFF); // the largest half
/// assert_eq!(f32_to_f16_bits(1e9), 0x7C00); // too large: infinity
/// const HALF_PI: u16 = f32_to_f16_bits(1.5707964);
/// assert_eq!(HALF_PI, 0x3E48);
/// ```
#[inline]
pub const fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    float_to_half(
        ((bits >> 16) & 0x8000) as u16,
        (bits >> 23) & 0xFF,
        (bits & 0x007F_FFFF) as u64,
        23,
        127,
        0xFF,
    )
}

/// The IEEE 754 `binary16` bits of `value`, rounded to nearest with ties to
/// even. Rounds once, directly from the `f64`, so it can differ from
/// `f32_to_f16_bits(value as f32)` in the rare cases where rounding to `f32`
/// first would land exactly on a halfway point. See [`f32_to_f16_bits`] for
/// the treatment of large, small and special values.
///
/// ```
/// use math::pack::f64_to_f16_bits;
///
/// assert_eq!(f64_to_f16_bits(0.5), 0x3800);
/// // A hair above the halfway point between 1.0 and the next half rounds up,
/// // even though `as f32` would erase the hair and round down to even.
/// let halfway = 1.000_488_281_25_f64;
/// assert_eq!(f64_to_f16_bits(halfway), 0x3C00);
/// assert_eq!(f64_to_f16_bits(halfway + 1e-12), 0x3C01);
/// ```
#[inline]
pub const fn f64_to_f16_bits(value: f64) -> u16 {
    let bits = value.to_bits();
    float_to_half(
        ((bits >> 48) & 0x8000) as u16,
        ((bits >> 52) & 0x7FF) as u32,
        bits & 0x000F_FFFF_FFFF_FFFF,
        52,
        1023,
        0x7FF,
    )
}

/// The `f32` equal to the `binary16` with the given bits. Every half is
/// exactly representable as an `f32`, so this never rounds; a `NaN` stays a
/// `NaN`. A `const fn`.
///
/// ```
/// use math::pack::f16_bits_to_f32;
///
/// assert_eq!(f16_bits_to_f32(0x3C00), 1.0);
/// assert_eq!(f16_bits_to_f32(0x0001), 2.0_f32.powi(-24)); // smallest subnormal
/// assert_eq!(f16_bits_to_f32(0xFC00), f32::NEG_INFINITY);
/// assert!(f16_bits_to_f32(0x7E00).is_nan());
/// ```
#[inline]
pub const fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1F) as u32;
    let mantissa = (bits & 0x03FF) as u32;
    if exponent == 0 {
        // Zero or a subnormal: `mantissa * 2^-24`, exact in an `f32`.
        let magnitude = mantissa as f32 * (1.0 / 16_777_216.0);
        return f32::from_bits(sign | magnitude.to_bits());
    }
    if exponent == 0x1F {
        // Infinity or NaN, with the payload moved up.
        return f32::from_bits(sign | 0x7F80_0000 | (mantissa << 13));
    }
    f32::from_bits(sign | ((exponent + 112) << 23) | (mantissa << 13))
}

/// The `f64` equal to the `binary16` with the given bits. Every half is
/// exactly representable as an `f64`, so this never rounds; a `NaN` stays a
/// `NaN`. A `const fn`.
#[inline]
pub const fn f16_bits_to_f64(bits: u16) -> f64 {
    let sign = ((bits & 0x8000) as u64) << 48;
    let exponent = ((bits >> 10) & 0x1F) as u64;
    let mantissa = (bits & 0x03FF) as u64;
    if exponent == 0 {
        let magnitude = mantissa as f64 * (1.0 / 16_777_216.0);
        return f64::from_bits(sign | magnitude.to_bits());
    }
    if exponent == 0x1F {
        return f64::from_bits(sign | 0x7FF0_0000_0000_0000 | (mantissa << 42));
    }
    f64::from_bits(sign | ((exponent + 1008) << 52) | (mantissa << 42))
}

/// Converts `src` to half-float bits in `dst`, for as many elements as both
/// slices hold, and returns that count. The bulk form of [`f32_to_f16_bits`]
/// for filling a vertex or texture buffer.
///
/// ```
/// use math::pack::f32_slice_to_f16;
///
/// let mut half = [0_u16; 3];
/// assert_eq!(f32_slice_to_f16(&[1.0, 0.5, -2.0], &mut half), 3);
/// assert_eq!(half, [0x3C00, 0x3800, 0xC000]);
/// ```
pub fn f32_slice_to_f16(src: &[f32], dst: &mut [u16]) -> usize {
    let count = src.len().min(dst.len());
    for (out, &value) in dst.iter_mut().zip(src) {
        *out = f32_to_f16_bits(value);
    }
    count
}

/// Converts the half-float bits in `src` to `f32` in `dst`, for as many
/// elements as both slices hold, and returns that count. The bulk form of
/// [`f16_bits_to_f32`].
pub fn f16_slice_to_f32(src: &[u16], dst: &mut [f32]) -> usize {
    let count = src.len().min(dst.len());
    for (out, &bits) in dst.iter_mut().zip(src) {
        *out = f16_bits_to_f32(bits);
    }
    count
}

/// Packs two floats as half floats into one `u32`, `x` in the low 16 bits and
/// `y` in the high 16 bits: the layout of GLSL's `packHalf2x16` and of the
/// `R16G16_FLOAT` format.
///
/// ```
/// use math::Vec2;
/// use math::pack::{pack_half2, unpack_half2};
///
/// let packed = pack_half2(Vec2::new(1.0, -2.0));
/// assert_eq!(packed, 0xC000_3C00);
/// assert_eq!(unpack_half2(packed), Vec2::new(1.0, -2.0));
/// ```
#[inline]
pub const fn pack_half2(value: Vec2) -> u32 {
    f32_to_f16_bits(value.x) as u32 | (f32_to_f16_bits(value.y) as u32) << 16
}

/// Unpacks two half floats from a `u32` (`x` from the low 16 bits, `y` from
/// the high 16 bits); the inverse of [`pack_half2`].
#[inline]
pub const fn unpack_half2(packed: u32) -> Vec2 {
    Vec2::new(
        f16_bits_to_f32(packed as u16),
        f16_bits_to_f32((packed >> 16) as u16),
    )
}

/// Packs four floats as half floats into an array of four `u16`, in the order
/// `x, y, z, w` (the layout of the `R16G16B16A16_FLOAT` format).
#[inline]
pub const fn pack_half4(value: Vec4) -> [u16; 4] {
    [
        f32_to_f16_bits(value.x),
        f32_to_f16_bits(value.y),
        f32_to_f16_bits(value.z),
        f32_to_f16_bits(value.w),
    ]
}

/// Unpacks four half floats (`x, y, z, w`); the inverse of [`pack_half4`].
#[inline]
pub const fn unpack_half4(packed: [u16; 4]) -> Vec4 {
    Vec4::new(
        f16_bits_to_f32(packed[0]),
        f16_bits_to_f32(packed[1]),
        f16_bits_to_f32(packed[2]),
        f16_bits_to_f32(packed[3]),
    )
}

/// A 16-bit IEEE 754 `binary16` float: 1 sign bit, 5 exponent bits and 10
/// mantissa bits. Half the size of an `f32`, with a range up to `65504`, about
/// three decimal digits of precision and subnormals down to `2^-24`; the
/// storage type of choice for vertex attributes, HDR textures and compact
/// animation data.
///
/// `F16` is a storage type: it converts to and from `f32`/`f64` but has no
/// arithmetic of its own. Do the math in `f32` and convert back.
///
/// ```
/// use math::pack::F16;
///
/// let h = F16::from_f32(0.1);
/// assert_eq!(h.to_bits(), 0x2E66);
/// assert!((h.to_f32() - 0.1).abs() < 5e-5); // about 3 digits
/// assert_eq!(F16::MAX.to_f32(), 65504.0);
/// assert_eq!(core::mem::size_of::<F16>(), 2);
/// ```
///
/// Comparison follows float rules (`NaN` is unequal to everything, `+0 == -0`)
/// and the type is neither `Eq` nor `Hash`; compare [`F16::to_bits`] for
/// identity. With the `serde` feature it is written as its `u16` bits.
#[derive(Clone, Copy)]
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct F16(u16);

impl F16 {
    /// Positive zero.
    pub const ZERO: Self = Self(0x0000);
    /// Negative zero.
    pub const NEG_ZERO: Self = Self(0x8000);
    /// One.
    pub const ONE: Self = Self(0x3C00);
    /// Negative one.
    pub const NEG_ONE: Self = Self(0xBC00);
    /// The largest finite value, `65504`.
    pub const MAX: Self = Self(0x7BFF);
    /// The smallest (most negative) finite value, `-65504`.
    pub const MIN: Self = Self(0xFBFF);
    /// The smallest positive normal value, `2^-14` (about `6.1e-5`).
    pub const MIN_POSITIVE: Self = Self(0x0400);
    /// The smallest positive subnormal value, `2^-24` (about `6.0e-8`).
    pub const MIN_POSITIVE_SUBNORMAL: Self = Self(0x0001);
    /// The difference between `1.0` and the next larger value, `2^-10`.
    pub const EPSILON: Self = Self(0x1400);
    /// Positive infinity.
    pub const INFINITY: Self = Self(0x7C00);
    /// Negative infinity.
    pub const NEG_INFINITY: Self = Self(0xFC00);
    /// A (quiet) not-a-number.
    pub const NAN: Self = Self(0x7E00);

    /// The half with the given IEEE 754 `binary16` bits.
    #[inline]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// The IEEE 754 `binary16` bits.
    #[inline]
    pub const fn to_bits(self) -> u16 {
        self.0
    }

    /// The half nearest to `value` (ties to even); see [`f32_to_f16_bits`].
    #[inline]
    pub const fn from_f32(value: f32) -> Self {
        Self(f32_to_f16_bits(value))
    }

    /// The half nearest to `value` (ties to even); see [`f64_to_f16_bits`].
    #[inline]
    pub const fn from_f64(value: f64) -> Self {
        Self(f64_to_f16_bits(value))
    }

    /// This half as an `f32` (exact).
    #[inline]
    pub const fn to_f32(self) -> f32 {
        f16_bits_to_f32(self.0)
    }

    /// This half as an `f64` (exact).
    #[inline]
    pub const fn to_f64(self) -> f64 {
        f16_bits_to_f64(self.0)
    }

    /// Whether this is a `NaN`.
    #[inline]
    pub const fn is_nan(self) -> bool {
        self.0 & 0x7FFF > 0x7C00
    }

    /// Whether this is positive or negative infinity.
    #[inline]
    pub const fn is_infinite(self) -> bool {
        self.0 & 0x7FFF == 0x7C00
    }

    /// Whether this is neither infinite nor a `NaN`.
    #[inline]
    pub const fn is_finite(self) -> bool {
        self.0 & 0x7C00 != 0x7C00
    }

    /// Whether this is a subnormal (non-zero, below [`F16::MIN_POSITIVE`] in
    /// magnitude).
    #[inline]
    pub const fn is_subnormal(self) -> bool {
        self.0 & 0x7C00 == 0 && self.0 & 0x03FF != 0
    }

    /// Whether the sign bit is set (negative numbers, `-0` and negative
    /// `NaN`s).
    #[inline]
    pub const fn is_sign_negative(self) -> bool {
        self.0 & 0x8000 != 0
    }

    /// The absolute value (the sign bit cleared).
    #[inline]
    pub const fn abs(self) -> Self {
        Self(self.0 & 0x7FFF)
    }
}

impl Default for F16 {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for F16 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.to_f32() == other.to_f32()
    }
}

impl PartialOrd for F16 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_f32().partial_cmp(&other.to_f32())
    }
}

impl From<F16> for f32 {
    #[inline]
    fn from(value: F16) -> f32 {
        value.to_f32()
    }
}

impl From<F16> for f64 {
    #[inline]
    fn from(value: F16) -> f64 {
        value.to_f64()
    }
}

impl fmt::Debug for F16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F16({:?})", self.to_f32())
    }
}

impl fmt::Display for F16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.to_f32(), f)
    }
}
