use fs::FileSystem;

/// Testing fs
#[test]
fn test_write_read() {
    let fs = OsFileSystem::new();

    let path = Utf8Path::new("test.txt");

    fs.write_bytes(path, b"Hello").unwrap();

    let data = fs.read_bytes(path).unwrap();

    assert_eq!(data, b"Hello");
}
