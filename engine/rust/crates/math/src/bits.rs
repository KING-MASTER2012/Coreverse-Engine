//! Bit tricks: [`popcount`], [`leading_zeros`], [`trailing_zeros`],
//! [`reverse_bits`], rotation ([`rotate_left`], [`rotate_right`]),
//! power-of-two tests ([`is_power_of_two`], [`next_power_of_two`],
//! [`prev_power_of_two`]), alignment ([`align_up`], [`align_down`],
//! [`is_aligned`]) and Morton codes ([`morton_encode_2d`],
//! [`morton_encode_3d`] and their decoders).
//!
//! Everything but the Morton codes is generic over [`Integer`], so one
//! definition serves all 12 standard integer types. Signed types are
//! supported where it makes sense: a negative number is never a power of
//! two, and alignment rounds towards positive infinity (`align_up`) or
//! negative infinity (`align_down`) using two's-complement masking, so
//! `align_down(-5, 4) == Some(-8)`. Rotation and bit reversal treat a signed
//! number as its raw two's-complement bits.
//!
//! Nothing here panics. Where a result can be out of range or an argument
//! invalid (an alignment that is not a power of two, a next power of two
//! that overflows), the function returns `None`.
//!
//! ```
//! use math::bits::{align_up, next_power_of_two, rotate_left};
//!
//! assert_eq!(next_power_of_two(100_u32), Some(128));
//! assert_eq!(align_up(13_usize, 8), Some(16));
//! assert_eq!(align_up(13_usize, 6), None); // 6 is not a power of two
//! assert_eq!(rotate_left(0b1000_0001_u8, 1), 0b0000_0011);
//! ```
//!
//! # Morton codes
//!
//! A Morton code (a *Z-order curve* index) interleaves the bits of two or
//! three coordinates into one integer, so that cells close together in 2D/3D
//! space get codes close together in memory. That makes it the standard key
//! for grid-based spatial hashes, linear octrees/quadtrees and BVH builders:
//! sorting by Morton code orders the cells along the curve, and a cell's
//! parent is its code shifted right by 2 (2D) or 3 (3D) bits.
//!
//! ```
//! use math::bits::{morton_decode_3d, morton_encode_2d, morton_encode_3d};
//!
//! // x occupies the even bits, y the odd ones.
//! assert_eq!(morton_encode_2d(0b11, 0b00), 0b0101);
//! assert_eq!(morton_encode_2d(0b00, 0b11), 0b1010);
//! // x, y and z take every third bit, starting at bit 0, 1 and 2.
//! assert_eq!(morton_encode_3d(1, 0, 0), 0b001);
//! assert_eq!(morton_encode_3d(0, 1, 0), 0b010);
//! assert_eq!(morton_encode_3d(0, 0, 1), 0b100);
//! assert_eq!(morton_decode_3d(morton_encode_3d(5, 6, 7)), (5, 6, 7));
//! ```
//!
//! # `const`
//!
//! The Morton functions are concrete and therefore `const fn`. The generic
//! functions are generic over a trait, and trait methods cannot be `const`
//! on stable Rust, so those cannot be `const fn`. For `const` contexts every
//! generic function also exists as a `const fn` on each concrete type, in a
//! module named after the type (`bits::u32::next_power_of_two`,
//! `bits::i64::rotate_left`, ...), with the same signature minus the
//! generics:
//!
//! ```
//! use math::bits;
//!
//! const SIZE: Option<u32> = bits::u32::next_power_of_two(1000);
//! const ALIGNED: Option<usize> = bits::usize::align_up(13, 8);
//!
//! assert_eq!((SIZE, ALIGNED), (Some(1024), Some(16)));
//! ```

use crate::integer::Integer;

/// The number of `1` bits in `n` (its population count).
#[inline]
pub fn popcount<T: Integer>(n: T) -> u32 {
    n.count_ones()
}

/// The number of leading `0` bits in `n`. `T::BITS` for zero, and `0` for
/// any negative signed number (its top bit is set).
#[inline]
pub fn leading_zeros<T: Integer>(n: T) -> u32 {
    n.leading_zeros()
}

/// The number of trailing `0` bits in `n`. `T::BITS` for zero.
#[inline]
pub fn trailing_zeros<T: Integer>(n: T) -> u32 {
    n.trailing_zeros()
}

/// `n` with the order of its bits reversed: bit `0` becomes bit
/// `T::BITS - 1`, bit `1` becomes bit `T::BITS - 2`, and so on.
///
/// A signed number is treated as its raw two's-complement bits, so
/// `reverse_bits(1_i8) == i8::MIN`.
#[inline]
pub fn reverse_bits<T: Integer>(n: T) -> T {
    n.reverse_bits()
}

/// `n` with its bits rotated left by `count` places: the bits shifted out on
/// the left re-enter on the right. `count` is taken modulo `T::BITS`, so
/// rotating by `T::BITS` changes nothing.
///
/// A signed number is treated as its raw two's-complement bits.
#[inline]
pub fn rotate_left<T: Integer>(n: T, count: u32) -> T {
    n.rotate_left(count)
}

/// `n` with its bits rotated right by `count` places: the bits shifted out
/// on the right re-enter on the left. `count` is taken modulo `T::BITS`.
#[inline]
pub fn rotate_right<T: Integer>(n: T, count: u32) -> T {
    n.rotate_right(count)
}

