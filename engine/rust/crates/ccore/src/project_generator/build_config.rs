use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use std::fs;

use super::error::ProjectGeneratorError;
use super::platform::Platform;

/// Optimization level for project binary
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    /// For debuggings
    Debug,
    /// For real any release of the game
    Release,
    /// For deployment
    Dist,
}

/// Per-platform build configuration, serialized to
/// `Build/<Platform>/Build.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Platform like Windows, Linux or Android
    pub platform: Platform,
    /// Target Triple
    pub target_triple: String,
    /// Optimization
    pub optimization: OptimizationLevel,
    /// Output directory, relative to this `Build.toml`'s own folder.
    pub output_dir: String,
    /// Dist Name
    pub output_name: Option<String>,
}

impl BuildConfig {
    /// Default build
    pub fn defaults_for(platform: Platform) -> Self {
        Self {
            platform,
            target_triple: platform.default_target_triple().to_string(),
            optimization: OptimizationLevel::Debug,
            output_dir: "dist".to_string(),
            output_name: None,
        }
    }
    /// Write function
    pub fn write(&self, path: &Utf8Path) -> Result<(), ProjectGeneratorError> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string).map_err(|e| ProjectGeneratorError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }
    /// Read funcion
    pub fn read(path: &Utf8Path) -> Result<Self, ProjectGeneratorError> {
        let content = fs::read_to_string(path).map_err(|e| ProjectGeneratorError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(toml::from_str(&content)?)
    }
}
