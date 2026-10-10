//! Integer math: [`gcd`], [`lcm`], [`is_prime`], [`isqrt`], logarithms
//! ([`ilog`], [`ilog2`], [`ilog10`]), rounding-up division ([`div_ceil`]),
//! [`abs_diff`], Euclidean division ([`div_euclid`], [`rem_euclid`]),
//! modular exponentiation ([`mod_pow`]) and overflow-safe arithmetic
//! wrappers ([`checked_add`], [`saturating_mul`], [`wrapping_pow`], ...).
//!
//! Everything is generic over [`Integer`], so one definition serves all 12
//! standard integer types. Nothing here panics: where an argument is out of
//! the function's domain (a zero divisor, a negative square root, a result
//! that does not fit) the function returns `None`.
//!
//! # Signed inputs and the `Unsigned` result
//!
//! [`gcd`], [`lcm`] and [`abs_diff`] accept signed and unsigned types alike
//! but return [`Integer::Unsigned`]: the result is a magnitude, and for a
//! signed type the exact answer can be `|MIN|`, which the signed type cannot
//! hold (`gcd(i32::MIN, 0) == 2_147_483_648`). For unsigned inputs
//! `Integer::Unsigned` is the input type itself, so nothing changes:
//!
//! ```
//! use math::int_math::{abs_diff, gcd, lcm};
//!
//! assert_eq!(gcd(12_u32, 18), 6);
//! assert_eq!(gcd(-12_i32, 18), 6_u32); // an `i32` in, a `u32` out
//! assert_eq!(gcd(i32::MIN, 0), 2_147_483_648_u32);
//! assert_eq!(lcm(4_u8, 6), Some(12));
//! assert_eq!(lcm(200_u8, 201), None); // does not fit in a `u8`
//! assert_eq!(abs_diff(i8::MIN, i8::MAX), 255_u8);
//! ```
//!
//! # Roots, logarithms, division and powers
//!
//! ```
//! use math::int_math::{div_ceil, ilog2, ilog10, isqrt, mod_pow};
//!
//! assert_eq!(isqrt(99_u32), 9);
//! assert_eq!(ilog2(1000_u32), Some(9));
//! assert_eq!(ilog10(1000_u32), Some(3));
//! assert_eq!(div_ceil(7_u32, 2), Some(4)); // 3.5 rounded up
//! assert_eq!(div_ceil(-7_i32, 2), Some(-3)); // -3.5 rounded up
//! assert_eq!(mod_pow(4_u64, 13, 497), Some(445)); // 4^13 mod 497
//! ```
//!
//! [`isqrt`] only takes unsigned types, like [`is_prime`] (the square root
//! of a negative number is not an integer); [`checked_isqrt`] accepts any
//! type and returns `None` for a negative input.
//!
//! # `const`
//!
//! The functions at this level are generic over a trait, and trait methods
//! cannot be `const` on stable Rust, so they cannot be `const fn` either.
//! For `const` contexts every function above also exists as a `const fn` on
//! each concrete type, in a module named after the type, written with the
//! same signature minus the generics:
//!
//! ```
//! use math::int_math;
//!
//! const G: u32 = int_math::u32::gcd(12, 18);
//! const ROOT: u32 = int_math::u32::isqrt(1_000_000);
//! const POW: Option<i64> = int_math::i64::mod_pow(-2, 5, 7);
//!
//! assert_eq!((G, ROOT, POW), (6, 1000, Some(3)));
//! ```
//!
//! Those modules (`i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`,
//! `u32`, `u64`, `u128`, `usize`) differ from the generic functions only
//! where the type forces it: for a signed type `isqrt` returns an `Option`
//! (so it matches [`checked_isqrt`]), and `is_prime` exists on the unsigned
//! modules only. The primitives' own `const` inherent methods
//! (`u32::checked_add`, `i32::unsigned_abs`, ...) cover the overflow-safe
//! wrappers.

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

/// The integer square root of `n`: the largest `r` with `r * r <= n`.
///
/// Only unsigned types are accepted (`T: Unsigned`) because a negative number
/// has no integer square root; see [`checked_isqrt`] for the version that
/// takes any integer type.
///
/// ```
/// use math::int_math::isqrt;
///
/// assert_eq!(isqrt(0_u8), 0);
/// assert_eq!(isqrt(15_u8), 3);
/// assert_eq!(isqrt(16_u8), 4);
/// assert_eq!(isqrt(u64::MAX), 4_294_967_295);
/// ```
#[inline]
pub fn isqrt<T: Unsigned>(n: T) -> T {
    // `checked_isqrt` is `Some` for every unsigned value; the fallback is
    // never taken.
    n.checked_isqrt().unwrap_or(T::ZERO)
}

