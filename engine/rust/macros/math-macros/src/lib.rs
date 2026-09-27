//! Proc-macros that generate `math`'s concrete vector and matrix type
//! families (e.g. `Vec2`/`Vec3`/`Vec4`/`DVec2`/... and `Mat2`/`Mat3`/`Mat4`/...)
//! from a single definition each, instead of hand-duplicating the same
//! fields, constructors and operator impls once per scalar type.
//!
//! Kept in `engine/rust/macros/` (not `engine/rust/crates/`) alongside
//! `vfs-macros`, matching this workspace's convention of keeping proc-macro
//! crates - which must always compile as their own separate crate - out of
//! the regular crate tree.
//!
//! Empty for now: the vector-generating macro lands in Phase 1.7, the
//! matrix-generating macro in Phase 1.9.