/// Whether `n` is a power of two (`1, 2, 4, 8, ...`).
///
/// Zero is not a power of two, and neither is any negative number.
#[inline]
pub fn is_power_of_two<T: Integer>(n: T) -> bool {
    n > T::ZERO && (n & (n - T::ONE)) == T::ZERO
}

/// The smallest power of two that is greater than or equal to `n`, or `None`
/// if that power does not fit in `T`.
///
/// Anything at or below `1` (zero, and every negative number) gives
/// `Some(1)`, the smallest power of two. The largest power of two a signed
/// type holds is `2^(BITS - 2)`, so e.g. `next_power_of_two(65_i8)` is
/// `None` (`128` does not fit in an `i8`).
#[inline]
pub fn next_power_of_two<T: Integer>(n: T) -> Option<T> {
    if n <= T::ONE {
        return Some(T::ONE);
    }
    // `n - 1` has `BITS - leading_zeros` significant bits; the answer is
    // one bit above them. The sign bit of a signed type is not available.
    let bits_needed = T::BITS - (n - T::ONE).leading_zeros();
    let usable_bits = if T::SIGNED { T::BITS - 1 } else { T::BITS };
    if bits_needed >= usable_bits {
        None
    } else {
        Some(T::ONE << bits_needed)
    }
}

/// The largest power of two that is less than or equal to `n`, or `None` if
/// there is none (`n` is zero or negative).
#[inline]
pub fn prev_power_of_two<T: Integer>(n: T) -> Option<T> {
    if n <= T::ZERO {
        None
    } else {
        Some(T::ONE << (T::BITS - 1 - n.leading_zeros()))
    }
}

/// `value` rounded **up** to the next multiple of `align`, or `None` if
/// `align` is not a power of two or the result overflows `T`.
///
/// A value that is already aligned is returned unchanged.
#[inline]
pub fn align_up<T: Integer>(value: T, align: T) -> Option<T> {
    if !is_power_of_two(align) {
        return None;
    }
    let mask = align - T::ONE;
    value.checked_add(mask).map(|sum| sum & !mask)
}

/// `value` rounded **down** to a multiple of `align`, or `None` if `align`
/// is not a power of two.
///
/// A value that is already aligned is returned unchanged.
#[inline]
pub fn align_down<T: Integer>(value: T, align: T) -> Option<T> {
    if !is_power_of_two(align) {
        return None;
    }
    Some(value & !(align - T::ONE))
}

/// Whether `value` is a multiple of `align`. `false` if `align` is not a
/// power of two.
#[inline]
pub fn is_aligned<T: Integer>(value: T, align: T) -> bool {
    is_power_of_two(align) && (value & (align - T::ONE)) == T::ZERO
}

/// Spreads the 32 bits of `x` out to the even bits of a `u64` (bit `i` moves
/// to bit `2 * i`).
const fn spread_2d(x: u32) -> u64 {
    let mut v = x as u64;
    v = (v | (v << 16)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v << 8)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333_3333_3333;
    v = (v | (v << 1)) & 0x5555_5555_5555_5555;
    v
}

/// The inverse of [`spread_2d`]: gathers the even bits of `code` into the
/// low 32 bits (bit `2 * i` moves to bit `i`); the odd bits are ignored.
const fn compact_2d(code: u64) -> u32 {
    let mut v = code & 0x5555_5555_5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333_3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v >> 16)) & 0x0000_0000_FFFF_FFFF;
    v as u32
}

/// Spreads the low 21 bits of `x` out to every third bit of a `u64` (bit `i`
/// moves to bit `3 * i`). Higher bits of `x` are dropped.
const fn spread_3d(x: u32) -> u64 {
    let mut v = (x as u64) & 0x001F_FFFF;
    v = (v | (v << 32)) & 0x001F_0000_0000_FFFF;
    v = (v | (v << 16)) & 0x001F_0000_FF00_00FF;
    v = (v | (v << 8)) & 0x100F_00F0_0F00_F00F;
    v = (v | (v << 4)) & 0x10C3_0C30_C30C_30C3;
    v = (v | (v << 2)) & 0x1249_2492_4924_9249;
    v
}

/// The inverse of [`spread_3d`]: gathers every third bit of `code`, starting
/// at bit `0`, into the low 21 bits (bit `3 * i` moves to bit `i`).
const fn compact_3d(code: u64) -> u32 {
    let mut v = code & 0x1249_2492_4924_9249;
    v = (v | (v >> 2)) & 0x10C3_0C30_C30C_30C3;
    v = (v | (v >> 4)) & 0x100F_00F0_0F00_F00F;
    v = (v | (v >> 8)) & 0x001F_0000_FF00_00FF;
    v = (v | (v >> 16)) & 0x001F_0000_0000_FFFF;
    v = (v | (v >> 32)) & 0x0000_0000_001F_FFFF;
    v as u32
}

/// The 2D Morton code of the cell `(x, y)`: the bits of `x` and `y`
/// interleaved, with bit `i` of `x` at bit `2 * i` of the result and bit `i`
/// of `y` at bit `2 * i + 1`.
///
/// Lossless: two 32-bit coordinates fill exactly the 64 bits of the code.
/// [`morton_decode_2d`] is the inverse.
///
/// ```
/// use math::bits::{morton_decode_2d, morton_encode_2d};
///
/// assert_eq!(morton_encode_2d(3, 5), 0b100_111);
/// assert_eq!(morton_decode_2d(0b100_111), (3, 5));
/// assert_eq!(morton_encode_2d(u32::MAX, u32::MAX), u64::MAX);
/// ```
#[inline]
pub const fn morton_encode_2d(x: u32, y: u32) -> u64 {
    spread_2d(x) | (spread_2d(y) << 1)
}

