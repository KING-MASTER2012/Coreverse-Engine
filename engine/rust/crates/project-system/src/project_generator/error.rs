use camino::Utf8PathBuf;
use thiserror::Error;

/// The enum for the generator's errors
#[derive(Debug, Error)]
pub enum ProjectGeneratorError {
    /// Control of available of directory
    #[error("project directory '{0}' already exists and is not empty")]
    DirectoryNotEmpty(Utf8PathBuf),
    /// Control of available of the name
    #[error("invalid project name: {0}")]
    InvalidName(String),
    /// Error of IO works
    #[error("io error at '{path}': {source}")]
    Io {
        /// Current path
        path: Utf8PathBuf,
        /// Target path
        #[source]
        source: std::io::Error,
    },
    /// Error of serialize project manifest
    #[error("failed to (de)serialize project manifest: {0}")]
    Json(#[from] serde_json::Error),
    /// Error of serialize build config
    #[error("failed to serialize build config: {0}")]
    Toml(#[from] toml::ser::Error),
    /// Error of parsing build config
    #[error("failed to parse build config: {0}")]
    TomlDe(#[from] toml::de::Error),
    /// Error of no manifest
    #[error("no .coreproject file found in '{0}'")]
    ManifestNotFound(Utf8PathBuf),
}
