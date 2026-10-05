//! Building transforms (Phase 2.3): rotations, scales and translations as
//! matrices, 2D homogeneous transforms in the 3x3 matrices, conversions
//! between the sizes, view and projection matrices, and the functions that
//! apply a 4x4 transform to a point or a vector.
//!
//! The `matrices!` macro generates what is the same for every size
//! (`from_diagonal`, the element-wise operators, ...). Everything here is
//! specific to one size, so it is written once as plain Rust inside a
//! `macro_rules!` and instantiated for the `f32` and the `f64` family, the
//! same way `quat` does it. Like there, this means the `f32` and `f64`
//! versions cannot drift apart.
//!
//! # Conventions
//!
//! - **Right-handed rotations.** A positive angle turns counter-clockwise
//!   when looking down the axis toward the origin: a quarter turn about `Z`
//!   takes `X` to `Y`. `from_axis_angle`, `from_rotation_*` and `from_quat`
//!   agree with `Quat` and with `Vec2::rotate`.
//! - **TRS order.** `from_scale_rotation_translation` is `T * R * S`: a point
//!   is scaled first, then rotated, then translated.
//! - **Projection naming** follows glam. `_rh`/`_lh` is the handedness of
//!   the view space (right-handed: the camera looks down `-Z`; left-handed:
//!   down `+Z`). With no further suffix the clip-space depth is `0..1`, which
//!   is what Vulkan, Direct3D and Metal use. `_gl` is the OpenGL range
//!   `-1..1`. `_reverse_z` maps the near plane to `1` and the far plane to
//!   `0`, which spreads float depth precision much more evenly.
//! - **No Y flip.** Clip space has `+Y` up, as in OpenGL. Vulkan's clip space
//!   has `+Y` down; that is handled by the renderer (a negative viewport
//!   height), not baked into the matrix.
//! - **Degenerate input is not checked.** A zero field of view, `aspect ==
//!   0`, `z_near == z_far`, a zero-length view direction or an `up` parallel
//!   to it produce non-finite elements (`NaN` or infinity), like
//!   `normalize` of a zero vector does, instead of panicking.

use crate::angle::Radians;
use crate::matrix::{DMat2, DMat3, DMat4, Mat2, Mat3, Mat4};
use crate::quat::{DQuat, Quat};
use crate::scalar::Scalar;
use crate::vector::{DVec2, DVec3, DVec4, Vec2, Vec3, Vec4};

