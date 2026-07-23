use std::fs;
use camino::Utf8PathBuf;

pub enum ProjectType{
    Blank, FirstPerson, ThirdPerson, RPG, MMORPG, JRPG, Sandbox, Architecture, Automative, TwoD
}
pub fn create_project(name: String, path: Utf8PathBuf, proj_type: ProjectType){
    let mut path = path;
    path.push(&name);
    path.set_extension("cproject");
    fs::File::create(path).unwrap();

}
pub fn add_template(path: Utf8PathBuf, proj_type: ProjectType){
   
}
