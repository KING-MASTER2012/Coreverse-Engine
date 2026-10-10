//! The view frustum for culling: [`Frustum`] and [`DFrustum`]. See the
//! [module documentation](super) for the conventions.

use super::Containment;
use super::aabb::{Aabb3, DAabb3};
use super::plane::{DPlane, Plane};
use super::sphere::{DSphere, Sphere};
use crate::matrix::{DMat4, Mat4};
use crate::vector::{DVec3, DVec4, Vec3, Vec4};

/// Six planes that bound a convex view volume, normals pointing inwards: a
/// point is inside when its signed distance to every plane is non-negative.
///
/// The planes are in the order `LEFT`, `RIGHT`, `BOTTOM`, `TOP`, `NEAR`,
/// `FAR` (the constants of the type are their indices). Built from a
/// view-projection matrix they are in the space the matrix takes its input
/// from (world space for `projection * view`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frustum {
    /// The six planes, see the type documentation for their order.
    pub planes: [Plane; 6],
}

/// The `f64` version of [`Frustum`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DFrustum {
    /// The six planes, see the type documentation of [`Frustum`] for their
    /// order.
    pub planes: [DPlane; 6],
}

macro_rules! frustum_impl {
    (
        $name:ident, $vec3:ident, $vec4:ident, $mat4:ident, $scalar:ident,
        $plane:ident, $aabb:ident, $sphere:ident
    ) => {
        impl $name {
            /// Index of the left plane in `planes`.
            pub const LEFT: usize = 0;
            /// Index of the right plane in `planes`.
            pub const RIGHT: usize = 1;
            /// Index of the bottom plane in `planes`.
            pub const BOTTOM: usize = 2;
            /// Index of the top plane in `planes`.
            pub const TOP: usize = 3;
            /// Index of the near plane in `planes`.
            pub const NEAR: usize = 4;
            /// Index of the far plane in `planes`.
            pub const FAR: usize = 5;

            /// The frustum with the given six planes (inward normals, in
            /// the order of the type documentation).
            #[inline]
            pub const fn from_planes(planes: [$plane; 6]) -> Self {
                Self { planes }
            }

            /// The frustum of a view-projection matrix (`projection *
            /// view`, column-major) for the depth range `0..=1` of Vulkan,
            /// Direct3D and Metal (`Mat4::perspective_rh` and friends, and
            /// the orthographic ones). The planes are normalized.
            ///
            /// This is the Gribb-Hartmann extraction: every plane is a sum
            /// or difference of rows of the matrix. It works for
            /// perspective and orthographic projections of either
            /// handedness, and for a reverse-Z projection too, where the
            /// plane at [`Self::NEAR`] is then the far plane and the one at
            /// [`Self::FAR`] the near plane (culling is not affected). A
            /// degenerate matrix gives planes that are not finite
            /// ([`Self::is_finite`]).
            #[inline]
            pub fn from_view_projection(view_projection: $mat4) -> Self {
                Self::from_rows(view_projection, view_projection.row(2))
            }

            /// [`Self::from_view_projection`] for the OpenGL depth range
            /// `-1..=1` (`Mat4::perspective_rh_gl` and friends).
            #[inline]
            pub fn from_view_projection_gl(view_projection: $mat4) -> Self {
                Self::from_rows(
                    view_projection,
                    view_projection.row(3) + view_projection.row(2),
                )
            }

            /// The planes from the rows of the matrix, `near` being the
            /// row that bounds the near depth.
            fn from_rows(view_projection: $mat4, near: $vec4) -> Self {
                let (x, y, w) = (
                    view_projection.row(0),
                    view_projection.row(1),
                    view_projection.row(3),
                );
                let z = view_projection.row(2);
                Self {
                    planes: [
                        Self::plane_from_row(w + x),
                        Self::plane_from_row(w - x),
                        Self::plane_from_row(w + y),
                        Self::plane_from_row(w - y),
                        Self::plane_from_row(near),
                        Self::plane_from_row(w - z),
                    ],
                }
            }

            /// The normalized plane `row . (x, y, z, 1) >= 0`.
            fn plane_from_row(row: $vec4) -> $plane {
                $plane::from_coefficients(row.x, row.y, row.z, row.w)
                    .unwrap_or($plane::new($vec3::splat(<$scalar>::NAN), <$scalar>::NAN))
            }

            /// Whether all six planes are finite.
            #[inline]
            pub fn is_finite(self) -> bool {
                self.planes.iter().all(|plane| plane.is_finite())
            }

            /// The frustum after the transform `matrix` has been applied to
            /// every point of it (see [`Plane::transformed`]), or `None`
            /// when `matrix` is singular. A world-space frustum moved into
            /// the space of a model, for example, takes the inverse of the
            /// model matrix.
            #[inline]
            pub fn transformed(self, matrix: $mat4) -> Option<Self> {
                let mut planes = self.planes;
                for plane in &mut planes {
                    *plane = plane.transformed(matrix)?;
                }
                Some(Self { planes })
            }

            /// Whether `point` is inside the frustum or on its boundary.
            #[inline]
            pub fn contains_point(self, point: $vec3) -> bool {
                self.planes
                    .iter()
                    .all(|plane| plane.signed_distance(point) >= 0.0)
            }

            /// Where `sphere` lies relative to the frustum. `Outside` is
            /// exact for every sphere that is behind a single plane; a
            /// sphere near an edge or corner of the frustum can be reported
            /// as `Intersecting` although it is outside, which is the usual
            /// price of culling with planes.
            #[inline]
            pub fn classify_sphere(self, sphere: $sphere) -> Containment {
                self.classify(|plane| (plane.signed_distance(sphere.center), sphere.radius))
            }

            /// Whether `sphere` is at least partly inside the frustum,
            /// with the conservative accuracy of [`Self::classify_sphere`].
            #[inline]
            pub fn intersects_sphere(self, sphere: $sphere) -> bool {
                self.classify_sphere(sphere).is_visible()
            }

            /// Where `aabb` lies relative to the frustum. `Outside` is
            /// exact for every box that is behind a single plane; a box
            /// near an edge or corner of the frustum can be reported as
            /// `Intersecting` although it is outside. An invalid box
            /// (including `EMPTY`) is `Outside`.
            #[inline]
            pub fn classify_aabb(self, aabb: $aabb) -> Containment {
                if !aabb.is_valid() {
                    return Containment::Outside;
                }
                let center = aabb.center();
                let half = aabb.half_extents();
                self.classify(|plane| {
                    // The distance from the center to the corner that
                    // sticks out furthest along the plane normal.
                    (plane.signed_distance(center), half.dot(plane.normal.abs()))
                })
            }

            /// Whether `aabb` is at least partly inside the frustum, with
            /// the conservative accuracy of [`Self::classify_aabb`]. This is
            /// the usual visibility test of a bounding box.
            #[inline]
            pub fn intersects_aabb(self, aabb: $aabb) -> bool {
                self.classify_aabb(aabb).is_visible()
            }

            /// Classifies a shape from, per plane, the signed distance of
            /// its center and its reach (the radius, or the extent of a box
            /// along the normal).
            fn classify(self, shape: impl Fn(&$plane) -> ($scalar, $scalar)) -> Containment {
                let mut result = Containment::Inside;
                for plane in &self.planes {
                    let (distance, reach) = shape(plane);
                    if distance < -reach {
                        return Containment::Outside;
                    }
                    if distance < reach {
                        result = Containment::Intersecting;
                    }
                }
                result
            }
        }
    };
}

frustum_impl!(Frustum, Vec3, Vec4, Mat4, f32, Plane, Aabb3, Sphere);
frustum_impl!(DFrustum, DVec3, DVec4, DMat4, f64, DPlane, DAabb3, DSphere);
