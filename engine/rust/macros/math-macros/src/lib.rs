//! Proc-macros that generate `math`'s concrete vector and matrix type
//! families (e.g. `Vec2`/`Vec3`/`Vec4`/`DVec2`/... and `Mat2`/`Mat3`/`Mat4`/...)
//! from a single definition each, instead of hand-duplicating the same
//! fields, constructors and operator impls once per scalar type.
//!
//! Kept in `engine/rust/macros/` (not `engine/rust/crates/`) alongside
//! `vfs-macros`, matching this workspace's convention of keeping proc-macro
//! crates - which must always compile as their own separate crate - out of
//! the regular crate tree.
//!
//! The vector-generating macro is [`vectors!`] (Phase 1.7); the
//! matrix-generating macro lands in Phase 1.9.

use proc_macro::TokenStream;

mod vector;
mod vector_ops;

/// Generates a family of vector types per `Prefix => scalar` entry.
///
/// Every entry produces `Prefix2`, `Prefix3` and `Prefix4`, so one
/// invocation can define all of `math`'s vectors:
///
/// ```ignore
/// math_macros::vectors! {
///     Vec  => f32,
///     DVec => f64,
///     IVec => i32,
///     UVec => u32,
/// }
/// ```
///
/// defines the 12 types `Vec2`..`UVec4`. Each is a `#[repr(C)]` struct with
/// public `x`, `y` (and `z`, `w`) fields and:
///
/// - `ZERO`, `ONE` and the unit axes `X`, `Y`, ... as associated constants,
///   and `const fn` `new`, `splat`, `from_array` and `to_array`;
/// - `From` conversions to and from `[scalar; N]`;
/// - component-wise `Add` and `Sub`, `Mul<scalar>` and `Div<scalar>`, their
///   `*Assign` forms, and `scalar * vector`;
/// - `Index<usize>` and `IndexMut<usize>`;
/// - `Neg`, for float and signed-integer scalars only;
/// - the operations of Phase 1.8, on every type: `dot`, `length_sq`,
///   `distance_sq` and the component-wise `min`, `max` and `clamp` (all
///   `const fn`); on the 3-component float and signed-integer types also
///   `cross`; and on float types only `length`, `distance`, `normalize`,
///   `try_normalize`, `normalize_or_zero`, `project_onto`, `reject_from`,
///   `reflect`, `refract` and `angle_between`. Unsigned types get no `cross`
///   (its subtraction would underflow), and integer types get nothing that
///   would return a non-integer.
///
/// Float and integer entries also differ in their derives: integer vectors
/// are additionally `Eq` and `Hash`.
///
/// The scalar must be one of `f32`, `f64`, `i8`..`i128`, `isize`,
/// `u8`..`u128` or `usize`. Anything else, an empty list or a repeated
/// prefix is a compile error pointing at the offending entry. The generated
/// code only names `::core` items, so it works in `no_std` crates, and all
/// of it carries documentation (the generated types satisfy
/// `#![deny(missing_docs)]`).
///
/// The float operations call `crate::scalar::Scalar` (for `sqrt`/`atan2`,
/// which `core` lacks, so that they follow `math`'s `std`/`libm` backend) and
/// return `crate::angle::Radians`. These `crate::` paths mean the macro can
/// only be invoked inside the `math` crate itself.
#[proc_macro]
pub fn vectors(input: TokenStream) -> TokenStream {
    vector::vectors_impl(input.into()).into()
}
