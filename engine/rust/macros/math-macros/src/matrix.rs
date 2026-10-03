//! Implementation of the `matrices!` macro.
//!
//! One invocation lists the matrix families to build, each as
//! `Prefix(VectorPrefix) => scalar`:
//!
//! ```text
//! matrices! {
//!     Mat(Vec)   => f32,
//!     DMat(DVec) => f64,
//! }
//! ```
//!
//! and expands to `Prefix2`, `Prefix3` and `Prefix4` for every entry
//! (`Mat2`..`DMat4`: 6 concrete types here). The columns of `Prefix<n>` are
//! `crate::vector::VectorPrefix<n>` values, so the vector family with that
//! prefix must exist in `math`'s `vector` module and use the same scalar.
//!
//! Matrices are stored column-major: the struct is its columns one after
//! another (`x_axis`, `y_axis`, ...), which is exactly the memory layout of
//! GLSL's `matN`. Element `(row r, column c)` is `column_c.component_r`.
//!
//! Only float scalars are accepted: `determinant` and `inverse` are the
//! point of a matrix here, and integer matrices have neither a meaningful
//! inverse nor anything in the engine that needs them.
//!
//! Everything that does not need `sqrt` is a `const fn`, `inverse` included
//! (it only divides).
//!
//! Like [`crate::vector`], the logic lives in [`matrices_impl`] over
//! `proc_macro2` types so the unit tests at the bottom can run it.

use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Error, Ident, Result, Token};

use crate::vector::COMPONENTS;

/// Column field names in order; a matrix of dimension `n` uses the first `n`.
const AXES: [&str; 4] = ["x_axis", "y_axis", "z_axis", "w_axis"];

/// The dimensions generated for every family.
const DIMENSIONS: [usize; 3] = [2, 3, 4];

const FLOAT_SCALARS: [&str; 2] = ["f32", "f64"];

/// One `Prefix(VectorPrefix) => scalar` entry of the macro input.
struct Family {
    /// Prefix of the generated matrix types (`Mat`, `DMat`).
    prefix: Ident,
    /// Prefix of the vector types used as columns (`Vec`, `DVec`).
    columns: Ident,
    scalar: Ident,
}

impl syn::parse::Parse for Family {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let prefix: Ident = input.parse()?;
        let inner;
        syn::parenthesized!(inner in input);
        let columns: Ident = inner.parse()?;
        if !inner.is_empty() {
            return Err(inner.error("expected a single column vector prefix, e.g. `Mat(Vec)`"));
        }
        input.parse::<Token![=>]>()?;
        let scalar: Ident = input.parse()?;
        if !FLOAT_SCALARS.contains(&scalar.to_string().as_str()) {
            return Err(Error::new_spanned(
                &scalar,
                format!(
                    "unsupported scalar type `{scalar}`; matrices need a float scalar: {}",
                    FLOAT_SCALARS.join(", ")
                ),
            ));
        }
        Ok(Self {
            prefix,
            columns,
            scalar,
        })
    }
}

/// Expands the `matrices!` input into the generated items, or into a
/// `compile_error!` describing what is wrong with the input.
pub(crate) fn matrices_impl(input: TokenStream) -> TokenStream {
    match expand(input) {
        Ok(items) => items,
        Err(error) => error.to_compile_error(),
    }
}

