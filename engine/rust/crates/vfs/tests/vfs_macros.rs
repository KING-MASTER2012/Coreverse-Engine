use vfs_macros::{file, file_bytes, metadata, path};

#[test]
fn path_macro_resolves_assets() {
    let path = path!("assets://text.txt");

    assert_eq!(path, "assets/text.txt");
}

#[test]
fn file_macro_reads_text() {
    let text = file!("assets://text.txt");

    assert_eq!(text, "Hello, Coreverse!\n");
}

const PNG: &[u8] = file_bytes!("assets://image.png");
#[test]
fn file_bytes_macro_reads_binary() {
    assert_eq!(&PNG[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn metadata_macro_returns_valid_values() {
    let meta = metadata!("assets://text.txt");

    assert!(meta.size > 0);

    assert!(meta.modified_unix >= 0);

    // bool olduğu kontrol edilmiş olur.
    let _ = meta.readonly;
}

#[test]
fn path_can_be_used_with_std_fs() {
    let path = path!("assets://text.txt");

    let text = std::fs::read_to_string(path).unwrap();

    assert_eq!(text, "Hello, Coreverse!\n");
}
