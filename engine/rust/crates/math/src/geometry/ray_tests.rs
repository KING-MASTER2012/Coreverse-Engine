//! Tests for the rays and their intersection tests. Instantiated once per
//! float family.

use super::*;
use crate::angle::Degrees;
use crate::test_support::Rng;
use crate::{DMat4, DVec2, DVec3, Mat4, Vec2, Vec3};

macro_rules! ray_tests {
    (
        $module:ident, $scalar:ty, $ray:ident, $aabb:ident, $sphere:ident, $plane:ident,
        $v2:ident, $v3:ident, $m4:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;

            fn unit_box() -> $aabb {
                $aabb::new($v3::splat(-1.0), $v3::splat(1.0))
            }

            fn random_vector(rng: &mut Rng, extent: f64) -> $v3 {
                $v3::new(
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                )
            }

            fn near(a: $scalar, b: $scalar) -> bool {
                (a - b).abs() < TOL
            }

            // ---- construction ------------------------------------------

            #[test]
            fn at_walks_along_the_direction() {
                let ray = $ray::new($v3::new(1.0, 2.0, 3.0), $v3::new(0.0, 2.0, 0.0));
                assert_eq!(ray.at(0.0), $v3::new(1.0, 2.0, 3.0));
                assert_eq!(ray.at(1.5), $v3::new(1.0, 5.0, 3.0));
            }

            #[test]
            fn from_points_normalizes() {
                let ray =
                    $ray::from_points($v3::new(1.0, 1.0, 1.0), $v3::new(1.0, 1.0, 6.0)).unwrap();
                assert_eq!(ray.direction, $v3::Z);
                assert_eq!(ray.origin, $v3::ONE);
                assert!($ray::from_points($v3::ONE, $v3::ONE).is_none());
                assert!($ray::from_points($v3::ONE, $v3::new(<$scalar>::NAN, 0.0, 0.0)).is_none());
            }

            #[test]
            fn validity() {
                assert!($ray::new($v3::ZERO, $v3::X).is_valid());
                assert!(!$ray::new($v3::ZERO, $v3::ZERO).is_valid());
                assert!(!$ray::new($v3::ZERO, $v3::new(<$scalar>::NAN, 0.0, 0.0)).is_valid());
                assert!(!$ray::new($v3::new(<$scalar>::INFINITY, 0.0, 0.0), $v3::X).is_valid());
            }

            // ---- ray against box -----------------------------------------

            #[test]
            fn aabb_hit_from_outside() {
                let ray = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::X);
                assert_eq!(ray.intersect_aabb(unit_box()), Some((4.0, 6.0)));
                let ray = $ray::new($v3::new(0.0, 0.0, 10.0), -$v3::Z);
                assert_eq!(ray.intersect_aabb(unit_box()), Some((9.0, 11.0)));
            }

            #[test]
            fn aabb_origin_inside_starts_at_zero() {
                let ray = $ray::new($v3::new(0.5, 0.0, 0.0), $v3::X);
                assert_eq!(ray.intersect_aabb(unit_box()), Some((0.0, 0.5)));
            }

            #[test]
            fn aabb_behind_and_missing() {
                let behind = $ray::new($v3::new(5.0, 0.0, 0.0), $v3::X);
                assert_eq!(behind.intersect_aabb(unit_box()), None);
                let beside = $ray::new($v3::new(-5.0, 1.5, 0.0), $v3::X);
                assert_eq!(beside.intersect_aabb(unit_box()), None);
                let diagonal_miss = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::new(1.0, 1.0, 0.0));
                assert_eq!(diagonal_miss.intersect_aabb(unit_box()), None);
            }

            #[test]
            fn aabb_diagonal_hit() {
                let ray = $ray::from_points($v3::new(-4.0, -4.0, -4.0), $v3::ZERO).unwrap();
                let (near_t, far_t) = ray.intersect_aabb(unit_box()).unwrap();
                let length = <$scalar as crate::Scalar>::sqrt(3.0);
                assert!(near(near_t, 3.0 * length));
                assert!(near(far_t, 5.0 * length));
            }

            #[test]
            fn aabb_parallel_to_a_face() {
                // Between the two planes of the axes it runs parallel to.
                let inside = $ray::new($v3::new(-5.0, 0.5, -0.5), $v3::X);
                assert_eq!(inside.intersect_aabb(unit_box()), Some((4.0, 6.0)));
                // On the face plane itself counts as a hit (inclusive).
                let on_face = $ray::new($v3::new(-5.0, 1.0, 0.0), $v3::X);
                assert_eq!(on_face.intersect_aabb(unit_box()), Some((4.0, 6.0)));
                // Outside the planes it never hits.
                let outside = $ray::new($v3::new(-5.0, 1.0001, 0.0), $v3::X);
                assert_eq!(outside.intersect_aabb(unit_box()), None);
            }

            #[test]
            fn aabb_touching_a_corner_or_edge_hits() {
                let corner = $ray::new($v3::new(-2.0, 1.0, 1.0), $v3::new(1.0, 0.0, 0.0));
                assert!(corner.intersect_aabb(unit_box()).is_some());
                let grazing =
                    $ray::from_points($v3::new(-2.0, 0.0, 1.0), $v3::new(0.0, 0.0, 1.0)).unwrap();
                assert!(grazing.intersect_aabb(unit_box()).is_some());
            }

            #[test]
            fn aabb_t_scales_with_a_non_unit_direction() {
                let unit = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::X);
                let double = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::new(2.0, 0.0, 0.0));
                assert_eq!(unit.intersect_aabb(unit_box()), Some((4.0, 6.0)));
                assert_eq!(double.intersect_aabb(unit_box()), Some((2.0, 3.0)));
            }

            #[test]
            fn aabb_rejects_invalid_input() {
                let ray = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::X);
                assert_eq!(ray.intersect_aabb($aabb::EMPTY), None);
                assert_eq!(ray.intersect_aabb($aabb::new($v3::ONE, $v3::ZERO)), None);
                assert_eq!(
                    $ray::new($v3::ZERO, $v3::ZERO).intersect_aabb(unit_box()),
                    None
                );
                assert_eq!(
                    $ray::new($v3::ZERO, $v3::new(<$scalar>::NAN, 0.0, 0.0))
                        .intersect_aabb(unit_box()),
                    None
                );
            }

            #[test]
            fn aabb_matches_sampling_for_random_rays() {
                let mut rng = Rng::new(31);
                for _ in 0..400 {
                    let aabb = $aabb::from_center_half_extents(
                        random_vector(&mut rng, 3.0),
                        random_vector(&mut rng, 2.0).abs() + $v3::splat(0.1),
                    );
                    let origin = random_vector(&mut rng, 8.0);
                    let direction = random_vector(&mut rng, 1.0);
                    if direction.length() < 0.1 {
                        continue;
                    }
                    let ray = $ray::new(origin, direction.normalize());
                    let result = ray.intersect_aabb(aabb);
                    if let Some((enter, leave)) = result {
                        assert!(0.0 <= enter && enter <= leave);
                        let slack = aabb.expand(1e-3);
                        assert!(slack.contains_point(ray.at(enter)));
                        assert!(slack.contains_point(ray.at(leave)));
                        assert!(aabb.contains_point(ray.at((enter + leave) * 0.5)));
                    } else {
                        for step in 0..=400 {
                            let point = ray.at(step as $scalar * 0.05);
                            assert!(!aabb.contains_point(point), "missed {point:?}");
                        }
                    }
                }
            }

            // ---- ray against sphere --------------------------------------

            #[test]
            fn sphere_hit_from_outside() {
                let sphere = $sphere::new($v3::ZERO, 1.0);
                let ray = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::X);
                let (enter, leave) = ray.intersect_sphere(sphere).unwrap();
                assert!(near(enter, 4.0) && near(leave, 6.0));
            }

            #[test]
            fn sphere_origin_inside_starts_at_zero() {
                let sphere = $sphere::new($v3::ZERO, 2.0);
                let ray = $ray::new($v3::new(1.0, 0.0, 0.0), $v3::X);
                let (enter, leave) = ray.intersect_sphere(sphere).unwrap();
                assert_eq!(enter, 0.0);
                assert!(near(leave, 1.0));
            }

            #[test]
            fn sphere_tangent_miss_and_behind() {
                let sphere = $sphere::new($v3::ZERO, 1.0);
                let tangent = $ray::new($v3::new(-5.0, 1.0, 0.0), $v3::X);
                let (enter, leave) = tangent.intersect_sphere(sphere).unwrap();
                assert!(near(enter, 5.0) && near(leave, 5.0));
                let miss = $ray::new($v3::new(-5.0, 1.01, 0.0), $v3::X);
                assert_eq!(miss.intersect_sphere(sphere), None);
                let behind = $ray::new($v3::new(5.0, 0.0, 0.0), $v3::X);
                assert_eq!(behind.intersect_sphere(sphere), None);
            }

            #[test]
            fn sphere_t_scales_with_a_non_unit_direction() {
                let sphere = $sphere::new($v3::ZERO, 1.0);
                let ray = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::new(2.0, 0.0, 0.0));
                let (enter, leave) = ray.intersect_sphere(sphere).unwrap();
                assert!(near(enter, 2.0) && near(leave, 3.0));
            }

            #[test]
            fn sphere_rejects_invalid_input() {
                let ray = $ray::new($v3::new(-5.0, 0.0, 0.0), $v3::X);
                assert_eq!(ray.intersect_sphere($sphere::new($v3::ZERO, -1.0)), None);
                assert_eq!(
                    ray.intersect_sphere($sphere::new($v3::ZERO, <$scalar>::NAN)),
                    None
                );
                assert_eq!(
                    $ray::new($v3::ZERO, $v3::ZERO).intersect_sphere($sphere::new($v3::ZERO, 1.0)),
                    None
                );
            }

            #[test]
            fn sphere_hit_points_are_on_the_surface() {
                let mut rng = Rng::new(41);
                for _ in 0..200 {
                    let sphere =
                        $sphere::new(random_vector(&mut rng, 3.0), rng.range(0.2, 3.0) as $scalar);
                    let ray = $ray::new(
                        random_vector(&mut rng, 8.0),
                        random_vector(&mut rng, 1.0).normalize(),
                    );
                    if let Some((enter, leave)) = ray.intersect_sphere(sphere) {
                        assert!(0.0 <= enter && enter <= leave);
                        assert!(sphere.distance_to_point(ray.at(leave)).abs() < 1e-3);
                        if enter > 0.0 {
                            assert!(sphere.distance_to_point(ray.at(enter)).abs() < 1e-3);
                        } else {
                            assert!(sphere.distance_to_point(ray.origin) <= 1e-3);
                        }
                    }
                }
            }

            // ---- ray against plane ---------------------------------------

            #[test]
            fn plane_hit_from_either_side() {
                let plane = $plane::from_point_normal($v3::new(0.0, 2.0, 0.0), $v3::Y);
                let from_below = $ray::new($v3::ZERO, $v3::Y);
                assert_eq!(from_below.intersect_plane(plane), Some(2.0));
                let from_above = $ray::new($v3::new(1.0, 7.0, 1.0), -$v3::Y);
                assert_eq!(from_above.intersect_plane(plane), Some(5.0));
                let slanted = $ray::from_points($v3::ZERO, $v3::new(3.0, 2.0, 0.0)).unwrap();
                let t = slanted.intersect_plane(plane).unwrap();
                assert!(plane.signed_distance(slanted.at(t)).abs() < TOL);
            }

            #[test]
            fn plane_behind_parallel_and_invalid() {
                let plane = $plane::from_point_normal($v3::new(0.0, 2.0, 0.0), $v3::Y);
                assert_eq!($ray::new($v3::ZERO, -$v3::Y).intersect_plane(plane), None);
                assert_eq!($ray::new($v3::ZERO, $v3::X).intersect_plane(plane), None);
                // Lying in the plane is parallel too.
                assert_eq!(
                    $ray::new($v3::new(0.0, 2.0, 0.0), $v3::X).intersect_plane(plane),
                    None
                );
                assert_eq!($ray::new($v3::ZERO, $v3::ZERO).intersect_plane(plane), None);
                // Starting on the plane hits at once.
                assert_eq!(
                    $ray::new($v3::new(0.0, 2.0, 0.0), $v3::Y).intersect_plane(plane),
                    Some(0.0)
                );
            }

            // ---- transforming a ray --------------------------------------

            #[test]
            fn transformed_ray_keeps_the_parameter() {
                let model = $m4::from_translation($v3::new(10.0, 0.0, 0.0))
                    * $m4::from_scale($v3::splat(2.0));
                let world_sphere = $sphere::new($v3::new(10.0, 0.0, 0.0), 2.0);
                let local_sphere = $sphere::new($v3::ZERO, 1.0);
                let world_ray =
                    $ray::from_points($v3::new(0.0, 0.5, 0.0), $v3::new(10.0, 0.5, 0.0)).unwrap();
                let local_ray = world_ray.transformed(model.inverse_affine().unwrap());

                let world = world_ray.intersect_sphere(world_sphere).unwrap();
                let local = local_ray.intersect_sphere(local_sphere).unwrap();
                assert!(near(world.0, local.0) && near(world.1, local.1));
            }

            #[test]
            fn transformed_moves_the_origin_and_turns_the_direction() {
                let ray = $ray::new($v3::new(1.0, 0.0, 0.0), $v3::X);
                let matrix = $m4::from_translation($v3::new(0.0, 5.0, 0.0));
                let moved = ray.transformed(matrix);
                assert_eq!(moved.origin, $v3::new(1.0, 5.0, 0.0));
                assert_eq!(moved.direction, $v3::X);
            }

            // ---- rays through the screen ---------------------------------

            fn camera_ray(ndc: $v2) -> ($ray, $v3) {
                // The camera sits at (0, 0, 5) and looks at the origin.
                let view = $m4::look_at_rh($v3::new(0.0, 0.0, 5.0), $v3::ZERO, $v3::Y);
                let projection = $m4::perspective_rh(Degrees::new(60.0), 1.5, 0.1, 100.0);
                let inverse = (projection * view).inverse().unwrap();
                let ray = $ray::from_ndc(inverse, ndc).unwrap();
                (ray, $v3::new(0.0, 0.0, 5.0))
            }

            #[test]
            fn from_ndc_center_looks_along_the_camera() {
                let (ray, eye) = camera_ray($v2::ZERO);
                assert!((ray.direction + $v3::Z).length() < TOL);
                // It starts on the near plane, 0.1 in front of the eye.
                assert!((ray.origin - (eye - $v3::Z * 0.1)).length() < TOL);
                assert!(near(ray.direction.length(), 1.0));
            }

            #[test]
            fn from_ndc_edges_follow_the_field_of_view() {
                let tan_half = crate::trig::tan(Degrees::<$scalar>::new(30.0));
                let (right, _) = camera_ray($v2::new(1.0, 0.0));
                // At the right edge x / depth = tan(fov / 2) * aspect.
                assert!(near(right.direction.x / -right.direction.z, tan_half * 1.5));
                let (top, _) = camera_ray($v2::new(0.0, 1.0));
                assert!(near(top.direction.y / -top.direction.z, tan_half));
                let (left, _) = camera_ray($v2::new(-1.0, 0.0));
                assert!(left.direction.x < 0.0 && near(left.direction.x, -right.direction.x));
            }

            #[test]
            fn from_ndc_picks_the_object_in_the_middle() {
                let (ray, _) = camera_ray($v2::ZERO);
                let (enter, _) = ray.intersect_sphere($sphere::new($v3::ZERO, 1.0)).unwrap();
                // The origin is on the near plane, 4.9 in front of the sphere center.
                assert!((enter - 3.9).abs() < 1e-3);
            }

            #[test]
            fn from_ndc_gl_and_reverse_z_give_the_same_ray() {
                let view = $m4::look_at_rh($v3::new(1.0, 2.0, 5.0), $v3::ZERO, $v3::Y);
                let fov = Degrees::new(50.0);
                let standard = (($m4::perspective_rh(fov, 1.2, 0.5, 80.0)) * view)
                    .inverse()
                    .unwrap();
                let gl = (($m4::perspective_rh_gl(fov, 1.2, 0.5, 80.0)) * view)
                    .inverse()
                    .unwrap();
                let reverse = (($m4::perspective_rh_reverse_z(fov, 1.2, 0.5, 80.0)) * view)
                    .inverse()
                    .unwrap();
                for ndc in [$v2::ZERO, $v2::new(0.4, -0.7), $v2::new(-1.0, 1.0)] {
                    let a = $ray::from_ndc(standard, ndc).unwrap();
                    let b = $ray::from_ndc_gl(gl, ndc).unwrap();
                    let c = $ray::from_ndc_reverse_z(reverse, ndc).unwrap();
                    for other in [b, c] {
                        assert!((a.origin - other.origin).length() < 1e-3, "{ndc:?}");
                        assert!((a.direction - other.direction).length() < 1e-3, "{ndc:?}");
                    }
                }
            }

            #[test]
            fn from_ndc_of_a_singular_matrix_is_none() {
                assert!($ray::from_ndc($m4::ZERO, $v2::ZERO).is_none());
            }
        }
    };
}

ray_tests!(ray_f32, f32, Ray, Aabb3, Sphere, Plane, Vec2, Vec3, Mat4);
ray_tests!(
    ray_f64, f64, DRay, DAabb3, DSphere, DPlane, DVec2, DVec3, DMat4
);