/// The cell `(x, y)` whose 2D Morton code is `code`; the inverse of
/// [`morton_encode_2d`].
#[inline]
pub const fn morton_decode_2d(code: u64) -> (u32, u32) {
    (compact_2d(code), compact_2d(code >> 1))
}

/// The 3D Morton code of the cell `(x, y, z)`: the bits of `x`, `y` and `z`
/// interleaved, with bit `i` of `x` at bit `3 * i` of the result, bit `i` of
/// `y` at bit `3 * i + 1` and bit `i` of `z` at bit `3 * i + 2`.
///
/// Each coordinate has **21 bits** (`0..2_097_152`) so that the code fits in
/// the 63 low bits of a `u64`; any higher bit of a coordinate is ignored.
/// [`morton_decode_3d`] is the inverse.
///
/// ```
/// use math::bits::{morton_decode_3d, morton_encode_3d};
///
/// assert_eq!(morton_encode_3d(0b11, 0b01, 0b10), 0b101_011);
/// assert_eq!(morton_decode_3d(0b101_011), (0b11, 0b01, 0b10));
/// // Bit 21 and above of each coordinate does not fit and is dropped.
/// assert_eq!(morton_encode_3d(1 << 21, 0, 0), 0);
/// ```
#[inline]
pub const fn morton_encode_3d(x: u32, y: u32, z: u32) -> u64 {
    spread_3d(x) | (spread_3d(y) << 1) | (spread_3d(z) << 2)
}

/// The cell `(x, y, z)` whose 3D Morton code is `code`; the inverse of
/// [`morton_encode_3d`]. Bit `63` of `code`, which no encoded cell uses, is
/// ignored.
#[inline]
pub const fn morton_decode_3d(code: u64) -> (u32, u32, u32) {
    (
        compact_3d(code),
        compact_3d(code >> 1),
        compact_3d(code >> 2),
    )
}

/// Generates the `const fn` versions of this module's functions for one
/// concrete integer type. Invoked once inside each of the 12 per-type
/// modules below. `$usable` is the number of bits a power of two can occupy
/// in the type: `BITS` for unsigned types, `BITS - 1` for signed ones (the
/// sign bit is not available).
macro_rules! const_bits {
    ($t:ident, $usable:expr) => {
        /// `const fn` version of [`super::popcount`].
        #[inline]
        pub const fn popcount(n: $t) -> u32 {
            n.count_ones()
        }

        /// `const fn` version of [`super::leading_zeros`].
        #[inline]
        pub const fn leading_zeros(n: $t) -> u32 {
            n.leading_zeros()
        }

        /// `const fn` version of [`super::trailing_zeros`].
        #[inline]
        pub const fn trailing_zeros(n: $t) -> u32 {
            n.trailing_zeros()
        }

        /// `const fn` version of [`super::reverse_bits`].
        #[inline]
        pub const fn reverse_bits(n: $t) -> $t {
            n.reverse_bits()
        }

        /// `const fn` version of [`super::rotate_left`].
        #[inline]
        pub const fn rotate_left(n: $t, count: u32) -> $t {
            n.rotate_left(count)
        }

        /// `const fn` version of [`super::rotate_right`].
        #[inline]
        pub const fn rotate_right(n: $t, count: u32) -> $t {
            n.rotate_right(count)
        }

        /// `const fn` version of [`super::is_power_of_two`].
        #[inline]
        pub const fn is_power_of_two(n: $t) -> bool {
            n > 0 && (n & (n - 1)) == 0
        }

        /// `const fn` version of [`super::next_power_of_two`].
        #[inline]
        pub const fn next_power_of_two(n: $t) -> Option<$t> {
            if n <= 1 {
                return Some(1);
            }
            // `n - 1` has `BITS - leading_zeros` significant bits; the
            // answer is one bit above them.
            let bits_needed = <$t>::BITS - (n - 1).leading_zeros();
            if bits_needed >= $usable {
                None
            } else {
                Some(1 << bits_needed)
            }
        }

        /// `const fn` version of [`super::prev_power_of_two`].
        #[inline]
        pub const fn prev_power_of_two(n: $t) -> Option<$t> {
            if n <= 0 {
                None
            } else {
                Some(1 << (<$t>::BITS - 1 - n.leading_zeros()))
            }
        }

        /// `const fn` version of [`super::align_up`].
        #[inline]
        pub const fn align_up(value: $t, align: $t) -> Option<$t> {
            if !is_power_of_two(align) {
                return None;
            }
            let mask = align - 1;
            match value.checked_add(mask) {
                Some(sum) => Some(sum & !mask),
                None => None,
            }
        }

        /// `const fn` version of [`super::align_down`].
        #[inline]
        pub const fn align_down(value: $t, align: $t) -> Option<$t> {
            if !is_power_of_two(align) {
                return None;
            }
            Some(value & !(align - 1))
        }

        /// `const fn` version of [`super::is_aligned`].
        #[inline]
        pub const fn is_aligned(value: $t, align: $t) -> bool {
            is_power_of_two(align) && (value & (align - 1)) == 0
        }
    };
}

/// `const fn` versions of this module's functions for `i8`.
pub mod i8 {
    const_bits!(i8, i8::BITS - 1);
}

/// `const fn` versions of this module's functions for `i16`.
pub mod i16 {
    const_bits!(i16, i16::BITS - 1);
}

