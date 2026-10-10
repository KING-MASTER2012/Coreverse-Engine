//! Geometric primitives for culling, picking and physics (Phase 2.5):
//! axis-aligned boxes ([`Aabb2`], [`Aabb3`]), [`Ray`], [`Plane`], [`Sphere`],
//! the view [`Frustum`] and the [`Transform`] (translation, rotation, scale)
//! struct, each in an `f32` and a `D`-prefixed `f64` version, with the
//! intersection tests between them.
//!
//! # Conventions
//!
//! - **Closed shapes.** Boxes and spheres include their surface: a point on
//!   the surface is contained, and two shapes that merely touch intersect.
//! - **Planes** are `normal . p + d = 0` with a unit `normal`; the signed
//!   distance of a point is positive on the side the normal points to. The
//!   [`Frustum`] planes point inwards, so a point is inside when its
//!   distance to all six is non-negative.
//! - **Invalid shapes.** `Aabb::EMPTY` (`min = +inf`, `max = -inf`) is the
//!   neutral element of `union`; it contains nothing and intersects
//!   nothing. A box with `min > max` on some axis is not [valid]
//!   (`Aabb3::is_valid`), and the intersection tests treat it like `EMPTY`.
//! - **Right-handed, column-major**, like the rest of the crate, so
//!   [`Transform::to_mat4`] is the `T * R * S` of `Mat4`.
//!
//! # Example
//!
//! ```
//! use math::{Aabb3, Ray, Vec3};
//!
//! let aabb = Aabb3::new(Vec3::splat(-1.0), Vec3::splat(1.0));
//! let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);
//!
//! // The ray enters the box at t = 4 and leaves it at t = 6.
//! assert_eq!(ray.intersect_aabb(aabb), Some((4.0, 6.0)));
//! assert!(aabb.contains_point(ray.at(5.0)));
//! ```
//!
//! [valid]: Aabb3::is_valid

mod aabb;
mod frustum;
mod plane;
mod ray;
mod sphere;
mod transform;

#[cfg(test)]
mod aabb_tests;
#[cfg(test)]
mod frustum_tests;
#[cfg(test)]
mod plane_tests;
#[cfg(test)]
mod ray_tests;
#[cfg(all(test, feature = "serde", feature = "std"))]
mod serde_tests;
#[cfg(test)]
mod sphere_tests;
#[cfg(test)]
mod transform_tests;

pub use aabb::{Aabb2, Aabb3, DAabb2, DAabb3};
pub use frustum::{DFrustum, Frustum};
pub use plane::{DPlane, Plane};
pub use ray::{DRay, Ray};
pub use sphere::{DSphere, Sphere};
pub use transform::{DTransform, Transform};

/// How a shape lies relative to a convex volume such as a [`Frustum`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Containment {
    /// Entirely outside the volume.
    Outside,
    /// Crosses the boundary of the volume (or, for a frustum test, may do:
    /// see [`Frustum::classify_aabb`]).
    Intersecting,
    /// Entirely inside the volume.
    Inside,
}

impl Containment {
    /// Whether any part of the shape can be inside the volume: everything
    /// except [`Containment::Outside`].
    #[inline]
    pub const fn is_visible(self) -> bool {
        !matches!(self, Self::Outside)
    }
}
