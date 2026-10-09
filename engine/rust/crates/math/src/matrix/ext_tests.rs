//! Tests for the Phase 2.4 matrix helpers: `is_finite`/`is_nan`,
//! `approx_eq*`, `Display`, the `f32`/`f64` conversions, and the
//! quaternion-based constructors (`from_euler`, `from_rotation_translation`).

use super::*;
use crate::angle::Radians;
use crate::quat::{DQuat, EulerRot, Quat};
use crate::test_support::{text, text_with_precision};
use crate::vector::{DVec3, Vec3};

macro_rules! helper_tests {
    ($module:ident, $scalar:ty, $m2:ident, $m3:ident, $m4:ident,
     $other2:ident, $other3:ident, $other4:ident,
     $as2:ident, $as3:ident, $as4:ident, $back2:ident, $back3:ident, $back4:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn finite_and_nan_look_at_every_element() {
                assert!($m2::IDENTITY.is_finite() && !$m2::IDENTITY.is_nan());
                assert!($m3::IDENTITY.is_finite() && !$m3::IDENTITY.is_nan());
                assert!($m4::IDENTITY.is_finite() && !$m4::IDENTITY.is_nan());

                // The bad value is in the last column, the easiest one to miss.
                let mut m = $m4::IDENTITY;
                m.w_axis.w = <$scalar>::INFINITY;
                assert!(!m.is_finite() && !m.is_nan());
                m.w_axis.w = <$scalar>::NAN;
                assert!(!m.is_finite() && m.is_nan());

                let mut m = $m3::IDENTITY;
                m.z_axis.z = <$scalar>::NAN;
                assert!(m.is_nan() && !m.is_finite());
                let mut m = $m2::IDENTITY;
                m.y_axis.y = <$scalar>::NAN;
                assert!(m.is_nan() && !m.is_finite());
            }

            #[test]
            fn approximate_equality() {
                let a = $m4::from_translation_helper();
                assert!(a.approx_eq(a));
                let mut b = a;
                b.w_axis.x += 1e-3;
                assert!(!a.approx_eq(b));
                assert!(!a.approx_eq_eps(b, 1e-5));
                assert!(a.approx_eq_eps(b, 1e-2));
                assert!(a.approx_eq_rel(b, 1e-2));
                assert!(!a.approx_eq_rel(b, 1e-6));
                // A difference in the very last element is seen too.
                let mut c = a;
                c.w_axis.w += 0.5;
                assert!(!a.approx_eq_eps(c, 0.1));
                let mut nan = a;
                nan.x_axis.x = <$scalar>::NAN;
                assert!(!a.approx_eq(nan) && !nan.approx_eq(nan));
                assert!($m2::IDENTITY.approx_eq_eps($m2::IDENTITY, 0.0));
                assert!($m3::IDENTITY.approx_eq($m3::IDENTITY));
            }

            #[test]
            fn display_lists_the_columns() {
                assert_eq!(text(&$m2::IDENTITY).as_str(), "[[1, 0], [0, 1]]");
                assert_eq!(
                    text(&$m3::IDENTITY).as_str(),
                    "[[1, 0, 0], [0, 1, 0], [0, 0, 1]]"
                );
                assert_eq!(
                    text(&$m4::IDENTITY).as_str(),
                    "[[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]"
                );
                assert_eq!(
                    text_with_precision(&$m2::IDENTITY, 1).as_str(),
                    "[[1.0, 0.0], [0.0, 1.0]]"
                );
            }

            #[test]
            fn conversions_keep_the_columns() {
                let m = $m2::from_cols_array([0.5, -1.0, 2.0, 4.0]);
                let other: $other2 = m.$as2();
                assert_eq!(other.to_cols_array(), [0.5, -1.0, 2.0, 4.0]);
                assert_eq!(other.$back2(), m);

                let m = $m3::from_cols_array([0.5, 1.0, 2.0, -3.0, 4.0, 5.0, 6.0, 7.0, -8.0]);
                let other: $other3 = m.$as3();
                assert_eq!(
                    other.to_cols_array(),
                    [0.5, 1.0, 2.0, -3.0, 4.0, 5.0, 6.0, 7.0, -8.0]
                );
                assert_eq!(other.$back3(), m);

                let array = [
                    0.5, 1.0, 2.0, 3.0, -4.0, 5.0, 6.0, 7.0, 8.0, 9.0, -10.0, 11.0, 12.0, 13.0,
                    14.0, 0.25,
                ];
                let m = $m4::from_cols_array(array);
                let other: $other4 = m.$as4();
                assert_eq!(
                    other.to_cols_array().map(|x| x as f64),
                    array.map(|x| x as f64)
                );
                assert_eq!(other.$back4(), m);
            }
        }
    };
}

