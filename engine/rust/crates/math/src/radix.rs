//! Base 2 to 10 conversion: [`to_radix`] turns an integer into its digit
//! string in a chosen base and [`from_radix`] parses one back.
//!
//! The crate never allocates, so [`to_radix`] returns a [`RadixString`], a
//! small fixed-size buffer that derefs to `&str`, instead of a `String`.
//! Converting a number between two bases is a parse followed by a format:
//!
//! ```
//! use math::radix::{from_radix, to_radix};
//!
//! let value: u32 = from_radix("1101", 2).unwrap();
//! assert_eq!(value, 13);
//! assert_eq!(to_radix(value, 8).unwrap(), "15");
//! assert_eq!(to_radix(-13_i32, 2).unwrap(), "-1101");
//! ```
//!
//! Every one of the 12 standard integer types works, including their
//! extremes: `i128::MIN` in base 2 is 128 digits plus the sign, the longest
//! string this module can produce ([`RadixString::CAPACITY`] bytes).
//!
//! Only bases 2 to 10 are supported, so digits are always `0`-`9` and there
//! is no case or letter handling to get wrong.

use core::fmt;
use core::ops::Deref;

use crate::integer::Integer;

/// Why a radix conversion failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadixError {
    /// The base is not in `2..=10`.
    InvalidBase,
    /// The text to parse was empty.
    Empty,
    /// The text has a character that is not a digit of the base (or a sign
    /// with nothing after it, or a `-` for an unsigned type).
    InvalidDigit,
    /// The number does not fit in the target type.
    Overflow,
}

impl fmt::Display for RadixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBase => "radix must be between 2 and 10",
            Self::Empty => "cannot parse an integer from an empty string",
            Self::InvalidDigit => "invalid digit for the radix",
            Self::Overflow => "number does not fit in the target integer type",
        })
    }
}

impl core::error::Error for RadixError {}

/// A number rendered in some base: an optional `-` followed by digits.
///
/// A fixed-size, stack-allocated buffer (this crate never allocates).
/// Use it as a `&str` through [`RadixString::as_str`] or `Deref`, or print
/// it with `{}`.
#[derive(Clone, Copy)]
pub struct RadixString {
    buf: [u8; Self::CAPACITY],
    start: usize,
}

impl RadixString {
    /// The buffer size: enough for the longest possible result, `i128::MIN`
    /// in base 2 (128 digits and a `-`).
    pub const CAPACITY: usize = 129;

    /// The rendered number.
    #[inline]
    pub fn as_str(&self) -> &str {
        // The buffer only ever holds ASCII digits and `-`, so this cannot
        // fail; the fallback only exists to keep the function panic-free.
        core::str::from_utf8(&self.buf[self.start..]).unwrap_or("")
    }

    /// The number of characters, including any `-`.
    #[inline]
    pub fn len(&self) -> usize {
        Self::CAPACITY - self.start
    }

