/// Metadata about a single filesystem entry, as reported by a
/// [`crate::FileSystem`] implementation. Deliberately minimal and
/// independent from any higher-level (VFS/Asset) metadata type - this
/// layer knows nothing about roots, projects, or asset formats.
#[derive(Debug, Clone, Copy)]
pub struct FsMetadata {
    /// Metadata lenght
    pub len: u64,
    /// Data of is directory
    pub is_dir: bool,
}