/// A transform with distinct, non-trivial elements, shared by the tests.
trait Sample {
    fn from_translation_helper() -> Self;
}

impl Sample for Mat4 {
    fn from_translation_helper() -> Self {
        Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(Radians::new(0.7)),
            Vec3::new(5.0, -6.0, 7.0),
        )
    }
}

impl Sample for DMat4 {
    fn from_translation_helper() -> Self {
        DMat4::from_scale_rotation_translation(
            DVec3::new(1.0, 2.0, 3.0),
            DQuat::from_rotation_y(Radians::new(0.7)),
            DVec3::new(5.0, -6.0, 7.0),
        )
    }
}

helper_tests!(
    matrices_f32,
    f32,
    Mat2,
    Mat3,
    Mat4,
    DMat2,
    DMat3,
    DMat4,
    as_dmat2,
    as_dmat3,
    as_dmat4,
    as_mat2,
    as_mat3,
    as_mat4
);
helper_tests!(
    matrices_f64,
    f64,
    DMat2,
    DMat3,
    DMat4,
    Mat2,
    Mat3,
    Mat4,
    as_mat2,
    as_mat3,
    as_mat4,
    as_dmat2,
    as_dmat3,
    as_dmat4
);

macro_rules! constructor_tests {
    ($module:ident, $scalar:ty, $m3:ident, $m4:ident, $quat:ident, $v3:ident) => {
        mod $module {
            use super::*;

            const ORDERS: [EulerRot; 4] =
                [EulerRot::XYZ, EulerRot::YXZ, EulerRot::ZYXEx, EulerRot::ZXZ];

            #[test]
            fn from_euler_matches_the_quaternion() {
                let (a, b, c) = (Radians::new(0.4), Radians::new(-1.1), Radians::new(2.2));
                for order in ORDERS {
                    let q = $quat::from_euler(order, a, b, c);
                    assert!($m3::from_euler(order, a, b, c).approx_eq_eps(q.to_mat3(), 1e-6));
                    assert!($m4::from_euler(order, a, b, c).approx_eq_eps(q.to_mat4(), 1e-6));
                }
            }

            #[test]
            fn from_euler_rotates_like_the_quaternion() {
                let (a, b, c) = (Radians::new(0.9), Radians::new(0.3), Radians::new(-0.6));
                let q = $quat::from_euler(EulerRot::YXZ, a, b, c);
                let m = $m3::from_euler(EulerRot::YXZ, a, b, c);
                let v = $v3::new(1.0, -2.0, 0.5);
                assert!((m * v - q * v).length() < 1e-5);
            }

            #[test]
            fn from_rotation_translation_is_translate_after_rotate() {
                let q = $quat::from_euler(
                    EulerRot::YXZ,
                    Radians::new(0.5),
                    Radians::new(-0.2),
                    Radians::new(1.3),
                );
                let t = $v3::new(3.0, -4.0, 5.0);
                let m = $m4::from_rotation_translation(q, t);
                assert!(
                    m.approx_eq_eps($m4::from_scale_rotation_translation($v3::ONE, q, t), 1e-6)
                );
                assert_eq!(m.w_axis.truncate(), t);
                assert_eq!(m.w_axis.w, 1.0);
                let p = $v3::new(1.0, 2.0, 3.0);
                assert!((m.transform_point3(p) - (q * p + t)).length() < 1e-5);
            }

            #[test]
            fn from_rotation_translation_of_identity_is_a_translation() {
                let t = $v3::new(1.0, 2.0, 3.0);
                assert_eq!(
                    $m4::from_rotation_translation($quat::IDENTITY, t),
                    $m4::from_translation(t)
                );
            }
        }
    };
}

constructor_tests!(constructors_f32, f32, Mat3, Mat4, Quat, Vec3);
constructor_tests!(constructors_f64, f64, DMat3, DMat4, DQuat, DVec3);
