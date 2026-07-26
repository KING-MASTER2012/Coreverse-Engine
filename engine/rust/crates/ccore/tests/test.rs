use ccore::project_generator as generator;
use camino::Utf8Path;

#[test]
pub fn test_project(){
    let options = generator::ProjectOptions::new("MyProject").with_description("A space adventure game").with_author("Talha");
    generator::generate(Utf8Path::new("D:/"), &options).unwrap();
}
