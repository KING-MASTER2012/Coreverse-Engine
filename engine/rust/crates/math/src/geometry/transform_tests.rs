//! Tests for the translation-rotation-scale transform. Instantiated once
//! per float family.

use super::*;
use crate::angle::Radians;
use crate::test_support::Rng;
use crate::{DMat4, DQuat, DVec3, Mat4, Quat, Vec3};

macro_rules! transform_tests {
    ($module:ident, $scalar:ty, $transform:ident, $v3:ident, $quat:ident, $m4:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-3;

            fn random_vector(rng: &mut Rng, extent: f64) -> $v3 {
                $v3::new(
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                )
            }

            fn random_rotation(rng: &mut Rng) -> $quat {
                $quat::from_axis_angle(
                    random_vector(rng, 1.0).normalize(),
                    Radians::new(rng.range(0.0, 6.28) as $scalar),
                )
            }

            fn random_transform(rng: &mut Rng, uniform: bool) -> $transform {
                let scale = if uniform {
                    $v3::splat(rng.range(0.4, 3.0) as $scalar)
                } else {
                    $v3::new(
                        rng.range(0.4, 3.0) as $scalar,
                        rng.range(0.4, 3.0) as $scalar,
                        rng.range(0.4, 3.0) as $scalar,
                    )
                };
                $transform::new(random_vector(rng, 10.0), random_rotation(rng), scale)
            }

            fn close(a: $v3, b: $v3) -> bool {
                (a - b).length() < TOL
            }

            #[test]
            fn identity_changes_nothing() {
                let point = $v3::new(1.0, -2.0, 3.0);
                assert_eq!($transform::IDENTITY.transform_point(point), point);
                assert_eq!($transform::IDENTITY.transform_vector(point), point);
                assert_eq!($transform::IDENTITY.to_mat4(), $m4::IDENTITY);
                assert_eq!($transform::default(), $transform::IDENTITY);
                assert!($transform::IDENTITY.is_finite());
            }

            #[test]
            fn constructors_set_one_part() {
                let t = $v3::new(1.0, 2.0, 3.0);
                assert_eq!($transform::from_translation(t).translation, t);
                assert_eq!($transform::from_translation(t).scale, $v3::ONE);
                let q = $quat::from_rotation_y(Radians::new(0.5));
                assert_eq!($transform::from_rotation(q).rotation, q);
                assert_eq!($transform::from_scale(t).scale, t);
                assert_eq!($transform::from_scale(t).translation, $v3::ZERO);
                let built = $transform::IDENTITY
                    .with_translation(t)
                    .with_rotation(q)
                    .with_scale(t);
                assert_eq!(built, $transform::new(t, q, t));
            }

            #[test]
            fn to_mat4_is_t_times_r_times_s() {
                let transform = $transform::new(
                    $v3::new(1.0, 2.0, 3.0),
                    $quat::from_rotation_z(Radians::new(0.7)),
                    $v3::new(2.0, 3.0, 4.0),
                );
                let expected = $m4::from_translation(transform.translation)
                    * $m4::from_rotation_z(Radians::new(0.7))
                    * $m4::from_scale(transform.scale);
                assert!(transform.to_mat4().approx_eq_eps(expected, 1e-5));
            }

            #[test]
            fn from_mat4_round_trips() {
                let mut rng = Rng::new(2);
                for _ in 0..100 {
                    let transform = random_transform(&mut rng, false);
                    let back = $transform::from_mat4(transform.to_mat4()).unwrap();
                    assert!(close(back.translation, transform.translation));
                    assert!(close(back.scale, transform.scale));
                    assert!(back.rotation.angle_between(transform.rotation).value() < TOL);
                }
                assert!($transform::from_mat4($m4::ZERO).is_none());
            }

            #[test]
            fn transform_point_matches_the_matrix() {
                let mut rng = Rng::new(3);
                for _ in 0..100 {
                    let transform = random_transform(&mut rng, false);
                    let matrix = transform.to_mat4();
                    let point = random_vector(&mut rng, 10.0);
                    assert!(close(
                        transform.transform_point(point),
                        matrix.transform_point3(point)
                    ));
                    assert!(close(
                        transform.transform_vector(point),
                        matrix.transform_vector3(point)
                    ));
                    assert!(close(transform * point, matrix.transform_point3(point)));
                }
            }

            #[test]
            fn inverse_transform_undoes_any_transform() {
                let mut rng = Rng::new(4);
                for _ in 0..100 {
                    let transform = random_transform(&mut rng, false);
                    let point = random_vector(&mut rng, 10.0);
                    let moved = transform.transform_point(point);
                    assert!(close(transform.inverse_transform_point(moved), point));
                    let vector = transform.transform_vector(point);
                    assert!(close(transform.inverse_transform_vector(vector), point));
                }
            }

            #[test]
            fn inverse_of_a_uniform_scale_transform() {
                let mut rng = Rng::new(5);
                for _ in 0..100 {
                    let transform = random_transform(&mut rng, true);
                    let inverse = transform.inverse().unwrap();
                    let point = random_vector(&mut rng, 10.0);
                    assert!(close(
                        inverse.transform_point(transform.transform_point(point)),
                        point
                    ));
                    assert!(close(
                        transform.transform_point(inverse.transform_point(point)),
                        point
                    ));
                    assert!(
                        (inverse * transform)
                            .to_mat4()
                            .approx_eq_eps($m4::IDENTITY, 1e-3)
                    );
                }
                assert_eq!($transform::IDENTITY.inverse(), Some($transform::IDENTITY));
            }

            #[test]
            fn inverse_without_rotation_may_scale_non_uniformly() {
                let transform = $transform::new(
                    $v3::new(1.0, 2.0, 3.0),
                    $quat::IDENTITY,
                    $v3::new(2.0, 4.0, 0.5),
                );
                let inverse = transform.inverse().unwrap();
                let point = $v3::new(5.0, -1.0, 2.0);
                assert!(close(
                    inverse.transform_point(transform.transform_point(point)),
                    point
                ));
            }

            #[test]
            fn inverse_refuses_shear_and_zero_scale() {
                let skewed = $transform::new(
                    $v3::ZERO,
                    $quat::from_rotation_z(Radians::new(0.5)),
                    $v3::new(1.0, 2.0, 3.0),
                );
                assert_eq!(skewed.inverse(), None);
                assert_eq!(
                    $transform::from_scale($v3::new(1.0, 0.0, 1.0)).inverse(),
                    None
                );
                assert_eq!(
                    $transform::from_scale($v3::new(1.0, <$scalar>::INFINITY, 1.0)).inverse(),
                    None
                );
                // Such a transform can still be undone point by point.
                let point = $v3::new(1.0, 2.0, 3.0);
                assert!(close(
                    skewed.inverse_transform_point(skewed.transform_point(point)),
                    point
                ));
            }

            #[test]
            fn mul_transform_matches_the_matrix_product_for_a_uniform_scale() {
                let mut rng = Rng::new(6);
                for _ in 0..100 {
                    let a = random_transform(&mut rng, true);
                    let b = random_transform(&mut rng, false);
                    let product = a * b;
                    let expected = a.to_mat4() * b.to_mat4();
                    assert!(
                        product.to_mat4().approx_eq_eps(expected, 1e-2),
                        "{product:?}"
                    );
                    assert_eq!(a.mul_transform(b), product);
                }
            }

            #[test]
            fn mul_transform_is_exact_without_rotation_in_the_right_operand() {
                let a = $transform::new(
                    $v3::new(1.0, 2.0, 3.0),
                    $quat::from_rotation_x(Radians::new(0.8)),
                    $v3::new(1.0, 2.0, 3.0),
                );
                let b = $transform::new(
                    $v3::new(4.0, 5.0, 6.0),
                    $quat::IDENTITY,
                    $v3::new(2.0, 1.0, 0.5),
                );
                assert!(
                    (a * b)
                        .to_mat4()
                        .approx_eq_eps(a.to_mat4() * b.to_mat4(), 1e-4)
                );
            }

            #[test]
            fn mul_transform_applies_the_right_operand_first() {
                let a = $transform::from_translation($v3::new(10.0, 0.0, 0.0));
                let b = $transform::from_scale($v3::splat(2.0));
                let point = $v3::new(1.0, 0.0, 0.0);
                assert_eq!((a * b).transform_point(point), $v3::new(12.0, 0.0, 0.0));
                assert_eq!((b * a).transform_point(point), $v3::new(22.0, 0.0, 0.0));
            }

            #[test]
            fn lerp_interpolates_each_part() {
                let a = $transform::new($v3::ZERO, $quat::IDENTITY, $v3::ONE);
                let b = $transform::new(
                    $v3::new(4.0, 0.0, 0.0),
                    $quat::from_rotation_z(Radians::new(1.0)),
                    $v3::splat(3.0),
                );
                assert_eq!(a.lerp(b, 0.0), a);
                let end = a.lerp(b, 1.0);
                assert!(close(end.translation, b.translation) && close(end.scale, b.scale));
                assert!(end.rotation.angle_between(b.rotation).value() < TOL);
                let mid = a.lerp(b, 0.5);
                assert!(close(mid.translation, $v3::new(2.0, 0.0, 0.0)));
                assert!(close(mid.scale, $v3::splat(2.0)));
                assert!(
                    mid.rotation
                        .angle_between($quat::from_rotation_z(Radians::new(0.5)))
                        .value()
                        < TOL
                );
            }

            #[test]
            fn directions_follow_the_rotation() {
                let identity = $transform::IDENTITY;
                assert_eq!(identity.right(), $v3::X);
                assert_eq!(identity.up(), $v3::Y);
                assert_eq!(identity.forward(), -$v3::Z);
                let turned = $transform::from_rotation($quat::from_rotation_y(Radians::new(
                    <$scalar as crate::Scalar>::PI / 2.0,
                )));
                assert!(close(turned.forward(), -$v3::X));
                assert!(close(turned.right(), -$v3::Z));
                assert!(close(turned.up(), $v3::Y));
            }

            #[test]
            fn looking_at_points_forward_at_the_target() {
                let eye = $v3::new(3.0, 4.0, 5.0);
                let target = $v3::new(-2.0, 1.0, 0.5);
                let transform = $transform::from_translation(eye).looking_at(target, $v3::Y);
                assert!(close(transform.forward(), (target - eye).normalize()));
                assert!(transform.up().dot($v3::Y) > 0.0);
                assert!(transform.right().dot(transform.up()).abs() < TOL);
                assert_eq!(transform.translation, eye);
                let to = $transform::IDENTITY.looking_to($v3::new(0.0, 0.0, -5.0), $v3::Y);
                assert!(to.rotation.angle_between($quat::IDENTITY).value() < TOL);
            }

            #[test]
            fn uniform_scale_check() {
                assert!($transform::IDENTITY.is_uniform_scale());
                assert!($transform::from_scale($v3::splat(3.0)).is_uniform_scale());
                assert!(!$transform::from_scale($v3::new(1.0, 1.0, 1.1)).is_uniform_scale());
            }

            #[test]
            fn finite_check() {
                assert!(
                    !$transform::from_translation($v3::new(<$scalar>::NAN, 0.0, 0.0)).is_finite()
                );
                assert!(!$transform::from_scale($v3::splat(<$scalar>::INFINITY)).is_finite());
            }
        }
    };
}

transform_tests!(transform_f32, f32, Transform, Vec3, Quat, Mat4);
transform_tests!(transform_f64, f64, DTransform, DVec3, DQuat, DMat4);