/// Implements the size-specific matrix constructors and transform functions
/// for one float family.
macro_rules! transforms {
    ($(
        {
            scalar: $scalar:ident,
            vec2: $vec2:ident,
            vec3: $vec3:ident,
            vec4: $vec4:ident,
            mat2: $mat2:ident,
            mat3: $mat3:ident,
            mat4: $mat4:ident,
            quat: $quat:ident,
        }
    )*) => {$(
        impl $mat2 {
            /// The 2D rotation by `angle`, counter-clockwise (clockwise for
            /// a negative angle). `angle` can be `Radians` or `Degrees`.
            ///
            /// `m * v` equals `v.rotate(angle)`.
            #[inline]
            pub fn from_angle(angle: impl Into<Radians<$scalar>>) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                Self::from_cols($vec2::new(cos, sin), $vec2::new(-sin, cos))
            }

            /// The 2D transform that scales by `scale` along the axes and
            /// then rotates by `angle`: `from_angle(angle) *
            /// from_diagonal(scale)`.
            #[inline]
            pub fn from_scale_angle(
                scale: $vec2,
                angle: impl Into<Radians<$scalar>>,
            ) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                Self::from_cols(
                    $vec2::new(cos * scale.x, sin * scale.x),
                    $vec2::new(-sin * scale.y, cos * scale.y),
                )
            }

            /// The upper left 2x2 block of a 3x3 matrix: for a 2D
            /// homogeneous transform its linear part (rotation, scale and
            /// shear), without the translation.
            #[inline]
            pub const fn from_mat3(m: $mat3) -> Self {
                Self::from_cols(
                    $vec2::new(m.x_axis.x, m.x_axis.y),
                    $vec2::new(m.y_axis.x, m.y_axis.y),
                )
            }
        }

        impl $mat3 {
            // ---- 3D linear transforms ---------------------------------

            /// The scale by `scale` along the three axes: the diagonal
            /// matrix of [`Self::from_diagonal`].
            #[inline]
            pub const fn from_scale(scale: $vec3) -> Self {
                Self::from_diagonal(scale)
            }

            /// The rotation by `angle` about the `X` axis,
            /// counter-clockwise when looking down the axis toward the
            /// origin. `angle` can be `Radians` or `Degrees`.
            #[inline]
            pub fn from_rotation_x(angle: impl Into<Radians<$scalar>>) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                Self::from_cols(
                    $vec3::new(1.0, 0.0, 0.0),
                    $vec3::new(0.0, cos, sin),
                    $vec3::new(0.0, -sin, cos),
                )
            }

            /// The rotation by `angle` about the `Y` axis, see
            /// [`Self::from_rotation_x`].
            #[inline]
            pub fn from_rotation_y(angle: impl Into<Radians<$scalar>>) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                Self::from_cols(
                    $vec3::new(cos, 0.0, -sin),
                    $vec3::new(0.0, 1.0, 0.0),
                    $vec3::new(sin, 0.0, cos),
                )
            }

            /// The rotation by `angle` about the `Z` axis, see
            /// [`Self::from_rotation_x`].
            #[inline]
            pub fn from_rotation_z(angle: impl Into<Radians<$scalar>>) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                Self::from_cols(
                    $vec3::new(cos, sin, 0.0),
                    $vec3::new(-sin, cos, 0.0),
                    $vec3::new(0.0, 0.0, 1.0),
                )
            }

            /// The rotation by `angle` about `axis` (Rodrigues' formula),
            /// counter-clockwise when looking down the axis toward the
            /// origin. `angle` can be `Radians` or `Degrees`.
            ///
            /// `axis` must be normalized, otherwise the result is not a
            /// rotation. The matrix equals the one of the quaternion with
            /// the same axis and angle.
            #[inline]
            pub fn from_axis_angle(axis: $vec3, angle: impl Into<Radians<$scalar>>) -> Self {
                let radians = angle.into().value();
                let (sin, cos) = (Scalar::sin(radians), Scalar::cos(radians));
                let t = 1.0 - cos;
                let (x, y, z) = (axis.x, axis.y, axis.z);
                Self::from_cols(
                    $vec3::new(t * x * x + cos, t * x * y + sin * z, t * x * z - sin * y),
                    $vec3::new(t * x * y - sin * z, t * y * y + cos, t * y * z + sin * x),
                    $vec3::new(t * x * z + sin * y, t * y * z - sin * x, t * z * z + cos),
                )
            }

            /// The rotation matrix of a unit quaternion, see the quaternion's
            /// `to_mat3`.
            #[inline]
            pub const fn from_quat(rotation: $quat) -> Self {
                rotation.to_mat3()
            }

            /// The inverse transpose, the matrix that transforms surface
            /// normals when `self` transforms positions (see
            /// [`Self::inverse`]), or `None` when `self` is singular.
            ///
            /// For a pure rotation (and a uniform scale, up to its length)
            /// this is the matrix itself; with a non-uniform scale it is
            /// what keeps normals perpendicular to the surface.
            #[inline]
            pub const fn inverse_transpose(self) -> Option<Self> {
                match self.inverse() {
                    Some(inverse) => Some(inverse.transpose()),
                    None => None,
                }
            }

            // ---- 2D homogeneous transforms ----------------------------
            //
            // A 2D point `(x, y)` is the column `(x, y, 1)` and a 2D
            // direction is `(x, y, 0)`; the translation is in `z_axis`.

            /// The 2D homogeneous transform that translates by
            /// `translation`.
            #[inline]
            pub const fn from_translation_2d(translation: $vec2) -> Self {
                Self::from_cols(
                    $vec3::X,
                    $vec3::Y,
                    $vec3::new(translation.x, translation.y, 1.0),
                )
            }

            /// The 2D homogeneous transform that scales by `scale`.
            #[inline]
            pub const fn from_scale_2d(scale: $vec2) -> Self {
                Self::from_diagonal($vec3::new(scale.x, scale.y, 1.0))
            }

            /// The 2D homogeneous transform that rotates by `angle`,
            /// counter-clockwise. `angle` can be `Radians` or `Degrees`.
            #[inline]
            pub fn from_angle_2d(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_mat2($mat2::from_angle(angle))
            }

            /// The 2D homogeneous transform that scales by `scale`, then
            /// rotates by `angle`, then translates by `translation`
            /// (`T * R * S`).
            #[inline]
            pub fn from_scale_angle_translation_2d(
                scale: $vec2,
                angle: impl Into<Radians<$scalar>>,
                translation: $vec2,
            ) -> Self {
                let linear = $mat2::from_scale_angle(scale, angle);
                Self::from_cols(
                    $vec3::new(linear.x_axis.x, linear.x_axis.y, 0.0),
                    $vec3::new(linear.y_axis.x, linear.y_axis.y, 0.0),
                    $vec3::new(translation.x, translation.y, 1.0),
                )
            }

            /// Transforms the 2D point `point` (as `(x, y, 1)`): the linear
            /// part is applied and the translation added. The last row is
            /// not used, so this is for affine transforms.
            #[inline]
            pub const fn transform_point2(self, point: $vec2) -> $vec2 {
                $vec2::new(
                    self.x_axis.x * point.x + self.y_axis.x * point.y + self.z_axis.x,
                    self.x_axis.y * point.x + self.y_axis.y * point.y + self.z_axis.y,
                )
            }

            /// Transforms the 2D direction `vector` (as `(x, y, 0)`): the
            /// linear part is applied, the translation is not.
            #[inline]
            pub const fn transform_vector2(self, vector: $vec2) -> $vec2 {
                $vec2::new(
                    self.x_axis.x * vector.x + self.y_axis.x * vector.y,
                    self.x_axis.y * vector.x + self.y_axis.y * vector.y,
                )
            }

            // ---- conversions ------------------------------------------

            /// A 2x2 matrix placed in the upper left corner of an otherwise
            /// identity 3x3 matrix: a 2D linear transform as a homogeneous
            /// one, without translation.
            #[inline]
            pub const fn from_mat2(m: $mat2) -> Self {
                Self::from_cols(
                    $vec3::new(m.x_axis.x, m.x_axis.y, 0.0),
                    $vec3::new(m.y_axis.x, m.y_axis.y, 0.0),
                    $vec3::Z,
                )
            }

            /// The upper left 3x3 block of a 4x4 matrix: for a 3D transform
            /// its linear part (rotation, scale and shear), without the
            /// translation and the projective row.
            #[inline]
            pub const fn from_mat4(m: $mat4) -> Self {
                Self::from_cols(m.x_axis.xyz(), m.y_axis.xyz(), m.z_axis.xyz())
            }
        }

        impl $mat4 {
            // ---- building transforms ----------------------------------

            /// The transform that translates by `translation`.
            #[inline]
            pub const fn from_translation(translation: $vec3) -> Self {
                Self::from_cols(
                    $vec4::X,
                    $vec4::Y,
                    $vec4::Z,
                    $vec4::new(translation.x, translation.y, translation.z, 1.0),
                )
            }

            /// The transform that scales by `scale` along the three axes.
            #[inline]
            pub const fn from_scale(scale: $vec3) -> Self {
                Self::from_diagonal($vec4::new(scale.x, scale.y, scale.z, 1.0))
            }

            /// The rotation by `angle` about the `X` axis,
            /// counter-clockwise when looking down the axis toward the
            /// origin. `angle` can be `Radians` or `Degrees`.
            #[inline]
            pub fn from_rotation_x(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_mat3($mat3::from_rotation_x(angle))
            }

            /// The rotation by `angle` about the `Y` axis, see
            /// [`Self::from_rotation_x`].
            #[inline]
            pub fn from_rotation_y(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_mat3($mat3::from_rotation_y(angle))
            }

            /// The rotation by `angle` about the `Z` axis, see
            /// [`Self::from_rotation_x`].
            #[inline]
            pub fn from_rotation_z(angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_mat3($mat3::from_rotation_z(angle))
            }

            /// The rotation by `angle` about `axis`, see
            /// the 3x3 `from_axis_angle`. `axis` must be normalized.
            #[inline]
            pub fn from_axis_angle(axis: $vec3, angle: impl Into<Radians<$scalar>>) -> Self {
                Self::from_mat3($mat3::from_axis_angle(axis, angle))
            }

            /// The rotation of a unit quaternion, see the quaternion's
            /// `to_mat4`.
            #[inline]
            pub const fn from_quat(rotation: $quat) -> Self {
                rotation.to_mat4()
            }

            /// A 3x3 matrix placed in the upper left corner of an otherwise
            /// identity 4x4 matrix: a linear 3D transform as a homogeneous
            /// one, without translation.
            #[inline]
            pub const fn from_mat3(m: $mat3) -> Self {
                Self::from_cols(
                    $vec4::new(m.x_axis.x, m.x_axis.y, m.x_axis.z, 0.0),
                    $vec4::new(m.y_axis.x, m.y_axis.y, m.y_axis.z, 0.0),
                    $vec4::new(m.z_axis.x, m.z_axis.y, m.z_axis.z, 0.0),
                    $vec4::W,
                )
            }

            /// The transform that scales by `scale`, then rotates by
            /// `rotation`, then translates by `translation`: `T * R * S`.
            ///
            /// `rotation` must be a unit quaternion. This is the usual model
            /// matrix of an object with a position, an orientation and a
            /// scale; [`Self::to_scale_rotation_translation`] takes it
            /// apart again.
            #[inline]
            pub const fn from_scale_rotation_translation(
                scale: $vec3,
                rotation: $quat,
                translation: $vec3,
            ) -> Self {
                let r = rotation.to_mat3();
                Self::from_cols(
                    $vec4::new(
                        r.x_axis.x * scale.x,
                        r.x_axis.y * scale.x,
                        r.x_axis.z * scale.x,
                        0.0,
                    ),
                    $vec4::new(
                        r.y_axis.x * scale.y,
                        r.y_axis.y * scale.y,
                        r.y_axis.z * scale.y,
                        0.0,
                    ),
                    $vec4::new(
                        r.z_axis.x * scale.z,
                        r.z_axis.y * scale.z,
                        r.z_axis.z * scale.z,
                        0.0,
                    ),
                    $vec4::new(translation.x, translation.y, translation.z, 1.0),
                )
            }

            /// Takes a transform apart into `(scale, rotation, translation)`,
            /// the inverse of [`Self::from_scale_rotation_translation`], or
            /// `None` when that is not possible.
            ///
            /// The matrix must be `T * R * S`: an affine transform without
            /// shear (its bottom row is ignored). The scale is the length of
            /// each column of the linear part. A mirroring transform (a
            /// negative determinant) gets its sign on the `x` scale, so that
            /// the rotation is always a proper one. `None` is returned when
            /// a column has zero or non-finite length, since the rotation of
            /// a flattened axis is undefined. With shear the result is
            /// meaningless.
            #[inline]
            pub fn to_scale_rotation_translation(self) -> Option<($vec3, $quat, $vec3)> {
                let linear = $mat3::from_mat4(self);
                let lengths = $vec3::new(
                    linear.x_axis.length(),
                    linear.y_axis.length(),
                    linear.z_axis.length(),
                );
                if !(lengths.x > 0.0 && lengths.y > 0.0 && lengths.z > 0.0 && lengths.is_finite()) {
                    return None;
                }
                let mirror = if linear.determinant() < 0.0 { -1.0 } else { 1.0 };
                let scale = $vec3::new(lengths.x * mirror, lengths.y, lengths.z);
                let rotation = $quat::from_mat3($mat3::from_cols(
                    linear.x_axis / scale.x,
                    linear.y_axis / scale.y,
                    linear.z_axis / scale.z,
                ));
                Some((scale, rotation, self.w_axis.xyz()))
            }

            // ---- applying transforms ----------------------------------

            /// Transforms the 3D point `point` (as `(x, y, z, 1)`): the
            /// linear part is applied and the translation added. The bottom
            /// row is not used and there is no division by `w`, so this is
            /// for affine transforms (use [`Self::project_point3`] for a
            /// projection).
            #[inline]
            pub const fn transform_point3(self, point: $vec3) -> $vec3 {
                $vec3::new(
                    self.x_axis.x * point.x
                        + self.y_axis.x * point.y
                        + self.z_axis.x * point.z
                        + self.w_axis.x,
                    self.x_axis.y * point.x
                        + self.y_axis.y * point.y
                        + self.z_axis.y * point.z
                        + self.w_axis.y,
                    self.x_axis.z * point.x
                        + self.y_axis.z * point.y
                        + self.z_axis.z * point.z
                        + self.w_axis.z,
                )
            }

            /// Transforms the 3D direction `vector` (as `(x, y, z, 0)`): the
            /// linear part is applied, the translation is not. Use it for
            /// directions and offsets; for surface normals use
            /// [`Self::normal_matrix`].
            #[inline]
            pub const fn transform_vector3(self, vector: $vec3) -> $vec3 {
                $vec3::new(
                    self.x_axis.x * vector.x + self.y_axis.x * vector.y + self.z_axis.x * vector.z,
                    self.x_axis.y * vector.x + self.y_axis.y * vector.y + self.z_axis.y * vector.z,
                    self.x_axis.z * vector.x + self.y_axis.z * vector.y + self.z_axis.z * vector.z,
                )
            }

            /// Transforms the 3D point `point` (as `(x, y, z, 1)`) by the
            /// whole matrix and divides by the resulting `w`: the
            /// perspective divide. With a view-projection matrix this gives
            /// the point in normalized device coordinates.
            ///
            /// A point on the camera plane has `w == 0` and gives non-finite
            /// components.
            #[inline]
            pub const fn project_point3(self, point: $vec3) -> $vec3 {
                let x = self.x_axis.x * point.x
                    + self.y_axis.x * point.y
                    + self.z_axis.x * point.z
                    + self.w_axis.x;
                let y = self.x_axis.y * point.x
                    + self.y_axis.y * point.y
                    + self.z_axis.y * point.z
                    + self.w_axis.y;
                let z = self.x_axis.z * point.x
                    + self.y_axis.z * point.y
                    + self.z_axis.z * point.z
                    + self.w_axis.z;
                let w = self.x_axis.w * point.x
                    + self.y_axis.w * point.y
                    + self.z_axis.w * point.z
                    + self.w_axis.w;
                $vec3::new(x / w, y / w, z / w)
            }

            /// The reverse of [`Self::project_point3`] for a view-projection
            /// matrix: the point whose projection is `ndc`, or `None` when
            /// the matrix is singular.
            ///
            /// It inverts the matrix on every call. To unproject many
            /// points, compute [`Self::inverse`] once and call
            /// `project_point3` on that.
            #[inline]
            pub const fn unproject_point3(self, ndc: $vec3) -> Option<$vec3> {
                match self.inverse() {
                    Some(inverse) => Some(inverse.project_point3(ndc)),
                    None => None,
                }
            }

            /// The inverse of an affine transform (its bottom row must be
            /// `(0, 0, 0, 1)`), or `None` when its linear part is singular.
            ///
            /// It only inverts the 3x3 linear part and derives the
            /// translation, which is much cheaper than the general
            /// [`Self::inverse`]. For a matrix that is not affine (a
            /// projection) the result is meaningless; use `inverse`.
            #[inline]
            pub const fn inverse_affine(self) -> Option<Self> {
                match $mat3::from_mat4(self).inverse() {
                    Some(linear) => {
                        let offset = linear.mul_vec(self.w_axis.xyz());
                        Some(Self::from_cols(
                            $vec4::new(linear.x_axis.x, linear.x_axis.y, linear.x_axis.z, 0.0),
                            $vec4::new(linear.y_axis.x, linear.y_axis.y, linear.y_axis.z, 0.0),
                            $vec4::new(linear.z_axis.x, linear.z_axis.y, linear.z_axis.z, 0.0),
                            $vec4::new(-offset.x, -offset.y, -offset.z, 1.0),
                        ))
                    }
                    None => None,
                }
            }

            /// The matrix that transforms surface normals when `self`
            /// transforms positions: the inverse transpose of the upper
            /// left 3x3 block, or `None` when that block is singular.
            ///
            /// Transforming a normal with the model matrix itself skews it
            /// as soon as the scale is not uniform; this matrix keeps it
            /// perpendicular to the surface. The result is not normalized.
            #[inline]
            pub const fn normal_matrix(self) -> Option<$mat3> {
                $mat3::from_mat4(self).inverse_transpose()
            }

            // ---- view matrices ----------------------------------------

            /// The right-handed view matrix of a camera at `eye` looking
            /// along `direction`, with `up` pointing up: it moves the world
            /// so that the camera sits at the origin looking down `-Z`.
            ///
            /// Neither `direction` nor `up` needs to be normalized, but
            /// `direction` must not be zero or parallel to `up`, otherwise
            /// the result is not finite.
            #[inline]
            pub fn look_to_rh(eye: $vec3, direction: $vec3, up: $vec3) -> Self {
                let forward = direction.normalize();
                let side = forward.cross(up).normalize();
                let up = side.cross(forward);
                Self::from_cols(
                    $vec4::new(side.x, up.x, -forward.x, 0.0),
                    $vec4::new(side.y, up.y, -forward.y, 0.0),
                    $vec4::new(side.z, up.z, -forward.z, 0.0),
                    $vec4::new(-side.dot(eye), -up.dot(eye), forward.dot(eye), 1.0),
                )
            }

            /// The left-handed view matrix of a camera at `eye` looking
            /// along `direction`, with `up` pointing up: the camera ends up
            /// at the origin looking down `+Z`. See [`Self::look_to_rh`].
            #[inline]
            pub fn look_to_lh(eye: $vec3, direction: $vec3, up: $vec3) -> Self {
                let forward = direction.normalize();
                let side = up.cross(forward).normalize();
                let up = forward.cross(side);
                Self::from_cols(
                    $vec4::new(side.x, up.x, forward.x, 0.0),
                    $vec4::new(side.y, up.y, forward.y, 0.0),
                    $vec4::new(side.z, up.z, forward.z, 0.0),
                    $vec4::new(-side.dot(eye), -up.dot(eye), -forward.dot(eye), 1.0),
                )
            }

            /// The right-handed view matrix of a camera at `eye` looking at
            /// the point `center`, see [`Self::look_to_rh`]. `eye` and
            /// `center` must differ.
            #[inline]
            pub fn look_at_rh(eye: $vec3, center: $vec3, up: $vec3) -> Self {
                Self::look_to_rh(eye, center - eye, up)
            }

            /// The left-handed view matrix of a camera at `eye` looking at
            /// the point `center`, see [`Self::look_to_lh`]. `eye` and
            /// `center` must differ.
            #[inline]
            pub fn look_at_lh(eye: $vec3, center: $vec3, up: $vec3) -> Self {
                Self::look_to_lh(eye, center - eye, up)
            }

            // ---- perspective projections ------------------------------

            /// `1 / tan(fov / 2)`, the focal length of a perspective
            /// projection with the vertical field of view `fov_y`.
            #[inline]
            fn focal_length(fov_y: impl Into<Radians<$scalar>>) -> $scalar {
                let half = fov_y.into().value() * 0.5;
                Scalar::cos(half) / Scalar::sin(half)
            }

            /// The right-handed perspective projection with clip-space
            /// depth `0..1` (Vulkan, Direct3D, Metal): view-space `z =
            /// -z_near` maps to depth `0` and `z = -z_far` to `1`.
            ///
            /// `fov_y` is the vertical field of view (`Radians` or
            /// `Degrees`), `aspect_ratio` is width divided by height, and
            /// both planes are positive distances in front of the camera.
            #[inline]
            pub fn perspective_rh(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let h = Self::focal_length(fov_y);
                let w = h / aspect_ratio;
                let r = z_far / (z_near - z_far);
                Self::from_cols(
                    $vec4::new(w, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, h, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, r, -1.0),
                    $vec4::new(0.0, 0.0, r * z_near, 0.0),
                )
            }

            /// The left-handed perspective projection with clip-space depth
            /// `0..1`: view-space `z = z_near` maps to depth `0` and
            /// `z = z_far` to `1`. See [`Self::perspective_rh`].
            #[inline]
            pub fn perspective_lh(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let h = Self::focal_length(fov_y);
                let w = h / aspect_ratio;
                let r = z_far / (z_far - z_near);
                Self::from_cols(
                    $vec4::new(w, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, h, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, r, 1.0),
                    $vec4::new(0.0, 0.0, -r * z_near, 0.0),
                )
            }

            /// The right-handed perspective projection with the OpenGL
            /// clip-space depth `-1..1`. See [`Self::perspective_rh`].
            #[inline]
            pub fn perspective_rh_gl(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let h = Self::focal_length(fov_y);
                let w = h / aspect_ratio;
                let a = (z_far + z_near) / (z_near - z_far);
                let b = 2.0 * z_far * z_near / (z_near - z_far);
                Self::from_cols(
                    $vec4::new(w, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, h, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, a, -1.0),
                    $vec4::new(0.0, 0.0, b, 0.0),
                )
            }

            /// The left-handed perspective projection with the OpenGL
            /// clip-space depth `-1..1`. See [`Self::perspective_lh`].
            #[inline]
            pub fn perspective_lh_gl(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let h = Self::focal_length(fov_y);
                let w = h / aspect_ratio;
                let a = (z_far + z_near) / (z_far - z_near);
                let b = -2.0 * z_far * z_near / (z_far - z_near);
                Self::from_cols(
                    $vec4::new(w, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, h, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, a, 1.0),
                    $vec4::new(0.0, 0.0, b, 0.0),
                )
            }

            /// [`Self::perspective_rh`] with reversed depth: the near plane
            /// maps to depth `1` and the far plane to `0`.
            ///
            /// Floating-point depth is densest near `0`, which a normal
            /// projection wastes on the near plane; reversing it puts that
            /// precision where perspective needs it. Use it together with a
            /// depth clear value of `0` and a `GREATER` depth test.
            #[inline]
            pub fn perspective_rh_reverse_z(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                Self::perspective_rh(fov_y, aspect_ratio, z_far, z_near)
            }

            /// [`Self::perspective_lh`] with reversed depth: the near plane
            /// maps to depth `1` and the far plane to `0`. See
            /// [`Self::perspective_rh_reverse_z`].
            #[inline]
            pub fn perspective_lh_reverse_z(
                fov_y: impl Into<Radians<$scalar>>,
                aspect_ratio: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                Self::perspective_lh(fov_y, aspect_ratio, z_far, z_near)
            }

            // ---- orthographic projections -----------------------------

            /// The right-handed orthographic projection of the box
            /// `left..right`, `bottom..top` (view-space `x` and `y`) and
            /// `-z_near..-z_far` (depth), with clip-space depth `0..1`.
            ///
            /// `z_near` and `z_far` are positive distances in front of the
            /// camera.
            #[inline]
            pub const fn orthographic_rh(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let rcp_width = 1.0 / (right - left);
                let rcp_height = 1.0 / (top - bottom);
                let r = 1.0 / (z_near - z_far);
                Self::from_cols(
                    $vec4::new(2.0 * rcp_width, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, 2.0 * rcp_height, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, r, 0.0),
                    $vec4::new(
                        -(left + right) * rcp_width,
                        -(top + bottom) * rcp_height,
                        r * z_near,
                        1.0,
                    ),
                )
            }

            /// The left-handed orthographic projection of the box
            /// `left..right`, `bottom..top` and `z_near..z_far`, with
            /// clip-space depth `0..1`. See [`Self::orthographic_rh`].
            #[inline]
            pub const fn orthographic_lh(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let rcp_width = 1.0 / (right - left);
                let rcp_height = 1.0 / (top - bottom);
                let r = 1.0 / (z_far - z_near);
                Self::from_cols(
                    $vec4::new(2.0 * rcp_width, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, 2.0 * rcp_height, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, r, 0.0),
                    $vec4::new(
                        -(left + right) * rcp_width,
                        -(top + bottom) * rcp_height,
                        -r * z_near,
                        1.0,
                    ),
                )
            }

            /// The right-handed orthographic projection with the OpenGL
            /// clip-space depth `-1..1`. See [`Self::orthographic_rh`].
            #[inline]
            pub const fn orthographic_rh_gl(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let rcp_width = 1.0 / (right - left);
                let rcp_height = 1.0 / (top - bottom);
                let rcp_depth = 1.0 / (z_near - z_far);
                Self::from_cols(
                    $vec4::new(2.0 * rcp_width, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, 2.0 * rcp_height, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, 2.0 * rcp_depth, 0.0),
                    $vec4::new(
                        -(left + right) * rcp_width,
                        -(top + bottom) * rcp_height,
                        (z_far + z_near) * rcp_depth,
                        1.0,
                    ),
                )
            }

            /// The left-handed orthographic projection with the OpenGL
            /// clip-space depth `-1..1`. See [`Self::orthographic_lh`].
            #[inline]
            pub const fn orthographic_lh_gl(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                let rcp_width = 1.0 / (right - left);
                let rcp_height = 1.0 / (top - bottom);
                let rcp_depth = 1.0 / (z_far - z_near);
                Self::from_cols(
                    $vec4::new(2.0 * rcp_width, 0.0, 0.0, 0.0),
                    $vec4::new(0.0, 2.0 * rcp_height, 0.0, 0.0),
                    $vec4::new(0.0, 0.0, 2.0 * rcp_depth, 0.0),
                    $vec4::new(
                        -(left + right) * rcp_width,
                        -(top + bottom) * rcp_height,
                        -(z_far + z_near) * rcp_depth,
                        1.0,
                    ),
                )
            }

            /// [`Self::orthographic_rh`] with reversed depth: the near plane
            /// maps to depth `1` and the far plane to `0`. See
            /// [`Self::perspective_rh_reverse_z`].
            #[inline]
            pub const fn orthographic_rh_reverse_z(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                Self::orthographic_rh(left, right, bottom, top, z_far, z_near)
            }

            /// [`Self::orthographic_lh`] with reversed depth: the near plane
            /// maps to depth `1` and the far plane to `0`. See
            /// [`Self::perspective_rh_reverse_z`].
            #[inline]
            pub const fn orthographic_lh_reverse_z(
                left: $scalar,
                right: $scalar,
                bottom: $scalar,
                top: $scalar,
                z_near: $scalar,
                z_far: $scalar,
            ) -> Self {
                Self::orthographic_lh(left, right, bottom, top, z_far, z_near)
            }
        }
    )*};
}

transforms! {
    {
        scalar: f32,
        vec2: Vec2,
        vec3: Vec3,
        vec4: Vec4,
        mat2: Mat2,
        mat3: Mat3,
        mat4: Mat4,
        quat: Quat,
    }
    {
        scalar: f64,
        vec2: DVec2,
        vec3: DVec3,
        vec4: DVec4,
        mat2: DMat2,
        mat3: DMat3,
        mat4: DMat4,
        quat: DQuat,
    }
}
