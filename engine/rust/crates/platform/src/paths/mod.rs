//! Standard directories for an application.
//!
//! [`AppDirs`] answers "where does this application keep its configuration,
//! data, cache and logs?" the way each operating system expects. Every path is
//! a [`Utf8PathBuf`]; a directory the OS reports that is not valid UTF-8 is an
//! error ([`PlatformError::NonUtf8Path`]), never a lossy conversion.
//!
//! Nothing is created implicitly: the query methods only compute paths, and
//! [`AppDirs::ensure`] creates a directory when you ask for it.
//!
//! # Layout
//!
//! `<org>` is [`ORGANIZATION`] (`Coreverse`; `coreverse` on Linux) and `<app>`
//! the name given to [`AppDirs::new`].
//!
//! | Kind   | Windows                              | macOS                                          | Linux                                      |
//! | :----- | :----------------------------------- | :--------------------------------------------- | :----------------------------------------- |
//! | config | `%APPDATA%\<org>\<app>\config`       | `~/Library/Application Support/<org>/<app>/config` | `$XDG_CONFIG_HOME/<org>/<app>`         |
//! | data   | `%LOCALAPPDATA%\<org>\<app>\data`    | `~/Library/Application Support/<org>/<app>/data`   | `$XDG_DATA_HOME/<org>/<app>`           |
//! | cache  | `%LOCALAPPDATA%\<org>\<app>\cache`   | `~/Library/Caches/<org>/<app>`                 | `$XDG_CACHE_HOME/<org>/<app>`              |
//! | log    | `%LOCALAPPDATA%\<org>\<app>\log`     | `~/Library/Logs/<org>/<app>`                   | `$XDG_STATE_HOME/<org>/<app>`              |
//! | temp   | `<temp dir>\<org>\<app>`             | `<temp dir>/<org>/<app>`                       | `$XDG_RUNTIME_DIR/<org>/<app>`, else `<temp dir>/<org>/<app>` |
//!
//! On Windows the roots come from `SHGetKnownFolderPath` (Roaming for config,
//! Local for the rest). On Linux an `XDG_*` variable is used when it is set to
//! an absolute path, as the XDG specification requires, and otherwise the
//! default under the home directory (`~/.config`, `~/.local/share`,
//! `~/.cache`, `~/.local/state`). `$XDG_RUNTIME_DIR` is preferred for `temp`
//! because it is private to the user, unlike `/tmp`, where another user could
//! have created `/tmp/coreverse` first.
//!
//! Android, iOS and unknown platforms report
//! [`PlatformError::Unsupported`] for every kind.

mod layout;

use camino::{Utf8Path, Utf8PathBuf};
use ccore::platform::Platform;

use crate::error::{PlatformError, Result};
pub(crate) use layout::KnownFolder;
use layout::Sources;

/// The organization directory shared by every Coreverse application. It is the
/// single source of the name: Linux uses its lowercase form.
pub const ORGANIZATION: &str = "Coreverse";

/// A standard directory.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum DirKind {
    /// User-editable settings. Roams with the user profile on Windows.
    Config,
    /// Persistent application data that is not configuration.
    Data,
    /// Data that can be recreated; may be deleted at any time.
    Cache,
    /// Log files.
    Log,
    /// Scratch files that need not survive a reboot.
    Temp,
    /// The user's home directory. Not specific to the application.
    Home,
    /// The directory of the running executable. Not specific to the
    /// application name.
    ExecutableDir,
}

/// The standard directories of one application.
///
/// ```
/// # use platform::paths::AppDirs;
/// let dirs = AppDirs::new("editor").unwrap();
/// assert_eq!(dirs.app_name(), "editor");
/// // Computing a path touches nothing on disk; `ensure` creates it.
/// let _config = dirs.config(); // Err(Unsupported) on mobile platforms
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct AppDirs {
    app: String,
}

impl AppDirs {
    /// Directories for the application called `app`.
    ///
    /// The name becomes a directory name, so it must be non-empty, must not
    /// be `.` or `..`, and must not contain path separators, control
    /// characters, any of `:*?"<>|`, or end in a space or a dot (Windows
    /// would silently strip those).
    ///
    /// # Errors
    ///
    /// [`PlatformError::Io`] with [`InvalidInput`](std::io::ErrorKind::InvalidInput)
    /// for an unusable name.
    pub fn new(app: impl Into<String>) -> Result<Self> {
        let app = app.into();
        validate_app_name(&app)?;
        Ok(Self { app })
    }

    /// The application name.
    #[must_use]
    pub fn app_name(&self) -> &str {
        &self.app
    }

    /// The path of `kind`, without creating it.
    ///
    /// # Errors
    ///
    /// * [`PlatformError::Unsupported`] on platforms without standard
    ///   directories (Android, iOS, unknown).
    /// * [`PlatformError::NonUtf8Path`] when the OS path is not UTF-8.
    /// * [`PlatformError::Os`] when the OS cannot say (no home directory, a
    ///   failing `SHGetKnownFolderPath`).
    pub fn dir(&self, kind: DirKind) -> Result<Utf8PathBuf> {
        layout::resolve(Platform::current(), kind, &self.app, &system_sources())
    }

