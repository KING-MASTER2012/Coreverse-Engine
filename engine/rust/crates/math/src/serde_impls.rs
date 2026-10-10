//! `serde` support (feature `serde`) for the vectors, matrices and
//! quaternions.
//!
//! They are written as flat arrays instead of structs with named fields:
//! `Vec3` as `[x, y, z]`, `Quat` as `[x, y, z, w]` and a matrix column by
//! column (the order of `to_cols_array`, 4 columns of 4 for a `Mat4`). That
//! is compact in every format and matches how the engine's data files and
//! GPU buffers lay these types out. The geometry types derive `serde`
//! normally and contain these.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Implements `Serialize`/`Deserialize` through an array: `$to` makes the
/// array of type `$array` and `$from` takes it back.
macro_rules! serde_as_array {
    ($($ty:ty => $array:ty, $to:ident, $from:ident;)+) => {
        $(
            impl Serialize for $ty {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.$to().serialize(serializer)
                }
            }

            impl<'de> Deserialize<'de> for $ty {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    <$array>::deserialize(deserializer).map(Self::$from)
                }
            }
        )+
    };
}

serde_as_array! {
    crate::Vec2 => [f32; 2], to_array, from_array;
    crate::Vec3 => [f32; 3], to_array, from_array;
    crate::Vec4 => [f32; 4], to_array, from_array;
    crate::DVec2 => [f64; 2], to_array, from_array;
    crate::DVec3 => [f64; 3], to_array, from_array;
    crate::DVec4 => [f64; 4], to_array, from_array;
    crate::IVec2 => [i32; 2], to_array, from_array;
    crate::IVec3 => [i32; 3], to_array, from_array;
    crate::IVec4 => [i32; 4], to_array, from_array;
    crate::UVec2 => [u32; 2], to_array, from_array;
    crate::UVec3 => [u32; 3], to_array, from_array;
    crate::UVec4 => [u32; 4], to_array, from_array;
    crate::Mat2 => [f32; 4], to_cols_array, from_cols_array;
    crate::Mat3 => [f32; 9], to_cols_array, from_cols_array;
    crate::Mat4 => [f32; 16], to_cols_array, from_cols_array;
    crate::DMat2 => [f64; 4], to_cols_array, from_cols_array;
    crate::DMat3 => [f64; 9], to_cols_array, from_cols_array;
    crate::DMat4 => [f64; 16], to_cols_array, from_cols_array;
    crate::Quat => [f32; 4], to_array, from_array;
    crate::DQuat => [f64; 4], to_array, from_array;
}
