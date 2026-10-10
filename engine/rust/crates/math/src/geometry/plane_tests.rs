//! Tests for the planes. Instantiated once per float family.

use super::*;
use crate::angle::Radians;
use crate::test_support::Rng;
use crate::{DMat4, DQuat, DVec3, Mat4, Quat, Vec3};

macro_rules! plane_tests {
    ($module:ident, $scalar:ty, $plane:ident, $v3:ident, $m4:ident, $quat:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;

            fn random_point(rng: &mut Rng) -> $v3 {
                $v3::new(
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                )
            }

            #[test]
            fn from_point_normal_normalizes_and_contains_the_point() {
                let point = $v3::new(1.0, 2.0, 3.0);
                let plane = $plane::from_point_normal(point, $v3::new(0.0, 0.0, 5.0));
                assert!(plane.is_normalized());
                assert_eq!(plane.normal, $v3::Z);
                assert!(plane.signed_distance(point).abs() < TOL);
                assert_eq!(plane.d, -3.0);
            }

            #[test]
            fn signed_distance_is_positive_on_the_normal_side() {
                let plane = $plane::from_point_normal($v3::new(0.0, 2.0, 0.0), $v3::Y);
                assert_eq!(plane.signed_distance($v3::new(5.0, 7.0, -1.0)), 5.0);
                assert_eq!(plane.signed_distance($v3::new(5.0, 2.0, -1.0)), 0.0);
                assert_eq!(plane.signed_distance($v3::new(5.0, -1.0, -1.0)), -3.0);
            }

            #[test]
            fn from_points_faces_the_counter_clockwise_side() {
                let plane = $plane::from_points($v3::ZERO, $v3::X, $v3::Y).unwrap();
                assert!((plane.normal - $v3::Z).length() < TOL);
                assert!(plane.d.abs() < TOL);
                let clockwise = $plane::from_points($v3::ZERO, $v3::Y, $v3::X).unwrap();
                assert!((clockwise.normal + $v3::Z).length() < TOL);
                // The offset follows the points: the same triangle at z = 4.
                let high = $plane::from_points(
                    $v3::new(0.0, 0.0, 4.0),
                    $v3::new(1.0, 0.0, 4.0),
                    $v3::new(0.0, 1.0, 4.0),
                )
                .unwrap();
                assert!(high.signed_distance($v3::new(7.0, 7.0, 4.0)).abs() < TOL);
                assert!((high.signed_distance($v3::new(0.0, 0.0, 6.0)) - 2.0).abs() < TOL);
            }

            #[test]
            fn from_points_rejects_degenerate_triangles() {
                assert!($plane::from_points($v3::ZERO, $v3::X, $v3::new(2.0, 0.0, 0.0)).is_none());
                assert!($plane::from_points($v3::ONE, $v3::ONE, $v3::X).is_none());
                assert!(
                    $plane::from_points($v3::ZERO, $v3::X, $v3::new(<$scalar>::NAN, 0.0, 1.0))
                        .is_none()
                );
            }

            #[test]
            fn from_coefficients_scales_to_a_unit_normal() {
                let plane = $plane::from_coefficients(0.0, 0.0, 2.0, -8.0).unwrap();
                assert_eq!(plane.normal, $v3::Z);
                assert_eq!(plane.d, -4.0);
                assert!($plane::from_coefficients(0.0, 0.0, 0.0, 1.0).is_none());
                assert!($plane::from_coefficients(<$scalar>::NAN, 0.0, 1.0, 1.0).is_none());
                assert!($plane::from_coefficients(<$scalar>::INFINITY, 0.0, 1.0, 1.0).is_none());
            }

            #[test]
            fn normalized_keeps_the_same_plane() {
                let stretched = $plane::new($v3::new(0.0, 0.0, 2.0), -4.0);
                assert!(!stretched.is_normalized());
                let plane = stretched.normalized().unwrap();
                assert!(plane.is_normalized());
                assert_eq!(
                    plane,
                    $plane::from_point_normal($v3::new(9.0, 9.0, 2.0), $v3::Z)
                );
                assert!($plane::new($v3::ZERO, 1.0).normalized().is_none());
            }

            #[test]
            fn flipped_swaps_the_sides() {
                let plane = $plane::from_point_normal($v3::ONE, $v3::new(1.0, 2.0, 3.0));
                let flipped = plane.flipped();
                let point = $v3::new(5.0, -1.0, 2.0);
                assert!(
                    (flipped.signed_distance(point) + plane.signed_distance(point)).abs() < TOL
                );
                assert_eq!(flipped.flipped(), plane);
            }

            #[test]
            fn closest_point_lies_on_the_plane() {
                let mut rng = Rng::new(3);
                for _ in 0..100 {
                    let plane =
                        $plane::from_point_normal(random_point(&mut rng), random_point(&mut rng));
                    let point = random_point(&mut rng);
                    let closest = plane.closest_point(point);
                    assert!(plane.signed_distance(closest).abs() < TOL);
                    assert!(
                        ((point - closest).length() - plane.signed_distance(point).abs()).abs()
                            < TOL
                    );
                }
            }

            #[test]
            fn closest_point_works_for_a_stretched_normal_too() {
                let plane = $plane::new($v3::new(0.0, 0.0, 2.0), -4.0);
                assert_eq!(
                    plane.closest_point($v3::new(1.0, 1.0, 5.0)),
                    $v3::new(1.0, 1.0, 2.0)
                );
                assert_eq!(plane.point_on_plane(), $v3::new(0.0, 0.0, 2.0));
            }

            #[test]
            fn point_on_plane_is_the_one_nearest_to_the_origin() {
                let plane =
                    $plane::from_point_normal($v3::new(3.0, 4.0, 5.0), $v3::new(1.0, 1.0, 0.0));
                let point = plane.point_on_plane();
                assert!(plane.signed_distance(point).abs() < TOL);
                assert!((point - plane.closest_point($v3::ZERO)).length() < TOL);
            }

            #[test]
            fn transformed_plane_contains_the_transformed_points() {
                let mut rng = Rng::new(17);
                for _ in 0..100 {
                    let points = [
                        random_point(&mut rng),
                        random_point(&mut rng),
                        random_point(&mut rng),
                    ];
                    let Some(plane) = $plane::from_points(points[0], points[1], points[2]) else {
                        continue;
                    };
                    let axis = random_point(&mut rng).normalize();
                    let matrix = $m4::from_scale_rotation_translation(
                        $v3::new(
                            rng.range(0.3, 3.0) as $scalar,
                            rng.range(0.3, 3.0) as $scalar,
                            rng.range(0.3, 3.0) as $scalar,
                        ),
                        $quat::from_axis_angle(axis, Radians::new(rng.range(0.0, 6.28) as $scalar)),
                        random_point(&mut rng),
                    );
                    let moved = plane.transformed(matrix).unwrap();
                    assert!(moved.is_normalized());
                    for point in points {
                        assert!(moved.signed_distance(matrix.transform_point3(point)).abs() < 1e-3);
                    }
                }
            }

            #[test]
            fn transformed_plane_keeps_its_facing_side() {
                let plane = $plane::from_point_normal($v3::ZERO, $v3::Y);
                let matrix = $m4::from_translation($v3::new(0.0, 5.0, 0.0));
                let moved = plane.transformed(matrix).unwrap();
                assert!((moved.signed_distance($v3::new(0.0, 6.0, 0.0)) - 1.0).abs() < TOL);
                assert!((moved.signed_distance($v3::new(0.0, 4.0, 0.0)) + 1.0).abs() < TOL);
                assert!(
                    plane
                        .transformed($m4::from_scale($v3::new(1.0, 0.0, 1.0)))
                        .is_none()
                );
            }

            #[test]
            fn finite_check() {
                assert!($plane::new($v3::Y, 1.0).is_finite());
                assert!(!$plane::new($v3::Y, <$scalar>::NAN).is_finite());
                assert!(!$plane::new($v3::splat(<$scalar>::INFINITY), 0.0).is_finite());
            }
        }
    };
}

plane_tests!(plane_f32, f32, Plane, Vec3, Mat4, Quat);
plane_tests!(plane_f64, f64, DPlane, DVec3, DMat4, DQuat);
