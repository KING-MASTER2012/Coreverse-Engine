//! Tests for the axis-aligned boxes. Instantiated once per float family.

use super::*;
use crate::angle::Radians;
use crate::test_support::Rng;
use crate::{DMat4, DQuat, DVec2, DVec3, Mat4, Quat, Vec2, Vec3};

macro_rules! aabb_tests {
    (
        $module:ident, $scalar:ty, $aabb2:ident, $aabb3:ident, $v2:ident, $v3:ident,
        $m4:ident, $quat:ident, $sphere:ident, $plane:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-3;

            fn unit_box() -> $aabb3 {
                $aabb3::new($v3::new(-1.0, -2.0, -3.0), $v3::new(1.0, 2.0, 3.0))
            }

            fn random_point(rng: &mut Rng) -> $v3 {
                $v3::new(
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                    rng.range(-10.0, 10.0) as $scalar,
                )
            }

            fn random_box(rng: &mut Rng) -> $aabb3 {
                let a = random_point(rng);
                let b = random_point(rng);
                $aabb3::new(a.min(b), a.max(b))
            }

            fn random_matrix(rng: &mut Rng) -> $m4 {
                let axis = $v3::new(
                    rng.range(-1.0, 1.0) as $scalar,
                    rng.range(-1.0, 1.0) as $scalar,
                    rng.range(-1.0, 1.0) as $scalar,
                )
                .normalize();
                $m4::from_scale_rotation_translation(
                    $v3::new(
                        rng.range(0.3, 3.0) as $scalar,
                        rng.range(0.3, 3.0) as $scalar,
                        rng.range(0.3, 3.0) as $scalar,
                    ),
                    $quat::from_axis_angle(axis, Radians::new(rng.range(0.0, 6.28) as $scalar)),
                    random_point(rng),
                )
            }

            #[test]
            fn measures() {
                let aabb = unit_box();
                assert_eq!(aabb.size(), $v3::new(2.0, 4.0, 6.0));
                assert_eq!(aabb.half_extents(), $v3::new(1.0, 2.0, 3.0));
                assert_eq!(aabb.center(), $v3::ZERO);
                assert_eq!(aabb.volume(), 48.0);
                assert_eq!(aabb.surface_area(), 2.0 * (8.0 + 24.0 + 12.0));
                let shifted =
                    $aabb3::from_center_size($v3::new(1.0, 1.0, 1.0), $v3::new(2.0, 4.0, 6.0));
                assert_eq!(shifted, aabb.translated($v3::new(1.0, 1.0, 1.0)));
                assert_eq!(
                    $aabb3::from_center_half_extents($v3::ZERO, $v3::new(1.0, 2.0, 3.0)),
                    aabb
                );
            }

            #[test]
            fn measures_2d() {
                let rect = $aabb2::new($v2::new(1.0, 2.0), $v2::new(4.0, 6.0));
                assert_eq!(rect.area(), 12.0);
                assert_eq!(rect.perimeter(), 14.0);
                assert_eq!(rect.center(), $v2::new(2.5, 4.0));
                assert_eq!(
                    rect.corners(),
                    [
                        $v2::new(1.0, 2.0),
                        $v2::new(4.0, 2.0),
                        $v2::new(1.0, 6.0),
                        $v2::new(4.0, 6.0)
                    ]
                );
            }

            #[test]
            fn validity() {
                assert!(unit_box().is_valid());
                assert!($aabb3::from_point($v3::ONE).is_valid());
                assert!(!$aabb3::EMPTY.is_valid());
                assert!(!$aabb3::new($v3::ONE, $v3::ZERO).is_valid());
                let nan = $aabb3::new($v3::new(<$scalar>::NAN, 0.0, 0.0), $v3::ONE);
                assert!(!nan.is_valid());
                assert!(unit_box().is_finite() && !$aabb3::EMPTY.is_finite());
                assert!(!$aabb2::EMPTY.is_valid());
            }

            #[test]
            fn contains_point_is_inclusive() {
                let aabb = unit_box();
                assert!(aabb.contains_point($v3::ZERO));
                assert!(aabb.contains_point($v3::new(1.0, 2.0, 3.0)));
                assert!(aabb.contains_point($v3::new(-1.0, 0.0, 3.0)));
                assert!(!aabb.contains_point($v3::new(1.001, 0.0, 0.0)));
                assert!(!aabb.contains_point($v3::new(0.0, 0.0, -3.5)));
                assert!(!aabb.contains_point($v3::new(<$scalar>::NAN, 0.0, 0.0)));
                assert!(!$aabb3::EMPTY.contains_point($v3::ZERO));
                let rect = $aabb2::new($v2::ZERO, $v2::ONE);
                assert!(
                    rect.contains_point($v2::new(1.0, 0.5))
                        && !rect.contains_point($v2::new(1.5, 0.5))
                );
            }

            #[test]
            fn contains_and_intersects() {
                let big = unit_box();
                let inner = $aabb3::new($v3::splat(-0.5), $v3::splat(0.5));
                let overlapping = $aabb3::new($v3::new(0.0, 0.0, 0.0), $v3::new(5.0, 5.0, 5.0));
                let touching = $aabb3::new($v3::new(1.0, -1.0, -1.0), $v3::new(2.0, 1.0, 1.0));
                let apart = $aabb3::new($v3::new(1.5, 0.0, 0.0), $v3::new(2.0, 1.0, 1.0));

                assert!(big.contains(inner) && !inner.contains(big));
                assert!(big.contains(big));
                assert!(!big.contains(overlapping));
                assert!(big.intersects(inner) && inner.intersects(big));
                assert!(big.intersects(overlapping));
                assert!(big.intersects(touching) && touching.intersects(big));
                assert!(!big.intersects(apart) && !apart.intersects(big));

                // A box that is apart on one axis only does not intersect.
                let above = $aabb3::new($v3::new(0.0, 0.0, 3.5), $v3::new(1.0, 1.0, 4.0));
                assert!(!big.intersects(above));
            }

            #[test]
            fn empty_box_intersects_nothing_and_is_contained_everywhere() {
                let aabb = unit_box();
                assert!(!aabb.intersects($aabb3::EMPTY) && !$aabb3::EMPTY.intersects(aabb));
                assert!(!$aabb3::EMPTY.intersects($aabb3::EMPTY));
                assert!(aabb.contains($aabb3::EMPTY));
                assert!(aabb.intersection($aabb3::EMPTY).is_none());
            }

            #[test]
            fn intersection_is_the_shared_box() {
                let a = unit_box();
                let b = $aabb3::new($v3::new(0.0, 1.0, -5.0), $v3::new(4.0, 4.0, 0.0));
                assert_eq!(
                    a.intersection(b),
                    Some($aabb3::new(
                        $v3::new(0.0, 1.0, -3.0),
                        $v3::new(1.0, 2.0, 0.0)
                    ))
                );
                assert_eq!(a.intersection(b), b.intersection(a));
                let touching = $aabb3::new($v3::new(1.0, 0.0, 0.0), $v3::new(2.0, 1.0, 1.0));
                let flat = a.intersection(touching).unwrap();
                assert_eq!(flat.size().x, 0.0);
                let apart = $aabb3::new($v3::splat(5.0), $v3::splat(6.0));
                assert_eq!(a.intersection(apart), None);
            }

            #[test]
            fn union_and_the_empty_box() {
                let a = unit_box();
                let b = $aabb3::new($v3::new(0.0, 0.0, 0.0), $v3::new(5.0, 1.0, 1.0));
                let union = a.union(b);
                assert_eq!(
                    union,
                    $aabb3::new($v3::new(-1.0, -2.0, -3.0), $v3::new(5.0, 2.0, 3.0))
                );
                assert!(union.contains(a) && union.contains(b));
                assert_eq!(a.union($aabb3::EMPTY), a);
                assert_eq!($aabb3::EMPTY.union(a), a);
                assert_eq!(
                    a.union_point($v3::new(0.0, 0.0, 10.0)).max,
                    $v3::new(1.0, 2.0, 10.0)
                );
                assert_eq!(
                    $aabb3::EMPTY.union_point($v3::new(1.0, 2.0, 3.0)),
                    $aabb3::from_point($v3::new(1.0, 2.0, 3.0))
                );
            }

            #[test]
            fn from_points_is_the_tight_box() {
                assert_eq!($aabb3::from_points(&[]), None);
                let points = [
                    $v3::new(1.0, 5.0, -2.0),
                    $v3::new(-3.0, 0.0, 4.0),
                    $v3::new(2.0, 2.0, 2.0),
                ];
                let aabb = $aabb3::from_points(&points).unwrap();
                assert_eq!(
                    aabb,
                    $aabb3::new($v3::new(-3.0, 0.0, -2.0), $v3::new(2.0, 5.0, 4.0))
                );
                assert!(points.iter().all(|&p| aabb.contains_point(p)));
                assert_eq!(
                    $aabb2::from_points(&[$v2::new(1.0, 2.0), $v2::new(-1.0, 3.0)]),
                    Some($aabb2::new($v2::new(-1.0, 2.0), $v2::new(1.0, 3.0)))
                );
            }

            #[test]
            fn expand_and_translate() {
                let aabb = unit_box();
                assert_eq!(aabb.expand(1.0).size(), aabb.size() + $v3::splat(2.0));
                assert_eq!(aabb.expand(1.0).center(), aabb.center());
                assert!(!aabb.expand(-2.5).is_valid());
                assert_eq!(
                    aabb.translated($v3::new(1.0, 0.0, 0.0)).min,
                    $v3::new(0.0, -2.0, -3.0)
                );
            }

            #[test]
            fn closest_point_and_distance() {
                let aabb = unit_box();
                let inside = $v3::new(0.5, 0.5, 0.5);
                assert_eq!(aabb.closest_point(inside), inside);
                assert_eq!(aabb.distance_to_point(inside), 0.0);
                // In front of a face: the distance is the gap.
                let face = $v3::new(4.0, 0.0, 0.0);
                assert_eq!(aabb.closest_point(face), $v3::new(1.0, 0.0, 0.0));
                assert_eq!(aabb.distance_to_point(face), 3.0);
                // Beyond a corner: a 3-4 step away from the corner (1, 2, 3).
                let corner = $v3::new(1.0, 5.0, 7.0);
                assert_eq!(aabb.closest_point(corner), $v3::new(1.0, 2.0, 3.0));
                assert_eq!(aabb.distance_to_point(corner), 5.0);
                assert_eq!(aabb.distance_sq_to_point(corner), 25.0);
            }

            #[test]
            fn longest_axis_picks_the_first_on_a_tie() {
                assert_eq!(unit_box().longest_axis(), 2);
                let flat = $aabb3::new($v3::ZERO, $v3::new(5.0, 5.0, 1.0));
                assert_eq!(flat.longest_axis(), 0);
                assert_eq!($aabb3::from_point($v3::ONE).longest_axis(), 0);
                assert_eq!($aabb2::new($v2::ZERO, $v2::new(1.0, 3.0)).longest_axis(), 1);
            }

            #[test]
            fn corners_are_the_eight_vertices() {
                let aabb = unit_box();
                let corners = aabb.corners();
                assert_eq!(corners[0], aabb.min);
                assert_eq!(corners[7], aabb.max);
                assert_eq!(corners[1], $v3::new(1.0, -2.0, -3.0));
                assert_eq!(corners[2], $v3::new(-1.0, 2.0, -3.0));
                assert_eq!(corners[4], $v3::new(-1.0, -2.0, 3.0));
                for (i, a) in corners.iter().enumerate() {
                    assert!(aabb.contains_point(*a));
                    for b in &corners[i + 1..] {
                        assert_ne!(a, b);
                    }
                }
                assert_eq!($aabb3::from_points(&corners), Some(aabb));
            }

            #[test]
            fn transform_is_the_box_of_the_transformed_corners() {
                let mut rng = Rng::new(11);
                for _ in 0..200 {
                    let aabb = random_box(&mut rng);
                    let matrix = random_matrix(&mut rng);
                    let expected = {
                        let mut transformed = aabb.corners();
                        for corner in &mut transformed {
                            *corner = matrix.transform_point3(*corner);
                        }
                        $aabb3::from_points(&transformed).unwrap()
                    };
                    let actual = aabb.transform(matrix);
                    assert!(
                        actual.approx_eq_eps(expected, TOL),
                        "{actual:?} {expected:?}"
                    );
                }
            }

            #[test]
            fn transform_by_a_translation_moves_the_box() {
                let aabb = unit_box();
                let moved = aabb.transform($m4::from_translation($v3::new(1.0, 2.0, 3.0)));
                assert!(moved.approx_eq_eps(aabb.translated($v3::new(1.0, 2.0, 3.0)), TOL));
                assert!(
                    !$aabb3::EMPTY
                        .transform($m4::from_translation($v3::ONE))
                        .is_valid()
                );
                assert_eq!($aabb3::EMPTY.transform($m4::IDENTITY), $aabb3::EMPTY);
            }

            #[test]
            fn bounding_sphere_contains_the_box() {
                let mut rng = Rng::new(5);
                for _ in 0..100 {
                    let aabb = random_box(&mut rng);
                    let sphere = aabb.bounding_sphere();
                    assert_eq!(sphere.center, aabb.center());
                    for corner in aabb.corners() {
                        assert!(
                            sphere.contains_point(corner) || sphere.distance_to_point(corner) < TOL
                        );
                    }
                }
            }

            #[test]
            fn sphere_tests() {
                let aabb = unit_box();
                assert!(aabb.intersects_sphere($sphere::new($v3::ZERO, 0.1)));
                assert!(aabb.intersects_sphere($sphere::new($v3::new(3.0, 0.0, 0.0), 2.0)));
                assert!(!aabb.intersects_sphere($sphere::new($v3::new(3.0, 0.0, 0.0), 1.9)));
                // Beyond the corner (1, 2, 3) at distance 5.
                let corner = $v3::new(1.0, 5.0, 7.0);
                assert!(aabb.intersects_sphere($sphere::new(corner, 5.0)));
                assert!(!aabb.intersects_sphere($sphere::new(corner, 4.9)));
            }

            #[test]
            fn plane_tests() {
                let aabb = unit_box();
                let through = $plane::from_point_normal($v3::ZERO, $v3::Y);
                assert!(aabb.intersects_plane(through));
                let touching = $plane::from_point_normal($v3::new(0.0, 2.0, 0.0), $v3::Y);
                assert!(aabb.intersects_plane(touching));
                let above = $plane::from_point_normal($v3::new(0.0, 2.5, 0.0), $v3::Y);
                assert!(!aabb.intersects_plane(above));
                // A diagonal plane just clips the corner (1, 2, 3).
                let normal = $v3::new(1.0, 1.0, 1.0).normalize();
                let corner_plane = $plane::from_point_normal($v3::new(1.0, 2.0, 3.0), normal);
                assert!(aabb.intersects_plane(corner_plane));
                let past = $plane::from_point_normal($v3::new(1.1, 2.1, 3.1), normal);
                assert!(!aabb.intersects_plane(past));
            }

            #[test]
            fn approximate_equality() {
                let a = unit_box();
                assert!(a.approx_eq_eps(a.translated($v3::splat(1e-5)), 1e-4));
                assert!(!a.approx_eq_eps(a.translated($v3::splat(1e-2)), 1e-4));
            }
        }
    };
}

aabb_tests!(
    aabb_f32, f32, Aabb2, Aabb3, Vec2, Vec3, Mat4, Quat, Sphere, Plane
);
aabb_tests!(
    aabb_f64, f64, DAabb2, DAabb3, DVec2, DVec3, DMat4, DQuat, DSphere, DPlane
);
