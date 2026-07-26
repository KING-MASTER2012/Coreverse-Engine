use camino::Utf8Path;

use root::Root;

use crate::backend::VfsMetadata;
use crate::context::VfsContext;
use crate::error::VfsError;
use crate::path::VfsPath;

/// Matches the original `add_file(src, root_type)` signature: copies `src`
/// into `root_type`'s top level, keeping the source file name.
pub fn add_file(src: &Utf8Path, root_type: Root) -> Result<(), VfsError> {
    let file_name = src
        .file_name()
        .ok_or_else(|| VfsError::InvalidPath(format!("'{src}' has no file name")))?;
    let dest = VfsPath::new(root_type, file_name)?;
    add_file_to(src, &dest)
}

/// Same as [`add_file`], but lets you choose exactly where inside the root
/// it lands (e.g. a subfolder, or a renamed file).
pub fn add_file_to(src: &Utf8Path, dest: &VfsPath) -> Result<(), VfsError> {
    let data = std::fs::read(src).map_err(|e| VfsError::Io {
        root: dest.root(),
        path: src.to_path_buf(),
        source: e,
    })?;
    write_bytes(dest, &data)
}

pub fn read_to_string(path: &VfsPath) -> Result<String, VfsError> {
    VfsContext::global()?
        .backend(path.root())?
        .read_to_string(path.rel())
}

pub fn read_bytes(path: &VfsPath) -> Result<Vec<u8>, VfsError> {
    VfsContext::global()?
        .backend(path.root())?
        .read_bytes(path.rel())
}

pub fn write_bytes(path: &VfsPath, data: &[u8]) -> Result<(), VfsError> {
    VfsContext::global()?
        .backend(path.root())?
        .write_bytes(path.rel(), data)
}

pub fn exists(path: &VfsPath) -> Result<bool, VfsError> {
    Ok(VfsContext::global()?
        .backend(path.root())?
        .exists(path.rel()))
}

pub fn remove(path: &VfsPath) -> Result<(), VfsError> {
    VfsContext::global()?
        .backend(path.root())?
        .remove(path.rel())
}

pub fn list_dir(path: &VfsPath) -> Result<Vec<VfsPath>, VfsError> {
    let entries = VfsContext::global()?
        .backend(path.root())?
        .list_dir(path.rel())?;
    entries
        .into_iter()
        .map(|rel| VfsPath::new(path.root(), rel))
        .collect()
}

pub fn metadata(path: &VfsPath) -> Result<VfsMetadata, VfsError> {
    VfsContext::global()?
        .backend(path.root())?
        .metadata(path.rel())
}

/// Convenience for the common "give me a fresh scratch file under Temp"
/// pattern - guarantees a unique name within this process.
pub fn new_temp_path(name_hint: &str) -> Result<VfsPath, VfsError> {
    let unique = format!("{}_{}", std::process::id(), name_hint);
    VfsPath::new(Root::Temp, unique)
}
