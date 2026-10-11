//! Normalised integers: `unorm` (`0..=1` stored as an unsigned integer) and
//! `snorm` (`-1..=1` stored as a signed one), as scalars and as the packed
//! vector formats of GLSL, Vulkan and Direct3D.

use crate::{Vec2, Vec4};

/// `x` clamped to `0..=1`, with a `NaN` becoming `0`.
#[inline]
const fn clamp_unorm(x: f32) -> f32 {
    if x > 1.0 {
        1.0
    } else if x >= 0.0 {
        x
    } else {
        // Below zero, or a NaN, which no comparison matches.
        0.0
    }
}

/// `x` clamped to `-1..=1`, with a `NaN` becoming `0`.
#[inline]
const fn clamp_snorm(x: f32) -> f32 {
    if x > 1.0 {
        1.0
    } else if x >= -1.0 {
        x
    } else if x < -1.0 {
        -1.0
    } else {
        // A NaN, which no comparison matches.
        0.0
    }
}

/// The unsigned integer nearest to `clamp(x, 0, 1) * max`, ties rounding up.
#[inline]
const fn to_unorm(x: f32, max: f32) -> u32 {
    // For a non-negative value, truncating after adding a half rounds to
    // nearest with ties up; `as` is exact here (the value is below `2^24`).
    (clamp_unorm(x) * max + 0.5) as u32
}

/// The signed integer nearest to `clamp(x, -1, 1) * max`, ties rounding away
/// from zero.
#[inline]
const fn to_snorm(x: f32, max: f32) -> i32 {
    let c = clamp_snorm(x);
    if c >= 0.0 {
        (c * max + 0.5) as i32
    } else {
        -((-c * max + 0.5) as i32)
    }
}

/// `value / max`.
#[inline]
const fn from_unorm(value: u32, max: f32) -> f32 {
    value as f32 / max
}

/// `value / max`, with the most negative integer (which has no positive
/// counterpart) clamped to `-1`.
#[inline]
const fn from_snorm(value: i32, max: f32) -> f32 {
    let f = value as f32 / max;
    if f < -1.0 { -1.0 } else { f }
}

/// The low `bits` bits of `value`, sign-extended to an `i32`.
#[inline]
const fn sign_extend(value: u32, bits: u32) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

/// Packs `x` as an 8-bit unorm: `0..=1` onto `0..=255`.
///
/// Out-of-range input saturates and a `NaN` gives `0`; the result is the
/// nearest integer, ties rounding up.
///
/// ```
/// use math::pack::{pack_unorm8, unpack_unorm8};
///
/// assert_eq!(pack_unorm8(0.0), 0);
/// assert_eq!(pack_unorm8(0.5), 128);
/// assert_eq!(pack_unorm8(1.0), 255);
/// assert_eq!(pack_unorm8(2.0), 255); // saturates
/// assert_eq!(unpack_unorm8(255), 1.0);
/// ```
#[inline]
pub const fn pack_unorm8(x: f32) -> u8 {
    to_unorm(x, 255.0) as u8
}

/// The `0..=1` float of an 8-bit unorm: `value / 255`.
#[inline]
pub const fn unpack_unorm8(value: u8) -> f32 {
    from_unorm(value as u32, 255.0)
}

/// Packs `x` as an 8-bit snorm: `-1..=1` onto `-127..=127`.
///
/// Out-of-range input saturates and a `NaN` gives `0`; the result is the
/// nearest integer, ties rounding away from zero. `-128` is never produced.
///
/// ```
/// use math::pack::{pack_snorm8, unpack_snorm8};
///
/// assert_eq!(pack_snorm8(1.0), 127);
/// assert_eq!(pack_snorm8(-1.0), -127);
/// assert_eq!(pack_snorm8(0.5), 64);
/// assert_eq!(pack_snorm8(-0.5), -64);
/// assert_eq!(unpack_snorm8(-128), -1.0); // the extra value clamps
/// ```
#[inline]
pub const fn pack_snorm8(x: f32) -> i8 {
    to_snorm(x, 127.0) as i8
}