/// The integer square root of `n`, or `None` if `n` is negative.
///
/// ```
/// use math::int_math::checked_isqrt;
///
/// assert_eq!(checked_isqrt(99_i32), Some(9));
/// assert_eq!(checked_isqrt(-4_i32), None);
/// assert_eq!(checked_isqrt(7_u8), Some(2));
/// ```
#[inline]
pub fn checked_isqrt<T: Integer>(n: T) -> Option<T> {
    n.checked_isqrt()
}

/// The logarithm of `n` in `base`, rounded down: the largest `k` with
/// `base^k <= n`. `None` if `n` is not positive or `base` is below `2`.
///
/// ```
/// use math::int_math::ilog;
///
/// assert_eq!(ilog(81_u32, 3), Some(4));
/// assert_eq!(ilog(80_u32, 3), Some(3));
/// assert_eq!(ilog(0_u32, 3), None);
/// assert_eq!(ilog(9_u32, 1), None);
/// ```
#[inline]
pub fn ilog<T: Integer>(n: T, base: T) -> Option<u32> {
    n.checked_ilog(base)
}

/// The base-2 logarithm of `n`, rounded down (the index of its highest set
/// bit). `None` if `n` is not positive.
#[inline]
pub fn ilog2<T: Integer>(n: T) -> Option<u32> {
    n.checked_ilog2()
}

/// The base-10 logarithm of `n`, rounded down (the number of decimal digits
/// minus one). `None` if `n` is not positive.
#[inline]
pub fn ilog10<T: Integer>(n: T) -> Option<u32> {
    n.checked_ilog10()
}

/// `a / b` rounded towards positive infinity, or `None` if `b` is zero or
/// the division overflows (`MIN / -1` for signed types).
///
/// For unsigned types this is the usual "how many `b`-sized chunks cover
/// `a`" (`div_ceil(10, 4) == Some(3)`); for signed types the exact quotient
/// is rounded up, so `div_ceil(-7, 2) == Some(-3)` and
/// `div_ceil(7, -2) == Some(-3)`.
#[inline]
pub fn div_ceil<T: Integer>(a: T, b: T) -> Option<T> {
    let q = a.checked_div(b)?;
    let r = a.checked_rem(b)?;
    // Integer division truncates towards zero, which already is the ceiling
    // when the exact quotient is negative (`r` and `b` differ in sign) or
    // exact (`r == 0`); otherwise it is one too small.
    if r != T::ZERO && (r > T::ZERO) == (b > T::ZERO) {
        q.checked_add(T::ONE)
    } else {
        Some(q)
    }
}

/// `|a - b|` as [`Integer::Unsigned`]. Never overflows, even for
/// `abs_diff(i8::MIN, i8::MAX) == 255_u8`.
#[inline]
pub fn abs_diff<T: Integer>(a: T, b: T) -> T::Unsigned {
    a.abs_diff(b)
}

/// Euclidean division of `a` by `b`: the quotient `q` for which
/// `a - q * b` is in `0..|b|`. `None` if `b` is zero or the division
/// overflows (`MIN / -1` for signed types).
///
/// Unlike `/`, the remainder is never negative, so `div_euclid(-7, 2)` is
/// `-4` (not `-3`), with [`rem_euclid`]`(-7, 2) == 1`.
///
/// ```
/// use math::int_math::{div_euclid, rem_euclid};
///
/// assert_eq!(div_euclid(-7_i32, 2), Some(-4));
/// assert_eq!(rem_euclid(-7_i32, 2), Some(1));
/// assert_eq!(div_euclid(7_i32, 0), None);
/// ```
#[inline]
pub fn div_euclid<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_div_euclid(b)
}

/// The Euclidean remainder of `a` divided by `b`, always in `0..|b|`.
/// `None` if `b` is zero or the division overflows (`MIN % -1` for signed
/// types). See [`div_euclid`].
#[inline]
pub fn rem_euclid<T: Integer>(a: T, b: T) -> Option<T> {
    a.checked_rem_euclid(b)
}

/// `a * b mod m` for `a, b` in `0..m`, without overflowing `T` even when
/// `a * b` itself does not fit.
fn mul_mod<T: Integer>(a: T, b: T, m: T) -> T {
    if let Some(product) = a.checked_mul(b) {
        return product % m;
    }
    // The product does not fit: add `a` once per set bit of `b` instead
    // (double-and-add), reducing after every step.
    let (mut a, mut b, mut result) = (a, b, T::ZERO);
    while b != T::ZERO {
        if (b & T::ONE) == T::ONE {
            result = add_mod(result, a, m);
        }
        a = add_mod(a, a, m);
        b = b >> 1_u32;
    }
    result
}

/// `x + y mod m` for `x, y` in `0..m`, without overflowing `T`.
fn add_mod<T: Integer>(x: T, y: T, m: T) -> T {
    // `m - y` is in `1..=m`, so it cannot overflow.
    if x >= m - y { x - (m - y) } else { x + y }
}

