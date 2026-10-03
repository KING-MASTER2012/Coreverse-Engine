//! Vector types: [`Vec2`]/[`Vec3`]/[`Vec4`] (`f32`), [`DVec2`]/[`DVec3`]/
//! [`DVec4`] (`f64`), [`IVec2`]/[`IVec3`]/[`IVec4`] (`i32`) and
//! [`UVec2`]/[`UVec3`]/[`UVec4`] (`u32`).
//!
//! All 12 are generated from the single [`math_macros::vectors!`]
//! invocation below, so they are guaranteed to have the same shape:
//!
//! | prefix | components | extra derives |
//! |--------|------------|---------------|
//! | `Vec`  | `f32`      |               |
//! | `DVec` | `f64`      |               |
//! | `IVec` | `i32`      | `Eq`, `Hash`  |
//! | `UVec` | `u32`      | `Eq`, `Hash`  |
//!
//! Every type is a plain `#[repr(C)]` struct with public `x`, `y` (and `z`,
//! `w`) fields, so `Vec3 { x: 1.0, y: 2.0, z: 3.0 }`, [`Vec3::new`] and
//! [`Vec3::from`]`([1.0, 2.0, 3.0])` are interchangeable. The `const fn`
//! constructors (`new`, `splat`, `from_array`) and the `ZERO`/`ONE`/`X`/`Y`/
//! `Z`/`W` constants can be used in `const` items.
//!
//! # Operators
//!
//! `+` and `-` work between two vectors of the same type. `*` and `/` take a
//! scalar on the right (`v * 2.0`, `v / 2.0`), and `*` also takes it on the
//! left (`2.0 * v`). All four have `*Assign` forms, and `-v` is available
//! for every type except the unsigned `UVec`s. Everything is component-wise,
//! and `v[i]` reads or writes a component by position.
//!
//! ```
//! use math::{IVec2, Vec3};
//!
//! let v = Vec3::new(1.0, 2.0, 3.0) * 2.0 + Vec3::X;
//! assert_eq!(v, Vec3::new(3.0, 4.0, 6.0));
//! assert_eq!(-v[1], -4.0);
//!
//! let mut p = IVec2::new(10, 20);
//! p += IVec2::splat(1);
//! assert_eq!(p.to_array(), [11, 21]);
//! ```
//!
//! Integer vectors follow Rust's integer rules, so overflow panics in debug
//! builds and wraps in release builds, and integer division by zero always
//! panics. Float vectors never panic: `Vec2::ONE / 0.0` is infinite.
//!
//! The vector operations (dot product, length, ...) are added in Phase 1.8.

