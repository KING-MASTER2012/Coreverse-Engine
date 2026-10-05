//! Implementation of the `vectors!` macro.
//!
//! One invocation lists the vector families to build, each as
//! `Prefix => scalar`:
//!
//! ```text
//! vectors! {
//!     Vec  => f32,
//!     DVec => f64,
//!     IVec => i32,
//!     UVec => u32,
//! }
//! ```
//!
//! and expands to `Prefix2`, `Prefix3` and `Prefix4` for every entry
//! (`Vec2`..`UVec4`: 12 concrete types here), each with its fields,
//! constructors and component-wise operator impls. Which impls are generated
//! depends on the scalar's kind (float, signed integer or unsigned integer),
//! so `UVec3` has no `Neg` while `Vec3` and `IVec3` do.
//!
//! The logic lives in [`vectors_impl`] over `proc_macro2` types, not in the
//! `#[proc_macro]` function itself, so the unit tests at the bottom of this
//! file can run it (a real proc-macro entry point cannot be called outside a
//! macro expansion).

use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Error, Ident, Result, Token};

/// Component names in order; a vector of dimension `n` uses the first `n`.
pub(crate) const COMPONENTS: [&str; 4] = ["x", "y", "z", "w"];

/// The dimensions generated for every family.
const DIMENSIONS: [usize; 3] = [2, 3, 4];

const FLOAT_SCALARS: [&str; 2] = ["f32", "f64"];
const SIGNED_SCALARS: [&str; 6] = ["i8", "i16", "i32", "i64", "i128", "isize"];
const UNSIGNED_SCALARS: [&str; 6] = ["u8", "u16", "u32", "u64", "u128", "usize"];

/// What kind of number a family's components are, which decides both the
/// literals used (`0.0` vs `0`) and which impls exist.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Float,
    Signed,
    Unsigned,
}

impl Kind {
    fn of(scalar: &str) -> Option<Self> {
        if FLOAT_SCALARS.contains(&scalar) {
            Some(Self::Float)
        } else if SIGNED_SCALARS.contains(&scalar) {
            Some(Self::Signed)
        } else if UNSIGNED_SCALARS.contains(&scalar) {
            Some(Self::Unsigned)
        } else {
            None
        }
    }

    /// Unsigned vectors cannot be negated.
    fn has_neg(self) -> bool {
        self != Self::Unsigned
    }

    /// Integer vectors are totally ordered by their fields' equality, so they
    /// can also be `Eq` and `Hash`; float vectors cannot (`NaN`).
    fn has_eq_hash(self) -> bool {
        self != Self::Float
    }

    pub(crate) fn zero(self) -> TokenStream {
        match self {
            Self::Float => quote!(0.0),
            Self::Signed | Self::Unsigned => quote!(0),
        }
    }

    pub(crate) fn one(self) -> TokenStream {
        match self {
            Self::Float => quote!(1.0),
            Self::Signed | Self::Unsigned => quote!(1),
        }
    }
}

/// One `Prefix => scalar` entry of the macro input.
pub(crate) struct Family {
    pub(crate) prefix: Ident,
    pub(crate) scalar: Ident,
    pub(crate) kind: Kind,
}

impl syn::parse::Parse for Family {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let prefix: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let scalar: Ident = input.parse()?;
        let Some(kind) = Kind::of(&scalar.to_string()) else {
            return Err(Error::new_spanned(
                &scalar,
                format!(
                    "unsupported scalar type `{scalar}`; expected one of: {}, {}, {}",
                    FLOAT_SCALARS.join(", "),
                    SIGNED_SCALARS.join(", "),
                    UNSIGNED_SCALARS.join(", "),
                ),
            ));
        };
        Ok(Self {
            prefix,
            scalar,
            kind,
        })
    }
}

