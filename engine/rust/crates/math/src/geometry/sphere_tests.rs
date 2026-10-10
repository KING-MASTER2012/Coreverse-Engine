//! Tests for the spheres. Instantiated once per float family.

use super::*;
use crate::test_support::Rng;
use crate::{DVec3, Vec3};

macro_rules! sphere_tests {
    ($module:ident, $scalar:ty, $sphere:ident, $aabb:ident, $plane:ident, $v3:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;
            const PI: $scalar = <$scalar as crate::Scalar>::PI;

            fn random_point(rng: &mut Rng) -> $v3 {
                $v3::new(
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                )
            }

            #[test]
            fn measures() {
                let unit = $sphere::new($v3::ZERO, 1.0);
                assert!((unit.volume() - 4.0 * PI / 3.0).abs() < TOL);
                assert!((unit.surface_area() - 4.0 * PI).abs() < TOL);
                let sphere = $sphere::new($v3::new(1.0, 2.0, 3.0), 2.0);
                assert_eq!(
                    sphere.aabb(),
                    $aabb::new($v3::new(-1.0, 0.0, 1.0), $v3::new(3.0, 4.0, 5.0))
                );
                assert_eq!(
                    sphere.translated($v3::X),
                    $sphere::new($v3::new(2.0, 2.0, 3.0), 2.0)
                );
            }

            #[test]
            fn validity() {
                assert!($sphere::new($v3::ZERO, 0.0).is_valid());
                assert!($sphere::new($v3::ONE, 3.0).is_valid());
                assert!(!$sphere::new($v3::ZERO, -1.0).is_valid());
                assert!(!$sphere::new($v3::ZERO, <$scalar>::NAN).is_valid());
                assert!(!$sphere::new($v3::new(<$scalar>::INFINITY, 0.0, 0.0), 1.0).is_valid());
            }

            #[test]
            fn contains_point_includes_the_surface() {
                let sphere = $sphere::new($v3::new(1.0, 0.0, 0.0), 2.0);
                assert!(sphere.contains_point($v3::new(1.0, 0.0, 0.0)));
                assert!(sphere.contains_point($v3::new(3.0, 0.0, 0.0)));
                assert!(sphere.contains_point($v3::new(-1.0, 0.0, 0.0)));
                assert!(!sphere.contains_point($v3::new(3.01, 0.0, 0.0)));
            }

            #[test]
            fn sphere_against_sphere() {
                let a = $sphere::new($v3::ZERO, 2.0);
                let inner = $sphere::new($v3::new(0.5, 0.0, 0.0), 1.0);
                let internally_tangent = $sphere::new($v3::new(1.0, 0.0, 0.0), 1.0);
                let touching = $sphere::new($v3::new(3.0, 0.0, 0.0), 1.0);
                let apart = $sphere::new($v3::new(3.1, 0.0, 0.0), 1.0);
                assert!(a.contains_sphere(inner) && !inner.contains_sphere(a));
                assert!(a.contains_sphere(internally_tangent));
                assert!(a.contains_sphere(a));
                assert!(!a.contains_sphere(touching));
                assert!(a.intersects_sphere(inner));
                assert!(a.intersects_sphere(touching) && touching.intersects_sphere(a));
                assert!(!a.intersects_sphere(apart));
            }

            #[test]
            fn sphere_against_aabb() {
                let aabb = $aabb::new($v3::splat(-1.0), $v3::splat(1.0));
                assert!($sphere::new($v3::ZERO, 0.5).intersects_aabb(aabb));
                assert!($sphere::new($v3::new(3.0, 0.0, 0.0), 2.0).intersects_aabb(aabb));
                assert!(!$sphere::new($v3::new(3.0, 0.0, 0.0), 1.9).intersects_aabb(aabb));
                // The corner (1, 1, 1) is sqrt(3) from (2, 2, 2).
                let center = $v3::splat(2.0);
                let reach = <$scalar as crate::Scalar>::sqrt(3.0);
                assert!($sphere::new(center, reach + 0.001).intersects_aabb(aabb));
                assert!(!$sphere::new(center, reach - 0.001).intersects_aabb(aabb));
            }

            #[test]
            fn sphere_against_plane() {
                let plane = $plane::from_point_normal($v3::ZERO, $v3::Y);
                assert!($sphere::new($v3::new(0.0, 0.5, 0.0), 1.0).intersects_plane(plane));
                assert!($sphere::new($v3::new(0.0, 1.0, 0.0), 1.0).intersects_plane(plane));
                assert!($sphere::new($v3::new(0.0, -0.9, 0.0), 1.0).intersects_plane(plane));
                assert!(!$sphere::new($v3::new(0.0, 1.5, 0.0), 1.0).intersects_plane(plane));
                assert!(!$sphere::new($v3::new(0.0, -1.5, 0.0), 1.0).intersects_plane(plane));
            }

            #[test]
            fn distance_is_signed() {
                let sphere = $sphere::new($v3::ZERO, 2.0);
                assert_eq!(sphere.distance_to_point($v3::new(5.0, 0.0, 0.0)), 3.0);
                assert_eq!(sphere.distance_to_point($v3::new(2.0, 0.0, 0.0)), 0.0);
                assert_eq!(sphere.distance_to_point($v3::new(1.0, 0.0, 0.0)), -1.0);
                assert_eq!(sphere.distance_to_point($v3::ZERO), -2.0);
            }

            #[test]
            fn closest_point() {
                let sphere = $sphere::new($v3::new(1.0, 0.0, 0.0), 2.0);
                let inside = $v3::new(1.5, 0.5, 0.0);
                assert_eq!(sphere.closest_point(inside), inside);
                assert_eq!(
                    sphere.closest_point($v3::new(11.0, 0.0, 0.0)),
                    $v3::new(3.0, 0.0, 0.0)
                );
                let outside = $v3::new(1.0, 7.0, 0.0);
                let closest = sphere.closest_point(outside);
                assert!((closest - $v3::new(1.0, 2.0, 0.0)).length() < TOL);
            }

            #[test]
            fn union_point_grows_just_enough() {
                let sphere = $sphere::new($v3::ZERO, 1.0);
                assert_eq!(sphere.union_point($v3::new(0.5, 0.0, 0.0)), sphere);
                let grown = sphere.union_point($v3::new(5.0, 0.0, 0.0));
                assert!((grown.radius - 3.0).abs() < TOL);
                assert!((grown.center - $v3::new(2.0, 0.0, 0.0)).length() < TOL);
                assert!(
                    grown.contains_point($v3::new(5.0, 0.0, 0.0))
                        || grown.distance_to_point($v3::new(5.0, 0.0, 0.0)) < TOL
                );
                // The far side of the old sphere is still inside, on the surface.
                assert!(grown.distance_to_point($v3::new(-1.0, 0.0, 0.0)) < TOL);
            }

            #[test]
            fn union_of_spheres() {
                let big = $sphere::new($v3::ZERO, 5.0);
                let small = $sphere::new($v3::new(1.0, 0.0, 0.0), 1.0);
                assert_eq!(big.union(small), big);
                assert_eq!(small.union(big), big);
                assert_eq!(big.union(big), big);

                let a = $sphere::new($v3::new(-3.0, 0.0, 0.0), 1.0);
                let b = $sphere::new($v3::new(5.0, 0.0, 0.0), 2.0);
                let union = a.union(b);
                // From x = -4 to x = 7: the diameter spans both spheres.
                assert!((union.radius - 5.5).abs() < TOL);
                assert!((union.center - $v3::new(1.5, 0.0, 0.0)).length() < TOL);
                assert!(
                    union.contains_sphere(a)
                        || union.distance_to_point($v3::new(-4.0, 0.0, 0.0)) < TOL
                );
                let commuted = b.union(a);
                assert!((commuted.radius - union.radius).abs() < TOL);
                assert!((commuted.center - union.center).length() < TOL);
            }

            #[test]
            fn from_points_contains_every_point() {
                assert_eq!($sphere::from_points(&[]), None);
                let single = $sphere::from_points(&[$v3::new(1.0, 2.0, 3.0)]).unwrap();
                assert_eq!(single, $sphere::new($v3::new(1.0, 2.0, 3.0), 0.0));
                let pair = $sphere::from_points(&[$v3::ZERO, $v3::new(4.0, 0.0, 0.0)]).unwrap();
                assert!((pair.radius - 2.0).abs() < TOL);
                assert!((pair.center - $v3::new(2.0, 0.0, 0.0)).length() < TOL);

                let mut rng = Rng::new(23);
                for _ in 0..50 {
                    let points: [$v3; 40] = core::array::from_fn(|_| random_point(&mut rng));
                    let sphere = $sphere::from_points(&points).unwrap();
                    for point in points {
                        assert!(
                            sphere.distance_to_point(point) < 1e-3,
                            "{sphere:?} {point:?}"
                        );
                    }
                    // It is no larger than the sphere around the bounding box.
                    let bounds = $aabb::from_points(&points).unwrap().bounding_sphere();
                    assert!(sphere.radius <= bounds.radius * 1.001);
                }
            }
        }
    };
}

sphere_tests!(sphere_f32, f32, Sphere, Aabb3, Plane, Vec3);
sphere_tests!(sphere_f64, f64, DSphere, DAabb3, DPlane, DVec3);
