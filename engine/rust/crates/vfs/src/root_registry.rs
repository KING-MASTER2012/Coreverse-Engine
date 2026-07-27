use root::Root;

/// Compile-time declared behavior for a single [`Root`].
///
/// Any crate (engine core, an editor plugin, a mod-loader crate, ...) can
/// contribute its own descriptor with `inventory::submit!` at the bottom of
/// this file's pattern - [`crate::context::VfsContext::init`] simply
/// collects every descriptor that exists in the final binary at startup.
/// That's what makes this "cross-crate": no central list has to know about
/// every root ahead of time.
#[derive(Debug, Clone, Copy)]
pub struct RootDescriptor {
    /// Which [`Root`] this descriptor configures.
    pub root: Root,

    /// Path relative to the project root, used whenever this root is
    /// backed by real files on disk: always in `Development` mode, and
    /// always (as the overlay) when `packed == true` in `Release` mode too.
    ///
    /// Matches `project_generator`'s folder names exactly (`Assets`,
    /// `Build`, `Cache`, `Config`, `Logs`, `Mods`, `Packages`, `Source`,
    /// `Temp`) - if you rename a folder there, update it here too.
    pub dev_path: &'static str,

    /// If `true`, this root's contents are baked into the `.coreproject`
    /// archive at export time and served from it (read-only, memory-mapped)
    /// in `Release` mode - with the loose `dev_path` still available on top
    /// as a writable overlay for mod/patch overrides.
    ///
    /// If `false`, the root is *always* real files on disk regardless of
    /// mode. Use this for anything that changes after shipping: save data,
    /// player settings, logs, downloaded mods, temp scratch files, build
    /// artifacts.
    pub packed: bool,
}

inventory::collect!(RootDescriptor);

// Default descriptors for the engine's built-in `Root` variants.
//
// These are defaults, not law: if some other crate submits its own
// `RootDescriptor` for the same `Root`, `VfsContext::init` will end up
// using whichever one it inserts last while walking `inventory::iter`
// (insertion order there is not guaranteed across crates). If you need a
// hard override, prefer adjusting the entry here rather than fighting
// iteration order.
inventory::submit! { RootDescriptor { root: Root::Assets,   dev_path: "Assets",   packed: true  } }
inventory::submit! { RootDescriptor { root: Root::Packages, dev_path: "Packages", packed: true  } }
inventory::submit! { RootDescriptor { root: Root::Source,   dev_path: "Source",   packed: true  } }
inventory::submit! { RootDescriptor { root: Root::Build,    dev_path: "Build",    packed: false } }
inventory::submit! { RootDescriptor { root: Root::Cache,    dev_path: "Cache",    packed: false } }
// Config is loose on purpose, even in Release: things like graphics
// settings and key bindings live here and need to be writable at runtime.
inventory::submit! { RootDescriptor { root: Root::Config,   dev_path: "Config",   packed: false } }
inventory::submit! { RootDescriptor { root: Root::Logs,     dev_path: "Logs",     packed: false } }
inventory::submit! { RootDescriptor { root: Root::Mods,     dev_path: "Mods",     packed: false } }
inventory::submit! { RootDescriptor { root: Root::Temp,     dev_path: "Temp",     packed: false } }
