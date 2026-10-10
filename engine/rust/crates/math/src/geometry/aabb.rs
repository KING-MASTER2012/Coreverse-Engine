//! Axis-aligned bounding boxes: [`Aabb2`], [`Aabb3`] and their `f64`
//! versions. See the [module documentation](super) for the conventions.

use super::plane::{DPlane, Plane};
use super::sphere::{DSphere, Sphere};
use crate::matrix::{DMat4, Mat4};
use crate::scalar::Scalar;
use crate::vector::{DVec2, DVec3, Vec2, Vec3};

/// A 2D axis-aligned box, the rectangle between `min` and `max`
/// (inclusive). `min <= max` on both axes for a [valid](Self::is_valid) box.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Aabb2 {
    /// The corner with the smallest coordinates.
    pub min: Vec2,
    /// The corner with the largest coordinates.
    pub max: Vec2,
}

/// A 3D axis-aligned box, the volume between `min` and `max` (inclusive).
/// `min <= max` on all three axes for a [valid](Self::is_valid) box.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Aabb3 {
    /// The corner with the smallest coordinates.
    pub min: Vec3,
    /// The corner with the largest coordinates.
    pub max: Vec3,
}

/// The `f64` version of [`Aabb2`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DAabb2 {
    /// The corner with the smallest coordinates.
    pub min: DVec2,
    /// The corner with the largest coordinates.
    pub max: DVec2,
}

/// The `f64` version of [`Aabb3`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DAabb3 {
    /// The corner with the smallest coordinates.
    pub min: DVec3,
    /// The corner with the largest coordinates.
    pub max: DVec3,
}