/// `const fn` versions of this module's functions for `i32`.
pub mod i32 {
    const_bits!(i32, i32::BITS - 1);
}

/// `const fn` versions of this module's functions for `i64`.
pub mod i64 {
    const_bits!(i64, i64::BITS - 1);
}

/// `const fn` versions of this module's functions for `i128`.
pub mod i128 {
    const_bits!(i128, i128::BITS - 1);
}

/// `const fn` versions of this module's functions for `isize`.
pub mod isize {
    const_bits!(isize, isize::BITS - 1);
}

/// `const fn` versions of this module's functions for `u8`.
pub mod u8 {
    const_bits!(u8, u8::BITS);
}

/// `const fn` versions of this module's functions for `u16`.
pub mod u16 {
    const_bits!(u16, u16::BITS);
}

/// `const fn` versions of this module's functions for `u32`.
pub mod u32 {
    const_bits!(u32, u32::BITS);
}

/// `const fn` versions of this module's functions for `u64`.
pub mod u64 {
    const_bits!(u64, u64::BITS);
}

/// `const fn` versions of this module's functions for `u128`.
pub mod u128 {
    const_bits!(u128, u128::BITS);
}

/// `const fn` versions of this module's functions for `usize`.
pub mod usize {
    const_bits!(usize, usize::BITS);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n<T: Integer>(value: u8) -> T {
        T::from_u8(value).expect("small test value fits every integer type")
    }

    /// `T::ZERO - n(value)`; only meaningful for signed `T`.
    fn neg<T: Integer>(value: u8) -> T {
        T::ZERO - n::<T>(value)
    }

    fn check_counting<T: Integer>() {
        assert_eq!(popcount(T::ZERO), 0);
        assert_eq!(popcount(n::<T>(0b1011)), 3);
        assert_eq!(popcount(!T::ZERO), T::BITS);
        assert_eq!(leading_zeros(T::ZERO), T::BITS);
        assert_eq!(leading_zeros(T::ONE), T::BITS - 1);
        assert_eq!(leading_zeros(T::MAX), if T::SIGNED { 1 } else { 0 });
        assert_eq!(leading_zeros(T::MIN), if T::SIGNED { 0 } else { T::BITS });
        assert_eq!(trailing_zeros(T::ZERO), T::BITS);
        assert_eq!(trailing_zeros(T::ONE), 0);
        assert_eq!(trailing_zeros(n::<T>(8)), 3);
        assert_eq!(trailing_zeros(n::<T>(12)), 2);
    }

    #[test]
    fn counting_works_for_all_twelve_types() {
        each_int!(check_counting);
    }

    #[test]
    fn signed_negative_numbers_have_no_leading_zeros() {
        assert_eq!(leading_zeros(-1_i32), 0);
        assert_eq!(leading_zeros(i8::MIN), 0);
        assert_eq!(popcount(-1_i16), 16);
    }

    fn check_is_power_of_two<T: Integer>() {
        assert!(!is_power_of_two(T::ZERO));
        assert!(is_power_of_two(T::ONE));
        assert!(is_power_of_two(n::<T>(2)));
        assert!(!is_power_of_two(n::<T>(3)));
        assert!(is_power_of_two(n::<T>(4)));
        assert!(!is_power_of_two(n::<T>(6)));
        assert!(is_power_of_two(n::<T>(64)));
        assert!(!is_power_of_two(n::<T>(100)));
        // Zero (unsigned MIN) and negative numbers (signed MIN) never are.
        assert!(!is_power_of_two(T::MAX));
        assert!(!is_power_of_two(T::MIN));
        if T::SIGNED {
            assert!(!is_power_of_two(neg::<T>(1)));
            assert!(!is_power_of_two(neg::<T>(4)));
        } else {
            assert!(is_power_of_two(T::ONE << (T::BITS - 1)));
        }
    }

    #[test]
    fn is_power_of_two_works_for_all_twelve_types() {
        each_int!(check_is_power_of_two);
    }

    #[test]
    fn is_power_of_two_agrees_with_std_for_every_u16() {
        for v in 0..=u16::MAX {
            assert_eq!(is_power_of_two(v), v.is_power_of_two(), "{v}");
        }
    }

    fn check_next_power_of_two<T: Integer>() {
        assert_eq!(next_power_of_two(T::ZERO), Some(T::ONE));
        assert_eq!(next_power_of_two(T::ONE), Some(T::ONE));
        assert_eq!(next_power_of_two(n::<T>(2)), Some(n::<T>(2)));
        assert_eq!(next_power_of_two(n::<T>(3)), Some(n::<T>(4)));
        assert_eq!(next_power_of_two(n::<T>(5)), Some(n::<T>(8)));
        assert_eq!(next_power_of_two(n::<T>(64)), Some(n::<T>(64)));
        assert_eq!(
            next_power_of_two(n::<T>(65)).is_some(),
            T::BITS > 8 || !T::SIGNED
        );
        assert_eq!(next_power_of_two(T::MAX), None);
        if T::SIGNED {
            assert_eq!(next_power_of_two(neg::<T>(5)), Some(T::ONE));
            assert_eq!(next_power_of_two(T::MIN), Some(T::ONE));
        }
    }

    #[test]
    fn next_power_of_two_works_for_all_twelve_types() {
        each_int!(check_next_power_of_two);
    }

