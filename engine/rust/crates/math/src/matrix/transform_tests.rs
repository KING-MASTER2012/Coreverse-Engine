//! Tests for the matrix additions of Phase 2.3: `from_diagonal` and the
//! element-wise operators, 2D and 3D transform construction, 2D homogeneous
//! transforms, the conversions between sizes, applying a 4x4 transform,
//! `inverse_affine`, the normal matrix, TRS composition and decomposition,
//! view matrices, and the perspective and orthographic projections.
//!
//! Everything is instantiated once per float family (`Mat*` with `f32` and
//! `DMat*` with `f64`). Values are checked against the quaternion types and
//! against plain matrix products wherever those give an independent answer.

use crate::angle::{Degrees, Radians};
use crate::quat::{DQuat, Quat};
use crate::scalar::Scalar;
use crate::vector::{DVec2, DVec3, DVec4, Vec2, Vec3, Vec4};

use super::*;

macro_rules! float_tests {
    (
        $module:ident, $scalar:ident,
        $v2:ident, $v3:ident, $v4:ident,
        $m2:ident, $m3:ident, $m4:ident,
        $quat:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-5;

            fn close(a: $scalar, b: $scalar) -> bool {
                (a - b).abs() <= TOL * (1.0 + a.abs().max(b.abs()))
            }

            macro_rules! assert_vec_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a.to_array(), $b.to_array());
                    assert!(
                        a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y)),
                        "{:?} is not close to {:?}",
                        a,
                        b
                    );
                }};
            }

            macro_rules! assert_mat_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a.to_cols_array(), $b.to_cols_array());
                    assert!(
                        a.iter().zip(b.iter()).all(|(x, y)| close(*x, *y)),
                        "{:?} is not close to {:?}",
                        a,
                        b
                    );
                }};
            }

            fn deg(degrees: $scalar) -> Degrees<$scalar> {
                Degrees::new(degrees)
            }

            fn tan(radians: $scalar) -> $scalar {
                <$scalar as Scalar>::tan(radians)
            }

            /// A rotation about an oblique axis.
            fn rotation() -> $quat {
                $quat::from_axis_angle($v3::new(1.0, 2.0, 3.0).normalize(), deg(40.0))
            }

            /// A model matrix with a non-uniform scale, that rotation and a
            /// translation.
            fn model() -> $m4 {
                $m4::from_scale_rotation_translation(
                    $v3::new(2.0, 3.0, 0.5),
                    rotation(),
                    $v3::new(1.0, -2.0, 3.0),
                )
            }

            // ---- from_diagonal and the element-wise operators ----------

            #[test]
            fn from_diagonal_puts_the_vector_on_the_diagonal() {
                assert_eq!(
                    $m2::from_diagonal($v2::new(2.0, 3.0)).to_cols_array(),
                    [2.0, 0.0, 0.0, 3.0]
                );
                assert_eq!(
                    $m3::from_diagonal($v3::new(1.0, 2.0, 3.0)).to_cols_array(),
                    [1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]
                );
                let m = $m4::from_diagonal($v4::new(1.0, 2.0, 3.0, 4.0));
                assert_eq!(m.col(2), $v4::new(0.0, 0.0, 3.0, 0.0));
                assert_eq!($m4::from_diagonal($v4::ONE), $m4::IDENTITY);
                assert_eq!($m3::from_diagonal($v3::ZERO), $m3::ZERO);
                // It scales: `m * v` is the component-wise product.
                assert_eq!(
                    $m3::from_diagonal($v3::new(2.0, 3.0, 4.0)) * $v3::new(1.0, 1.0, 2.0),
                    $v3::new(2.0, 3.0, 8.0)
                );
            }

            #[test]
            fn element_wise_operators() {
                let a = $m3::from_cols_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
                let b = $m3::from_cols_array([9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0]);
                assert_eq!((a + b).to_cols_array(), [10.0; 9]);
                assert_eq!(
                    (a - b).to_cols_array(),
                    [-8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0]
                );
                assert_eq!(
                    (-a).to_cols_array(),
                    [-1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0, -8.0, -9.0]
                );
                assert_eq!(
                    (a * 2.0).to_cols_array(),
                    [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0]
                );
                assert_eq!(2.0 * a, a * 2.0);
                assert_eq!((a * 2.0) / 2.0, a);
                assert_eq!(a - a, $m3::ZERO);
                assert_eq!(a + $m3::ZERO, a);
            }

            #[test]
            fn element_wise_assign_operators() {
                let a = $m2::from_cols_array([1.0, 2.0, 3.0, 4.0]);
                let mut m = a;
                m += a;
                assert_eq!(m, a * 2.0);
                m -= a;
                assert_eq!(m, a);
                m *= 3.0;
                assert_eq!(m.to_cols_array(), [3.0, 6.0, 9.0, 12.0]);
                m /= 3.0;
                assert_eq!(m, a);
            }

            #[test]
            fn operators_work_for_every_size() {
                assert_eq!($m2::IDENTITY + $m2::IDENTITY, $m2::IDENTITY * 2.0);
                assert_eq!(-$m4::IDENTITY + $m4::IDENTITY, $m4::ZERO);
                assert_eq!($m4::IDENTITY * 0.5 * 2.0, $m4::IDENTITY);
                // The scalar forms must not get in the way of the products.
                assert_eq!($m3::IDENTITY * $v3::ONE, $v3::ONE);
                assert_eq!($m3::IDENTITY * $m3::IDENTITY, $m3::IDENTITY);
            }

            // ---- 2x2 rotations -----------------------------------------

            #[test]
            fn mat2_rotation_turns_counter_clockwise() {
                assert_vec_close!($m2::from_angle(deg(90.0)) * $v2::X, $v2::Y);
                assert_vec_close!($m2::from_angle(deg(90.0)) * $v2::Y, -$v2::X);
                assert_vec_close!($m2::from_angle(deg(-90.0)) * $v2::X, -$v2::Y);
                assert_mat_close!($m2::from_angle(deg(0.0)), $m2::IDENTITY);
                assert!(close($m2::from_angle(deg(33.0)).determinant(), 1.0));
            }

            #[test]
            fn mat2_rotation_matches_vector_rotate() {
                for degrees in [0.0, 17.0, 90.0, 135.0, 200.0, -45.0] {
                    let v = $v2::new(3.0, -4.0);
                    assert_vec_close!($m2::from_angle(deg(degrees)) * v, v.rotate(deg(degrees)));
                }
                let radians = Radians::<$scalar>::new(1.25);
                assert_mat_close!(
                    $m2::from_angle(radians),
                    $m2::from_angle(Degrees::from(radians))
                );
            }

            #[test]
            fn mat2_scale_angle_scales_first_then_rotates() {
                let scale = $v2::new(2.0, 3.0);
                let m = $m2::from_scale_angle(scale, deg(90.0));
                let v = $v2::new(1.0, 1.0);
                assert_vec_close!(m * v, $m2::from_angle(deg(90.0)) * (scale * v));
                assert_mat_close!(m, $m2::from_angle(deg(90.0)) * $m2::from_diagonal(scale));
                assert_mat_close!(
                    $m2::from_scale_angle($v2::ONE, deg(30.0)),
                    $m2::from_angle(deg(30.0))
                );
            }

            // ---- 3x3 rotations -----------------------------------------

            #[test]
            fn mat3_axis_rotations_follow_the_right_hand_rule() {
                assert_vec_close!($m3::from_rotation_z(deg(90.0)) * $v3::X, $v3::Y);
                assert_vec_close!($m3::from_rotation_x(deg(90.0)) * $v3::Y, $v3::Z);
                assert_vec_close!($m3::from_rotation_y(deg(90.0)) * $v3::Z, $v3::X);
                assert_vec_close!($m3::from_rotation_z(deg(90.0)) * $v3::Y, -$v3::X);
                // The rotation axis itself is untouched.
                assert_vec_close!($m3::from_rotation_x(deg(77.0)) * $v3::X, $v3::X);
                assert_vec_close!($m3::from_rotation_y(deg(77.0)) * $v3::Y, $v3::Y);
                assert_vec_close!($m3::from_rotation_z(deg(77.0)) * $v3::Z, $v3::Z);
            }

            #[test]
            fn mat3_rotations_match_the_quaternion() {
                for degrees in [0.0, 30.0, 90.0, 123.0, 270.0, -45.0] {
                    let angle = deg(degrees);
                    assert_mat_close!(
                        $m3::from_rotation_x(angle),
                        $quat::from_rotation_x(angle).to_mat3()
                    );
                    assert_mat_close!(
                        $m3::from_rotation_y(angle),
                        $quat::from_rotation_y(angle).to_mat3()
                    );
                    assert_mat_close!(
                        $m3::from_rotation_z(angle),
                        $quat::from_rotation_z(angle).to_mat3()
                    );
                }
            }

            #[test]
            fn mat3_axis_angle_matches_the_quaternion_and_the_axis_rotations() {
                let axis = $v3::new(1.0, 2.0, 3.0).normalize();
                for degrees in [0.0, 30.0, 90.0, 180.0, 250.0, -60.0] {
                    let angle = deg(degrees);
                    assert_mat_close!(
                        $m3::from_axis_angle(axis, angle),
                        $quat::from_axis_angle(axis, angle).to_mat3()
                    );
                }
                assert_mat_close!(
                    $m3::from_axis_angle($v3::X, deg(50.0)),
                    $m3::from_rotation_x(deg(50.0))
                );
                assert_mat_close!(
                    $m3::from_axis_angle($v3::Y, deg(50.0)),
                    $m3::from_rotation_y(deg(50.0))
                );
                assert_mat_close!(
                    $m3::from_axis_angle($v3::Z, deg(50.0)),
                    $m3::from_rotation_z(deg(50.0))
                );
            }

            #[test]
            fn mat3_rotations_are_orthonormal_with_determinant_one() {
                let m = $m3::from_axis_angle($v3::new(-2.0, 1.0, 0.5).normalize(), deg(111.0));
                assert!(close(m.determinant(), 1.0));
                assert_mat_close!(m * m.transpose(), $m3::IDENTITY);
                assert_mat_close!(m.inverse().unwrap(), m.transpose());
            }

            #[test]
            fn mat3_from_quat_is_the_rotation_matrix() {
                let q = rotation();
                assert_eq!($m3::from_quat(q), q.to_mat3());
                let v = $v3::new(1.0, -2.0, 0.5);
                assert_vec_close!($m3::from_quat(q) * v, q * v);
            }

            #[test]
            fn mat3_from_scale_scales_the_axes() {
                assert_eq!(
                    $m3::from_scale($v3::new(2.0, 3.0, 4.0)) * $v3::ONE,
                    $v3::new(2.0, 3.0, 4.0)
                );
                assert_eq!($m3::from_scale($v3::ONE), $m3::IDENTITY);
            }

            #[test]
            fn mat3_inverse_transpose() {
                // For a rotation it is the matrix itself.
                let r = $m3::from_axis_angle($v3::Y, deg(35.0));
                assert_mat_close!(r.inverse_transpose().unwrap(), r);
                // For a scale it inverts the factors.
                let s = $m3::from_scale($v3::new(2.0, 4.0, 8.0));
                assert_mat_close!(
                    s.inverse_transpose().unwrap(),
                    $m3::from_scale($v3::new(0.5, 0.25, 0.125))
                );
                // In general it is the transpose of the inverse.
                let m = r * s;
                assert_mat_close!(
                    m.inverse_transpose().unwrap(),
                    m.inverse().unwrap().transpose()
                );
                assert!($m3::ZERO.inverse_transpose().is_none());
                assert!(
                    $m3::from_scale($v3::new(1.0, 0.0, 1.0))
                        .inverse_transpose()
                        .is_none()
                );
            }

            // ---- 2D homogeneous transforms ------------------------------

            #[test]
            fn translation_2d_moves_points_but_not_directions() {
                let m = $m3::from_translation_2d($v2::new(5.0, -3.0));
                assert_eq!(m.transform_point2($v2::new(1.0, 2.0)), $v2::new(6.0, -1.0));
                assert_eq!(m.transform_vector2($v2::new(1.0, 2.0)), $v2::new(1.0, 2.0));
                assert_eq!(m.z_axis, $v3::new(5.0, -3.0, 1.0));
                assert_eq!($m3::from_translation_2d($v2::ZERO), $m3::IDENTITY);
            }

            #[test]
            fn scale_2d_scales_both_points_and_directions() {
                let m = $m3::from_scale_2d($v2::new(2.0, 3.0));
                assert_eq!(m.transform_point2($v2::new(1.0, 1.0)), $v2::new(2.0, 3.0));
                assert_eq!(m.transform_vector2($v2::new(1.0, 1.0)), $v2::new(2.0, 3.0));
                assert_eq!(m.determinant(), 6.0);
                assert_eq!($m3::from_scale_2d($v2::ONE), $m3::IDENTITY);
            }

            #[test]
            fn angle_2d_rotates_counter_clockwise() {
                let m = $m3::from_angle_2d(deg(90.0));
                assert_vec_close!(m.transform_point2($v2::X), $v2::Y);
                assert_vec_close!(m.transform_vector2($v2::Y), -$v2::X);
                assert_eq!(m.z_axis, $v3::Z);
                assert_mat_close!(
                    $m3::from_angle_2d(deg(25.0)),
                    $m3::from_mat2($m2::from_angle(deg(25.0)))
                );
            }

            #[test]
            fn scale_angle_translation_2d_is_translate_rotate_scale() {
                let (scale, angle, translation) =
                    ($v2::new(2.0, 3.0), deg(90.0), $v2::new(10.0, -5.0));
                let m = $m3::from_scale_angle_translation_2d(scale, angle, translation);
                let expected = $m3::from_translation_2d(translation)
                    * $m3::from_angle_2d(angle)
                    * $m3::from_scale_2d(scale);
                assert_mat_close!(m, expected);
                // The point is scaled, then rotated, then moved.
                let p = $v2::new(1.0, 1.0);
                let by_hand = (scale * p).rotate(angle) + translation;
                assert_vec_close!(m.transform_point2(p), by_hand);
                // The last row stays `(0, 0, 1)`.
                assert_eq!(m.row(2), $v3::Z);
            }

            #[test]
            fn transform_point2_and_vector2_match_the_matrix_product() {
                let m = $m3::from_scale_angle_translation_2d(
                    $v2::new(1.5, 0.5),
                    deg(33.0),
                    $v2::new(4.0, -1.0),
                );
                let p = $v2::new(-2.0, 7.0);
                let full = m * $v3::new(p.x, p.y, 1.0);
                assert_vec_close!(m.transform_point2(p), $v2::new(full.x, full.y));
                let full = m * $v3::new(p.x, p.y, 0.0);
                assert_vec_close!(m.transform_vector2(p), $v2::new(full.x, full.y));
            }

            // ---- conversions between sizes ------------------------------

            #[test]
            fn conversions_between_sizes() {
                let m2 = $m2::from_cols_array([1.0, 2.0, 3.0, 4.0]);
                assert_eq!(
                    $m3::from_mat2(m2).to_cols_array(),
                    [1.0, 2.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 1.0]
                );
                assert_eq!($m2::from_mat3($m3::from_mat2(m2)), m2);

                let m3 = $m3::from_cols_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
                assert_eq!(
                    $m4::from_mat3(m3).to_cols_array(),
                    [
                        1.0, 2.0, 3.0, 0.0, 4.0, 5.0, 6.0, 0.0, 7.0, 8.0, 9.0, 0.0, 0.0, 0.0, 0.0,
                        1.0
                    ]
                );
                assert_eq!($m3::from_mat4($m4::from_mat3(m3)), m3);
                assert_eq!($m2::from_mat3(m3).to_cols_array(), [1.0, 2.0, 4.0, 5.0]);
            }

            #[test]
            fn from_mat4_drops_the_translation_and_the_projective_row() {
                let m = $m4::from_translation($v3::new(7.0, 8.0, 9.0))
                    * $m4::from_rotation_z(deg(90.0));
                assert_mat_close!($m3::from_mat4(m), $m3::from_rotation_z(deg(90.0)));
                assert_eq!($m4::from_mat3($m3::IDENTITY), $m4::IDENTITY);
            }

            // ---- 4x4 transform construction ------------------------------

            #[test]
            fn translation_moves_points_but_not_directions() {
                let m = $m4::from_translation($v3::new(1.0, 2.0, 3.0));
                assert_eq!(
                    m.transform_point3($v3::new(10.0, 20.0, 30.0)),
                    $v3::new(11.0, 22.0, 33.0)
                );
                assert_eq!(
                    m.transform_vector3($v3::new(10.0, 20.0, 30.0)),
                    $v3::new(10.0, 20.0, 30.0)
                );
                assert_eq!(m.w_axis, $v4::new(1.0, 2.0, 3.0, 1.0));
                assert_eq!(
                    m * $v4::new(10.0, 20.0, 30.0, 1.0),
                    $v4::new(11.0, 22.0, 33.0, 1.0)
                );
                assert_eq!($m4::from_translation($v3::ZERO), $m4::IDENTITY);
            }

            #[test]
            fn scale_scales_points_and_directions() {
                let m = $m4::from_scale($v3::new(2.0, 3.0, 4.0));
                assert_eq!(m.transform_point3($v3::ONE), $v3::new(2.0, 3.0, 4.0));
                assert_eq!(m.transform_vector3($v3::ONE), $v3::new(2.0, 3.0, 4.0));
                assert_eq!(m.w_axis, $v4::W);
                assert_eq!(m.determinant(), 24.0);
                assert_eq!($m4::from_scale($v3::ONE), $m4::IDENTITY);
            }

            #[test]
            fn rotations_match_the_quaternion_and_the_3x3_matrix() {
                for degrees in [0.0, 30.0, 90.0, 200.0, -75.0] {
                    let angle = deg(degrees);
                    assert_mat_close!(
                        $m4::from_rotation_x(angle),
                        $quat::from_rotation_x(angle).to_mat4()
                    );
                    assert_mat_close!(
                        $m4::from_rotation_y(angle),
                        $quat::from_rotation_y(angle).to_mat4()
                    );
                    assert_mat_close!(
                        $m4::from_rotation_z(angle),
                        $quat::from_rotation_z(angle).to_mat4()
                    );
                    assert_mat_close!(
                        $m4::from_rotation_z(angle),
                        $m4::from_mat3($m3::from_rotation_z(angle))
                    );
                }
                let axis = $v3::new(0.0, 3.0, -4.0).normalize();
                assert_mat_close!(
                    $m4::from_axis_angle(axis, deg(66.0)),
                    $quat::from_axis_angle(axis, deg(66.0)).to_mat4()
                );
                assert_eq!($m4::from_quat(rotation()), rotation().to_mat4());
            }

            #[test]
            fn from_scale_rotation_translation_is_translate_rotate_scale() {
                let (scale, translation) = ($v3::new(2.0, 3.0, 0.5), $v3::new(1.0, -2.0, 3.0));
                let expected = $m4::from_translation(translation)
                    * $m4::from_quat(rotation())
                    * $m4::from_scale(scale);
                assert_mat_close!(model(), expected);
                // A point is scaled first, then rotated, then moved.
                let p = $v3::new(1.0, -1.0, 2.0);
                assert_vec_close!(
                    model().transform_point3(p),
                    rotation() * (scale * p) + translation
                );
                assert_eq!(model().row(3), $v4::W);
            }

            #[test]
            fn from_scale_rotation_translation_with_identity_parts() {
                assert_eq!(
                    $m4::from_scale_rotation_translation($v3::ONE, $quat::IDENTITY, $v3::ZERO),
                    $m4::IDENTITY
                );
                assert_eq!(
                    $m4::from_scale_rotation_translation(
                        $v3::ONE,
                        $quat::IDENTITY,
                        $v3::new(1.0, 2.0, 3.0)
                    ),
                    $m4::from_translation($v3::new(1.0, 2.0, 3.0))
                );
                assert_eq!(
                    $m4::from_scale_rotation_translation(
                        $v3::new(2.0, 3.0, 4.0),
                        $quat::IDENTITY,
                        $v3::ZERO
                    ),
                    $m4::from_scale($v3::new(2.0, 3.0, 4.0))
                );
            }

            // ---- decomposition --------------------------------------------

            #[test]
            fn decomposition_inverts_the_composition() {
                let (scale, q, translation) = model().to_scale_rotation_translation().unwrap();
                assert_vec_close!(scale, $v3::new(2.0, 3.0, 0.5));
                assert_vec_close!(translation, $v3::new(1.0, -2.0, 3.0));
                // `q` and `-q` are the same rotation.
                assert!(
                    close(q.dot(rotation()).abs(), 1.0),
                    "{q:?} vs {:?}",
                    rotation()
                );
                assert_mat_close!(
                    $m4::from_scale_rotation_translation(scale, q, translation),
                    model()
                );
            }

            #[test]
            fn decomposition_of_the_simple_cases() {
                let (scale, q, translation) =
                    $m4::IDENTITY.to_scale_rotation_translation().unwrap();
                assert_eq!((scale, translation), ($v3::ONE, $v3::ZERO));
                assert_vec_close!(q, $quat::IDENTITY);

                let t = $m4::from_translation($v3::new(4.0, 5.0, 6.0));
                assert_eq!(
                    t.to_scale_rotation_translation().unwrap().2,
                    $v3::new(4.0, 5.0, 6.0)
                );

                let s = $m4::from_scale($v3::new(2.0, 3.0, 4.0));
                assert_vec_close!(
                    s.to_scale_rotation_translation().unwrap().0,
                    $v3::new(2.0, 3.0, 4.0)
                );
            }

            #[test]
            fn decomposition_handles_mirroring() {
                for scale in [
                    $v3::new(-2.0, 3.0, 0.5),
                    $v3::new(2.0, -3.0, 0.5),
                    $v3::new(2.0, 3.0, -0.5),
                    $v3::new(-2.0, -3.0, -0.5),
                ] {
                    let original = $m4::from_scale_rotation_translation(
                        scale,
                        rotation(),
                        $v3::new(1.0, 2.0, 3.0),
                    );
                    let (s, q, t) = original.to_scale_rotation_translation().unwrap();
                    // A mirror is carried by the sign of the `x` scale, so the
                    // rotation stays a proper one.
                    assert!(s.y > 0.0 && s.z > 0.0, "{s:?}");
                    assert!(close(q.length(), 1.0));
                    assert_mat_close!($m4::from_scale_rotation_translation(s, q, t), original);
                }
                let (s, _, _) = $m4::from_scale($v3::new(-2.0, 3.0, 1.0))
                    .to_scale_rotation_translation()
                    .unwrap();
                assert_vec_close!(s, $v3::new(-2.0, 3.0, 1.0));
            }

            #[test]
            fn decomposition_fails_for_a_flattened_or_broken_matrix() {
                assert!(
                    $m4::from_scale($v3::new(1.0, 0.0, 1.0))
                        .to_scale_rotation_translation()
                        .is_none()
                );
                assert!($m4::ZERO.to_scale_rotation_translation().is_none());
                let mut nan = $m4::IDENTITY;
                nan.y_axis.y = $scalar::NAN;
                assert!(nan.to_scale_rotation_translation().is_none());
                let mut inf = $m4::IDENTITY;
                inf.x_axis.x = $scalar::INFINITY;
                assert!(inf.to_scale_rotation_translation().is_none());
            }

            // ---- applying transforms ---------------------------------------

            #[test]
            fn transform_point3_and_vector3_match_the_matrix_product() {
                let p = $v3::new(0.5, -4.0, 2.0);
                let full = model() * $v4::new(p.x, p.y, p.z, 1.0);
                assert_vec_close!(model().transform_point3(p), full.xyz());
                let full = model() * $v4::new(p.x, p.y, p.z, 0.0);
                assert_vec_close!(model().transform_vector3(p), full.xyz());
            }

            #[test]
            fn project_point3_divides_by_w() {
                // A matrix that only produces `w = 2`.
                let halve = $m4::from_cols($v4::X, $v4::Y, $v4::Z, $v4::new(0.0, 0.0, 0.0, 2.0));
                assert_eq!(
                    halve.project_point3($v3::new(4.0, 6.0, 8.0)),
                    $v3::new(2.0, 3.0, 4.0)
                );
                // Affine matrices have `w == 1`, so the two functions agree.
                let p = $v3::new(1.0, 2.0, 3.0);
                assert_vec_close!(model().project_point3(p), model().transform_point3(p));
                // `w == 0` is not finite.
                let flat = $m4::from_cols($v4::X, $v4::Y, $v4::Z, $v4::ZERO);
                assert!(!flat.project_point3($v3::ONE).is_finite());
            }

            #[test]
            fn unproject_point3_reverses_project_point3() {
                let view = $m4::look_at_rh($v3::new(3.0, 4.0, 5.0), $v3::ZERO, $v3::Y);
                let projection = $m4::perspective_rh(deg(60.0), 1.5, 0.1, 100.0);
                let view_projection = projection * view;
                for p in [
                    $v3::ZERO,
                    $v3::new(1.0, 2.0, -3.0),
                    $v3::new(-4.0, 0.5, 1.0),
                ] {
                    let ndc = view_projection.project_point3(p);
                    let back = view_projection.unproject_point3(ndc).unwrap();
                    assert!(back.approx_eq_eps(p, 1e-3), "{back:?} vs {p:?}");
                }
                assert!($m4::ZERO.unproject_point3($v3::ONE).is_none());
            }

            #[test]
            fn inverse_affine_matches_the_general_inverse() {
                let inverse = model().inverse_affine().unwrap();
                assert_mat_close!(inverse, model().inverse().unwrap());
                assert_mat_close!(model() * inverse, $m4::IDENTITY);
                assert_mat_close!(inverse * model(), $m4::IDENTITY);
                assert_eq!(inverse.row(3), $v4::W);
                let p = $v3::new(1.0, 2.0, 3.0);
                assert_vec_close!(inverse.transform_point3(model().transform_point3(p)), p);
            }

            #[test]
            fn inverse_affine_of_the_simple_cases() {
                assert_eq!($m4::IDENTITY.inverse_affine(), Some($m4::IDENTITY));
                assert_eq!(
                    $m4::from_translation($v3::new(1.0, 2.0, 3.0)).inverse_affine(),
                    Some($m4::from_translation($v3::new(-1.0, -2.0, -3.0)))
                );
                assert_mat_close!(
                    $m4::from_scale($v3::new(2.0, 4.0, 8.0))
                        .inverse_affine()
                        .unwrap(),
                    $m4::from_scale($v3::new(0.5, 0.25, 0.125))
                );
                assert!(
                    $m4::from_scale($v3::new(1.0, 0.0, 1.0))
                        .inverse_affine()
                        .is_none()
                );
                assert!($m4::ZERO.inverse_affine().is_none());
            }

            #[test]
            fn normal_matrix_keeps_normals_perpendicular() {
                // A plane with tangent (1, 1, 0) and normal (1, -1, 0), seen
                // through a non-uniform scale.
                let m = $m4::from_scale($v3::new(4.0, 1.0, 1.0));
                let (tangent, normal) = ($v3::new(1.0, 1.0, 0.0), $v3::new(1.0, -1.0, 0.0));
                assert_eq!(tangent.dot(normal), 0.0);
                let moved_tangent = m.transform_vector3(tangent);
                // Transforming the normal like a direction breaks that...
                assert!(m.transform_vector3(normal).dot(moved_tangent).abs() > 1.0);
                // ...and the normal matrix keeps it.
                let moved_normal = m.normal_matrix().unwrap() * normal;
                assert!(close(moved_normal.dot(moved_tangent), 0.0));
            }

            #[test]
            fn normal_matrix_of_a_general_model_matrix() {
                let (tangent, normal) = ($v3::new(1.0, 2.0, 0.0), $v3::new(2.0, -1.0, 0.0));
                assert_eq!(tangent.dot(normal), 0.0);
                let moved_tangent = model().transform_vector3(tangent);
                let moved_normal = model().normal_matrix().unwrap() * normal;
                assert!(close(moved_normal.dot(moved_tangent), 0.0));
                // It is the inverse transpose of the linear part.
                assert_mat_close!(
                    model().normal_matrix().unwrap(),
                    $m3::from_mat4(model()).inverse().unwrap().transpose()
                );
                // A rotation leaves it unchanged.
                let r = $m4::from_quat(rotation());
                assert_mat_close!(r.normal_matrix().unwrap(), $m3::from_quat(rotation()));
                assert!(
                    $m4::from_scale($v3::new(1.0, 0.0, 1.0))
                        .normal_matrix()
                        .is_none()
                );
            }

            // ---- view matrices ----------------------------------------------

            #[test]
            fn look_at_rh_puts_the_camera_at_the_origin_looking_down_minus_z() {
                let (eye, center) = ($v3::new(3.0, 4.0, 5.0), $v3::new(-1.0, 2.0, 0.5));
                let view = $m4::look_at_rh(eye, center, $v3::Y);
                assert_vec_close!(view.transform_point3(eye), $v3::ZERO);
                let distance = (center - eye).length();
                assert_vec_close!(view.transform_point3(center), $v3::new(0.0, 0.0, -distance));
                // Up stays up and the side vector becomes `+X`.
                let up_point = view.transform_point3(eye + $v3::Y);
                assert!(up_point.y > 0.0, "{up_point:?}");
                let right = (center - eye).cross($v3::Y);
                assert!(view.transform_point3(eye + right).x > 0.0);
            }

            #[test]
            fn look_at_lh_puts_the_camera_at_the_origin_looking_down_plus_z() {
                let (eye, center) = ($v3::new(3.0, 4.0, 5.0), $v3::new(-1.0, 2.0, 0.5));
                let view = $m4::look_at_lh(eye, center, $v3::Y);
                assert_vec_close!(view.transform_point3(eye), $v3::ZERO);
                let distance = (center - eye).length();
                assert_vec_close!(view.transform_point3(center), $v3::new(0.0, 0.0, distance));
                let up_point = view.transform_point3(eye + $v3::Y);
                assert!(up_point.y > 0.0, "{up_point:?}");
                // In a left-handed world the side vector is `up x forward`.
                let right = $v3::Y.cross(center - eye);
                assert!(view.transform_point3(eye + right).x > 0.0);
            }

            #[test]
            fn look_at_of_the_standard_cases() {
                // Standing on +Z looking at the origin, up is +Y: the identity
                // rotation, only moved back by the distance.
                let view = $m4::look_at_rh($v3::new(0.0, 0.0, 5.0), $v3::ZERO, $v3::Y);
                assert_mat_close!(view, $m4::from_translation($v3::new(0.0, 0.0, -5.0)));
                // The same camera in a left-handed world looks down +Z from -Z.
                let view = $m4::look_at_lh($v3::new(0.0, 0.0, -5.0), $v3::ZERO, $v3::Y);
                assert_mat_close!(view, $m4::from_translation($v3::new(0.0, 0.0, 5.0)));
            }

            #[test]
            fn view_matrices_are_rigid_transforms() {
                let (eye, center, up) = (
                    $v3::new(3.0, 4.0, 5.0),
                    $v3::new(-1.0, 2.0, 0.5),
                    $v3::new(0.2, 1.0, 0.1),
                );
                for view in [
                    $m4::look_at_rh(eye, center, up),
                    $m4::look_at_lh(eye, center, up),
                ] {
                    let linear = $m3::from_mat4(view);
                    assert!(close(linear.determinant(), 1.0));
                    assert_mat_close!(linear * linear.transpose(), $m3::IDENTITY);
                    assert_eq!(view.row(3), $v4::W);
                    assert_mat_close!(view * view.inverse_affine().unwrap(), $m4::IDENTITY);
                }
            }

            #[test]
            fn look_to_equals_look_at_with_the_direction() {
                let (eye, center, up) = ($v3::new(1.0, 2.0, 3.0), $v3::new(4.0, -5.0, 6.0), $v3::Y);
                assert_eq!(
                    $m4::look_to_rh(eye, center - eye, up),
                    $m4::look_at_rh(eye, center, up)
                );
                assert_eq!(
                    $m4::look_to_lh(eye, center - eye, up),
                    $m4::look_at_lh(eye, center, up)
                );
                // The direction and the up vector need not be normalized.
                assert_mat_close!(
                    $m4::look_to_rh(eye, (center - eye) * 7.0, up * 3.0),
                    $m4::look_to_rh(eye, (center - eye).normalize(), up)
                );
            }

            #[test]
            fn look_at_with_a_degenerate_direction_is_not_finite() {
                let eye = $v3::new(1.0, 2.0, 3.0);
                let same = $m4::look_at_rh(eye, eye, $v3::Y);
                assert!(!same.x_axis.is_finite());
                let parallel = $m4::look_to_rh(eye, $v3::Y, $v3::Y);
                assert!(!parallel.x_axis.is_finite());
            }

            // ---- perspective projections ----------------------------------------

            const NEAR: $scalar = 0.5;
            const FAR: $scalar = 50.0;

            /// The depth `project_point3` gives a point `distance` in front of
            /// the camera; `sign` is `-1` in a right-handed view space and `1`
            /// in a left-handed one.
            fn depth(m: $m4, sign: $scalar, distance: $scalar) -> $scalar {
                m.project_point3($v3::new(0.0, 0.0, sign * distance)).z
            }

            #[test]
            fn perspective_depth_ranges() {
                let (fov, aspect) = (deg(60.0), 1.5);
                let rh = $m4::perspective_rh(fov, aspect, NEAR, FAR);
                assert!(close(depth(rh, -1.0, NEAR), 0.0));
                assert!(close(depth(rh, -1.0, FAR), 1.0));
                let lh = $m4::perspective_lh(fov, aspect, NEAR, FAR);
                assert!(close(depth(lh, 1.0, NEAR), 0.0));
                assert!(close(depth(lh, 1.0, FAR), 1.0));
                let rh_gl = $m4::perspective_rh_gl(fov, aspect, NEAR, FAR);
                assert!(close(depth(rh_gl, -1.0, NEAR), -1.0));
                assert!(close(depth(rh_gl, -1.0, FAR), 1.0));
                let lh_gl = $m4::perspective_lh_gl(fov, aspect, NEAR, FAR);
                assert!(close(depth(lh_gl, 1.0, NEAR), -1.0));
                assert!(close(depth(lh_gl, 1.0, FAR), 1.0));
            }

            #[test]
            fn reverse_z_swaps_the_depth_of_the_planes() {
                let (fov, aspect) = (deg(60.0), 1.5);
                let rh = $m4::perspective_rh_reverse_z(fov, aspect, NEAR, FAR);
                assert!(close(depth(rh, -1.0, NEAR), 1.0));
                assert!(close(depth(rh, -1.0, FAR), 0.0));
                let lh = $m4::perspective_lh_reverse_z(fov, aspect, NEAR, FAR);
                assert!(close(depth(lh, 1.0, NEAR), 1.0));
                assert!(close(depth(lh, 1.0, FAR), 0.0));
                // Depth between the planes decreases with distance.
                let middle = depth(rh, -1.0, 5.0);
                assert!(middle > 0.0 && middle < 1.0 && middle > depth(rh, -1.0, 20.0));
            }

            #[test]
            fn standard_depth_increases_with_distance() {
                for (m, sign) in [
                    ($m4::perspective_rh(deg(60.0), 1.0, NEAR, FAR), -1.0),
                    ($m4::perspective_lh(deg(60.0), 1.0, NEAR, FAR), 1.0),
                    ($m4::perspective_rh_gl(deg(60.0), 1.0, NEAR, FAR), -1.0),
                    ($m4::perspective_lh_gl(deg(60.0), 1.0, NEAR, FAR), 1.0),
                ] {
                    let mut last = depth(m, sign, NEAR);
                    for distance in [1.0, 2.0, 5.0, 10.0, 25.0, FAR] {
                        let now = depth(m, sign, distance);
                        assert!(now > last, "{distance}: {now} <= {last}");
                        last = now;
                    }
                }
            }

            #[test]
            fn perspective_field_of_view_and_aspect_ratio() {
                let (fov, aspect) = (deg(60.0), 2.0);
                let half = tan(Radians::from(fov).value() * 0.5);
                for (m, sign) in [
                    ($m4::perspective_rh(fov, aspect, NEAR, FAR), -1.0),
                    ($m4::perspective_lh(fov, aspect, NEAR, FAR), 1.0),
                    ($m4::perspective_rh_gl(fov, aspect, NEAR, FAR), -1.0),
                    ($m4::perspective_lh_gl(fov, aspect, NEAR, FAR), 1.0),
                    ($m4::perspective_rh_reverse_z(fov, aspect, NEAR, FAR), -1.0),
                ] {
                    let d = 7.0;
                    // The top edge of the frustum is at `tan(fov / 2) * d`...
                    let top = m.project_point3($v3::new(0.0, half * d, sign * d));
                    assert!(close(top.y, 1.0) && close(top.x, 0.0), "{top:?}");
                    // ...and the right edge `aspect` times further out.
                    let right = m.project_point3($v3::new(half * aspect * d, 0.0, sign * d));
                    assert!(close(right.x, 1.0) && close(right.y, 0.0), "{right:?}");
                    // Below and to the left are negative.
                    let corner =
                        m.project_point3($v3::new(-half * aspect * d, -half * d, sign * d));
                    assert!(close(corner.x, -1.0) && close(corner.y, -1.0), "{corner:?}");
                }
            }

            #[test]
            fn perspective_w_is_the_distance_in_front_of_the_camera() {
                let rh = $m4::perspective_rh(deg(90.0), 1.0, NEAR, FAR);
                assert!(close((rh * $v4::new(0.0, 0.0, -8.0, 1.0)).w, 8.0));
                let lh = $m4::perspective_lh(deg(90.0), 1.0, NEAR, FAR);
                assert!(close((lh * $v4::new(0.0, 0.0, 8.0, 1.0)).w, 8.0));
                // Behind the camera `w` is negative, which clipping rejects.
                assert!((rh * $v4::new(0.0, 0.0, 8.0, 1.0)).w < 0.0);
            }

            #[test]
            fn perspective_accepts_radians_and_degrees() {
                let from_degrees = $m4::perspective_rh(deg(75.0), 1.3, 0.1, 100.0);
                let from_radians = $m4::perspective_rh(Radians::from(deg(75.0)), 1.3, 0.1, 100.0);
                assert_mat_close!(from_degrees, from_radians);
            }

            #[test]
            fn view_projection_puts_the_target_in_the_middle_of_the_screen() {
                let (eye, target) = ($v3::new(3.0, 4.0, 5.0), $v3::new(-1.0, 2.0, 0.5));
                let view_projection = $m4::perspective_rh(deg(70.0), 1.6, 0.1, 100.0)
                    * $m4::look_at_rh(eye, target, $v3::Y);
                let ndc = view_projection.project_point3(target);
                assert!(close(ndc.x, 0.0) && close(ndc.y, 0.0), "{ndc:?}");
                assert!(ndc.z > 0.0 && ndc.z < 1.0, "{ndc:?}");
                // Points to the right of the view direction land at positive x.
                let right = (target - eye).cross($v3::Y).normalize();
                assert!(view_projection.project_point3(target + right).x > 0.0);
                assert!(view_projection.project_point3(target - right).x < 0.0);
                assert!(view_projection.project_point3(target + $v3::Y).y > 0.0);
            }

            #[test]
            fn degenerate_perspective_input_is_not_finite() {
                assert!(
                    !$m4::perspective_rh(deg(0.0), 1.0, NEAR, FAR)
                        .y_axis
                        .is_finite()
                );
                assert!(
                    !$m4::perspective_rh(deg(60.0), 0.0, NEAR, FAR)
                        .x_axis
                        .is_finite()
                );
                assert!(
                    !$m4::perspective_rh(deg(60.0), 1.0, 1.0, 1.0)
                        .z_axis
                        .is_finite()
                );
            }

            // ---- orthographic projections --------------------------------------------

            const BOX: ($scalar, $scalar, $scalar, $scalar) = (-4.0, 6.0, -2.0, 8.0);

            fn ortho_corner(m: $m4, x: $scalar, y: $scalar, z: $scalar) -> $v3 {
                m.transform_point3($v3::new(x, y, z))
            }

            #[test]
            fn orthographic_maps_the_box_to_the_clip_cube() {
                let (l, r, b, t) = BOX;
                // Right-handed, depth 0..1.
                let m = $m4::orthographic_rh(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, -NEAR), $v3::new(-1.0, -1.0, 0.0));
                assert_vec_close!(ortho_corner(m, r, t, -FAR), $v3::new(1.0, 1.0, 1.0));
                // Left-handed, depth 0..1.
                let m = $m4::orthographic_lh(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, NEAR), $v3::new(-1.0, -1.0, 0.0));
                assert_vec_close!(ortho_corner(m, r, t, FAR), $v3::new(1.0, 1.0, 1.0));
                // OpenGL depth -1..1.
                let m = $m4::orthographic_rh_gl(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, -NEAR), $v3::new(-1.0, -1.0, -1.0));
                assert_vec_close!(ortho_corner(m, r, t, -FAR), $v3::new(1.0, 1.0, 1.0));
                let m = $m4::orthographic_lh_gl(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, NEAR), $v3::new(-1.0, -1.0, -1.0));
                assert_vec_close!(ortho_corner(m, r, t, FAR), $v3::new(1.0, 1.0, 1.0));
            }

            #[test]
            fn orthographic_reverse_z_swaps_the_depth_of_the_planes() {
                let (l, r, b, t) = BOX;
                let m = $m4::orthographic_rh_reverse_z(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, -NEAR), $v3::new(-1.0, -1.0, 1.0));
                assert_vec_close!(ortho_corner(m, r, t, -FAR), $v3::new(1.0, 1.0, 0.0));
                let m = $m4::orthographic_lh_reverse_z(l, r, b, t, NEAR, FAR);
                assert_vec_close!(ortho_corner(m, l, b, NEAR), $v3::new(-1.0, -1.0, 1.0));
                assert_vec_close!(ortho_corner(m, r, t, FAR), $v3::new(1.0, 1.0, 0.0));
            }

            #[test]
            fn orthographic_centre_and_linear_depth() {
                let (l, r, b, t) = BOX;
                let m = $m4::orthographic_rh(l, r, b, t, NEAR, FAR);
                let centre = ortho_corner(m, (l + r) * 0.5, (b + t) * 0.5, -(NEAR + FAR) * 0.5);
                assert_vec_close!(centre, $v3::new(0.0, 0.0, 0.5));
                // It is affine: the last row is `(0, 0, 0, 1)`.
                assert_eq!(m.row(3), $v4::W);
                // And depth is linear, unlike with a perspective projection.
                let quarter = ortho_corner(m, 0.0, 0.0, -(NEAR + 0.25 * (FAR - NEAR)));
                assert!(close(quarter.z, 0.25));
            }

            #[test]
            fn orthographic_degenerate_boxes_are_not_finite() {
                assert!(
                    !$m4::orthographic_rh(1.0, 1.0, 0.0, 1.0, NEAR, FAR)
                        .x_axis
                        .is_finite()
                );
                assert!(
                    !$m4::orthographic_rh(0.0, 1.0, 2.0, 2.0, NEAR, FAR)
                        .y_axis
                        .is_finite()
                );
                assert!(
                    !$m4::orthographic_rh(0.0, 1.0, 0.0, 1.0, 3.0, 3.0)
                        .z_axis
                        .is_finite()
                );
            }

            // ---- const evaluation ------------------------------------------------------

            #[test]
            fn const_functions_work_in_const_context() {
                const DIAGONAL: $m3 = $m3::from_diagonal($v3::new(1.0, 2.0, 3.0));
                const TRANSLATION: $m4 = $m4::from_translation($v3::new(1.0, 2.0, 3.0));
                const SCALE: $m4 = $m4::from_scale($v3::new(2.0, 2.0, 2.0));
                const SCALE_3: $m3 = $m3::from_scale($v3::new(2.0, 2.0, 2.0));
                const T2: $m3 = $m3::from_translation_2d($v2::new(1.0, 2.0));
                const S2: $m3 = $m3::from_scale_2d($v2::new(1.0, 2.0));
                const FROM_2: $m3 = $m3::from_mat2($m2::IDENTITY);
                const FROM_3: $m4 = $m4::from_mat3($m3::IDENTITY);
                const FROM_4: $m3 = $m3::from_mat4($m4::IDENTITY);
                const FROM_3_TO_2: $m2 = $m2::from_mat3($m3::IDENTITY);
                const FROM_QUAT: $m3 = $m3::from_quat($quat::IDENTITY);
                const FROM_QUAT_4: $m4 = $m4::from_quat($quat::IDENTITY);
                const TRS: $m4 =
                    $m4::from_scale_rotation_translation($v3::ONE, $quat::IDENTITY, $v3::X);
                const POINT: $v3 = TRANSLATION.transform_point3($v3::ONE);
                const VECTOR: $v3 = TRANSLATION.transform_vector3($v3::ONE);
                const POINT_2: $v2 = T2.transform_point2($v2::ONE);
                const VECTOR_2: $v2 = T2.transform_vector2($v2::ONE);
                const PROJECTED: $v3 = SCALE.project_point3($v3::ONE);
                const INVERSE: Option<$m4> = TRANSLATION.inverse_affine();
                const UNPROJECTED: Option<$v3> = SCALE.unproject_point3($v3::ONE);
                const NORMAL: Option<$m3> = SCALE.normal_matrix();
                const INVERSE_TRANSPOSE: Option<$m3> = SCALE_3.inverse_transpose();
                const ORTHO: $m4 = $m4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                const ORTHO_LH: $m4 = $m4::orthographic_lh(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                const ORTHO_RH_GL: $m4 = $m4::orthographic_rh_gl(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                const ORTHO_LH_GL: $m4 = $m4::orthographic_lh_gl(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                const ORTHO_RZ: $m4 =
                    $m4::orthographic_rh_reverse_z(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                const ORTHO_LZ: $m4 =
                    $m4::orthographic_lh_reverse_z(-1.0, 1.0, -1.0, 1.0, 0.0, 2.0);
                assert_eq!(DIAGONAL.determinant(), 6.0);
                assert_eq!(TRANSLATION.w_axis, $v4::new(1.0, 2.0, 3.0, 1.0));
                assert_eq!(SCALE.determinant(), 8.0);
                assert_eq!(T2.z_axis, $v3::new(1.0, 2.0, 1.0));
                assert_eq!(S2.determinant(), 2.0);
                assert_eq!(
                    (FROM_2, FROM_3, FROM_4, FROM_3_TO_2),
                    ($m3::IDENTITY, $m4::IDENTITY, $m3::IDENTITY, $m2::IDENTITY)
                );
                assert_eq!((FROM_QUAT, FROM_QUAT_4), ($m3::IDENTITY, $m4::IDENTITY));
                assert_eq!(TRS, $m4::from_translation($v3::X));
                assert_eq!((POINT, VECTOR), ($v3::new(2.0, 3.0, 4.0), $v3::ONE));
                assert_eq!((POINT_2, VECTOR_2), ($v2::new(2.0, 3.0), $v2::ONE));
                assert_eq!(PROJECTED, $v3::new(2.0, 2.0, 2.0));
                assert_eq!(
                    INVERSE,
                    Some($m4::from_translation($v3::new(-1.0, -2.0, -3.0)))
                );
                assert_eq!(UNPROJECTED, Some($v3::splat(0.5)));
                assert_eq!(NORMAL, Some($m3::from_scale($v3::splat(0.5))));
                assert_eq!(INVERSE_TRANSPOSE, Some($m3::from_scale($v3::splat(0.5))));
                for ortho in [
                    ORTHO,
                    ORTHO_LH,
                    ORTHO_RH_GL,
                    ORTHO_LH_GL,
                    ORTHO_RZ,
                    ORTHO_LZ,
                ] {
                    assert!(ortho.x_axis.is_finite());
                }
            }
        }
    };
}

float_tests!(mat_f32, f32, Vec2, Vec3, Vec4, Mat2, Mat3, Mat4, Quat);
float_tests!(
    dmat_f64, f64, DVec2, DVec3, DVec4, DMat2, DMat3, DMat4, DQuat
);
