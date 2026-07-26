mod build_config;
mod error;
mod manifest;
mod platform;
mod templates;

pub use build_config::{BuildConfig, OptimizationLevel};
pub use error::ProjectGeneratorError;
pub use manifest::CProjectManifest;
pub use platform::Platform;

use camino::Utf8Path;
use std::fs;

/// Top-level directories created for every new project, in the exact
/// layout the engine expects to find at the project root.
const TOP_LEVEL_DIRS: &[&str] = &[
    "Assets", "Build", "Cache", "Config", "Logs", "Mods", "Packages", "Source", "Temp",
];

/// Directories that stay tracked by git even while empty (a `.gitkeep` is
/// dropped in each). `Cache/Logs/Temp` and `Build/*/dist` are deliberately
/// excluded - `.gitignore` already ignores them entirely.
const KEEP_TRACKED_DIRS: &[&str] = &["Assets", "Config", "Mods", "Packages", "Source"];

/// Options controlling how a new project is scaffolded. Construct with
/// [`ProjectOptions::new`], then chain the `with_*` setters for anything
/// beyond the defaults.
#[derive(Debug, Clone)]
pub struct ProjectOptions {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub engine_version: String,
    pub platforms: Vec<Platform>,
    pub overwrite: bool,
}

impl ProjectOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            author: None,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            platforms: Platform::ALL.to_vec(),
            overwrite: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_engine_version(mut self, version: impl Into<String>) -> Self {
        self.engine_version = version.into();
        self
    }

    pub fn with_platforms(mut self, platforms: Vec<Platform>) -> Self {
        self.platforms = platforms;
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    fn validate(&self) -> Result<(), ProjectGeneratorError> {
        if self.name.trim().is_empty() {
            return Err(ProjectGeneratorError::InvalidName(
                "project name cannot be empty".into(),
            ));
        }
        const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        if self.name.chars().any(|c| FORBIDDEN.contains(&c)) {
            return Err(ProjectGeneratorError::InvalidName(format!(
                "'{}' contains characters not allowed in a file name",
                self.name
            )));
        }
        Ok(())
    }
}

/// Scaffolds a brand new project inside `parent_dir`: the project itself
/// gets its own folder, `parent_dir.join(&options.name)` - full directory
/// layout, per-platform `Build.toml`, `.gitignore`, `README.md`, and the
/// `<Name>.cproject` manifest go inside that folder. Returns the resulting
/// project root path.
///
/// Only that project folder needs to be empty (or missing) - `parent_dir`
/// itself can be anything, including a directory full of unrelated files
/// (e.g. a drive root), since nothing is ever written directly into it.
/// If the project folder already exists and is non-empty, this fails
/// unless [`ProjectOptions::overwrite`] is set - existing files are never
/// deleted, only created/overwritten as needed.
pub fn generate(
    parent_dir: &Utf8Path,
    options: &ProjectOptions,
) -> Result<camino::Utf8PathBuf, ProjectGeneratorError> {
    options.validate()?;

    let project_root = parent_dir.join(&options.name);

    if project_root.exists() {
        let has_entries = fs::read_dir(&project_root)
            .map_err(|e| io_err(&project_root, e))?
            .next()
            .is_some();
        if has_entries && !options.overwrite {
            return Err(ProjectGeneratorError::DirectoryNotEmpty(project_root));
        }
    } else {
        fs::create_dir_all(&project_root).map_err(|e| io_err(&project_root, e))?;
    }

    for dir in TOP_LEVEL_DIRS {
        let path = project_root.join(dir);
        fs::create_dir_all(&path).map_err(|e| io_err(&path, e))?;
    }

    let build_root = project_root.join("Build");
    for platform in &options.platforms {
        let platform_dir = build_root.join(platform.dir_name());
        fs::create_dir_all(platform_dir.join("dist")).map_err(|e| io_err(&platform_dir, e))?;

        let config = BuildConfig::defaults_for(*platform);
        config.write(&platform_dir.join("Build.toml"))?;
    }

    for dir in KEEP_TRACKED_DIRS {
        let keep_path = project_root.join(dir).join(".gitkeep");
        fs::write(&keep_path, b"").map_err(|e| io_err(&keep_path, e))?;
    }

    let gitignore_path = project_root.join(".gitignore");
    fs::write(&gitignore_path, templates::GITIGNORE).map_err(|e| io_err(&gitignore_path, e))?;

    let readme_path = project_root.join("README.md");
    let readme = templates::render_readme(&options.name, &options.description, &options.engine_version);
    fs::write(&readme_path, readme).map_err(|e| io_err(&readme_path, e))?;

    let manifest = CProjectManifest::new(&options.name, &options.engine_version, options.author.clone());
    let manifest_path = project_root.join(format!("{}.cproject", options.name));
    manifest.write(&manifest_path)?;

    Ok(project_root)
}

fn io_err(path: &Utf8Path, source: std::io::Error) -> ProjectGeneratorError {
    ProjectGeneratorError::Io {
        path: path.to_path_buf(),
        source,
    }
}