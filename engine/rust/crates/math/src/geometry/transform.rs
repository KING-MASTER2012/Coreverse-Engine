//! The translation-rotation-scale transform: [`Transform`] and
//! [`DTransform`]. See the [module documentation](super) for the
//! conventions.

use crate::approx::approx_eq_rel;
use crate::matrix::{DMat4, Mat4};
use crate::quat::{DQuat, Quat};
use crate::vector::{DVec3, Vec3};
use core::ops::Mul;

/// A transform stored as its three parts: it scales by `scale`, then
/// rotates by `rotation`, then moves by `translation`, which is the
/// `T * R * S` of [`Mat4::from_scale_rotation_translation`]. Compared with a
/// matrix it is smaller, cheap to interpolate ([`Self::lerp`]) and keeps
/// the parts apart.
///
/// The rotation must be a unit quaternion for the transform to be rigid.
///
/// A TRS transform cannot represent shear, which is what a rotation
/// followed by a non-uniform scale produces; the functions that would
/// create it ([`Self::mul_transform`], [`Self::inverse`]) are documented to
/// be exact only without it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    /// The translation, applied last.
    pub translation: Vec3,
    /// The rotation, applied after the scale.
    pub rotation: Quat,
    /// The scale along the three axes, applied first (before rotating).
    pub scale: Vec3,
}

/// The `f64` version of [`Transform`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DTransform {
    /// The translation, applied last.
    pub translation: DVec3,
    /// The rotation, applied after the scale.
    pub rotation: DQuat,
    /// The scale along the three axes, applied first (before rotating).
    pub scale: DVec3,
}

