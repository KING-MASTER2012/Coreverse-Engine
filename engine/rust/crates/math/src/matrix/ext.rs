//! General helpers for the matrices (Phase 2.4): `is_finite`, `is_nan`,
//! `approx_eq*`, `Display`, conversion between the `f32` and `f64` families,
//! and the Euler/rotation-translation constructors that go with `Quat`.
//!
//! Written once in `macro_rules!` and instantiated for all six matrix types.

use super::{DMat2, DMat3, DMat4, Mat2, Mat3, Mat4};
use crate::angle::Radians;
use crate::quat::{DQuat, EulerRot, Quat};
use crate::vector::{DVec3, DVec4, Vec3, Vec4};

macro_rules! matrix_extras {
    (
        $name:ident, $scalar:ident, $other:ident, $as_other:ident, $as_vec:ident,
        $first:ident $(, $rest:ident)*
    ) => {
        impl $name {
            /// Whether every element is finite (neither infinite nor `NaN`).
            #[inline]
            pub const fn is_finite(&self) -> bool {
                self.$first.is_finite() $(&& self.$rest.is_finite())*
            }

            /// Whether any element is `NaN`.
            #[inline]
            pub const fn is_nan(&self) -> bool {
                self.$first.is_nan() $(|| self.$rest.is_nan())*
            }

            /// Whether every element is within the type's machine epsilon of
            /// the matching one of `rhs`, see [`crate::approx::approx_eq`].
            /// Use [`Self::approx_eq_eps`] after more than a couple of
            /// operations' worth of rounding error.
            #[inline]
            pub fn approx_eq(&self, rhs: Self) -> bool {
                self.$first.approx_eq(rhs.$first) $(&& self.$rest.approx_eq(rhs.$rest))*
            }

            /// Whether every element is within `epsilon` of the matching one
            /// of `rhs` (absolute difference).
            #[inline]
            pub fn approx_eq_eps(&self, rhs: Self, epsilon: $scalar) -> bool {
                self.$first.approx_eq_eps(rhs.$first, epsilon)
                    $(&& self.$rest.approx_eq_eps(rhs.$rest, epsilon))*
            }

            /// Whether every element is within `rel_epsilon` of the matching
            /// one of `rhs`, scaled by the larger magnitude of the two.
            #[inline]
            pub fn approx_eq_rel(&self, rhs: Self, rel_epsilon: $scalar) -> bool {
                self.$first.approx_eq_rel(rhs.$first, rel_epsilon)
                    $(&& self.$rest.approx_eq_rel(rhs.$rest, rel_epsilon))*
            }

            #[doc = concat!(
                "Converts to a [`",
                stringify!($other),
                "`], casting every element with `as`."
            )]
            #[inline]
            pub const fn $as_other(&self) -> $other {
                $other::from_cols(self.$first.$as_vec() $(, self.$rest.$as_vec())*)
            }
        }

        impl ::core::fmt::Display for $name {
            /// Formats as a list of columns, `[[x_axis], [y_axis], ...]`,
            /// each element with the formatter's own options.
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str("[")?;
                ::core::fmt::Display::fmt(&self.$first, formatter)?;
                $(
                    formatter.write_str(", ")?;
                    ::core::fmt::Display::fmt(&self.$rest, formatter)?;
                )*
                formatter.write_str("]")
            }
        }
    };
}

matrix_extras!(Mat2, f32, DMat2, as_dmat2, as_dvec2, x_axis, y_axis);
matrix_extras!(Mat3, f32, DMat3, as_dmat3, as_dvec3, x_axis, y_axis, z_axis);
matrix_extras!(
    Mat4, f32, DMat4, as_dmat4, as_dvec4, x_axis, y_axis, z_axis, w_axis
);
matrix_extras!(DMat2, f64, Mat2, as_mat2, as_vec2, x_axis, y_axis);
matrix_extras!(DMat3, f64, Mat3, as_mat3, as_vec3, x_axis, y_axis, z_axis);
matrix_extras!(
    DMat4, f64, Mat4, as_mat4, as_vec4, x_axis, y_axis, z_axis, w_axis
);

/// The constructors that need a quaternion.
macro_rules! rotation_constructors {
    ($mat3:ident, $mat4:ident, $quat:ident, $vec3:ident, $vec4:ident, $scalar:ident) => {
        impl $mat3 {
            /// The rotation matrix of three turns about coordinate axes, see
            /// [`Quat::from_euler`]: the same as
            /// `Quat::from_euler(order, a, b, c).to_mat3()`.
            #[inline]
            pub fn from_euler(
                order: EulerRot,
                a: impl Into<Radians<$scalar>>,
                b: impl Into<Radians<$scalar>>,
                c: impl Into<Radians<$scalar>>,
            ) -> Self {
                $quat::from_euler(order, a, b, c).to_mat3()
            }
        }

        impl $mat4 {
            /// The rotation matrix of three turns about coordinate axes, see
            /// [`Quat::from_euler`]: the same as
            /// `Quat::from_euler(order, a, b, c).to_mat4()`.
            #[inline]
            pub fn from_euler(
                order: EulerRot,
                a: impl Into<Radians<$scalar>>,
                b: impl Into<Radians<$scalar>>,
                c: impl Into<Radians<$scalar>>,
            ) -> Self {
                $quat::from_euler(order, a, b, c).to_mat4()
            }

            /// The rigid transform that rotates by the unit quaternion
            /// `rotation` and then translates by `translation`: `T * R`,
            /// the same as `from_scale_rotation_translation` with unit scale
            /// but cheaper.
            #[inline]
            pub const fn from_rotation_translation(rotation: $quat, translation: $vec3) -> Self {
                let m = rotation.to_mat4();
                Self::from_cols(
                    m.x_axis,
                    m.y_axis,
                    m.z_axis,
                    $vec4::new(translation.x, translation.y, translation.z, 1.0),
                )
            }
        }
    };
}

rotation_constructors!(Mat3, Mat4, Quat, Vec3, Vec4, f32);
rotation_constructors!(DMat3, DMat4, DQuat, DVec3, DVec4, f64);