    /// Always `false`: even zero renders as `"0"`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl Deref for RadixString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for RadixString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RadixString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

impl fmt::Debug for RadixString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl PartialEq for RadixString {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for RadixString {}

impl PartialEq<str> for RadixString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for RadixString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// Checks `base` is in `2..=10` and returns it as a `u8`.
#[inline]
fn check_base(base: u32) -> Result<u8, RadixError> {
    match u8::try_from(base) {
        Ok(b) if (2..=10).contains(&b) => Ok(b),
        _ => Err(RadixError::InvalidBase),
    }
}

/// `value` written in `base` (`2..=10`), with a leading `-` if negative.
///
/// Works for every integer type and never overflows, not even for `MIN`
/// (`to_radix(i8::MIN, 2)` is `"-10000000"`). Fails only with
/// [`RadixError::InvalidBase`].
pub fn to_radix<T: Integer>(value: T, base: u32) -> Result<RadixString, RadixError> {
    let base = check_base(base)?;
    let base = <T::Unsigned as Integer>::from_u8(base).ok_or(RadixError::InvalidBase)?;

    // Work on the magnitude in the unsigned type so `MIN` cannot overflow,
    // filling the buffer from the back.
    let mut magnitude = value.unsigned_abs();
    let mut buf = [0_u8; RadixString::CAPACITY];
    let mut pos = RadixString::CAPACITY;
    loop {
        let digit = (magnitude % base).to_u8().unwrap_or(0);
        magnitude = magnitude / base;
        pos -= 1;
        buf[pos] = b'0' + digit;
        if magnitude == <T::Unsigned as Integer>::ZERO {
            break;
        }
    }
    if value.is_negative() {
        pos -= 1;
        buf[pos] = b'-';
    }
    Ok(RadixString { buf, start: pos })
}

/// Parses `text` as a number in `base` (`2..=10`).
///
/// Accepts an optional leading `+`, or `-` for signed types, then at least
/// one digit. Nothing else is allowed: no whitespace, no `_` separators, no
/// `0b`-style prefixes. Negative numbers are accumulated downwards, so
/// `i8::MIN` (`"-10000000"` in base 2) parses without overflowing.
///
/// # Errors
///
/// [`RadixError::InvalidBase`] for a base outside `2..=10`,
/// [`RadixError::Empty`] for `""`, [`RadixError::InvalidDigit`] for any
/// other malformed text and [`RadixError::Overflow`] if the number does not
/// fit in `T`.
pub fn from_radix<T: Integer>(text: &str, base: u32) -> Result<T, RadixError> {
    let base = check_base(base)?;
    let base_value = T::from_u8(base).ok_or(RadixError::InvalidBase)?;

    let bytes = text.as_bytes();
    let Some((&first, rest)) = bytes.split_first() else {
        return Err(RadixError::Empty);
    };
    let (negative, digits) = match first {
        b'-' => (true, rest),
        b'+' => (false, rest),
        _ => (false, bytes),
    };
    if digits.is_empty() || (negative && !T::SIGNED) {
        return Err(RadixError::InvalidDigit);
    }

    let mut value = T::ZERO;
    for &byte in digits {
        // Anything below `b'0'` wraps to a large number and fails the check.
        let digit = byte.wrapping_sub(b'0');
        if digit >= base {
            return Err(RadixError::InvalidDigit);
        }
        let digit = T::from_u8(digit).ok_or(RadixError::InvalidDigit)?;
        let shifted = value.checked_mul(base_value).ok_or(RadixError::Overflow)?;
        value = if negative {
            shifted.checked_sub(digit)
        } else {
            shifted.checked_add(digit)
        }
            .ok_or(RadixError::Overflow)?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed-size `fmt::Write` sink, since tests cannot rely on `String`
    /// in the `no_std` build.
    struct Sink {
        buf: [u8; 200],
        len: usize,
    }

    impl Sink {
        fn new() -> Self {
            Self {
                buf: [0; 200],
                len: 0,
            }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).unwrap()
        }
    }

    impl fmt::Write for Sink {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let end = self.len + s.len();
            self.buf[self.len..end].copy_from_slice(s.as_bytes());
            self.len = end;
            Ok(())
        }
    }

    fn n<T: Integer>(value: u8) -> T {
        T::from_u8(value).expect("small test value fits every integer type")
    }

    #[test]
    fn to_radix_known_values() {
        assert_eq!(to_radix(10_u32, 2).unwrap(), "1010");
        assert_eq!(to_radix(255_u32, 2).unwrap(), "11111111");
        assert_eq!(to_radix(8_u32, 8).unwrap(), "10");
        assert_eq!(to_radix(255_u32, 8).unwrap(), "377");
        assert_eq!(to_radix(255_u32, 10).unwrap(), "255");
        assert_eq!(to_radix(100_u8, 3).unwrap(), "10201");
        assert_eq!(to_radix(1234_u16, 5).unwrap(), "14414");
        assert_eq!(to_radix(63_usize, 7).unwrap(), "120");
        assert_eq!(to_radix(2_u8, 9).unwrap(), "2");
    }