macro_rules! transform_impl {
    ($name:ident, $vec3:ident, $quat:ident, $mat4:ident, $scalar:ident, $tolerance:expr) => {
        impl $name {
            /// The transform that changes nothing.
            pub const IDENTITY: Self = Self {
                translation: $vec3::ZERO,
                rotation: $quat::IDENTITY,
                scale: $vec3::ONE,
            };

            /// The transform with the given parts.
            #[inline]
            pub const fn new(translation: $vec3, rotation: $quat, scale: $vec3) -> Self {
                Self {
                    translation,
                    rotation,
                    scale,
                }
            }

            /// A translation only.
            #[inline]
            pub const fn from_translation(translation: $vec3) -> Self {
                Self::new(translation, $quat::IDENTITY, $vec3::ONE)
            }

            /// A rotation only.
            #[inline]
            pub const fn from_rotation(rotation: $quat) -> Self {
                Self::new($vec3::ZERO, rotation, $vec3::ONE)
            }

            /// A scale only.
            #[inline]
            pub const fn from_scale(scale: $vec3) -> Self {
                Self::new($vec3::ZERO, $quat::IDENTITY, scale)
            }

            /// The same transform with another translation.
            #[inline]
            pub const fn with_translation(self, translation: $vec3) -> Self {
                Self {
                    translation,
                    ..self
                }
            }

            /// The same transform with another rotation.
            #[inline]
            pub const fn with_rotation(self, rotation: $quat) -> Self {
                Self { rotation, ..self }
            }

            /// The same transform with another scale.
            #[inline]
            pub const fn with_scale(self, scale: $vec3) -> Self {
                Self { scale, ..self }
            }

            /// The matrix `T * R * S` of this transform.
            #[inline]
            pub const fn to_mat4(self) -> $mat4 {
                $mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
            }

            /// The transform of a matrix made of a translation, a rotation
            /// and a scale, or `None` when the matrix has no such
            /// decomposition (see
            /// [`Mat4::to_scale_rotation_translation`]).
            #[inline]
            pub fn from_mat4(matrix: $mat4) -> Option<Self> {
                let (scale, rotation, translation) = matrix.to_scale_rotation_translation()?;
                Some(Self::new(translation, rotation, scale))
            }

            /// Whether every part is finite.
            #[inline]
            pub const fn is_finite(self) -> bool {
                self.translation.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
            }

            /// Whether the scale is the same along all three axes (within a
            /// small relative tolerance).
            #[inline]
            pub fn is_uniform_scale(self) -> bool {
                approx_eq_rel(self.scale.x, self.scale.y, $tolerance)
                    && approx_eq_rel(self.scale.y, self.scale.z, $tolerance)
            }

            /// The direction of the local `+X` axis in the parent space.
            #[inline]
            pub fn right(self) -> $vec3 {
                self.rotation * $vec3::X
            }

            /// The direction of the local `+Y` axis in the parent space.
            #[inline]
            pub fn up(self) -> $vec3 {
                self.rotation * $vec3::Y
            }

            /// The direction the transform looks along in the parent space:
            /// the local `-Z` axis (right-handed, like `Mat4::look_to_rh`).
            #[inline]
            pub fn forward(self) -> $vec3 {
                self.rotation * -$vec3::Z
            }

            /// The same transform, turned so that it looks from its
            /// translation towards `target`, with `up` pointing up. The
            /// rotation is not finite when `target` is the translation or
            /// in the direction of `up`.
            #[inline]
            pub fn looking_at(self, target: $vec3, up: $vec3) -> Self {
                self.looking_to(target - self.translation, up)
            }

            /// The same transform, turned to look along `direction` with
            /// `up` pointing up, see [`Self::looking_at`].
            #[inline]
            pub fn looking_to(self, direction: $vec3, up: $vec3) -> Self {
                self.with_rotation($quat::look_rotation_rh(direction, up))
            }

            /// Transforms a point: scales, rotates, then translates it.
            #[inline]
            pub fn transform_point(self, point: $vec3) -> $vec3 {
                self.rotation * (self.scale * point) + self.translation
            }

            /// Transforms a direction or offset: scales and rotates it,
            /// without the translation.
            #[inline]
            pub fn transform_vector(self, vector: $vec3) -> $vec3 {
                self.rotation * (self.scale * vector)
            }

            /// The point that [`Self::transform_point`] sends to `point`.
            /// Exact for any scale, but not finite when a scale component is
            /// zero.
            #[inline]
            pub fn inverse_transform_point(self, point: $vec3) -> $vec3 {
                self.inverse_transform_vector(point - self.translation)
            }

            /// The vector that [`Self::transform_vector`] sends to
            /// `vector`. Exact for any scale, but not finite when a scale
            /// component is zero.
            #[inline]
            pub fn inverse_transform_vector(self, vector: $vec3) -> $vec3 {
                (self.rotation.conjugate() * vector) / self.scale
            }

            /// `self` applied after `rhs`: the transform that does what
            /// `rhs` does and then what `self` does (like `Mat4 * Mat4`).
            ///
            /// Exact when `self` has a uniform scale or `rhs` has no
            /// rotation; otherwise the true product contains shear, which
            /// a TRS transform cannot hold, and the result only
            /// approximates it. Use [`Self::to_mat4`] matrices when that
            /// matters.
            #[inline]
            pub fn mul_transform(self, rhs: Self) -> Self {
                Self {
                    translation: self.transform_point(rhs.translation),
                    rotation: self.rotation * rhs.rotation,
                    scale: self.scale * rhs.scale,
                }
            }

            /// The transform that undoes this one, or `None` when it has
            /// none as a TRS transform: a scale component is zero or not
            /// finite, or the scale is not uniform while the rotation is
            /// not the identity (the inverse then contains shear). For
            /// those use [`Self::inverse_transform_point`] or
            /// [`Mat4::inverse_affine`].
            #[inline]
            pub fn inverse(self) -> Option<Self> {
                let scale = self.scale;
                let invertible =
                    scale.x != 0.0 && scale.y != 0.0 && scale.z != 0.0 && scale.is_finite();
                if !invertible {
                    return None;
                }
                let no_rotation =
                    self.rotation.angle_between($quat::IDENTITY).value() <= $tolerance;
                if !(self.is_uniform_scale() || no_rotation) {
                    return None;
                }
                let scale = $vec3::ONE / scale;
                let rotation = self.rotation.conjugate();
                Some(Self {
                    translation: -(scale * (rotation * self.translation)),
                    rotation,
                    scale,
                })
            }

            /// The transform `t` of the way from `self` to `other`: the
            /// translation and the scale are interpolated linearly, the
            /// rotation with [`slerp`](Quat::slerp) along the shorter arc.
            #[inline]
            pub fn lerp(self, other: Self, t: $scalar) -> Self {
                Self {
                    translation: self.translation.lerp(other.translation, t),
                    rotation: self.rotation.slerp(other.rotation, t),
                    scale: self.scale.lerp(other.scale, t),
                }
            }
        }

        impl Default for $name {
            /// The identity transform.
            #[inline]
            fn default() -> Self {
                Self::IDENTITY
            }
        }

        impl Mul for $name {
            type Output = Self;

            /// `self * rhs` is [`Self::mul_transform`]: `rhs` first.
            #[inline]
            fn mul(self, rhs: Self) -> Self {
                self.mul_transform(rhs)
            }
        }

        impl Mul<$vec3> for $name {
            type Output = $vec3;

            /// `transform * point` is [`Self::transform_point`].
            #[inline]
            fn mul(self, point: $vec3) -> $vec3 {
                self.transform_point(point)
            }
        }
    };
}

transform_impl!(Transform, Vec3, Quat, Mat4, f32, 1e-5);
transform_impl!(DTransform, DVec3, DQuat, DMat4, f64, 1e-10);
