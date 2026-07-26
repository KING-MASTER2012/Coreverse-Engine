#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Root {
    Assets,
    Build,
    Cache,
    Config,
    Logs,
    Mods,
    Packages,
    Source,
    Temp
}
