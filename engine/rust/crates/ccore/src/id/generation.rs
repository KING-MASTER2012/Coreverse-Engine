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
    ///
    /// The generation wraps from `u32::MAX` back to `0`, after which a very
    /// old handle to the slot would look valid again. Allocators that want
    /// to rule that out should use [`GenerationId::checked_next_generation`]
    /// and retire the slot instead.
    #[must_use]
    pub const fn next_generation(&self) -> Self {
        Self {
            index: self.index,
            generation: self.generation.wrapping_add(1),
        }
    }
}

impl GenerationId {
    /// Like [`GenerationId::next_generation`], but `None` instead of wrapping
    /// when the generation counter is exhausted, so the slot can be retired
    /// rather than reused with a generation that old handles may still hold.
    #[must_use]
    pub const fn checked_next_generation(&self) -> Option<Self> {
        match self.generation.checked_add(1) {
            Some(generation) => Some(Self {
                index: self.index,
                generation,
            }),
            None => None,
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

    #[test]
    fn checked_next_generation_stops_instead_of_wrapping() {
        let id = GenerationId::new(4, u32::MAX - 1);
        let last = id.checked_next_generation().expect("one more generation");
        assert_eq!(last.generation(), u32::MAX);
        assert_eq!(last.checked_next_generation(), None);
        // The unchecked version still wraps, as documented.
        assert_eq!(last.next_generation().generation(), 0);
    }
}
