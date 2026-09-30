//! Integer math: [`gcd`], [`lcm`], [`is_prime`] and overflow-safe
//! arithmetic wrappers ([`checked_add`], [`saturating_mul`],
//! [`wrapping_pow`], ...).
//!
//! Everything is generic over [`Integer`], so one definition serves all 12
//! standard integer types.
//!
//! # Signed inputs and the `Unsigned` result
//!
//! [`gcd`] and [`lcm`] accept signed and unsigned types alike but return
//! [`Integer::Unsigned`]: the result is a magnitude, and for a signed type
//! the exact answer can be `|MIN|`, which the signed type cannot hold
//! (`gcd(i32::MIN, 0) == 2_147_483_648`). For unsigned inputs
//! `Integer::Unsigned` is the input type itself, so nothing changes:
//!
//! ```
//! use math::int_math::{gcd, lcm};
//!
//! assert_eq!(gcd(12_u32, 18), 6);
//! assert_eq!(gcd(-12_i32, 18), 6_u32); // an `i32` in, a `u32` out
//! assert_eq!(gcd(i32::MIN, 0), 2_147_483_648_u32);
//! assert_eq!(lcm(4_u8, 6), Some(12));
//! assert_eq!(lcm(200_u8, 201), None); // does not fit in a `u8`
//! ```
//!
//! # `const`
//!
//! The functions here are generic over a trait, and trait methods cannot be
//! `const` on stable Rust, so they cannot be `const fn` either. The
//! primitives' own `const` inherent methods (`u32::checked_add`,
//! `i32::unsigned_abs`, ...) are available for `const` contexts.

use crate::integer::{Integer, Unsigned};

/// The greatest common divisor of `a` and `b`, always non-negative.
///
/// `gcd(0, 0) == 0`, and `gcd(x, 0) == |x|`. The sign of the inputs does not
/// matter: `gcd(-12, 18) == gcd(12, -18) == 6`. Uses the binary (Stein)
/// algorithm, so it needs no division.
///
/// Returns [`Integer::Unsigned`] so that `MIN` inputs cannot overflow; see
/// the [module docs](self).
#[inline]
pub fn gcd<T: Integer>(a: T, b: T) -> T::Unsigned {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();
    if a == T::Unsigned::ZERO {
        return b;
    }
    if b == T::Unsigned::ZERO {
        return a;
    }
    // Factor out the powers of two both share, then work on odd values.
    let common_twos = (a | b).trailing_zeros();
    a = a >> a.trailing_zeros();
    loop {
        b = b >> b.trailing_zeros();
        if a > b {
            core::mem::swap(&mut a, &mut b);
        }
        b = b - a;
        if b == T::Unsigned::ZERO {
            return a << common_twos;
        }
    }
}

/// The least common multiple of `a` and `b`, always non-negative, or `None`
/// if it does not fit in [`Integer::Unsigned`].
///
/// If either input is zero the result is `Some(0)`. The sign of the inputs
/// does not matter: `lcm(-4, 6) == Some(12)`.
#[inline]
pub fn lcm<T: Integer>(a: T, b: T) -> Option<T::Unsigned> {
    let (ma, mb) = (a.unsigned_abs(), b.unsigned_abs());
    if ma == T::Unsigned::ZERO || mb == T::Unsigned::ZERO {
        return Some(T::Unsigned::ZERO);
    }
    // Divide before multiplying to keep the intermediate as small as possible.
    (ma / gcd(a, b)).checked_mul(mb)
}

/// Whether `n` is prime. `0` and `1` are not.
///
/// Only unsigned types are accepted (`T: Unsigned`): primality of a negative
/// number is not a question this function answers, and the bound turns
/// passing an `i32` into a compile error.
///
/// This is trial division by `6k ± 1` up to `√n`, so it takes `O(√n)` time:
/// instant for anything a game normally checks, but it is not meant for
/// 64-bit or 128-bit values near the top of their range.
///
/// ```
/// use math::int_math::is_prime;
///
/// assert!(is_prime(2_u32));
/// assert!(is_prime(4_294_967_291_u32)); // the largest prime below 2^32
/// assert!(!is_prime(1_u8));
/// assert!(!is_prime(91_u8)); // 7 * 13
/// ```
#[inline]
pub fn is_prime<T: Unsigned>(n: T) -> bool {
    let two = T::ONE + T::ONE;
    let three = two + T::ONE;
    if n < two {
        return false;
    }
    if n < two + two {
        // 2 and 3.
        return true;
    }
    if n % two == T::ZERO || n % three == T::ZERO {
        return false;
    }
    // Every prime above 3 is 6k - 1 or 6k + 1. `i <= n / i` is `i * i <= n`
    // without the risk of `i * i` overflowing.
    let six = three + three;
    let mut i = six - T::ONE;
    while i <= n / i {
        if n % i == T::ZERO || n % (i + two) == T::ZERO {
            return false;
        }
        i = i + six;
    }
    true
}