/// Expands the `vectors!` input into the generated items, or into a
/// `compile_error!` describing what is wrong with the input.
pub(crate) fn vectors_impl(input: TokenStream) -> TokenStream {
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
            "expected at least one `Prefix => scalar` entry, e.g. `Vec => f32`",
        ));
    }

    let mut seen: Vec<String> = Vec::new();
    for family in &families {
        let prefix = family.prefix.to_string();
        if seen.contains(&prefix) {
            return Err(Error::new_spanned(
                &family.prefix,
                format!("duplicate vector family prefix `{prefix}`"),
            ));
        }
        seen.push(prefix);
    }

    // Some generated functions name the other families (`as_dvec3`, ...).
    let families: Vec<Family> = families.into_iter().collect();
    let vectors = families
        .iter()
        .flat_map(|family| DIMENSIONS.map(|dim| vector(family, dim, &families)));
    Ok(quote! { #(#vectors)* })
}

/// Everything for one type: `Prefix<dim>` of `family.scalar` components.
/// `families` is every family of the invocation, `family` included.
fn vector(family: &Family, dim: usize, families: &[Family]) -> TokenStream {
    let Family {
        prefix,
        scalar,
        kind,
    } = family;
    let name = format_ident!("{}{}", prefix, dim);
    let fields: Vec<Ident> = COMPONENTS[..dim]
        .iter()
        .map(|component| format_ident!("{}", component))
        .collect();
    let field_docs: Vec<String> = fields
        .iter()
        .map(|field| format!("The `{field}` component."))
        .collect();
    let indices: Vec<Literal> = (0..dim).map(Literal::usize_unsuffixed).collect();
    let dim_literal = Literal::usize_unsuffixed(dim);
    let (zero, one) = (kind.zero(), kind.one());

    let derives = if kind.has_eq_hash() {
        quote!(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)
    } else {
        quote!(Clone, Copy, Debug, Default, PartialEq)
    };

    let order = COMPONENTS[..dim].join(", ");
    let arithmetic_note = if *kind == Kind::Float {
        String::new()
    } else {
        "\n\nArithmetic on the components follows Rust's rules for integers: \
         it panics on overflow in debug builds and wraps around in release \
         builds, and dividing by zero always panics."
            .to_owned()
    };
    let neg_note = if kind.has_neg() {
        ""
    } else {
        "\n\nThere is no unary `-`, since the components are unsigned."
    };
    let struct_doc = format!(
        "A {dim}-dimensional vector with `{scalar}` components.\n\n\
         The components are stored in `{order}` order with no padding \
         (`#[repr(C)]`), so the type is exactly {dim} `{scalar}`s in memory. \
         `+`, `-`, `*` and `/` (and their `*Assign` forms) work component by \
         component; `*` and `/` take either a scalar or a vector of the same \
         type on the right (a vector is multiplied or divided component by \
         component), and `*` also accepts a scalar on the left.\
         {arithmetic_note}{neg_note}"
    );

    let axis_consts = (0..dim).map(|axis| {
        let const_name = format_ident!("{}", COMPONENTS[axis].to_uppercase());
        let args = (0..dim).map(|component| if component == axis { &one } else { &zero });
        let doc = format!("The unit vector along the `{}` axis.", COMPONENTS[axis]);
        quote! {
            #[doc = #doc]
            pub const #const_name: Self = Self::new(#(#args),*);
        }
    });

    let operations = crate::vector_ops::operations(&name, scalar, *kind, dim, &fields);
    let extensions = crate::vector_ext::extensions(&crate::vector_ext::Spec {
        name: &name,
        prefix,
        scalar,
        kind: *kind,
        dim,
        fields: &fields,
        families,
    });

    let neg_impl = kind.has_neg().then(|| {
        quote! {
            impl ::core::ops::Neg for #name {
                type Output = Self;

                #[inline]
                fn neg(self) -> Self {
                    Self { #(#fields: -self.#fields),* }
                }
            }
        }
    });

    let component_wise = [
        (
            "Add",
            "add",
            quote!(+),
            "AddAssign",
            "add_assign",
            quote!(+=),
        ),
        (
            "Sub",
            "sub",
            quote!(-),
            "SubAssign",
            "sub_assign",
            quote!(-=),
        ),
    ]
    .map(
        |(trait_name, method, op, assign_trait, assign_method, assign_op)| {
            let (trait_name, method) =
                (format_ident!("{}", trait_name), format_ident!("{}", method));
            let (assign_trait, assign_method) = (
                format_ident!("{}", assign_trait),
                format_ident!("{}", assign_method),
            );
            quote! {
                impl ::core::ops::#trait_name for #name {
                    type Output = Self;

                    #[inline]
                    fn #method(self, rhs: Self) -> Self {
                        Self { #(#fields: self.#fields #op rhs.#fields),* }
                    }
                }

                impl ::core::ops::#assign_trait for #name {
                    #[inline]
                    fn #assign_method(&mut self, rhs: Self) {
                        #(self.#fields #assign_op rhs.#fields;)*
                    }
                }
            }
        },
    );

    let by_scalar = [
        (
            "Mul",
            "mul",
            quote!(*),
            "MulAssign",
            "mul_assign",
            quote!(*=),
        ),
        (
            "Div",
            "div",
            quote!(/),
            "DivAssign",
            "div_assign",
            quote!(/=),
        ),
    ]
    .map(
        |(trait_name, method, op, assign_trait, assign_method, assign_op)| {
            let (trait_name, method) =
                (format_ident!("{}", trait_name), format_ident!("{}", method));
            let (assign_trait, assign_method) = (
                format_ident!("{}", assign_trait),
                format_ident!("{}", assign_method),
            );
            quote! {
                impl ::core::ops::#trait_name<#scalar> for #name {
                    type Output = Self;

                    #[inline]
                    fn #method(self, rhs: #scalar) -> Self {
                        Self { #(#fields: self.#fields #op rhs),* }
                    }
                }

                impl ::core::ops::#assign_trait<#scalar> for #name {
                    #[inline]
                    fn #assign_method(&mut self, rhs: #scalar) {
                        #(self.#fields #assign_op rhs;)*
                    }
                }
            }
        },
    );

    let index_doc = format!(
        "Component access by position: `0` is `x`, `1` is `y`, and so on up to `{}`.\n\n\
         # Panics\n\nPanics if the index is `{dim}` or more, like indexing an array.",
        dim - 1
    );

    quote! {
        #[doc = #struct_doc]
        #[derive(#derives)]
        #[repr(C)]
        pub struct #name {
            #(
                #[doc = #field_docs]
                pub #fields: #scalar,
            )*
        }

        impl #name {
            /// The vector with every component `0`.
            pub const ZERO: Self = Self::splat(#zero);

            /// The vector with every component `1`.
            pub const ONE: Self = Self::splat(#one);

            #(#axis_consts)*

            /// Creates a vector from its components.
            #[inline]
            pub const fn new(#(#fields: #scalar),*) -> Self {
                Self { #(#fields),* }
            }

            /// Creates a vector with every component set to `value`.
            #[inline]
            pub const fn splat(value: #scalar) -> Self {
                Self { #(#fields: value),* }
            }

            /// Creates a vector from an array of its components, in order.
            #[inline]
            pub const fn from_array(array: [#scalar; #dim_literal]) -> Self {
                Self { #(#fields: array[#indices]),* }
            }

            /// The components as an array, in order.
            #[inline]
            pub const fn to_array(self) -> [#scalar; #dim_literal] {
                [#(self.#fields),*]
            }
        }

        #operations

        #extensions

        impl ::core::convert::From<[#scalar; #dim_literal]> for #name {
            #[inline]
            fn from(array: [#scalar; #dim_literal]) -> Self {
                Self::from_array(array)
            }
        }

        impl ::core::convert::From<#name> for [#scalar; #dim_literal] {
            #[inline]
            fn from(vector: #name) -> Self {
                vector.to_array()
            }
        }

        #neg_impl

        #(#component_wise)*

        #(#by_scalar)*

        impl ::core::ops::Mul<#name> for #scalar {
            type Output = #name;

            #[inline]
            fn mul(self, rhs: #name) -> #name {
                #name { #(#fields: self * rhs.#fields),* }
            }
        }

        #[doc = #index_doc]
        impl ::core::ops::Index<usize> for #name {
            type Output = #scalar;

            #[inline]
            fn index(&self, index: usize) -> &#scalar {
                match index {
                    #(#indices => &self.#fields,)*
                    _ => ::core::panic!(
                        "index out of bounds: the len is {} but the index is {}",
                        #dim_literal,
                        index
                    ),
                }
            }
        }

        #[doc = #index_doc]
        impl ::core::ops::IndexMut<usize> for #name {
            #[inline]
            fn index_mut(&mut self, index: usize) -> &mut #scalar {
                match index {
                    #(#indices => &mut self.#fields,)*
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use syn::{ImplItem, Item, Type};

    /// What the expansion defined: struct names, and per self-type the set of
    /// implemented traits (`Neg`, `Add`, `Mul<f32>`...) as strings.
    struct Expansion {
        structs: Vec<String>,
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
        let tokens = vectors_impl(input);
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
                        let path = path.trim_start_matches("::core::");
                        let path = path
                            .trim_start_matches("ops::")
                            .trim_start_matches("convert::");
                        traits
                            .entry(type_name(&item.self_ty))
                            .or_default()
                            .insert(path.to_owned());
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

    fn error_message(input: TokenStream) -> String {
        expand(input)
            .expect_err("input should be rejected")
            .to_string()
    }

    #[test]
    fn one_family_yields_three_dimensions() {
        let out = expansion(quote! { Vec => f32 });
        assert_eq!(out.structs, ["Vec2", "Vec3", "Vec4"]);
    }

    #[test]
    fn four_families_yield_twelve_types() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 });
        assert_eq!(out.structs.len(), 12);
        for name in [
            "Vec2", "Vec3", "Vec4", "DVec2", "DVec3", "DVec4", "IVec2", "IVec3", "IVec4", "UVec2",
            "UVec3", "UVec4",
        ] {
            assert!(out.structs.iter().any(|s| s == name), "missing {name}");
        }
    }

    #[test]
    fn trailing_comma_is_allowed() {
        let out = expansion(quote! { Vec => f32, });
        assert_eq!(out.structs.len(), 3);
    }

    #[test]
    fn every_type_gets_the_component_wise_operators() {
        let out = expansion(quote! { Vec => f32, IVec => i32, UVec => u32 });
        for (prefix, scalar) in [("Vec", "f32"), ("IVec", "i32"), ("UVec", "u32")] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                let traits = &out.traits[&name];
                for expected in [
                    "Add".to_owned(),
                    "Sub".to_owned(),
                    "AddAssign".to_owned(),
                    "SubAssign".to_owned(),
                    format!("Mul<{scalar}>"),
                    format!("Div<{scalar}>"),
                    format!("MulAssign<{scalar}>"),
                    format!("DivAssign<{scalar}>"),
                    "Index<usize>".to_owned(),
                    "IndexMut<usize>".to_owned(),
                    format!("From<[{scalar};{dim}]>"),
                ] {
                    assert!(traits.contains(&expected), "{name} lacks {expected}");
                }
                assert!(out.traits[scalar].contains(&format!("Mul<{name}>")));
            }
        }
    }

    const SHARED_OPERATIONS: [&str; 6] = ["dot", "length_sq", "distance_sq", "min", "max", "clamp"];
    const FLOAT_OPERATIONS: [&str; 10] = [
        "length",
        "distance",
        "normalize",
        "try_normalize",
        "normalize_or_zero",
        "project_onto",
        "reject_from",
        "reflect",
        "refract",
        "angle_between",
    ];

    #[test]
    fn every_type_gets_the_shared_operations() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 });
        for prefix in ["Vec", "DVec", "IVec", "UVec"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                for function in SHARED_OPERATIONS {
                    assert!(
                        out.methods[&name].contains(function),
                        "{name} lacks {function}"
                    );
                }
            }
        }
    }

    #[test]
    fn float_only_operations_exist_for_floats_only() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 });
        for prefix in ["Vec", "DVec"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                for function in FLOAT_OPERATIONS {
                    assert!(
                        out.methods[&name].contains(function),
                        "{name} lacks {function}"
                    );
                }
            }
        }
        for prefix in ["IVec", "UVec"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                for function in FLOAT_OPERATIONS {
                    assert!(
                        !out.methods[&name].contains(function),
                        "{name} must not have {function}"
                    );
                }
            }
        }
    }

    #[test]
    fn cross_exists_only_for_three_components_and_never_unsigned() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 });
        for name in ["Vec3", "DVec3", "IVec3"] {
            assert!(out.methods[name].contains("cross"), "{name} needs cross");
        }
        for name in [
            "Vec2", "Vec4", "DVec2", "DVec4", "IVec2", "IVec4", "UVec2", "UVec3", "UVec4",
        ] {
            assert!(
                !out.methods[name].contains("cross"),
                "{name} must not have cross"
            );
        }
    }

    const ALL_FAMILIES: fn() -> TokenStream =
        || quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 };

    /// Asserts that exactly the types in `expected` have `function`.
    fn assert_has(out: &Expansion, function: &str, expected: &[&str]) {
        for prefix in ["Vec", "DVec", "IVec", "UVec"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                assert_eq!(
                    out.methods[&name].contains(function),
                    expected.contains(&name.as_str()),
                    "{name}::{function}"
                );
            }
        }
    }

    #[test]
    fn every_type_gets_the_element_reductions() {
        let out = expansion(ALL_FAMILIES());
        let everything: Vec<String> = ["Vec", "DVec", "IVec", "UVec"]
            .iter()
            .flat_map(|prefix| (2..=4).map(move |dim| format!("{prefix}{dim}")))
            .collect();
        let everything: Vec<&str> = everything.iter().map(String::as_str).collect();
        for function in [
            "min_element",
            "max_element",
            "element_sum",
            "element_product",
        ] {
            assert_has(&out, function, &everything);
        }
    }

    #[test]
    fn resizing_functions_exist_where_the_size_fits() {
        let out = expansion(ALL_FAMILIES());
        let sized = |dims: &[usize]| -> Vec<String> {
            ["Vec", "DVec", "IVec", "UVec"]
                .iter()
                .flat_map(|prefix| dims.iter().map(move |dim| format!("{prefix}{dim}")))
                .collect()
        };
        for (function, dims) in [
            ("extend", &[2, 3][..]),
            ("truncate", &[3, 4][..]),
            ("xy", &[3, 4][..]),
            ("xyz", &[4][..]),
        ] {
            let names = sized(dims);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            assert_has(&out, function, &names);
        }
    }

    #[test]
    fn rounding_lerp_and_approx_functions_are_float_only() {
        let out = expansion(ALL_FAMILIES());
        let floats = ["Vec2", "Vec3", "Vec4", "DVec2", "DVec3", "DVec4"];
        for function in [
            "floor",
            "ceil",
            "round",
            "trunc",
            "fract",
            "lerp",
            "is_finite",
            "is_nan",
            "approx_eq",
            "approx_eq_eps",
            "approx_eq_rel",
        ] {
            assert_has(&out, function, &floats);
        }
    }

    #[test]
    fn abs_and_signum_exist_for_floats_and_signed_integers_only() {
        let out = expansion(ALL_FAMILIES());
        let signed = [
            "Vec2", "Vec3", "Vec4", "DVec2", "DVec3", "DVec4", "IVec2", "IVec3", "IVec4",
        ];
        assert_has(&out, "abs", &signed);
        assert_has(&out, "signum", &signed);
    }

    #[test]
    fn euclidean_functions_exist_for_integers_only() {
        let out = expansion(ALL_FAMILIES());
        let integers = ["IVec2", "IVec3", "IVec4", "UVec2", "UVec3", "UVec4"];
        assert_has(&out, "rem_euclid", &integers);
        assert_has(&out, "div_euclid", &integers);
        assert_has(&out, "abs_diff", &integers);
    }

    #[test]
    fn abs_diff_of_a_signed_family_needs_the_unsigned_family_of_the_same_width() {
        // The unsigned partner is part of the invocation: generated.
        let out = expansion(quote! { IVec => i32, UVec => u32 });
        assert!(out.methods["IVec3"].contains("abs_diff"));
        assert!(
            vectors_impl(quote! { IVec => i32, UVec => u32 })
                .to_string()
                .contains("-> UVec3")
        );
        // No partner, or one of another width: not generated.
        let alone = expansion(quote! { IVec => i32 });
        assert!(!alone.methods["IVec3"].contains("abs_diff"));
        let wrong_width = expansion(quote! { IVec => i32, WVec => u64 });
        assert!(!wrong_width.methods["IVec3"].contains("abs_diff"));
        // Unsigned families always have it (it returns the same type).
        let unsigned_only = expansion(quote! { UVec => u32 });
        assert!(unsigned_only.methods["UVec2"].contains("abs_diff"));
    }

    #[test]
    fn conversions_target_every_other_family_of_the_invocation() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32 });
        for dim in 2..=4 {
            let vec = &out.methods[&format!("Vec{dim}")];
            assert!(vec.contains(&format!("as_dvec{dim}")));
            assert!(vec.contains(&format!("as_ivec{dim}")));
            assert!(!vec.contains(&format!("as_vec{dim}")), "no self-conversion");
            let int = &out.methods[&format!("IVec{dim}")];
            assert!(int.contains(&format!("as_vec{dim}")));
            assert!(int.contains(&format!("as_dvec{dim}")));
            assert!(
                !int.contains(&format!("as_uvec{dim}")),
                "UVec is not in the invocation"
            );
        }
        // A lone family converts to nothing.
        let alone = expansion(quote! { Vec => f32 });
        assert!(
            !alone.methods["Vec3"]
                .iter()
                .any(|name| name.starts_with("as_"))
        );
    }

    #[test]
    fn conversions_between_families_with_the_same_scalar_use_no_cast() {
        // `x as f32` on an `f32` is a clippy warning in the generated code.
        let tokens = vectors_impl(quote! { A => f32, B => f32 }).to_string();
        assert!(!tokens.contains("as f32"), "{tokens}");
        let tokens = vectors_impl(quote! { A => f32, B => f64 }).to_string();
        assert!(tokens.contains("as f64"), "{tokens}");
    }

    #[test]
    fn two_and_three_dimensional_helpers_exist_for_the_matching_float_sizes_only() {
        let out = expansion(ALL_FAMILIES());
        for function in ["perp", "perp_dot", "rotate", "from_angle"] {
            assert_has(&out, function, &["Vec2", "DVec2"]);
        }
        for function in ["any_orthogonal", "orthonormal_basis"] {
            assert_has(&out, function, &["Vec3", "DVec3"]);
        }
    }

    #[test]
    fn every_type_gets_display_and_the_component_wise_vector_operators() {
        let out = expansion(ALL_FAMILIES());
        for prefix in ["Vec", "DVec", "IVec", "UVec"] {
            for dim in 2..=4 {
                let name = format!("{prefix}{dim}");
                for expected in ["fmt::Display", "Mul", "Div", "MulAssign", "DivAssign"] {
                    assert!(
                        out.traits[&name].contains(expected),
                        "{name} lacks {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn integer_expansion_does_not_reference_the_scalar_trait() {
        // Only float operations need `crate::scalar::Scalar`/`Radians`.
        let ints = vectors_impl(quote! { IVec => i32, UVec => u32 }).to_string();
        assert!(
            !ints.contains("Scalar") && !ints.contains("Radians"),
            "{ints}"
        );
        let floats = vectors_impl(quote! { Vec => f32 }).to_string();
        assert!(floats.contains("Scalar") && floats.contains("Radians"));
    }

    #[test]
    fn neg_exists_for_floats_and_signed_but_not_unsigned() {
        let out = expansion(quote! { Vec => f32, DVec => f64, IVec => i32, UVec => u32 });
        for name in ["Vec3", "DVec3", "IVec3", "Vec2", "IVec4"] {
            assert!(out.traits[name].contains("Neg"), "{name} should have Neg");
        }
        for name in ["UVec2", "UVec3", "UVec4"] {
            assert!(
                !out.traits[name].contains("Neg"),
                "{name} must not have Neg"
            );
        }
    }

    #[test]
    fn eq_and_hash_are_derived_only_for_integer_families() {
        let tokens = |input: TokenStream| vectors_impl(input).to_string();
        let float = tokens(quote! { Vec => f32 });
        let int = tokens(quote! { IVec => i32 });
        assert!(!float.contains("Hash") && !float.contains("Eq ,"));
        assert!(int.contains("Hash") && int.contains("Eq"));
    }

    #[test]
    fn every_integer_and_float_scalar_is_accepted() {
        for scalar in ["f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize"] {
            let scalar = format_ident!("{}", scalar);
            assert_eq!(expansion(quote! { V => #scalar }).structs.len(), 3);
        }
        for scalar in ["u8", "u16", "u32", "u64", "u128", "usize"] {
            let scalar = format_ident!("{}", scalar);
            assert_eq!(expansion(quote! { V => #scalar }).structs.len(), 3);
        }
    }

    #[test]
    fn unsupported_scalar_is_rejected() {
        let message = error_message(quote! { Vec => String });
        assert!(
            message.contains("unsupported scalar type `String`"),
            "{message}"
        );
        assert!(
            message.contains("f32") && message.contains("usize"),
            "{message}"
        );
        assert!(error_message(quote! { Vec => f16 }).contains("unsupported"));
        assert!(error_message(quote! { Vec => bool }).contains("unsupported"));
    }

    #[test]
    fn duplicate_prefix_is_rejected() {
        let message = error_message(quote! { Vec => f32, Vec => f64 });
        assert!(
            message.contains("duplicate vector family prefix `Vec`"),
            "{message}"
        );
    }

    #[test]
    fn empty_input_is_rejected() {
        let message = error_message(quote! {});
        assert!(message.contains("at least one"), "{message}");
    }

    #[test]
    fn malformed_input_reports_a_parse_error() {
        assert!(expand(quote! { Vec f32 }).is_err());
        assert!(expand(quote! { Vec => }).is_err());
        assert!(expand(quote! { => f32 }).is_err());
        assert!(expand(quote! { Vec => f32 IVec => i32 }).is_err());
        assert!(expand(quote! { "Vec" => f32 }).is_err());
    }

    #[test]
    fn errors_become_compile_error_tokens() {
        let tokens = vectors_impl(quote! { Vec => String }).to_string();
        assert!(tokens.contains("compile_error"), "{tokens}");
    }
}