/// `base` to the power `exp`, modulo `modulus`: modular exponentiation by
/// squaring, in `O(log exp)` multiplications.
///
/// The result is in `0..modulus`. A negative `base` is reduced
/// Euclidean-style first (`mod_pow(-2, 3, 5) == Some(2)`, as `-8 mod 5 == 2`),
/// and `exp` is an [`Integer::Unsigned`] so that the full range of the
/// exponent is available. Intermediate products never overflow `T`, whatever
/// the modulus: `mod_pow` works on `u64` moduli above `2^32` and on `u128`
/// alike, which makes it a building block for primality tests and hashing.
///
/// Returns `None` if `modulus` is not positive. `mod_pow(x, 0, m)` is
/// `1 % m` (so `0` for `m == 1`).
///
/// ```
/// use math::int_math::mod_pow;
///
/// assert_eq!(mod_pow(4_u32, 13, 497), Some(445));
/// assert_eq!(mod_pow(7_u32, 0, 13), Some(1));
/// assert_eq!(mod_pow(5_u32, 3, 1), Some(0));
/// assert_eq!(mod_pow(5_u32, 3, 0), None);
/// // 2^61 - 1 is prime, so by Fermat's little theorem 3^(p-1) == 1 (mod p).
/// let p = (1_u64 << 61) - 1;
/// assert_eq!(mod_pow(3_u64, p - 1, p), Some(1));
/// ```
#[inline]
pub fn mod_pow<T: Integer>(base: T, exp: T::Unsigned, modulus: T) -> Option<T> {
    if modulus <= T::ZERO {
        return None;
    }
    let mut base = base.checked_rem_euclid(modulus)?;
    let mut result = T::ONE.checked_rem_euclid(modulus)?;
    let mut exp = exp;
    while exp != T::Unsigned::ZERO {
        if (exp & T::Unsigned::ONE) == T::Unsigned::ONE {
            result = mul_mod(result, base, modulus);
        }
        exp = exp >> 1_u32;
        if exp != T::Unsigned::ZERO {
            base = mul_mod(base, base, modulus);
        }
    }
    Some(result)
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

/// Generates the `const fn` versions of this module's functions for one
/// concrete integer type. Invoked once inside each of the 12 per-type
/// modules below, as `const_int_math!(signed i32 => u32)` or
/// `const_int_math!(unsigned u32)`.
macro_rules! const_int_math {
    (signed $t:ident => $u:ident) => {
        /// The absolute value as an unsigned number (exact for `MIN`).
        const fn magnitude(x: $t) -> $u {
            x.unsigned_abs()
        }

        /// Whether `x` is above zero.
        const fn is_positive(x: $t) -> bool {
            x > 0
        }

        /// `const fn` version of [`super::checked_isqrt`]: the integer
        /// square root of `n`, or `None` if `n` is negative.
        #[inline]
        pub const fn isqrt(n: $t) -> Option<$t> {
            n.checked_isqrt()
        }

        const_int_math!(@common $t => $u);
    };
    (unsigned $t:ident) => {
        /// The value itself: an unsigned number is its own magnitude.
        const fn magnitude(x: $t) -> $t {
            x
        }

        /// Whether `x` is above zero.
        const fn is_positive(x: $t) -> bool {
            x != 0
        }

        /// `const fn` version of [`super::isqrt`].
        #[inline]
        pub const fn isqrt(n: $t) -> $t {
            n.isqrt()
        }

        /// `const fn` version of [`super::is_prime`].
        #[inline]
        pub const fn is_prime(n: $t) -> bool {
            if n < 2 {
                return false;
            }
            if n < 4 {
                // 2 and 3.
                return true;
            }
            if n % 2 == 0 || n % 3 == 0 {
                return false;
            }
            // Every prime above 3 is 6k - 1 or 6k + 1. `i <= n / i` is
            // `i * i <= n` without the risk of `i * i` overflowing.
            let mut i: $t = 5;
            while i <= n / i {
                if n % i == 0 || n % (i + 2) == 0 {
                    return false;
                }
                i += 6;
            }
            true
        }

        const_int_math!(@common $t => $t);
    };
    (@common $t:ident => $u:ident) => {
        /// `const fn` version of [`super::gcd`].
        #[inline]
        pub const fn gcd(a: $t, b: $t) -> $u {
            let mut a = magnitude(a);
            let mut b = magnitude(b);
            if a == 0 {
                return b;
            }
            if b == 0 {
                return a;
            }
            // Factor out the powers of two both share, then work on odd
            // values.
            let common_twos = (a | b).trailing_zeros();
            a >>= a.trailing_zeros();
            loop {
                b >>= b.trailing_zeros();
                if a > b {
                    (a, b) = (b, a);
                }
                b -= a;
                if b == 0 {
                    return a << common_twos;
                }
            }
        }

        /// `const fn` version of [`super::lcm`].
        #[inline]
        pub const fn lcm(a: $t, b: $t) -> Option<$u> {
            let (ma, mb) = (magnitude(a), magnitude(b));
            if ma == 0 || mb == 0 {
                return Some(0);
            }
            (ma / gcd(a, b)).checked_mul(mb)
        }

        /// `const fn` version of [`super::abs_diff`].
        #[inline]
        pub const fn abs_diff(a: $t, b: $t) -> $u {
            a.abs_diff(b)
        }

        /// `const fn` version of [`super::div_ceil`].
        #[inline]
        pub const fn div_ceil(a: $t, b: $t) -> Option<$t> {
            let q = match a.checked_div(b) {
                Some(q) => q,
                None => return None,
            };
            // `b` is not zero and the division did not overflow, so neither
            // can the remainder.
            let r = a % b;
            if r != 0 && (is_positive(r) == is_positive(b)) {
                q.checked_add(1)
            } else {
                Some(q)
            }
        }

        /// `const fn` version of [`super::div_euclid`].
        #[inline]
        pub const fn div_euclid(a: $t, b: $t) -> Option<$t> {
            a.checked_div_euclid(b)
        }

        /// `const fn` version of [`super::rem_euclid`].
        #[inline]
        pub const fn rem_euclid(a: $t, b: $t) -> Option<$t> {
            a.checked_rem_euclid(b)
        }

        /// `const fn` version of [`super::ilog`].
        #[inline]
        pub const fn ilog(n: $t, base: $t) -> Option<u32> {
            n.checked_ilog(base)
        }

        /// `const fn` version of [`super::ilog2`].
        #[inline]
        pub const fn ilog2(n: $t) -> Option<u32> {
            n.checked_ilog2()
        }

        /// `const fn` version of [`super::ilog10`].
        #[inline]
        pub const fn ilog10(n: $t) -> Option<u32> {
            n.checked_ilog10()
        }

        /// `x + y mod m` for `x, y` in `0..m`, without overflowing.
        const fn add_mod(x: $t, y: $t, m: $t) -> $t {
            if x >= m - y { x - (m - y) } else { x + y }
        }

        /// `a * b mod m` for `a, b` in `0..m`, without overflowing.
        const fn mul_mod(a: $t, b: $t, m: $t) -> $t {
            if let Some(product) = a.checked_mul(b) {
                return product % m;
            }
            let (mut a, mut b, mut result) = (a, b, 0);
            while b != 0 {
                if b & 1 == 1 {
                    result = add_mod(result, a, m);
                }
                a = add_mod(a, a, m);
                b >>= 1;
            }
            result
        }

        /// `const fn` version of [`super::mod_pow`].
        #[inline]
        pub const fn mod_pow(base: $t, exp: $u, modulus: $t) -> Option<$t> {
            if !is_positive(modulus) {
                return None;
            }
            // `modulus` is positive, so the remainder cannot overflow.
            let mut base = base.rem_euclid(modulus);
            let mut result = 1 % modulus;
            let mut exp = exp;
            while exp != 0 {
                if exp & 1 == 1 {
                    result = mul_mod(result, base, modulus);
                }
                exp >>= 1;
                if exp != 0 {
                    base = mul_mod(base, base, modulus);
                }
            }
            Some(result)
        }
    };
}

/// `const fn` versions of this module's functions for `i8`.
pub mod i8 {
    const_int_math!(signed i8 => u8);
}

/// `const fn` versions of this module's functions for `i16`.
pub mod i16 {
    const_int_math!(signed i16 => u16);
}

/// `const fn` versions of this module's functions for `i32`.
pub mod i32 {
    const_int_math!(signed i32 => u32);
}

/// `const fn` versions of this module's functions for `i64`.
pub mod i64 {
    const_int_math!(signed i64 => u64);
}

/// `const fn` versions of this module's functions for `i128`.
pub mod i128 {
    const_int_math!(signed i128 => u128);
}

/// `const fn` versions of this module's functions for `isize`.
pub mod isize {
    const_int_math!(signed isize => usize);
}

/// `const fn` versions of this module's functions for `u8`.
pub mod u8 {
    const_int_math!(unsigned u8);
}

/// `const fn` versions of this module's functions for `u16`.
pub mod u16 {
    const_int_math!(unsigned u16);
}

/// `const fn` versions of this module's functions for `u32`.
pub mod u32 {
    const_int_math!(unsigned u32);
}

/// `const fn` versions of this module's functions for `u64`.
pub mod u64 {
    const_int_math!(unsigned u64);
}

/// `const fn` versions of this module's functions for `u128`.
pub mod u128 {
    const_int_math!(unsigned u128);
}

/// `const fn` versions of this module's functions for `usize`.
pub mod usize {
    const_int_math!(unsigned usize);
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

    // ---- Phase 2.6: isqrt, ilog, div_ceil, abs_diff, euclid, mod_pow ----

    /// `T::ZERO - n(value)`; only meaningful for signed `T`.
    fn negative<T: Integer>(value: u8) -> T {
        T::ZERO - n::<T>(value)
    }

    /// `value` as the exponent type of `T`.
    fn exp<T: Integer>(value: u8) -> T::Unsigned {
        T::Unsigned::from_u8(value).expect("small test value fits every integer type")
    }

    fn check_isqrt<T: Unsigned>() {
        assert_eq!(isqrt(T::ZERO), T::ZERO);
        assert_eq!(isqrt(T::ONE), T::ONE);
        assert_eq!(isqrt(n::<T>(3)), T::ONE);
        assert_eq!(isqrt(n::<T>(4)), n::<T>(2));
        assert_eq!(isqrt(n::<T>(15)), n::<T>(3));
        assert_eq!(isqrt(n::<T>(16)), n::<T>(4));
        // `r * r <= MAX < (r + 1)^2` at the top of the range.
        let r = isqrt(T::MAX);
        assert!(r.checked_mul(r).is_some());
        assert!((r + T::ONE).checked_mul(r + T::ONE).is_none());
    }

    #[test]
    fn isqrt_works_for_all_unsigned_types() {
        check_isqrt::<u8>();
        check_isqrt::<u16>();
        check_isqrt::<u32>();
        check_isqrt::<u64>();
        check_isqrt::<u128>();
        check_isqrt::<usize>();
        assert_eq!(isqrt(u8::MAX), 15);
        assert_eq!(isqrt(u16::MAX), 255);
        assert_eq!(isqrt(u32::MAX), 65_535);
        assert_eq!(isqrt(u64::MAX), 4_294_967_295);
        assert_eq!(isqrt(u128::MAX), u128::from(u64::MAX));
    }

    #[test]
    fn isqrt_is_exact_for_every_u16() {
        for v in 0..=u16::MAX {
            let r = u32::from(isqrt(v));
            let v = u32::from(v);
            assert!(r * r <= v && (r + 1) * (r + 1) > v, "{v}");
        }
    }

    fn check_checked_isqrt<T: Integer>() {
        assert_eq!(checked_isqrt(T::ZERO), Some(T::ZERO));
        assert_eq!(checked_isqrt(n::<T>(99)), Some(n::<T>(9)));
        assert_eq!(checked_isqrt(n::<T>(100)), Some(n::<T>(10)));
        let r = checked_isqrt(T::MAX).unwrap();
        assert!(r.checked_mul(r).is_some());
        assert!((r + T::ONE).checked_mul(r + T::ONE).is_none());
        if T::SIGNED {
            assert_eq!(checked_isqrt(T::MIN), None);
            assert_eq!(checked_isqrt(negative::<T>(1)), None);
        } else {
            assert_eq!(checked_isqrt(T::MIN), Some(T::ZERO));
        }
    }

    #[test]
    fn checked_isqrt_works_for_all_twelve_types() {
        each_int!(check_checked_isqrt);
        assert_eq!(checked_isqrt(i8::MAX), Some(11));
        assert_eq!(checked_isqrt(i32::MAX), Some(46_340));
    }

    fn check_ilog<T: Integer>() {
        assert_eq!(ilog2(T::ZERO), None);
        assert_eq!(ilog2(T::MIN), None);
        assert_eq!(ilog2(T::ONE), Some(0));
        assert_eq!(ilog2(n::<T>(8)), Some(3));
        assert_eq!(ilog2(n::<T>(9)), Some(3));
        assert_eq!(ilog2(T::MAX), Some(T::BITS - 1 - u32::from(T::SIGNED)));
        assert_eq!(ilog10(T::ZERO), None);
        assert_eq!(ilog10(n::<T>(1)), Some(0));
        assert_eq!(ilog10(n::<T>(9)), Some(0));
        assert_eq!(ilog10(n::<T>(10)), Some(1));
        assert_eq!(ilog10(n::<T>(99)), Some(1));
        assert_eq!(ilog10(n::<T>(100)), Some(2));
        assert_eq!(ilog(n::<T>(81), n::<T>(3)), Some(4));
        assert_eq!(ilog(n::<T>(80), n::<T>(3)), Some(3));
        assert_eq!(ilog(n::<T>(7), n::<T>(2)), Some(2));
        assert_eq!(ilog(T::ZERO, n::<T>(2)), None);
        assert_eq!(ilog(n::<T>(5), T::ONE), None);
        assert_eq!(ilog(n::<T>(5), T::ZERO), None);
        if T::SIGNED {
            assert_eq!(ilog2(negative::<T>(8)), None);
            assert_eq!(ilog10(negative::<T>(100)), None);
            assert_eq!(ilog(negative::<T>(81), n::<T>(3)), None);
        }
    }

    #[test]
    fn logarithms_work_for_all_twelve_types() {
        each_int!(check_ilog);
        assert_eq!(ilog10(u64::MAX), Some(19));
        assert_eq!(ilog10(u128::MAX), Some(38));
    }

    fn check_div_ceil<T: Integer>() {
        let two = n::<T>(2);
        assert_eq!(div_ceil(n::<T>(7), two), Some(n::<T>(4)));
        assert_eq!(div_ceil(n::<T>(6), two), Some(n::<T>(3)));
        assert_eq!(div_ceil(T::ZERO, n::<T>(5)), Some(T::ZERO));
        assert_eq!(div_ceil(T::ONE, n::<T>(5)), Some(T::ONE));
        assert_eq!(div_ceil(n::<T>(5), n::<T>(5)), Some(T::ONE));
        assert_eq!(div_ceil(T::ONE, T::ZERO), None);
        assert_eq!(div_ceil(T::MAX, T::MAX), Some(T::ONE));
        assert_eq!(div_ceil(T::MAX, T::ONE), Some(T::MAX));
        // `MAX` is odd, so halving it rounds up.
        assert_eq!(div_ceil(T::MAX, two), Some(T::MAX / two + T::ONE));
        if T::SIGNED {
            let minus = negative::<T>;
            assert_eq!(div_ceil(minus(7), two), Some(minus(3)));
            assert_eq!(div_ceil(n::<T>(7), minus(2)), Some(minus(3)));
            assert_eq!(div_ceil(minus(7), minus(2)), Some(n::<T>(4)));
            assert_eq!(div_ceil(minus(6), minus(2)), Some(n::<T>(3)));
            assert_eq!(div_ceil(minus(6), two), Some(minus(3)));
            assert_eq!(div_ceil(T::MIN, minus(1)), None);
            assert_eq!(div_ceil(T::MIN, T::ONE), Some(T::MIN));
            assert_eq!(div_ceil(T::MIN, two), Some(T::MIN / two));
        }
    }

    #[test]
    fn div_ceil_works_for_all_twelve_types() {
        each_int!(check_div_ceil);
    }

    /// Floor division on `i32`, the reference for the exhaustive 8-bit tests.
    fn floor_div(a: i32, b: i32) -> i32 {
        if b > 0 {
            a.div_euclid(b)
        } else {
            (-a).div_euclid(-b)
        }
    }

    #[test]
    fn div_ceil_is_exact_for_every_i8_pair() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                let expected = if b == 0 {
                    None
                } else {
                    i8::try_from(-floor_div(-i32::from(a), i32::from(b))).ok()
                };
                assert_eq!(div_ceil(a, b), expected, "{a} / {b}");
            }
        }
    }

    #[test]
    fn div_ceil_is_exact_for_every_u8_pair() {
        for a in 0..=u8::MAX {
            for b in 0..=u8::MAX {
                let expected = if b == 0 {
                    None
                } else {
                    Some(u32::from(a).div_ceil(u32::from(b)) as u8)
                };
                assert_eq!(div_ceil(a, b), expected, "{a} / {b}");
            }
        }
    }

    #[test]
    fn abs_diff_never_overflows() {
        assert_eq!(abs_diff(3_u32, 10), 7);
        assert_eq!(abs_diff(10_u32, 3), 7);
        assert_eq!(abs_diff(-3_i32, 4), 7_u32);
        assert_eq!(abs_diff(i8::MIN, i8::MAX), 255_u8);
        assert_eq!(abs_diff(i64::MAX, i64::MIN), u64::MAX);
        assert_eq!(abs_diff(u128::MAX, 0), u128::MAX);
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                let expected = (i32::from(a) - i32::from(b)).unsigned_abs();
                assert_eq!(u32::from(abs_diff(a, b)), expected, "{a} {b}");
            }
        }
    }

    #[test]
    fn euclidean_division_matches_the_definition_for_every_i8_pair() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                let (q, r) = (div_euclid(a, b), rem_euclid(a, b));
                if b == 0 || (a == i8::MIN && b == -1) {
                    assert_eq!((q, r), (None, None), "{a} {b}");
                    continue;
                }
                let (q, r) = (i32::from(q.unwrap()), i32::from(r.unwrap()));
                let (a, b) = (i32::from(a), i32::from(b));
                assert_eq!(a, q * b + r, "{a} {b}");
                assert!((0..b.abs()).contains(&r), "{a} {b}");
            }
        }
    }

    #[test]
    fn euclidean_division_known_values() {
        assert_eq!(div_euclid(-7_i32, 2), Some(-4));
        assert_eq!(rem_euclid(-7_i32, 2), Some(1));
        assert_eq!(div_euclid(7_i32, -2), Some(-3));
        assert_eq!(rem_euclid(7_i32, -2), Some(1));
        assert_eq!(div_euclid(7_u32, 2), Some(3));
        assert_eq!(rem_euclid(7_u32, 0), None);
        assert_eq!(div_euclid(i64::MIN, -1), None);
    }

    fn check_mod_pow<T: Integer>() {
        assert_eq!(mod_pow(n::<T>(3), exp::<T>(4), n::<T>(7)), Some(n::<T>(4)));
        assert_eq!(
            mod_pow(n::<T>(2), exp::<T>(10), n::<T>(100)),
            Some(n::<T>(24))
        );
        assert_eq!(mod_pow(n::<T>(5), exp::<T>(0), n::<T>(7)), Some(T::ONE));
        assert_eq!(mod_pow(T::ZERO, exp::<T>(0), n::<T>(7)), Some(T::ONE));
        assert_eq!(mod_pow(n::<T>(5), exp::<T>(3), T::ONE), Some(T::ZERO));
        assert_eq!(mod_pow(n::<T>(5), exp::<T>(3), T::ZERO), None);
        // `(MAX - 1)^2 == (-1)^2 == 1 (mod MAX)`; the square does not fit in
        // `T`, so this goes through the overflow-free multiplication.
        assert_eq!(mod_pow(T::MAX - T::ONE, exp::<T>(2), T::MAX), Some(T::ONE));
        assert_eq!(mod_pow(T::MAX, exp::<T>(5), T::MAX), Some(T::ZERO));
        if T::SIGNED {
            assert_eq!(mod_pow(n::<T>(3), exp::<T>(2), negative::<T>(7)), None);
            assert_eq!(mod_pow(T::ONE, exp::<T>(2), T::MIN), None);
            // -2^3 = -8 == 2 (mod 5).
            assert_eq!(
                mod_pow(negative::<T>(2), exp::<T>(3), n::<T>(5)),
                Some(n::<T>(2))
            );
            // (-1)^even == 1 and (-1)^odd == modulus - 1.
            assert_eq!(
                mod_pow(negative::<T>(1), exp::<T>(6), n::<T>(9)),
                Some(T::ONE)
            );
            assert_eq!(
                mod_pow(negative::<T>(1), exp::<T>(7), n::<T>(9)),
                Some(n::<T>(8))
            );
        }
    }

    #[test]
    fn mod_pow_works_for_all_twelve_types() {
        each_int!(check_mod_pow);
    }

    /// `base^exp mod modulus` by repeated multiplication on `u32`.
    fn naive_mod_pow(base: u32, exp: u32, modulus: u32) -> u32 {
        let mut result = 1 % modulus;
        for _ in 0..exp {
            result = result * (base % modulus) % modulus;
        }
        result
    }

    #[test]
    fn mod_pow_is_exact_for_every_u8_base_and_modulus() {
        for base in 0..=u8::MAX {
            for modulus in 1..=u8::MAX {
                for e in 0..=12_u8 {
                    let expected = naive_mod_pow(u32::from(base), u32::from(e), u32::from(modulus));
                    assert_eq!(
                        mod_pow(base, e, modulus).map(u32::from),
                        Some(expected),
                        "{base}^{e} mod {modulus}"
                    );
                }
            }
        }
    }

    #[test]
    fn mod_pow_reduces_negative_bases_for_every_i8_base() {
        for base in i8::MIN..=i8::MAX {
            for modulus in 1..=i8::MAX {
                for e in 0..=6_u8 {
                    let reduced = i32::from(base).rem_euclid(i32::from(modulus));
                    let expected = naive_mod_pow(reduced as u32, u32::from(e), modulus as u32);
                    assert_eq!(
                        mod_pow(base, e, modulus).map(|v| v as u32),
                        Some(expected),
                        "{base}^{e} mod {modulus}"
                    );
                }
            }
        }
    }

    #[test]
    fn mod_pow_handles_moduli_whose_squares_overflow() {
        // 2^61 - 1 and 2^127 - 1 are prime, so Fermat's little theorem gives
        // a^(p - 1) == 1 for any `a` that is not a multiple of p, and
        // 2^127 == 1 (mod 2^127 - 1).
        let p64 = (1_u64 << 61) - 1;
        let p128 = (1_u128 << 127) - 1;
        for a in [2_u64, 3, 5, 12_345, u64::from(u32::MAX)] {
            assert_eq!(mod_pow(a, p64 - 1, p64), Some(1), "{a}");
        }
        for a in [2_u128, 3, 5, 12_345, u128::from(u64::MAX)] {
            assert_eq!(mod_pow(a, p128 - 1, p128), Some(1), "{a}");
        }
        assert_eq!(mod_pow(2_u128, 127, p128), Some(1));
        assert_eq!(mod_pow(2_u128, 128, p128), Some(2));
        assert_eq!(
            mod_pow(7_i64, i64::MAX as u64 - 1, i64::MAX),
            mod_pow(7_u64, i64::MAX as u64 - 1, i64::MAX as u64).map(|v| v as i64)
        );
    }

    // ---- the concrete `const fn` modules agree with the generic functions

    /// A spread of interesting values of the type, negated copies included
    /// (a large unsigned value for unsigned types).
    macro_rules! samples {
        ($t:ident) => {{
            let base: [$t; 10] = [0, 1, 2, 3, 6, 7, 10, 100, <$t>::MAX - 1, <$t>::MAX];
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

    /// Asserts that `int_math::$t`'s functions equal the generic ones on
    /// every pair of `samples`.
    macro_rules! assert_concrete_matches_generic {
        ($t:ident) => {{
            let xs = samples!($t);
            for &a in &xs {
                assert_eq!($t::ilog2(a), ilog2(a));
                assert_eq!($t::ilog10(a), ilog10(a));
                for &b in &xs {
                    assert_eq!($t::gcd(a, b), gcd(a, b));
                    assert_eq!($t::lcm(a, b), lcm(a, b));
                    assert_eq!($t::abs_diff(a, b), abs_diff(a, b));
                    assert_eq!($t::div_ceil(a, b), div_ceil(a, b));
                    assert_eq!($t::div_euclid(a, b), div_euclid(a, b));
                    assert_eq!($t::rem_euclid(a, b), rem_euclid(a, b));
                    assert_eq!($t::ilog(a, b), ilog(a, b));
                    let e = Integer::unsigned_abs(a);
                    assert_eq!($t::mod_pow(a, e, b), mod_pow(a, e, b));
                }
            }
        }};
    }

    #[test]
    fn signed_concrete_modules_match_the_generic_functions() {
        macro_rules! check {
            ($($t:ident),*) => {$(
                assert_concrete_matches_generic!($t);
                for &a in &samples!($t) {
                    assert_eq!($t::isqrt(a), checked_isqrt(a));
                }
            )*};
        }
        check!(i8, i16, i32, i64, i128, isize);
    }

    #[test]
    fn unsigned_concrete_modules_match_the_generic_functions() {
        macro_rules! check {
            ($($t:ident),*) => {$(
                assert_concrete_matches_generic!($t);
                for &a in &samples!($t) {
                    assert_eq!($t::isqrt(a), isqrt(a));
                    // Trial division is `O(sqrt n)`; keep to small values.
                    if a < 200 {
                        assert_eq!($t::is_prime(a), is_prime(a));
                    }
                }
            )*};
        }
        check!(u8, u16, u32, u64, u128, usize);
    }

    #[test]
    fn concrete_modules_match_the_generic_functions_for_every_8_bit_pair() {
        for a in i8::MIN..=i8::MAX {
            assert_eq!(i8::isqrt(a), checked_isqrt(a));
            for b in i8::MIN..=i8::MAX {
                assert_eq!(i8::gcd(a, b), gcd(a, b));
                assert_eq!(i8::lcm(a, b), lcm(a, b));
                assert_eq!(i8::div_ceil(a, b), div_ceil(a, b));
                assert_eq!(i8::mod_pow(a, 5, b), mod_pow(a, 5, b));
            }
        }
        for a in 0..=u8::MAX {
            assert_eq!(u8::isqrt(a), isqrt(a));
            assert_eq!(u8::is_prime(a), is_prime(a));
            for b in 0..=u8::MAX {
                assert_eq!(u8::gcd(a, b), gcd(a, b));
                assert_eq!(u8::lcm(a, b), lcm(a, b));
                assert_eq!(u8::div_ceil(a, b), div_ceil(a, b));
                assert_eq!(u8::mod_pow(a, 5, b), mod_pow(a, 5, b));
            }
        }
    }

    #[test]
    fn is_prime_const_version_matches_for_every_u16() {
        for v in 0..=u16::MAX {
            assert_eq!(u16::is_prime(v), is_prime(v), "{v}");
        }
        assert!(u32::is_prime(4_294_967_291));
        assert!(!u32::is_prime(4_294_967_295));
    }

    #[test]
    fn concrete_modules_work_in_const_contexts() {
        const GCD: u32 = u32::gcd(12, 18);
        const GCD_SIGNED: u32 = i32::gcd(-12, 18);
        const LCM: Option<u8> = u8::lcm(200, 201);
        const ROOT: u64 = u64::isqrt(1_000_000_000_000);
        const SIGNED_ROOT: Option<i32> = i32::isqrt(-1);
        const LOG: Option<u32> = u32::ilog10(12_345);
        const CEIL: Option<i16> = i16::div_ceil(-7, 2);
        const POW: Option<u64> = u64::mod_pow(4, 13, 497);
        const PRIME: bool = u16::is_prime(65_521);
        const DIFF: u8 = i8::abs_diff(i8::MIN, i8::MAX);
        assert_eq!(GCD, 6);
        assert_eq!(GCD_SIGNED, 6);
        assert_eq!(LCM, None);
        assert_eq!(ROOT, 1_000_000);
        assert_eq!(SIGNED_ROOT, None);
        assert_eq!(LOG, Some(4));
        assert_eq!(CEIL, Some(-3));
        assert_eq!(POW, Some(445));
        assert!(core::hint::black_box(PRIME));
        assert_eq!(DIFF, 255);
    }
}
