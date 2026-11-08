use crate::id::GenerationId;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A strongly-typed, generation-checked reference to a `T` stored in some
/// external table (asset registry, ECS storage, resource pool, ...).
///
/// `Handle<T>` never carries a `T` itself and never carries a persistent
/// id: it is pure runtime plumbing (`index` + `generation`) with a phantom
/// type tag so `Handle<Texture>` and `Handle<Mesh>` can't be mixed up at
/// compile time even though both are, underneath, just a `GenerationId`.
///
/// For the on-disk identity of the thing a handle points at (e.g. "which
/// asset is this, across saves"), see [`crate::id::AssetId`] instead —
/// a typical asset system resolves `AssetId -> Handle<T>` once at load
/// time and uses the cheap handle for everything after that.
pub struct Handle<T> {
    id: GenerationId,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Function that creates new Handle<T>
    ///
    /// Anyone may construct a handle, so a `Handle` is **not** proof that
    /// anything is alive: the storage that owns the slots must compare the
    /// generation on every access and reject mismatches. A forged or stale
    /// handle is then just an invalid handle, never undefined behaviour.
    pub const fn new(index: u32, generation: u32) -> Self {
        Self {
            id: GenerationId::new(index, generation),
            _marker: PhantomData,
        }
    }
    /// Function get returns self from id
    pub const fn from_id(id: GenerationId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }
    /// Function that returns id
    pub const fn id(&self) -> GenerationId {
        self.id
    }
    /// unction that returns index
    pub const fn index(&self) -> u32 {
        self.id.index()
    }
    /// Function that generates u32 id
    pub const fn generation(&self) -> u32 {
        self.id.generation()
    }

    /// A non-owning view of this handle: same slot + generation. `ccore`
    /// doesn't implement ref-counting itself, so today this is just a
    /// typed alias — systems that add ref-counted handles on top can use
    /// the strong/weak split to distinguish "keeps this alive" from "just
    /// looks at this" at the type level.
    pub const fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

// Manual impls: a derive would require `T: Copy/Clone/Eq/...`, but a
// handle to T doesn't need T itself to have any of these properties -
// it never stores a T.
impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for Handle<T> {}
impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle").field("id", &self.id).finish()
    }
}
impl<T> fmt::Display for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({})", self.id)
    }
}

/// A non-owning counterpart to [`Handle<T>`]. See [`Handle::downgrade`].
pub struct WeakHandle<T> {
    id: GenerationId,
    _marker: PhantomData<fn() -> T>,
}

impl<T> WeakHandle<T> {
    /// Function that returns id
    pub const fn id(&self) -> GenerationId {
        self.id
    }
    /// Turns the weak handle back into a [`Handle`] if the slot it points at
    /// still holds the same generation.
    ///
    /// A weak handle by itself cannot know whether its target is alive - only
    /// the storage can - so the storage supplies that answer as `is_alive`
    /// (typically "the slot's current generation equals `id`"). Returns
    /// `None` when the target was freed or its slot recycled.
    pub fn upgrade(&self, is_alive: impl FnOnce(GenerationId) -> bool) -> Option<Handle<T>> {
        is_alive(self.id).then_some(Handle {
            id: self.id,
            _marker: PhantomData,
        })
    }
}

impl<T> Copy for WeakHandle<T> {}
impl<T> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialEq for WeakHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for WeakHandle<T> {}
impl<T> Hash for WeakHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<T> fmt::Debug for WeakHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakHandle").field("id", &self.id).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Texture;
    struct Mesh;

    #[test]
    fn handles_to_different_types_are_distinct_types() {
        let tex: Handle<Texture> = Handle::new(0, 0);
        let mesh: Handle<Mesh> = Handle::new(0, 0);
        // Same index/generation, but `tex` and `mesh` are not the same
        // Rust type - this is checked at compile time, not runtime.
        assert_eq!(tex.index(), mesh.index());
    }

    #[test]
    fn downgrade_upgrade_roundtrips() {
        let handle: Handle<Texture> = Handle::new(3, 2);
        let weak = handle.downgrade();
        assert_eq!(weak.upgrade(|_| true), Some(handle));
    }

    #[test]
    fn upgrade_fails_once_the_slot_was_recycled() {
        let handle: Handle<Texture> = Handle::new(3, 2);
        let weak = handle.downgrade();
        // The storage now holds generation 3 in slot 3.
        let current = GenerationId::new(3, 3);
        assert_eq!(weak.upgrade(|id| id == current), None);
        assert!(weak.upgrade(|id| id == handle.id()).is_some());
    }
}
