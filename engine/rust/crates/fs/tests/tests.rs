//! Test file of FS
use camino::Utf8Path;
use std::time::{SystemTime, UNIX_EPOCH};

use fs::{FileSystem, OsFileSystem};

fn temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let dir = std::env::temp_dir().join(format!("coreverse_test_{nanos}"));

    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn write_and_read_bytes() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("test.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    fs.write_bytes(path, b"Hello Coreverse").unwrap();

    let bytes = fs.read_bytes(path).unwrap();

    assert_eq!(bytes, b"Hello Coreverse");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_to_string() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("text.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    fs.write_bytes(path, b"Rust Engine").unwrap();

    let text = fs.read_to_string(path).unwrap();

    assert_eq!(text, "Rust Engine");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn exists() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("exists.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    assert!(!fs.exists(path));

    fs.write_bytes(path, b"abc").unwrap();

    assert!(fs.exists(path));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("meta.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    fs.write_bytes(path, b"12345").unwrap();

    let meta = fs.metadata(path).unwrap();

    assert_eq!(meta.len, 5);
    assert!(!meta.is_dir);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_dir() {
    let fs = OsFileSystem;

    let root = temp_dir();

    let file1_buf = root.join("a.txt");
    let file2_buf = root.join("b.txt");

    let file1 = Utf8Path::from_path(&file1_buf).unwrap();
    let file2 = Utf8Path::from_path(&file2_buf).unwrap();

    fs.write_bytes(file1, b"A").unwrap();
    fs.write_bytes(file2, b"B").unwrap();

    let root_utf8 = Utf8Path::from_path(root.as_path()).unwrap();

    let entries = fs.read_dir(root_utf8).unwrap();

    assert_eq!(entries.len(), 2);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn remove_file() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("delete.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    fs.write_bytes(path, b"delete").unwrap();

    assert!(fs.exists(path));

    fs.remove_file(path).unwrap();

    assert!(!fs.exists(path));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn create_and_remove_directory() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let dir = root.join("a/b/c");

    let path = Utf8Path::from_path(dir.as_path()).unwrap();

    fs.create_dir_all(path).unwrap();

    assert!(path.exists());

    fs.remove_dir_all(Utf8Path::from_path(root.as_path()).unwrap())
        .unwrap();

    assert!(!root.exists());
}

#[test]
fn mmap_read() {
    let fs = OsFileSystem;

    let root = temp_dir();
    let file = root.join("mapped.txt");

    let path = Utf8Path::from_path(file.as_path()).unwrap();

    fs.write_bytes(path, b"MemoryMapped").unwrap();

    let mmap = fs.mmap_read(path).unwrap();

    assert_eq!(&mmap[..], b"MemoryMapped");

    std::fs::remove_dir_all(root).unwrap();
}
