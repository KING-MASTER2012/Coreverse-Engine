//! Generation of the vector operations (Phase 1.8): `dot`, `cross`,
//! `length`, `normalize`, `project_onto`, `reflect`, `refract`,
//! `angle_between`, component-wise `min`/`max`/`clamp` and friends.
//!
//! [`operations`] returns one inherent `impl` block for a single vector type;
//! `vector.rs` splices it into that type's expansion. Which functions exist
//! depends on the scalar's kind and the dimension:
//!
//! | function                                          | float | signed int | unsigned int |
//! |---------------------------------------------------|:-----:|:----------:|:------------:|
//! | `dot`, `length_sq`, `distance_sq`                 |   x   |     x      |      x       |
//! | `min`, `max`, `clamp`                             |   x   |     x      |      x       |
//! | `cross` (3 components only)                       |   x   |     x      |              |
//! | `length`, `distance`, `normalize` & co.           |   x   |            |              |
//! | `project_onto`, `reject_from`, `reflect`          |   x   |            |              |
//! | `refract`, `angle_between`                        |   x   |            |              |
//!
//! Everything that is possible as a `const fn` is one (float arithmetic is
//! allowed in `const fn`, but `sqrt`, `acos`, ... are not, so the functions
//! built on them are plain `#[inline]` functions).
//!
//! Float functions reach `sqrt`/`atan2` through `crate::scalar::Scalar`, so
//! they use the `libm` backend under the `no_std` feature (`core` has no
//! `sqrt`). That `crate::` path is why `vectors!` can only be invoked inside
//! the `math` crate itself.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::vector::Kind;

/// The inherent `impl` block with every operation of one vector type.
pub(crate) fn operations(
    name: &Ident,
    scalar: &Ident,
    kind: Kind,
    dim: usize,
    fields: &[Ident],
) -> TokenStream {
    let shared = shared_operations(scalar, kind, fields);
    let cross = (dim == 3 && kind != Kind::Unsigned).then(|| cross_operation(fields));
    let float = (kind == Kind::Float).then(|| float_operations(scalar, kind));
    quote! {
        impl #name {
            #shared
            #cross
            #float
        }
    }
}

