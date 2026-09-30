//! Bit tricks: [`popcount`], [`leading_zeros`], [`trailing_zeros`],
//! power-of-two tests ([`is_power_of_two`], [`next_power_of_two`],
//! [`prev_power_of_two`]) and alignment ([`align_up`], [`align_down`],
//! [`is_aligned`]).
//!
//! Everything is generic over [`Integer`], so one definition serves all 12
//! standard integer types. Signed types are supported where it makes sense:
//! a negative number is never a power of two, and alignment rounds towards
//! positive infinity (`align_up`) or negative infinity (`align_down`) using
//! two's-complement masking, so `align_down(-5, 4) == Some(-8)`.
//!
//! Nothing here panics. Where a result can be out of range or an argument
//! invalid (an alignment that is not a power of two, a next power of two
//! that overflows), the function returns `None`.
//!
//! ```
//! use math::bits::{align_up, next_power_of_two};
//!
//! assert_eq!(next_power_of_two(100_u32), Some(128));
//! assert_eq!(align_up(13_usize, 8), Some(16));
//! assert_eq!(align_up(13_usize, 6), None); // 6 is not a power of two
//! ```
//!
//! # `const`
//!
//! These are generic over a trait, and trait methods cannot be `const` on
//! stable Rust, so they cannot be `const fn`. For `const` contexts the
//! primitives have their own `const` methods (`u32::count_ones`,
//! `u32::next_power_of_two`, `u32::next_multiple_of`, ...).

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
}
