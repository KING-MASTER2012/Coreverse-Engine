//! Unit quaternions for rotations: [`Quat`] (`f32`) and [`DQuat`] (`f64`).
//!
//! A quaternion `x*i + y*j + z*k + w` with `x`, `y`, `z` the *vector part* and
//! `w` the *scalar part*. A **unit** quaternion (length 1) represents a
//! rotation: [`Quat::from_axis_angle`] with a unit axis `a` and angle `θ`
//! gives `(a * sin(θ/2), cos(θ/2))`. The rotations are right-handed and the
//! angle is counter-clockwise when looking down the axis toward the origin, so
//! a quarter turn about `Z` takes `X` to `Y`.
//!
//! The two types come from a single `macro_rules!` definition, so they cannot
//! drift apart; the matrix and vector types they work with come from
//! `vectors!` and `matrices!`.
//!
//! # Conventions
//!
//! - **Composition matches [`matrix`](crate::matrix).** `a * b` is the
//!   rotation that applies `b` first and then `a`, exactly like `Mat3 * Mat3`:
//!   `(a * b).to_mat3() == a.to_mat3() * b.to_mat3()`. `q * v` rotates the
//!   vector `v`, like `m * v`.
//! - **Unit length is the caller's job.** Rotating (`q * v`), `to_mat3`,
//!   `to_mat4`, `slerp` and friends assume unit quaternions and do not check
//!   it; a non-unit quaternion gives a scaled, meaningless result. Products of
//!   unit quaternions drift from unit length through rounding, so call
//!   [`Quat::normalize`] now and then when composing many rotations.
//! - **`q` and `-q` are the same rotation** (`(x, y, z, w)` and
//!   `(-x, -y, -z, -w)`). Functions that interpolate pick the shorter way
//!   around; functions that return a quaternion may return either sign.
//! - `Default` is [`Quat::IDENTITY`] (no rotation), not zero.
//!
//! # Operations
//!
//! | method                                   | notes                                                 |
//! |------------------------------------------|-------------------------------------------------------|
//! | `IDENTITY`                               | the rotation that does nothing                        |
//! | `new`, `from_array`, `to_array`, `xyz`   | all `const fn`                                        |
//! | `from_axis_angle`, `from_rotation_x/y/z` | angle as `Radians` or `Degrees`                       |
//! | `to_axis_angle`                          | angle in `0..=PI`; `(X, 0)` for the identity          |
//! | `q * v`, `mul_vec3`                      | rotates a vector (`const fn`)                         |
//! | `a * b`, `mul_quat`                      | composition, `b` first (`const fn`)                   |
//! | `conjugate`, `inverse`                   | for unit quaternions the conjugate *is* the inverse   |
//! | `dot`, `length`, `length_sq`             |                                                       |
//! | `normalize`, `try_normalize`, `normalize_or_identity` | like the vector versions                 |
//! | `nlerp`, `slerp`                         | shortest path; `slerp` has constant angular speed     |
//! | `to_mat3`, `to_mat4`, `from_mat3`, `from_mat4` | rotation matrices                               |
//! | `+`, `-`, unary `-`, `* scalar`, `/ scalar` | component-wise, for building interpolations        |
//! | `from_euler`, `to_euler` (`EulerRot`)    | 24 axis orders, intrinsic and extrinsic (Phase 2.4)   |
//! | `from_scaled_axis`, `to_scaled_axis`     | rotation vector: axis times angle                     |
//! | `from_rotation_arc`                      | shortest rotation between two directions              |
//! | `look_rotation_rh/lh`                    | orientation from a forward and an up direction        |
//! | `angle_between`, `rotate_towards`        | shortest angle between rotations; stepping towards one |
//! | `is_normalized`, `exp`, `ln`, `pow`      | fractional rotations: `q.pow(0.5)`                    |
//! | `to_swing_twist`                         | `swing * twist == q`, twist about a given axis        |
//! | `is_finite`, `is_nan`, `approx_eq*`, `Display`, `as_dquat`/`as_quat` | general helpers      |
//!
//! ```
//! use math::{Degrees, Quat, Vec3};
//!
//! let quarter_turn = Quat::from_rotation_z(Degrees::new(90.0));
//! let rotated = quarter_turn * Vec3::X;
//! assert!((rotated - Vec3::Y).length() < 1e-6);
//!
//! // Halfway between "no rotation" and a quarter turn is an eighth of a turn.
//! let halfway = Quat::IDENTITY.slerp(quarter_turn, 0.5);
//! assert!((halfway - Quat::from_rotation_z(Degrees::new(45.0))).length() < 1e-6);
//! ```