    #[test]
    fn zero_is_a_single_digit() {
        for base in 2..=10 {
            assert_eq!(to_radix(0_u8, base).unwrap(), "0");
            assert_eq!(to_radix(0_i64, base).unwrap(), "0");
        }
    }

    #[test]
    fn negative_numbers_get_a_sign() {
        assert_eq!(to_radix(-10_i32, 2).unwrap(), "-1010");
        assert_eq!(to_radix(-255_i32, 10).unwrap(), "-255");
        assert_eq!(to_radix(-1_i8, 2).unwrap(), "-1");
        assert_eq!(to_radix(-64_i16, 8).unwrap(), "-100");
    }

    #[test]
    fn extremes_do_not_overflow() {
        assert_eq!(to_radix(i8::MIN, 2).unwrap(), "-10000000");
        assert_eq!(to_radix(i8::MIN, 10).unwrap(), "-128");
        assert_eq!(to_radix(u8::MAX, 2).unwrap(), "11111111");
        assert_eq!(to_radix(i32::MIN, 10).unwrap(), "-2147483648");
        assert_eq!(to_radix(u64::MAX, 10).unwrap(), "18446744073709551615");
        assert_eq!(
            to_radix(i128::MIN, 10).unwrap(),
            "-170141183460469231731687303715884105728"
        );
        assert_eq!(
            to_radix(u128::MAX, 10).unwrap(),
            "340282366920938463463374607431768211455"
        );
    }

    #[test]
    fn longest_possible_result_fits_the_buffer() {
        let longest = to_radix(i128::MIN, 2).unwrap();
        assert_eq!(longest.len(), RadixString::CAPACITY);
        assert!(longest.starts_with("-1000"));
        assert_eq!(to_radix(u128::MAX, 2).unwrap().len(), 128);
        assert_eq!(to_radix(i128::MAX, 2).unwrap().len(), 127);
    }

    #[test]
    fn invalid_bases_are_rejected() {
        for base in [0, 1, 11, 16, 36, u32::MAX] {
            assert_eq!(
                to_radix(5_u32, base),
                Err(RadixError::InvalidBase),
                "{base}"
            );
            assert_eq!(from_radix::<u32>("5", base), Err(RadixError::InvalidBase));
        }
    }

    #[test]
    fn radix_string_behaves_like_a_str() {
        let s = to_radix(-13_i32, 2).unwrap();
        assert_eq!(s.as_str(), "-1101");
        assert_eq!(&*s, "-1101");
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
        assert!(s.starts_with('-'));
        assert_eq!(s, to_radix(-13_i64, 2).unwrap());
        assert_ne!(s, to_radix(13_i32, 2).unwrap());
    }

    #[test]
    fn display_and_debug() {
        use core::fmt::Write;
        let s = to_radix(-13_i32, 2).unwrap();
        let mut out = Sink::new();
        write!(out, "{s}|{s:?}|{s:>8}|").unwrap();
        assert_eq!(out.as_str(), "-1101|\"-1101\"|   -1101|");
    }

    #[test]
    fn error_display_messages() {
        use core::fmt::Write;
        let mut out = Sink::new();
        write!(out, "{}", RadixError::Overflow).unwrap();
        assert_eq!(
            out.as_str(),
            "number does not fit in the target integer type"
        );
    }

    #[test]
    fn from_radix_known_values() {
        assert_eq!(from_radix::<u32>("1010", 2), Ok(10));
        assert_eq!(from_radix::<u32>("377", 8), Ok(255));
        assert_eq!(from_radix::<u32>("255", 10), Ok(255));
        assert_eq!(from_radix::<u8>("10201", 3), Ok(100));
        assert_eq!(from_radix::<i32>("-1010", 2), Ok(-10));
        assert_eq!(from_radix::<i32>("+1010", 2), Ok(10));
        assert_eq!(from_radix::<u64>("0007", 10), Ok(7));
        assert_eq!(from_radix::<i16>("-0", 10), Ok(0));
    }

