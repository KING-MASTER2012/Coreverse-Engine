//! Ergonomic, non-generic constants for direct use (`math::consts::f32::PI`
//! etc.), organized the same way `core::f32::consts`/`core::f64::consts`
//! are. Every value here is defined in terms of [`crate::Scalar`]'s
//! associated consts, so this module can never drift from what code
//! written generically against [`crate::Scalar`] sees.

/// `f32` constants.
pub mod f32 {
    use crate::Scalar;

    /// π.
    pub const PI: f32 = <f32 as Scalar>::PI;
    /// 2π (a full turn in radians).
    pub const TAU: f32 = <f32 as Scalar>::TAU;
    /// Euler's number.
    pub const E: f32 = <f32 as Scalar>::E;
    /// Multiply a value in degrees by this to get radians.
    pub const DEG_TO_RAD: f32 = <f32 as Scalar>::DEG_TO_RAD;
    /// Multiply a value in radians by this to get degrees.
    pub const RAD_TO_DEG: f32 = <f32 as Scalar>::RAD_TO_DEG;
    /// The default tolerance [`crate::approx::approx_eq`] uses for `f32`
    /// (this type's machine epsilon).
    pub const DEFAULT_EPSILON: f32 = <f32 as Scalar>::DEFAULT_EPSILON;
}

/// `f64` constants.
pub mod f64 {
    use crate::Scalar;

    /// π.
    pub const PI: f64 = <f64 as Scalar>::PI;
    /// 2π (a full turn in radians).
    pub const TAU: f64 = <f64 as Scalar>::TAU;
    /// Euler's number.
    pub const E: f64 = <f64 as Scalar>::E;
    /// Multiply a value in degrees by this to get radians.
    pub const DEG_TO_RAD: f64 = <f64 as Scalar>::DEG_TO_RAD;
    /// Multiply a value in radians by this to get degrees.
    pub const RAD_TO_DEG: f64 = <f64 as Scalar>::RAD_TO_DEG;
    /// The default tolerance [`crate::approx::approx_eq`] uses for `f64`
    /// (this type's machine epsilon).
    pub const DEFAULT_EPSILON: f64 = <f64 as Scalar>::DEFAULT_EPSILON;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_core_consts() {
        assert_eq!(f32::PI, core::f32::consts::PI);
        assert_eq!(f64::PI, core::f64::consts::PI);
    }
}