pub use self::euler::EulerRot;

use crate::angle::Radians;
use crate::matrix::{DMat3, DMat4, Mat3, Mat4};
use crate::scalar::Scalar;
use crate::vector::{DVec3, DVec4, Vec3, Vec4};

/// Defines one quaternion type over a float scalar, together with its
/// matching vector and matrix types.
macro_rules! quaternions {
    ($(
        $(#[$meta:meta])*
        $name:ident {
            scalar: $scalar:ident,
            vec3: $vec3:ident,
            vec4: $vec4:ident,
            mat3: $mat3:ident,
            mat4: $mat4:ident,
            parallel: $parallel:expr,
        }
    )*) => {$(
        $(#[$meta])*
        #[repr(C)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name {
            /// The `x` component of the vector part.
            pub x: $scalar,
            /// The `y` component of the vector part.
            pub y: $scalar,
            /// The `z` component of the vector part.
            pub z: $scalar,
            /// The scalar part.
            pub w: $scalar,
        }

        impl $name {
            /// The identity rotation, `(0, 0, 0, 1)`: it leaves every vector
            /// unchanged and is neutral for multiplication.
            pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

            /// Above this `dot` of two unit quaternions the angle between
            /// them is so small that `slerp` falls back to `nlerp`, which is
            /// then indistinguishable but avoids dividing by a tiny sine.
            const PARALLEL_THRESHOLD: $scalar = $parallel;

            /// Builds a quaternion from its components. This does not
            /// normalize: only unit quaternions are rotations.
            #[inline]
            pub const fn new(x: $scalar, y: $scalar, z: $scalar, w: $scalar) -> Self {
                Self { x, y, z, w }
            }

            /// Builds a quaternion from `[x, y, z, w]`.
            #[inline]
            pub const fn from_array(array: [$scalar; 4]) -> Self {
                Self::new(array[0], array[1], array[2], array[3])
            }

            /// The components as `[x, y, z, w]`.
            #[inline]
            pub const fn to_array(self) -> [$scalar; 4] {
                [self.x, self.y, self.z, self.w]
            }

            /// The vector part `(x, y, z)`.
            #[inline]
            pub const fn xyz(self) -> $vec3 {
                $vec3::new(self.x, self.y, self.z)
            }

            /// The rotation by `angle` about `axis`, counter-clockwise when
            /// looking down the axis toward the origin. `angle` can be
            /// `Radians` or `Degrees`.
            ///
            /// `axis` must be normalized, otherwise the result is not a unit
            /// quaternion and therefore not a rotation.
            #[inline]
            pub fn from_axis_angle(axis: $vec3, angle: impl Into<Radians<$scalar>>) -> Self {
                let half = angle.into().value() * 0.5;
                let (sin, cos) = (Scalar::sin(half), Scalar::cos(half));
                Self::new(axis.x * sin, axis.y * sin, axis.z * sin, cos)
            }

            /// The rotation by `angle` about the `X` axis.
            #[inline]
            pub fn from_rotation_x(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_axis_angle($vec3::X, angle)
            }

            /// The rotation by `angle` about the `Y` axis.
            #[inline]
            pub fn from_rotation_y(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_axis_angle($vec3::Y, angle)
            }

            /// The rotation by `angle` about the `Z` axis.
            #[inline]
            pub fn from_rotation_z(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_axis_angle($vec3::Z, angle)
            }

            /// The axis and angle of this unit quaternion, the inverse of
            /// [`Self::from_axis_angle`].
            ///
            /// The angle is in `0..=PI`: of the two equivalent descriptions
            /// (`q` and `-q`) the one with the shorter rotation is used, with
            /// the axis flipped as needed. For a rotation of zero (the
            /// identity) any axis would do, and `(X, 0)` is returned.
            #[inline]
            pub fn to_axis_angle(self) -> ($vec3, Radians<$scalar>) {
                let q = if self.w < 0.0 { -self } else { self };
                let vector = q.xyz();
                let length = vector.length();
                if length > 0.0 {
                    (
                        vector / length,
                        Radians::new(2.0 * Scalar::atan2(length, q.w)),
                    )
                } else {
                    ($vec3::X, Radians::new(0.0))
                }
            }

            /// The conjugate `(-x, -y, -z, w)`. For a unit quaternion this is
            /// the inverse rotation, and cheaper than [`Self::inverse`].
            #[inline]
            pub const fn conjugate(self) -> Self {
                Self::new(-self.x, -self.y, -self.z, self.w)
            }

            /// The multiplicative inverse, `conjugate / length_sq`, so that
            /// `q * q.inverse()` is the identity for any non-zero `q`.
            ///
            /// The result has `NaN`/infinite components for the zero
            /// quaternion. Use [`Self::conjugate`] for unit quaternions.
            #[inline]
            pub const fn inverse(self) -> Self {
                let scale = 1.0 / self.length_sq();
                Self::new(
                    -self.x * scale,
                    -self.y * scale,
                    -self.z * scale,
                    self.w * scale,
                )
            }

            /// The dot product of the two quaternions seen as 4-vectors. For
            /// unit quaternions it is `cos(angle / 2)` of the rotation that
            /// takes one to the other, and `1` or `-1` for the same rotation.
            #[inline]
            pub const fn dot(self, rhs: Self) -> $scalar {
                self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w
            }

            /// The squared length, `self.dot(self)`.
            #[inline]
            pub const fn length_sq(self) -> $scalar {
                self.dot(self)
            }

            /// The length (norm). A rotation has length 1.
            #[inline]
            pub fn length(self) -> $scalar {
                Scalar::sqrt(self.length_sq())
            }

            /// The quaternion scaled to length 1.
            ///
            /// The zero quaternion has no direction: the result is then `NaN`
            /// in every component. Use [`Self::try_normalize`] or
            /// [`Self::normalize_or_identity`] when that can happen.
            #[inline]
            pub fn normalize(self) -> Self {
                self / self.length()
            }

            /// Like [`Self::normalize`], but `None` when the quaternion has
            /// no usable direction: zero length, a non-finite component, or a
            /// squared length that overflows or underflows.
            #[inline]
            pub fn try_normalize(self) -> ::core::option::Option<Self> {
                let length = self.length();
                if length > 0.0 && length.is_finite() {
                    ::core::option::Option::Some(self / length)
                } else {
                    ::core::option::Option::None
                }
            }

            /// Like [`Self::normalize`], but the identity rotation for input
            /// that [`Self::try_normalize`] rejects.
            #[inline]
            pub fn normalize_or_identity(self) -> Self {
                self.try_normalize().unwrap_or(Self::IDENTITY)
            }

            /// The Hamilton product `self * rhs`: the rotation that applies
            /// `rhs` first and then `self`. Not commutative.
            #[inline]
            pub const fn mul_quat(self, rhs: Self) -> Self {
                Self::new(
                    self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
                    self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
                    self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
                    self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
                )
            }

            /// Rotates `rhs` by this unit quaternion.
            ///
            /// Uses `v + w t + u x t` with `t = 2 (u x v)` and `u` the vector
            /// part, which needs far fewer operations than building the
            /// matrix. The quaternion must be normalized; a non-unit one
            /// scales the result by its squared length.
            #[inline]
            pub const fn mul_vec3(self, rhs: $vec3) -> $vec3 {
                let tx = 2.0 * (self.y * rhs.z - self.z * rhs.y);
                let ty = 2.0 * (self.z * rhs.x - self.x * rhs.z);
                let tz = 2.0 * (self.x * rhs.y - self.y * rhs.x);
                $vec3::new(
                    rhs.x + self.w * tx + (self.y * tz - self.z * ty),
                    rhs.y + self.w * ty + (self.z * tx - self.x * tz),
                    rhs.z + self.w * tz + (self.x * ty - self.y * tx),
                )
            }

            /// Interpolates from `self` (at `t = 0`) to `rhs` (at `t = 1`) by
            /// blending the components linearly and normalizing: "normalized
            /// lerp". Cheaper than [`Self::slerp`] and takes the same
            /// shorter path, but the rotation speed is not constant (fastest
            /// in the middle).
            ///
            /// Both quaternions must be normalized. `t` is not clamped, so
            /// values outside `0..=1` extrapolate.
            #[inline]
            pub fn nlerp(self, rhs: Self, t: $scalar) -> Self {
                let rhs = if self.dot(rhs) < 0.0 { -rhs } else { rhs };
                (self * (1.0 - t) + rhs * t).normalize()
            }

            /// Spherical linear interpolation from `self` (at `t = 0`) to
            /// `rhs` (at `t = 1`): the rotation turns at constant angular
            /// speed along the shorter of the two arcs.
            ///
            /// Both quaternions must be normalized. `t` is not clamped, so
            /// values outside `0..=1` extrapolate along the same arc. At
            /// `t = 1` the result may be `-rhs`, which is the same rotation.
            /// For nearly equal rotations this falls back to
            /// [`Self::nlerp`], which is then indistinguishable.
            #[inline]
            pub fn slerp(self, rhs: Self, t: $scalar) -> Self {
                let mut dot = self.dot(rhs);
                let rhs = if dot < 0.0 {
                    dot = -dot;
                    -rhs
                } else {
                    rhs
                };
                if dot > Self::PARALLEL_THRESHOLD {
                    return (self * (1.0 - t) + rhs * t).normalize();
                }
                let theta = Scalar::acos(dot);
                let sin_theta = Scalar::sin(theta);
                let scale_self = Scalar::sin((1.0 - t) * theta) / sin_theta;
                let scale_rhs = Scalar::sin(t * theta) / sin_theta;
                self * scale_self + rhs * scale_rhs
            }

            #[doc = concat!(
                "The rotation matrix of this unit quaternion: `q * v` equals `q.to_mat3() * v`. ",
                "The quaternion must be normalized; a non-unit one gives a scaled matrix, ",
                "not a rotation (see [`",
                stringify!($mat3),
                "`])."
            )]
            #[inline]
            pub const fn to_mat3(self) -> $mat3 {
                let (x2, y2, z2) = (self.x + self.x, self.y + self.y, self.z + self.z);
                let (xx, xy, xz) = (self.x * x2, self.x * y2, self.x * z2);
                let (yy, yz, zz) = (self.y * y2, self.y * z2, self.z * z2);
                let (wx, wy, wz) = (self.w * x2, self.w * y2, self.w * z2);
                $mat3::from_cols(
                    $vec3::new(1.0 - (yy + zz), xy + wz, xz - wy),
                    $vec3::new(xy - wz, 1.0 - (xx + zz), yz + wx),
                    $vec3::new(xz + wy, yz - wx, 1.0 - (xx + yy)),
                )
            }

            #[doc = concat!(
                "The rotation as a homogeneous [`",
                stringify!($mat4),
                "`]: the matrix of [`Self::to_mat3`] with no translation, ",
                "in the upper left corner of an otherwise identity matrix."
            )]
            #[inline]
            pub const fn to_mat4(self) -> $mat4 {
                let m = self.to_mat3();
                $mat4::from_cols(
                    $vec4::new(m.x_axis.x, m.x_axis.y, m.x_axis.z, 0.0),
                    $vec4::new(m.y_axis.x, m.y_axis.y, m.y_axis.z, 0.0),
                    $vec4::new(m.z_axis.x, m.z_axis.y, m.z_axis.z, 0.0),
                    $vec4::W,
                )
            }

            #[doc = concat!(
                "The unit quaternion of a rotation matrix, the inverse of [`Self::to_mat3`].\n\n",
                "`m` must be a rotation: orthonormal columns and determinant `+1`. ",
                "Scale, shear and mirroring give a meaningless result. Tiny deviations ",
                "from orthonormality (rounding after many multiplications) are fine, ",
                "since the result is normalized. The sign of the result is arbitrary: ",
                "`q` and `-q` are the same rotation.\n\n",
                "Uses Shepperd's method: it divides by the largest of four candidate ",
                "terms, so it stays accurate for rotations of any angle, including ",
                "half turns."
            )]
            #[inline]
            pub fn from_mat3(m: $mat3) -> Self {
                let (m00, m10, m20) = (m.x_axis.x, m.x_axis.y, m.x_axis.z);
                let (m01, m11, m21) = (m.y_axis.x, m.y_axis.y, m.y_axis.z);
                let (m02, m12, m22) = (m.z_axis.x, m.z_axis.y, m.z_axis.z);
                let trace = m00 + m11 + m22;
                let quaternion = if trace > 0.0 {
                    let s = Scalar::sqrt(trace + 1.0) * 2.0;
                    Self::new((m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s)
                } else if m00 > m11 && m00 > m22 {
                    let s = Scalar::sqrt(1.0 + m00 - m11 - m22) * 2.0;
                    Self::new(0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s)
                } else if m11 > m22 {
                    let s = Scalar::sqrt(1.0 + m11 - m00 - m22) * 2.0;
                    Self::new((m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s)
                } else {
                    let s = Scalar::sqrt(1.0 + m22 - m00 - m11) * 2.0;
                    Self::new((m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s)
                };
                quaternion.normalize()
            }

            #[doc = concat!(
                "The unit quaternion of the rotation in the upper left 3x3 block of `m`, ",
                "see [`Self::from_mat3`]. The translation and the bottom row of `m` are ",
                "ignored, so a rigid transform (rotation followed by translation) works; ",
                "scale, shear and projection do not."
            )]
            #[inline]
            pub fn from_mat4(m: $mat4) -> Self {
                Self::from_mat3($mat3::from_cols(
                    $vec3::new(m.x_axis.x, m.x_axis.y, m.x_axis.z),
                    $vec3::new(m.y_axis.x, m.y_axis.y, m.y_axis.z),
                    $vec3::new(m.z_axis.x, m.z_axis.y, m.z_axis.z),
                ))
            }
        }

        impl ::core::default::Default for $name {
            /// The identity rotation.
            #[inline]
            fn default() -> Self {
                Self::IDENTITY
            }
        }

        impl ::core::ops::Mul<$name> for $name {
            type Output = $name;

            /// The composition of rotations, see [`Self::mul_quat`].
            #[inline]
            fn mul(self, rhs: $name) -> $name {
                self.mul_quat(rhs)
            }
        }

        impl ::core::ops::Mul<$vec3> for $name {
            type Output = $vec3;

            /// Rotates the vector, see [`Self::mul_vec3`].
            #[inline]
            fn mul(self, rhs: $vec3) -> $vec3 {
                self.mul_vec3(rhs)
            }
        }

        impl ::core::ops::MulAssign<$name> for $name {
            #[inline]
            fn mul_assign(&mut self, rhs: $name) {
                *self = self.mul_quat(rhs);
            }
        }

        impl ::core::ops::Add for $name {
            type Output = $name;

            /// Component-wise sum. Not a composition of rotations.
            #[inline]
            fn add(self, rhs: $name) -> $name {
                Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z, self.w + rhs.w)
            }
        }

        impl ::core::ops::Sub for $name {
            type Output = $name;

            /// Component-wise difference.
            #[inline]
            fn sub(self, rhs: $name) -> $name {
                Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z, self.w - rhs.w)
            }
        }

        impl ::core::ops::Neg for $name {
            type Output = $name;

            /// Negates every component. `-q` is the same rotation as `q`.
            #[inline]
            fn neg(self) -> $name {
                Self::new(-self.x, -self.y, -self.z, -self.w)
            }
        }

        impl ::core::ops::Mul<$scalar> for $name {
            type Output = $name;

            /// Scales every component.
            #[inline]
            fn mul(self, rhs: $scalar) -> $name {
                Self::new(self.x * rhs, self.y * rhs, self.z * rhs, self.w * rhs)
            }
        }

        impl ::core::ops::Mul<$name> for $scalar {
            type Output = $name;

            /// Scales every component.
            #[inline]
            fn mul(self, rhs: $name) -> $name {
                rhs * self
            }
        }

        impl ::core::ops::Div<$scalar> for $name {
            type Output = $name;

            /// Divides every component.
            #[inline]
            fn div(self, rhs: $scalar) -> $name {
                Self::new(self.x / rhs, self.y / rhs, self.z / rhs, self.w / rhs)
            }
        }

        impl ::core::convert::From<[$scalar; 4]> for $name {
            #[inline]
            fn from(array: [$scalar; 4]) -> Self {
                Self::from_array(array)
            }
        }

        impl ::core::convert::From<$name> for [$scalar; 4] {
            #[inline]
            fn from(quaternion: $name) -> Self {
                quaternion.to_array()
            }
        }
    )*};
}

quaternions! {
    /// A rotation as a unit quaternion with `f32` components.
    ///
    /// `#[repr(C)]` in `x, y, z, w` order with no padding: exactly four
    /// `f32`s in memory. See the [module documentation](self) for the
    /// conventions.
    Quat {
        scalar: f32,
        vec3: Vec3,
        vec4: Vec4,
        mat3: Mat3,
        mat4: Mat4,
        parallel: 0.9995,
    }

    /// A rotation as a unit quaternion with `f64` components.
    ///
    /// `#[repr(C)]` in `x, y, z, w` order with no padding: exactly four
    /// `f64`s in memory. See the [module documentation](self) for the
    /// conventions.
    DQuat {
        scalar: f64,
        vec3: DVec3,
        vec4: DVec4,
        mat3: DMat3,
        mat4: DMat4,
        parallel: 0.999_999_999,
    }
}

mod euler;
mod ext;

#[cfg(test)]
mod euler_tests;
#[cfg(test)]
mod ext_tests;
#[cfg(test)]
mod tests;
