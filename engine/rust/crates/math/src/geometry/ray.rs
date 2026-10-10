//! Rays and their intersection tests: [`Ray`] and [`DRay`]. See the
//! [module documentation](super) for the conventions.

use super::aabb::{Aabb3, DAabb3};
use super::plane::{DPlane, Plane};
use super::sphere::{DSphere, Sphere};
use crate::matrix::{DMat4, Mat4};
use crate::scalar::Scalar;
use crate::vector::{DVec2, DVec3, Vec2, Vec3};

/// A half-line: the points `origin + t * direction` for `t >= 0`.
///
/// The intersection functions return the parameter `t`, which is a distance
/// in length units only when `direction` is a unit vector (as it is for rays
/// made by [`Self::from_points`], [`Self::from_ndc`] and the like).
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ray {
    /// Where the ray starts.
    pub origin: Vec3,
    /// The direction of the ray; unit length for `t` to be a distance.
    pub direction: Vec3,
}

/// The `f64` version of [`Ray`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DRay {
    /// Where the ray starts.
    pub origin: DVec3,
    /// The direction of the ray; unit length for `t` to be a distance.
    pub direction: DVec3,
}

macro_rules! ray_impl {
    (
        $name:ident, $vec2:ident, $vec3:ident, $mat4:ident, $scalar:ident,
        $aabb:ident, $sphere:ident, $plane:ident
    ) => {
        impl $name {
            /// The ray from `origin` along `direction`. `direction` is kept
            /// as given; see the type documentation.
            #[inline]
            pub const fn new(origin: $vec3, direction: $vec3) -> Self {
                Self { origin, direction }
            }

            /// The ray from `origin` towards `target`, with a unit
            /// direction. `None` when the two points coincide or are not
            /// finite.
            #[inline]
            pub fn from_points(origin: $vec3, target: $vec3) -> Option<Self> {
                Some(Self {
                    origin,
                    direction: (target - origin).try_normalize()?,
                })
            }

            /// The ray that a view-projection matrix sends to the point
            /// `ndc` of the screen, for the depth range `0..=1` of
            /// Vulkan, Direct3D and Metal (`Mat4::perspective_rh` and
            /// friends): it starts on the near plane and has a unit
            /// direction. `ndc` is in normalized device coordinates, `x`
            /// and `y` in `-1..=1`.
            ///
            /// `inverse_view_projection` is the inverse of `projection *
            /// view`. `None` when the point cannot be unprojected (a
            /// singular matrix).
            #[inline]
            pub fn from_ndc(inverse_view_projection: $mat4, ndc: $vec2) -> Option<Self> {
                Self::through_depths(inverse_view_projection, ndc, 0.0, 1.0)
            }

            /// [`Self::from_ndc`] for the OpenGL depth range `-1..=1`
            /// (`Mat4::perspective_rh_gl` and friends).
            #[inline]
            pub fn from_ndc_gl(inverse_view_projection: $mat4, ndc: $vec2) -> Option<Self> {
                Self::through_depths(inverse_view_projection, ndc, -1.0, 1.0)
            }

            /// [`Self::from_ndc`] for a reverse-Z projection
            /// (`Mat4::perspective_rh_reverse_z`), where the near plane is
            /// at depth `1` and the far plane at depth `0`.
            #[inline]
            pub fn from_ndc_reverse_z(inverse_view_projection: $mat4, ndc: $vec2) -> Option<Self> {
                Self::through_depths(inverse_view_projection, ndc, 1.0, 0.0)
            }

            /// The ray from the unprojected `ndc` at depth `near_depth`
            /// towards the one at `far_depth`.
            fn through_depths(
                inverse_view_projection: $mat4,
                ndc: $vec2,
                near_depth: $scalar,
                far_depth: $scalar,
            ) -> Option<Self> {
                let near =
                    inverse_view_projection.project_point3($vec3::new(ndc.x, ndc.y, near_depth));
                let far =
                    inverse_view_projection.project_point3($vec3::new(ndc.x, ndc.y, far_depth));
                Self::from_points(near, far)
            }

            /// Whether the origin and the direction are finite and the
            /// direction is not zero. The intersection tests report no hit
            /// for a ray that is not valid.
            #[inline]
            pub fn is_valid(self) -> bool {
                self.origin.is_finite()
                    && self.direction.is_finite()
                    && self.direction != $vec3::ZERO
            }

            /// The point `origin + t * direction`.
            #[inline]
            pub fn at(self, t: $scalar) -> $vec3 {
                self.origin + self.direction * t
            }

            /// The ray after `matrix` (an affine transform) has moved its
            /// origin and turned its direction. The direction is not
            /// normalized again, so `t` keeps meaning the same point of the
            /// ray: useful to intersect a world-space ray with a model-space
            /// shape by passing the inverse of the model matrix.
            #[inline]
            pub fn transformed(self, matrix: $mat4) -> Self {
                Self {
                    origin: matrix.transform_point3(self.origin),
                    direction: matrix.transform_vector3(self.direction),
                }
            }

            /// Where the ray passes through `aabb`: the parameters
            /// `(near, far)` at which it enters and leaves the box, with
            /// `0 <= near <= far`. `near` is `0` when the origin is inside
            /// the box, and boxes behind the origin are not hit.
            ///
            /// A ray parallel to a face hits when it lies between the two
            /// planes of that axis (inclusive). `None` for a miss, for an
            /// invalid ray and for a box that is not
            /// [valid](Aabb3::is_valid).
            #[inline]
            pub fn intersect_aabb(self, aabb: $aabb) -> Option<($scalar, $scalar)> {
                if !self.is_valid() || !aabb.is_valid() {
                    return None;
                }
                let origin = self.origin.to_array();
                let direction = self.direction.to_array();
                let low = aabb.min.to_array();
                let high = aabb.max.to_array();
                let mut near: $scalar = 0.0;
                let mut far = <$scalar>::INFINITY;
                for (((&o, &d), &lo), &hi) in origin
                    .iter()
                    .zip(direction.iter())
                    .zip(low.iter())
                    .zip(high.iter())
                {
                    if d == 0.0 {
                        if !(lo..=hi).contains(&o) {
                            return None;
                        }
                    } else {
                        let t1 = (lo - o) / d;
                        let t2 = (hi - o) / d;
                        let (enter, leave) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                        near = Scalar::max(near, enter);
                        far = Scalar::min(far, leave);
                        if near > far {
                            return None;
                        }
                    }
                }
                Some((near, far))
            }

            /// Where the ray passes through `sphere`: the parameters
            /// `(near, far)` at which it enters and leaves, with
            /// `0 <= near <= far`. `near` is `0` when the origin is inside
            /// the sphere, and spheres behind the origin are not hit.
            /// `None` for a miss, for an invalid ray and for a sphere with a
            /// negative radius.
            #[inline]
            pub fn intersect_sphere(self, sphere: $sphere) -> Option<($scalar, $scalar)> {
                if !self.is_valid() || sphere.radius.is_nan() || sphere.radius < 0.0 {
                    return None;
                }
                let offset = self.origin - sphere.center;
                let a = self.direction.dot(self.direction);
                let half_b = offset.dot(self.direction);
                let c = offset.dot(offset) - sphere.radius * sphere.radius;
                let discriminant = half_b * half_b - a * c;
                if discriminant < 0.0 {
                    return None;
                }
                let root = Scalar::sqrt(discriminant);
                let near = (-half_b - root) / a;
                let far = (-half_b + root) / a;
                if far < 0.0 {
                    return None;
                }
                Some((Scalar::max(near, 0.0), far))
            }

            /// The parameter `t >= 0` at which the ray crosses `plane`, from
            /// either side. `None` when the plane is behind the origin, when
            /// the ray is parallel to it (even if it lies in the plane), and
            /// for an invalid ray.
            #[inline]
            pub fn intersect_plane(self, plane: $plane) -> Option<$scalar> {
                if !self.is_valid() {
                    return None;
                }
                let denominator = plane.normal.dot(self.direction);
                if denominator == 0.0 {
                    return None;
                }
                let t = -plane.signed_distance(self.origin) / denominator;
                (t >= 0.0).then_some(t)
            }
        }
    };
}

ray_impl!(Ray, Vec2, Vec3, Mat4, f32, Aabb3, Sphere, Plane);
ray_impl!(DRay, DVec2, DVec3, DMat4, f64, DAabb3, DSphere, DPlane);
