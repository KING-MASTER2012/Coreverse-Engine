//! Euler angles for the quaternions (Phase 2.4): [`EulerRot`], and
//! `from_euler`/`to_euler` on [`Quat`] and [`DQuat`].
//!
//! An Euler rotation is three rotations about coordinate axes, one after the
//! other. Two things have to be fixed to make the three angles mean anything:
//! the *order* of the axes and whether each axis belongs to the *rotating*
//! frame (**intrinsic**: every rotation is about the axes as left by the
//! previous one) or to the *fixed* world frame (**extrinsic**).
//!
//! # Conventions
//!
//! - **Intrinsic `ABC` with angles `(a, b, c)`** is `R_A(a) * R_B(b) * R_C(c)`
//!   as a product of rotations: `c` is applied to a vector first, `a` last,
//!   exactly like `Quat * Quat` and `Mat3 * Mat3` everywhere else in this
//!   crate. `EulerRot::XYZ` is therefore `Quat::from_rotation_x(a) *
//!   Quat::from_rotation_y(b) * Quat::from_rotation_z(c)`.
//! - **Extrinsic `ABCEx` with angles `(a, b, c)`** rotates about the fixed `A`
//!   axis by `a` first, then about the fixed `B` by `b`, then about `C` by
//!   `c`. It is the intrinsic order `CBA` with the angles reversed:
//!   `from_euler(XYZEx, a, b, c) == from_euler(ZYX, c, b, a)`.
//! - Angles are [`Radians`] or [`Degrees`](crate::Degrees), like everywhere
//!   else; `to_euler` returns [`Radians`].
//! - **12 axis orders**: the six *Tait-Bryan* orders with three different axes
//!   (`XYZ`, `XZY`, `YXZ`, `YZX`, `ZXY`, `ZYX`, whose middle angle is limited
//!   to `[-PI/2, PI/2]`) and the six *proper Euler* orders that repeat the
//!   first axis (`XYX`, `XZX`, `YXY`, `YZY`, `ZXZ`, `ZYZ`, whose middle angle
//!   is limited to `[0, PI]`). Each exists in an intrinsic and an extrinsic
//!   form, 24 variants in all.
//! - **Gimbal lock.** When the middle angle is at the end of its range the
//!   first and the last rotation turn about the same axis and only their sum
//!   (or difference) matters. `to_euler` then returns `0` for the first
//!   angle and puts everything into the last one. The rotation round-trips
//!   either way.
//!
//! `to_euler` follows Bernardes and Viollet, "Quaternion to Euler angles
//! conversion: A direct, general and computationally efficient method"
//! (2022): one algorithm for every order, working on the quaternion
//! directly without going through a matrix.

use super::{DQuat, Quat};
use crate::angle::Radians;
use crate::scalar::Scalar;

