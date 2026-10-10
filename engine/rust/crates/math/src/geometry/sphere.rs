//! Spheres: [`Sphere`] and [`DSphere`]. See the [module documentation](super)
//! for the conventions.

use super::aabb::{Aabb3, DAabb3};
use super::plane::{DPlane, Plane};
use crate::scalar::Scalar;
use crate::vector::{DVec3, Vec3};

/// A solid sphere: every point within `radius` of `center`, surface
/// included. A [valid](Self::is_valid) sphere has a non-negative radius.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sphere {
    /// The center of the sphere.
    pub center: Vec3,
    /// The radius of the sphere.
    pub radius: f32,
}

/// The `f64` version of [`Sphere`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DSphere {
    /// The center of the sphere.
    pub center: DVec3,
    /// The radius of the sphere.
    pub radius: f64,
}

macro_rules! sphere_impl {
    ($name:ident, $vec3:ident, $scalar:ident, $aabb:ident, $plane:ident) => {
        impl $name {
            /// The sphere around `center` with the given `radius`.
            #[inline]
            pub const fn new(center: $vec3, radius: $scalar) -> Self {
                Self { center, radius }
            }

            /// Whether the center is finite and the radius finite and not
            /// negative.
            #[inline]
            pub const fn is_valid(self) -> bool {
                self.center.is_finite() && self.radius.is_finite() && self.radius >= 0.0
            }

            /// A sphere that contains all `points`, or `None` for an empty
            /// slice. This is Ritter's algorithm: fast and linear, but not
            /// the smallest possible sphere (typically a few percent, at
            /// worst around a fifth, larger).
            #[inline]
            pub fn from_points(points: &[$vec3]) -> Option<Self> {
                let first = *points.first()?;
                let farthest_from = |from: $vec3| {
                    points.iter().copied().fold(from, |best, point| {
                        if (point - from).length_sq() > (best - from).length_sq() {
                            point
                        } else {
                            best
                        }
                    })
                };
                let a = farthest_from(first);
                let b = farthest_from(a);
                let mut sphere = Self::new((a + b) * 0.5, (b - a).length() * 0.5);
                for &point in points {
                    sphere = sphere.union_point(point);
                }
                Some(sphere)
            }

            /// The box around the sphere.
            #[inline]
            pub fn aabb(self) -> $aabb {
                $aabb::from_center_half_extents(self.center, $vec3::splat(self.radius))
            }

            /// The volume of the sphere.
            #[inline]
            pub fn volume(self) -> $scalar {
                (4.0 / 3.0) * <$scalar as Scalar>::PI * self.radius * self.radius * self.radius
            }

            /// The area of the surface of the sphere.
            #[inline]
            pub fn surface_area(self) -> $scalar {
                4.0 * <$scalar as Scalar>::PI * self.radius * self.radius
            }

            /// The sphere moved by `offset`.
            #[inline]
            pub fn translated(self, offset: $vec3) -> Self {
                Self::new(self.center + offset, self.radius)
            }

            /// Whether `point` is inside the sphere or on its surface.
            #[inline]
            pub fn contains_point(self, point: $vec3) -> bool {
                (point - self.center).length_sq() <= self.radius * self.radius
            }

            /// Whether `other` is entirely inside this sphere (touching the
            /// surface from the inside is fine).
            #[inline]
            pub fn contains_sphere(self, other: Self) -> bool {
                (other.center - self.center).length() + other.radius <= self.radius
            }

            /// Whether the two spheres share at least one point; spheres
            /// that only touch intersect.
            #[inline]
            pub fn intersects_sphere(self, other: Self) -> bool {
                let reach = self.radius + other.radius;
                (other.center - self.center).length_sq() <= reach * reach
            }

            /// Whether the sphere and `aabb` share at least one point.
            #[inline]
            pub fn intersects_aabb(self, aabb: $aabb) -> bool {
                aabb.intersects_sphere(self)
            }

            /// Whether the sphere and `plane` share at least one point, that
            /// is, whether the plane cuts the sphere or touches it.
            #[inline]
            pub fn intersects_plane(self, plane: $plane) -> bool {
                Scalar::abs(plane.signed_distance(self.center)) <= self.radius
            }

            /// The signed distance from the surface to `point`: negative
            /// inside the sphere, positive outside.
            #[inline]
            pub fn distance_to_point(self, point: $vec3) -> $scalar {
                (point - self.center).length() - self.radius
            }

            /// The point of the sphere nearest to `point`: `point` itself
            /// when it is inside, else the point of the surface that faces
            /// it.
            #[inline]
            pub fn closest_point(self, point: $vec3) -> $vec3 {
                let offset = point - self.center;
                let distance = offset.length();
                if distance <= self.radius {
                    point
                } else {
                    self.center + offset * (self.radius / distance)
                }
            }

            /// The smallest sphere that contains this sphere and `point`.
            #[inline]
            pub fn union_point(self, point: $vec3) -> Self {
                let offset = point - self.center;
                let distance = offset.length();
                if distance <= self.radius {
                    return self;
                }
                let radius = (self.radius + distance) * 0.5;
                Self::new(
                    self.center + offset * ((radius - self.radius) / distance),
                    radius,
                )
            }

            /// The smallest sphere that contains both spheres.
            #[inline]
            pub fn union(self, other: Self) -> Self {
                let offset = other.center - self.center;
                let distance = offset.length();
                if distance + other.radius <= self.radius {
                    return self;
                }
                if distance + self.radius <= other.radius {
                    return other;
                }
                let radius = (distance + self.radius + other.radius) * 0.5;
                Self::new(
                    self.center + offset * ((radius - self.radius) / distance),
                    radius,
                )
            }
        }
    };
}

sphere_impl!(Sphere, Vec3, f32, Aabb3, Plane);
sphere_impl!(DSphere, DVec3, f64, DAabb3, DPlane);
