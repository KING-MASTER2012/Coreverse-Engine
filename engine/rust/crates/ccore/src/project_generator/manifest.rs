use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use time::OffsetDateTime;
use uuid::Uuid;

use super::error::ProjectGeneratorError;

const FORMAT_VERSION: u32 = 1;

/// Contents of the top-level `<ProjectName>.cproject` file - the project's
/// equivalent of Unreal's `.uproject`. Plain JSON on purpose: readable and
/// diff-friendly in version control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CProjectManifest {
    pub format_version: u32,
    pub id: Uuid,
    pub name: String,
    pub engine_version: String,
    pub author: Option<String>,
    pub created_at: String,
}

impl CProjectManifest {
    pub fn new(name: &str, engine_version: &str, author: Option<String>) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            id: Uuid::new_v4(),
            name: name.to_string(),
            engine_version: engine_version.to_string(),
            author,
            created_at: OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }

    pub fn write(&self, path: &Utf8Path) -> Result<(), ProjectGeneratorError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json).map_err(|e| ProjectGeneratorError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }

    /// Finds the single `*.cproject` file directly inside `project_root`
    /// and parses it. Used when opening an existing project rather than
    /// scaffolding a new one.
    pub fn find_and_read(project_root: &Utf8Path) -> Result<Self, ProjectGeneratorError> {
        let entry = fs::read_dir(project_root)
            .map_err(|e| ProjectGeneratorError::Io {
                path: project_root.to_path_buf(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .find(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "cproject")
                    .unwrap_or(false)
            })
            .ok_or_else(|| ProjectGeneratorError::ManifestNotFound(project_root.to_path_buf()))?;

        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|p| ProjectGeneratorError::InvalidName(format!("non-utf8 path: {p:?}")))?;
        let content = fs::read_to_string(&path).map_err(|e| ProjectGeneratorError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(serde_json::from_str(&content)?)
    }
}
