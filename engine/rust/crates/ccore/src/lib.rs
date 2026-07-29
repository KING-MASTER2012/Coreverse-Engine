//! `ccore` — the shared foundation crate for the Coreverse Engine.
//!
//! Every other Coreverse crate (logger, VFS, asset, ECS, renderer, ...) is
//! expected to depend on `ccore` for the handful of types that need a
//! single, canonical definition across the whole engine: errors, ids and
//! handles, versioning, platform detection, and a few small common traits
//! and value types.
//!
//! `ccore` deliberately does *not* contain: logging, a virtual filesystem,
//! an event system, ECS, rendering, asset management, reflection, a thread
//! pool, configuration, or a plugin manager. Those are each substantial
//! enough to be their own crate, depending on `ccore` rather than living
//! inside it.
//!
//! Named `ccore` rather than `core` specifically to avoid colliding with
//! Rust's own implicitly-linked `core` crate.

/// Exports build_info
pub mod build_info;
/// Exports error
pub mod error;
/// Exports handle
pub mod handle;
/// Exports id
pub mod id;
/// Exports platform
pub mod platform;
/// Exports prelude
pub mod prelude;
/// Exports traits
pub mod traits;
/// Exports types
pub mod types;
/// Exports version
pub mod version;

pub use build_info::BuildInfo;
pub use error::{CoreError, CoreResult};
pub use handle::{Handle, WeakHandle};
pub use version::Version;

/// Re-exports used internally by macros (e.g. [`define_persistent_id!`])
/// so consuming crates don't need their own direct `uuid` dependency, and
/// so it can't fall out of sync with whatever `uuid` version `ccore`
/// itself pins. Not part of the public API - do not use directly.
#[doc(hidden)]
pub mod __private {
    pub use uuid::Uuid;
}
