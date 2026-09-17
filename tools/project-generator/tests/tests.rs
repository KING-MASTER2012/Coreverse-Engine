//! Project generator tested in this file.
use camino::Utf8Path;
use project_generator as generator;

/// Test function.
#[test]
fn test_project() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = Utf8Path::from_path(temp_dir.path()).unwrap();

    let options = generator::ProjectOptions::new("MyProject")
        .with_description("A space adventure game")
        .with_author("Alex");

    generator::generate(temp_path, &options).unwrap();
}
