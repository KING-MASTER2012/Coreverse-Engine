//! `aose` — **A**uto from **O**OP convert **S**ystem to **E**CS. Placeholder
//! crate, not yet implemented.
//!
//! AOSE is the engine's own macro system that converts object-oriented code
//! (types that own their data and behavior) into the data-oriented form of an
//! ECS: components plus systems, generated automatically instead of being
//! rewritten by hand.
//!
//! Nothing here is stable API yet; the design happens together with the `ecs`
//! crate.
//!
//! Its sibling is [`acse`](../acse/index.html), which connects the result to
//! the ECS world. If AOSE becomes a real procedural-macro crate, the macro part
//! belongs under `engine/rust/macros/` (next to `vfs-macros` and
//! `math-macros`), with this crate keeping the runtime support; for now both
//! crates stay in `crates/`.
