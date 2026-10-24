//! `acse` — **A**uto **C**onnecting **S**ystem for **E**CS. Placeholder crate,
//! not yet implemented.
//!
//! ACSE is the engine's own macro system for wiring user code into the
//! [`ecs`](../ecs/index.html) world automatically, so that connecting
//! components and systems to it does not have to be written by hand.
//!
//! Nothing here is stable API yet; the design happens together with the `ecs`
//! crate.
//!
//! Its sibling is [`aose`](../aose/index.html), which converts OOP-style code
//! into ECS. If ACSE becomes a real procedural-macro crate, the macro part
//! belongs under `engine/rust/macros/` (next to `vfs-macros` and
//! `math-macros`), with this crate keeping the runtime support; for now both
//! crates stay in `crates/`.