/// The methods every box has, whatever its dimension.
macro_rules! aabb_common {
    ($name:ident, $vec:ident, $scalar:ident) => {
        impl $name {
            /// The empty box: `min` is `+infinity` and `max` `-infinity`.
            /// It contains nothing, intersects nothing, and is the neutral
            /// element of [`Self::union`] and [`Self::union_point`], so it
            /// is the natural start for accumulating bounds.
            pub const EMPTY: Self = Self {
                min: $vec::splat(<$scalar>::INFINITY),
                max: $vec::splat(<$scalar>::NEG_INFINITY),
            };

            /// The box between the corners `min` and `max`. Nothing is
            /// checked: see [`Self::is_valid`].
            #[inline]
            pub const fn new(min: $vec, max: $vec) -> Self {
                Self { min, max }
            }

            /// The box that contains only `point`.
            #[inline]
            pub const fn from_point(point: $vec) -> Self {
                Self {
                    min: point,
                    max: point,
                }
            }

            /// The box around `center` that extends `half_extents` to
            /// either side.
            #[inline]
            pub fn from_center_half_extents(center: $vec, half_extents: $vec) -> Self {
                Self {
                    min: center - half_extents,
                    max: center + half_extents,
                }
            }

            /// The box around `center` with the given total `size`.
            #[inline]
            pub fn from_center_size(center: $vec, size: $vec) -> Self {
                Self::from_center_half_extents(center, size * 0.5)
            }

            /// The smallest box that contains all `points`, or `None` for
            /// an empty slice.
            #[inline]
            pub fn from_points(points: &[$vec]) -> Option<Self> {
                let (first, rest) = points.split_first()?;
                let mut result = Self::from_point(*first);
                for &point in rest {
                    result = result.union_point(point);
                }
                Some(result)
            }

            /// Whether `min <= max` on every axis. `false` for
            /// [`Self::EMPTY`] and for any box with a `NaN` corner.
            #[inline]
            pub fn is_valid(self) -> bool {
                self.min
                    .to_array()
                    .iter()
                    .zip(self.max.to_array().iter())
                    .all(|(low, high)| low <= high)
            }

            /// Whether every coordinate of both corners is finite.
            #[inline]
            pub const fn is_finite(self) -> bool {
                self.min.is_finite() && self.max.is_finite()
            }

            /// The size along every axis, `max - min`.
            #[inline]
            pub fn size(self) -> $vec {
                self.max - self.min
            }

            /// Half of [`Self::size`]: the distance from the center to the
            /// faces.
            #[inline]
            pub fn half_extents(self) -> $vec {
                self.size() * 0.5
            }

            /// The center of the box.
            #[inline]
            pub fn center(self) -> $vec {
                (self.min + self.max) * 0.5
            }

            /// The index of the axis along which the box is longest (`0` =
            /// x), the lowest one on a tie. Useful to pick the split axis
            /// of a bounding volume hierarchy.
            #[inline]
            pub fn longest_axis(self) -> usize {
                let size = self.size().to_array();
                let mut axis = 0;
                for (index, &length) in size.iter().enumerate().skip(1) {
                    if length > size[axis] {
                        axis = index;
                    }
                }
                axis
            }

            /// Whether `point` is inside the box or on its surface.
            #[inline]
            pub fn contains_point(self, point: $vec) -> bool {
                self.min
                    .to_array()
                    .iter()
                    .zip(self.max.to_array().iter())
                    .zip(point.to_array().iter())
                    .all(|((low, high), value)| (*low..=*high).contains(value))
            }

            /// Whether `other` is entirely inside this box (touching the
            /// surface is fine). Every box contains [`Self::EMPTY`].
            #[inline]
            pub fn contains(self, other: Self) -> bool {
                self.min
                    .to_array()
                    .iter()
                    .zip(self.max.to_array().iter())
                    .zip(other.min.to_array().iter().zip(other.max.to_array().iter()))
                    .all(|((low, high), (other_low, other_high))| {
                        low <= other_low && other_high <= high
                    })
            }

            /// Whether the two boxes share at least one point; boxes that
            /// only touch intersect.
            #[inline]
            pub fn intersects(self, other: Self) -> bool {
                self.min
                    .to_array()
                    .iter()
                    .zip(self.max.to_array().iter())
                    .zip(other.min.to_array().iter().zip(other.max.to_array().iter()))
                    .all(|((low, high), (other_low, other_high))| {
                        low <= other_high && other_low <= high
                    })
            }

            /// The box both boxes share, or `None` when they do not
            /// intersect. Boxes that only touch give a box of zero size.
            #[inline]
            pub fn intersection(self, other: Self) -> Option<Self> {
                let result = Self {
                    min: self.min.max(other.min),
                    max: self.max.min(other.max),
                };
                result.is_valid().then_some(result)
            }

            /// The smallest box that contains both boxes.
            #[inline]
            pub fn union(self, other: Self) -> Self {
                Self {
                    min: self.min.min(other.min),
                    max: self.max.max(other.max),
                }
            }

            /// The smallest box that contains this box and `point`.
            #[inline]
            pub fn union_point(self, point: $vec) -> Self {
                Self {
                    min: self.min.min(point),
                    max: self.max.max(point),
                }
            }

            /// The box grown by `margin` on every side; a negative `margin`
            /// shrinks it (and can make it invalid).
            #[inline]
            pub fn expand(self, margin: $scalar) -> Self {
                Self {
                    min: self.min - $vec::splat(margin),
                    max: self.max + $vec::splat(margin),
                }
            }

            /// The box moved by `offset`.
            #[inline]
            pub fn translated(self, offset: $vec) -> Self {
                Self {
                    min: self.min + offset,
                    max: self.max + offset,
                }
            }

            /// The point of the box nearest to `point`: `point` itself when
            /// it is inside.
            #[inline]
            pub fn closest_point(self, point: $vec) -> $vec {
                point.clamp(self.min, self.max)
            }

            /// The squared distance from `point` to the box, zero when the
            /// point is inside.
            #[inline]
            pub fn distance_sq_to_point(self, point: $vec) -> $scalar {
                (point - self.closest_point(point)).length_sq()
            }

            /// The distance from `point` to the box, zero when the point is
            /// inside.
            #[inline]
            pub fn distance_to_point(self, point: $vec) -> $scalar {
                (point - self.closest_point(point)).length()
            }

            /// Whether both corners are within `epsilon` of the matching
            /// corners of `other` (absolute difference).
            #[inline]
            pub fn approx_eq_eps(self, other: Self, epsilon: $scalar) -> bool {
                self.min.approx_eq_eps(other.min, epsilon)
                    && self.max.approx_eq_eps(other.max, epsilon)
            }
        }
    };
}

