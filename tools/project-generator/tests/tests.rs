//! Project generator tested in this file.
use camino::Utf8Path;
use project_generator as generator;

/// Test function.
#[test]
fn test_project() {
    let options = generator::ProjectOptions::new("MyProject")
        .with_description("A space adventure game")
        .with_author("Alex");
    generator::generate(Utf8Path::new("D:/"), &options).unwrap();
}
