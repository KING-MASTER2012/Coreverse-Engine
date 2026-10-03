//! Tests for the column-major matrices of Phase 1.9.
//!
//! Instantiated once per float family (`Mat*` over `f32`, `DMat*` over `f64`).
//! The test matrices have small integer elements, so products and
//! determinants are exact; inverses are compared with a tolerance. The
//! determinant is also checked against an independent reference (Laplace
//! expansion over plain arrays, since this crate can be `no_std`).

use super::*;
use crate::vector::*;

/// Converts an element to `f64` without tripping `useless_conversion` for the
/// `f64` family.
fn to_f64<T: Into<f64>>(value: T) -> f64 {
    value.into()
}

/// Determinant of the top-left `n x n` block of `m` (row-major), by Laplace
/// expansion along the first remaining row. `used` is the set of columns
/// already taken by earlier rows.
fn reference_det(m: &[[f64; 4]; 4], n: usize, row: usize, used: u8) -> f64 {
    if row == n {
        return 1.0;
    }
    let mut total = 0.0;
    let mut position = 0;
    for column in 0..n {
        if used & (1 << column) != 0 {
            continue;
        }
        let sign = if position % 2 == 0 { 1.0 } else { -1.0 };
        total += sign * m[row][column] * reference_det(m, n, row + 1, used | (1 << column));
        position += 1;
    }
    total
}

/// The reference determinant of a matrix given as its column-major array.
fn reference_det_of(cols: &[f64], n: usize) -> f64 {
    let mut rows = [[0.0; 4]; 4];
    for column in 0..n {
        for row in 0..n {
            rows[row][column] = cols[column * n + row];
        }
    }
    reference_det(&rows, n, 0, 0)
}