    /// The path of `kind` after creating it, and any missing parents, with
    /// [`std::fs::create_dir_all`]. Succeeds when it already exists.
    ///
    /// [`DirKind::Home`] and [`DirKind::ExecutableDir`] belong to the system,
    /// not to the application, but are accepted for symmetry.
    ///
    /// # Errors
    ///
    /// Everything [`dir`](Self::dir) reports, and [`PlatformError::Io`] when
    /// the directory cannot be created.
    ///
    /// On Unix [`DirKind::Temp`] is created private to the user (mode
    /// `0700`) and an existing directory is only accepted if it is a real
    /// directory (not a symlink) owned by the current user and closed to
    /// everyone else; otherwise this fails instead of handing out a place
    /// another user could have prepared.
    pub fn ensure(&self, kind: DirKind) -> Result<Utf8PathBuf> {
        let path = self.dir(kind)?;
        if kind == DirKind::Temp {
            create_private(path)
        } else {
            create(path)
        }
    }

    /// [`DirKind::Config`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn config(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Config)
    }

    /// [`DirKind::Data`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn data(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Data)
    }

    /// [`DirKind::Cache`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn cache(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Cache)
    }

    /// [`DirKind::Log`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn log(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Log)
    }

    /// [`DirKind::Temp`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn temp(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Temp)
    }

    /// [`DirKind::Home`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn home(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::Home)
    }

    /// [`DirKind::ExecutableDir`].
    ///
    /// # Errors
    ///
    /// See [`dir`](Self::dir).
    pub fn executable_dir(&self) -> Result<Utf8PathBuf> {
        self.dir(DirKind::ExecutableDir)
    }
}

fn validate_app_name(app: &str) -> Result<()> {
    let reason = if app.is_empty() {
        Some("is empty")
    } else if app == "." || app == ".." {
        Some("is a relative directory name")
    } else if app.ends_with([' ', '.']) {
        Some("ends in a space or a dot")
    } else if is_windows_device_name(app) {
        Some("is a reserved Windows device name (CON, NUL, COM1, ...)")
    } else if app.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        Some("contains a path separator, a control character or one of :*?\"<>|")
    } else {
        None
    };
    match reason {
        None => Ok(()),
        Some(reason) => Err(PlatformError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("application name {app:?} {reason}"),
        ))),
    }
}

/// Whether `name` is one of the DOS device names Windows reserves in every
/// directory (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`),
/// with or without an extension and in any case: a directory called that
/// cannot be created or opened there. Rejected on every platform so an
/// application name is portable.
fn is_windows_device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            upper
                .strip_prefix(prefix)
                .is_some_and(|digit| matches!(digit.as_bytes(), [b'1'..=b'9']))
        })
}

fn create(path: Utf8PathBuf) -> Result<Utf8PathBuf> {
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Creates `path` for the current user alone and checks that it really is
/// theirs. Where there are no Unix permissions this is [`create`].
#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
fn create_private(path: Utf8PathBuf) -> Result<Utf8PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&path)?;

    // `symlink_metadata`: a symlink planted at this name must not pass.
    let meta = std::fs::symlink_metadata(&path)?;
    let unsafe_reason = if !meta.is_dir() {
        Some("is not a directory (or is a symlink)")
    } else if meta.uid() != crate::sys::effective_uid() {
        Some("is owned by another user")
    } else if meta.mode() & 0o077 != 0 {
        Some("is accessible to other users")
    } else {
        None
    };
    match unsafe_reason {
        None => Ok(path),
        Some(reason) => Err(PlatformError::unusable(
            "temp directory",
            format!("{path} {reason}; refusing to use it"),
        )),
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
fn create_private(path: Utf8PathBuf) -> Result<Utf8PathBuf> {
    create(path)
}

/// The id of the current user, on platforms that have one.
fn user_id() -> Option<u32> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))]
    {
        Some(crate::sys::effective_uid())
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        None
    }
}

pub(crate) fn utf8(path: std::path::PathBuf) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).map_err(PlatformError::NonUtf8Path)
}

fn home_dir() -> Result<Utf8PathBuf> {
    match std::env::home_dir() {
        Some(path) => utf8(path),
        None => Err(PlatformError::unusable(
            "home_dir",
            "the home directory is not set and could not be looked up",
        )),
    }
}

fn executable_dir() -> Result<Utf8PathBuf> {
    let exe = utf8(std::env::current_exe()?)?;
    match exe.parent() {
        Some(dir) => Ok(Utf8Path::to_path_buf(dir)),
        None => Err(PlatformError::unusable(
            "current_exe",
            format!("{exe} has no parent directory"),
        )),
    }
}

#[cfg(windows)]
fn known_folder(folder: KnownFolder) -> Result<Utf8PathBuf> {
    utf8(crate::sys::known_folder(folder)?)
}