fn expand(input: TokenStream) -> Result<TokenStream> {
    let families = Punctuated::<Family, Token![,]>::parse_terminated.parse2(input)?;
    if families.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "expected at least one `Prefix(VectorPrefix) => scalar` entry, e.g. `Mat(Vec) => f32`",
        ));
    }

    let mut seen: Vec<String> = Vec::new();
    for family in &families {
        let prefix = family.prefix.to_string();
        if seen.contains(&prefix) {
            return Err(Error::new_spanned(
                &family.prefix,
                format!("duplicate matrix family prefix `{prefix}`"),
            ));
        }
        seen.push(prefix);
    }

    let matrices = families
        .iter()
        .flat_map(|family| DIMENSIONS.map(|dim| matrix(family, dim)));
    Ok(quote! { #(#matrices)* })
}

/// Parses generated source text. Used for the dense numeric formulas, which
/// are far easier to read and check as plain Rust than as `quote!` input.
fn parse_text(text: &str) -> TokenStream {
    text.parse()
        .expect("the generated formula text must be valid Rust tokens")
}

/// Everything for one type: `Prefix<dim>` of `dim` columns.
fn matrix(family: &Family, dim: usize) -> TokenStream {
    let Family {
        prefix,
        columns,
        scalar,
    } = family;
    let name = format_ident!("{}{}", prefix, dim);
    let column_name = format_ident!("{}{}", columns, dim);
    let column = quote!(crate::vector::#column_name);
    let axes: Vec<Ident> = AXES[..dim]
        .iter()
        .map(|axis| format_ident!("{}", axis))
        .collect();
    let comps: Vec<Ident> = COMPONENTS[..dim]
        .iter()
        .map(|component| format_ident!("{}", component))
        .collect();
    let unit_consts: Vec<Ident> = COMPONENTS[..dim]
        .iter()
        .map(|component| format_ident!("{}", component.to_uppercase()))
        .collect();
    let indices: Vec<Literal> = (0..dim).map(Literal::usize_unsuffixed).collect();
    let dim_literal = Literal::usize_unsuffixed(dim);
    let len_literal = Literal::usize_unsuffixed(dim * dim);

    let axis_docs: Vec<String> = unit_consts
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            format!("Column {index}: where the matrix sends the unit vector `{unit}`.")
        })
        .collect();

    let struct_doc = format!(
        "A {dim}x{dim} matrix with `{scalar}` elements, stored column-major.\n\n\
         The matrix is {dim} columns, each a [`{column_name}`](crate::vector::{column_name}). \
         With `#[repr(C)]` the memory is simply the columns one after another, the \
         layout GLSL and Vulkan expect for a `mat{dim}`. Element (row `r`, column `c`) \
         is `matrix.col(c)[r]`.\n\n\
         Matrices transform column vectors: `m * v` is the matrix times a vector, and \
         `a * b` is the matrix that applies `b` first and then `a`. Matrix \
         multiplication is not commutative.\n\n\
         `Default` is the identity matrix, not zero."
    );

    // ---- constructors and conversions ---------------------------------

    let identity_cols: Vec<TokenStream> = unit_consts
        .iter()
        .map(|unit| quote!(#column::#unit))
        .collect();
    let zero_cols: Vec<TokenStream> = (0..dim).map(|_| quote!(#column::ZERO)).collect();

    let from_array_cols: Vec<TokenStream> = (0..dim)
        .map(|c| {
            let elements: Vec<TokenStream> = (0..dim)
                .map(|r| {
                    let index = Literal::usize_unsuffixed(c * dim + r);
                    quote!(array[#index])
                })
                .collect();
            quote!(#column::new(#(#elements),*))
        })
        .collect();
    let to_array_elements: Vec<TokenStream> = axes
        .iter()
        .flat_map(|axis| comps.iter().map(move |comp| quote!(self.#axis.#comp)))
        .collect();
    let from_array_2d_cols: Vec<TokenStream> = indices
        .iter()
        .map(|index| quote!(#column::from_array(array[#index])))
        .collect();
    let to_array_2d_cols: Vec<TokenStream> = axes
        .iter()
        .map(|axis| quote!(self.#axis.to_array()))
        .collect();

    // ---- rows, transpose, products ------------------------------------

    let row_exprs: Vec<TokenStream> = comps
        .iter()
        .map(|comp| {
            let elements: Vec<TokenStream> =
                axes.iter().map(|axis| quote!(self.#axis.#comp)).collect();
            quote!(#column::new(#(#elements),*))
        })
        .collect();
    let mul_vec_components: Vec<TokenStream> = comps
        .iter()
        .map(|comp| {
            let terms: Vec<TokenStream> = axes
                .iter()
                .zip(&comps)
                .map(|(axis, vector_comp)| quote!(self.#axis.#comp * rhs.#vector_comp))
                .collect();
            quote!(#(#terms)+*)
        })
        .collect();

    let (determinant_body, inverse_body) = determinant_and_inverse(dim, &column);

    quote! {
        #[doc = #struct_doc]
        #[repr(C)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct #name {
            #(
                #[doc = #axis_docs]
                pub #axes: #column,
            )*
        }

        impl #name {
            /// The identity matrix: it leaves every vector and matrix unchanged
            /// when multiplied with it.
            pub const IDENTITY: Self = Self::from_cols(#(#identity_cols),*);

            /// The matrix with every element zero.
            pub const ZERO: Self = Self::from_cols(#(#zero_cols),*);

            /// Builds a matrix from its columns.
            #[inline]
            pub const fn from_cols(#(#axes: #column),*) -> Self {
                Self { #(#axes),* }
            }

            /// Builds a matrix from a flat array in column-major order: the
            /// first column's elements first, then the second column's, and
            /// so on. This is the memory order of the matrix itself.
            #[inline]
            pub const fn from_cols_array(array: [#scalar; #len_literal]) -> Self {
                Self::from_cols(#(#from_array_cols),*)
            }

            /// The elements as a flat array in column-major order, the
            /// inverse of [`Self::from_cols_array`].
            #[inline]
            pub const fn to_cols_array(self) -> [#scalar; #len_literal] {
                [#(#to_array_elements),*]
            }

            /// Builds a matrix from an array of columns: `array[c][r]` is
            /// the element in column `c`, row `r`.
            #[inline]
            pub const fn from_cols_array_2d(array: [[#scalar; #dim_literal]; #dim_literal]) -> Self {
                Self::from_cols(#(#from_array_2d_cols),*)
            }

            /// The matrix as an array of columns, the inverse of
            /// [`Self::from_cols_array_2d`].
            #[inline]
            pub const fn to_cols_array_2d(self) -> [[#scalar; #dim_literal]; #dim_literal] {
                [#(#to_array_2d_cols),*]
            }

            /// The column at `index`.
            ///
            /// # Panics
            ///
            /// Panics if `index` is not below the number of columns.
            #[inline]
            pub const fn col(self, index: usize) -> #column {
                match index {
                    #(#indices => self.#axes,)*
                    _ => ::core::panic!("column index out of bounds"),
                }
            }

            /// The row at `index`, as a vector.
            ///
            /// # Panics
            ///
            /// Panics if `index` is not below the number of rows.
            #[inline]
            pub const fn row(self, index: usize) -> #column {
                match index {
                    #(#indices => #row_exprs,)*
                    _ => ::core::panic!("row index out of bounds"),
                }
            }

            /// The transpose: rows and columns swapped. Transposing twice
            /// gives back the original matrix.
            #[inline]
            pub const fn transpose(self) -> Self {
                Self::from_cols(#(#row_exprs),*)
            }

            /// The matrix-vector product `self * rhs`: `rhs` as a column
            /// vector, transformed by the matrix.
            #[inline]
            pub const fn mul_vec(self, rhs: #column) -> #column {
                #column::new(#(#mul_vec_components),*)
            }

            /// The matrix product `self * rhs`: the matrix that applies
            /// `rhs` first and then `self`. Not commutative.
            #[inline]
            pub const fn mul_mat(self, rhs: Self) -> Self {
                Self::from_cols(#(self.mul_vec(rhs.#axes)),*)
            }

            /// The determinant. Its sign tells whether the matrix keeps or
            /// flips orientation, its magnitude how it scales volume, and it
            /// is zero for a singular (non-invertible) matrix.
            #[inline]
            pub const fn determinant(self) -> #scalar {
                #determinant_body
            }

            /// The inverse matrix, so that `m * m.inverse().unwrap()` is the
            /// identity, or `None` when the matrix is singular: its
            /// determinant is exactly zero or not finite.
            ///
            /// Only an exactly singular matrix is rejected. A nearly
            /// singular one yields an inverse with huge elements and little
            /// precision, which is inherent to the matrix, not an error.
            #[inline]
            pub const fn inverse(self) -> ::core::option::Option<Self> {
                #inverse_body
            }
        }

        impl ::core::default::Default for #name {
            /// The identity matrix.
            #[inline]
            fn default() -> Self {
                Self::IDENTITY
            }
        }

        impl ::core::ops::Mul<#name> for #name {
            type Output = #name;

            /// The matrix product, see [`Self::mul_mat`].
            #[inline]
            fn mul(self, rhs: #name) -> #name {
                self.mul_mat(rhs)
            }
        }

        impl ::core::ops::Mul<#column> for #name {
            type Output = #column;

            /// The matrix-vector product, see [`Self::mul_vec`].
            #[inline]
            fn mul(self, rhs: #column) -> #column {
                self.mul_vec(rhs)
            }
        }

        impl ::core::ops::MulAssign<#name> for #name {
            #[inline]
            fn mul_assign(&mut self, rhs: #name) {
                *self = self.mul_mat(rhs);
            }
        }

        impl ::core::convert::From<[[#scalar; #dim_literal]; #dim_literal]> for #name {
            #[inline]
            fn from(array: [[#scalar; #dim_literal]; #dim_literal]) -> Self {
                Self::from_cols_array_2d(array)
            }
        }

        impl ::core::convert::From<#name> for [[#scalar; #dim_literal]; #dim_literal] {
            #[inline]
            fn from(matrix: #name) -> Self {
                matrix.to_cols_array_2d()
            }
        }

        impl ::core::ops::Index<usize> for #name {
            type Output = #column;

            /// The column at `index`.
            #[inline]
            fn index(&self, index: usize) -> &#column {
                match index {
                    #(#indices => &self.#axes,)*
                    _ => ::core::panic!(
                        "index out of bounds: the len is {} but the index is {}",
                        #dim_literal,
                        index
                    ),
                }
            }
        }

        impl ::core::ops::IndexMut<usize> for #name {
            /// The column at `index`, mutably.
            #[inline]
            fn index_mut(&mut self, index: usize) -> &mut #column {
                match index {
                    #(#indices => &mut self.#axes,)*
                    _ => ::core::panic!(
                        "index out of bounds: the len is {} but the index is {}",
                        #dim_literal,
                        index
                    ),
                }
            }
        }
    }
}

/// The body of `determinant` and of `inverse` for one dimension.
fn determinant_and_inverse(dim: usize, column: &TokenStream) -> (TokenStream, TokenStream) {
    match dim {
        2 => two_by_two(column),
        3 => three_by_three(column),
        4 => four_by_four(column),
        _ => unreachable!("only dimensions 2, 3 and 4 are generated"),
    }
}

/// The shared tail of every `inverse`: reject a singular matrix.
fn singular_check() -> TokenStream {
    quote! {
        if det == 0.0 || !det.is_finite() {
            return ::core::option::Option::None;
        }
        let inv_det = 1.0 / det;
    }
}

fn two_by_two(column: &TokenStream) -> (TokenStream, TokenStream) {
    let det = quote!(self.x_axis.x * self.y_axis.y - self.y_axis.x * self.x_axis.y);
    let check = singular_check();
    let inverse = quote! {
        let det = #det;
        #check
        ::core::option::Option::Some(Self::from_cols(
            #column::new(self.y_axis.y * inv_det, -self.x_axis.y * inv_det),
            #column::new(-self.y_axis.x * inv_det, self.x_axis.x * inv_det),
        ))
    };
    (det, inverse)
}

/// The rows of the inverse are the cross products of the other two columns
/// divided by the determinant, which is the triple product of the columns.
fn three_by_three(column: &TokenStream) -> (TokenStream, TokenStream) {
    let det = quote!(self.x_axis.dot(self.y_axis.cross(self.z_axis)));
    let check = singular_check();
    let inverse = quote! {
        let (a, b, c) = (self.x_axis, self.y_axis, self.z_axis);
        let r0 = b.cross(c);
        let r1 = c.cross(a);
        let r2 = a.cross(b);
        let det = a.dot(r0);
        #check
        ::core::option::Option::Some(Self::from_cols(
            #column::new(r0.x * inv_det, r1.x * inv_det, r2.x * inv_det),
            #column::new(r0.y * inv_det, r1.y * inv_det, r2.y * inv_det),
            #column::new(r0.z * inv_det, r1.z * inv_det, r2.z * inv_det),
        ))
    };
    (det, inverse)
}

/// The 2x2 minors of the top two rows (`s0`..`s5`) and of the bottom two rows
/// (`c0`..`c5`), both indexed by the column pair they use in the order
/// `01 02 03 12 13 23` (so `s5` and `c5` use columns 2 and 3).
const MINORS: &str = "
    let s0 = m00 * m11 - m10 * m01;
    let s1 = m00 * m12 - m10 * m02;
    let s2 = m00 * m13 - m10 * m03;
    let s3 = m01 * m12 - m11 * m02;
    let s4 = m01 * m13 - m11 * m03;
    let s5 = m02 * m13 - m12 * m03;
    let c5 = m22 * m33 - m32 * m23;
    let c4 = m21 * m33 - m31 * m23;
    let c3 = m21 * m32 - m31 * m22;
    let c2 = m20 * m33 - m30 * m23;
    let c1 = m20 * m32 - m30 * m22;
    let c0 = m20 * m31 - m30 * m21;
    let det = s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0;
";

/// The adjugate: `ADJUGATE[r][c]` is the element at row `r`, column `c` of
/// the inverse before dividing by the determinant.
const ADJUGATE: [[&str; 4]; 4] = [
    [
        "m11 * c5 - m12 * c4 + m13 * c3",
        "-m01 * c5 + m02 * c4 - m03 * c3",
        "m31 * s5 - m32 * s4 + m33 * s3",
        "-m21 * s5 + m22 * s4 - m23 * s3",
    ],
    [
        "-m10 * c5 + m12 * c2 - m13 * c1",
        "m00 * c5 - m02 * c2 + m03 * c1",
        "-m30 * s5 + m32 * s2 - m33 * s1",
        "m20 * s5 - m22 * s2 + m23 * s1",
    ],
    [
        "m10 * c4 - m11 * c2 + m13 * c0",
        "-m00 * c4 + m01 * c2 - m03 * c0",
        "m30 * s4 - m31 * s2 + m33 * s0",
        "-m20 * s4 + m21 * s2 - m23 * s0",
    ],
    [
        "-m10 * c3 + m11 * c1 - m12 * c0",
        "m00 * c3 - m01 * c1 + m02 * c0",
        "-m30 * s3 + m31 * s1 - m32 * s0",
        "m20 * s3 - m21 * s1 + m22 * s0",
    ],
];

/// Laplace expansion along 2x2 minors: 12 minors (24 products) instead of the
/// 16 3x3 cofactors a direct adjugate would need.
fn four_by_four(column: &TokenStream) -> (TokenStream, TokenStream) {
    let mut bindings = String::new();
    for (c, axis) in AXES.iter().enumerate() {
        for (r, component) in COMPONENTS.iter().enumerate() {
            bindings.push_str(&format!("let m{r}{c} = self.{axis}.{component};\n"));
        }
    }
    let prelude = parse_text(&format!("{bindings}{MINORS}"));

    let columns: Vec<TokenStream> = (0..4)
        .map(|c| {
            let elements: Vec<TokenStream> = (0..4)
                .map(|r| parse_text(&format!("({}) * inv_det", ADJUGATE[r][c])))
                .collect();
            quote!(#column::new(#(#elements),*))
        })
        .collect();

    let check = singular_check();
    let determinant = quote! {
        #prelude
        det
    };
    let inverse = quote! {
        #prelude
        #check
        ::core::option::Option::Some(Self::from_cols(#(#columns),*))
    };
    (determinant, inverse)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use syn::{ImplItem, Item, Type};

    use super::*;

    /// What a parsed expansion contains, by type name.
    struct Expansion {
        structs: Vec<String>,
        /// Per self-type, the (whitespace-free) trait paths it implements.
        traits: BTreeMap<String, BTreeSet<String>>,
        /// Per self-type, the names of its inherent associated functions.
        methods: BTreeMap<String, BTreeSet<String>>,
    }

    fn type_name(ty: &Type) -> String {
        quote!(#ty).to_string().replace(' ', "")
    }

    /// Expands `input` and parses the result back as a file, which also
    /// proves the output is syntactically valid Rust.
    fn expansion(input: TokenStream) -> Expansion {
        let tokens = matrices_impl(input);
        let file: syn::File =
            syn::parse2(tokens).expect("the expansion must be a valid sequence of items");
        let mut structs = Vec::new();
        let mut traits: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut methods: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for item in file.items {
            match item {
                Item::Struct(item) => structs.push(item.ident.to_string()),
                Item::Impl(item) => {
                    if let Some((path, _)) = &item.trait_ {
                        let path = quote!(#path).to_string().replace(' ', "");
                        traits
                            .entry(type_name(&item.self_ty))
                            .or_default()
                            .insert(path);
                    } else {
                        for member in &item.items {
                            if let ImplItem::Fn(function) = member {
                                methods
                                    .entry(type_name(&item.self_ty))
                                    .or_default()
                                    .insert(function.sig.ident.to_string());
                            }
                        }
                    }
                }
                other => panic!("unexpected item in the expansion: {}", quote!(#other)),
            }
        }
        Expansion {
            structs,
            traits,
            methods,
        }
    }

    #[test]
    fn generates_three_dimensions_per_family() {
        let out = expansion(quote! { Mat(Vec) => f32, DMat(DVec) => f64 });
        assert_eq!(
            out.structs,
            ["Mat2", "Mat3", "Mat4", "DMat2", "DMat3", "DMat4"]
        );
    }

    #[test]
    fn every_type_gets_the_full_api() {
        let out = expansion(quote! { Mat(Vec) => f32, DMat(DVec) => f64 });
        let api = [
            "from_cols",
            "from_cols_array",
            "to_cols_array",
            "from_cols_array_2d",
            "to_cols_array_2d",
            "col",
            "row",
            "transpose",
            "mul_vec",
            "mul_mat",
            "determinant",
            "inverse",
        ];
        for prefix in ["Mat", "DMat"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                for function in api {
                    assert!(
                        out.methods[&name].contains(function),
                        "{name} lacks {function}"
                    );
                }
            }
        }
    }

    #[test]
    fn trait_impls_use_the_matching_column_type() {
        let out = expansion(quote! { Mat(Vec) => f32, DMat(DVec) => f64 });
        for (matrix, column, scalar) in [
            ("Mat2", "Vec2", "f32"),
            ("Mat3", "Vec3", "f32"),
            ("Mat4", "Vec4", "f32"),
            ("DMat2", "DVec2", "f64"),
            ("DMat3", "DVec3", "f64"),
            ("DMat4", "DVec4", "f64"),
        ] {
            let dim: usize = matrix[matrix.len() - 1..].parse().unwrap();
            let traits = &out.traits[matrix];
            for expected in [
                "::core::default::Default".to_owned(),
                format!("::core::ops::Mul<{matrix}>"),
                format!("::core::ops::Mul<crate::vector::{column}>"),
                format!("::core::ops::MulAssign<{matrix}>"),
                "::core::ops::Index<usize>".to_owned(),
                "::core::ops::IndexMut<usize>".to_owned(),
                format!("::core::convert::From<[[{scalar};{dim}];{dim}]>"),
            ] {
                assert!(traits.contains(&expected), "{matrix} lacks {expected}");
            }
        }
    }

    #[test]
    fn the_array_conversion_goes_both_ways() {
        let out = expansion(quote! { Mat(Vec) => f32 });
        // `From<Mat3> for [[f32; 3]; 3]` is implemented on the array type.
        assert!(out.traits["[[f32;3];3]"].contains("::core::convert::From<Mat3>"));
    }

    #[test]
    fn a_family_can_be_listed_alone() {
        let out = expansion(quote! { DMat(DVec) => f64 });
        assert_eq!(out.structs, ["DMat2", "DMat3", "DMat4"]);
    }

    #[test]
    fn non_float_scalars_are_rejected() {
        for scalar in ["i32", "u8", "usize", "bool", "String"] {
            let scalar = format_ident!("{}", scalar);
            let error = expand(quote! { Mat(Vec) => #scalar }).unwrap_err();
            assert!(
                error.to_string().contains("unsupported scalar type"),
                "{error}"
            );
        }
    }

    #[test]
    fn duplicate_prefixes_are_rejected() {
        let error = expand(quote! { Mat(Vec) => f32, Mat(DVec) => f64 }).unwrap_err();
        assert!(error.to_string().contains("duplicate"), "{error}");
    }

    #[test]
    fn empty_input_is_rejected() {
        let error = expand(quote! {}).unwrap_err();
        assert!(error.to_string().contains("at least one"), "{error}");
    }

    #[test]
    fn malformed_input_reports_a_parse_error() {
        assert!(expand(quote! { Mat => f32 }).is_err());
        assert!(expand(quote! { Mat(Vec) f32 }).is_err());
        assert!(expand(quote! { Mat(Vec) => }).is_err());
        assert!(expand(quote! { Mat() => f32 }).is_err());
        assert!(expand(quote! { Mat(Vec, DVec) => f32 }).is_err());
        assert!(expand(quote! { (Vec) => f32 }).is_err());
        assert!(expand(quote! { Mat(Vec) => f32 DMat(DVec) => f64 }).is_err());
    }

    #[test]
    fn errors_become_compile_error_tokens() {
        let tokens = matrices_impl(quote! { Mat(Vec) => String }).to_string();
        assert!(tokens.contains("compile_error"), "{tokens}");
    }

    #[test]
    fn the_adjugate_table_only_uses_defined_names() {
        // Every `mRC`, `sN` and `cN` in the formulas must be bound by the
        // prelude; a typo would otherwise only surface as a compile error in
        // `math`.
        let defined: BTreeSet<String> = (0..4)
            .flat_map(|r| (0..4).map(move |c| format!("m{r}{c}")))
            .chain((0..6).flat_map(|i| [format!("s{i}"), format!("c{i}")]))
            .collect();
        for row in ADJUGATE {
            for expression in row {
                for token in expression.split(|ch: char| !ch.is_alphanumeric()) {
                    if !token.is_empty() {
                        assert!(defined.contains(token), "unknown name `{token}`");
                    }
                }
            }
        }
    }
}
