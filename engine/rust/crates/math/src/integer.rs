//! The internal [`Integer`] trait: the common surface `math`'s generic
//! integer functions ([`crate::int_math`], [`crate::bits`],
//! [`crate::radix`]) are written against, implemented **once** for all 12
//! standard integer types (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`).
//! It is the integer counterpart of [`crate::Scalar`].
//!
//! Like `Scalar`, the trait is a thin layer: nearly every method just
//! forwards to the inherent method of the same name on the primitive. Two
//! extras make the generic code possible without unsafe casts:
//!
//! - [`Integer::Unsigned`], the unsigned type of the same width (itself for
//!   the unsigned types). Anything whose exact result can be as large as
//!   `|MIN|` (a GCD, an LCM, a digit string's magnitude) is computed and
//!   returned in it, so `gcd(i32::MIN, 0)` is `2_147_483_648u32` instead of
//!   an overflow.
//! - [`Integer::from_u8`]/[`Integer::to_u8`], checked conversions to and
//!   from small values, which stand in for the integer literals generic
//!   code cannot write.
//!
//! Nothing here allocates or needs `std`.
//!
//! Not meant to be implemented outside this crate.

use core::fmt::Debug;
use core::hash::Hash;
use core::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub};

/// A standard integer type (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`)
/// that `math`'s generic integer functions are written against.
///
/// The arithmetic operators here behave exactly as they do on the
/// primitives, including overflow panics in debug builds. The
/// `checked_*`/`saturating_*`/`wrapping_*` methods (and the free-function
/// wrappers in [`crate::int_math`]) are the overflow-safe way to compute.
pub trait Integer:
Copy
+ Eq
+ Ord
+ Hash
+ Debug
+ Add<Output = Self>
+ Sub<Output = Self>
+ Mul<Output = Self>
+ Div<Output = Self>
+ Rem<Output = Self>
+ BitAnd<Output = Self>
+ BitOr<Output = Self>
+ BitXor<Output = Self>
+ Not<Output = Self>
+ Shl<u32, Output = Self>
+ Shr<u32, Output = Self>
{
    /// The unsigned integer type with the same width as `Self`. For an
    /// unsigned type this is `Self`.
    type Unsigned: Unsigned;

    /// Additive identity, `0`.
    const ZERO: Self;
    /// Multiplicative identity, `1`.
    const ONE: Self;
    /// The smallest value of this type.
    const MIN: Self;
    /// The largest value of this type.
    const MAX: Self;
    /// The size of this type in bits.
    const BITS: u32;
    /// Whether this type is signed.
    const SIGNED: bool;

    /// Whether `self` is below zero. Always `false` for unsigned types.
    fn is_negative(self) -> bool;

    /// The absolute value as [`Integer::Unsigned`]. Unlike `abs`, this is
    /// exact for `MIN` (`i8::MIN.unsigned_abs() == 128u8`), so it never
    /// overflows.
    fn unsigned_abs(self) -> Self::Unsigned;

    /// The number of `1` bits.
    fn count_ones(self) -> u32;

    /// The number of leading `0` bits (`BITS` for zero).
    fn leading_zeros(self) -> u32;

    /// The number of trailing `0` bits (`BITS` for zero).
    fn trailing_zeros(self) -> u32;

    /// `self + rhs`, or `None` on overflow.
    fn checked_add(self, rhs: Self) -> Option<Self>;

    /// `self - rhs`, or `None` on overflow.
    fn checked_sub(self, rhs: Self) -> Option<Self>;

    /// `self * rhs`, or `None` on overflow.
    fn checked_mul(self, rhs: Self) -> Option<Self>;

    /// `self / rhs`, or `None` if `rhs` is zero or the division overflows
    /// (`MIN / -1` for signed types).
    fn checked_div(self, rhs: Self) -> Option<Self>;

    /// `self % rhs`, or `None` if `rhs` is zero or the division overflows
    /// (`MIN % -1` for signed types).
    fn checked_rem(self, rhs: Self) -> Option<Self>;

    /// `self` to the power `exp`, or `None` on overflow.
    fn checked_pow(self, exp: u32) -> Option<Self>;

    /// `self + rhs`, clamped to `[MIN, MAX]` instead of overflowing.
    fn saturating_add(self, rhs: Self) -> Self;

    /// `self - rhs`, clamped to `[MIN, MAX]` instead of overflowing.
    fn saturating_sub(self, rhs: Self) -> Self;

    /// `self * rhs`, clamped to `[MIN, MAX]` instead of overflowing.
    fn saturating_mul(self, rhs: Self) -> Self;

    /// `self` to the power `exp`, clamped to `[MIN, MAX]`.
    fn saturating_pow(self, exp: u32) -> Self;

    /// `self + rhs`, wrapping around at the type's boundary.
    fn wrapping_add(self, rhs: Self) -> Self;

    /// `self - rhs`, wrapping around at the type's boundary.
    fn wrapping_sub(self, rhs: Self) -> Self;

    /// `self * rhs`, wrapping around at the type's boundary.
    fn wrapping_mul(self, rhs: Self) -> Self;

    /// `self` to the power `exp`, wrapping around at the type's boundary.
    fn wrapping_pow(self, exp: u32) -> Self;

    /// `value` as this type, or `None` if it does not fit (`200` does not
    /// fit in an `i8`). Stands in for an integer literal in generic code.
    fn from_u8(value: u8) -> Option<Self>;

    /// `self` as a `u8`, or `None` if it is negative or above `255`.
    fn to_u8(self) -> Option<u8>;
}

