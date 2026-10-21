//! Where each platform keeps what.
//!
//! The functions here are pure: everything they read from the system comes in
//! through [`Sources`]. They are compiled on every platform, so the layout of
//! Windows and macOS is unit-tested on Linux too.

use std::ffi::OsString;
use std::path::PathBuf;

use camino::Utf8PathBuf;
use ccore::platform::Platform;

use super::{DirKind, ORGANIZATION, utf8};
use crate::error::{PlatformError, Result};

/// A Windows known folder (`FOLDERID_*`).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum KnownFolder {
    /// `FOLDERID_RoamingAppData`.
    RoamingAppData,
    /// `FOLDERID_LocalAppData`.
    LocalAppData,
}

/// What the layout functions may ask the system.
pub(crate) struct Sources<'a> {
    pub(crate) env: &'a dyn Fn(&str) -> Option<OsString>,
    pub(crate) home: &'a dyn Fn() -> Result<Utf8PathBuf>,
    pub(crate) temp: &'a dyn Fn() -> Result<Utf8PathBuf>,
    pub(crate) executable_dir: &'a dyn Fn() -> Result<Utf8PathBuf>,
    pub(crate) known_folder: &'a dyn Fn(KnownFolder) -> Result<Utf8PathBuf>,
}

/// The path of `kind` for `app` on `platform`.
pub(crate) fn resolve(
    platform: Platform,
    kind: DirKind,
    app: &str,
    sources: &Sources<'_>,
) -> Result<Utf8PathBuf> {
    match platform {
        Platform::Windows => windows(kind, app, sources),
        Platform::MacOS => apple(kind, app, sources),
        Platform::Linux => linux(kind, app, sources),
        _ => Err(PlatformError::unsupported("standard directories")),
    }
}

fn org_app(base: Utf8PathBuf, org: &str, app: &str) -> Utf8PathBuf {
    base.join(org).join(app)
}

fn windows(kind: DirKind, app: &str, s: &Sources<'_>) -> Result<Utf8PathBuf> {
    let local = |sub: &str| -> Result<Utf8PathBuf> {
        Ok(org_app(
            (s.known_folder)(KnownFolder::LocalAppData)?,
            ORGANIZATION,
            app,
        )
            .join(sub))
    };
    match kind {
        DirKind::Home => (s.home)(),
        DirKind::ExecutableDir => (s.executable_dir)(),
        // Settings follow the user between machines; the rest stays local.
        DirKind::Config => Ok(org_app(
            (s.known_folder)(KnownFolder::RoamingAppData)?,
            ORGANIZATION,
            app,
        )
            .join("config")),
        DirKind::Data => local("data"),
        DirKind::Cache => local("cache"),
        DirKind::Log => local("log"),
        DirKind::Temp => Ok(org_app((s.temp)()?, ORGANIZATION, app)),
    }
}

fn apple(kind: DirKind, app: &str, s: &Sources<'_>) -> Result<Utf8PathBuf> {
    let library =
        |name: &str| -> Result<Utf8PathBuf> { Ok((s.home)()?.join("Library").join(name)) };
    match kind {
        DirKind::Home => (s.home)(),
        DirKind::ExecutableDir => (s.executable_dir)(),
        DirKind::Config => {
            Ok(org_app(library("Application Support")?, ORGANIZATION, app).join("config"))
        }
        DirKind::Data => {
            Ok(org_app(library("Application Support")?, ORGANIZATION, app).join("data"))
        }
        DirKind::Cache => Ok(org_app(library("Caches")?, ORGANIZATION, app)),
        DirKind::Log => Ok(org_app(library("Logs")?, ORGANIZATION, app)),
        DirKind::Temp => Ok(org_app((s.temp)()?, ORGANIZATION, app)),
    }
}

fn linux(kind: DirKind, app: &str, s: &Sources<'_>) -> Result<Utf8PathBuf> {
    // The directory name is lowercase on Linux, derived from the one constant.
    let org = ORGANIZATION.to_lowercase();
    match kind {
        DirKind::Home => (s.home)(),
        DirKind::ExecutableDir => (s.executable_dir)(),
        DirKind::Config => Ok(org_app(xdg(s, "XDG_CONFIG_HOME", &[".config"])?, &org, app)),
        DirKind::Data => Ok(org_app(
            xdg(s, "XDG_DATA_HOME", &[".local", "share"])?,
            &org,
            app,
        )),
        DirKind::Cache => Ok(org_app(xdg(s, "XDG_CACHE_HOME", &[".cache"])?, &org, app)),
        DirKind::Log => Ok(org_app(
            xdg(s, "XDG_STATE_HOME", &[".local", "state"])?,
            &org,
            app,
        )),
        DirKind::Temp => {
            let base = match absolute_env(s, "XDG_RUNTIME_DIR")? {
                Some(runtime) => runtime,
                None => (s.temp)()?,
            };
            Ok(org_app(base, &org, app))
        }
    }
}

