use std::fmt::Debug;
use std::hash::Hash;
use uuid::Uuid;

/// Marker trait for anything that can act as an identifier within the
/// engine.
///
/// `Id` intentionally carries no methods: what makes a type an identifier
/// is its *shape* (copyable, comparable, hashable, orderable, debuggable),
/// not any particular accessor. Runtime-safety semantics
/// (`index`/`generation`) and persistence semantics (`uuid`) are different
/// concerns and live on the [`RuntimeId`] / [`PersistentId`] sub-traits
/// below — mixing those concerns into the base trait is what leads to
/// treating a runtime handle and a persistent asset id as if they were the
/// same kind of thing.
pub trait Id: Copy + Clone + Eq + Ord + Hash + Send + Sync + Debug + 'static {}

impl<T> Id for T where T: Copy + Clone + Eq + Ord + Hash + Send + Sync + Debug + 'static {}

/// An identifier whose validity is scoped to a single running process.
///
/// Runtime ids are cheap, index-based, and reused: an `index` slot can be
/// recycled once its `generation` is bumped, so a stale id still pointing
/// at the old generation can be detected instead of silently aliasing
/// whatever now occupies that slot. Use this for ECS entities, slot-map
/// keys, component/resource storage, and any other in-memory table lookup.
///
/// See [`crate::id::GenerationId`] and [`crate::handle::Handle`].
pub trait RuntimeId: Id {
    /// Function that returns index
    fn index(&self) -> u32;
    /// Function that returns generation
    fn generation(&self) -> u32;
}

/// An identifier meant to remain stable across process runs, disk saves,
/// and project reloads.
///
/// Persistent ids are backed by a UUID rather than a table slot: they
/// don't need a live registry to stay meaningful, which is what makes them
/// suitable for assets, scenes, plugins, and projects referenced from disk.
///
/// See [`crate::id::UuidId`] and [`crate::define_persistent_id!`].
pub trait PersistentId: Id {
    /// Function that returns uuid
    fn uuid(&self) -> Uuid;
}