    #[test]
    fn next_power_of_two_boundaries() {
        assert_eq!(next_power_of_two(128_u8), Some(128));
        assert_eq!(next_power_of_two(129_u8), None);
        assert_eq!(next_power_of_two(64_i8), Some(64));
        assert_eq!(next_power_of_two(65_i8), None);
        assert_eq!(next_power_of_two(1_u32 << 31), Some(1 << 31));
        assert_eq!(next_power_of_two((1_u32 << 31) + 1), None);
        assert_eq!(next_power_of_two(1_i32 << 30), Some(1 << 30));
        assert_eq!(next_power_of_two((1_i32 << 30) + 1), None);
        assert_eq!(next_power_of_two(1000_u16), Some(1024));
    }

    #[test]
    fn next_power_of_two_agrees_with_std_for_every_u16() {
        for v in 0..=u16::MAX {
            assert_eq!(next_power_of_two(v), v.checked_next_power_of_two(), "{v}");
        }
    }

    fn check_prev_power_of_two<T: Integer>() {
        assert_eq!(prev_power_of_two(T::ZERO), None);
        assert_eq!(prev_power_of_two(T::ONE), Some(T::ONE));
        assert_eq!(prev_power_of_two(n::<T>(2)), Some(n::<T>(2)));
        assert_eq!(prev_power_of_two(n::<T>(3)), Some(n::<T>(2)));
        assert_eq!(prev_power_of_two(n::<T>(100)), Some(n::<T>(64)));
        assert_eq!(prev_power_of_two(n::<T>(64)), Some(n::<T>(64)));
        assert_eq!(
            prev_power_of_two(T::MAX),
            Some(T::ONE << (if T::SIGNED { T::BITS - 2 } else { T::BITS - 1 }))
        );
        if T::SIGNED {
            assert_eq!(prev_power_of_two(neg::<T>(3)), None);
            assert_eq!(prev_power_of_two(T::MIN), None);
        }
    }

    #[test]
    fn prev_power_of_two_works_for_all_twelve_types() {
        each_int!(check_prev_power_of_two);
    }

    #[test]
    fn prev_power_of_two_boundaries() {
        assert_eq!(prev_power_of_two(255_u8), Some(128));
        assert_eq!(prev_power_of_two(127_i8), Some(64));
        assert_eq!(prev_power_of_two(u32::MAX), Some(1 << 31));
    }

    #[test]
    fn prev_and_next_bracket_every_positive_u16() {
        for v in 1..=u16::MAX {
            let prev = prev_power_of_two(v).unwrap();
            assert!(is_power_of_two(prev));
            assert!(prev <= v && u32::from(v) < u32::from(prev) * 2, "{v}");
            if let Some(next) = next_power_of_two(v) {
                assert!(is_power_of_two(next) && next >= v, "{v}");
                assert!(u32::from(next) <= u32::from(prev) * 2, "{v}");
            }
        }
    }

    #[test]
    fn align_up_known_values() {
        assert_eq!(align_up(0_usize, 8), Some(0));
        assert_eq!(align_up(1_usize, 8), Some(8));
        assert_eq!(align_up(8_usize, 8), Some(8));
        assert_eq!(align_up(9_usize, 8), Some(16));
        assert_eq!(align_up(13_u32, 1), Some(13));
        assert_eq!(align_up(4095_u64, 4096), Some(4096));
        assert_eq!(align_up(4097_u64, 4096), Some(8192));
    }

    #[test]
    fn align_up_rejects_bad_alignments() {
        assert_eq!(align_up(13_u32, 0), None);
        assert_eq!(align_up(13_u32, 3), None);
        assert_eq!(align_up(13_u32, 12), None);
        assert_eq!(align_up(13_i32, -4), None);
        assert_eq!(align_up(13_i8, i8::MIN), None);
    }

    #[test]
    fn align_up_overflow_is_none() {
        assert_eq!(align_up(250_u8, 16), None);
        assert_eq!(align_up(241_u8, 16), None);
        assert_eq!(align_up(240_u8, 16), Some(240));
        assert_eq!(align_up(125_i8, 4), None);
        assert_eq!(align_up(124_i8, 4), Some(124));
        assert_eq!(align_up(u64::MAX, 2), None);
        assert_eq!(align_up(u64::MAX, 1), Some(u64::MAX));
    }

    #[test]
    fn align_down_known_values() {
        assert_eq!(align_down(0_usize, 8), Some(0));
        assert_eq!(align_down(7_usize, 8), Some(0));
        assert_eq!(align_down(8_usize, 8), Some(8));
        assert_eq!(align_down(15_usize, 8), Some(8));
        assert_eq!(align_down(250_u8, 16), Some(240));
        assert_eq!(align_down(u64::MAX, 4096), Some(u64::MAX - 4095));
        assert_eq!(align_down(13_u32, 3), None);
        assert_eq!(align_down(13_u32, 0), None);
    }

    #[test]
    fn alignment_of_negative_numbers_rounds_consistently() {
        assert_eq!(align_up(-5_i32, 4), Some(-4));
        assert_eq!(align_down(-5_i32, 4), Some(-8));
        assert_eq!(align_up(-8_i32, 4), Some(-8));
        assert_eq!(align_down(-8_i32, 4), Some(-8));
        assert_eq!(align_up(-1_i8, 16), Some(0));
        assert_eq!(align_down(-1_i8, 16), Some(-16));
        assert_eq!(align_down(i8::MIN, 4), Some(i8::MIN));
        assert_eq!(align_up(i8::MIN, 64), Some(i8::MIN));
    }

