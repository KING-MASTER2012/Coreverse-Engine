//! Generation of the vector additions of Phase 2.2: element reductions,
//! rounding, `lerp`, resizing (`extend`/`truncate`/`xy`/`xyz`), conversions
//! between the families (`as_dvec3`, ...), approximate comparison, the 2D and
//! 3D helpers, `Display`, and the component-wise `*` and `/` between two
//! vectors.
//!
//! [`extensions`] returns the inherent `impl` block plus the trait impls for a
//! single vector type; `vector.rs` splices it into that type's expansion,
//! next to the Phase 1.8 operations from `vector_ops.rs`. Which functions
//! exist depends on the scalar's kind and the dimension:
//!
//! | function                                              | float | signed int | unsigned int |
//! |-------------------------------------------------------|:-----:|:----------:|:------------:|
//! | `min_element`, `max_element`, `element_sum`, `element_product` | x | x      |      x       |
//! | `extend`, `truncate`, `xy`, `xyz` (where the size fits) |  x    |     x      |      x       |
//! | `as_<family><dim>` (one per other family)             |   x   |     x      |      x       |
//! | `Display`, `Mul<Self>`, `Div<Self>` (+ `*Assign`)     |   x   |     x      |      x       |
//! | `abs`                                                 |   x   |     x      |              |
//! | `signum`                                              |   x   |     x      |              |
//! | `rem_euclid`, `div_euclid`, `abs_diff`                |       |     x      |      x       |
//! | `floor`, `ceil`, `round`, `trunc`, `fract`, `lerp`    |   x   |            |              |
//! | `is_finite`, `is_nan`, `approx_eq*`                   |   x   |            |              |
//! | `perp`, `perp_dot`, `rotate`, `from_angle` (2 components) | x |            |              |
//! | `any_orthogonal`, `orthonormal_basis` (3 components)  |   x   |            |              |
//!
//! `abs_diff` of a signed family returns the unsigned family of the same
//! width (`IVec3::abs_diff` gives a `UVec3`) and is only generated when that
//! family is part of the same `vectors!` invocation.
//!
//! Like the Phase 1.8 operations, the float functions that need `floor`,
//! `sin` and friends go through `crate::scalar::Scalar` (so they follow the
//! `std`/`libm` backend), and everything that does not is a `const fn`.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::vector::{Family, Kind};

/// Everything the generator needs to know about one vector type.
pub(crate) struct Spec<'a> {
    /// The type's name, e.g. `Vec3`.
    pub name: &'a Ident,
    /// The family prefix, e.g. `Vec`.
    pub prefix: &'a Ident,
    /// The component type, e.g. `f32`.
    pub scalar: &'a Ident,
    /// Float, signed or unsigned.
    pub kind: Kind,
    /// Number of components (2, 3 or 4).
    pub dim: usize,
    /// The component names, `x`, `y`, ... (as many as `dim`).
    pub fields: &'a [Ident],
    /// Every family of the invocation (this one included).
    pub families: &'a [Family],
}

/// The inherent `impl` block with the Phase 2.2 functions of one vector type,
/// followed by its trait impls.
pub(crate) fn extensions(spec: &Spec<'_>) -> TokenStream {
    let name = spec.name;
    let reductions = reductions(spec);
    let resizing = resizing(spec);
    let conversions = conversions(spec);
    let float = (spec.kind == Kind::Float).then(|| float_functions(spec));
    let signed = (spec.kind == Kind::Signed).then(|| signed_functions(spec));
    let integer = (spec.kind != Kind::Float).then(|| integer_functions(spec));
    let two = (spec.kind == Kind::Float && spec.dim == 2).then(|| vec2_functions(spec));
    let three = (spec.kind == Kind::Float && spec.dim == 3).then(|| vec3_functions(spec));
    let traits = trait_impls(spec);
    quote! {
        impl #name {
            #reductions
            #resizing
            #conversions
            #float
            #signed
            #integer
            #two
            #three
        }

        #traits
    }
}

