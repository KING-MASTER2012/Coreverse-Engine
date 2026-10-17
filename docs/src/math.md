# Math

`engine/rust/crates/math` is the engine's math library: scalar/angle/easing helpers, integer and bit math, vectors,
matrices, quaternions, Euler angles, geometry primitives, curves, random numbers and noise. Backends: `std` (default)
and `no_std` through `libm`; the optional `serde` feature serialises vectors, matrices and quaternions as flat arrays.

## Conventions

* `f32` types are `#[repr(C)]` and tightly packed (`Vec3` is 12 bytes, `Mat4` 64).
* **Column-major** matrices with column vectors (GLSL's `mat4`); `a * b` applies `b` first.
* **Right-handed**, Y-up. Clip-space depth is **0..1** (`Mat4::perspective_rh`), with no Y flip: the renderer flips Y
  with a negative viewport height.
* Angles are radians (`Radians`, `Degrees` newtypes on the Rust side).

## The C++ mirror: `cv-math`

`engine/cpp/math` is a header-only library (`cv_math::Vec2/3/4`, `Quat`, `Mat4`, `lookAtRh`, `perspectiveRh`,
`inverse`, `slerp`, ...) with the same layout and conventions. It covers what the renderer and editor need today, not
the whole crate.

Equivalence is enforced by a **golden file**, `tests/math_golden/golden.txt`:

* the Rust test `engine/rust/crates/math/tests/cpp_golden.rs` recomputes it from the Rust types and compares;
* the C++ test `tests/cpp/math_layout_test.cpp` recomputes it from the C++ types and compares;
* `layout` lines (size, alignment, field offsets) must match exactly, value lines within `1e-5` relative.

After an intentional change: `UPDATE_GOLDEN=1 cargo test -p math --test cpp_golden`, then update the C++ side so
`ctest -R math_layout_test` passes again.

## Benchmarks

`cargo bench -p math` (criterion): vector ops, matrix multiply/inverse/projection, quaternion ops, AABB tests, noise.
Use `--no-default-features --features no_std` to measure the `libm` backend.