macro_rules! matrix_tests {
    (
        $module:ident, $scalar:ty,
        $m2:ident, $m3:ident, $m4:ident,
        $v2:ident, $v3:ident, $v4:ident
    ) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;

            fn close_all<const N: usize>(a: [$scalar; N], b: [$scalar; N]) -> bool {
                a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= TOL)
            }

            macro_rules! assert_close {
                ($a:expr, $b:expr) => {{
                    let (a, b) = ($a, $b);
                    assert!(
                        close_all(a.to_cols_array(), b.to_cols_array()),
                        "{:?} is not close to {:?}",
                        a,
                        b
                    );
                }};
            }

            // Written row by row below, so these are the transposes of how
            // `from_cols_array` reads them: each line is one column.

            /// `[[1, 2], [3, 4]]`, determinant -2.
            fn a2() -> $m2 {
                $m2::from_cols_array([1.0, 3.0, 2.0, 4.0])
            }

            /// `[[5, 6], [7, 8]]`, determinant -2.
            fn b2() -> $m2 {
                $m2::from_cols_array([5.0, 7.0, 6.0, 8.0])
            }

            /// `[[2, 1, 0], [0, 3, 1], [1, 0, 4]]`, determinant 25.
            fn a3() -> $m3 {
                $m3::from_cols_array([2.0, 0.0, 1.0, 1.0, 3.0, 0.0, 0.0, 1.0, 4.0])
            }

            /// `[[1, 2, 3], [4, 5, 6], [7, 8, 10]]`, determinant -3.
            fn b3() -> $m3 {
                $m3::from_cols_array([1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 10.0])
            }

            /// `[[2, 1, 0, 1], [0, 3, 1, 0], [1, 0, 4, 2], [0, 1, 0, 5]]`.
            fn a4() -> $m4 {
                $m4::from_cols_array([
                    2.0, 0.0, 1.0, 0.0, 1.0, 3.0, 0.0, 1.0, 0.0, 1.0, 4.0, 0.0, 1.0, 0.0, 2.0, 5.0,
                ])
            }

            /// `[[1, 2, 0, 0], [3, 1, 0, 1], [0, 0, 2, 1], [1, 0, 1, 3]]`.
            fn b4() -> $m4 {
                $m4::from_cols_array([
                    1.0, 3.0, 0.0, 1.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0, 1.0, 1.0, 3.0,
                ])
            }

            /// A fully populated, non-symmetric matrix with fractions.
            fn p4() -> $m4 {
                $m4::from_cols_array([
                    0.5, 3.0, 1.0, -1.0, -1.0, 0.25, 2.0, 0.0, 2.0, -1.0, 0.5, 1.0, 0.0, 1.0, -2.0,
                    4.0,
                ])
            }

            /// A translation by `(5, 6, 7)`: the offset is the last column.
            fn translation() -> $m4 {
                $m4::from_cols($v4::X, $v4::Y, $v4::Z, $v4::new(5.0, 6.0, 7.0, 1.0))
            }

            #[test]
            fn identity_and_zero_constants() {
                assert_eq!($m2::IDENTITY.to_cols_array(), [1.0, 0.0, 0.0, 1.0]);
                assert_eq!(
                    $m3::IDENTITY.to_cols_array(),
                    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                );
                assert_eq!(
                    $m4::IDENTITY.to_cols_array_2d(),
                    [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0]
                    ]
                );
                assert_eq!($m2::ZERO.to_cols_array(), [0.0; 4]);
                assert_eq!($m3::ZERO.to_cols_array(), [0.0; 9]);
                assert_eq!($m4::ZERO.to_cols_array(), [0.0; 16]);
            }

            #[test]
            fn default_is_the_identity() {
                assert_eq!($m2::default(), $m2::IDENTITY);
                assert_eq!($m3::default(), $m3::IDENTITY);
                assert_eq!($m4::default(), $m4::IDENTITY);
            }

            #[test]
            fn the_memory_layout_is_the_columns_without_padding() {
                assert_eq!(size_of::<$m2>(), 4 * size_of::<$scalar>());
                assert_eq!(size_of::<$m3>(), 9 * size_of::<$scalar>());
                assert_eq!(size_of::<$m4>(), 16 * size_of::<$scalar>());
            }

            #[test]
            fn from_cols_stores_the_columns_in_the_axis_fields() {
                let m = $m3::from_cols(
                    $v3::new(1.0, 2.0, 3.0),
                    $v3::new(4.0, 5.0, 6.0),
                    $v3::new(7.0, 8.0, 9.0),
                );
                assert_eq!(m.x_axis, $v3::new(1.0, 2.0, 3.0));
                assert_eq!(m.y_axis, $v3::new(4.0, 5.0, 6.0));
                assert_eq!(m.z_axis, $v3::new(7.0, 8.0, 9.0));
                let m = $m2::from_cols($v2::new(1.0, 2.0), $v2::new(3.0, 4.0));
                assert_eq!(
                    (m.x_axis, m.y_axis),
                    ($v2::new(1.0, 2.0), $v2::new(3.0, 4.0))
                );
                assert_eq!(translation().w_axis, $v4::new(5.0, 6.0, 7.0, 1.0));
            }

            #[test]
            fn flat_arrays_are_column_major() {
                let m = $m3::from_cols_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
                // The first three numbers are the first column.
                assert_eq!(m.x_axis, $v3::new(1.0, 2.0, 3.0));
                assert_eq!(m.y_axis, $v3::new(4.0, 5.0, 6.0));
                assert_eq!(m.z_axis, $v3::new(7.0, 8.0, 9.0));
                assert_eq!(
                    m.to_cols_array(),
                    [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
                );
            }

            #[test]
            fn array_conversions_round_trip() {
                let m = a4();
                assert_eq!($m4::from_cols_array(m.to_cols_array()), m);
                assert_eq!($m4::from_cols_array_2d(m.to_cols_array_2d()), m);
                let nested: [[$scalar; 3]; 3] = a3().into();
                assert_eq!(nested, a3().to_cols_array_2d());
                assert_eq!($m3::from(nested), a3());
                // `array_2d[c][r]` is column `c`, row `r`.
                assert_eq!(a2().to_cols_array_2d(), [[1.0, 3.0], [2.0, 4.0]]);
            }

            #[test]
            fn columns_and_rows_are_accessible() {
                let m = a3();
                assert_eq!(m.col(0), $v3::new(2.0, 0.0, 1.0));
                assert_eq!(m.col(2), $v3::new(0.0, 1.0, 4.0));
                assert_eq!(m.row(0), $v3::new(2.0, 1.0, 0.0));
                assert_eq!(m.row(1), $v3::new(0.0, 3.0, 1.0));
                assert_eq!(m.row(2), $v3::new(1.0, 0.0, 4.0));
                assert_eq!(a2().row(1), $v2::new(3.0, 4.0));
                assert_eq!(translation().row(0), $v4::new(1.0, 0.0, 0.0, 5.0));
                for index in 0..3 {
                    assert_eq!(m[index], m.col(index));
                }
            }

            #[test]
            fn index_mut_edits_a_column() {
                let mut m = $m3::IDENTITY;
                m[1] = $v3::new(7.0, 8.0, 9.0);
                m[2].x = -1.0;
                assert_eq!(
                    m.to_cols_array(),
                    [1.0, 0.0, 0.0, 7.0, 8.0, 9.0, -1.0, 0.0, 1.0]
                );
            }

            #[test]
            #[should_panic(expected = "column index out of bounds")]
            fn col_panics_out_of_bounds() {
                let _ = $m3::IDENTITY.col(3);
            }

            #[test]
            #[should_panic(expected = "row index out of bounds")]
            fn row_panics_out_of_bounds() {
                let _ = $m2::IDENTITY.row(2);
            }

            #[test]
            #[should_panic(expected = "index out of bounds: the len is 4 but the index is 4")]
            fn indexing_panics_out_of_bounds() {
                let _ = $m4::IDENTITY[4];
            }

            #[test]
            #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
            fn index_mut_panics_out_of_bounds() {
                let mut m = $m2::IDENTITY;
                m[2] = $v2::ZERO;
            }

            #[test]
            fn transpose_swaps_rows_and_columns() {
                assert_eq!(a2().transpose().to_cols_array(), [1.0, 2.0, 3.0, 4.0]);
                let m = a3();
                let t = m.transpose();
                for index in 0..3 {
                    assert_eq!(t.col(index), m.row(index));
                    assert_eq!(t.row(index), m.col(index));
                }
                assert_eq!(
                    translation().transpose().x_axis,
                    $v4::new(1.0, 0.0, 0.0, 5.0)
                );
            }

            #[test]
            fn transposing_twice_is_the_identity_operation() {
                assert_eq!(a2().transpose().transpose(), a2());
                assert_eq!(b3().transpose().transpose(), b3());
                assert_eq!(p4().transpose().transpose(), p4());
                assert_eq!($m4::IDENTITY.transpose(), $m4::IDENTITY);
            }

            #[test]
            fn transpose_of_a_product_reverses_the_order() {
                assert_eq!(
                    (a2() * b2()).transpose(),
                    b2().transpose() * a2().transpose()
                );
                assert_eq!(
                    (a3() * b3()).transpose(),
                    b3().transpose() * a3().transpose()
                );
                assert_eq!(
                    (a4() * b4()).transpose(),
                    b4().transpose() * a4().transpose()
                );
            }

            #[test]
            fn matrix_times_vector() {
                // 90 degrees counter-clockwise: X goes to Y, Y goes to -X.
                let rotate = $m2::from_cols($v2::Y, -$v2::X);
                assert_eq!(rotate * $v2::X, $v2::Y);
                assert_eq!(rotate * $v2::Y, -$v2::X);
                assert_eq!(rotate * $v2::new(2.0, 3.0), $v2::new(-3.0, 2.0));

                let m = $m3::from_cols_array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
                // 1 * column0 + 0 * column1 - 1 * column2
                assert_eq!(m * $v3::new(1.0, 0.0, -1.0), $v3::new(-6.0, -6.0, -6.0));
                assert_eq!(m * $v3::X, m.col(0));
                assert_eq!(m * $v3::Z, m.col(2));
                assert_eq!(
                    m.mul_vec($v3::new(1.0, 1.0, 1.0)),
                    $v3::new(12.0, 15.0, 18.0)
                );

                assert_eq!(
                    $m4::IDENTITY * $v4::new(1.0, 2.0, 3.0, 4.0),
                    $v4::new(1.0, 2.0, 3.0, 4.0)
                );
                assert_eq!($m3::ZERO * $v3::ONE, $v3::ZERO);
            }

            #[test]
            fn a_translation_matrix_moves_points_but_not_directions() {
                let t = translation();
                // w = 1: a point, translated.
                assert_eq!(
                    t * $v4::new(1.0, 2.0, 3.0, 1.0),
                    $v4::new(6.0, 8.0, 10.0, 1.0)
                );
                // w = 0: a direction, unchanged.
                assert_eq!(
                    t * $v4::new(1.0, 2.0, 3.0, 0.0),
                    $v4::new(1.0, 2.0, 3.0, 0.0)
                );
            }

            #[test]
            fn matrix_times_matrix() {
                assert_eq!((a2() * b2()).to_cols_array(), [19.0, 43.0, 22.0, 50.0]);
                assert_eq!((b2() * a2()).to_cols_array(), [23.0, 31.0, 34.0, 46.0]);
                assert_ne!(a2() * b2(), b2() * a2());
                assert_eq!(a3().mul_mat(b3()), a3() * b3());
            }

            #[test]
            fn identity_and_zero_in_products() {
                assert_eq!($m2::IDENTITY * a2(), a2());
                assert_eq!(a2() * $m2::IDENTITY, a2());
                assert_eq!($m3::IDENTITY * b3(), b3());
                assert_eq!(b3() * $m3::IDENTITY, b3());
                assert_eq!($m4::IDENTITY * p4(), p4());
                assert_eq!(p4() * $m4::IDENTITY, p4());
                assert_eq!(a3() * $m3::ZERO, $m3::ZERO);
                assert_eq!($m4::ZERO * a4(), $m4::ZERO);
            }

            #[test]
            fn a_product_applies_the_right_matrix_first() {
                let v2 = $v2::new(1.0, -2.0);
                assert_eq!((a2() * b2()) * v2, a2() * (b2() * v2));
                let v3 = $v3::new(1.0, -2.0, 3.0);
                assert_eq!((a3() * b3()) * v3, a3() * (b3() * v3));
                let v4 = $v4::new(1.0, -2.0, 3.0, 1.0);
                assert_eq!((a4() * b4()) * v4, a4() * (b4() * v4));
                // Translate, then rotate about Z is not rotate, then translate.
                let rotate = $m4::from_cols($v4::Y, -$v4::X, $v4::Z, $v4::W);
                let point = $v4::new(1.0, 0.0, 0.0, 1.0);
                assert_eq!(
                    (rotate * translation()) * point,
                    $v4::new(-6.0, 6.0, 7.0, 1.0)
                );
                assert_eq!(
                    (translation() * rotate) * point,
                    $v4::new(5.0, 7.0, 7.0, 1.0)
                );
            }

            #[test]
            fn products_are_associative() {
                assert_eq!((a3() * b3()) * a3(), a3() * (b3() * a3()));
                assert_eq!((a4() * b4()) * a4(), a4() * (b4() * a4()));
                assert_eq!((a2() * b2()) * a2(), a2() * (b2() * a2()));
            }

            #[test]
            fn mul_assign_is_the_product() {
                let mut m = a3();
                m *= b3();
                assert_eq!(m, a3() * b3());
                let mut m = a4();
                m *= $m4::IDENTITY;
                assert_eq!(m, a4());
            }

            #[test]
            fn determinants_of_known_matrices() {
                assert_eq!($m2::IDENTITY.determinant(), 1.0);
                assert_eq!($m3::IDENTITY.determinant(), 1.0);
                assert_eq!($m4::IDENTITY.determinant(), 1.0);
                assert_eq!($m4::ZERO.determinant(), 0.0);
                assert_eq!(a2().determinant(), -2.0);
                assert_eq!(a3().determinant(), 25.0);
                assert_eq!(b3().determinant(), -3.0);
                assert_eq!(translation().determinant(), 1.0);
                let scale = $m4::from_cols($v4::X * 2.0, $v4::Y * 3.0, $v4::Z * 4.0, $v4::W * 5.0);
                assert_eq!(scale.determinant(), 120.0);
                // A rotation keeps orientation, a mirror flips it.
                assert_eq!($m2::from_cols($v2::Y, -$v2::X).determinant(), 1.0);
                assert_eq!($m2::from_cols($v2::X, -$v2::Y).determinant(), -1.0);
            }

            #[test]
            fn determinants_match_an_independent_reference() {
                let check2 = |m: $m2| {
                    let cols = m.to_cols_array().map(to_f64);
                    let expected = reference_det_of(&cols, 2);
                    assert!((to_f64(m.determinant()) - expected).abs() < 1e-3, "{m:?}");
                };
                let check3 = |m: $m3| {
                    let cols = m.to_cols_array().map(to_f64);
                    let expected = reference_det_of(&cols, 3);
                    assert!((to_f64(m.determinant()) - expected).abs() < 1e-3, "{m:?}");
                };
                let check4 = |m: $m4| {
                    let cols = m.to_cols_array().map(to_f64);
                    let expected = reference_det_of(&cols, 4);
                    assert!((to_f64(m.determinant()) - expected).abs() < 1e-3, "{m:?}");
                };
                for m in [a2(), b2(), a2() * b2(), a2().transpose()] {
                    check2(m);
                }
                for m in [a3(), b3(), a3() * b3(), b3().transpose()] {
                    check3(m);
                }
                for m in [
                    a4(),
                    b4(),
                    p4(),
                    a4() * b4(),
                    p4().transpose(),
                    translation(),
                ] {
                    check4(m);
                }
            }

            #[test]
            fn determinant_laws() {
                // det(A B) = det(A) det(B)
                assert_eq!(
                    (a2() * b2()).determinant(),
                    a2().determinant() * b2().determinant()
                );
                assert_eq!(
                    (a3() * b3()).determinant(),
                    a3().determinant() * b3().determinant()
                );
                assert_eq!(
                    (a4() * b4()).determinant(),
                    a4().determinant() * b4().determinant()
                );
                // det(A^T) = det(A)
                assert_eq!(a3().transpose().determinant(), a3().determinant());
                assert_eq!(p4().transpose().determinant(), p4().determinant());
                // Swapping two columns negates it.
                let m = a4();
                let swapped = $m4::from_cols(m.y_axis, m.x_axis, m.z_axis, m.w_axis);
                assert_eq!(swapped.determinant(), -m.determinant());
                // Two equal columns make it zero.
                let equal = $m3::from_cols(b3().x_axis, b3().x_axis, b3().z_axis);
                assert_eq!(equal.determinant(), 0.0);
            }

            #[test]
            fn inverse_of_known_matrices() {
                assert_eq!(
                    a2().inverse().unwrap().to_cols_array(),
                    [-2.0, 1.5, 1.0, -0.5]
                );
                assert_eq!($m2::IDENTITY.inverse(), Some($m2::IDENTITY));
                assert_eq!($m3::IDENTITY.inverse(), Some($m3::IDENTITY));
                assert_eq!($m4::IDENTITY.inverse(), Some($m4::IDENTITY));
                // Undoing a translation negates the offset.
                let undo = translation().inverse().unwrap();
                assert_eq!(undo.w_axis, $v4::new(-5.0, -6.0, -7.0, 1.0));
                assert_eq!(undo * translation(), $m4::IDENTITY);
                // The inverse of a rotation is its transpose.
                let rotate = $m3::from_cols($v3::Y, -$v3::X, $v3::Z);
                assert_eq!(rotate.inverse(), Some(rotate.transpose()));
                // A uniform scale inverts to the reciprocal scale.
                let scale = $m3::from_cols($v3::X * 2.0, $v3::Y * 4.0, $v3::Z * 0.5);
                assert_eq!(
                    scale.inverse().unwrap(),
                    $m3::from_cols($v3::X * 0.5, $v3::Y * 0.25, $v3::Z * 2.0)
                );
            }

            #[test]
            fn a_matrix_times_its_inverse_is_the_identity() {
                let m = a2();
                assert_close!(m * m.inverse().unwrap(), $m2::IDENTITY);
                assert_close!(m.inverse().unwrap() * m, $m2::IDENTITY);
                let m = b2();
                assert_close!(m * m.inverse().unwrap(), $m2::IDENTITY);
                assert_close!(m.inverse().unwrap() * m, $m2::IDENTITY);
                for m in [a3(), b3()] {
                    assert_close!(m * m.inverse().unwrap(), $m3::IDENTITY);
                    assert_close!(m.inverse().unwrap() * m, $m3::IDENTITY);
                }
                for m in [a4(), b4(), p4(), translation()] {
                    assert_close!(m * m.inverse().unwrap(), $m4::IDENTITY);
                    assert_close!(m.inverse().unwrap() * m, $m4::IDENTITY);
                }
            }

            #[test]
            fn inverse_undoes_a_transformation() {
                let v = $v4::new(1.0, -2.0, 3.0, 1.0);
                for m in [a4(), b4(), p4(), translation()] {
                    let back = m.inverse().unwrap() * (m * v);
                    assert!(
                        close_all(back.to_array(), v.to_array()),
                        "{back:?} is not {v:?}"
                    );
                }
            }

            #[test]
            fn inverse_laws() {
                // (A^-1)^-1 = A
                assert_close!(a3().inverse().unwrap().inverse().unwrap(), a3());
                assert_close!(p4().inverse().unwrap().inverse().unwrap(), p4());
                // (A B)^-1 = B^-1 A^-1
                assert_close!(
                    (a3() * b3()).inverse().unwrap(),
                    b3().inverse().unwrap() * a3().inverse().unwrap()
                );
                assert_close!(
                    (a4() * b4()).inverse().unwrap(),
                    b4().inverse().unwrap() * a4().inverse().unwrap()
                );
                // (A^T)^-1 = (A^-1)^T
                assert_close!(
                    a4().transpose().inverse().unwrap(),
                    a4().inverse().unwrap().transpose()
                );
                assert_close!(
                    p4().transpose().inverse().unwrap(),
                    p4().inverse().unwrap().transpose()
                );
                // det(A^-1) = 1 / det(A)
                let det = a3().inverse().unwrap().determinant();
                assert!((det - 1.0 / a3().determinant()).abs() <= TOL);
            }

            #[test]
            fn singular_matrices_have_no_inverse() {
                assert_eq!($m2::ZERO.inverse(), None);
                assert_eq!($m3::ZERO.inverse(), None);
                assert_eq!($m4::ZERO.inverse(), None);
                // Dependent rows or columns.
                assert_eq!($m2::from_cols_array([1.0, 2.0, 2.0, 4.0]).inverse(), None);
                assert_eq!(
                    $m3::from_cols_array([1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]).inverse(),
                    None
                );
                let m = a4();
                assert_eq!(
                    $m4::from_cols(m.x_axis, m.x_axis, m.z_axis, m.w_axis).inverse(),
                    None
                );
                // Projecting onto a plane loses a dimension.
                assert_eq!($m3::from_cols($v3::X, $v3::Y, $v3::ZERO).inverse(), None);
            }

            #[test]
            fn matrices_with_nan_or_infinite_determinants_have_no_inverse() {
                let mut nan = $m3::IDENTITY;
                nan.y_axis.y = <$scalar>::NAN;
                assert_eq!(nan.inverse(), None);
                let mut nan = $m4::IDENTITY;
                nan.w_axis.x = <$scalar>::NAN;
                assert_eq!(nan.inverse(), None);
                // The determinant overflows to infinity.
                let huge = $m2::from_cols($v2::X * <$scalar>::MAX, $v2::Y * <$scalar>::MAX);
                assert!(huge.determinant().is_infinite());
                assert_eq!(huge.inverse(), None);
            }

            #[test]
            fn nearly_singular_matrices_still_invert() {
                let m = $m2::from_cols_array([1.0, 1.0, 1.0, 1.0 + 1e-3]);
                let inverse = m.inverse().unwrap();
                assert!(inverse.x_axis.x.abs() > 900.0, "{inverse:?}");
            }

            #[test]
            fn const_operations_work_in_const_contexts() {
                const A: $m3 = $m3::from_cols_array([2.0, 0.0, 1.0, 1.0, 3.0, 0.0, 0.0, 1.0, 4.0]);
                const B: $m3 = $m3::from_cols_array([1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 10.0]);
                const SINGULAR: $m2 = $m2::from_cols_array([1.0, 2.0, 2.0, 4.0]);
                const ID: $m3 = $m3::IDENTITY;
                const TRANSPOSED: $m3 = A.transpose();
                const PRODUCT: $m3 = A.mul_mat(B);
                const DETERMINANT: $scalar = A.determinant();
                const INVERSE: Option<$m3> = A.inverse();
                const NO_INVERSE: Option<$m2> = SINGULAR.inverse();
                const IMAGE: $v3 = A.mul_vec($v3::ONE);
                const COLUMN: $v3 = A.col(1);
                const ROW: $v3 = A.row(1);
                const FLAT: [$scalar; 9] = A.to_cols_array();
                assert_eq!(TRANSPOSED, A.transpose());
                assert_eq!(PRODUCT, A * B);
                assert_eq!(DETERMINANT, 25.0);
                assert_eq!(INVERSE, A.inverse());
                assert_eq!(NO_INVERSE, None);
                assert_eq!(IMAGE, $v3::new(3.0, 4.0, 5.0));
                assert_eq!(
                    (COLUMN, ROW),
                    ($v3::new(1.0, 3.0, 0.0), $v3::new(0.0, 3.0, 1.0))
                );
                assert_eq!(FLAT[0], 2.0);
                assert_eq!(ID * ID, ID);
            }
        }
    };
}

matrix_tests!(mat_f32, f32, Mat2, Mat3, Mat4, Vec2, Vec3, Vec4);
matrix_tests!(dmat_f64, f64, DMat2, DMat3, DMat4, DVec2, DVec3, DVec4);
