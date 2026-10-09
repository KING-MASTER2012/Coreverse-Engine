//! Tests for the Euler conversions of Phase 2.4.
//!
//! Instantiated once per float family. Rotations are compared as rotations
//! (`q` and `-q` are the same one), and `to_euler` is checked by round trip
//! (`from_euler(to_euler(q)) == q`) for every one of the 24 orders.

use super::*;
use crate::angle::Degrees;

const TAIT_BRYAN: [EulerRot; 6] = [
    EulerRot::XYZ,
    EulerRot::XZY,
    EulerRot::YXZ,
    EulerRot::YZX,
    EulerRot::ZXY,
    EulerRot::ZYX,
];
const PROPER: [EulerRot; 6] = [
    EulerRot::XYX,
    EulerRot::XZX,
    EulerRot::YXY,
    EulerRot::YZY,
    EulerRot::ZXZ,
    EulerRot::ZYZ,
];
const ALL: [EulerRot; 24] = [
    EulerRot::XYZ,
    EulerRot::XZY,
    EulerRot::YXZ,
    EulerRot::YZX,
    EulerRot::ZXY,
    EulerRot::ZYX,
    EulerRot::XYX,
    EulerRot::XZX,
    EulerRot::YXY,
    EulerRot::YZY,
    EulerRot::ZXZ,
    EulerRot::ZYZ,
    EulerRot::XYZEx,
    EulerRot::XZYEx,
    EulerRot::YXZEx,
    EulerRot::YZXEx,
    EulerRot::ZXYEx,
    EulerRot::ZYXEx,
    EulerRot::XYXEx,
    EulerRot::XZXEx,
    EulerRot::YXYEx,
    EulerRot::YZYEx,
    EulerRot::ZXZEx,
    EulerRot::ZYZEx,
];

/// Whether the order repeats its first axis (proper Euler), intrinsic or
/// extrinsic: then the middle angle is in `[0, PI]`, otherwise in
/// `[-PI/2, PI/2]`.
fn is_proper(order: EulerRot) -> bool {
    PROPER.contains(&order)
        || matches!(
            order,
            EulerRot::XYXEx
                | EulerRot::XZXEx
                | EulerRot::YXYEx
                | EulerRot::YZYEx
                | EulerRot::ZXZEx
                | EulerRot::ZYZEx
        )
}