/// The order of the three axis rotations of an Euler rotation, intrinsic or
/// extrinsic, for [`Quat::from_euler`] and [`Quat::to_euler`].
///
/// - **Intrinsic** (`XYZ`, ...): every rotation is about the axes of the
///   frame as the previous one left it. The angles `(a, b, c)` of `XYZ` give
///   `from_rotation_x(a) * from_rotation_y(b) * from_rotation_z(c)`, so
///   `c` is applied to a vector first.
/// - **Extrinsic** (`XYZEx`, ...): every rotation is about the fixed world
///   axes, the first angle first. `XYZEx` with `(a, b, c)` is the same
///   rotation as `ZYX` with `(c, b, a)`.
/// - **Tait-Bryan orders** (three different axes: `XYZ`, `XZY`, `YXZ`, `YZX`,
///   `ZXY`, `ZYX`) have a middle angle limited to `[-PI/2, PI/2]`;
///   **proper Euler orders** (`XYX`, `XZX`, `YXY`, `YZY`, `ZXZ`, `ZYZ`)
///   limit it to `[0, PI]`. A common choice for cameras and characters is
///   `YXZ`: yaw, then pitch, then roll.
#[allow(
    clippy::upper_case_acronyms,
    reason = "axis orders are conventionally written XYZ, ZYX, ..."
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EulerRot {
    /// Intrinsic `X`, `Y`, `Z`.
    XYZ,
    /// Intrinsic `X`, `Z`, `Y`.
    XZY,
    /// Intrinsic `Y`, `X`, `Z`.
    YXZ,
    /// Intrinsic `Y`, `Z`, `X`.
    YZX,
    /// Intrinsic `Z`, `X`, `Y`.
    ZXY,
    /// Intrinsic `Z`, `Y`, `X`.
    ZYX,
    /// Intrinsic proper Euler order `X`, `Y`, `X`.
    XYX,
    /// Intrinsic proper Euler order `X`, `Z`, `X`.
    XZX,
    /// Intrinsic proper Euler order `Y`, `X`, `Y`.
    YXY,
    /// Intrinsic proper Euler order `Y`, `Z`, `Y`.
    YZY,
    /// Intrinsic proper Euler order `Z`, `X`, `Z`.
    ZXZ,
    /// Intrinsic proper Euler order `Z`, `Y`, `Z`.
    ZYZ,
    /// Extrinsic `X`, `Y`, `Z` (about the fixed axes).
    XYZEx,
    /// Extrinsic `X`, `Z`, `Y`.
    XZYEx,
    /// Extrinsic `Y`, `X`, `Z`.
    YXZEx,
    /// Extrinsic `Y`, `Z`, `X`.
    YZXEx,
    /// Extrinsic `Z`, `X`, `Y`.
    ZXYEx,
    /// Extrinsic `Z`, `Y`, `X`.
    ZYXEx,
    /// Extrinsic proper Euler order `X`, `Y`, `X`.
    XYXEx,
    /// Extrinsic proper Euler order `X`, `Z`, `X`.
    XZXEx,
    /// Extrinsic proper Euler order `Y`, `X`, `Y`.
    YXYEx,
    /// Extrinsic proper Euler order `Y`, `Z`, `Y`.
    YZYEx,
    /// Extrinsic proper Euler order `Z`, `X`, `Z`.
    ZXZEx,
    /// Extrinsic proper Euler order `Z`, `Y`, `Z`.
    ZYZEx,
}

impl EulerRot {
    /// The three axes (`0` = X, `1` = Y, `2` = Z) in the order the angles
    /// are given, and whether the order is extrinsic.
    const fn decode(self) -> ([usize; 3], bool) {
        match self {
            Self::XYZ => ([0, 1, 2], false),
            Self::XZY => ([0, 2, 1], false),
            Self::YXZ => ([1, 0, 2], false),
            Self::YZX => ([1, 2, 0], false),
            Self::ZXY => ([2, 0, 1], false),
            Self::ZYX => ([2, 1, 0], false),
            Self::XYX => ([0, 1, 0], false),
            Self::XZX => ([0, 2, 0], false),
            Self::YXY => ([1, 0, 1], false),
            Self::YZY => ([1, 2, 1], false),
            Self::ZXZ => ([2, 0, 2], false),
            Self::ZYZ => ([2, 1, 2], false),
            Self::XYZEx => ([0, 1, 2], true),
            Self::XZYEx => ([0, 2, 1], true),
            Self::YXZEx => ([1, 0, 2], true),
            Self::YZXEx => ([1, 2, 0], true),
            Self::ZXYEx => ([2, 0, 1], true),
            Self::ZYXEx => ([2, 1, 0], true),
            Self::XYXEx => ([0, 1, 0], true),
            Self::XZXEx => ([0, 2, 0], true),
            Self::YXYEx => ([1, 0, 1], true),
            Self::YZYEx => ([1, 2, 1], true),
            Self::ZXZEx => ([2, 0, 2], true),
            Self::ZYZEx => ([2, 1, 2], true),
        }
    }
}