    #[test]
    fn from_radix_extremes() {
        assert_eq!(from_radix::<i8>("-128", 10), Ok(i8::MIN));
        assert_eq!(from_radix::<i8>("-10000000", 2), Ok(i8::MIN));
        assert_eq!(from_radix::<i8>("127", 10), Ok(i8::MAX));
        assert_eq!(from_radix::<u8>("255", 10), Ok(u8::MAX));
        assert_eq!(from_radix::<i64>("-9223372036854775808", 10), Ok(i64::MIN));
        assert_eq!(
            from_radix::<u128>("340282366920938463463374607431768211455", 10),
            Ok(u128::MAX)
        );
    }

    #[test]
    fn from_radix_errors() {
        assert_eq!(from_radix::<u32>("", 10), Err(RadixError::Empty));
        assert_eq!(from_radix::<i32>("-", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<i32>("+", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>("12", 2), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>("1 2", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>(" 12", 10), Err(RadixError::InvalidDigit));
        assert_eq!(
            from_radix::<u32>("1_000", 10),
            Err(RadixError::InvalidDigit)
        );
        assert_eq!(from_radix::<u32>("0b101", 2), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>("ff", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>("/", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u32>(":", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u8>("-1", 10), Err(RadixError::InvalidDigit));
        assert_eq!(from_radix::<u8>("-0", 10), Err(RadixError::InvalidDigit));
        assert_eq!(
            from_radix::<u8>("\u{0661}\u{0662}", 10),
            Err(RadixError::InvalidDigit)
        );
    }

    #[test]
    fn from_radix_overflow() {
        assert_eq!(from_radix::<u8>("256", 10), Err(RadixError::Overflow));
        assert_eq!(from_radix::<i8>("128", 10), Err(RadixError::Overflow));
        assert_eq!(from_radix::<i8>("-129", 10), Err(RadixError::Overflow));
        assert_eq!(from_radix::<u8>("100000000", 2), Err(RadixError::Overflow));
        assert_eq!(
            from_radix::<u64>("18446744073709551616", 10),
            Err(RadixError::Overflow)
        );
        // Leading zeros are not overflow.
        assert_eq!(from_radix::<u8>("000000000000255", 10), Ok(255));
    }

    fn check_round_trip<T: Integer>() {
        let mut samples = [T::ZERO; 8];
        samples[1] = T::ONE;
        samples[2] = n::<T>(7);
        samples[3] = n::<T>(100);
        samples[4] = T::MAX;
        samples[5] = T::MIN;
        samples[6] = T::MAX - n::<T>(1);
        samples[7] = T::MIN + n::<T>(1);
        for base in 2..=10 {
            for &value in &samples {
                let text = to_radix(value, base).unwrap();
                assert_eq!(
                    from_radix::<T>(&text, base),
                    Ok(value),
                    "{text} base {base}"
                );
            }
        }
    }

    #[test]
    fn round_trips_for_all_twelve_types_and_bases() {
        each_int!(check_round_trip);
    }

    #[test]
    fn round_trips_every_i8_and_u8_in_every_base() {
        for base in 2..=10 {
            for v in i8::MIN..=i8::MAX {
                assert_eq!(from_radix::<i8>(&to_radix(v, base).unwrap(), base), Ok(v));
            }
            for v in 0..=u8::MAX {
                assert_eq!(from_radix::<u8>(&to_radix(v, base).unwrap(), base), Ok(v));
            }
        }
    }

    #[test]
    fn converting_between_bases() {
        let value: u16 = from_radix("101101", 2).unwrap();
        assert_eq!(value, 45);
        assert_eq!(to_radix(value, 5).unwrap(), "140");
        assert_eq!(to_radix(value, 10).unwrap(), "45");
    }

    #[test]
    fn matches_a_reference_digit_count() {
        // Base-2 length is the bit length, so it can be checked independently.
        for v in [1_u32, 2, 3, 255, 256, 65_535, 65_536, u32::MAX] {
            let expected = usize::try_from(32 - v.leading_zeros()).unwrap();
            assert_eq!(to_radix(v, 2).unwrap().len(), expected, "{v}");
        }
    }
}
