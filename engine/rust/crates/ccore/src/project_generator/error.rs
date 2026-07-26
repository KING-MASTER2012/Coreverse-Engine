use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectGeneratorError {
    #[error("project directory '{0}' already exists and is not empty")]
    DirectoryNotEmpty(Utf8PathBuf),

    #[error("invalid project name: {0}")]
    InvalidName(String),

    #[error("io error at '{path}': {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to (de)serialize project manifest: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to serialize build config: {0}")]
    Toml(#[from] toml::ser::Error),

    #[error("failed to parse build config: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("no .cproject file found in '{0}'")]
    ManifestNotFound(Utf8PathBuf),
}
