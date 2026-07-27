use std::sync::Arc;

use camino::Utf8Path;
use fs::MemoryFileSystem;
use vfs::{
    FileSystem, FsMetadata, OsFileSystem, Root, RootDescriptor, VfsContext, VfsError, VfsMode,
    VfsPath,
};

#[test]
fn public_api_exports_are_accessible() {
    let _ = std::any::TypeId::of::<VfsContext>();
    let _ = std::any::TypeId::of::<VfsMode>();
    let _ = std::any::TypeId::of::<VfsError>();
    let _ = std::any::TypeId::of::<VfsPath>();
    let _ = std::any::TypeId::of::<Root>();
    let _ = std::any::TypeId::of::<RootDescriptor>();
    let _ = std::any::TypeId::of::<MemoryFileSystem>();
    let _ = std::any::TypeId::of::<OsFileSystem>();
    let _ = std::any::TypeId::of::<FsMetadata>();
}

#[test]
fn memory_filesystem_is_a_filesystem() {
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::default());
    let _ = fs;
}

#[test]
fn init_with_memory_filesystem() {
    let fs = Arc::new(MemoryFileSystem::default());

    let result = VfsContext::init_with_fs(Utf8Path::new("."), None, VfsMode::Development, fs);

    assert!(result.is_ok());

    let ctx = VfsContext::global().unwrap();
    let _ = ctx.fs();
}