/// The `-1..=1` float of an 8-bit snorm: `value / 127`, with `-128` giving
/// `-1`.
#[inline]
pub const fn unpack_snorm8(value: i8) -> f32 {
    from_snorm(value as i32, 127.0)
}

/// Packs `x` as a 16-bit unorm: `0..=1` onto `0..=65535`. See
/// [`pack_unorm8`] for the rounding.
#[inline]
pub const fn pack_unorm16(x: f32) -> u16 {
    to_unorm(x, 65535.0) as u16
}

/// The `0..=1` float of a 16-bit unorm: `value / 65535`.
#[inline]
pub const fn unpack_unorm16(value: u16) -> f32 {
    from_unorm(value as u32, 65535.0)
}

/// Packs `x` as a 16-bit snorm: `-1..=1` onto `-32767..=32767`. See
/// [`pack_snorm8`] for the rounding.
#[inline]
pub const fn pack_snorm16(x: f32) -> i16 {
    to_snorm(x, 32767.0) as i16
}

/// The `-1..=1` float of a 16-bit snorm: `value / 32767`, with `-32768`
/// giving `-1`.
#[inline]
pub const fn unpack_snorm16(value: i16) -> f32 {
    from_snorm(value as i32, 32767.0)
}

/// Packs `x` as a 10-bit unorm: `0..=1` onto `0..=1023`, in the low 10 bits of
/// the result. See [`pack_unorm8`] for the rounding.
#[inline]
pub const fn pack_unorm10(x: f32) -> u16 {
    to_unorm(x, 1023.0) as u16
}

/// The `0..=1` float of a 10-bit unorm held in the low 10 bits of `value`
/// (higher bits are ignored): `bits / 1023`.
#[inline]
pub const fn unpack_unorm10(value: u16) -> f32 {
    from_unorm((value & 0x3FF) as u32, 1023.0)
}

/// Packs `x` as a 10-bit snorm: `-1..=1` onto `-511..=511`, as a two's
/// complement number in the low 10 bits of the result. See [`pack_snorm8`] for
/// the rounding.
#[inline]
pub const fn pack_snorm10(x: f32) -> u16 {
    (to_snorm(x, 511.0) & 0x3FF) as u16
}

/// The `-1..=1` float of a 10-bit snorm held (two's complement) in the low 10
/// bits of `value` (higher bits are ignored): `value / 511`, with `-512`
/// giving `-1`.
#[inline]
pub const fn unpack_snorm10(value: u16) -> f32 {
    from_snorm(sign_extend(value as u32, 10), 511.0)
}

/// Packs `x` as a 2-bit unorm: `0..=1` onto `0..=3`, in the low 2 bits of the
/// result. See [`pack_unorm8`] for the rounding.
#[inline]
pub const fn pack_unorm2(x: f32) -> u8 {
    to_unorm(x, 3.0) as u8
}

/// The `0..=1` float of a 2-bit unorm held in the low 2 bits of `value`
/// (higher bits are ignored): `bits / 3`.
#[inline]
pub const fn unpack_unorm2(value: u8) -> f32 {
    from_unorm((value & 3) as u32, 3.0)
}

/// Packs `x` as a 2-bit snorm: `-1..=1` onto `-1..=1`, as a two's complement
/// number in the low 2 bits of the result (`0b11` is `-1`, `0b01` is `+1`).
/// With so few values this is the sign, rounded: anything from `0.5` up packs
/// to `+1`, from `-0.5` down to `-1`.
#[inline]
pub const fn pack_snorm2(x: f32) -> u8 {
    (to_snorm(x, 1.0) & 3) as u8
}

/// The `-1..=1` float of a 2-bit snorm held (two's complement) in the low 2
/// bits of `value` (higher bits are ignored); the value `-2` (`0b10`) is
/// clamped to `-1`.
#[inline]
pub const fn unpack_snorm2(value: u8) -> f32 {
    from_snorm(sign_extend(value as u32, 2), 1.0)
}

