//! Coreverse's core math library.
//!
//! Everything here is `f32`/`f64` scalar math, angles, interpolation and
//! easing curves, parametric curves (Bézier, Hermite, Catmull-Rom,
//! B-splines, arc length), deterministic random numbers and noise (PCG32,
//! SplitMix64, Perlin, simplex), GPU packing (half floats, unorm/snorm,
//! octahedral normals), integer/bit math, vectors, matrices, quaternions
//! and the geometric primitives built on them (boxes, rays, planes,
//! spheres, the view frustum, TRS transforms) - all `#![no_std]`-compatible
//! with no heap allocation, so this crate never needs `alloc` either. The optional
//! `serde` feature adds `Serialize`/`Deserialize` for all of them.
//!
//! # `std` vs `no_std`
//!
//! This crate builds two ways:
//! - **`std` (default)** - transcendental functions (`sin`, `cos`, `sqrt`,
//!   `pow`, ...) call straight through to the platform's native libm via
//!   `std`. Fastest option, used for every normal desktop build of the
//!   engine.
//! - **`no_std`** - built with `default-features = false, features =
//!   ["no_std"]`. The same functions are instead backed by the `libm`
//!   crate's pure-Rust, software implementations, and the crate itself
//!   compiles under `#![no_std]`. Slower than the `std` path (no hardware/
//!   platform libm), kept available for a future no-OS target rather than
//!   for day-to-day engine builds.
//!
//! The `no_std` attribute below is gated on the *absence* of the `std`
//! feature, not on the presence of `no_std`, specifically so that Cargo's
//! feature unification can't leave this crate compiled as `no_std` while
//! something else in the same build still expects `std` to be enabled: if
//! any crate in the dependency graph enables `std` (directly or via the
//! default), that wins. The `no_std` feature only controls which backend
//! the transcendental functions in `trig.rs` use; declare it explicitly if
//! you also need `#![no_std]` itself.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(any(feature = "std", feature = "no_std")))]
compile_error!(
    "`math` needs a float backend: keep the default `std` feature, or build with \
     `default-features = false, features = [\"no_std\"]` to use `libm`."
);

/// Calls the generic test function `$f::<T>()` once for each of the 12
/// standard integer types.
#[cfg(test)]
macro_rules! each_int {
    ($f:ident) => {
        $f::<i8>();
        $f::<i16>();
        $f::<i32>();
        $f::<i64>();
        $f::<i128>();
        $f::<isize>();
        $f::<u8>();
        $f::<u16>();
        $f::<u32>();
        $f::<u64>();
        $f::<u128>();
        $f::<usize>();
    };
}

pub mod angle;
/// Approximate floating-point equality (absolute, custom-epsilon and
/// relative). See [`approx::approx_eq`] and friends.
pub mod approx;
pub mod bits;
/// Ergonomic, non-generic `f32`/`f64` constants. See [`consts::f32`]/
/// [`consts::f64`].
pub mod consts;
pub mod curve;
pub mod ease;
pub mod geometry;
pub mod int_math;
pub mod integer;
/// Interpolation helpers: `lerp`, `inverse_lerp`, `remap`, `smoothstep`,
/// `step` and `delta`, generic over [`Scalar`] plus `const fn` variants
/// for `f32`/`f64`. See [`interp::lerp`] and friends.
pub mod interp;
pub mod matrix;
pub mod noise;
pub mod pack;
pub mod quat;
pub mod radix;
pub mod random;
/// The internal [`Scalar`] trait every generic function in this crate is
/// written against.
pub mod scalar;
#[cfg(feature = "serde")]
mod serde_impls;
#[cfg(test)]
mod test_support;
pub mod trig;
pub mod vector;

pub use angle::{Degrees, Radians};
pub use curve::{ArcLengthTable, BSpline, CubicBezier, CurvePoint, QuadraticBezier};
pub use geometry::{
    Aabb2, Aabb3, Containment, DAabb2, DAabb3, DFrustum, DPlane, DRay, DSphere, DTransform,
    Frustum, Plane, Ray, Sphere, Transform,
};
pub use integer::{Integer, Unsigned};
pub use matrix::{DMat2, DMat3, DMat4, Mat2, Mat3, Mat4};
pub use pack::F16;
pub use quat::{DQuat, EulerRot, Quat};
pub use random::{Pcg32, Rng, SplitMix64};
pub use scalar::Scalar;
pub use vector::{DVec2, DVec3, DVec4, IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
