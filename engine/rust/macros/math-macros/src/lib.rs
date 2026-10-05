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
//! The vector-generating macro is [`vectors!`] (Phase 1.7, with the vector
//! operations of Phase 1.8 and the additions of Phase 2.2); the matrix-generating macro is [`matrices!`]
//! (Phase 1.9).

use proc_macro::TokenStream;

mod matrix;
mod vector;
mod vector_ext;
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
/// - component-wise `Mul` and `Div` between two vectors of the same type,
///   with their `*Assign` forms, and `Display` (`[x, y, z]`);
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
/// - the additions of Phase 2.2, on every type: `min_element`,
///   `max_element`, `element_sum`, `element_product`, `extend` and
///   `truncate` / `xy` / `xyz` where the size fits, and one `as_<prefix><dim>`
///   conversion (`as_dvec3`, ...) to every *other* family of the same
///   invocation; on float and signed-integer types `abs` and `signum`; on
///   integer types `rem_euclid`, `div_euclid` and `abs_diff` (a signed
///   family returns the unsigned family of the same width, so it only gets
///   `abs_diff` when that family is in the invocation); on float types
///   `floor`, `ceil`, `round`, `trunc`, `fract`, `lerp`, `is_finite`,
///   `is_nan` and `approx_eq` / `approx_eq_eps` / `approx_eq_rel`; on the
///   2-component floats `perp`, `perp_dot`, `rotate` and `from_angle`; on the
///   3-component floats `any_orthogonal` and `orthonormal_basis`.
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
/// `floor`, `sin`, ... which `core` lacks, so that they follow `math`'s `std`/`libm` backend) and
/// return `crate::angle::Radians`. These `crate::` paths mean the macro can
/// only be invoked inside the `math` crate itself.
#[proc_macro]
pub fn vectors(input: TokenStream) -> TokenStream {
    vector::vectors_impl(input.into()).into()
}

/// Generates a family of column-major matrix types per
/// `Prefix(VectorPrefix) => scalar` entry.
///
/// Every entry produces `Prefix2`, `Prefix3` and `Prefix4`, whose columns are
/// the `crate::vector::VectorPrefix2`/`3`/`4` vectors of the same scalar, so
/// one invocation can define all of `math`'s matrices:
///
/// ```ignore
/// math_macros::matrices! {
///     Mat(Vec)   => f32,
///     DMat(DVec) => f64,
/// }
/// ```
///
/// defines the 6 types `Mat2`..`DMat4`. Each is a `#[repr(C)]` struct whose
/// public fields `x_axis`, `y_axis` (and `z_axis`, `w_axis`) are the columns,
/// so the memory is the columns one after another, the layout GLSL and Vulkan
/// expect. It has:
///
/// - the `IDENTITY` and `ZERO` associated constants, and `const fn`
///   `from_cols`, `from_cols_array`, `to_cols_array`, `from_cols_array_2d`,
///   `to_cols_array_2d`, `col` and `row`;
/// - `transpose`, `determinant` and `inverse` (`None` for a singular
///   matrix), `mul_mat` and `mul_vec`, all `const fn`;
/// - `Mul<matrix>` and `Mul<column vector>` (`m * v` transforms a column
///   vector, `a * b` applies `b` first), `MulAssign<matrix>`, `Index<usize>`
///   and `IndexMut<usize>` (by column), `From` conversions to and from
///   `[[scalar; N]; N]`, and `Default` (the identity).
///
/// The scalar must be `f32` or `f64`. Anything else, an empty list, a
/// repeated prefix or a malformed entry is a compile error pointing at the
/// offending entry. The generated code names `::core` items, `crate::vector`
/// (for the column types) and nothing else, and all of it carries
/// documentation (the generated types satisfy `#![deny(missing_docs)]`). The
/// `crate::vector` paths mean the macro can only be invoked inside the `math`
/// crate itself.
#[proc_macro]
pub fn matrices(input: TokenStream) -> TokenStream {
    matrix::matrices_impl(input.into()).into()
}