/// Packs a [`Vec4`] as four 8-bit unorms in a `u32`, `x` in the lowest byte
/// and `w` in the highest: the layout of GLSL's `packUnorm4x8` and of the
/// `R8G8B8A8_UNORM` format on a little-endian machine. The usual way to store
/// a colour in 4 bytes.
///
/// ```
/// use math::Vec4;
/// use math::pack::{pack_unorm4x8, unpack_unorm4x8};
///
/// let packed = pack_unorm4x8(Vec4::new(1.0, 0.0, 0.0, 1.0)); // opaque red
/// assert_eq!(packed, 0xFF00_00FF);
/// assert_eq!(unpack_unorm4x8(packed), Vec4::new(1.0, 0.0, 0.0, 1.0));
/// ```
#[inline]
pub const fn pack_unorm4x8(value: Vec4) -> u32 {
    pack_unorm8(value.x) as u32
        | (pack_unorm8(value.y) as u32) << 8
        | (pack_unorm8(value.z) as u32) << 16
        | (pack_unorm8(value.w) as u32) << 24
}

/// Unpacks four 8-bit unorms (`x` from the lowest byte); the inverse of
/// [`pack_unorm4x8`].
#[inline]
pub const fn unpack_unorm4x8(packed: u32) -> Vec4 {
    Vec4::new(
        unpack_unorm8(packed as u8),
        unpack_unorm8((packed >> 8) as u8),
        unpack_unorm8((packed >> 16) as u8),
        unpack_unorm8((packed >> 24) as u8),
    )
}

/// Packs a [`Vec4`] as four 8-bit snorms in a `u32`, `x` in the lowest byte
/// and `w` in the highest (two's complement bytes): the layout of GLSL's
/// `packSnorm4x8` and of `R8G8B8A8_SNORM`. A compact format for a tangent or
/// a normal plus a sign.
#[inline]
pub const fn pack_snorm4x8(value: Vec4) -> u32 {
    (pack_snorm8(value.x) as u8) as u32
        | ((pack_snorm8(value.y) as u8) as u32) << 8
        | ((pack_snorm8(value.z) as u8) as u32) << 16
        | ((pack_snorm8(value.w) as u8) as u32) << 24
}

/// Unpacks four 8-bit snorms (`x` from the lowest byte); the inverse of
/// [`pack_snorm4x8`].
#[inline]
pub const fn unpack_snorm4x8(packed: u32) -> Vec4 {
    Vec4::new(
        unpack_snorm8(packed as u8 as i8),
        unpack_snorm8((packed >> 8) as u8 as i8),
        unpack_snorm8((packed >> 16) as u8 as i8),
        unpack_snorm8((packed >> 24) as u8 as i8),
    )
}

/// Packs a [`Vec2`] as two 16-bit unorms in a `u32`, `x` in the low half and
/// `y` in the high half: the layout of GLSL's `packUnorm2x16` and of
/// `R16G16_UNORM`.
#[inline]
pub const fn pack_unorm2x16(value: Vec2) -> u32 {
    pack_unorm16(value.x) as u32 | (pack_unorm16(value.y) as u32) << 16
}

/// Unpacks two 16-bit unorms (`x` from the low half); the inverse of
/// [`pack_unorm2x16`].
#[inline]
pub const fn unpack_unorm2x16(packed: u32) -> Vec2 {
    Vec2::new(
        unpack_unorm16(packed as u16),
        unpack_unorm16((packed >> 16) as u16),
    )
}

/// Packs a [`Vec2`] as two 16-bit snorms in a `u32`, `x` in the low half and
/// `y` in the high half (two's complement): the layout of GLSL's
/// `packSnorm2x16` and of `R16G16_SNORM`.
#[inline]
pub const fn pack_snorm2x16(value: Vec2) -> u32 {
    (pack_snorm16(value.x) as u16) as u32 | ((pack_snorm16(value.y) as u16) as u32) << 16
}

