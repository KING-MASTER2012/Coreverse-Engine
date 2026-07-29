//! Identity in `ccore` is deliberately split into two families that solve
//! different problems instead of forcing one model on everything:
//!
//! - **Runtime ids** ([`RuntimeId`], [`GenerationId`], [`crate::handle::Handle`])
//!   are index + generation pairs. They exist for safe, cheap in-memory
//!   table lookups (ECS entities, slot maps, asset registries at runtime)
//!   and become meaningless the moment the process exits.
//! - **Persistent ids** ([`PersistentId`], [`UuidId`], and newtypes created
//!   via [`define_persistent_id!`]) are UUID-backed. They exist to name
//!   something stably across saves, reloads, and different machines
//!   (assets, scenes, plugins, projects).
//!
//! A `Handle<Texture>` and an `AssetId` both "identify a texture", but they
//! answer different questions: the handle says "which live slot in this
//! process' asset table", the id says "which asset, on disk, forever".
//! Keeping them as separate types means one can never be used by mistake
//! where the other is required.

/// Exports generation
pub mod generation;
/// Exports traits
pub mod traits;
/// Exports uuid_id
pub mod uuid_id;

pub use generation::GenerationId;
pub use traits::{Id, PersistentId, RuntimeId};
pub use uuid_id::UuidId;

crate::define_persistent_id!(
    /// Persistent identifier for an asset, stable across saves and reloads.
    AssetId
);
crate::define_persistent_id!(
    /// Persistent identifier for a scene.
    SceneId
);
crate::define_persistent_id!(
    /// Persistent identifier for a plugin.
    PluginId
);

crate::define_persistent_id!(
    /// Persistent identifier for a project.
    ProjectId
);
