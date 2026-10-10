//! Tests for the curves of Phase 2.7: Bézier, Hermite, Catmull-Rom,
//! B-splines and arc length.
//!
//! The curve families are instantiated once per float family (`Vec2`/`Vec3`
//! over `f32`, `DVec2`/`DVec3` over `f64`); the properties checked are
//! structural (end points, derivatives against finite differences, agreement
//! between equivalent forms) rather than tables of numbers.

use super::*;
use crate::{DVec2, DVec3, DVec4, Vec2, Vec3, Vec4};

/// The distance between two points of a curve.
fn dist<S: Scalar, P: CurvePoint<S>>(a: P, b: P) -> S {
    (a - b).length()
}

macro_rules! curve_tests {
    ($module:ident, $s:ty, $v2:ident, $v3:ident, $tol:expr) => {
        mod $module {
            use super::*;

            /// The tolerance for comparisons of values of order one.
            const TOL: $s = $tol;
            /// The step of the central finite differences.
            const H: $s = 0.01;

            /// Whether two points are within `tol` of each other.
            fn near<P: CurvePoint<$s>>(a: P, b: P, tol: $s) -> bool {
                dist(a, b) <= tol
            }

            /// The control points of a curve with some wiggle.
            fn pts() -> [$v3; 4] {
                [
                    $v3::new(0.0, 0.0, 0.0),
                    $v3::new(1.0, 2.0, -1.0),
                    $v3::new(3.0, 2.5, 1.0),
                    $v3::new(4.0, -1.0, 0.5),
                ]
            }

            /// Samples of the parameter, beyond `0..=1` on both sides.
            const TS: [$s; 9] = [-0.25, 0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0, 1.25];

            #[test]
            fn curve_point_length() {
                assert_eq!(CurvePoint::<$s>::length(-3.0 as $s), 3.0);
                assert_eq!(CurvePoint::<$s>::length($v2::new(3.0, 4.0)), 5.0);
                assert_eq!(CurvePoint::<$s>::length($v3::new(2.0, 3.0, 6.0)), 7.0);
            }

            #[test]
            fn quadratic_bezier_basics() {
                let (p0, p1, p2) = ($v2::new(0.0, 0.0), $v2::new(1.0, 2.0), $v2::new(2.0, 0.0));
                assert_eq!(quadratic_bezier(p0, p1, p2, 0.0), p0);
                assert_eq!(quadratic_bezier(p0, p1, p2, 1.0), p2);
                assert!(near(
                    quadratic_bezier(p0, p1, p2, 0.5),
                    $v2::new(1.0, 1.0),
                    TOL
                ));
                // Velocity at the ends is twice the first and last leg.
                assert!(near(
                    quadratic_bezier_derivative(p0, p1, p2, 0.0),
                    (p1 - p0) * 2.0,
                    TOL
                ));
                assert!(near(
                    quadratic_bezier_derivative(p0, p1, p2, 1.0),
                    (p2 - p1) * 2.0,
                    TOL
                ));
                assert!(near(
                    quadratic_bezier_second_derivative(p0, p1, p2),
                    $v2::new(0.0, -8.0),
                    TOL
                ));
            }

            #[test]
            fn cubic_bezier_basics() {
                let [p0, p1, p2, p3] = pts();
                assert_eq!(cubic_bezier(p0, p1, p2, p3, 0.0), p0);
                assert_eq!(cubic_bezier(p0, p1, p2, p3, 1.0), p3);
                assert!(near(
                    cubic_bezier_derivative(p0, p1, p2, p3, 0.0),
                    (p1 - p0) * 3.0,
                    TOL
                ));
                assert!(near(
                    cubic_bezier_derivative(p0, p1, p2, p3, 1.0),
                    (p3 - p2) * 3.0,
                    TOL
                ));
                let square = [
                    $v2::new(0.0, 0.0),
                    $v2::new(0.0, 1.0),
                    $v2::new(1.0, 1.0),
                    $v2::new(1.0, 0.0),
                ];
                assert_eq!(
                    cubic_bezier(square[0], square[1], square[2], square[3], 0.5),
                    $v2::new(0.5, 0.75)
                );
            }

            #[test]
            fn collinear_evenly_spaced_control_points_give_linear_motion() {
                for t in TS {
                    assert!((cubic_bezier(0.0, 1.0, 2.0, 3.0, t) - 3.0 * t).abs() <= TOL);
                    assert!((quadratic_bezier(0.0, 1.0, 2.0, t) - 2.0 * t).abs() <= TOL);
                }
            }

            #[test]
            fn bezier_derivatives_match_finite_differences() {
                let [p0, p1, p2, p3] = pts();
                for t in [0.1, 0.4, 0.5, 0.8] {
                    let numeric = (cubic_bezier(p0, p1, p2, p3, t + H)
                        - cubic_bezier(p0, p1, p2, p3, t - H))
                        * (0.5 / H);
                    assert!(near(
                        cubic_bezier_derivative(p0, p1, p2, p3, t),
                        numeric,
                        0.05
                    ));
                    let numeric2 = (cubic_bezier_derivative(p0, p1, p2, p3, t + H)
                        - cubic_bezier_derivative(p0, p1, p2, p3, t - H))
                        * (0.5 / H);
                    assert!(near(
                        cubic_bezier_second_derivative(p0, p1, p2, p3, t),
                        numeric2,
                        0.05
                    ));
                    let numeric_q = (quadratic_bezier(p0, p1, p2, t + H)
                        - quadratic_bezier(p0, p1, p2, t - H))
                        * (0.5 / H);
                    assert!(near(
                        quadratic_bezier_derivative(p0, p1, p2, t),
                        numeric_q,
                        0.05
                    ));
                }
            }

            #[test]
            fn bezier_split_reproduces_the_curve() {
                let [p0, p1, p2, p3] = pts();
                let (left, right) = cubic_bezier_split(p0, p1, p2, p3, 0.3);
                assert_eq!(left[3], right[0]);
                assert!(near(left[3], cubic_bezier(p0, p1, p2, p3, 0.3), TOL));
                assert_eq!((left[0], right[3]), (p0, p3));
                for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let l = cubic_bezier(left[0], left[1], left[2], left[3], s);
                    let r = cubic_bezier(right[0], right[1], right[2], right[3], s);
                    assert!(near(l, cubic_bezier(p0, p1, p2, p3, 0.3 * s), TOL));
                    assert!(near(r, cubic_bezier(p0, p1, p2, p3, 0.3 + 0.7 * s), TOL));
                }
                let (ql, qr) = quadratic_bezier_split(p0, p1, p2, 0.6);
                assert_eq!(ql[2], qr[0]);
                for s in [0.0, 0.5, 1.0] {
                    let l = quadratic_bezier(ql[0], ql[1], ql[2], s);
                    let r = quadratic_bezier(qr[0], qr[1], qr[2], s);
                    assert!(near(l, quadratic_bezier(p0, p1, p2, 0.6 * s), TOL));
                    assert!(near(r, quadratic_bezier(p0, p1, p2, 0.6 + 0.4 * s), TOL));
                }
            }

            #[test]
            fn bezier_structs_match_the_functions() {
                let [p0, p1, p2, p3] = pts();
                let cubic = CubicBezier::new(p0, p1, p2, p3);
                let quad = QuadraticBezier::new(p0, p1, p2);
                assert_eq!(cubic.to_array(), [p0, p1, p2, p3]);
                assert_eq!(quad.to_array(), [p0, p1, p2]);
                for t in TS {
                    assert_eq!(cubic.point(t), cubic_bezier(p0, p1, p2, p3, t));
                    assert_eq!(
                        cubic.derivative(t),
                        cubic_bezier_derivative(p0, p1, p2, p3, t)
                    );
                    assert_eq!(
                        cubic.second_derivative(t),
                        cubic_bezier_second_derivative(p0, p1, p2, p3, t)
                    );
                    assert_eq!(quad.point(t), quadratic_bezier(p0, p1, p2, t));
                    assert_eq!(
                        quad.derivative(t),
                        quadratic_bezier_derivative(p0, p1, p2, t)
                    );
                }
                assert_eq!(
                    quad.second_derivative::<$s>(),
                    quadratic_bezier_second_derivative(p0, p1, p2)
                );
                let (a, b) = cubic.split(0.4);
                assert_eq!(a.p3, b.p0);
                let (a, b) = quad.split(0.4);
                assert_eq!(a.p2, b.p0);
            }

            #[test]
            fn hermite_basics() {
                let (p0, m0, p1, m1) = (
                    $v3::new(0.0, 1.0, 2.0),
                    $v3::new(1.0, 0.0, -1.0),
                    $v3::new(3.0, 3.0, 0.0),
                    $v3::new(0.0, 2.0, 1.0),
                );
                assert_eq!(hermite(p0, m0, p1, m1, 0.0), p0);
                assert_eq!(hermite(p0, m0, p1, m1, 1.0), p1);
                assert!(near(hermite_derivative(p0, m0, p1, m1, 0.0), m0, TOL));
                assert!(near(hermite_derivative(p0, m0, p1, m1, 1.0), m1, TOL));
                // Zero end velocities: the smoothstep curve.
                for t in TS {
                    let h: $s = hermite(0.0, 0.0, 1.0, 0.0, t);
                    assert!((h - t * t * (3.0 - 2.0 * t)).abs() <= TOL);
                }
                // A line at constant speed has equal end velocities.
                let line = hermite(0.0, 5.0, 5.0, 5.0, 0.3 as $s);
                assert!((line - 1.5).abs() <= TOL);
            }

            #[test]
            fn hermite_derivative_matches_finite_differences() {
                let (p0, m0, p1, m1) = (
                    $v3::new(0.0, 1.0, 2.0),
                    $v3::new(1.0, 0.0, -1.0),
                    $v3::new(3.0, 3.0, 0.0),
                    $v3::new(0.0, 2.0, 1.0),
                );
                for t in [0.1, 0.4, 0.8] {
                    let numeric = (hermite(p0, m0, p1, m1, t + H) - hermite(p0, m0, p1, m1, t - H))
                        * (0.5 / H);
                    assert!(near(hermite_derivative(p0, m0, p1, m1, t), numeric, 0.05));
                }
            }

            #[test]
            fn hermite_and_bezier_describe_the_same_curve() {
                let (p0, m0, p1, m1) = (
                    $v3::new(0.0, 1.0, 2.0),
                    $v3::new(1.0, 0.0, -1.0),
                    $v3::new(3.0, 3.0, 0.0),
                    $v3::new(0.0, 2.0, 1.0),
                );
                let [b0, b1, b2, b3] = hermite_to_bezier(p0, m0, p1, m1);
                assert_eq!((b0, b3), (p0, p1));
                for t in TS {
                    assert!(near(
                        cubic_bezier(b0, b1, b2, b3, t),
                        hermite(p0, m0, p1, m1, t),
                        TOL
                    ));
                }
                let [q0, n0, q1, n1] = bezier_to_hermite(b0, b1, b2, b3);
                assert!(near(q0, p0, TOL) && near(q1, p1, TOL));
                assert!(near(n0, m0, TOL) && near(n1, m1, TOL));
                let curve = CubicBezier::from_hermite(p0, m0, p1, m1);
                assert_eq!(curve.to_array(), [b0, b1, b2, b3]);
                let back = curve.to_hermite();
                assert!(near(back[1], m0, TOL));
            }

            #[test]
            fn catmull_rom_passes_through_its_middle_points() {
                let [p0, p1, p2, p3] = pts();
                assert_eq!(catmull_rom(p0, p1, p2, p3, 0.0), p1);
                assert_eq!(catmull_rom(p0, p1, p2, p3, 1.0), p2);
                assert!(near(
                    catmull_rom_derivative(p0, p1, p2, p3, 0.0),
                    (p2 - p0) * 0.5,
                    TOL
                ));
                assert!(near(
                    catmull_rom_derivative(p0, p1, p2, p3, 1.0),
                    (p3 - p1) * 0.5,
                    TOL
                ));
                for t in TS {
                    let h: $s = catmull_rom(0.0, 1.0, 2.0, 3.0, t);
                    assert!((h - (1.0 + t)).abs() <= TOL);
                }
            }

            #[test]
            fn catmull_rom_alpha_zero_is_the_uniform_spline() {
                let [p0, p1, p2, p3] = pts();
                for t in TS {
                    assert!(near(
                        catmull_rom_alpha(p0, p1, p2, p3, 0.0, t),
                        catmull_rom(p0, p1, p2, p3, t),
                        10.0 * TOL
                    ));
                }
            }

            #[test]
            fn catmull_rom_alpha_passes_through_its_middle_points() {
                let [p0, p1, p2, p3] = pts();
                for alpha in [0.0, 0.5, 1.0] {
                    assert_eq!(catmull_rom_alpha(p0, p1, p2, p3, alpha, 0.0), p1);
                    assert_eq!(catmull_rom_alpha(p0, p1, p2, p3, alpha, 1.0), p2);
                    // And it is a continuous curve in between.
                    let mut previous = p1;
                    for i in 1..=50 {
                        let t = i as $s / 50.0;
                        let point = catmull_rom_alpha(p0, p1, p2, p3, alpha, t);
                        assert!(dist(point, previous) < 0.5, "{alpha} {t}");
                        previous = point;
                    }
                }
            }

            #[test]
            fn centripetal_catmull_rom_stays_close_to_uneven_waypoints() {
                // A short leg followed by a very long one makes the uniform
                // spline overshoot; the centripetal one hugs the segment.
                let p = [
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 0.0),
                    $v2::new(1.1, 0.5),
                    $v2::new(10.0, 0.5),
                ];
                // How far a curve point is from the straight line through the
                // middle segment's end points.
                let chord = p[2] - p[1];
                let off_line = |q: $v2| {
                    let d = q - p[1];
                    (chord.x * d.y - chord.y * d.x).abs() / chord.length()
                };
                let mut worst_uniform: $s = 0.0;
                let mut worst_centripetal: $s = 0.0;
                for i in 0..=100 {
                    let t = i as $s / 100.0;
                    let u = catmull_rom_alpha(p[0], p[1], p[2], p[3], 0.0, t);
                    let c = catmull_rom_alpha(p[0], p[1], p[2], p[3], 0.5, t);
                    worst_uniform = worst_uniform.max(off_line(u));
                    worst_centripetal = worst_centripetal.max(off_line(c));
                }
                assert!(worst_centripetal < worst_uniform);
            }

            #[test]
            fn catmull_rom_alpha_handles_coincident_points() {
                let a = $v2::new(1.0, 2.0);
                let b = $v2::new(3.0, 1.0);
                let cases = [
                    (a, a, a, a),
                    (a, a, b, b),
                    (a, b, b, a),
                    (a, a, a, b),
                    (b, a, a, a),
                ];
                for (p0, p1, p2, p3) in cases {
                    for alpha in [0.0, 0.5, 1.0] {
                        for t in TS {
                            let point = catmull_rom_alpha(p0, p1, p2, p3, alpha, t);
                            assert!(point.x.is_finite() && point.y.is_finite());
                        }
                    }
                }
                // Four equal points are a constant curve.
                let point = catmull_rom_alpha(a, a, a, a, 0.5, 0.4);
                assert_eq!(point, a);
            }

            #[test]
            fn catmull_rom_spline_visits_every_point() {
                let path = [
                    $v2::new(0.0, 0.0),
                    $v2::new(2.0, 1.0),
                    $v2::new(3.0, -1.0),
                    $v2::new(5.0, 2.0),
                    $v2::new(6.0, 0.0),
                ];
                for alpha in [0.0, 0.5, 1.0] {
                    for (i, p) in path.iter().enumerate() {
                        assert_eq!(catmull_rom_spline(&path, alpha, i as $s), Some(*p));
                    }
                    // No jumps between neighbouring parameters, even across the
                    // joints between segments.
                    let mut previous = path[0];
                    for step in 1..=400 {
                        let t = step as $s / 100.0;
                        let point = catmull_rom_spline(&path, alpha, t).unwrap();
                        assert!(dist(point, previous) < 0.1, "{alpha} {t}");
                        previous = point;
                    }
                }
            }

            #[test]
            fn catmull_rom_spline_is_smooth_across_joints() {
                let path = [
                    $v2::new(0.0, 0.0),
                    $v2::new(2.0, 1.0),
                    $v2::new(3.0, -1.0),
                    $v2::new(5.0, 2.0),
                ];
                for joint in [1.0, 2.0] {
                    let e: $s = 0.001;
                    let before = catmull_rom_spline(&path, 0.0, joint - e).unwrap();
                    let at = catmull_rom_spline(&path, 0.0, joint).unwrap();
                    let after = catmull_rom_spline(&path, 0.0, joint + e).unwrap();
                    // The one-sided velocities agree: the tangent is continuous.
                    assert!(near(
                        (at - before) * (1.0 / e),
                        (after - at) * (1.0 / e),
                        0.05
                    ));
                }
            }

            #[test]
            fn catmull_rom_spline_ends_head_towards_their_neighbours() {
                let path = [$v2::new(0.0, 0.0), $v2::new(2.0, 2.0)];
                // Two points: the mirrored ends make a straight line.
                for t in [0.0, 0.25, 0.5, 1.0] {
                    let point = catmull_rom_spline(&path, 0.0, t).unwrap();
                    assert!(near(point, $v2::new(2.0 * t, 2.0 * t), TOL));
                }
            }

            #[test]
            fn catmull_rom_spline_rejects_invalid_input() {
                let one = [$v2::new(0.0, 0.0)];
                let path = [$v2::new(0.0, 0.0), $v2::new(1.0, 1.0), $v2::new(2.0, 0.0)];
                let none: [$v2; 0] = [];
                assert_eq!(catmull_rom_spline(&none, 0.0, 0.0), None);
                assert_eq!(catmull_rom_spline(&one, 0.0, 0.0), None);
                assert_eq!(catmull_rom_spline(&path, 0.0, -0.01), None);
                assert_eq!(catmull_rom_spline(&path, 0.0, 2.01), None);
                assert_eq!(catmull_rom_spline(&path, 0.0, <$s>::NAN), None);
                assert_eq!(catmull_rom_spline(&path, 0.0, <$s>::INFINITY), None);
                assert!(catmull_rom_spline(&path, 0.0, 2.0).is_some());
            }

            #[test]
            fn cubic_b_spline_basics() {
                let [p0, p1, p2, p3] = pts();
                assert!(near(
                    cubic_b_spline(p0, p1, p2, p3, 0.0),
                    (p0 + p1 * 4.0 + p2) * (1.0 / 6.0),
                    TOL
                ));
                assert!(near(
                    cubic_b_spline(p0, p1, p2, p3, 1.0),
                    (p1 + p2 * 4.0 + p3) * (1.0 / 6.0),
                    TOL
                ));
                // The basis is a partition of unity: a constant stays constant,
                // a line stays a line.
                for t in TS {
                    let c: $s = cubic_b_spline(2.0, 2.0, 2.0, 2.0, t);
                    assert!((c - 2.0).abs() <= TOL);
                    let l: $s = cubic_b_spline(0.0, 1.0, 2.0, 3.0, t);
                    assert!((l - (1.0 + t)).abs() <= TOL);
                }
            }

            #[test]
            fn cubic_b_spline_derivative_matches_finite_differences() {
                let [p0, p1, p2, p3] = pts();
                for t in [0.1, 0.4, 0.8] {
                    let numeric = (cubic_b_spline(p0, p1, p2, p3, t + H)
                        - cubic_b_spline(p0, p1, p2, p3, t - H))
                        * (0.5 / H);
                    assert!(near(
                        cubic_b_spline_derivative(p0, p1, p2, p3, t),
                        numeric,
                        0.05
                    ));
                }
            }

            #[test]
            fn cubic_b_spline_chain_is_c2_continuous() {
                let p = [
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 2.0),
                    $v2::new(3.0, 2.5),
                    $v2::new(4.0, -1.0),
                    $v2::new(6.0, 0.5),
                ];
                // The end of segment (p0..p3) is the start of (p1..p4).
                assert!(near(
                    cubic_b_spline(p[0], p[1], p[2], p[3], 1.0),
                    cubic_b_spline(p[1], p[2], p[3], p[4], 0.0),
                    TOL
                ));
                assert!(near(
                    cubic_b_spline_derivative(p[0], p[1], p[2], p[3], 1.0),
                    cubic_b_spline_derivative(p[1], p[2], p[3], p[4], 0.0),
                    TOL
                ));
            }

            #[test]
            fn clamped_uniform_knots() {
                let mut knots = [9.0 as $s; 8];
                assert!(fill_clamped_uniform_knots(&mut knots, 5, 2));
                assert_eq!(knots, [0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0]);
                let mut knots = [9.0 as $s; 8];
                assert!(fill_clamped_uniform_knots(&mut knots, 4, 3));
                assert_eq!(knots, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);
                // Invalid combinations leave the slice alone.
                let mut knots = [9.0 as $s; 8];
                assert!(!fill_clamped_uniform_knots(&mut knots, 5, 0));
                assert!(!fill_clamped_uniform_knots(&mut knots, 3, 3));
                assert!(!fill_clamped_uniform_knots(&mut knots, 5, 3));
                assert!(!fill_clamped_uniform_knots(&mut knots, 6, 2));
                assert_eq!(knots, [9.0; 8]);
            }

            #[test]
            fn clamped_b_spline_with_degree_plus_one_points_is_a_bezier() {
                let [p0, p1, p2, p3] = pts();
                let mut knots = [0.0 as $s; 8];
                assert!(fill_clamped_uniform_knots(&mut knots, 4, 3));
                let cubic_points = [p0, p1, p2, p3];
                let cubic = BSpline::new(3, &knots, &cubic_points).unwrap();
                assert_eq!(cubic.domain(), (0.0, 1.0));
                let mut knots = [0.0 as $s; 6];
                assert!(fill_clamped_uniform_knots(&mut knots, 3, 2));
                let quad_points = [p0, p1, p2];
                let quad = BSpline::new(2, &knots, &quad_points).unwrap();
                for i in 0..=20 {
                    let t = i as $s / 20.0;
                    assert!(near(
                        cubic.point(t).unwrap(),
                        cubic_bezier(p0, p1, p2, p3, t),
                        TOL
                    ));
                    assert!(near(
                        quad.point(t).unwrap(),
                        quadratic_bezier(p0, p1, p2, t),
                        TOL
                    ));
                }
            }

            #[test]
            fn clamped_b_spline_interpolates_its_end_points() {
                let points = [
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 2.0),
                    $v2::new(3.0, 2.0),
                    $v2::new(4.0, 0.0),
                    $v2::new(6.0, 1.0),
                    $v2::new(7.0, 3.0),
                ];
                for degree in 1..=4 {
                    let mut knots = [0.0 as $s; 12];
                    let knots = &mut knots[..points.len() + degree + 1];
                    assert!(fill_clamped_uniform_knots(knots, points.len(), degree));
                    let spline = BSpline::new(degree, knots, &points).unwrap();
                    let (lo, hi) = spline.domain();
                    assert_eq!(spline.point(lo), Some(points[0]));
                    let end = spline.point(hi).unwrap();
                    assert!(near(end, points[5], TOL), "degree {degree}");
                    // Continuous in between.
                    let mut previous = points[0];
                    for step in 1..=200 {
                        let t = lo + (hi - lo) * (step as $s / 200.0);
                        let point = spline.point(t).unwrap();
                        assert!(dist(point, previous) < 0.3, "degree {degree} t {t}");
                        previous = point;
                    }
                }
            }

            #[test]
            fn degree_one_b_spline_is_the_control_polyline() {
                let points = [$v2::new(0.0, 0.0), $v2::new(2.0, 0.0), $v2::new(2.0, 4.0)];
                let mut knots = [0.0 as $s; 5];
                assert!(fill_clamped_uniform_knots(&mut knots, 3, 1));
                let spline = BSpline::new(1, &knots, &points).unwrap();
                assert!(near(spline.point(0.5).unwrap(), $v2::new(1.0, 0.0), TOL));
                assert!(near(spline.point(1.0).unwrap(), $v2::new(2.0, 0.0), TOL));
                assert!(near(spline.point(1.5).unwrap(), $v2::new(2.0, 2.0), TOL));
                assert!(near(spline.point(2.0).unwrap(), $v2::new(2.0, 4.0), TOL));
            }

            #[test]
            fn unclamped_uniform_b_spline_matches_the_cubic_segments() {
                let p = [
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 2.0),
                    $v2::new(3.0, 2.5),
                    $v2::new(4.0, -1.0),
                    $v2::new(6.0, 0.5),
                    $v2::new(7.0, 1.5),
                ];
                // Knots 0, 1, 2, ...: the domain is 3..=6 and the segment that
                // starts at `3 + i` is controlled by `p[i..i + 4]`.
                let knots: [$s; 10] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
                let spline = BSpline::new(3, &knots, &p).unwrap();
                assert_eq!(spline.domain(), (3.0, 6.0));
                for i in 0..3 {
                    for local in [0.0, 0.3, 0.6, 0.9] {
                        let expected = cubic_b_spline(p[i], p[i + 1], p[i + 2], p[i + 3], local);
                        let t = 3.0 + i as $s + local;
                        assert!(near(spline.point(t).unwrap(), expected, TOL), "{i} {local}");
                    }
                }
                // The end of the domain is the end of the last segment.
                let end = cubic_b_spline(p[2], p[3], p[4], p[5], 1.0);
                assert!(near(spline.point(6.0).unwrap(), end, TOL));
            }

            #[test]
            fn b_spline_rejects_invalid_input() {
                let points = [$v2::new(0.0, 0.0), $v2::new(1.0, 1.0), $v2::new(2.0, 0.0)];
                let good: [$s; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
                assert!(BSpline::new(2, &good, &points).is_some());
                // Degree 0, or not below the number of points.
                assert!(BSpline::new(0, &good[..4], &points).is_none());
                assert!(BSpline::new(3, &good[..7.min(good.len())], &points).is_none());
                // Wrong knot count.
                assert!(BSpline::new(2, &good[..5], &points).is_none());
                // Decreasing and NaN knots.
                let decreasing: [$s; 6] = [0.0, 0.0, 1.0, 0.5, 1.0, 1.0];
                assert!(BSpline::new(2, &decreasing, &points).is_none());
                let nan: [$s; 6] = [0.0, 0.0, <$s>::NAN, 1.0, 1.0, 1.0];
                assert!(BSpline::new(2, &nan, &points).is_none());
                // An empty domain.
                let flat: [$s; 6] = [0.0; 6];
                assert!(BSpline::new(2, &flat, &points).is_none());
                // Above the supported degree.
                let many = [$v2::ZERO; 20];
                let knots = [0.0 as $s; 20 + MAX_B_SPLINE_DEGREE + 2];
                assert!(BSpline::new(MAX_B_SPLINE_DEGREE + 1, &knots, &many).is_none());
            }

            #[test]
            fn b_spline_point_outside_the_domain() {
                let points = [$v2::new(0.0, 0.0), $v2::new(1.0, 1.0), $v2::new(2.0, 0.0)];
                let knots: [$s; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
                let spline = BSpline::new(2, &knots, &points).unwrap();
                assert_eq!(spline.degree(), 2);
                assert_eq!(spline.knots(), &knots);
                assert_eq!(spline.control_points(), &points);
                assert_eq!(spline.point(-0.1), None);
                assert_eq!(spline.point(1.1), None);
                assert_eq!(spline.point(<$s>::NAN), None);
                assert_eq!(spline.point_clamped(-5.0), points[0]);
                assert!(near(spline.point_clamped(5.0), points[2], TOL));
                assert_eq!(spline.point_clamped(<$s>::NAN), points[0]);
                assert_eq!(spline.point_clamped(0.5), spline.point(0.5).unwrap());
            }

            #[test]
            fn b_spline_with_repeated_interior_knots() {
                // A double interior knot lowers the continuity but must still
                // evaluate (no division by a zero-width span).
                let points = [
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 2.0),
                    $v2::new(2.0, -1.0),
                    $v2::new(3.0, 1.0),
                    $v2::new(4.0, 0.0),
                ];
                let knots: [$s; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 2.0];
                let spline = BSpline::new(2, &knots, &points).unwrap();
                for step in 0..=40 {
                    let point = spline.point(step as $s / 20.0).unwrap();
                    assert!(point.x.is_finite() && point.y.is_finite());
                }
                assert_eq!(spline.point(0.0), Some(points[0]));
                assert!(near(spline.point(2.0).unwrap(), points[4], TOL));
                // The curve passes through the control point at a double knot.
                assert!(near(spline.point(1.0).unwrap(), points[2], TOL));
            }

            #[test]
            fn arc_length_of_a_straight_line_is_its_distance() {
                // Monotone collinear control points: the length is the
                // distance, however unevenly they are spaced.
                let curve = CubicBezier::new(0.0 as $s, 0.0, 1.0, 3.0);
                assert!((curve.length::<$s>() - 3.0).abs() <= 10.0 * TOL);
                let curve = QuadraticBezier::new(
                    $v3::ZERO,
                    $v3::new(1.0, 0.0, 0.0),
                    $v3::new(4.0, 0.0, 0.0),
                );
                assert!((curve.length::<$s>() - 4.0).abs() <= 10.0 * TOL);
                assert!(
                    (curve.length_between::<$s>(0.0, 0.5) + curve.length_between::<$s>(0.5, 1.0)
                        - curve.length::<$s>())
                    .abs()
                        <= 10.0 * TOL
                );
                // The order of the bounds does not matter.
                assert_eq!(
                    curve.length_between::<$s>(0.8, 0.2),
                    curve.length_between::<$s>(0.2, 0.8)
                );
                assert_eq!(curve.length_between::<$s>(0.3, 0.3), 0.0);
            }

            #[test]
            fn polyline_length_converges_to_the_gauss_length() {
                let curve = CubicBezier::new(
                    $v3::new(0.0, 0.0, 0.0),
                    $v3::new(1.0, 2.0, -1.0),
                    $v3::new(3.0, 2.5, 1.0),
                    $v3::new(4.0, -1.0, 0.5),
                );
                let exact = curve.length::<$s>();
                let coarse = arc_length_polyline(|t| curve.point(t), 0.0, 1.0, 8);
                let fine = arc_length_polyline(|t| curve.point(t), 0.0, 1.0, 512);
                // Chords underestimate, and more chords are closer.
                assert!(coarse < fine && fine <= exact + 1e-4);
                assert!((exact - fine).abs() < 1e-3 * exact);
                // Zero segments count as one chord.
                let chord = arc_length_polyline(|t| curve.point(t), 0.0, 1.0, 0);
                assert!((chord - dist(curve.p0, curve.p3)).abs() <= TOL);
            }

            #[test]
            fn a_cubic_quarter_circle_is_close_to_pi_over_two() {
                let k: $s = 0.552_284_7;
                let curve = CubicBezier::new(
                    $v2::new(1.0, 0.0),
                    $v2::new(1.0, k),
                    $v2::new(k, 1.0),
                    $v2::new(0.0, 1.0),
                );
                let expected = <$s as Scalar>::PI / 2.0;
                assert!((curve.length::<$s>() - expected).abs() < 1e-3);
            }

            #[test]
            fn arc_length_table_maps_between_parameter_and_distance() {
                let curve = CubicBezier::new(
                    $v2::new(0.0, 0.0),
                    $v2::new(0.1, 0.0),
                    $v2::new(0.2, 0.0),
                    $v2::new(4.0, 0.0),
                );
                let table = curve.arc_length_table::<$s, 128>();
                assert_eq!(table.domain(), (0.0, 1.0));
                assert_eq!(table.lengths().len(), 128);
                assert_eq!(table.lengths()[0], 0.0);
                assert!((table.total_length() - 4.0).abs() < 1e-3);
                // Endpoints and clamping.
                assert_eq!(table.length_at(-1.0), 0.0);
                assert_eq!(table.length_at(0.0), 0.0);
                assert_eq!(table.length_at(1.0), table.total_length());
                assert_eq!(table.length_at(2.0), table.total_length());
                assert_eq!(table.t_at_length(-1.0), 0.0);
                assert_eq!(table.t_at_length(0.0), 0.0);
                assert_eq!(table.t_at_length(table.total_length()), 1.0);
                assert_eq!(table.t_at_length(table.total_length() * 2.0), 1.0);
                // `length_at` and `t_at_length` are inverses.
                for t in [0.05, 0.2, 0.5, 0.8, 0.95] {
                    let back = table.t_at_length(table.length_at(t));
                    assert!((back - t).abs() < 1e-3, "{t} -> {back}");
                }
                // Constant speed: equal distance steps are equal chords.
                let mut previous = curve.point(0.0 as $s);
                let step = table.total_length() / 20.0;
                for i in 1..=20 {
                    let t = table.t_at_length(step * i as $s);
                    let point = curve.point(t);
                    assert!((dist(point, previous) - step).abs() < 0.02, "{i}");
                    previous = point;
                }
                assert!(
                    (table.t_at_fraction(0.5) - table.t_at_length(table.total_length() * 0.5))
                        .abs()
                        <= TOL
                );
                assert_eq!(table.t_at_fraction(-1.0), 0.0);
                assert_eq!(table.t_at_fraction(2.0), 1.0);
            }

            #[test]
            fn arc_length_table_is_monotonic() {
                let curve = CubicBezier::new(
                    $v3::new(0.0, 0.0, 0.0),
                    $v3::new(1.0, 2.0, -1.0),
                    $v3::new(3.0, 2.5, 1.0),
                    $v3::new(4.0, -1.0, 0.5),
                );
                let by_derivative = curve.arc_length_table::<$s, 64>();
                let by_points = ArcLengthTable::<$s, 64>::new(|t| curve.point(t), 0.0, 1.0);
                assert!(by_derivative.lengths().windows(2).all(|w| w[0] <= w[1]));
                assert!(by_points.lengths().windows(2).all(|w| w[0] <= w[1]));
                // The chord table underestimates the exact one, slightly.
                assert!(by_points.total_length() <= by_derivative.total_length() + TOL);
                assert!(
                    (by_points.total_length() - by_derivative.total_length()).abs()
                        < 1e-2 * by_derivative.total_length()
                );
                assert!((by_derivative.total_length() - curve.length::<$s>()).abs() <= 10.0 * TOL);
            }

            #[test]
            fn arc_length_table_of_a_degenerate_curve() {
                let point = $v2::new(1.0, 1.0);
                let curve = CubicBezier::new(point, point, point, point);
                let table = curve.arc_length_table::<$s, 8>();
                assert_eq!(table.total_length(), 0.0);
                assert_eq!(table.t_at_length(0.0), 0.0);
                assert_eq!(table.t_at_length(5.0), 0.0);
                // A zero-length curve has no distance to follow; the fraction
                // falls back to the plain parameter.
                assert_eq!(table.t_at_fraction(0.25), 0.25);
                let tiny = ArcLengthTable::<$s, 2>::new(|t| $v2::new(t, 0.0), 0.0, 2.0);
                assert_eq!(tiny.total_length(), 2.0);
                assert!((tiny.t_at_length(1.0) - 1.0).abs() <= TOL);
                assert!((tiny.length_at(0.5) - 0.5).abs() <= TOL);
            }

            #[test]
            fn arc_length_table_with_a_domain_not_starting_at_zero() {
                // A circle quarter parametrised by angle 1..=2.
                let table =
                    ArcLengthTable::<$s, 256>::new(|a: $s| $v2::new(a.cos(), a.sin()), 1.0, 2.0);
                assert_eq!(table.domain(), (1.0, 2.0));
                assert!((table.total_length() - 1.0).abs() < 1e-3);
                assert!((table.t_at_length(0.5) - 1.5).abs() < 1e-3);
                assert_eq!(table.t_at_length(0.0), 1.0);
                assert_eq!(table.t_at_length(100.0), 2.0);
                assert_eq!(table.length_at(0.0), 0.0);
            }

            #[cfg(all(feature = "serde", feature = "std"))]
            #[test]
            fn bezier_structs_serialize() {
                let cubic = CubicBezier::new(
                    $v2::new(0.0, 0.0),
                    $v2::new(1.0, 2.0),
                    $v2::new(3.0, 2.0),
                    $v2::new(4.0, 0.0),
                );
                let json = serde_json::to_string(&cubic).unwrap();
                assert_eq!(
                    json,
                    r#"{"p0":[0.0,0.0],"p1":[1.0,2.0],"p2":[3.0,2.0],"p3":[4.0,0.0]}"#
                );
                assert_eq!(
                    serde_json::from_str::<CubicBezier<$v2>>(&json).unwrap(),
                    cubic
                );
                let quad = QuadraticBezier::new($v2::ZERO, $v2::ONE, $v2::new(2.0, 0.0));
                let json = serde_json::to_string(&quad).unwrap();
                assert_eq!(
                    serde_json::from_str::<QuadraticBezier<$v2>>(&json).unwrap(),
                    quad
                );
            }
        }
    };
}