/// Unpacks two 16-bit snorms (`x` from the low half); the inverse of
/// [`pack_snorm2x16`].
#[inline]
pub const fn unpack_snorm2x16(packed: u32) -> Vec2 {
    Vec2::new(
        unpack_snorm16(packed as u16 as i16),
        unpack_snorm16((packed >> 16) as u16 as i16),
    )
}

/// Packs a [`Vec4`] as three 10-bit unorms and a 2-bit unorm in a `u32`: `x`
/// in bits `0..10`, `y` in `10..20`, `z` in `20..30` and `w` in `30..32`.
/// This is the `A2B10G10R10_UNORM_PACK32` format of Vulkan (and
/// `R10G10B10A2_UNORM` of Direct3D): a colour with more precision per channel
/// than 8 bits and a coarse alpha.
///
/// ```
/// use math::Vec4;
/// use math::pack::{pack_unorm_1010102, unpack_unorm_1010102};
///
/// let packed = pack_unorm_1010102(Vec4::new(1.0, 0.0, 1.0, 1.0));
/// assert_eq!(packed, 0b11_1111111111_0000000000_1111111111);
/// assert_eq!(unpack_unorm_1010102(packed), Vec4::new(1.0, 0.0, 1.0, 1.0));
/// ```
#[inline]
pub const fn pack_unorm_1010102(value: Vec4) -> u32 {
    pack_unorm10(value.x) as u32
        | (pack_unorm10(value.y) as u32) << 10
        | (pack_unorm10(value.z) as u32) << 20
        | (pack_unorm2(value.w) as u32) << 30
}

/// Unpacks three 10-bit unorms and a 2-bit unorm; the inverse of
/// [`pack_unorm_1010102`].
#[inline]
pub const fn unpack_unorm_1010102(packed: u32) -> Vec4 {
    Vec4::new(
        unpack_unorm10(packed as u16),
        unpack_unorm10((packed >> 10) as u16),
        unpack_unorm10((packed >> 20) as u16),
        unpack_unorm2((packed >> 30) as u8),
    )
}

/// Packs a [`Vec4`] as three 10-bit snorms and a 2-bit snorm in a `u32`,
/// two's complement fields: `x` in bits `0..10`, `y` in `10..20`, `z` in
/// `20..30` and `w` in `30..32`. This is the `A2B10G10R10_SNORM_PACK32`
/// format of Vulkan (`R10G10B10A2_SNORM`): a unit vector with 10 bits per
/// axis and room for a sign (`w` is `-1`, `0` or `+1`), the standard way to
/// store a tangent frame in 4 bytes.
///
/// ```
/// use math::Vec4;
/// use math::pack::{pack_snorm_1010102, unpack_snorm_1010102};
///
/// let tangent = Vec4::new(0.0, 0.6, 0.8, -1.0);
/// let back = unpack_snorm_1010102(pack_snorm_1010102(tangent));
/// assert!((back - tangent).length() < 2e-3);
/// assert_eq!(back.w, -1.0);
/// ```
#[inline]
pub const fn pack_snorm_1010102(value: Vec4) -> u32 {
    pack_snorm10(value.x) as u32
        | (pack_snorm10(value.y) as u32) << 10
        | (pack_snorm10(value.z) as u32) << 20
        | (pack_snorm2(value.w) as u32) << 30
}

/// Unpacks three 10-bit snorms and a 2-bit snorm; the inverse of
/// [`pack_snorm_1010102`].
#[inline]
pub const fn unpack_snorm_1010102(packed: u32) -> Vec4 {
    Vec4::new(
        unpack_snorm10(packed as u16),
        unpack_snorm10((packed >> 10) as u16),
        unpack_snorm10((packed >> 20) as u16),
        unpack_snorm2((packed >> 30) as u8),
    )
}
