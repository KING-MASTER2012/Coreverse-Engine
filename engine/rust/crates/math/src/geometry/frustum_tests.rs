//! Tests for the view frustum. Instantiated once per float family.

use super::*;
use crate::angle::{Degrees, Radians};
use crate::test_support::Rng;
use crate::{DMat4, DQuat, DVec3, DVec4, Mat4, Quat, Vec3, Vec4};

macro_rules! frustum_tests {
    (
        $module:ident, $scalar:ty, $frustum:ident, $aabb:ident, $sphere:ident, $plane:ident,
        $v3:ident, $v4:ident, $m4:ident, $quat:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-3;
            const NEAR: $scalar = 0.5;
            const FAR: $scalar = 50.0;

            /// A camera at (0, 0, 10) looking at the origin (down -Z).
            fn view() -> $m4 {
                $m4::look_at_rh($v3::new(0.0, 0.0, 10.0), $v3::ZERO, $v3::Y)
            }

            fn projection() -> $m4 {
                $m4::perspective_rh(Degrees::new(60.0), 1.5, NEAR, FAR)
            }

            fn frustum() -> $frustum {
                $frustum::from_view_projection(projection() * view())
            }

            fn random_point(rng: &mut Rng, extent: f64) -> $v3 {
                $v3::new(
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                    rng.range(-extent, extent) as $scalar,
                )
            }

            #[test]
            fn planes_are_unit_and_face_inwards() {
                let frustum = frustum();
                assert!(frustum.is_finite());
                for plane in &frustum.planes {
                    assert!(plane.is_normalized());
                }
                let inside = $v3::ZERO;
                for plane in &frustum.planes {
                    assert!(plane.signed_distance(inside) > 0.0);
                }
            }

            #[test]
            fn near_and_far_planes_sit_at_the_right_distances() {
                let frustum = frustum();
                let eye = $v3::new(0.0, 0.0, 10.0);
                let near = frustum.planes[$frustum::NEAR];
                let far = frustum.planes[$frustum::FAR];
                assert!((near.normal + $v3::Z).length() < TOL, "near faces -Z");
                assert!((near.signed_distance(eye) + NEAR).abs() < TOL);
                assert!(
                    (far.normal - $v3::Z).length() < TOL,
                    "far faces back at the camera"
                );
                assert!((far.signed_distance(eye) - FAR).abs() < TOL);
                // The side planes pass through the eye.
                for index in [
                    $frustum::LEFT,
                    $frustum::RIGHT,
                    $frustum::BOTTOM,
                    $frustum::TOP,
                ] {
                    assert!(frustum.planes[index].signed_distance(eye).abs() < TOL);
                }
            }

            #[test]
            fn contains_point_matches_the_view_volume() {
                let frustum = frustum();
                assert!(frustum.contains_point($v3::ZERO));
                assert!(frustum.contains_point($v3::new(2.0, 1.0, 0.0)));
                // Behind the camera, before the near plane, past the far plane.
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, 20.0)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, 10.0 - NEAR + 0.01)));
                assert!(frustum.contains_point($v3::new(0.0, 0.0, 10.0 - NEAR - 0.01)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, 10.0 - FAR - 0.01)));
                assert!(frustum.contains_point($v3::new(0.0, 0.0, 10.0 - FAR + 0.01)));
                // The half height at depth 10 is 10 * tan(30 deg), the width 1.5 times that.
                let half_height = 10.0 * crate::trig::tan(Degrees::<$scalar>::new(30.0));
                assert!(frustum.contains_point($v3::new(0.0, half_height - 0.01, 0.0)));
                assert!(!frustum.contains_point($v3::new(0.0, half_height + 0.01, 0.0)));
                assert!(frustum.contains_point($v3::new(1.5 * half_height - 0.01, 0.0, 0.0)));
                assert!(!frustum.contains_point($v3::new(1.5 * half_height + 0.01, 0.0, 0.0)));
                assert!(!frustum.contains_point($v3::new(-1.5 * half_height - 0.01, 0.0, 0.0)));
                assert!(!frustum.contains_point($v3::new(0.0, -half_height - 0.01, 0.0)));
            }

            #[test]
            fn contains_point_agrees_with_the_projection() {
                let view_projection = projection() * view();
                let frustum = $frustum::from_view_projection(view_projection);
                let mut rng = Rng::new(7);
                for _ in 0..500 {
                    let point = random_point(&mut rng, 40.0);
                    let clip = view_projection * $v4::new(point.x, point.y, point.z, 1.0);
                    let by_clip = clip.w > 0.0
                        && clip.x.abs() <= clip.w
                        && clip.y.abs() <= clip.w
                        && clip.z >= 0.0
                        && clip.z <= clip.w;
                    let margin = frustum
                        .planes
                        .iter()
                        .map(|plane| plane.signed_distance(point).abs())
                        .fold(<$scalar>::INFINITY, <$scalar>::min);
                    if margin > 1e-3 {
                        assert_eq!(frustum.contains_point(point), by_clip, "{point:?}");
                    }
                }
            }

            #[test]
            fn sphere_classification() {
                let frustum = frustum();
                assert_eq!(
                    frustum.classify_sphere($sphere::new($v3::ZERO, 1.0)),
                    Containment::Inside
                );
                assert_eq!(
                    frustum.classify_sphere($sphere::new($v3::new(0.0, 0.0, 30.0), 1.0)),
                    Containment::Outside
                );
                assert_eq!(
                    frustum.classify_sphere($sphere::new($v3::new(0.0, 0.0, 10.0 - FAR), 2.0)),
                    Containment::Intersecting
                );
                assert!(
                    frustum.intersects_sphere($sphere::new($v3::new(0.0, 0.0, 10.0 - FAR), 2.0))
                );
                assert!(!frustum.intersects_sphere($sphere::new($v3::new(100.0, 0.0, 0.0), 5.0)));
                // A big sphere around the camera crosses the near plane.
                assert_eq!(
                    frustum.classify_sphere($sphere::new($v3::new(0.0, 0.0, 10.0), 3.0)),
                    Containment::Intersecting
                );
            }

            #[test]
            fn aabb_classification() {
                let frustum = frustum();
                let inside = $aabb::new($v3::splat(-1.0), $v3::splat(1.0));
                assert_eq!(frustum.classify_aabb(inside), Containment::Inside);
                assert!(frustum.intersects_aabb(inside));

                let behind = $aabb::new($v3::new(-1.0, -1.0, 15.0), $v3::new(1.0, 1.0, 16.0));
                assert_eq!(frustum.classify_aabb(behind), Containment::Outside);
                assert!(!frustum.intersects_aabb(behind));

                let beside = $aabb::new($v3::new(50.0, -1.0, -1.0), $v3::new(52.0, 1.0, 1.0));
                assert_eq!(frustum.classify_aabb(beside), Containment::Outside);

                let straddling = $aabb::new($v3::new(8.0, -1.0, -1.0), $v3::new(20.0, 1.0, 1.0));
                assert_eq!(frustum.classify_aabb(straddling), Containment::Intersecting);
                assert!(frustum.intersects_aabb(straddling));

                let huge = $aabb::new($v3::splat(-1000.0), $v3::splat(1000.0));
                assert_eq!(frustum.classify_aabb(huge), Containment::Intersecting);
            }

            #[test]
            fn invalid_boxes_are_outside() {
                let frustum = frustum();
                assert_eq!(frustum.classify_aabb($aabb::EMPTY), Containment::Outside);
                assert!(!frustum.intersects_aabb($aabb::new($v3::ONE, $v3::ZERO)));
            }

            #[test]
            fn aabb_classification_matches_checking_the_corners() {
                let frustum = frustum();
                let mut rng = Rng::new(13);
                for _ in 0..1000 {
                    let center = random_point(&mut rng, 30.0);
                    let half = random_point(&mut rng, 6.0).abs() + $v3::splat(0.05);
                    let aabb = $aabb::from_center_half_extents(center, half);
                    let corners = aabb.corners();
                    let mut outside = false;
                    let mut inside = true;
                    for plane in &frustum.planes {
                        let behind = corners
                            .iter()
                            .filter(|&&c| plane.signed_distance(c) < 0.0)
                            .count();
                        outside |= behind == 8;
                        inside &= behind == 0;
                    }
                    let expected = if outside {
                        Containment::Outside
                    } else if inside {
                        Containment::Inside
                    } else {
                        Containment::Intersecting
                    };
                    assert_eq!(frustum.classify_aabb(aabb), expected, "{aabb:?}");
                }
            }

            #[test]
            fn gl_depth_range_gives_the_same_frustum() {
                let standard = frustum();
                let gl = $frustum::from_view_projection_gl(
                    $m4::perspective_rh_gl(Degrees::new(60.0), 1.5, NEAR, FAR) * view(),
                );
                for (a, b) in standard.planes.iter().zip(gl.planes.iter()) {
                    assert!((a.normal - b.normal).length() < TOL, "{a:?} {b:?}");
                    assert!((a.d - b.d).abs() < TOL, "{a:?} {b:?}");
                }
            }

            #[test]
            fn reverse_z_gives_the_same_volume_with_near_and_far_swapped() {
                let standard = frustum();
                let reverse = $frustum::from_view_projection(
                    $m4::perspective_rh_reverse_z(Degrees::new(60.0), 1.5, NEAR, FAR) * view(),
                );
                let swapped = [$frustum::NEAR, $frustum::FAR];
                assert!(
                    (reverse.planes[swapped[0]].normal - standard.planes[swapped[1]].normal)
                        .length()
                        < TOL
                );
                assert!((reverse.planes[swapped[0]].d - standard.planes[swapped[1]].d).abs() < TOL);
                assert!(
                    (reverse.planes[swapped[1]].normal - standard.planes[swapped[0]].normal)
                        .length()
                        < TOL
                );
                let mut rng = Rng::new(19);
                for _ in 0..200 {
                    let point = random_point(&mut rng, 40.0);
                    assert_eq!(
                        standard.contains_point(point),
                        reverse.contains_point(point)
                    );
                }
            }

            #[test]
            fn left_handed_projection() {
                let view = $m4::look_at_lh($v3::new(0.0, 0.0, -10.0), $v3::ZERO, $v3::Y);
                let projection = $m4::perspective_lh(Degrees::new(60.0), 1.5, NEAR, FAR);
                let frustum = $frustum::from_view_projection(projection * view);
                assert!(frustum.contains_point($v3::ZERO));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, -20.0)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, 45.0)));
            }

            #[test]
            fn orthographic_frustum_is_a_box() {
                let projection = $m4::orthographic_rh(-2.0, 2.0, -1.0, 1.0, 0.5, 10.0);
                let frustum = $frustum::from_view_projection(projection);
                assert!(frustum.contains_point($v3::new(0.0, 0.0, -5.0)));
                assert!(frustum.contains_point($v3::new(1.9, 0.9, -9.9)));
                assert!(!frustum.contains_point($v3::new(2.1, 0.0, -5.0)));
                assert!(!frustum.contains_point($v3::new(0.0, -1.1, -5.0)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, -0.4)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, -10.1)));
                assert!(!frustum.contains_point($v3::new(0.0, 0.0, 1.0)));
            }

            #[test]
            fn transformed_frustum_follows_the_points() {
                let frustum = frustum();
                let mut rng = Rng::new(29);
                for _ in 0..30 {
                    let axis = random_point(&mut rng, 1.0).normalize();
                    let matrix = $m4::from_scale_rotation_translation(
                        $v3::new(
                            rng.range(0.5, 2.0) as $scalar,
                            rng.range(0.5, 2.0) as $scalar,
                            rng.range(0.5, 2.0) as $scalar,
                        ),
                        $quat::from_axis_angle(axis, Radians::new(rng.range(0.0, 6.28) as $scalar)),
                        random_point(&mut rng, 10.0),
                    );
                    let moved = frustum.transformed(matrix).unwrap();
                    for _ in 0..100 {
                        let point = random_point(&mut rng, 40.0);
                        let margin = frustum
                            .planes
                            .iter()
                            .map(|plane| plane.signed_distance(point).abs())
                            .fold(<$scalar>::INFINITY, <$scalar>::min);
                        if margin > 1e-2 {
                            assert_eq!(
                                frustum.contains_point(point),
                                moved.contains_point(matrix.transform_point3(point))
                            );
                        }
                    }
                }
                assert!(
                    frustum
                        .transformed($m4::from_scale($v3::new(1.0, 0.0, 1.0)))
                        .is_none()
                );
            }

            #[test]
            fn degenerate_matrix_gives_non_finite_planes() {
                assert!(!$frustum::from_view_projection($m4::ZERO).is_finite());
            }

            #[test]
            fn from_planes_keeps_the_order() {
                let planes = [
                    $plane::new($v3::X, 1.0),
                    $plane::new($v3::Y, 2.0),
                    $plane::new($v3::Z, 3.0),
                    $plane::new(-$v3::X, 4.0),
                    $plane::new(-$v3::Y, 5.0),
                    $plane::new(-$v3::Z, 6.0),
                ];
                let frustum = $frustum::from_planes(planes);
                assert_eq!(frustum.planes[$frustum::TOP], planes[3]);
                assert_eq!(frustum.planes[$frustum::FAR], planes[5]);
                // A box [-1, 1]^3 built from six planes.
                let cube = $frustum::from_planes([
                    $plane::new($v3::X, 1.0),
                    $plane::new(-$v3::X, 1.0),
                    $plane::new($v3::Y, 1.0),
                    $plane::new(-$v3::Y, 1.0),
                    $plane::new($v3::Z, 1.0),
                    $plane::new(-$v3::Z, 1.0),
                ]);
                assert!(cube.contains_point($v3::new(0.5, -0.5, 1.0)));
                assert!(!cube.contains_point($v3::new(1.5, 0.0, 0.0)));
            }

            #[test]
            fn containment_visibility() {
                assert!(!Containment::Outside.is_visible());
                assert!(Containment::Intersecting.is_visible());
                assert!(Containment::Inside.is_visible());
            }
        }
    };
}

frustum_tests!(
    frustum_f32,
    f32,
    Frustum,
    Aabb3,
    Sphere,
    Plane,
    Vec3,
    Vec4,
    Mat4,
    Quat
);
frustum_tests!(
    frustum_f64,
    f64,
    DFrustum,
    DAabb3,
    DSphere,
    DPlane,
    DVec3,
    DVec4,
    DMat4,
    DQuat
);
