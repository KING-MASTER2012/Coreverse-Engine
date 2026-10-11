//! Packing for vertex and GPU formats (Phase 2.9): 16-bit half floats
//! ([`F16`], [`f32_to_f16_bits`]), normalised integers ([`pack_unorm8`],
//! [`pack_snorm16`], [`pack_unorm4x8`], [`pack_snorm_1010102`], ...) and
//! octahedral unit-vector encoding ([`octahedral_encode`],
//! [`pack_octahedral16`]).
//!
//! GPU memory and bandwidth are the scarce resources of a renderer, and most
//! vertex and texture data does not need 32-bit floats: a colour fits in four
//! bytes, a normal in two or four, a texture coordinate in two half floats.
//! This module converts between the engine's `f32` values and those compact
//! forms, with the exact rounding and layout the graphics APIs define, so
//! the bytes you write match what the shader reads.
//!
//! ```
//! use math::{Vec2, Vec3, Vec4};
//! use math::pack::{pack_half2, pack_octahedral16, pack_unorm4x8, unpack_octahedral16};
//!
//! // A vertex's 32 bytes of floats in 12: a colour, a texcoord and a normal.
//! let colour = pack_unorm4x8(Vec4::new(1.0, 0.5, 0.0, 1.0)); // 4 bytes
//! let uv = pack_half2(Vec2::new(0.25, 0.75)); // 4 bytes
//! let normal = pack_octahedral16(Vec3::new(0.0, 0.6, 0.8)); // 4 bytes
//!
//! assert_eq!(colour, 0xFF00_80FF);
//! let decoded = unpack_octahedral16(normal);
//! assert!((decoded - Vec3::new(0.0, 0.6, 0.8)).length() < 1e-4);
//! # let _ = uv;
//! ```
//!
//! # Contents
//!
//! - **Half floats** ([`F16`] and the `f32_to_f16_bits` family): IEEE 754
//!   `binary16`, converted bit-exactly with round-to-nearest-even, including
//!   subnormals, infinities and `NaN`s; [`pack_half2`] and [`pack_half4`] lay
//!   them out like `R16G16_FLOAT` and `R16G16B16A16_FLOAT`.
//! - **Normalised integers**: *unorm* maps `0..=1` onto `0..=2^n - 1` and
//!   *snorm* maps `-1..=1` onto `-(2^(n-1) - 1)..=2^(n-1) - 1` (so `-1.0` and
//!   the most negative integer both decode to `-1.0`). Scalars of 2, 8, 10
//!   and 16 bits, and the packed vector formats `4x8`, `2x16` and
//!   `10_10_10_2` (the bit layouts of GLSL's `packUnorm4x8`,
//!   `packSnorm2x16`, and the Vulkan/D3D `A2B10G10R10` formats).
//! - **Octahedral normals**: a unit vector in two numbers instead of three,
//!   which packs into 16 or 32 bits with far lower error than quantising `x`,
//!   `y` and `z` separately.
//!
//! # Rounding
//!
//! Converting *to* a normalised integer clamps to the representable range
//! (so out-of-range input saturates, and a `NaN` becomes `0`) and rounds to
//! the nearest value, ties away from zero (`0.5 * 255` packs to `128`).
//! Converting *from* one is `integer / max`, exact to `f32` precision, so
//! every packed value survives a round trip unchanged.
//!
//! # `const`
//!
//! Everything except the octahedral functions (which need a square root) is a
//! `const fn`, so tables of packed constants can be built at compile time.

mod half;
mod norm;
mod octahedral;

pub use half::{
    F16, f16_bits_to_f32, f16_bits_to_f64, f16_slice_to_f32, f32_slice_to_f16, f32_to_f16_bits,
    f64_to_f16_bits, pack_half2, pack_half4, unpack_half2, unpack_half4,
};
pub use norm::{
    pack_snorm_1010102, pack_snorm2, pack_snorm2x16, pack_snorm4x8, pack_snorm8, pack_snorm10,
    pack_snorm16, pack_unorm_1010102, pack_unorm2, pack_unorm2x16, pack_unorm4x8, pack_unorm8,
    pack_unorm10, pack_unorm16, unpack_snorm_1010102, unpack_snorm2, unpack_snorm2x16,
    unpack_snorm4x8, unpack_snorm8, unpack_snorm10, unpack_snorm16, unpack_unorm_1010102,
    unpack_unorm2, unpack_unorm2x16, unpack_unorm4x8, unpack_unorm8, unpack_unorm10,
    unpack_unorm16,
};
pub use octahedral::{
    octahedral_decode, octahedral_encode, pack_octahedral8, pack_octahedral16, unpack_octahedral8,
    unpack_octahedral16,
};

#[cfg(test)]
mod tests;