/// `a + b`, or `None` on overflow.
#[inline]
pub fn checked_add<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_add(b)
}

/// `a - b`, or `None` on overflow.
#[inline]
pub fn checked_sub<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_sub(b)
}

/// `a * b`, or `None` on overflow.
#[inline]
pub fn checked_mul<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_mul(b)
}

/// `a / b`, or `None` if `b` is zero or the division overflows
/// (`MIN / -1` for signed types).
#[inline]
pub fn checked_div<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_div(b)
}

/// `a % b`, or `None` if `b` is zero or the division overflows
/// (`MIN % -1` for signed types).
#[inline]
pub fn checked_rem<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_rem(b)
}

/// `base` to the power `exp`, or `None` on overflow.
#[inline]
pub fn checked_pow<T: Integer>(base: T, exp: u32) -> Option<T> {
    base.checked_pow(exp)
}

/// `a + b`, clamped to `[MIN, MAX]` instead of overflowing.
#[inline]
pub fn saturating_add<T: Integer>(a: T, b: T) -> T {
    a.saturating_add(b)
}

/// `a - b`, clamped to `[MIN, MAX]` instead of overflowing.
#[inline]
pub fn saturating_sub<T: Integer>(a: T, b: T) -> T {
    a.saturating_sub(b)
}

/// `a * b`, clamped to `[MIN, MAX]` instead of overflowing.
#[inline]
pub fn saturating_mul<T: Integer>(a: T, b: T) -> T {
    a.saturating_mul(b)
}

/// `base` to the power `exp`, clamped to `[MIN, MAX]` instead of
/// overflowing.
#[inline]
pub fn saturating_pow<T: Integer>(base: T, exp: u32) -> T {
    base.saturating_pow(exp)
}

/// `a + b`, wrapping around at the type's boundary.
#[inline]
pub fn wrapping_add<T: Integer>(a: T, b: T) -> T {
    a.wrapping_add(b)
}

/// `a - b`, wrapping around at the type's boundary.
#[inline]
pub fn wrapping_sub<T: Integer>(a: T, b: T) -> T {
    a.wrapping_sub(b)
}

/// `a * b`, wrapping around at the type's boundary.
#[inline]
pub fn wrapping_mul<T: Integer>(a: T, b: T) -> T {
    a.wrapping_mul(b)
}