/// Defines the Euler conversions for one quaternion type.
macro_rules! euler_impl {
    ($name:ident, $scalar:ident, $gimbal:expr) => {
        impl $name {
            /// The rotation about the axis with index `axis` (`0` = X).
            #[inline]
            fn axis_rotation(axis: usize, angle: Radians<$scalar>) -> Self {
                match axis {
                    0 => Self::from_rotation_x(angle),
                    1 => Self::from_rotation_y(angle),
                    _ => Self::from_rotation_z(angle),
                }
            }

            /// The rotation of three turns about coordinate axes in the given
            /// `order`, with the angles `a`, `b`, `c` in that order. Each can
            /// be `Radians` or `Degrees`. See [`EulerRot`] for the meaning of
            /// the 24 orders.
            ///
            /// ```
            /// use math::{Degrees, EulerRot, Quat, Vec3};
            ///
            /// // Yaw 90 degrees about Y, then pitch 0, then roll 0.
            /// let q = Quat::from_euler(
            ///     EulerRot::YXZ,
            ///     Degrees::new(90.0),
            ///     Degrees::new(0.0),
            ///     Degrees::new(0.0),
            /// );
            /// assert!((q * Vec3::X - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-6);
            /// ```
            #[inline]
            pub fn from_euler(
                order: EulerRot,
                a: impl Into<Radians<$scalar>>,
                b: impl Into<Radians<$scalar>>,
                c: impl Into<Radians<$scalar>>,
            ) -> Self {
                let ([i, j, k], extrinsic) = order.decode();
                let (a, b, c) = (a.into(), b.into(), c.into());
                if extrinsic {
                    Self::axis_rotation(k, c)
                        * Self::axis_rotation(j, b)
                        * Self::axis_rotation(i, a)
                } else {
                    Self::axis_rotation(i, a)
                        * Self::axis_rotation(j, b)
                        * Self::axis_rotation(k, c)
                }
            }

            /// The three angles `(a, b, c)` of this rotation in the given
            /// `order`, the inverse of [`Self::from_euler`]: every angle is
            /// in `(-PI, PI]`, except that the middle one stays inside
            /// `[-PI/2, PI/2]` (Tait-Bryan orders) or `[0, PI]` (proper Euler
            /// orders), which makes the result unique.
            ///
            /// At gimbal lock (the middle angle at the end of its range) the
            /// first and the last angle are not separately defined; the
            /// first is then `0`. The quaternion should be normalized; only
            /// its direction matters, so a slightly non-unit one still gives
            /// the right angles.
            #[inline]
            pub fn to_euler(
                self,
                order: EulerRot,
            ) -> (Radians<$scalar>, Radians<$scalar>, Radians<$scalar>) {
                let ([i, j, k], extrinsic) = order.decode();
                if extrinsic {
                    let [t1, t2, t3] = self.intrinsic_angles(k, j, i);
                    (Radians::new(t3), Radians::new(t2), Radians::new(t1))
                } else {
                    let [t1, t2, t3] = self.intrinsic_angles(i, j, k);
                    (Radians::new(t1), Radians::new(t2), Radians::new(t3))
                }
            }

            /// The angles of the intrinsic rotation about the axes `i`, `j`,
            /// `k`: the algorithm of the module documentation.
            fn intrinsic_angles(self, i: usize, j: usize, k: usize) -> [$scalar; 3] {
                const PI: $scalar = <$scalar as Scalar>::PI;
                let vector = [self.x, self.y, self.z];
                let symmetric = i == k;
                let k = if symmetric { 3 - i - j } else { k };
                // -1 for the cyclic orders (XYZ, YZX, ZXY), +1 for the others:
                // the sign of the permutation, negated to match this crate's
                // quaternion product (checked by the round-trip tests).
                let sign: $scalar =
                    if (i, j, k) == (0, 1, 2) || (i, j, k) == (1, 2, 0) || (i, j, k) == (2, 0, 1) {
                        -1.0
                    } else {
                        1.0
                    };
                let (a, b, c, d) = if symmetric {
                    (self.w, vector[i], vector[j], vector[k] * sign)
                } else {
                    (
                        self.w - vector[j],
                        vector[i] + vector[k] * sign,
                        vector[j] + self.w,
                        vector[k] * sign - vector[i],
                    )
                };
                let theta2 =
                    2.0 * Scalar::atan2(Scalar::sqrt(c * c + d * d), Scalar::sqrt(a * a + b * b));
                let theta_plus = Scalar::atan2(b, a);
                let theta_minus = Scalar::atan2(d, c);
                let (theta1, mut theta3) = if Scalar::abs(theta2) < $gimbal {
                    (0.0, 2.0 * theta_plus)
                } else if Scalar::abs(theta2 - PI) < $gimbal {
                    (0.0, 2.0 * theta_minus)
                } else {
                    (theta_plus - theta_minus, theta_plus + theta_minus)
                };
                let mut theta2 = theta2;
                if !symmetric {
                    theta3 *= sign;
                    theta2 -= PI * 0.5;
                }
                let wrap = |angle: $scalar| Radians::new(angle).normalized_signed().value();
                [wrap(theta1), theta2, wrap(theta3)]
            }
        }
    };
}

euler_impl!(Quat, f32, 1e-6);
euler_impl!(DQuat, f64, 1e-12);