math_macros::vectors! {
    Vec => f32,
    DVec => f64,
    IVec => i32,
    UVec => u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Debug;
    use core::hash::Hash;
    use core::marker::PhantomData;
    use core::ops::{
        Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign,
    };

    /// Checks every operator on `V` (an `N`-component vector of `S`) against
    /// plain per-component arithmetic on arrays. The values are chosen so
    /// that every result is exact for floats and integers alike, and so that
    /// subtraction never underflows an unsigned scalar.
    fn operators<V, S, const N: usize>()
    where
        V: Copy
            + Debug
            + PartialEq
            + From<[S; N]>
            + Into<[S; N]>
            + Add<Output = V>
            + Sub<Output = V>
            + Mul<S, Output = V>
            + Div<S, Output = V>
            + AddAssign
            + SubAssign
            + MulAssign<S>
            + DivAssign<S>
            + Index<usize, Output = S>
            + IndexMut<usize>,
        S: Copy + Debug + PartialEq + From<u8> + Mul<V, Output = V>,
    {
        let from_u8 = |f: fn(u8) -> u8| -> [S; N] {
            core::array::from_fn(|i| S::from(f(u8::try_from(i).unwrap() + 1)))
        };
        let a = from_u8(|k| 2 * k); // 2, 4, 6, 8
        let b = from_u8(|k| k); // 1, 2, 3, 4
        let sum = from_u8(|k| 3 * k);
        let diff = from_u8(|k| k);
        let tripled = from_u8(|k| 6 * k);
        let halved = from_u8(|k| k);
        let (three, two) = (S::from(3), S::from(2));
        let (v, w) = (V::from(a), V::from(b));

        assert_eq!(v + w, V::from(sum));
        assert_eq!(v - w, V::from(diff));
        assert_eq!(v * three, V::from(tripled));
        assert_eq!(three * v, V::from(tripled));
        assert_eq!(v / two, V::from(halved));

        let mut t = v;
        t += w;
        assert_eq!(t, V::from(sum));
        t -= w;
        assert_eq!(t, v);
        t *= three;
        assert_eq!(t, V::from(tripled));
        t /= three;
        assert_eq!(t, v);

        for (i, &expected) in a.iter().enumerate() {
            assert_eq!(v[i], expected);
        }
        let mut t = v;
        for i in 0..N {
            t[i] = S::from(9);
        }
        assert_eq!(Into::<[S; N]>::into(t), [S::from(9); N]);
        assert_eq!(Into::<[S; N]>::into(v), a);
    }

    /// Checks unary `-` on `V` against negating each array element.
    fn negation<V, S, const N: usize>()
    where
        V: Copy + Debug + PartialEq + From<[S; N]> + Neg<Output = V>,
        S: Copy + Debug + PartialEq + From<u8> + Neg<Output = S>,
    {
        let a: [S; N] = core::array::from_fn(|i| S::from(u8::try_from(i).unwrap() + 1));
        assert_eq!(-V::from(a), V::from(a.map(|c| -c)));
    }

    fn assert_eq_and_hash<T: Eq + Hash>() {}

    /// Whether `$t: Neg`, decided at compile time. An inherent associated
    /// const only applies when its bound holds; otherwise the trait's
    /// default `false` is used instead.
    macro_rules! implements_neg {
        ($t:ty) => {{
            #[allow(
                dead_code,
                reason = "only one of the two constants is selected per type"
            )]
            trait Fallback {
                const IMPLEMENTS_NEG: bool = false;
            }
            struct Probe<T>(PhantomData<T>);
            impl<T> Fallback for Probe<T> {}
            impl<T: Neg> Probe<T> {
                #[allow(
                    dead_code,
                    reason = "only one of the two constants is selected per type"
                )]
                const IMPLEMENTS_NEG: bool = true;
            }
            <Probe<$t>>::IMPLEMENTS_NEG
        }};
    }

    /// Tests that are the same for every family, instantiated once per
    /// family in its own module so the helper `s` does not clash.
    macro_rules! family_tests {
        ($module:ident, $scalar:ty, $v2:ident, $v3:ident, $v4:ident) => {
            mod $module {
                use super::*;

                fn s(value: u8) -> $scalar {
                    <$scalar>::from(value)
                }

                #[test]
                fn new_sets_every_field() {
                    let v = $v2::new(s(1), s(2));
                    assert_eq!((v.x, v.y), (s(1), s(2)));
                    let v = $v3::new(s(1), s(2), s(3));
                    assert_eq!((v.x, v.y, v.z), (s(1), s(2), s(3)));
                    let v = $v4::new(s(1), s(2), s(3), s(4));
                    assert_eq!((v.x, v.y, v.z, v.w), (s(1), s(2), s(3), s(4)));
                }

                #[test]
                fn struct_literals_and_new_agree() {
                    assert_eq!($v2 { x: s(5), y: s(6) }, $v2::new(s(5), s(6)));
                }

                #[test]
                fn splat_zero_and_one() {
                    assert_eq!($v2::splat(s(7)), $v2::new(s(7), s(7)));
                    assert_eq!($v3::splat(s(7)), $v3::new(s(7), s(7), s(7)));
                    assert_eq!($v4::splat(s(7)), $v4::new(s(7), s(7), s(7), s(7)));
                    assert_eq!($v2::ZERO, $v2::new(s(0), s(0)));
                    assert_eq!($v3::ZERO, $v3::new(s(0), s(0), s(0)));
                    assert_eq!($v4::ZERO, $v4::new(s(0), s(0), s(0), s(0)));
                    assert_eq!($v2::ONE, $v2::new(s(1), s(1)));
                    assert_eq!($v3::ONE, $v3::new(s(1), s(1), s(1)));
                    assert_eq!($v4::ONE, $v4::new(s(1), s(1), s(1), s(1)));
                }

                #[test]
                fn default_is_zero() {
                    assert_eq!($v2::default(), $v2::ZERO);
                    assert_eq!($v3::default(), $v3::ZERO);
                    assert_eq!($v4::default(), $v4::ZERO);
                }

                #[test]
                fn unit_axes() {
                    assert_eq!($v2::X, $v2::new(s(1), s(0)));
                    assert_eq!($v2::Y, $v2::new(s(0), s(1)));
                    assert_eq!($v3::X, $v3::new(s(1), s(0), s(0)));
                    assert_eq!($v3::Y, $v3::new(s(0), s(1), s(0)));
                    assert_eq!($v3::Z, $v3::new(s(0), s(0), s(1)));
                    assert_eq!($v4::X, $v4::new(s(1), s(0), s(0), s(0)));
                    assert_eq!($v4::Y, $v4::new(s(0), s(1), s(0), s(0)));
                    assert_eq!($v4::Z, $v4::new(s(0), s(0), s(1), s(0)));
                    assert_eq!($v4::W, $v4::new(s(0), s(0), s(0), s(1)));
                }

                #[test]
                fn array_conversions_round_trip() {
                    let v = $v3::new(s(1), s(2), s(3));
                    assert_eq!(v.to_array(), [s(1), s(2), s(3)]);
                    assert_eq!($v3::from_array([s(1), s(2), s(3)]), v);
                    assert_eq!($v3::from([s(1), s(2), s(3)]), v);
                    assert_eq!(<[$scalar; 3]>::from(v), [s(1), s(2), s(3)]);
                    assert_eq!($v2::from_array([s(8), s(9)]).to_array(), [s(8), s(9)]);
                    assert_eq!(
                        $v4::from_array([s(1), s(2), s(3), s(4)]).to_array(),
                        [s(1), s(2), s(3), s(4)]
                    );
                }

                #[test]
                fn memory_layout_is_tightly_packed() {
                    let size = core::mem::size_of::<$scalar>();
                    let align = core::mem::align_of::<$scalar>();
                    assert_eq!(core::mem::size_of::<$v2>(), 2 * size);
                    assert_eq!(core::mem::size_of::<$v3>(), 3 * size);
                    assert_eq!(core::mem::size_of::<$v4>(), 4 * size);
                    assert_eq!(core::mem::align_of::<$v2>(), align);
                    assert_eq!(core::mem::align_of::<$v3>(), align);
                    assert_eq!(core::mem::align_of::<$v4>(), align);
                }

                #[test]
                fn operators_match_per_component_arithmetic() {
                    operators::<$v2, $scalar, 2>();
                    operators::<$v3, $scalar, 3>();
                    operators::<$v4, $scalar, 4>();
                }

                #[test]
                #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
                fn v2_index_out_of_bounds_panics() {
                    let v = $v2::ONE;
                    let _ = v[2];
                }

                #[test]
                #[should_panic(expected = "index out of bounds: the len is 3 but the index is 3")]
                fn v3_index_out_of_bounds_panics() {
                    let v = $v3::ONE;
                    let _ = v[3];
                }

                #[test]
                #[should_panic(expected = "index out of bounds: the len is 4 but the index is 4")]
                fn v4_index_out_of_bounds_panics() {
                    let v = $v4::ONE;
                    let _ = v[4];
                }

                #[test]
                #[should_panic(expected = "index out of bounds: the len is 3 but the index is 7")]
                fn index_mut_out_of_bounds_panics() {
                    let mut v = $v3::ONE;
                    v[7] = s(1);
                }
            }
        };
    }

    family_tests!(vec_f32, f32, Vec2, Vec3, Vec4);
    family_tests!(dvec_f64, f64, DVec2, DVec3, DVec4);
    family_tests!(ivec_i32, i32, IVec2, IVec3, IVec4);
    family_tests!(uvec_u32, u32, UVec2, UVec3, UVec4);

    #[test]
    fn negation_works_for_float_and_signed_vectors() {
        negation::<Vec2, f32, 2>();
        negation::<Vec3, f32, 3>();
        negation::<Vec4, f32, 4>();
        negation::<DVec2, f64, 2>();
        negation::<DVec3, f64, 3>();
        negation::<DVec4, f64, 4>();
        negation::<IVec2, i32, 2>();
        negation::<IVec3, i32, 3>();
        negation::<IVec4, i32, 4>();
    }

    #[test]
    fn negation_exists_exactly_where_it_makes_sense() {
        for has_neg in [
            implements_neg!(Vec2),
            implements_neg!(Vec3),
            implements_neg!(Vec4),
            implements_neg!(DVec2),
            implements_neg!(DVec3),
            implements_neg!(DVec4),
            implements_neg!(IVec2),
            implements_neg!(IVec3),
            implements_neg!(IVec4),
        ] {
            assert!(has_neg);
        }
        for has_neg in [
            implements_neg!(UVec2),
            implements_neg!(UVec3),
            implements_neg!(UVec4),
        ] {
            assert!(!has_neg);
        }
    }

    #[test]
    fn integer_vectors_are_eq_and_hash() {
        assert_eq_and_hash::<IVec2>();
        assert_eq_and_hash::<IVec3>();
        assert_eq_and_hash::<IVec4>();
        assert_eq_and_hash::<UVec2>();
        assert_eq_and_hash::<UVec3>();
        assert_eq_and_hash::<UVec4>();
    }

    #[test]
    fn negation_flips_the_sign_of_zero_components_only_for_floats() {
        assert_eq!(-IVec3::ZERO, IVec3::ZERO);
        assert!((-Vec2::new(0.0, 1.0)).x.is_sign_negative());
    }

    #[test]
    fn constructors_work_in_const_contexts() {
        const V: Vec3 = Vec3::new(1.0, 2.0, 3.0);
        const ARRAY: [f32; 3] = V.to_array();
        const SPLAT: IVec2 = IVec2::splat(-4);
        const FROM_ARRAY: UVec4 = UVec4::from_array([1, 2, 3, 4]);
        const AXIS: DVec3 = DVec3::Y;
        assert_eq!(ARRAY, [1.0, 2.0, 3.0]);
        assert_eq!(SPLAT, IVec2::new(-4, -4));
        assert_eq!(FROM_ARRAY.w, 4);
        assert_eq!(AXIS, DVec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn vectors_are_copy() {
        let a = Vec3::ONE;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn float_division_by_zero_is_infinite_not_a_panic() {
        let v = Vec2::new(1.0, -1.0) / 0.0;
        assert_eq!(v.x, f32::INFINITY);
        assert_eq!(v.y, f32::NEG_INFINITY);
        assert!((Vec2::ZERO / 0.0).x.is_nan());
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn integer_division_by_zero_panics() {
        let _ = IVec3::ONE / 0;
    }

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn unsigned_division_by_zero_panics() {
        let _ = UVec2::ONE / 0;
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn integer_overflow_panics_in_debug_builds() {
        let _ = IVec2::new(i32::MAX, 0) + IVec2::X;
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "attempt to subtract with overflow")]
    fn unsigned_underflow_panics_in_debug_builds() {
        let _ = UVec2::ZERO - UVec2::X;
    }

    #[test]
    fn scalar_on_the_left_matches_scalar_on_the_right() {
        assert_eq!(
            3.0 * Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(1.0, 2.0, 3.0, 4.0) * 3.0
        );
        assert_eq!(2 * IVec2::new(-3, 4), IVec2::new(-6, 8));
        assert_eq!(5 * UVec3::new(1, 2, 3), UVec3::new(5, 10, 15));
    }

    #[test]
    fn debug_output_names_the_type_and_fields() {
        // `format!` needs `alloc`, so check through a fixed buffer.
        use core::fmt::Write;
        struct Sink([u8; 64], usize);
        impl Write for Sink {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                self.0[self.1..self.1 + s.len()].copy_from_slice(s.as_bytes());
                self.1 += s.len();
                Ok(())
            }
        }
        let mut sink = Sink([0; 64], 0);
        write!(sink, "{:?}", IVec2::new(1, 2)).unwrap();
        assert_eq!(
            core::str::from_utf8(&sink.0[..sink.1]).unwrap(),
            "IVec2 { x: 1, y: 2 }"
        );
    }
}
