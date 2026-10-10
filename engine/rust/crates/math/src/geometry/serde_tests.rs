//! Tests for the `serde` support (feature `serde`): the compact array form
//! of the vectors, matrices and quaternions, and the derived form of the
//! geometry types.

use super::*;
use crate::angle::{Degrees, Radians};
use crate::{
    DMat2, DMat3, DMat4, DQuat, DVec2, DVec3, DVec4, IVec2, IVec3, IVec4, Mat2, Mat3, Mat4, Quat,
    UVec2, UVec3, UVec4, Vec2, Vec3, Vec4,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Serializes `value` to JSON and reads it back.
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug>(value: T) -> String {
    let json = serde_json::to_string(&value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(back, value, "{json}");
    json
}

#[test]
fn vectors_are_flat_arrays() {
    assert_eq!(round_trip(Vec2::new(1.0, 2.0)), "[1.0,2.0]");
    assert_eq!(round_trip(Vec3::new(1.0, 2.5, -3.0)), "[1.0,2.5,-3.0]");
    assert_eq!(
        round_trip(Vec4::new(1.0, 2.0, 3.0, 4.0)),
        "[1.0,2.0,3.0,4.0]"
    );
    assert_eq!(round_trip(DVec2::new(0.5, 0.25)), "[0.5,0.25]");
    assert_eq!(round_trip(DVec3::new(1.0, 2.0, 3.0)), "[1.0,2.0,3.0]");
    assert_eq!(
        round_trip(DVec4::new(1.0, 2.0, 3.0, 4.0)),
        "[1.0,2.0,3.0,4.0]"
    );
    assert_eq!(round_trip(IVec2::new(-1, 2)), "[-1,2]");
    assert_eq!(round_trip(IVec3::new(1, 2, 3)), "[1,2,3]");
    assert_eq!(round_trip(IVec4::new(1, 2, 3, 4)), "[1,2,3,4]");
    assert_eq!(round_trip(UVec2::new(1, 2)), "[1,2]");
    assert_eq!(round_trip(UVec3::new(1, 2, 3)), "[1,2,3]");
    assert_eq!(round_trip(UVec4::new(1, 2, 3, 4)), "[1,2,3,4]");
}

#[test]
fn matrices_are_flat_arrays_column_by_column() {
    // The first column is (1, 2), the second (3, 4).
    let m2 = Mat2::from_cols_array([1.0, 2.0, 3.0, 4.0]);
    assert_eq!(round_trip(m2), "[1.0,2.0,3.0,4.0]");
    assert_eq!(
        round_trip(DMat2::from_cols_array([1.0, 2.0, 3.0, 4.0])),
        "[1.0,2.0,3.0,4.0]"
    );
    let m3 = Mat3::from_cols_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    assert_eq!(round_trip(m3), "[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0]");
    round_trip(DMat3::IDENTITY);
    let m4 = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let json = round_trip(m4);
    assert_eq!(
        json,
        "[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,1.0,2.0,3.0,1.0]"
    );
    round_trip(DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn quaternions_are_xyzw_arrays() {
    assert_eq!(
        round_trip(Quat::new(0.0, 0.5, 0.0, 1.0)),
        "[0.0,0.5,0.0,1.0]"
    );
    // Values that binary floating point holds exactly: the default `f64`
    // parser of `serde_json` is not guaranteed to round-trip every digit.
    round_trip(DQuat::new(0.5, 0.25, -0.125, 1.0));
    round_trip(Quat::from_rotation_z(Degrees::new(30.0)));
}

#[test]
fn a_wrong_length_or_type_is_an_error() {
    assert!(serde_json::from_str::<Vec3>("[1.0,2.0]").is_err());
    assert!(serde_json::from_str::<Vec3>("[1.0,2.0,3.0,4.0]").is_err());
    assert!(serde_json::from_str::<Vec3>("{\"x\":1.0,\"y\":2.0,\"z\":3.0}").is_err());
    assert!(serde_json::from_str::<Mat4>("[1.0,2.0,3.0]").is_err());
    assert!(serde_json::from_str::<Quat>("\"identity\"").is_err());
    assert!(serde_json::from_str::<UVec2>("[-1,2]").is_err());
}

#[test]
fn geometry_types_round_trip() {
    round_trip(Aabb2::new(Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0)));
    round_trip(Aabb3::new(
        Vec3::new(-1.0, -2.0, -3.0),
        Vec3::new(1.0, 2.0, 3.0),
    ));
    round_trip(DAabb3::new(DVec3::splat(-1.0), DVec3::splat(1.0)));
    round_trip(Sphere::new(Vec3::new(1.0, 2.0, 3.0), 4.0));
    round_trip(DSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0));
    round_trip(Plane::from_point_normal(Vec3::ONE, Vec3::Y));
    round_trip(Ray::new(Vec3::new(1.0, 2.0, 3.0), Vec3::X));
    round_trip(DRay::new(DVec3::ZERO, DVec3::Z));
}

#[test]
fn geometry_types_use_named_fields_around_the_arrays() {
    let json = round_trip(Aabb3::new(
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(1.0, 2.0, 3.0),
    ));
    assert_eq!(json, "{\"min\":[-1.0,0.0,1.0],\"max\":[1.0,2.0,3.0]}");
    let json = round_trip(Sphere::new(Vec3::new(1.0, 2.0, 3.0), 4.0));
    assert_eq!(json, "{\"center\":[1.0,2.0,3.0],\"radius\":4.0}");
    let json = round_trip(Plane::new(Vec3::Y, -2.0));
    assert_eq!(json, "{\"normal\":[0.0,1.0,0.0],\"d\":-2.0}");
}

#[test]
fn frustum_and_transform_round_trip() {
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let projection = Mat4::perspective_rh(Degrees::new(60.0), 1.5, 0.5, 50.0);
    let frustum = Frustum::from_view_projection(projection * view);
    let json = round_trip(frustum);
    assert!(json.starts_with("{\"planes\":[{\"normal\":["));
    round_trip(DFrustum::from_planes([
        DPlane::new(DVec3::X, 1.0),
        DPlane::new(-DVec3::X, 1.0),
        DPlane::new(DVec3::Y, 0.5),
        DPlane::new(-DVec3::Y, 0.5),
        DPlane::new(DVec3::Z, 0.25),
        DPlane::new(-DVec3::Z, 8.0),
    ]));

    let transform = Transform::new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_rotation_y(Radians::new(0.5)),
        Vec3::new(1.0, 2.0, 3.0),
    );
    let json = round_trip(transform);
    assert!(json.starts_with("{\"translation\":[1.0,2.0,3.0],\"rotation\":["));
    round_trip(DTransform::IDENTITY);
    round_trip(DTransform::new(
        DVec3::new(1.5, -2.0, 0.25),
        DQuat::new(0.5, 0.5, 0.5, 0.5),
        DVec3::new(2.0, 1.0, 0.5),
    ));
}

#[test]
fn deserialized_geometry_behaves_like_the_original() {
    let aabb = Aabb3::new(Vec3::splat(-1.0), Vec3::splat(1.0));
    let json = serde_json::to_string(&aabb).unwrap();
    let back: Aabb3 = serde_json::from_str(&json).unwrap();
    let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);
    assert_eq!(ray.intersect_aabb(back), ray.intersect_aabb(aabb));
}