    #[test]
    fn align_up_and_down_bracket_the_value() {
        for align in [1_u16, 2, 4, 8, 64, 256, 4096] {
            for v in (0..=u16::MAX).step_by(7) {
                let down = align_down(v, align).unwrap();
                assert!(down <= v && v - down < align, "down {v} {align}");
                assert!(is_aligned(down, align));
                if let Some(up) = align_up(v, align) {
                    assert!(up >= v && up - v < align, "up {v} {align}");
                    assert!(is_aligned(up, align));
                }
            }
        }
    }

    #[test]
    fn is_aligned_cases() {
        assert!(is_aligned(0_u32, 8));
        assert!(is_aligned(16_u32, 8));
        assert!(!is_aligned(12_u32, 8));
        assert!(is_aligned(12_u32, 4));
        assert!(is_aligned(7_u32, 1));
        assert!(is_aligned(-8_i32, 4));
        assert!(!is_aligned(-6_i32, 4));
        // An invalid alignment is never "aligned", not even for zero.
        assert!(!is_aligned(0_u32, 0));
        assert!(!is_aligned(12_u32, 6));
        assert!(!is_aligned(8_i32, -4));
    }

    fn check_alignment_generic<T: Integer>() {
        let eight = n::<T>(8);
        assert_eq!(align_up(n::<T>(9), eight), Some(n::<T>(16)));
        assert_eq!(align_down(n::<T>(9), eight), Some(eight));
        assert_eq!(align_up(eight, eight), Some(eight));
        assert!(is_aligned(n::<T>(16), eight));
        assert!(!is_aligned(n::<T>(17), eight));
        assert_eq!(align_up(n::<T>(9), n::<T>(6)), None);
        assert_eq!(align_up(T::MAX, eight), None);
    }

    #[test]
    fn alignment_works_for_all_twelve_types() {
        each_int!(check_alignment_generic);
    }

    // ---- Phase 2.6: reverse_bits, rotation, Morton codes ----

    fn check_reverse_bits<T: Integer>() {
        assert_eq!(reverse_bits(T::ZERO), T::ZERO);
        assert_eq!(reverse_bits(!T::ZERO), !T::ZERO);
        assert_eq!(reverse_bits(T::ONE), T::ONE << (T::BITS - 1));
        assert_eq!(reverse_bits(T::ONE << (T::BITS - 1)), T::ONE);
        // 0b1011 has bits 0, 1 and 3 set; reversed they are the top bit, the
        // one below it, and the fourth from the top: 0b1101 in the top nibble.
        assert_eq!(
            reverse_bits(n::<T>(0b1011)),
            n::<T>(0b1101) << (T::BITS - 4)
        );
        let x = n::<T>(0b1011);
        assert_eq!(reverse_bits(reverse_bits(x)), x);
        assert_eq!(popcount(reverse_bits(x)), popcount(x));
    }

    #[test]
    fn reverse_bits_works_for_all_twelve_types() {
        each_int!(check_reverse_bits);
        assert_eq!(reverse_bits(1_i8), i8::MIN);
        assert_eq!(reverse_bits(0x12_34_u16), 0x2C_48);
        assert_eq!(reverse_bits(0x0000_0001_u32), 0x8000_0000);
    }

    fn check_rotate<T: Integer>() {
        let x = n::<T>(0b1011);
        let top = T::ONE << (T::BITS - 1);
        assert_eq!(rotate_left(x, 0), x);
        assert_eq!(rotate_right(x, 0), x);
        // A full turn, and more than one, changes nothing.
        assert_eq!(rotate_left(x, T::BITS), x);
        assert_eq!(rotate_right(x, T::BITS), x);
        assert_eq!(rotate_left(x, T::BITS + 1), rotate_left(x, 1));
        assert_eq!(rotate_left(T::ONE, T::BITS - 1), top);
        assert_eq!(rotate_right(T::ONE, 1), top);
        assert_eq!(rotate_left(top, 1), T::ONE);
        assert_eq!(rotate_right(top, T::BITS - 1), T::ONE);
        assert_eq!(rotate_right(rotate_left(x, 3), 3), x);
        assert_eq!(rotate_left(rotate_right(x, 3), 3), x);
        assert_eq!(popcount(rotate_left(!x, 5)), popcount(!x));
        assert_eq!(rotate_left(!T::ZERO, 7), !T::ZERO);
        // Rotating left by `k` is rotating right by `BITS - k`.
        assert_eq!(rotate_left(x, 2), rotate_right(x, T::BITS - 2));
    }

    #[test]
    fn rotation_works_for_all_twelve_types() {
        each_int!(check_rotate);
        assert_eq!(rotate_left(0b1000_0001_u8, 1), 0b0000_0011);
        assert_eq!(rotate_right(0b1000_0001_u8, 1), 0b1100_0000);
        assert_eq!(rotate_left(0x1234_5678_u32, 8), 0x3456_7812);
        assert_eq!(rotate_left(i8::MIN, 1), 1);
    }

    /// The Morton code of `(x, y)` built bit by bit, as the reference.
    fn naive_encode_2d(x: u32, y: u32) -> u64 {
        let mut code = 0_u64;
        for i in 0..32 {
            code |= u64::from((x >> i) & 1) << (2 * i);
            code |= u64::from((y >> i) & 1) << (2 * i + 1);
        }
        code
    }

    /// The Morton code of `(x, y, z)` built bit by bit (21 bits each).
    fn naive_encode_3d(x: u32, y: u32, z: u32) -> u64 {
        let mut code = 0_u64;
        for i in 0..21 {
            code |= u64::from((x >> i) & 1) << (3 * i);
            code |= u64::from((y >> i) & 1) << (3 * i + 1);
            code |= u64::from((z >> i) & 1) << (3 * i + 2);
        }
        code
    }