#[cfg(not(windows))]
fn known_folder(_folder: KnownFolder) -> Result<Utf8PathBuf> {
    Err(PlatformError::unsupported("known folders"))
}

/// The real environment, for [`layout::resolve`].
fn system_sources() -> Sources<'static> {
    Sources {
        env: &|name| std::env::var_os(name),
        home: &home_dir,
        temp: &|| utf8(std::env::temp_dir()),
        executable_dir: &executable_dir,
        known_folder: &known_folder,
        user_id: &user_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{Feature, Support, support};

    fn desktop() -> bool {
        support(Feature::StandardDirs, Platform::current()) != Support::Unsupported
    }

    #[test]
    fn organization_is_the_single_source_of_the_name() {
        assert_eq!(ORGANIZATION, "Coreverse");
    }

    #[test]
    fn accepts_ordinary_names() {
        for name in ["editor", "My Game", "a.b", "tool-2", "ünïcode"] {
            assert_eq!(AppDirs::new(name).unwrap().app_name(), name);
        }
    }

    #[test]
    fn rejects_names_that_would_escape_or_break() {
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "..\\x",
            "c:x",
            "a*",
            "a?",
            "a\"",
            "a<",
            "a>",
            "a|",
            "trailing ",
            "trailing.",
            "nul\0",
            "tab\t",
        ] {
            let err = AppDirs::new(name).expect_err(name);
            match err {
                PlatformError::Io(io) => {
                    assert_eq!(io.kind(), std::io::ErrorKind::InvalidInput, "{name:?}");
                }
                other => panic!("{name:?}: unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn real_directories_end_in_the_app_name() {
        if !desktop() {
            return;
        }
        let dirs = AppDirs::new("paths-test-app").unwrap();
        for kind in [
            DirKind::Config,
            DirKind::Data,
            DirKind::Cache,
            DirKind::Log,
            DirKind::Temp,
        ] {
            let path = dirs.dir(kind).unwrap();
            assert!(path.is_absolute(), "{kind:?}: {path}");
            assert!(
                path.components().any(|c| c.as_str() == "paths-test-app"),
                "{kind:?}: {path}"
            );
        }
        assert!(dirs.home().unwrap().is_absolute());
        let exe_dir = dirs.executable_dir().unwrap();
        assert!(exe_dir.is_dir(), "{exe_dir}");
    }

    #[test]
    fn rejects_windows_device_names_on_every_platform() {
        for name in [
            "CON", "con", "Nul", "aux.txt", "PRN", "COM1", "lpt9", "com3.log",
        ] {
            assert!(AppDirs::new(name).is_err(), "{name:?} must be rejected");
        }
        // Lookalikes that are fine.
        for name in ["CONSOLE", "COM0", "COM10", "LPT", "com", "nulled", "my.con"] {
            assert!(AppDirs::new(name).is_ok(), "{name:?} must be accepted");
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_temp_is_private_and_refuses_foreign_or_open_directories() {
        use std::os::unix::fs::PermissionsExt;

        if !desktop() {
            return;
        }
        let app = format!("paths-private-test-{}", std::process::id());
        let dirs = AppDirs::new(app.as_str()).unwrap();
        let made = dirs.ensure(DirKind::Temp).unwrap();
        let mode = std::fs::metadata(&made).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}: others must have no access");

        // A directory someone left open is refused, not silently reused.
        std::fs::set_permissions(&made, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(dirs.ensure(DirKind::Temp).is_err());

        std::fs::remove_dir_all(&made).unwrap();
        // Clean up the per-user parent too if we created it empty.
        let _ = std::fs::remove_dir(made.parent().unwrap());
    }

    #[test]
    fn querying_creates_nothing_and_ensure_does() {
        if !desktop() {
            return;
        }
        let app = format!("paths-ensure-test-{}", std::process::id());
        let dirs = AppDirs::new(app.as_str()).unwrap();
        let temp = dirs.temp().unwrap();
        assert!(!temp.exists(), "query must not create {temp}");

        let made = dirs.ensure(DirKind::Temp).unwrap();
        assert_eq!(made, temp);
        assert!(made.is_dir());
        // Idempotent.
        assert_eq!(dirs.ensure(DirKind::Temp).unwrap(), temp);

        std::fs::remove_dir_all(&made).unwrap();
    }

    #[test]
    fn create_reports_io_errors() {
        let dir = tempfile::tempdir().unwrap();
        let file = Utf8PathBuf::from_path_buf(dir.path().join("file")).unwrap();
        std::fs::write(&file, b"x").unwrap();
        // A directory cannot be created below a regular file.
        let err = create(file.join("child")).unwrap_err();
        assert!(matches!(err, PlatformError::Io(_)), "{err:?}");
    }

    #[test]
    fn mobile_and_unknown_platforms_are_unsupported() {
        if desktop() {
            return;
        }
        let dirs = AppDirs::new("x").unwrap();
        assert!(matches!(
            dirs.config(),
            Err(PlatformError::Unsupported { .. })
        ));
    }
}