/// The methods only the 2D box has.
macro_rules! aabb2_impl {
    ($name:ident, $vec:ident, $scalar:ident) => {
        impl $name {
            /// The area of the rectangle.
            #[inline]
            pub fn area(self) -> $scalar {
                let size = self.size();
                size.x * size.y
            }

            /// The length of the outline of the rectangle.
            #[inline]
            pub fn perimeter(self) -> $scalar {
                let size = self.size();
                2.0 * (size.x + size.y)
            }

            /// The four corners. Corner `i` takes `max` on the x axis when
            /// bit 0 of `i` is set and on the y axis when bit 1 is set, so
            /// `corners()[0]` is `min` and `corners()[3]` is `max`.
            #[inline]
            pub const fn corners(self) -> [$vec; 4] {
                [
                    $vec::new(self.min.x, self.min.y),
                    $vec::new(self.max.x, self.min.y),
                    $vec::new(self.min.x, self.max.y),
                    $vec::new(self.max.x, self.max.y),
                ]
            }
        }
    };
}

/// The methods only the 3D box has.
macro_rules! aabb3_impl {
    ($name:ident, $vec:ident, $scalar:ident, $mat4:ident, $sphere:ident, $plane:ident) => {
        impl $name {
            /// The volume of the box.
            #[inline]
            pub fn volume(self) -> $scalar {
                let size = self.size();
                size.x * size.y * size.z
            }

            /// The total area of the six faces.
            #[inline]
            pub fn surface_area(self) -> $scalar {
                let size = self.size();
                2.0 * (size.x * size.y + size.y * size.z + size.z * size.x)
            }

            /// The eight corners. Corner `i` takes `max` on the x axis when
            /// bit 0 of `i` is set, on the y axis when bit 1 is set and on
            /// the z axis when bit 2 is set, so `corners()[0]` is `min` and
            /// `corners()[7]` is `max`.
            #[inline]
            pub const fn corners(self) -> [$vec; 8] {
                [
                    $vec::new(self.min.x, self.min.y, self.min.z),
                    $vec::new(self.max.x, self.min.y, self.min.z),
                    $vec::new(self.min.x, self.max.y, self.min.z),
                    $vec::new(self.max.x, self.max.y, self.min.z),
                    $vec::new(self.min.x, self.min.y, self.max.z),
                    $vec::new(self.max.x, self.min.y, self.max.z),
                    $vec::new(self.min.x, self.max.y, self.max.z),
                    $vec::new(self.max.x, self.max.y, self.max.z),
                ]
            }

            /// The smallest axis-aligned box that contains this box after
            /// `matrix` has transformed it: exactly the box around the eight
            /// transformed corners, but computed from the center and the
            /// absolute value of the linear part (Arvo's method). `matrix`
            /// must be affine (no projection); the box grows under
            /// rotations. [`Self::EMPTY`] stays empty.
            #[inline]
            pub fn transform(self, matrix: $mat4) -> Self {
                if !self.is_valid() {
                    return Self::EMPTY;
                }
                let half = self.half_extents();
                let center = matrix.transform_point3(self.center());
                let new_half = matrix.x_axis.xyz().abs() * half.x
                    + matrix.y_axis.xyz().abs() * half.y
                    + matrix.z_axis.xyz().abs() * half.z;
                Self::from_center_half_extents(center, new_half)
            }

            /// The smallest sphere around the center of the box that
            /// contains it.
            #[inline]
            pub fn bounding_sphere(self) -> $sphere {
                $sphere::new(self.center(), self.half_extents().length())
            }

            /// Whether the box and `sphere` share at least one point.
            #[inline]
            pub fn intersects_sphere(self, sphere: $sphere) -> bool {
                self.distance_sq_to_point(sphere.center) <= sphere.radius * sphere.radius
            }

            /// Whether the box and `plane` share at least one point, that
            /// is, whether the plane cuts the box or touches it.
            #[inline]
            pub fn intersects_plane(self, plane: $plane) -> bool {
                let reach = self.half_extents().dot(plane.normal.abs());
                Scalar::abs(plane.signed_distance(self.center())) <= reach
            }
        }
    };
}

aabb_common!(Aabb2, Vec2, f32);
aabb_common!(Aabb3, Vec3, f32);
aabb_common!(DAabb2, DVec2, f64);
aabb_common!(DAabb3, DVec3, f64);
aabb2_impl!(Aabb2, Vec2, f32);
aabb2_impl!(DAabb2, DVec2, f64);
aabb3_impl!(Aabb3, Vec3, f32, Mat4, Sphere, Plane);
aabb3_impl!(DAabb3, DVec3, f64, DMat4, DSphere, DPlane);
