//! Strongly typed angles: [`Radians`] and [`Degrees`].
//!
//! Both are thin `#[repr(transparent)]` wrappers around a float, so mixing
//! the two units up is a compile error instead of a silent 57x bug. They
//! convert into each other with [`From`]/[`Into`]:
//!
//! ```
//! use math::angle::{Degrees, Radians};
//!
//! let quarter: Radians<f32> = Degrees(90.0).into();
//! assert!((quarter.0 - core::f32::consts::FRAC_PI_2).abs() < 1e-6);
//!
//! let back: Degrees<f32> = quarter.into();
//! assert!((back.0 - 90.0).abs() < 1e-4);
//! ```
//!
//! The wrapped value is public (`angle.0`), and an angle supports the
//! arithmetic that makes sense for one: adding or subtracting two angles of
//! the same unit, negating one, and scaling one by a plain number. Angles are
//! never normalized behind your back: `Degrees(450.0)` stays `450.0`, and
//! two angles compare equal only if their stored values do. Wrapping is
//! always an explicit call: [`Radians::normalized`] (into `[0, TAU)`),
//! [`Radians::normalized_signed`] (into `(-PI, PI]`), the shortest signed
//! turn between two angles with [`Radians::delta_angle`], and
//! [`Radians::lerp_angle`] to interpolate along that shortest turn (all
//! four exist on [`Degrees`] too).
//!
//! The trigonometric functions that consume and produce these types live in
//! [`crate::trig`].
//!
//! # `const`
//!
//! [`Radians::new`], [`Radians::value`] and their [`Degrees`] twins are
//! `const fn`, as are the `ZERO`/`PI`/`TAU` associated constants. The
//! [`From`] conversions cannot be (trait methods are not `const` on stable),
//! but in `const` code you can convert by hand with
//! [`Scalar::DEG_TO_RAD`]/[`Scalar::RAD_TO_DEG`], which are `const`s.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::Scalar;