/// `items` joined with the binary operator `op` (`a * b * c`). Written out
/// by hand because `quote!`'s repetition separator must be a single token,
/// and `*`, `&&` and `||` read ambiguously next to the repetition marker.
fn join(items: impl IntoIterator<Item = TokenStream>, op: &TokenStream) -> TokenStream {
    let mut items = items.into_iter();
    let first = items.next().expect("a vector has at least two components");
    items.fold(first, |acc, item| quote!(#acc #op #item))
}

/// `min_element`, `max_element`, `element_sum` and `element_product`.
fn reductions(spec: &Spec<'_>) -> TokenStream {
    let Spec {
        scalar,
        kind,
        fields,
        ..
    } = spec;
    let first = &fields[0];
    let rest = &fields[1..];
    let (min_body, max_body) = if *kind == Kind::Float {
        (
            quote!(#(let result = result.min(self.#rest);)*),
            quote!(#(let result = result.max(self.#rest);)*),
        )
    } else {
        // `Ord::min` is not `const`, so compare by hand.
        (
            quote!(#(let result = if self.#rest < result { self.#rest } else { result };)*),
            quote!(#(let result = if self.#rest > result { self.#rest } else { result };)*),
        )
    };
    let nan_note = if *kind == Kind::Float {
        "\n\nA `NaN` component is ignored in favour of the others, like \
         `f32::min`."
    } else {
        ""
    };
    let overflow = if *kind == Kind::Float {
        ""
    } else {
        "\n\nArithmetic follows Rust's rules for integers: it panics on \
         overflow in debug builds and wraps around in release builds."
    };
    let min_doc = format!("The smallest component.{nan_note}");
    let max_doc = format!("The largest component.{nan_note}");
    let sum_doc = format!("The sum of the components.{overflow}");
    let product_doc = format!("The product of the components.{overflow}");
    let sum = join(fields.iter().map(|field| quote!(self.#field)), &quote!(+));
    let product = join(fields.iter().map(|field| quote!(self.#field)), &quote!(*));
    quote! {
        #[doc = #min_doc]
        #[inline]
        pub const fn min_element(self) -> #scalar {
            let result = self.#first;
            #min_body
            result
        }

        #[doc = #max_doc]
        #[inline]
        pub const fn max_element(self) -> #scalar {
            let result = self.#first;
            #max_body
            result
        }

        #[doc = #sum_doc]
        #[inline]
        pub const fn element_sum(self) -> #scalar {
            #sum
        }

        #[doc = #product_doc]
        #[inline]
        pub const fn element_product(self) -> #scalar {
            #product
        }
    }
}

/// `extend` (to the next size), `truncate` (to the previous one) and the
/// `xy`/`xyz` swizzles.
fn resizing(spec: &Spec<'_>) -> TokenStream {
    let Spec {
        prefix,
        scalar,
        dim,
        fields,
        ..
    } = spec;
    let sized = |size: usize| format_ident!("{}{}", prefix, size);

    let extend = (*dim < 4).then(|| {
        let bigger = sized(dim + 1);
        let doc = format!(
            "This vector with one more component, `value`, appended: a \
             [`{bigger}`]. Going from a 3D position to a homogeneous 4D one \
             is `position.extend(1.0)`."
        );
        quote! {
            #[doc = #doc]
            #[inline]
            pub const fn extend(self, value: #scalar) -> #bigger {
                #bigger::new(#(self.#fields,)* value)
            }
        }
    });

    let truncate = (*dim > 2).then(|| {
        let smaller = sized(dim - 1);
        let kept = &fields[..dim - 1];
        let doc = format!("This vector without its last component: a [`{smaller}`].");
        quote! {
            #[doc = #doc]
            #[inline]
            pub const fn truncate(self) -> #smaller {
                #smaller::new(#(self.#kept),*)
            }
        }
    });

    let xy = (*dim > 2).then(|| {
        let smaller = sized(2);
        let (x, y) = (&fields[0], &fields[1]);
        let doc = format!("The `x` and `y` components as a [`{smaller}`].");
        quote! {
            #[doc = #doc]
            #[inline]
            pub const fn xy(self) -> #smaller {
                #smaller::new(self.#x, self.#y)
            }
        }
    });

    let xyz = (*dim == 4).then(|| {
        let smaller = sized(3);
        let (x, y, z) = (&fields[0], &fields[1], &fields[2]);
        let doc = format!("The `x`, `y` and `z` components as a [`{smaller}`].");
        quote! {
            #[doc = #doc]
            #[inline]
            pub const fn xyz(self) -> #smaller {
                #smaller::new(self.#x, self.#y, self.#z)
            }
        }
    });

    quote! { #extend #truncate #xy #xyz }
}

/// `as_<family><dim>`: one conversion per *other* family of the invocation.
fn conversions(spec: &Spec<'_>) -> TokenStream {
    let Spec {
        prefix,
        scalar,
        dim,
        fields,
        families,
        ..
    } = spec;
    let conversions = families
        .iter()
        .filter(|family| family.prefix != **prefix)
        .map(|family| {
            let target = format_ident!("{}{}", family.prefix, dim);
            let method = format_ident!("as_{}{}", family.prefix.to_string().to_lowercase(), dim);
            let target_scalar = &family.scalar;
            let args = fields.iter().map(|field| {
                if family.scalar == **scalar {
                    quote!(self.#field)
                } else {
                    quote!(self.#field as #target_scalar)
                }
            });
            let doc = format!(
                "Converts to a [`{target}`], casting every component with `as`.\n\n\
                 That is Rust's `as` conversion between `{scalar}` and \
                 `{target_scalar}`: float to integer saturates at the \
                 integer's range (and `NaN` becomes `0`), integer to integer \
                 wraps around (keeps the low bits), and integer to float \
                 rounds to the nearest representable value."
            );
            quote! {
                #[doc = #doc]
                #[inline]
                pub const fn #method(self) -> #target {
                    #target::new(#(#args),*)
                }
            }
        });
    quote! { #(#conversions)* }
}

/// Float-only functions that apply to any size.
fn float_functions(spec: &Spec<'_>) -> TokenStream {
    let Spec { scalar, fields, .. } = spec;
    let scalar_trait = quote!(<#scalar as crate::scalar::Scalar>);
    let all = quote!(&&);
    let any = quote!(||);
    let is_finite = join(fields.iter().map(|f| quote!(self.#f.is_finite())), &all);
    let is_nan = join(fields.iter().map(|f| quote!(self.#f.is_nan())), &any);
    let approx = join(
        fields
            .iter()
            .map(|f| quote!(crate::approx::approx_eq(self.#f, rhs.#f))),
        &all,
    );
    let approx_eps = join(
        fields
            .iter()
            .map(|f| quote!(crate::approx::approx_eq_eps(self.#f, rhs.#f, epsilon))),
        &all,
    );
    let approx_rel = join(
        fields
            .iter()
            .map(|f| quote!(crate::approx::approx_eq_rel(self.#f, rhs.#f, rel_epsilon))),
        &all,
    );

    // The rounding functions all have the same shape.
    let rounding = [
        (
            "floor",
            "Rounds every component down to the nearest integer (towards \
             negative infinity).",
        ),
        (
            "ceil",
            "Rounds every component up to the nearest integer (towards \
             positive infinity).",
        ),
        (
            "round",
            "Rounds every component to the nearest integer, ties away from \
             zero (`2.5` becomes `3.0`, `-2.5` becomes `-3.0`).",
        ),
        (
            "trunc",
            "Rounds every component towards zero, dropping the fractional \
             part (`-1.7` becomes `-1.0`).",
        ),
        (
            "fract",
            "The fractional part of every component, `self - self.trunc()`.\n\n\
             It keeps the sign of the component (`-3.75` gives `-0.75`), so \
             `v == v.trunc() + v.fract()`. This is **not** the GLSL `fract`; \
             for that use `v - v.floor()`.",
        ),
    ]
    .map(|(function, doc)| {
        let function = format_ident!("{}", function);
        quote! {
            #[doc = #doc]
            #[inline]
            pub fn #function(self) -> Self {
                Self { #(#fields: #scalar_trait::#function(self.#fields)),* }
            }
        }
    });

    quote! {
        /// The absolute value of every component.
        #[inline]
        pub const fn abs(self) -> Self {
            Self { #(#fields: self.#fields.abs()),* }
        }

        /// The IEEE-754 sign of every component: `1.0` for positive
        /// (including `+0.0`), `-1.0` for negative (including `-0.0`), `NaN`
        /// for `NaN`. Never zero, like `f32::signum`.
        #[inline]
        pub const fn signum(self) -> Self {
            Self { #(#fields: self.#fields.signum()),* }
        }

        #(#rounding)*

        /// Linear interpolation from `self` (at `t = 0`) to `rhs` (at
        /// `t = 1`), the same `a + (b - a) * t` as [`crate::interp::lerp`] on
        /// every component.
        ///
        /// `t` is not clamped, so values outside `0..=1` extrapolate.
        #[inline]
        pub const fn lerp(self, rhs: Self, t: #scalar) -> Self {
            Self { #(#fields: self.#fields + (rhs.#fields - self.#fields) * t),* }
        }

        /// Whether every component is finite (neither infinite nor `NaN`).
        #[inline]
        pub const fn is_finite(self) -> bool {
            #is_finite
        }

        /// Whether any component is `NaN`.
        #[inline]
        pub const fn is_nan(self) -> bool {
            #is_nan
        }

        /// Whether every component of `self` is within the type's machine
        /// epsilon of the matching one of `rhs`, see
        /// [`crate::approx::approx_eq`].
        ///
        /// Use [`Self::approx_eq_eps`] or [`Self::approx_eq_rel`] after more
        /// than a couple of operations' worth of rounding error. A `NaN`
        /// component is never approximately equal to anything.
        #[inline]
        pub fn approx_eq(self, rhs: Self) -> bool {
            #approx
        }

        /// Whether every component of `self` is within `epsilon` of the
        /// matching one of `rhs` (absolute difference), see
        /// [`crate::approx::approx_eq_eps`].
        #[inline]
        pub fn approx_eq_eps(self, rhs: Self, epsilon: #scalar) -> bool {
            #approx_eps
        }

        /// Whether every component of `self` is within `rel_epsilon` of the
        /// matching one of `rhs`, scaled by the larger magnitude of the two,
        /// see [`crate::approx::approx_eq_rel`].
        #[inline]
        pub fn approx_eq_rel(self, rhs: Self, rel_epsilon: #scalar) -> bool {
            #approx_rel
        }
    }
}

/// `abs` and `signum` for the signed integer types.
fn signed_functions(spec: &Spec<'_>) -> TokenStream {
    let fields = spec.fields;
    quote! {
        /// The absolute value of every component.
        ///
        /// Like `i32::abs`, it overflows for the most negative value of the
        /// scalar type: that panics in debug builds and stays negative in
        /// release builds.
        #[inline]
        pub const fn abs(self) -> Self {
            Self { #(#fields: self.#fields.abs()),* }
        }

        /// The sign of every component: `-1` for negative, `0` for zero and
        /// `1` for positive (the integer `signum` has a zero, unlike the
        /// float one).
        #[inline]
        pub const fn signum(self) -> Self {
            Self { #(#fields: self.#fields.signum()),* }
        }
    }
}

/// `rem_euclid`, `div_euclid` and `abs_diff` for both integer kinds.
fn integer_functions(spec: &Spec<'_>) -> TokenStream {
    let Spec {
        name,
        scalar,
        kind,
        dim,
        fields,
        families,
        ..
    } = spec;

    // Signed vectors return the unsigned family of the same width, if the
    // invocation has one; unsigned vectors return themselves.
    let target = if *kind == Kind::Unsigned {
        Some((quote!(#name), "the same type".to_owned()))
    } else {
        let unsigned = scalar.to_string().replacen('i', "u", 1);
        families
            .iter()
            .find(|family| family.scalar == unsigned)
            .map(|family| {
                let target = format_ident!("{}{}", family.prefix, dim);
                (quote!(#target), format!("a [`{target}`]"))
            })
    };
    let abs_diff = target.map(|(target, returns)| {
        let doc = format!(
            "The absolute difference of every component pair, as {returns}.\n\n\
             Unlike a subtraction followed by `abs`, it cannot overflow, since \
             the result has the unsigned type."
        );
        quote! {
            #[doc = #doc]
            #[inline]
            pub const fn abs_diff(self, rhs: Self) -> #target {
                #target { #(#fields: self.#fields.abs_diff(rhs.#fields)),* }
            }
        }
    });

    let signed_note = if *kind == Kind::Signed {
        ", or when dividing the most negative value by `-1`"
    } else {
        ""
    };
    let rem_doc = format!(
        "The Euclidean remainder of every component: always in `0..rhs.abs()`, \
         also for a negative `self` (`-1` modulo `3` is `2`), unlike `%`.\n\n\
         # Panics\n\n\
         Panics if a component of `rhs` is zero{signed_note}."
    );
    let div_doc = format!(
        "The Euclidean quotient of every component, rounded so that \
         `self == self.div_euclid(rhs) * rhs + self.rem_euclid(rhs)` (for a \
         negative `self` this rounds down where `/` rounds towards \
         zero).\n\n\
         # Panics\n\n\
         Panics if a component of `rhs` is zero{signed_note}."
    );

    quote! {
        #abs_diff

        #[doc = #rem_doc]
        #[inline]
        pub const fn rem_euclid(self, rhs: Self) -> Self {
            Self { #(#fields: self.#fields.rem_euclid(rhs.#fields)),* }
        }

        #[doc = #div_doc]
        #[inline]
        pub const fn div_euclid(self, rhs: Self) -> Self {
            Self { #(#fields: self.#fields.div_euclid(rhs.#fields)),* }
        }
    }
}

/// `perp`, `perp_dot`, `rotate` and `from_angle` for the 2-component floats.
fn vec2_functions(spec: &Spec<'_>) -> TokenStream {
    let scalar = spec.scalar;
    let scalar_trait = quote!(<#scalar as crate::scalar::Scalar>);
    quote! {
        /// This vector rotated a quarter turn counter-clockwise: `(-y, x)`.
        ///
        /// The result is perpendicular to `self` and has the same length.
        #[inline]
        pub const fn perp(self) -> Self {
            Self::new(-self.y, self.x)
        }

        /// The perpendicular dot product (the 2D cross product):
        /// `self.perp().dot(rhs)`, which is `x * rhs.y - y * rhs.x`.
        ///
        /// It is positive when `rhs` is counter-clockwise from `self`,
        /// negative when clockwise, and zero when they are parallel; its
        /// magnitude is the area of the parallelogram the two span.
        #[inline]
        pub const fn perp_dot(self, rhs: Self) -> #scalar {
            self.x * rhs.y - self.y * rhs.x
        }

        /// The unit vector pointing at `angle` counter-clockwise from the
        /// `x` axis: `(cos(angle), sin(angle))`.
        ///
        /// `angle` is a `Radians` or a `Degrees`.
        #[inline]
        pub fn from_angle(angle: impl Into<crate::angle::Radians<#scalar>>) -> Self {
            let radians = angle.into().value();
            Self::new(#scalar_trait::cos(radians), #scalar_trait::sin(radians))
        }

        /// This vector rotated counter-clockwise by `angle` (clockwise for a
        /// negative one) around the origin.
        ///
        /// `angle` is a `Radians` or a `Degrees`. The length is preserved,
        /// up to rounding.
        #[inline]
        pub fn rotate(self, angle: impl Into<crate::angle::Radians<#scalar>>) -> Self {
            let radians = angle.into().value();
            let (sin, cos) = (#scalar_trait::sin(radians), #scalar_trait::cos(radians));
            Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
        }
    }
}

/// `any_orthogonal` and `orthonormal_basis` for the 3-component floats.
fn vec3_functions(spec: &Spec<'_>) -> TokenStream {
    let scalar = spec.scalar;
    let scalar_trait = quote!(<#scalar as crate::scalar::Scalar>);
    quote! {
        /// Some vector perpendicular to `self`: its dot product with `self`
        /// is zero.
        ///
        /// The choice is arbitrary but deterministic, and the result is not
        /// normalized (its length is at most that of `self`). It is the zero
        /// vector only if `self` is. Use [`Self::orthonormal_basis`] for a
        /// whole frame.
        #[inline]
        pub const fn any_orthogonal(self) -> Self {
            if self.x.abs() > self.z.abs() {
                Self::new(-self.y, self.x, 0.0)
            } else {
                Self::new(0.0, -self.z, self.y)
            }
        }

        /// Two unit vectors `(tangent, bitangent)` that, together with `self`,
        /// form a right-handed orthonormal basis:
        /// `tangent.cross(bitangent) == self`.
        ///
        /// `self` must be normalized, otherwise the result is not
        /// orthonormal. Branchless and continuous except at a single point
        /// (it handles every direction, including `-Z`, without a special
        /// case), after Duff et al., "Building an Orthonormal Basis,
        /// Revisited" (2017).
        #[inline]
        pub fn orthonormal_basis(self) -> (Self, Self) {
            let sign = #scalar_trait::copysign(1.0, self.z);
            let a = -1.0 / (sign + self.z);
            let b = self.x * self.y * a;
            (
                Self::new(1.0 + sign * self.x * self.x * a, sign * b, -sign * self.x),
                Self::new(b, sign + self.y * self.y * a, -self.y),
            )
        }
    }
}

/// `Display`, and the component-wise `*` and `/` between two vectors.
fn trait_impls(spec: &Spec<'_>) -> TokenStream {
    let Spec {
        name, kind, fields, ..
    } = spec;

    let writes = fields.iter().enumerate().map(|(index, field)| {
        let separator = (index > 0).then(|| quote!(formatter.write_str(", ")?;));
        quote! {
            #separator
            ::core::fmt::Display::fmt(&self.#field, formatter)?;
        }
    });
    let display_doc = "Formats as `[x, y, ...]`, each component with the formatter's \
                       own options, so `{:.2}` or `{:>5}` apply to every component.";

    let zero_note = if *kind == Kind::Float {
        ""
    } else {
        " Integer division by a zero component panics."
    };
    let operators = [
        (
            "Mul",
            "mul",
            quote!(*),
            "MulAssign",
            "mul_assign",
            quote!(*=),
            "Multiplies",
        ),
        (
            "Div",
            "div",
            quote!(/),
            "DivAssign",
            "div_assign",
            quote!(/=),
            "Divides",
        ),
    ]
    .map(
        |(trait_name, method, op, assign_trait, assign_method, assign_op, verb)| {
            let (trait_name, method) =
                (format_ident!("{}", trait_name), format_ident!("{}", method));
            let (assign_trait, assign_method) = (
                format_ident!("{}", assign_trait),
                format_ident!("{}", assign_method),
            );
            let doc = format!(
                "{verb} component by component (the Hadamard product/quotient, as \
                 `vec * vec` is in GLSL).{zero_note}"
            );
            quote! {
                #[doc = #doc]
                impl ::core::ops::#trait_name for #name {
                    type Output = Self;

                    #[inline]
                    fn #method(self, rhs: Self) -> Self {
                        Self { #(#fields: self.#fields #op rhs.#fields),* }
                    }
                }

                #[doc = #doc]
                impl ::core::ops::#assign_trait for #name {
                    #[inline]
                    fn #assign_method(&mut self, rhs: Self) {
                        #(self.#fields #assign_op rhs.#fields;)*
                    }
                }
            }
        },
    );

    quote! {
        #[doc = #display_doc]
        impl ::core::fmt::Display for #name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str("[")?;
                #(#writes)*
                formatter.write_str("]")
            }
        }

        #(#operators)*
    }
}
