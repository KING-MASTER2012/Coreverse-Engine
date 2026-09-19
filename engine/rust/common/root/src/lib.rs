//! Common Root Enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The Enum of Roots of Project's File
pub enum Root {
    /// Root of assets
    Assets,
    /// Root of build files and dists
    Build,
    /// Root of cache
    Cache,
    /// Root of configs
    Config,
    /// Root of logs
    Logs,
    /// Root of mods
    Mods,
    /// Root of packages
    Packages,
    /// Root of codes
    Source,
    /// Root of temporary files and folders
    Temp,
}

impl Root {
    /// Stable numeric id for this root, matching `vfs::format`'s
    /// on-disk `.coreproject` archive layout (`root_to_id`/`id_to_root`)
    /// exactly — this is what makes it safe to also use across an ABI
    /// boundary (see the `ffi` crate's `vfs` module). Do not renumber
    /// without also updating `vfs::format` and bumping the archive
    /// format version.
    pub fn as_u8(self) -> u8 {
        match self {
            Root::Assets => 0,
            Root::Build => 1,
            Root::Cache => 2,
            Root::Config => 3,
            Root::Logs => 4,
            Root::Mods => 5,
            Root::Packages => 6,
            Root::Source => 7,
            Root::Temp => 8,
        }
    }

    /// Inverse of [`Self::as_u8`]. Returns `None` for any byte outside
    /// `0..=8`.
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Root::Assets,
            1 => Root::Build,
            2 => Root::Cache,
            3 => Root::Config,
            4 => Root::Logs,
            5 => Root::Mods,
            6 => Root::Packages,
            7 => Root::Source,
            8 => Root::Temp,
            _ => return None,
        })
    }
}