/// `base` to the power `exp`, wrapping around at the type's boundary.
#[inline]
pub fn wrapping_pow<T: Integer>(base: T, exp: u32) -> T {
    base.wrapping_pow(exp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n<T: Integer>(value: u8) -> T {
        T::from_u8(value).expect("small test value fits every integer type")
    }

    /// The obvious Euclid, on `u128`, as the reference for `gcd`.
    fn euclid(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }

    #[test]
    fn gcd_known_values() {
        assert_eq!(gcd(12_u32, 18), 6);
        assert_eq!(gcd(17_u32, 5), 1);
        assert_eq!(gcd(100_u64, 75), 25);
        assert_eq!(gcd(48_u8, 36), 12);
        assert_eq!(gcd(1_u16, 65_535), 1);
        assert_eq!(gcd(7_usize, 7), 7);
    }

    #[test]
    fn gcd_with_zero() {
        assert_eq!(gcd(0_u32, 0), 0);
        assert_eq!(gcd(0_u32, 9), 9);
        assert_eq!(gcd(9_u32, 0), 9);
        assert_eq!(gcd(-9_i32, 0), 9_u32);
        assert_eq!(gcd(0_i32, 0), 0_u32);
    }

    #[test]
    fn gcd_ignores_sign() {
        assert_eq!(gcd(-12_i32, 18), 6_u32);
        assert_eq!(gcd(12_i32, -18), 6_u32);
        assert_eq!(gcd(-12_i32, -18), 6_u32);
        assert_eq!(gcd(-1_i64, 1), 1_u64);
    }

    #[test]
    fn gcd_of_min_does_not_overflow() {
        assert_eq!(gcd(i8::MIN, 0), 128_u8);
        assert_eq!(gcd(0, i8::MIN), 128_u8);
        assert_eq!(gcd(i8::MIN, i8::MIN), 128_u8);
        assert_eq!(gcd(i8::MIN, 6), 2_u8);
        assert_eq!(gcd(i8::MIN, -64), 64_u8);
        assert_eq!(gcd(i32::MIN, 0), 2_147_483_648_u32);
        assert_eq!(gcd(i64::MIN, i64::MIN), 9_223_372_036_854_775_808_u64);
        assert_eq!(
            gcd(i128::MIN, 0),
            170_141_183_460_469_231_731_687_303_715_884_105_728_u128
        );
    }

    #[test]
    fn gcd_matches_euclid_for_every_u8_pair() {
        for a in 0..=u8::MAX {
            for b in 0..=u8::MAX {
                assert_eq!(
                    u128::from(gcd(a, b)),
                    euclid(a.into(), b.into()),
                    "gcd({a}, {b})"
                );
            }
        }
    }

    #[test]
    fn gcd_matches_euclid_for_every_i8_pair() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                let expected = euclid(u128::from(a.unsigned_abs()), u128::from(b.unsigned_abs()));
                assert_eq!(u128::from(gcd(a, b)), expected, "gcd({a}, {b})");
            }
        }
    }

    #[test]
    fn gcd_large_values() {
        assert_eq!(gcd(u64::MAX, u64::MAX - 1), 1);
        assert_eq!(gcd(u64::MAX, 3), 3);
        assert_eq!(gcd(1_u128 << 100, 1_u128 << 60), 1_u128 << 60);
        assert_eq!(gcd(2_u64.pow(40) * 3, 2_u64.pow(35) * 9), 2_u64.pow(35) * 3);
    }

    #[test]
    fn lcm_known_values() {
        assert_eq!(lcm(4_u8, 6), Some(12));
        assert_eq!(lcm(21_u32, 6), Some(42));
        assert_eq!(lcm(5_u64, 7), Some(35));
        assert_eq!(lcm(9_u16, 9), Some(9));
        assert_eq!(lcm(-4_i32, 6), Some(12_u32));
        assert_eq!(lcm(-4_i32, -6), Some(12_u32));
    }

    #[test]
    fn lcm_with_zero_is_zero() {
        assert_eq!(lcm(0_u32, 5), Some(0));
        assert_eq!(lcm(5_u32, 0), Some(0));
        assert_eq!(lcm(0_i8, 0), Some(0_u8));
    }

    #[test]
    fn lcm_overflow_is_none() {
        assert_eq!(lcm(200_u8, 201), None);
        assert_eq!(lcm(u32::MAX, u32::MAX - 1), None);
        assert_eq!(lcm(u64::MAX, 2), None);
        // Fits only because the unsigned result has one more bit than i8.
        assert_eq!(lcm(i8::MIN, 1), Some(128_u8));
        assert_eq!(lcm(i8::MIN, 3), None);
        assert_eq!(lcm(200_u8, 200), Some(200));
    }

    #[test]
    fn lcm_times_gcd_is_the_product() {
        for a in 1_u8..=60 {
            for b in 1_u8..=60 {
                let l = lcm(a, b).map_or(0, u128::from);
                if l != 0 {
                    assert_eq!(l * u128::from(gcd(a, b)), u128::from(a) * u128::from(b));
                }
            }
        }
    }

    /// A sieve of Eratosthenes over `0..=65_535`, as the reference for primes.
    fn sieve() -> [bool; 65_536] {
        let mut is_p = [true; 65_536];
        is_p[0] = false;
        is_p[1] = false;
        let mut i = 2;
        while i * i < is_p.len() {
            if is_p[i] {
                let mut j = i * i;
                while j < is_p.len() {
                    is_p[j] = false;
                    j += i;
                }
            }
            i += 1;
        }
        is_p
    }

    #[test]
    fn is_prime_matches_a_sieve_for_every_u8() {
        let reference = sieve();
        for n in 0..=u8::MAX {
            assert_eq!(is_prime(n), reference[usize::from(n)], "{n}");
        }
    }

    #[test]
    fn is_prime_matches_a_sieve_for_every_u16() {
        let reference = sieve();
        for n in 0..=u16::MAX {
            assert_eq!(is_prime(n), reference[usize::from(n)], "{n}");
        }
    }

    #[test]
    fn is_prime_small_edge_cases() {
        assert!(!is_prime(0_u32));
        assert!(!is_prime(1_u32));
        assert!(is_prime(2_u32));
        assert!(is_prime(3_u32));
        assert!(!is_prime(4_u32));
        assert!(is_prime(5_u32));
        assert!(is_prime(251_u8)); // the largest prime a u8 holds
        assert!(!is_prime(255_u8));
    }

    #[test]
    fn is_prime_larger_types() {
        assert!(is_prime(65_521_u16)); // the largest u16 prime
        assert!(is_prime(2_147_483_647_u32)); // 2^31 - 1
        assert!(is_prime(4_294_967_291_u32)); // the largest u32 prime
        assert!(!is_prime(u32::MAX)); // 3 * 5 * 17 * 257 * 65537
        assert!(is_prime(1_000_000_007_u64));
        assert!(!is_prime(1_000_000_007_u64 * 3));
        assert!(!is_prime(3_215_031_751_u64)); // a strong pseudoprime to bases 2, 3, 5, 7
        assert!(is_prime(1_000_003_usize));
    }

    #[test]
    fn is_prime_does_not_overflow_near_the_top_of_a_type() {
        // `i * i` would overflow here; `i <= n / i` must not.
        assert!(!is_prime(u16::MAX));
        assert!(is_prime(65_521_u16));
        assert!(!is_prime(u64::MAX));
        assert!(!is_prime(u8::MAX));
    }

    fn check_checked_wrappers<T: Integer>() {
        assert_eq!(checked_add(n::<T>(2), n::<T>(3)), Some(n::<T>(5)));
        assert_eq!(checked_sub(n::<T>(5), n::<T>(3)), Some(n::<T>(2)));
        assert_eq!(checked_mul(n::<T>(4), n::<T>(5)), Some(n::<T>(20)));
        assert_eq!(checked_div(n::<T>(9), n::<T>(2)), Some(n::<T>(4)));
        assert_eq!(checked_rem(n::<T>(9), n::<T>(2)), Some(n::<T>(1)));
        assert_eq!(checked_pow(n::<T>(3), 4), Some(n::<T>(81)));
        assert_eq!(checked_add(T::MAX, T::ONE), None);
        assert_eq!(checked_sub(T::MIN, T::ONE), None);
        assert_eq!(checked_mul(T::MAX, n::<T>(2)), None);
        assert_eq!(checked_div(T::ONE, T::ZERO), None);
        assert_eq!(checked_rem(T::ONE, T::ZERO), None);
        assert_eq!(checked_pow(T::MAX, 2), None);
    }

    fn check_saturating_and_wrapping_wrappers<T: Integer>() {
        assert_eq!(saturating_add(T::MAX, T::ONE), T::MAX);
        assert_eq!(saturating_sub(T::MIN, T::ONE), T::MIN);
        assert_eq!(saturating_mul(T::MAX, n::<T>(2)), T::MAX);
        assert_eq!(saturating_pow(T::MAX, 3), T::MAX);
        assert_eq!(saturating_add(n::<T>(2), n::<T>(3)), n::<T>(5));
        assert_eq!(wrapping_add(T::MAX, T::ONE), T::MIN);
        assert_eq!(wrapping_sub(T::MIN, T::ONE), T::MAX);
        assert_eq!(wrapping_mul(n::<T>(4), n::<T>(5)), n::<T>(20));
        assert_eq!(wrapping_pow(n::<T>(2), 3), n::<T>(8));
    }

    #[test]
    fn overflow_safe_wrappers_work_for_all_twelve_types() {
        each_int!(check_checked_wrappers);
        each_int!(check_saturating_and_wrapping_wrappers);
    }

    #[test]
    fn signed_boundary_cases() {
        assert_eq!(checked_div(i32::MIN, -1), None);
        assert_eq!(checked_rem(i32::MIN, -1), None);
        assert_eq!(saturating_sub(i32::MIN, 1), i32::MIN);
        assert_eq!(saturating_add(i8::MAX, 100), i8::MAX);
        assert_eq!(saturating_mul(i8::MIN, -1), i8::MAX);
        assert_eq!(wrapping_mul(i8::MIN, -1), i8::MIN);
        assert_eq!(wrapping_pow(2_u8, 8), 0);
        assert_eq!(saturating_pow(2_u8, 8), u8::MAX);
        assert_eq!(saturating_pow(-2_i8, 7), i8::MIN);
    }
}