curve_tests!(single, f32, Vec2, Vec3, 1e-4);
curve_tests!(double, f64, DVec2, DVec3, 1e-9);

#[test]
fn quadratic_bezier_arc_length_has_a_closed_form() {
    // (2t, 2t(1 - t)) is the parabola y = x - x^2 / 2 on 0..=2, whose length
    // is sqrt(2) + asinh(1).
    let curve = QuadraticBezier::new(
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(2.0, 0.0),
    );
    let expected = 2.295_587_149_392_638;
    assert!((curve.length::<f64>() - expected).abs() < 1e-9);
    let table = curve.arc_length_table::<f64, 64>();
    assert!((table.total_length() - expected).abs() < 1e-9);
}

#[test]
fn scalar_and_wide_vector_points_work() {
    let a = cubic_bezier(0.0_f64, 1.0, 2.0, 3.0, 0.5);
    assert!((a - 1.5).abs() < 1e-12);
    let b = cubic_bezier(Vec4::ZERO, Vec4::ONE, Vec4::ONE, Vec4::ONE * 2.0, 1.0);
    assert_eq!(b, Vec4::ONE * 2.0);
    let c = hermite(DVec4::ZERO, DVec4::ZERO, DVec4::ONE, DVec4::ZERO, 0.5);
    assert!((c - DVec4::ONE * 0.5).length() < 1e-12);
    let d = catmull_rom_spline(&[DVec3::ZERO, DVec3::ONE], 0.5, 0.5).unwrap();
    assert!((d - DVec3::ONE * 0.5).length() < 1e-12);
}

#[test]
fn gauss_integration_handles_a_non_polynomial_speed() {
    // The unit circle's speed is constant, so its length is exact.
    let speed = |t: f64| DVec2::new(-t.sin(), t.cos());
    let length = arc_length_gauss(speed, 0.0, core::f64::consts::TAU, 8);
    assert!((length - core::f64::consts::TAU).abs() < 1e-9);
    // More panels do not change a converged answer, and zero counts as one.
    let more = arc_length_gauss(speed, 0.0, core::f64::consts::TAU, 64);
    assert!((more - length).abs() < 1e-9);
    assert!(arc_length_gauss(speed, 0.0, 1.0, 0) > 0.0);
}