/// An unsigned [`Integer`] (`u8`..`u128`, `usize`). Its
/// [`Integer::Unsigned`] is always itself.
///
/// Functions that only make sense for non-negative numbers, such as
/// [`crate::int_math::is_prime`], take `T: Unsigned`, so passing a signed
/// type is a compile error rather than a runtime surprise.
pub trait Unsigned: Integer<Unsigned = Self> {}

/// Everything the signed and unsigned impls share, expanded inside each
/// `impl Integer for` block.
macro_rules! integer_items {
    ($t:ty) => {
        const ZERO: Self = 0;
        const ONE: Self = 1;
        const MIN: Self = <$t>::MIN;
        const MAX: Self = <$t>::MAX;
        const BITS: u32 = <$t>::BITS;

        #[inline]
        fn count_ones(self) -> u32 {
            <$t>::count_ones(self)
        }

        #[inline]
        fn leading_zeros(self) -> u32 {
            <$t>::leading_zeros(self)
        }

        #[inline]
        fn trailing_zeros(self) -> u32 {
            <$t>::trailing_zeros(self)
        }

        #[inline]
        fn checked_add(self, rhs: Self) -> Option<Self> {
            <$t>::checked_add(self, rhs)
        }

        #[inline]
        fn checked_sub(self, rhs: Self) -> Option<Self> {
            <$t>::checked_sub(self, rhs)
        }

        #[inline]
        fn checked_mul(self, rhs: Self) -> Option<Self> {
            <$t>::checked_mul(self, rhs)
        }

        #[inline]
        fn checked_div(self, rhs: Self) -> Option<Self> {
            <$t>::checked_div(self, rhs)
        }

        #[inline]
        fn checked_rem(self, rhs: Self) -> Option<Self> {
            <$t>::checked_rem(self, rhs)
        }

        #[inline]
        fn checked_pow(self, exp: u32) -> Option<Self> {
            <$t>::checked_pow(self, exp)
        }

        #[inline]
        fn saturating_add(self, rhs: Self) -> Self {
            <$t>::saturating_add(self, rhs)
        }

        #[inline]
        fn saturating_sub(self, rhs: Self) -> Self {
            <$t>::saturating_sub(self, rhs)
        }

        #[inline]
        fn saturating_mul(self, rhs: Self) -> Self {
            <$t>::saturating_mul(self, rhs)
        }

        #[inline]
        fn saturating_pow(self, exp: u32) -> Self {
            <$t>::saturating_pow(self, exp)
        }

        #[inline]
        fn wrapping_add(self, rhs: Self) -> Self {
            <$t>::wrapping_add(self, rhs)
        }

        #[inline]
        fn wrapping_sub(self, rhs: Self) -> Self {
            <$t>::wrapping_sub(self, rhs)
        }

        #[inline]
        fn wrapping_mul(self, rhs: Self) -> Self {
            <$t>::wrapping_mul(self, rhs)
        }

        #[inline]
        fn wrapping_pow(self, exp: u32) -> Self {
            <$t>::wrapping_pow(self, exp)
        }

        #[inline]
        fn from_u8(value: u8) -> Option<Self> {
            <$t>::try_from(value).ok()
        }

        #[inline]
        fn to_u8(self) -> Option<u8> {
            u8::try_from(self).ok()
        }
    };
}

