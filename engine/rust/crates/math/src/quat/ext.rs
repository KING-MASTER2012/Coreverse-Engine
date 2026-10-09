//! The rest of the quaternion API of Phase 2.4: rotation vectors, shortest
//! arcs, look rotations, angles between rotations, `exp`/`ln`/`pow`,
//! swing-twist decomposition, and the general helpers (`is_finite`,
//! `approx_eq`, `Display`, conversion between `Quat` and `DQuat`).
//!
//! The Euler conversions live in [`euler`](super::euler). Everything here is
//! written once inside a `macro_rules!` and instantiated for both float
//! families, like the rest of the quaternion code.

use super::{DQuat, Quat};
use crate::angle::Radians;
use crate::matrix::{DMat3, Mat3};
use crate::scalar::Scalar;
use crate::vector::{DVec3, Vec3};

macro_rules! quaternion_extras {
    (
        $name:ident, $scalar:ident, $vec3:ident, $mat3:ident,
        $other:ident, $other_scalar:ident, $as_other:ident,
        $normalized_tolerance:expr
    ) => {
        impl $name {
            /// `1 - 2 * epsilon`: above this `dot` two unit vectors are
            /// treated as the same direction (and below its negative as
            /// opposite) by [`Self::from_rotation_arc`].
            const ARC_THRESHOLD: $scalar = 1.0 - 2.0 * <$scalar as Scalar>::DEFAULT_EPSILON;

            // ---- rotation vectors --------------------------------------

            /// The rotation described by a *rotation vector* (scaled axis):
            /// the direction of `v` is the axis and its length the angle in
            /// radians. The zero vector is the identity. Unlike
            /// [`Self::from_axis_angle`] this needs no normalized axis and is
            /// well defined (and smooth) around zero.
            #[inline]
            pub fn from_scaled_axis(v: $vec3) -> Self {
                let angle = v.length();
                if angle > 0.0 {
                    let half = angle * 0.5;
                    let scale = Scalar::sin(half) / angle;
                    Self::new(v.x * scale, v.y * scale, v.z * scale, Scalar::cos(half))
                } else {
                    Self::IDENTITY
                }
            }

            /// The rotation vector of this unit quaternion, the inverse of
            /// [`Self::from_scaled_axis`]: axis times angle, with the
            /// shorter of the two equivalent descriptions (`q` and `-q`) so
            /// the length is in `0..=PI`. The identity gives the zero
            /// vector.
            #[inline]
            pub fn to_scaled_axis(self) -> $vec3 {
                let q = if self.w < 0.0 { -self } else { self };
                let vector = q.xyz();
                let length = vector.length();
                if length > 0.0 {
                    vector * (2.0 * Scalar::atan2(length, q.w) / length)
                } else {
                    $vec3::ZERO
                }
            }

            // ---- arcs and look rotations -------------------------------

            /// The shortest rotation that turns the direction `from` into the
            /// direction `to`. Both must be normalized.
            ///
            /// Equal directions give the identity. For opposite directions
            /// there are infinitely many shortest rotations (a half turn
            /// about any perpendicular axis); a deterministic one is picked.
            /// The result is a unit quaternion.
            #[inline]
            pub fn from_rotation_arc(from: $vec3, to: $vec3) -> Self {
                let dot = from.dot(to);
                if dot > Self::ARC_THRESHOLD {
                    Self::IDENTITY
                } else if dot < -Self::ARC_THRESHOLD {
                    let axis = from.any_orthogonal().normalize();
                    Self::new(axis.x, axis.y, axis.z, 0.0)
                } else {
                    let cross = from.cross(to);
                    Self::new(cross.x, cross.y, cross.z, 1.0 + dot).normalize()
                }
            }

            /// The right-handed orientation of an object that looks along
            /// `forward` with `up` pointing up: it takes the local `-Z`
            /// axis to `forward` and keeps the local `+Y` axis in the plane
            /// of `forward` and `up`, on the `up` side. This is the same
            /// convention as [`look_to_rh`](crate::Mat4::look_to_rh): the
            /// view matrix of a camera is the inverse of this rotation.
            ///
            /// Neither vector needs to be normalized, but `forward` must not
            /// be zero or parallel to `up`, otherwise the result is not
            /// finite.
            #[inline]
            pub fn look_rotation_rh(forward: $vec3, up: $vec3) -> Self {
                let back = (-forward).normalize();
                let right = up.cross(back).normalize();
                let up = back.cross(right);
                Self::from_mat3($mat3::from_cols(right, up, back))
            }

            /// The left-handed orientation of an object that looks along
            /// `forward` with `up` pointing up: it takes the local `+Z` axis
            /// to `forward`, otherwise like [`Self::look_rotation_rh`]. This
            /// matches [`look_to_lh`](crate::Mat4::look_to_lh).
            #[inline]
            pub fn look_rotation_lh(forward: $vec3, up: $vec3) -> Self {
                let forward = forward.normalize();
                let right = up.cross(forward).normalize();
                let up = forward.cross(right);
                Self::from_mat3($mat3::from_cols(right, up, forward))
            }

            // ---- angles between rotations ------------------------------

            /// The angle of the shortest rotation that takes `self` to
            /// `rhs`, in `0..=PI`. `q` and `-q` are the same rotation, so
            /// the angle between them is zero. Neither quaternion needs to
            /// be normalized (only the direction of each matters).
            #[inline]
            pub fn angle_between(self, rhs: Self) -> Radians<$scalar> {
                let relative = self.conjugate() * rhs;
                let vector = relative.xyz().length();
                Radians::new(2.0 * Scalar::atan2(vector, Scalar::abs(relative.w)))
            }

            /// Whether the length is 1 within a small tolerance (`2e-4` for
            /// `f32`), that is, whether this is a usable rotation.
            #[inline]
            pub fn is_normalized(self) -> bool {
                Scalar::abs(self.length_sq() - 1.0) <= $normalized_tolerance
            }

            /// Rotates `self` towards `rhs` by at most `max_angle` along the
            /// shortest arc, and not past it: when the remaining angle is
            /// at most `max_angle` the result is `rhs` itself. A negative
            /// `max_angle` counts as zero (the result is `self`). Both
            /// quaternions must be normalized.
            #[inline]
            pub fn rotate_towards(self, rhs: Self, max_angle: impl Into<Radians<$scalar>>) -> Self {
                let max_angle = Scalar::max(max_angle.into().value(), 0.0);
                let angle = self.angle_between(rhs).value();
                if angle <= max_angle {
                    rhs
                } else {
                    self.slerp(rhs, max_angle / angle)
                }
            }

            // ---- exp, ln, pow ------------------------------------------

            /// The quaternion exponential `e^q`. For a pure quaternion
            /// `(v, 0)` it is the rotation by `2 * |v|` about `v`, so `exp`
            /// of half a rotation vector is the rotation itself.
            #[inline]
            pub fn exp(self) -> Self {
                let length = self.xyz().length();
                let scale = Scalar::exp(self.w);
                if length > 0.0 {
                    let factor = scale * Scalar::sin(length) / length;
                    Self::new(
                        self.x * factor,
                        self.y * factor,
                        self.z * factor,
                        scale * Scalar::cos(length),
                    )
                } else {
                    Self::new(0.0, 0.0, 0.0, scale)
                }
            }

            /// The quaternion logarithm, the inverse of [`Self::exp`]. For a
            /// unit quaternion it is the pure quaternion `(axis * angle / 2,
            /// 0)`. The branch follows the sign of `w`, so `q` and `-q` give
            /// different results (a negative real quaternion uses the `X`
            /// axis). The zero quaternion gives `-infinity` in `w`.
            #[inline]
            pub fn ln(self) -> Self {
                let length = self.xyz().length();
                let scale = Scalar::ln(self.length());
                if length > 0.0 {
                    let factor = Scalar::atan2(length, self.w) / length;
                    Self::new(self.x * factor, self.y * factor, self.z * factor, scale)
                } else if self.w < 0.0 {
                    Self::new(<$scalar as Scalar>::PI, 0.0, 0.0, scale)
                } else {
                    Self::new(0.0, 0.0, 0.0, scale)
                }
            }

            /// `self` raised to a real power: for a unit quaternion the
            /// rotation about the same axis by `exponent` times the angle
            /// (`pow(0.5)` is half the rotation, `pow(-1.0)` the inverse).
            ///
            /// This follows the arc that [`Self::ln`] picks, which is the
            /// rotation of at most a half turn only when `w >= 0`; negate a
            /// quaternion with `w < 0` first to get the shorter way around.
            #[inline]
            pub fn pow(self, exponent: $scalar) -> Self {
                (self.ln() * exponent).exp()
            }

            // ---- swing-twist ---------------------------------------------

            /// Splits the rotation into `(swing, twist)` with
            /// `swing * twist == self`: `twist` is the part that turns
            /// about `twist_axis` (so `twist * twist_axis == twist_axis`)
            /// and `swing` the rest, which moves the axis itself. Useful for
            /// joint limits (twist of a limb versus its bend). `twist_axis`
            /// and `self` must be normalized.
            ///
            /// When the rotation is a half turn about an axis perpendicular
            /// to `twist_axis` the twist is not defined; it is then the
            /// identity and the swing is `self`.
            #[inline]
            pub fn to_swing_twist(self, twist_axis: $vec3) -> (Self, Self) {
                let projected = twist_axis * twist_axis.dot(self.xyz());
                let twist = Self::new(projected.x, projected.y, projected.z, self.w)
                    .normalize_or_identity();
                (self * twist.conjugate(), twist)
            }

            // ---- general helpers ---------------------------------------

            /// Whether every component is finite (neither infinite nor
            /// `NaN`).
            #[inline]
            pub const fn is_finite(self) -> bool {
                self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
            }

            /// Whether any component is `NaN`.
            #[inline]
            pub const fn is_nan(self) -> bool {
                self.x.is_nan() || self.y.is_nan() || self.z.is_nan() || self.w.is_nan()
            }

            /// Whether every component is within the type's machine epsilon
            /// of the matching one of `rhs`, see [`crate::approx::approx_eq`].
            ///
            /// This compares components, so `q` and `-q` (the same
            /// rotation) are *not* approximately equal; compare
            /// [`Self::angle_between`] against a tolerance for that.
            #[inline]
            pub fn approx_eq(self, rhs: Self) -> bool {
                crate::approx::approx_eq(self.x, rhs.x)
                    && crate::approx::approx_eq(self.y, rhs.y)
                    && crate::approx::approx_eq(self.z, rhs.z)
                    && crate::approx::approx_eq(self.w, rhs.w)
            }

            /// Whether every component is within `epsilon` of the matching
            /// one of `rhs` (absolute difference), see
            /// [`crate::approx::approx_eq_eps`].
            #[inline]
            pub fn approx_eq_eps(self, rhs: Self, epsilon: $scalar) -> bool {
                crate::approx::approx_eq_eps(self.x, rhs.x, epsilon)
                    && crate::approx::approx_eq_eps(self.y, rhs.y, epsilon)
                    && crate::approx::approx_eq_eps(self.z, rhs.z, epsilon)
                    && crate::approx::approx_eq_eps(self.w, rhs.w, epsilon)
            }

            /// Whether every component is within `rel_epsilon` of the
            /// matching one of `rhs`, scaled by the larger magnitude of the
            /// two, see [`crate::approx::approx_eq_rel`].
            #[inline]
            pub fn approx_eq_rel(self, rhs: Self, rel_epsilon: $scalar) -> bool {
                crate::approx::approx_eq_rel(self.x, rhs.x, rel_epsilon)
                    && crate::approx::approx_eq_rel(self.y, rhs.y, rel_epsilon)
                    && crate::approx::approx_eq_rel(self.z, rhs.z, rel_epsilon)
                    && crate::approx::approx_eq_rel(self.w, rhs.w, rel_epsilon)
            }

            /// Converts to the quaternion of the other float precision,
            /// casting every component with `as`.
            #[inline]
            pub const fn $as_other(self) -> $other {
                $other::new(
                    self.x as $other_scalar,
                    self.y as $other_scalar,
                    self.z as $other_scalar,
                    self.w as $other_scalar,
                )
            }
        }

        impl ::core::fmt::Display for $name {
            /// Formats as `[x, y, z, w]`, each component with the
            /// formatter's own options, so `{:.2}` applies to every
            /// component.
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str("[")?;
                ::core::fmt::Display::fmt(&self.x, formatter)?;
                formatter.write_str(", ")?;
                ::core::fmt::Display::fmt(&self.y, formatter)?;
                formatter.write_str(", ")?;
                ::core::fmt::Display::fmt(&self.z, formatter)?;
                formatter.write_str(", ")?;
                ::core::fmt::Display::fmt(&self.w, formatter)?;
                formatter.write_str("]")
            }
        }
    };
}

quaternion_extras!(Quat, f32, Vec3, Mat3, DQuat, f64, as_dquat, 4e-4);
quaternion_extras!(DQuat, f64, DVec3, DMat3, Quat, f32, as_quat, 1e-9);
