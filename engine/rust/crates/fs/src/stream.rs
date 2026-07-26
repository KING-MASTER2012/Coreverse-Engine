use std::io::{Read, Seek};

/// Marker trait combining `Read + Seek`, so [`crate::FileSystem::open_read`]
/// can hand back a single trait object instead of forcing every
/// implementation to choose between the two. Blanket-implemented for
/// anything that already implements both (e.g. `std::fs::File`,
/// `std::io::Cursor<Vec<u8>>`).
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}
