use super::traits::RuntimeId;
use std::fmt;

/// A runtime-only identifier made of a slot `index` plus a `generation`
/// counter.
///
/// When a slot is freed and reused, its generation is incremented, so any
/// `GenerationId` still holding the old generation is recognizably stale
/// rather than silently aliasing the new occupant. This is the backing
/// type for [`crate::handle::Handle<T>`]; use it directly when you need a
/// runtime id but don't need `Handle`'s type-tagging (e.g. `EntityId` in an
/// ECS).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct GenerationId {
    index: u32,
    generation: u32,
}

impl GenerationId {
    /// Function that creates new GenerationId
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    /// Function that returns index
    pub const fn index(&self) -> u32 {
        self.index
    }
    /// Function that returns generation
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns a copy of this id bumped to the next generation, keeping the
    /// same slot index. Typically called by the owning storage when a slot
    /// is freed and about to be recycled.
    #[must_use]
    pub const fn next_generation(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation.wrapping_add(1),
        }
    }
}

impl RuntimeId for GenerationId {
    fn index(&self) -> u32 {
        self.index
    }

    fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for GenerationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.index, self.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_generation_keeps_index_bumps_generation() {
        let id = GenerationId::new(4, 0);
        let recycled = id.next_generation();
        assert_eq!(recycled.index(), 4);
        assert_eq!(recycled.generation(), 1);
        assert_ne!(id, recycled);
    }
}