/// `a.min(b)` for floats (`NaN`-aware, like `f32::min`), a comparison for
/// integers (`Ord::min` is not `const`).
fn min_expr(kind: Kind, a: &TokenStream, b: &TokenStream) -> TokenStream {
    if kind == Kind::Float {
        quote!(#a.min(#b))
    } else {
        quote!(if #a < #b { #a } else { #b })
    }
}

/// See [`min_expr`].
fn max_expr(kind: Kind, a: &TokenStream, b: &TokenStream) -> TokenStream {
    if kind == Kind::Float {
        quote!(#a.max(#b))
    } else {
        quote!(if #a > #b { #a } else { #b })
    }
}

/// Operations every family has: `dot`, the squared lengths and distances,
/// and component-wise `min`/`max`/`clamp`.
fn shared_operations(scalar: &Ident, kind: Kind, fields: &[Ident]) -> TokenStream {
    let is_float = kind == Kind::Float;
    let overflow = if is_float {
        ""
    } else {
        "\n\nArithmetic follows Rust's rules for integers: it panics on \
         overflow in debug builds and wraps around in release builds."
    };

    let dot_doc =
        format!("The dot product: the sum of the products of the matching components.{overflow}");
    let length_sq_doc = if is_float {
        "The squared length, `self.dot(self)`.\n\nCheaper than [`Self::length`] \
         because it needs no square root, and enough for comparing lengths."
            .to_owned()
    } else {
        format!(
            "The squared length, `self.dot(self)`.\n\nThere is no `length` for \
             integer vectors since it is not an integer, so this is the \
             integer vector's measure of size.{overflow}"
        )
    };
    let unsigned_note = if kind == Kind::Unsigned {
        "\n\nThe difference is taken per component as the larger minus the \
         smaller, so unsigned components never underflow."
    } else {
        ""
    };
    let distance_sq_doc = format!(
        "The squared distance between the points `self` and `rhs`.\n\n\
         Cheaper than a distance because it needs no square root, and enough \
         for comparing distances.{unsigned_note}{overflow}"
    );

    let diffs = fields.iter().map(|f| {
        let d = format_ident!("d{}", f);
        let expr = if kind == Kind::Unsigned {
            quote!(if self.#f > rhs.#f { self.#f - rhs.#f } else { rhs.#f - self.#f })
        } else {
            quote!(self.#f - rhs.#f)
        };
        quote!(let #d = #expr;)
    });
    let squares = fields.iter().map(|f| {
        let d = format_ident!("d{}", f);
        quote!(#d * #d)
    });

    let min_fields = fields.iter().map(|f| {
        let e = min_expr(kind, &quote!(self.#f), &quote!(rhs.#f));
        quote!(#f: #e)
    });
    let max_fields = fields.iter().map(|f| {
        let e = max_expr(kind, &quote!(self.#f), &quote!(rhs.#f));
        quote!(#f: #e)
    });
    let clamp_fields = fields.iter().map(|f| {
        if is_float {
            quote!(#f: self.#f.clamp(lo.#f, hi.#f))
        } else {
            quote! {
                #f: {
                    ::core::assert!(
                        lo.#f <= hi.#f,
                        "clamp: every component of `lo` must be <= the matching one of `hi`"
                    );
                    if self.#f < lo.#f {
                        lo.#f
                    } else if self.#f > hi.#f {
                        hi.#f
                    } else {
                        self.#f
                    }
                }
            }
        }
    });

    let nan_note = if is_float {
        "\n\nA `NaN` component is ignored in favour of the other vector's \
         component, like `f32::min`."
    } else {
        ""
    };
    let min_doc = format!("The component-wise minimum of `self` and `rhs`.{nan_note}");
    let max_doc = format!("The component-wise maximum of `self` and `rhs`.{nan_note}");
    let clamp_panics = if is_float {
        "Panics if a component of `lo` is greater than the matching component \
         of `hi`, or if a bound is `NaN`."
    } else {
        "Panics if a component of `lo` is greater than the matching component \
         of `hi`."
    };
    let clamp_doc = format!(
        "Clamps every component of `self` to the range between the matching \
         components of `lo` and `hi` (both inclusive).\n\n# Panics\n\n{clamp_panics}"
    );

    quote! {
        #[doc = #dot_doc]
        #[inline]
        pub const fn dot(self, rhs: Self) -> #scalar {
            #(self.#fields * rhs.#fields)+*
        }

        #[doc = #length_sq_doc]
        #[inline]
        pub const fn length_sq(self) -> #scalar {
            self.dot(self)
        }

        #[doc = #distance_sq_doc]
        #[inline]
        pub const fn distance_sq(self, rhs: Self) -> #scalar {
            #(#diffs)*
            #(#squares)+*
        }

        #[doc = #min_doc]
        #[inline]
        pub const fn min(self, rhs: Self) -> Self {
            Self { #(#min_fields),* }
        }

        #[doc = #max_doc]
        #[inline]
        pub const fn max(self, rhs: Self) -> Self {
            Self { #(#max_fields),* }
        }

        #[doc = #clamp_doc]
        #[inline]
        pub const fn clamp(self, lo: Self, hi: Self) -> Self {
            Self { #(#clamp_fields),* }
        }
    }
}

/// `cross` for the 3-component signed and float types.
fn cross_operation(fields: &[Ident]) -> TokenStream {
    let (x, y, z) = (&fields[0], &fields[1], &fields[2]);
    quote! {
        /// The cross product: a vector perpendicular to both `self` and
        /// `rhs`, following the right-hand rule (`X.cross(Y) == Z`). It is
        /// zero when the two are parallel, and `a.cross(b) == -b.cross(a)`.
        #[inline]
        pub const fn cross(self, rhs: Self) -> Self {
            Self::new(
                self.#y * rhs.#z - self.#z * rhs.#y,
                self.#z * rhs.#x - self.#x * rhs.#z,
                self.#x * rhs.#y - self.#y * rhs.#x,
            )
        }
    }
}

/// Everything that needs `sqrt`/`atan2` or a division: float types only.
fn float_operations(scalar: &Ident, kind: Kind) -> TokenStream {
    let (zero, one) = (kind.zero(), kind.one());
    let scalar_trait = quote!(<#scalar as crate::scalar::Scalar>);

    quote! {
        /// The length (Euclidean norm) of the vector.
        ///
        /// Use [`Self::length_sq`] when only comparing lengths: it avoids the
        /// square root.
        #[inline]
        pub fn length(self) -> #scalar {
            #scalar_trait::sqrt(self.length_sq())
        }

        /// The distance between the points `self` and `rhs`.
        ///
        /// Use [`Self::distance_sq`] when only comparing distances.
        #[inline]
        pub fn distance(self, rhs: Self) -> #scalar {
            #scalar_trait::sqrt(self.distance_sq(rhs))
        }

        /// The vector scaled to length `1`, keeping its direction.
        ///
        /// A zero-length vector has no direction: the result is then `NaN` in
        /// every component (`0 / 0`). Use [`Self::try_normalize`] or
        /// [`Self::normalize_or_zero`] when that can happen.
        #[inline]
        pub fn normalize(self) -> Self {
            self / self.length()
        }

        /// Like [`Self::normalize`], but `None` when the vector has no
        /// usable direction: zero length, a non-finite component, or a
        /// squared length that overflows or underflows (components beyond
        /// roughly `1e19` / below `1e-19` for `f32`).
        #[inline]
        pub fn try_normalize(self) -> ::core::option::Option<Self> {
            let length = self.length();
            if length > #zero && length.is_finite() {
                ::core::option::Option::Some(self / length)
            } else {
                ::core::option::Option::None
            }
        }

        /// Like [`Self::normalize`], but the zero vector for input that
        /// [`Self::try_normalize`] rejects.
        #[inline]
        pub fn normalize_or_zero(self) -> Self {
            self.try_normalize().unwrap_or(Self::ZERO)
        }

        /// The projection of `self` onto `onto`: the part of `self` that
        /// points along `onto`. `onto` need not be normalized.
        ///
        /// The result is `NaN` if `onto` is the zero vector.
        #[inline]
        pub fn project_onto(self, onto: Self) -> Self {
            onto * (self.dot(onto) / onto.dot(onto))
        }

        /// The rejection of `self` from `from`: the part of `self`
        /// perpendicular to `from`, so that
        /// `v == v.project_onto(w) + v.reject_from(w)`. `from` need not be
        /// normalized.
        ///
        /// The result is `NaN` if `from` is the zero vector.
        #[inline]
        pub fn reject_from(self, from: Self) -> Self {
            self - self.project_onto(from)
        }

        /// Reflects the incident vector `self` off a surface with the given
        /// `normal`, like GLSL's `reflect(I, N)`: `self - 2 * dot(normal,
        /// self) * normal`.
        ///
        /// `normal` must be normalized, otherwise the result is scaled
        /// incorrectly.
        #[inline]
        pub fn reflect(self, normal: Self) -> Self {
            self - normal * (2.0 * normal.dot(self))
        }

        /// Refracts the incident vector `self` through a surface with the
        /// given `normal`, like GLSL's `refract(I, N, eta)`. `eta` is the
        /// ratio of the refractive indices, incident side over transmitted
        /// side.
        ///
        /// Returns `None` on total internal reflection, when no refracted ray
        /// exists. Both `self` and `normal` must be normalized, otherwise the
        /// result is not meaningful.
        #[inline]
        pub fn refract(self, normal: Self, eta: #scalar) -> ::core::option::Option<Self> {
            let n_dot_i = normal.dot(self);
            let k = #one - eta * eta * (#one - n_dot_i * n_dot_i);
            if k < #zero {
                ::core::option::Option::None
            } else {
                ::core::option::Option::Some(
                    self * eta - normal * (eta * n_dot_i + #scalar_trait::sqrt(k)),
                )
            }
        }

        /// The unsigned angle between `self` and `rhs`, in `0..=PI` radians.
        ///
        /// Computed as `2 * atan2(|b|a - |a|b|, ||b|a + |a|b|)` rather than
        /// with `acos` of the normalized dot product, which loses almost all
        /// precision for angles near `0` and `PI`. Neither vector needs to be
        /// normalized.
        ///
        /// The angle is undefined if either vector has zero length: the
        /// result is `NaN` then.
        #[inline]
        pub fn angle_between(self, rhs: Self) -> crate::angle::Radians<#scalar> {
            let (length_a, length_b) = (self.length(), rhs.length());
            if length_a == #zero || length_b == #zero {
                return crate::angle::Radians::new(#scalar_trait::NAN);
            }
            let a = self * length_b;
            let b = rhs * length_a;
            crate::angle::Radians::new(
                2.0 * #scalar_trait::atan2((a - b).length(), (a + b).length()),
            )
        }
    }
}
