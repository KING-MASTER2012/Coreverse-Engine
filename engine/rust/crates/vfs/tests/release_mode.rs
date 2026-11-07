//! Release mode: several packed roots sharing one archive, directory
//! semantics of the packed/hybrid backends, and unaddressable entries.
//!
//! Its own test binary: `VfsContext::init_with_fs` succeeds once per
//! process (see `list_dir_and_metadata.rs`).

use std::sync::Arc;

use camino::Utf8Path;
use fs::{FileSystem, MemoryFileSystem};
use vfs::format::{self, PackedEntry};
use vfs::{Root, VfsContext, VfsError, VfsMode, VfsPath, file_manager};

fn entry(root: Root, rel: &str, data: &[u8]) -> PackedEntry {
    PackedEntry {
        root,
        rel: rel.into(),
        data: data.to_vec(),
    }
}

fn names(paths: &[VfsPath]) -> Vec<String> {
    paths.iter().map(|p| p.rel().as_str().to_string()).collect()
}

#[test]
fn release_mode_serves_packed_roots_from_one_shared_archive() {
    let memory = MemoryFileSystem::new();
    format::write_archive(
        &memory,
        &[
            entry(Root::Assets, "textures/a.png", b"A"),
            entry(Root::Assets, "textures/sub/b.png", b"B"),
            entry(Root::Assets, "textures2/c.png", b"C"),
            entry(Root::Assets, "top.txt", b"T"),
            entry(Root::Packages, "pkg/one.bin", b"P1"),
            entry(Root::Source, "main.cv", b"S"),
        ],
        Utf8Path::new("game.cvpk"),
    )
    .unwrap();
    // The exporter's temporary file must not be left behind.
    assert!(!memory.exists(Utf8Path::new("game.cvpk.tmp")));

    // Loose overlay content next to the archive, including a name that
    // cannot be a VFS path.
    memory.seed("./Assets/textures/override.png", b"O".to_vec());
    memory.seed("./Assets/textures/bad:name.png", b"X".to_vec());

    let fs: Arc<dyn FileSystem> = Arc::new(memory);
    VfsContext::init_with_fs(
        Utf8Path::new("."),
        Some(Utf8Path::new("game.cvpk")),
        VfsMode::Release,
        fs,
    )
    .expect("release init with a shared archive");

    // All three packed roots read from the one archive.
    let read = |root, rel: &str| file_manager::read_bytes(&VfsPath::new(root, rel).unwrap());
    assert_eq!(read(Root::Assets, "textures/a.png").unwrap(), b"A");
    assert_eq!(read(Root::Packages, "pkg/one.bin").unwrap(), b"P1");
    assert_eq!(read(Root::Source, "main.cv").unwrap(), b"S");

    // Direct children only, no string-prefix matches ("textures2"), the
    // overlay merged in, sorted, and the unaddressable "bad:name.png" left out.
    let textures = file_manager::list_dir(&VfsPath::new(Root::Assets, "textures").unwrap())
        .expect("listing a hybrid directory");
    assert_eq!(
        names(&textures),
        ["textures/a.png", "textures/override.png", "textures/sub"]
    );

    // The root lists its files and virtual directories.
    let root = file_manager::list_dir(&VfsPath::new(Root::Assets, "").unwrap()).unwrap();
    assert_eq!(names(&root), ["textures", "textures2", "top.txt"]);

    // Virtual directories exist and report as directories.
    let dir = VfsPath::new(Root::Assets, "textures/sub").unwrap();
    assert!(file_manager::exists(&dir).unwrap());
    assert!(file_manager::metadata(&dir).unwrap().is_dir);

    // A directory that exists nowhere is an error, not an empty listing.
    let missing = VfsPath::new(Root::Assets, "nope").unwrap();
    let err = file_manager::list_dir(&missing).unwrap_err();
    assert!(err.is_not_found(), "{err:?}");
    assert!(!file_manager::exists(&missing).unwrap());

    // Archive-only files cannot be removed; overlay files can.
    let packed_only = VfsPath::new(Root::Assets, "top.txt").unwrap();
    assert!(matches!(
        file_manager::remove(&packed_only),
        Err(VfsError::ReadOnlyBackend(_))
    ));
    file_manager::remove(&VfsPath::new(Root::Assets, "textures/override.png").unwrap()).unwrap();
}
