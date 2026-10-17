//! Criterion benchmarks for the hot paths of `math`: vector/matrix/quaternion
//! arithmetic, transforms, bounding volumes and noise.
//!
//! Run with `cargo bench -p math` (add `--no-default-features --features
//! no_std` to measure the `libm` backend). Every input goes through
//! `black_box` so the optimizer cannot fold the work away.
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use math::noise::{fbm_3d, perlin_3d};
use math::{Aabb3, Mat4, Quat, Radians, Vec3};

fn vectors(c: &mut Criterion) {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(-2.0, 0.5, 4.0);
    c.bench_function("vec3/dot", |bench| {
        bench.iter(|| black_box(a).dot(black_box(b)))
    });
    c.bench_function("vec3/cross", |bench| {
        bench.iter(|| black_box(a).cross(black_box(b)))
    });
    c.bench_function("vec3/normalize", |bench| {
        bench.iter(|| black_box(a).normalize())
    });
}

fn matrices(c: &mut Criterion) {
    let q = Quat::from_axis_angle(
        Vec3::new(0.267_261_24, 0.534_522_5, 0.801_783_7),
        Radians::new(0.9),
    );
    let model = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 1.0, 0.5),
        q,
        Vec3::new(1.0, -2.0, 3.0),
    );
    let view = Mat4::look_at_rh(Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(Radians::new(1.047_197_6), 16.0 / 9.0, 0.1, 100.0);

    c.bench_function("mat4/mul", |bench| {
        bench.iter(|| black_box(proj) * black_box(view))
    });
    c.bench_function("mat4/mvp", |bench| {
        bench.iter(|| black_box(proj) * black_box(view) * black_box(model))
    });
    c.bench_function("mat4/inverse", |bench| {
        bench.iter(|| black_box(model).inverse())
    });
    c.bench_function("mat4/inverse_affine", |bench| {
        bench.iter(|| black_box(model).inverse_affine())
    });
    c.bench_function("mat4/transform_point3", |bench| {
        bench.iter(|| black_box(model).transform_point3(black_box(Vec3::ONE)))
    });
    c.bench_function("mat4/perspective_rh", |bench| {
        bench.iter(|| {
            Mat4::perspective_rh(
                black_box(Radians::new(1.047_197_6)),
                black_box(1.777),
                0.1,
                100.0,
            )
        })
    });
    c.bench_function("mat4/look_at_rh", |bench| {
        bench.iter(|| Mat4::look_at_rh(black_box(Vec3::new(3.0, 4.0, 5.0)), Vec3::ZERO, Vec3::Y))
    });
}

fn quaternions(c: &mut Criterion) {
    let q1 = Quat::from_axis_angle(
        Vec3::new(0.267_261_24, 0.534_522_5, 0.801_783_7),
        Radians::new(0.9),
    );
    let q2 = Quat::from_rotation_y(Radians::new(-1.1));
    c.bench_function("quat/mul", |bench| {
        bench.iter(|| black_box(q1).mul_quat(black_box(q2)))
    });
    c.bench_function("quat/slerp", |bench| {
        bench.iter(|| black_box(q1).slerp(black_box(q2), black_box(0.3)))
    });
    c.bench_function("quat/to_mat4", |bench| {
        bench.iter(|| black_box(q1).to_mat4())
    });
    c.bench_function("quat/rotate_vec3", |bench| {
        bench.iter(|| black_box(q1).mul_vec3(black_box(Vec3::ONE)))
    });
}

fn geometry(c: &mut Criterion) {
    let a = Aabb3::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    let b = Aabb3::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 2.0, 2.0));
    c.bench_function("aabb3/intersects", |bench| {
        bench.iter(|| black_box(a).intersects(black_box(b)))
    });
}

fn noise(c: &mut Criterion) {
    c.bench_function("noise/perlin_3d", |bench| {
        bench.iter(|| perlin_3d(black_box(1.3_f32), black_box(4.1), black_box(0.7), 5))
    });
    c.bench_function("noise/fbm_3d_4_octaves", |bench| {
        bench.iter(|| {
            fbm_3d(
                black_box(1.3_f32),
                black_box(4.1),
                black_box(0.7),
                4,
                2.0,
                0.5,
                |x, y, z, o| perlin_3d(x, y, z, 5 + o),
            )
        })
    });
}

criterion_group!(benches, vectors, matrices, quaternions, geometry, noise);
criterion_main!(benches);