/// Generates everything [`Radians`] and [`Degrees`] have in common:
/// the type itself, its constructor/accessor and its operators.
macro_rules! angle_type {
    ($(#[$meta:meta])* $name:ident, $unit:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name<T>(
            #[doc = concat!("The angle's value, in ", $unit, ".")]
            pub T,
        );

        impl<T> $name<T> {
            #[doc = concat!("Wraps a raw value that is already in ", $unit, ".")]
            #[inline]
            pub const fn new(value: T) -> Self {
                Self(value)
            }
        }

        impl<T: Copy> $name<T> {
            #[doc = concat!("The raw value, in ", $unit, ".")]
            #[inline]
            pub const fn value(self) -> T {
                self.0
            }
        }

        impl<T: Scalar> $name<T> {
            /// The zero angle.
            pub const ZERO: Self = Self(T::ZERO);
        }

        impl<T: Add<Output = T>> Add for $name<T> {
            type Output = Self;

            #[inline]
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl<T: Sub<Output = T>> Sub for $name<T> {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl<T: Neg<Output = T>> Neg for $name<T> {
            type Output = Self;

            #[inline]
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        /// Scales the angle by a plain number.
        impl<T: Mul<Output = T>> Mul<T> for $name<T> {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: T) -> Self {
                Self(self.0 * rhs)
            }
        }

        /// Divides the angle by a plain number.
        impl<T: Div<Output = T>> Div<T> for $name<T> {
            type Output = Self;

            #[inline]
            fn div(self, rhs: T) -> Self {
                Self(self.0 / rhs)
            }
        }

        impl<T: AddAssign> AddAssign for $name<T> {
            #[inline]
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl<T: SubAssign> SubAssign for $name<T> {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl<T: MulAssign> MulAssign<T> for $name<T> {
            #[inline]
            fn mul_assign(&mut self, rhs: T) {
                self.0 *= rhs;
            }
        }

        impl<T: DivAssign> DivAssign<T> for $name<T> {
            #[inline]
            fn div_assign(&mut self, rhs: T) {
                self.0 /= rhs;
            }
        }
    };
}

angle_type!(
    /// An angle in radians.
    ///
    /// The unit [`crate::trig`] works in. One full turn is
    /// [`Radians::TAU`].
    Radians,
    "radians"
);

angle_type!(
    /// An angle in degrees.
    ///
    /// One full turn is `360`. Convert to [`Radians`] with
    /// [`From`]/[`Into`] before calling anything that is not documented as
    /// taking degrees; [`crate::trig`]'s functions accept either.
    Degrees,
    "degrees"
);

impl<T: Scalar> Radians<T> {
    /// Half a turn, π radians.
    pub const PI: Self = Self(T::PI);

    /// A full turn, 2π radians.
    pub const TAU: Self = Self(T::TAU);

    /// This angle in degrees. Same as `Degrees::from(self)`.
    #[inline]
    pub fn to_degrees(self) -> Degrees<T> {
        self.into()
    }
}

impl<T: Scalar> Degrees<T> {
    /// This angle in radians. Same as `Radians::from(self)`.
    #[inline]
    pub fn to_radians(self) -> Radians<T> {
        self.into()
    }
}

/// Generates the wrapping/shortest-turn methods that [`Radians`] and
/// [`Degrees`] share. `$full` is a full turn and `$half` half a turn, in
/// the type's own unit (as expressions over the scalar type `T`).
macro_rules! angle_wrapping {
    ($name:ident, $full:expr, $half:expr) => {
        impl<T: Scalar> $name<T> {
            /// This angle wrapped into one full turn: `[0, TAU)` for
            /// radians, `[0, 360)` for degrees.
            ///
            /// Uses the Euclidean remainder, so negative angles wrap up
            /// (`-90°` becomes `270°`) and the result is never exactly a
            /// full turn. `NaN` and infinite angles give `NaN`.
            #[inline]
            pub fn normalized(self) -> Self {
                Self(crate::interp::repeat(self.0, $full))
            }

            /// This angle wrapped into `(-half turn, half turn]`, the
            /// signed form of the same direction: `(-PI, PI]` for radians,
            /// `(-180, 180]` for degrees.
            ///
            /// The range includes the half turn itself and excludes minus
            /// the half turn (`-180°` becomes `180°`), the same convention
            /// as `atan2`'s result range. `NaN` and infinite angles give
            /// `NaN`.
            #[inline]
            pub fn normalized_signed(self) -> Self {
                let wrapped = self.normalized().0;
                if wrapped > $half {
                    Self(wrapped - $full)
                } else {
                    Self(wrapped)
                }
            }

            /// The shortest signed turn from `self` to `target`, in
            /// `(-half turn, half turn]`: positive means counterclockwise
            /// (increasing angle). Works for any inputs, wrapped or not
            /// (`350°` to `10°` is `+20°`, not `-340°`).
            #[inline]
            pub fn delta_angle(self, target: Self) -> Self {
                Self(target.0 - self.0).normalized_signed()
            }

            /// Interpolates from `self` towards `target` along the
            /// shortest turn ([`Self::delta_angle`]): `t = 0` is `self`,
            /// `t = 1` is `target` (as an equivalent angle, not
            /// necessarily the same stored value). Does not clamp `t`, and
            /// the result is not normalized.
            #[inline]
            pub fn lerp_angle(self, target: Self, t: T) -> Self {
                Self(self.0 + self.delta_angle(target).0 * t)
            }
        }
    };
}

angle_wrapping!(Radians, T::TAU, T::PI);
angle_wrapping!(Degrees, T::from_f64(360.0), T::from_f64(180.0));

impl<T: Scalar> From<Degrees<T>> for Radians<T> {
    #[inline]
    fn from(degrees: Degrees<T>) -> Self {
        Self(degrees.0 * T::DEG_TO_RAD)
    }
}

impl<T: Scalar> From<Radians<T>> for Degrees<T> {
    #[inline]
    fn from(radians: Radians<T>) -> Self {
        Self(radians.0 * T::RAD_TO_DEG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approx::approx_eq_eps;

    const CONST_ANGLE: Radians<f32> = Radians::new(1.5);
    const CONST_VALUE: f32 = CONST_ANGLE.value();

    #[test]
    fn constructors_and_accessors() {
        assert_eq!(Radians::new(2.0_f32), Radians(2.0));
        assert_eq!(Degrees::new(90.0_f64).value(), 90.0);
        assert_eq!(Radians(1.25_f64).0, 1.25);
    }

    #[test]
    fn const_constructors_work_at_compile_time() {
        assert_eq!(CONST_VALUE, 1.5);
        assert_eq!(Radians::<f32>::PI.0, f32::PI);
        assert_eq!(Radians::<f64>::TAU.0, f64::TAU);
        assert_eq!(Degrees::<f32>::ZERO.0, 0.0);
    }

    #[test]
    fn default_is_zero() {
        assert_eq!(Radians::<f32>::default(), Radians::ZERO);
        assert_eq!(Degrees::<f64>::default(), Degrees::ZERO);
    }

    #[test]
    fn degrees_to_radians_known_values() {
        let cases = [
            (0.0_f64, 0.0),
            (90.0, core::f64::consts::FRAC_PI_2),
            (180.0, core::f64::consts::PI),
            (360.0, core::f64::consts::TAU),
            (-45.0, -core::f64::consts::FRAC_PI_4),
        ];
        for (deg, rad) in cases {
            let r: Radians<f64> = Degrees(deg).into();
            assert!(approx_eq_eps(r.0, rad, 1e-12), "{deg} degrees -> {}", r.0);
        }
    }

    #[test]
    fn radians_to_degrees_known_values() {
        let d: Degrees<f32> = Radians(core::f32::consts::PI).into();
        assert!(approx_eq_eps(d.0, 180.0, 1e-4));
        let d = Radians(core::f64::consts::FRAC_PI_3).to_degrees();
        assert!(approx_eq_eps(d.0, 60.0, 1e-12));
    }

    #[test]
    fn conversion_round_trips() {
        for deg in [-720.0_f64, -90.0, 0.0, 1.0, 33.3, 180.0, 1000.0] {
            let back: Degrees<f64> = Radians::from(Degrees(deg)).into();
            assert!(approx_eq_eps(back.0, deg, 1e-9), "{deg} -> {}", back.0);
        }
        let back = Degrees(72.5_f32).to_radians().to_degrees();
        assert!(approx_eq_eps(back.0, 72.5, 1e-4));
    }

    #[test]
    fn conversion_does_not_normalize() {
        let r: Radians<f64> = Degrees(450.0).into();
        assert!(approx_eq_eps(r.0, 2.5 * core::f64::consts::PI, 1e-12));
    }

    #[test]
    fn add_sub_neg() {
        assert_eq!(Degrees(30.0_f32) + Degrees(60.0), Degrees(90.0));
        assert_eq!(Degrees(30.0_f32) - Degrees(60.0), Degrees(-30.0));
        assert_eq!(-Radians(1.5_f64), Radians(-1.5));
    }

    #[test]
    fn scale_by_scalar() {
        assert_eq!(Degrees(45.0_f32) * 2.0, Degrees(90.0));
        assert_eq!(Radians(3.0_f64) / 2.0, Radians(1.5));
    }

    #[test]
    fn assign_operators() {
        let mut a = Degrees(10.0_f32);
        a += Degrees(5.0);
        assert_eq!(a, Degrees(15.0));
        a -= Degrees(3.0);
        assert_eq!(a, Degrees(12.0));
        a *= 2.0;
        assert_eq!(a, Degrees(24.0));
        a /= 4.0;
        assert_eq!(a, Degrees(6.0));
    }

    #[test]
    fn ordering_and_equality_use_the_stored_value() {
        assert!(Degrees(10.0_f32) < Degrees(20.0));
        assert!(Radians(2.0_f64) > Radians(1.0));
        // Not normalized: a full turn is not equal to zero.
        assert_ne!(Degrees(360.0_f32), Degrees(0.0));
    }

    #[test]
    fn zero_converts_to_zero() {
        let r: Radians<f32> = Degrees::ZERO.into();
        assert_eq!(r, Radians::ZERO);
    }

    // ---- wrapping (Phase 2.1)

    #[test]
    fn normalized_wraps_into_one_turn() {
        let cases = [
            (0.0_f64, 0.0),
            (90.0, 90.0),
            (359.0, 359.0),
            (360.0, 0.0),
            (450.0, 90.0),
            (-90.0, 270.0),
            (-360.0, 0.0),
            (-725.0, 355.0),
            (1080.5, 0.5),
        ];
        for (input, expected) in cases {
            let got = Degrees(input).normalized().0;
            assert!(approx_eq_eps(got, expected, 1e-9), "{input} -> {got}");
            assert!((0.0..360.0).contains(&got), "{input} -> {got}");
        }
    }

    #[test]
    fn normalized_radians_wraps_into_tau() {
        let r = Radians(3.0 * core::f64::consts::PI).normalized();
        assert!(approx_eq_eps(r.0, core::f64::consts::PI, 1e-12));
        let r = Radians(-core::f32::consts::FRAC_PI_2).normalized();
        assert!(approx_eq_eps(r.0, 3.0 * core::f32::consts::FRAC_PI_2, 1e-5));
        assert_eq!(Radians(0.5_f32).normalized(), Radians(0.5));
    }

    #[test]
    fn normalized_never_returns_a_full_turn() {
        for tiny in [-1.0e-10_f32, -1.0e-30] {
            let d = Degrees(tiny).normalized().0;
            assert!((0.0..360.0).contains(&d), "{tiny} -> {d}");
            let r = Radians(tiny).normalized().0;
            assert!((0.0..core::f32::consts::TAU).contains(&r), "{tiny} -> {r}");
        }
    }

    #[test]
    fn normalized_signed_range_is_open_closed() {
        let cases = [
            (0.0_f64, 0.0),
            (90.0, 90.0),
            (180.0, 180.0),
            (-180.0, 180.0),
            (181.0, -179.0),
            (-181.0, 179.0),
            (270.0, -90.0),
            (540.0, 180.0),
            (-725.0, -5.0),
        ];
        for (input, expected) in cases {
            let got = Degrees(input).normalized_signed().0;
            assert!(approx_eq_eps(got, expected, 1e-9), "{input} -> {got}");
            assert!(got > -180.0 && got <= 180.0, "{input} -> {got}");
        }
        let r = Radians(core::f64::consts::PI * 1.5).normalized_signed();
        assert!(approx_eq_eps(r.0, -core::f64::consts::FRAC_PI_2, 1e-12));
    }

    #[test]
    fn normalizing_non_finite_angles_gives_nan() {
        assert!(Degrees(f32::NAN).normalized().0.is_nan());
        assert!(Degrees(f32::INFINITY).normalized().0.is_nan());
        assert!(Radians(f64::NEG_INFINITY).normalized_signed().0.is_nan());
    }

    #[test]
    fn delta_angle_takes_the_short_way_round() {
        let d = |from: f64, to: f64| Degrees(from).delta_angle(Degrees(to)).0;
        assert!(approx_eq_eps(d(350.0, 10.0), 20.0, 1e-9));
        assert!(approx_eq_eps(d(10.0, 350.0), -20.0, 1e-9));
        assert!(approx_eq_eps(d(0.0, 90.0), 90.0, 1e-9));
        assert!(approx_eq_eps(d(90.0, 0.0), -90.0, 1e-9));
        // Unwrapped inputs.
        assert!(approx_eq_eps(d(720.0 + 10.0, -360.0 + 30.0), 20.0, 1e-9));
        // Opposite directions: the tie goes to +half turn.
        assert!(approx_eq_eps(d(0.0, 180.0), 180.0, 1e-9));
        assert!(approx_eq_eps(d(180.0, 0.0), 180.0, 1e-9));
        assert_eq!(d(42.0, 42.0), 0.0);
    }

    #[test]
    fn delta_angle_in_radians() {
        let from = Radians(0.1_f64);
        let to = Radians(core::f64::consts::TAU - 0.1);
        assert!(approx_eq_eps(from.delta_angle(to).0, -0.2, 1e-12));
    }

    #[test]
    fn lerp_angle_follows_the_shortest_arc() {
        let a = Degrees(350.0_f64);
        let b = Degrees(10.0);
        assert!(approx_eq_eps(a.lerp_angle(b, 0.0).0, 350.0, 1e-9));
        // Halfway across the 0 degree seam, not through 180.
        assert!(approx_eq_eps(
            a.lerp_angle(b, 0.5).0.rem_euclid(360.0),
            0.0,
            1e-9
        ));
        assert!(approx_eq_eps(
            a.lerp_angle(b, 1.0).normalized().0,
            10.0,
            1e-9
        ));
        // Extrapolates, unclamped.
        assert!(approx_eq_eps(
            a.lerp_angle(b, 2.0).normalized().0,
            30.0,
            1e-9
        ));
        // Radians twin.
        let r = Radians(0.0_f32).lerp_angle(Radians(core::f32::consts::FRAC_PI_2), 0.5);
        assert!(approx_eq_eps(r.0, core::f32::consts::FRAC_PI_4, 1e-6));
    }
}