    /// A tiny deterministic generator (an LCG) for the randomized tests.
    fn next_u32(state: &mut u64) -> u32 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) as u32
    }

    #[test]
    fn morton_2d_known_values() {
        assert_eq!(morton_encode_2d(0, 0), 0);
        assert_eq!(morton_encode_2d(1, 0), 0b01);
        assert_eq!(morton_encode_2d(0, 1), 0b10);
        assert_eq!(morton_encode_2d(0b11, 0b00), 0b0101);
        assert_eq!(morton_encode_2d(0b00, 0b11), 0b1010);
        assert_eq!(morton_encode_2d(3, 5), 0b100_111);
        assert_eq!(morton_encode_2d(u32::MAX, 0), 0x5555_5555_5555_5555);
        assert_eq!(morton_encode_2d(0, u32::MAX), 0xAAAA_AAAA_AAAA_AAAA);
        assert_eq!(morton_encode_2d(u32::MAX, u32::MAX), u64::MAX);
        assert_eq!(morton_decode_2d(0), (0, 0));
        assert_eq!(morton_decode_2d(u64::MAX), (u32::MAX, u32::MAX));
        assert_eq!(morton_decode_2d(0b100_111), (3, 5));
    }

    #[test]
    fn morton_2d_matches_the_bit_by_bit_reference() {
        for x in 0..64 {
            for y in 0..64 {
                assert_eq!(morton_encode_2d(x, y), naive_encode_2d(x, y), "{x} {y}");
                assert_eq!(morton_decode_2d(morton_encode_2d(x, y)), (x, y));
            }
        }
        let mut state = 1;
        for _ in 0..5_000 {
            let (x, y) = (next_u32(&mut state), next_u32(&mut state));
            assert_eq!(morton_encode_2d(x, y), naive_encode_2d(x, y), "{x} {y}");
            assert_eq!(morton_decode_2d(morton_encode_2d(x, y)), (x, y));
        }
    }

    #[test]
    fn morton_2d_decode_then_encode_is_the_identity() {
        let mut state = 2;
        for _ in 0..5_000 {
            let code = u64::from(next_u32(&mut state)) << 32 | u64::from(next_u32(&mut state));
            let (x, y) = morton_decode_2d(code);
            assert_eq!(morton_encode_2d(x, y), code);
        }
    }

    #[test]
    fn morton_2d_parent_cell_is_the_code_shifted_by_two() {
        for x in 0..32 {
            for y in 0..32 {
                assert_eq!(
                    morton_encode_2d(x, y) >> 2,
                    morton_encode_2d(x >> 1, y >> 1),
                    "{x} {y}"
                );
            }
        }
    }

    #[test]
    fn morton_3d_known_values() {
        assert_eq!(morton_encode_3d(0, 0, 0), 0);
        assert_eq!(morton_encode_3d(1, 0, 0), 0b001);
        assert_eq!(morton_encode_3d(0, 1, 0), 0b010);
        assert_eq!(morton_encode_3d(0, 0, 1), 0b100);
        assert_eq!(morton_encode_3d(0b11, 0b01, 0b10), 0b101_011);
        assert_eq!(morton_encode_3d(0x1F_FFFF, 0, 0), 0x1249_2492_4924_9249);
        let full = 0x1F_FFFF;
        assert_eq!(morton_encode_3d(full, full, full), (1 << 63) - 1);
        assert_eq!(morton_decode_3d((1 << 63) - 1), (full, full, full));
        assert_eq!(morton_decode_3d(0b101_011), (0b11, 0b01, 0b10));
    }

    #[test]
    fn morton_3d_drops_coordinate_bits_above_21() {
        assert_eq!(morton_encode_3d(1 << 21, 0, 0), 0);
        assert_eq!(morton_encode_3d(0, u32::MAX << 21, 0), 0);
        assert_eq!(morton_encode_3d(0x20_0005, 0, 0), morton_encode_3d(5, 0, 0));
        // Bit 63 of a code is not part of any cell and is ignored.
        assert_eq!(morton_decode_3d(1 << 63), (0, 0, 0));
        assert_eq!(morton_decode_3d(u64::MAX), morton_decode_3d(u64::MAX >> 1));
    }

    #[test]
    fn morton_3d_matches_the_bit_by_bit_reference() {
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    assert_eq!(
                        morton_encode_3d(x, y, z),
                        naive_encode_3d(x, y, z),
                        "{x} {y} {z}"
                    );
                    assert_eq!(morton_decode_3d(morton_encode_3d(x, y, z)), (x, y, z));
                }
            }
        }
        let mut state = 3;
        for _ in 0..5_000 {
            let (x, y, z) = (
                next_u32(&mut state),
                next_u32(&mut state),
                next_u32(&mut state),
            );
            assert_eq!(morton_encode_3d(x, y, z), naive_encode_3d(x, y, z));
            let mask = 0x1F_FFFF;
            assert_eq!(
                morton_decode_3d(morton_encode_3d(x, y, z)),
                (x & mask, y & mask, z & mask)
            );
        }
    }

    #[test]
    fn morton_3d_parent_cell_is_the_code_shifted_by_three() {
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    assert_eq!(
                        morton_encode_3d(x, y, z) >> 3,
                        morton_encode_3d(x >> 1, y >> 1, z >> 1),
                        "{x} {y} {z}"
                    );
                }
            }
        }
    }

    #[test]
    fn morton_functions_work_in_const_contexts() {
        const CODE_2D: u64 = morton_encode_2d(3, 5);
        const CELL_2D: (u32, u32) = morton_decode_2d(CODE_2D);
        const CODE_3D: u64 = morton_encode_3d(5, 6, 7);
        const CELL_3D: (u32, u32, u32) = morton_decode_3d(CODE_3D);
        assert_eq!(CELL_2D, (3, 5));
        assert_eq!(CELL_3D, (5, 6, 7));
    }

    // ---- the concrete `const fn` modules agree with the generic functions

    /// A spread of interesting values of the type, negated copies included
    /// (a large unsigned value for unsigned types).
    macro_rules! samples {
        ($t:ident) => {{
            let base: [$t; 10] = [0, 1, 2, 3, 6, 7, 10, 64, <$t>::MAX - 1, <$t>::MAX];
            let mut out = [0 as $t; 21];
            let mut i = 0;
            while i < base.len() {
                out[i] = base[i];
                out[10 + i] = base[i].wrapping_neg();
                i += 1;
            }
            out[20] = <$t>::MIN;
            out
        }};
    }

    /// Asserts that `bits::$t`'s functions equal the generic ones on every
    /// sample (and on every pair of samples, for the two-argument ones).
    macro_rules! assert_concrete_matches_generic {
        ($t:ident) => {{
            let xs = samples!($t);
            for &a in &xs {
                assert_eq!($t::popcount(a), popcount(a));
                assert_eq!($t::leading_zeros(a), leading_zeros(a));
                assert_eq!($t::trailing_zeros(a), trailing_zeros(a));
                assert_eq!($t::reverse_bits(a), reverse_bits(a));
                assert_eq!($t::is_power_of_two(a), is_power_of_two(a));
                assert_eq!($t::next_power_of_two(a), next_power_of_two(a));
                assert_eq!($t::prev_power_of_two(a), prev_power_of_two(a));
                for k in [0, 1, 3, 7, 8, 31, 32, 64, 129] {
                    assert_eq!($t::rotate_left(a, k), rotate_left(a, k));
                    assert_eq!($t::rotate_right(a, k), rotate_right(a, k));
                }
                for &b in &xs {
                    assert_eq!($t::align_up(a, b), align_up(a, b));
                    assert_eq!($t::align_down(a, b), align_down(a, b));
                    assert_eq!($t::is_aligned(a, b), is_aligned(a, b));
                }
            }
        }};
    }

    #[test]
    fn concrete_modules_match_the_generic_functions() {
        assert_concrete_matches_generic!(i8);
        assert_concrete_matches_generic!(i16);
        assert_concrete_matches_generic!(i32);
        assert_concrete_matches_generic!(i64);
        assert_concrete_matches_generic!(i128);
        assert_concrete_matches_generic!(isize);
        assert_concrete_matches_generic!(u8);
        assert_concrete_matches_generic!(u16);
        assert_concrete_matches_generic!(u32);
        assert_concrete_matches_generic!(u64);
        assert_concrete_matches_generic!(u128);
        assert_concrete_matches_generic!(usize);
    }

    #[test]
    fn concrete_modules_match_the_generic_functions_for_every_8_bit_value() {
        for a in i8::MIN..=i8::MAX {
            assert_eq!(i8::next_power_of_two(a), next_power_of_two(a));
            assert_eq!(i8::prev_power_of_two(a), prev_power_of_two(a));
            assert_eq!(i8::is_power_of_two(a), is_power_of_two(a));
            for b in i8::MIN..=i8::MAX {
                assert_eq!(i8::align_up(a, b), align_up(a, b));
                assert_eq!(i8::align_down(a, b), align_down(a, b));
                assert_eq!(i8::is_aligned(a, b), is_aligned(a, b));
            }
        }
        for a in 0..=u8::MAX {
            assert_eq!(u8::next_power_of_two(a), next_power_of_two(a));
            assert_eq!(u8::prev_power_of_two(a), prev_power_of_two(a));
            for b in 0..=u8::MAX {
                assert_eq!(u8::align_up(a, b), align_up(a, b));
                assert_eq!(u8::align_down(a, b), align_down(a, b));
                assert_eq!(u8::is_aligned(a, b), is_aligned(a, b));
            }
        }
    }

    #[test]
    fn concrete_modules_work_in_const_contexts() {
        const POP: u32 = u64::popcount(0xFF00);
        const REV: u8 = u8::reverse_bits(0b0000_0110);
        const ROT: u32 = u32::rotate_left(0x8000_0001, 1);
        const NEXT: Option<u32> = u32::next_power_of_two(1000);
        const NEXT_SIGNED: Option<i8> = i8::next_power_of_two(65);
        const PREV: Option<u16> = u16::prev_power_of_two(1000);
        const UP: Option<usize> = usize::align_up(13, 8);
        const DOWN: Option<i32> = i32::align_down(-5, 4);
        const ALIGNED: bool = u64::is_aligned(4096, 4096);
        assert_eq!(POP, 8);
        assert_eq!(REV, 0b0110_0000);
        assert_eq!(ROT, 0x0000_0003);
        assert_eq!(NEXT, Some(1024));
        assert_eq!(NEXT_SIGNED, None);
        assert_eq!(PREV, Some(512));
        assert_eq!(UP, Some(16));
        assert_eq!(DOWN, Some(-8));
        assert!(core::hint::black_box(ALIGNED));
    }
}