macro_rules! impl_signed {
    ($($t:ty => $u:ty),* $(,)?) => {$(
        impl Integer for $t {
            type Unsigned = $u;
            const SIGNED: bool = true;

            #[inline]
            fn is_negative(self) -> bool {
                self < 0
            }

            #[inline]
            fn unsigned_abs(self) -> $u {
                <$t>::unsigned_abs(self)
            }

            integer_items!($t);
        }
    )*};
}

macro_rules! impl_unsigned {
    ($($t:ty),* $(,)?) => {$(
        impl Integer for $t {
            type Unsigned = $t;
            const SIGNED: bool = false;

            #[inline]
            fn is_negative(self) -> bool {
                false
            }

            #[inline]
            fn unsigned_abs(self) -> $t {
                self
            }

            integer_items!($t);
        }

        impl Unsigned for $t {}
    )*};
}

impl_signed! {
    i8 => u8,
    i16 => u16,
    i32 => u32,
    i64 => u64,
    i128 => u128,
    isize => usize,
}

impl_unsigned! { u8, u16, u32, u64, u128, usize }

#[cfg(test)]
mod tests {
    use super::*;

    /// A small value as `T`, for tests that must run on all 12 types.
    fn n<T: Integer>(value: u8) -> T {
        T::from_u8(value).expect("small test value fits every integer type")
    }

    #[test]
    fn constants_match_the_primitives() {
        assert_eq!(<i8 as Integer>::BITS, 8);
        assert_eq!(<u16 as Integer>::BITS, 16);
        assert_eq!(<i32 as Integer>::BITS, 32);
        assert_eq!(<u64 as Integer>::BITS, 64);
        assert_eq!(<i128 as Integer>::BITS, 128);
        assert_eq!(<usize as Integer>::BITS, usize::BITS);
        assert_eq!(<isize as Integer>::MIN, isize::MIN);
        assert_eq!(<u8 as Integer>::MAX, u8::MAX);
        assert_eq!(<i64 as Integer>::MIN, i64::MIN);
    }

    fn check_identities<T: Integer>() {
        assert_eq!(T::ZERO, n::<T>(0));
        assert_eq!(T::ONE, n::<T>(1));
        assert_eq!(T::SIGNED, T::MIN != T::ZERO);
        assert!(!T::ZERO.is_negative());
        assert_eq!(T::MIN.is_negative(), T::SIGNED);
        assert!(!T::MAX.is_negative());
        // BITS agrees with the number of bits in MAX / !ZERO.
        assert_eq!((!T::ZERO).count_ones(), T::BITS);
        assert_eq!(T::ZERO.leading_zeros(), T::BITS);
        assert_eq!(T::ZERO.trailing_zeros(), T::BITS);
    }

    #[test]
    fn identities_hold_for_all_twelve_types() {
        each_int!(check_identities);
    }

