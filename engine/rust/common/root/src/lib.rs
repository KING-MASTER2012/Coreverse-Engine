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