macro_rules! euler_tests {
    ($module:ident, $scalar:ty, $quat:ident, $v3:ident) => {
        mod $module {
            use super::*;

            const TOL: $scalar = 1e-4;
            const PI: $scalar = <$scalar as Scalar>::PI;

            fn same_rotation(a: $quat, b: $quat) -> bool {
                a.angle_between(b).value() <= TOL
            }

            fn rad(value: $scalar) -> Radians<$scalar> {
                Radians::new(value)
            }

            /// A deterministic pseudo random number in `[0, 1)`.
            fn next(state: &mut u64) -> $scalar {
                *state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((*state >> 40) as $scalar) / ((1u64 << 24) as $scalar)
            }

            fn random_rotation(state: &mut u64) -> $quat {
                let axis =
                    $v3::new(next(state) - 0.5, next(state) - 0.5, next(state) - 0.5).normalize();
                $quat::from_axis_angle(axis, rad(next(state) * 2.0 * PI))
            }

            #[test]
            fn from_euler_intrinsic_is_the_product_in_order() {
                let (a, b, c) = (rad(0.3), rad(-0.7), rad(1.1));
                let expected = $quat::from_rotation_x(a)
                    * $quat::from_rotation_y(b)
                    * $quat::from_rotation_z(c);
                assert!(same_rotation(
                    $quat::from_euler(EulerRot::XYZ, a, b, c),
                    expected
                ));
                let expected = $quat::from_rotation_z(a)
                    * $quat::from_rotation_x(b)
                    * $quat::from_rotation_z(c);
                assert!(same_rotation(
                    $quat::from_euler(EulerRot::ZXZ, a, b, c),
                    expected
                ));
            }

            #[test]
            fn intrinsic_applies_the_last_angle_to_a_vector_first() {
                let (a, b, c) = (rad(0.4), rad(0.9), rad(-0.5));
                let q = $quat::from_euler(EulerRot::XYZ, a, b, c);
                let v = $v3::new(0.3, -1.2, 0.8);
                let step = $quat::from_rotation_x(a)
                    * ($quat::from_rotation_y(b) * ($quat::from_rotation_z(c) * v));
                assert!((q * v - step).length() < TOL);
            }

            #[test]
            fn extrinsic_applies_the_first_angle_to_a_vector_first() {
                let (a, b, c) = (rad(0.4), rad(0.9), rad(-0.5));
                let q = $quat::from_euler(EulerRot::XYZEx, a, b, c);
                let v = $v3::new(0.3, -1.2, 0.8);
                let step = $quat::from_rotation_z(c)
                    * ($quat::from_rotation_y(b) * ($quat::from_rotation_x(a) * v));
                assert!((q * v - step).length() < TOL);
            }

            #[test]
            fn extrinsic_is_the_reversed_intrinsic_order() {
                let (a, b, c) = (rad(0.4), rad(0.9), rad(-0.5));
                let pairs = [
                    (EulerRot::XYZEx, EulerRot::ZYX),
                    (EulerRot::YXZEx, EulerRot::ZXY),
                    (EulerRot::ZXZEx, EulerRot::ZXZ),
                    (EulerRot::XYXEx, EulerRot::XYX),
                ];
                for (extrinsic, intrinsic) in pairs {
                    assert!(
                        same_rotation(
                            $quat::from_euler(extrinsic, a, b, c),
                            $quat::from_euler(intrinsic, c, b, a)
                        ),
                        "{extrinsic:?}"
                    );
                }
            }

            #[test]
            fn degrees_are_accepted() {
                let q = $quat::from_euler(
                    EulerRot::YXZ,
                    Degrees::new(90.0),
                    Degrees::new(0.0),
                    Degrees::new(0.0),
                );
                assert!(same_rotation(q, $quat::from_rotation_y(Degrees::new(90.0))));
            }

            #[test]
            fn single_axis_angles_come_back() {
                let q = $quat::from_rotation_z(rad(PI / 2.0));
                let (a, b, c) = q.to_euler(EulerRot::ZYX);
                assert!((a.value() - PI / 2.0).abs() < TOL);
                assert!(b.value().abs() < TOL && c.value().abs() < TOL);
            }

            #[test]
            fn angles_round_trip_in_the_unique_range() {
                // First and last angle anywhere in (-PI, PI), the middle one
                // strictly inside its range: the angles come back unchanged.
                for order in ALL {
                    let proper = is_proper(order);
                    let middles: [$scalar; 3] = if proper {
                        [0.3, 1.2, 2.9]
                    } else {
                        [-1.4, 0.2, 1.4]
                    };
                    for a in [-3.0, -1.0, 0.5, 2.8] {
                        for b in middles {
                            for c in [-2.5, -0.2, 1.0, 3.1] {
                                let q = $quat::from_euler(order, rad(a), rad(b), rad(c));
                                let (ra, rb, rc) = q.to_euler(order);
                                assert!(
                                    (ra.value() - a).abs() < 1e-3
                                        && (rb.value() - b).abs() < 1e-3
                                        && (rc.value() - c).abs() < 1e-3,
                                    "{order:?} ({a}, {b}, {c}) came back as ({}, {}, {})",
                                    ra.value(),
                                    rb.value(),
                                    rc.value()
                                );
                            }
                        }
                    }
                }
            }

            #[test]
            fn random_rotations_round_trip_for_every_order() {
                let mut state = 0x1234_5678_9abc_def0;
                for _ in 0..200 {
                    let q = random_rotation(&mut state);
                    for order in ALL {
                        let (a, b, c) = q.to_euler(order);
                        let back = $quat::from_euler(order, a, b, c);
                        assert!(same_rotation(q, back), "{order:?} {q:?}");
                    }
                }
            }

            #[test]
            fn middle_angle_stays_in_its_range() {
                let mut state = 42;
                for _ in 0..200 {
                    let q = random_rotation(&mut state);
                    for order in ALL {
                        let (_, b, _) = q.to_euler(order);
                        if is_proper(order) {
                            assert!(b.value() >= -TOL && b.value() <= PI + TOL, "{order:?}");
                        } else {
                            assert!(b.value().abs() <= PI / 2.0 + TOL, "{order:?}");
                        }
                    }
                }
            }

            #[test]
            fn gimbal_lock_round_trips_and_zeroes_the_first_angle() {
                let tait_bryan_lock = [PI / 2.0, -PI / 2.0];
                for order in TAIT_BRYAN {
                    for b in tait_bryan_lock {
                        let q = $quat::from_euler(order, rad(0.7), rad(b), rad(-0.2));
                        let (a, rb, c) = q.to_euler(order);
                        assert!(a.value().abs() < TOL, "{order:?} {b}");
                        assert!(rb.value().is_finite() && c.value().is_finite());
                        assert!(
                            same_rotation(q, $quat::from_euler(order, a, rb, c)),
                            "{order:?} {b}"
                        );
                    }
                }
                for order in PROPER {
                    for b in [0.0, PI] {
                        let q = $quat::from_euler(order, rad(0.7), rad(b), rad(-0.2));
                        let (a, rb, c) = q.to_euler(order);
                        assert!(a.value().abs() < TOL, "{order:?} {b}");
                        assert!(
                            same_rotation(q, $quat::from_euler(order, a, rb, c)),
                            "{order:?} {b}"
                        );
                    }
                }
            }

            #[test]
            fn identity_has_zero_angles() {
                for order in ALL {
                    let (a, b, c) = $quat::IDENTITY.to_euler(order);
                    assert!(
                        a.value().abs() < TOL && b.value().abs() < TOL && c.value().abs() < TOL
                    );
                }
            }

            #[test]
            fn to_euler_ignores_the_length() {
                let q = $quat::from_euler(EulerRot::YXZ, rad(0.4), rad(0.3), rad(-0.8));
                let (a, b, c) = (q * 1.0001).to_euler(EulerRot::YXZ);
                assert!((a.value() - 0.4).abs() < TOL);
                assert!((b.value() - 0.3).abs() < TOL);
                assert!((c.value() + 0.8).abs() < TOL);
            }

            #[test]
            fn euler_quat_is_a_unit_quaternion() {
                for order in ALL {
                    assert!(
                        $quat::from_euler(order, rad(0.4), rad(1.3), rad(-2.0)).is_normalized()
                    );
                }
            }
        }
    };
}

euler_tests!(quat_f32, f32, Quat, Vec3);
euler_tests!(quat_f64, f64, DQuat, DVec3);
