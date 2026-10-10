//! Planes: [`Plane`] and [`DPlane`]. See the [module documentation](super)
//! for the conventions.

use crate::matrix::{DMat4, Mat4};
use crate::scalar::Scalar;
use crate::vector::{DVec3, DVec4, Vec3, Vec4};

/// An infinite plane: the points `p` with `normal . p + d == 0`.
///
/// The functions that measure distances expect a unit `normal`
/// ([`Self::is_normalized`]); every constructor except [`Self::new`]
/// guarantees it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Plane {
    /// The direction the plane faces; a unit vector.
    pub normal: Vec3,
    /// The offset: `-normal . p` for any point `p` on the plane, so the
    /// plane is `-d` away from the origin along `normal`.
    pub d: f32,
}

/// The `f64` version of [`Plane`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DPlane {
    /// The direction the plane faces; a unit vector.
    pub normal: DVec3,
    /// The offset: `-normal . p` for any point `p` on the plane, so the
    /// plane is `-d` away from the origin along `normal`.
    pub d: f64,
}

macro_rules! plane_impl {
    ($name:ident, $vec3:ident, $vec4:ident, $mat4:ident, $scalar:ident, $tolerance:expr) => {
        impl $name {
            /// The plane `normal . p + d == 0`. Nothing is normalized; use
            /// [`Self::normalized`] if `normal` is not a unit vector.
            #[inline]
            pub const fn new(normal: $vec3, d: $scalar) -> Self {
                Self { normal, d }
            }

            /// The plane through `point` that faces `normal`, which is
            /// normalized here (a zero `normal` gives non-finite values).
            #[inline]
            pub fn from_point_normal(point: $vec3, normal: $vec3) -> Self {
                let normal = normal.normalize();
                Self {
                    normal,
                    d: -normal.dot(point),
                }
            }

            /// The plane through three points, facing the side from which
            /// `a`, `b`, `c` appear counter-clockwise (right-handed).
            /// `None` when the points are collinear or not finite.
            #[inline]
            pub fn from_points(a: $vec3, b: $vec3, c: $vec3) -> Option<Self> {
                let normal = (b - a).cross(c - a).try_normalize()?;
                Some(Self {
                    normal,
                    d: -normal.dot(a),
                })
            }

            /// The plane `a x + b y + c z + d == 0` from its four
            /// coefficients, scaled so that the normal `(a, b, c)` has unit
            /// length. `None` when `(a, b, c)` is zero or not finite.
            #[inline]
            pub fn from_coefficients(
                a: $scalar,
                b: $scalar,
                c: $scalar,
                d: $scalar,
            ) -> Option<Self> {
                let normal = $vec3::new(a, b, c);
                let length = normal.length();
                if length > 0.0 && length.is_finite() {
                    Some(Self {
                        normal: normal / length,
                        d: d / length,
                    })
                } else {
                    None
                }
            }

            /// Whether the normal has unit length (within a small
            /// tolerance).
            #[inline]
            pub fn is_normalized(self) -> bool {
                Scalar::abs(self.normal.length_sq() - 1.0) <= $tolerance
            }

            /// The same plane with a unit normal, or `None` when the normal
            /// is zero or not finite.
            #[inline]
            pub fn normalized(self) -> Option<Self> {
                Self::from_coefficients(self.normal.x, self.normal.y, self.normal.z, self.d)
            }

            /// The same plane facing the other way.
            #[inline]
            pub fn flipped(self) -> Self {
                Self {
                    normal: -self.normal,
                    d: -self.d,
                }
            }

            /// Whether the normal and the offset are finite.
            #[inline]
            pub const fn is_finite(self) -> bool {
                self.normal.is_finite() && self.d.is_finite()
            }

            /// The signed distance from the plane to `point`: positive on
            /// the side `normal` faces, negative behind the plane, zero on
            /// it. Only a distance in length units for a unit `normal`.
            #[inline]
            pub fn signed_distance(self, point: $vec3) -> $scalar {
                self.normal.dot(point) + self.d
            }

            /// The point of the plane nearest to `point`.
            #[inline]
            pub fn closest_point(self, point: $vec3) -> $vec3 {
                point - self.normal * (self.signed_distance(point) / self.normal.length_sq())
            }

            /// The point of the plane nearest to the origin.
            #[inline]
            pub fn point_on_plane(self) -> $vec3 {
                -self.normal * (self.d / self.normal.length_sq())
            }

            /// The plane after the affine or projective transform
            /// `matrix` has been applied to every point of it, or `None`
            /// when `matrix` is singular. The normal of the result is
            /// normalized again.
            #[inline]
            pub fn transformed(self, matrix: $mat4) -> Option<Self> {
                let inverse_transpose = matrix.inverse()?.transpose();
                let coefficients = inverse_transpose.mul_vec($vec4::new(
                    self.normal.x,
                    self.normal.y,
                    self.normal.z,
                    self.d,
                ));
                Self::from_coefficients(
                    coefficients.x,
                    coefficients.y,
                    coefficients.z,
                    coefficients.w,
                )
            }
        }
    };
}

plane_impl!(Plane, Vec3, Vec4, Mat4, f32, 4e-4);
plane_impl!(DPlane, DVec3, DVec4, DMat4, f64, 1e-9);
