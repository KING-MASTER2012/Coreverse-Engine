//! Approximate floating-point equality: exact `==` on `f32`/`f64` is
//! almost never what you want after any arithmetic, so comparisons
//! elsewhere in `math` - and in code built on top of it - should go
//! through one of these instead.

use crate::Scalar;

/// Whether `a` and `b` are within [`Scalar::DEFAULT_EPSILON`] of each
/// other (absolute difference). The right default for values that
/// haven't accumulated more than a couple of operations' worth of
/// rounding error; reach for [`approx_eq_eps`] or [`approx_eq_rel`] once
/// they have.
pub fn approx_eq<T: Scalar>(a: T, b: T) -> bool {
    approx_eq_eps(a, b, T::DEFAULT_EPSILON)
}

/// Whether `a` and `b` are within `epsilon` of each other (absolute
/// difference). Use this over [`approx_eq`] when the default tolerance is
/// too strict or too loose for how `a`/`b` were computed.
pub fn approx_eq_eps<T: Scalar>(a: T, b: T, epsilon: T) -> bool {
    (a - b).abs() <= epsilon
}

/// Whether `a` and `b` are within `rel_epsilon` of each other, scaled by
/// their own magnitude, rather than a fixed absolute tolerance.
///
/// Prefer this over [`approx_eq_eps`] when comparing values that could be
/// very large or very small: an absolute epsilon that's sensible near
/// `1.0` is either far too strict for large values or far too loose for
/// tiny ones. `rel_epsilon` is a fraction of the larger operand's
/// magnitude, e.g. `0.001` accepts a 0.1% difference.
///
/// Checks exact equality (`a == b`) first, so `a == b == 0.0` and
/// `a == b == ±inf` compare equal without dividing by zero or producing
/// `NaN`. Note that an infinite `a`/`b` against any nonzero finite value
/// also compares equal under this formula (`largest` becomes infinite,
/// so the scaled tolerance does too) - this function is meant for
/// comparing finite magnitudes, not for detecting overflow.
pub fn approx_eq_rel<T: Scalar>(a: T, b: T, rel_epsilon: T) -> bool {
    if a == b {
        return true;
    }
    let diff = (a - b).abs();
    let a_abs = a.abs();
    let b_abs = b.abs();
    let largest = if a_abs > b_abs { a_abs } else { b_abs };
    diff <= largest * rel_epsilon
}

/// Whether `a` is within [`Scalar::DEFAULT_EPSILON`] of zero.
pub fn approx_zero<T: Scalar>(a: T) -> bool {
    approx_zero_eps(a, T::DEFAULT_EPSILON)
}

/// Whether `a` is within `epsilon` of zero.
pub fn approx_zero_eps<T: Scalar>(a: T, epsilon: T) -> bool {
    a.abs() <= epsilon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_default_epsilon() {
        assert!(approx_eq(1.0_f32, 1.0_f32));
        assert!(approx_eq(1.0_f32, 1.0_f32 + f32::EPSILON));
        assert!(!approx_eq(1.0_f32, 1.1_f32));

        assert!(approx_eq(1.0_f64, 1.0_f64));
        assert!(!approx_eq(1.0_f64, 1.1_f64));
    }

    #[test]
    fn custom_epsilon() {
        assert!(approx_eq_eps(1.0_f32, 1.05_f32, 0.1));
        assert!(!approx_eq_eps(1.0_f32, 1.2_f32, 0.1));
    }

    #[test]
    fn relative_epsilon_handles_scale() {
        // 1% of 1_000_000.0 is 10_000.0, so this large a gap is still "close".
        assert!(approx_eq_rel(1_000_000.0_f32, 1_009_000.0_f32, 0.01));
        // The same absolute gap (9_000.0) is nowhere near 1% of 1.0.
        assert!(!approx_eq_rel(1.0_f32, 9_001.0_f32, 0.01));
    }

    #[test]
    fn relative_epsilon_handles_zero_and_same_infinity() {
        assert!(approx_eq_rel(0.0_f32, 0.0_f32, 0.01));
        assert!(approx_eq_rel(f32::INFINITY, f32::INFINITY, 0.01));
        assert!(approx_eq_rel(f32::NEG_INFINITY, f32::NEG_INFINITY, 0.01));
    }

    #[test]
    fn zero_tests() {
        assert!(approx_zero(0.0_f32));
        assert!(approx_zero_eps(0.0009_f32, 0.01));
        assert!(!approx_zero_eps(0.02_f32, 0.01));
    }
}