/// The value of `var` if it is set to an absolute path. The XDG specification
/// says relative values are invalid and must be ignored; an unset or empty
/// variable counts as not set. A value that is absolute but not UTF-8 is an
/// error rather than being skipped.
fn absolute_env(s: &Sources<'_>, var: &str) -> Result<Option<Utf8PathBuf>> {
    match (s.env)(var) {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                utf8(path).map(Some)
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// `$var`, or `~/<default>` when it is not usable.
fn xdg(s: &Sources<'_>, var: &str, default: &[&str]) -> Result<Utf8PathBuf> {
    if let Some(path) = absolute_env(s, var)? {
        return Ok(path);
    }
    let mut path = (s.home)()?;
    path.extend(default);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// An absolute path on whatever host runs the tests.
    fn abs(name: &str) -> Utf8PathBuf {
        let mut root = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("UTF-8 temp dir");
        root.push("layout-test");
        root.push(name);
        root
    }

    struct Fake {
        env: HashMap<&'static str, OsString>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                env: HashMap::new(),
            }
        }

        fn with(mut self, var: &'static str, value: impl Into<OsString>) -> Self {
            self.env.insert(var, value.into());
            self
        }

        fn run(&self, platform: Platform, kind: DirKind, app: &str) -> Result<Utf8PathBuf> {
            let env = |name: &str| self.env.get(name).cloned();
            let home = || Ok(abs("home"));
            let temp = || Ok(abs("tmp"));
            let exe = || Ok(abs("bin"));
            let known = |folder: KnownFolder| match folder {
                KnownFolder::RoamingAppData => Ok(abs("roaming")),
                KnownFolder::LocalAppData => Ok(abs("local")),
            };
            resolve(
                platform,
                kind,
                app,
                &Sources {
                    env: &env,
                    home: &home,
                    temp: &temp,
                    executable_dir: &exe,
                    known_folder: &known,
                },
            )
        }
    }

    fn expect(base: &str, parts: &[&str]) -> Utf8PathBuf {
        let mut path = abs(base);
        path.extend(parts);
        path
    }

    #[test]
    fn windows_layout() {
        let f = Fake::new();
        let run = |k| f.run(Platform::Windows, k, "game").unwrap();
        assert_eq!(
            run(DirKind::Config),
            expect("roaming", &["Coreverse", "game", "config"])
        );
        assert_eq!(
            run(DirKind::Data),
            expect("local", &["Coreverse", "game", "data"])
        );
        assert_eq!(
            run(DirKind::Cache),
            expect("local", &["Coreverse", "game", "cache"])
        );
        assert_eq!(
            run(DirKind::Log),
            expect("local", &["Coreverse", "game", "log"])
        );
        assert_eq!(run(DirKind::Temp), expect("tmp", &["Coreverse", "game"]));
        assert_eq!(run(DirKind::Home), abs("home"));
        assert_eq!(run(DirKind::ExecutableDir), abs("bin"));
    }

    #[test]
    fn macos_layout() {
        let f = Fake::new();
        let run = |k| f.run(Platform::MacOS, k, "game").unwrap();
        let lib = |name: &str, rest: &[&str]| {
            let mut p = expect("home", &["Library", name, "Coreverse", "game"]);
            p.extend(rest);
            p
        };
        assert_eq!(
            run(DirKind::Config),
            lib("Application Support", &["config"])
        );
        assert_eq!(run(DirKind::Data), lib("Application Support", &["data"]));
        assert_eq!(run(DirKind::Cache), lib("Caches", &[]));
        assert_eq!(run(DirKind::Log), lib("Logs", &[]));
        assert_eq!(run(DirKind::Temp), expect("tmp", &["Coreverse", "game"]));
    }

    #[test]
    fn linux_defaults_are_under_home_with_lowercase_name() {
        let f = Fake::new();
        let run = |k| f.run(Platform::Linux, k, "game").unwrap();
        assert_eq!(
            run(DirKind::Config),
            expect("home", &[".config", "coreverse", "game"])
        );
        assert_eq!(
            run(DirKind::Data),
            expect("home", &[".local", "share", "coreverse", "game"])
        );
        assert_eq!(
            run(DirKind::Cache),
            expect("home", &[".cache", "coreverse", "game"])
        );
        assert_eq!(
            run(DirKind::Log),
            expect("home", &[".local", "state", "coreverse", "game"])
        );
        assert_eq!(run(DirKind::Temp), expect("tmp", &["coreverse", "game"]));
    }

    #[test]
    fn linux_honours_absolute_xdg_variables() {
        let f = Fake::new()
            .with("XDG_CONFIG_HOME", abs("c").into_os_string())
            .with("XDG_DATA_HOME", abs("d").into_os_string())
            .with("XDG_CACHE_HOME", abs("k").into_os_string())
            .with("XDG_STATE_HOME", abs("s").into_os_string())
            .with("XDG_RUNTIME_DIR", abs("r").into_os_string());
        let run = |k| f.run(Platform::Linux, k, "game").unwrap();
        assert_eq!(run(DirKind::Config), expect("c", &["coreverse", "game"]));
        assert_eq!(run(DirKind::Data), expect("d", &["coreverse", "game"]));
        assert_eq!(run(DirKind::Cache), expect("k", &["coreverse", "game"]));
        assert_eq!(run(DirKind::Log), expect("s", &["coreverse", "game"]));
        assert_eq!(run(DirKind::Temp), expect("r", &["coreverse", "game"]));
    }

    #[test]
    fn linux_ignores_relative_and_empty_xdg_variables() {
        let f = Fake::new()
            .with("XDG_CONFIG_HOME", "relative/dir")
            .with("XDG_DATA_HOME", "")
            .with("XDG_RUNTIME_DIR", "also/relative");
        let run = |k| f.run(Platform::Linux, k, "game").unwrap();
        assert_eq!(
            run(DirKind::Config),
            expect("home", &[".config", "coreverse", "game"])
        );
        assert_eq!(
            run(DirKind::Data),
            expect("home", &[".local", "share", "coreverse", "game"])
        );
        assert_eq!(run(DirKind::Temp), expect("tmp", &["coreverse", "game"]));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_xdg_value_is_an_error() {
        use std::os::unix::ffi::OsStringExt;
        let mut bytes = b"/tmp/".to_vec();
        bytes.push(0xFF);
        let f = Fake::new().with("XDG_DATA_HOME", OsString::from_vec(bytes));
        let err = f
            .run(Platform::Linux, DirKind::Data, "game")
            .expect_err("invalid UTF-8 must not be skipped");
        assert!(matches!(err, PlatformError::NonUtf8Path(_)), "{err:?}");
        // Unrelated kinds are unaffected.
        assert!(f.run(Platform::Linux, DirKind::Config, "game").is_ok());
    }

    #[test]
    fn other_platforms_are_unsupported() {
        let f = Fake::new();
        for platform in [Platform::Android, Platform::IOS, Platform::Unknown] {
            for kind in [DirKind::Config, DirKind::Home, DirKind::ExecutableDir] {
                let err = f.run(platform, kind, "game").unwrap_err();
                assert!(
                    matches!(err, PlatformError::Unsupported { .. }),
                    "{platform:?} {kind:?}: {err:?}"
                );
            }
        }
    }

    #[test]
    fn failing_sources_propagate() {
        let env = |_: &str| None;
        let home = || Err(PlatformError::unusable("home_dir", "no home"));
        let temp = || Ok(abs("tmp"));
        let exe = || Ok(abs("bin"));
        let known = |_: KnownFolder| Err(PlatformError::unusable("SHGetKnownFolderPath", "boom"));
        let sources = Sources {
            env: &env,
            home: &home,
            temp: &temp,
            executable_dir: &exe,
            known_folder: &known,
        };
        for platform in [Platform::Linux, Platform::MacOS] {
            let err = resolve(platform, DirKind::Data, "g", &sources).unwrap_err();
            assert!(matches!(err, PlatformError::Os { .. }), "{err:?}");
        }
        let err = resolve(Platform::Windows, DirKind::Config, "g", &sources).unwrap_err();
        assert!(matches!(err, PlatformError::Os { .. }), "{err:?}");
        // Temp does not need the home directory on Linux.
        assert!(resolve(Platform::Linux, DirKind::Temp, "g", &sources).is_ok());
    }
}