    fn check_unsigned_abs<T: Integer>() {
        // `MIN` is exactly representable in the unsigned counterpart.
        let mag = T::MIN.unsigned_abs();
        if T::SIGNED {
            assert_eq!(mag.count_ones(), 1);
            assert_eq!(mag.leading_zeros(), 0);
        } else {
            assert_eq!(mag.count_ones(), 0);
        }
        assert_eq!(n::<T>(7).unsigned_abs().to_u8(), Some(7));
        if T::SIGNED {
            assert_eq!((T::ZERO - n::<T>(7)).unsigned_abs().to_u8(), Some(7));
        }
    }

    #[test]
    fn unsigned_abs_is_exact_for_min() {
        each_int!(check_unsigned_abs);
        assert_eq!(Integer::unsigned_abs(i8::MIN), 128u8);
        assert_eq!(Integer::unsigned_abs(i32::MIN), 2_147_483_648u32);
    }

    #[test]
    fn from_u8_and_to_u8_are_checked() {
        assert_eq!(<i8 as Integer>::from_u8(127), Some(127));
        assert_eq!(<i8 as Integer>::from_u8(128), None);
        assert_eq!(<u8 as Integer>::from_u8(255), Some(255));
        assert_eq!(<i16 as Integer>::from_u8(255), Some(255));
        assert_eq!(Integer::to_u8(255_u32), Some(255));
        assert_eq!(Integer::to_u8(256_u32), None);
        assert_eq!(Integer::to_u8(-1_i32), None);
        assert_eq!(Integer::to_u8(0_i128), Some(0));
    }

    fn check_forwarders<T: Integer>() {
        let (one, two, three) = (n::<T>(1), n::<T>(2), n::<T>(3));
        assert_eq!(one.checked_add(two), Some(three));
        assert_eq!(three.checked_sub(one), Some(two));
        assert_eq!(two.checked_mul(three).and_then(|x| x.to_u8()), Some(6));
        assert_eq!(n::<T>(7).checked_div(two), Some(three));
        assert_eq!(n::<T>(7).checked_rem(two), Some(one));
        assert_eq!(two.checked_pow(3).and_then(|x| x.to_u8()), Some(8));
        assert_eq!(T::MAX.checked_add(one), None);
        assert_eq!(T::MIN.checked_sub(one), None);
        assert_eq!(T::MAX.checked_mul(two), None);
        assert_eq!(one.checked_div(T::ZERO), None);
        assert_eq!(one.checked_rem(T::ZERO), None);
        assert_eq!(T::MAX.checked_pow(2), None);
        assert_eq!(T::MAX.saturating_add(one), T::MAX);
        assert_eq!(T::MIN.saturating_sub(one), T::MIN);
        assert_eq!(T::MAX.saturating_mul(two), T::MAX);
        assert_eq!(T::MAX.saturating_pow(2), T::MAX);
        assert_eq!(T::MAX.wrapping_add(one), T::MIN);
        assert_eq!(T::MIN.wrapping_sub(one), T::MAX);
        assert_eq!(two.wrapping_pow(2).to_u8(), Some(4));
        // 2 * MAX wraps to -2 for signed types and MAX - 1 for unsigned ones.
        let wrapped = if T::SIGNED {
            T::ZERO - two
        } else {
            T::MAX - one
        };
        assert_eq!(T::MAX.wrapping_mul(two), wrapped);
    }

    #[test]
    fn forwarders_behave_like_the_primitives() {
        each_int!(check_forwarders);
    }

    #[test]
    fn signed_division_overflow_is_none() {
        assert_eq!(Integer::checked_div(i8::MIN, -1), None);
        assert_eq!(Integer::checked_rem(i32::MIN, -1), None);
    }

    fn check_unsigned_is_self<T: Integer>() {
        // Round-trips through `Unsigned` for every non-negative small value.
        for v in [0_u8, 1, 5, 100] {
            let x = n::<T>(v);
            assert_eq!(x.unsigned_abs().to_u8(), Some(v));
        }
    }

    #[test]
    fn unsigned_counterpart_round_trips() {
        each_int!(check_unsigned_is_self);
    }
}
