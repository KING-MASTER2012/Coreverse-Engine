//! The running process, and asking the desktop to open things.
//!
//! * [`id`] and [`current_exe`] describe this process.
//! * [`open_url`] opens a web or mail link in the user's default application.
//! * [`reveal_in_file_manager`] shows a file or folder in Explorer, Finder or
//!   the Linux file manager.
//!
//! Opening and revealing exist on desktop platforms only (Windows through
//! `ShellExecuteW`, macOS through `open`, Linux through `xdg-open`). On
//! Android, iOS and unknown platforms they return
//! [`PlatformError::Unsupported`]. A launcher that is started successfully but
//! fails later cannot be noticed: `Ok(())` means "the request was handed to
//! the system".

mod launch;

use camino::{Utf8Path, Utf8PathBuf};
use ccore::platform::Platform;

use crate::error::{PlatformError, Result};
use crate::paths::utf8;

/// The operating-system id of this process.
#[must_use]
pub fn id() -> u32 {
    std::process::id()
}

/// The path of the running executable.
///
/// The path may be a symbolic link on some platforms, and on Linux and
/// Windows can be stale if the file was replaced while running.
///
/// # Errors
///
/// [`PlatformError::Io`] when the OS cannot tell, and
/// [`PlatformError::NonUtf8Path`] when the path is not UTF-8.
pub fn current_exe() -> Result<Utf8PathBuf> {
    utf8(std::env::current_exe()?)
}

fn desktop() -> bool {
    matches!(
        Platform::current(),
        Platform::Windows | Platform::MacOS | Platform::Linux
    )
}

/// Opens `url` in the default application: a browser for `http` and `https`,
/// the mail client for `mailto`.
///
/// Only those three schemes are accepted. The system launchers would happily
/// start a program or open a document given a file path or a scheme such as
/// `ms-msdt:`, which makes an unrestricted `open_url` dangerous for URLs that
/// come from data. The URL must also be free of whitespace and control
/// characters.
///
/// # Errors
///
/// * [`PlatformError::Unsupported`] on Android, iOS and unknown platforms.
/// * [`PlatformError::Io`] with
///   [`InvalidInput`](std::io::ErrorKind::InvalidInput) for a URL that is not
///   allowed.
/// * [`PlatformError::Io`] when the launcher cannot be started (no `xdg-open`
///   installed, for instance), [`PlatformError::Os`] when `ShellExecuteW`
///   refuses.
pub fn open_url(url: &str) -> Result<()> {
    if !desktop() {
        return Err(PlatformError::unsupported("opening URLs"));
    }
    launch::validate_url(url)?;
    open_url_impl(url)
}

/// Shows `path` in the file manager: selected in its folder on Windows and
/// macOS; on Linux the folder is opened (the containing folder of a file),
/// because there is no portable way to select an item.
///
/// A relative path is resolved against the current directory.
///
/// # Errors
///
/// * [`PlatformError::Unsupported`] on Android, iOS and unknown platforms.
/// * [`PlatformError::Io`] of kind [`NotFound`](std::io::ErrorKind::NotFound)
///   when `path` does not exist, and with the launcher's error when it cannot
///   be started.
/// * [`PlatformError::Os`] when `ShellExecuteW` refuses.
/// * [`PlatformError::NonUtf8Path`] when the resolved path is not UTF-8.
pub fn reveal_in_file_manager(path: &Utf8Path) -> Result<()> {
    if !desktop() {
        return Err(PlatformError::unsupported("revealing files"));
    }
    if !path.exists() {
        return Err(PlatformError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{path} does not exist"),
        )));
    }
    let absolute = utf8(std::path::absolute(path)?)?;
    reveal_impl(&absolute)
}

#[cfg(windows)]
fn open_url_impl(url: &str) -> Result<()> {
    crate::sys::shell_execute(url, None)
}

#[cfg(windows)]
fn reveal_impl(path: &Utf8Path) -> Result<()> {
    let params = launch::windows_select_params(path.as_str())?;
    crate::sys::shell_execute("explorer.exe", Some(&params))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn open_url_impl(url: &str) -> Result<()> {
    let launch = launch::open_url_launch(Platform::current(), url)
        .ok_or_else(|| PlatformError::unsupported("opening URLs"))?;
    run(&launch)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn reveal_impl(path: &Utf8Path) -> Result<()> {
    let launch = launch::reveal_launch(Platform::current(), path.as_std_path(), path.is_dir())
        .ok_or_else(|| PlatformError::unsupported("revealing files"))?;
    run(&launch)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn open_url_impl(_url: &str) -> Result<()> {
    Err(PlatformError::unsupported("opening URLs"))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn reveal_impl(_path: &Utf8Path) -> Result<()> {
    Err(PlatformError::unsupported("revealing files"))
}

/// Starts the launcher without waiting for it.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run(launch: &launch::Launch) -> Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new(launch.program)
        .args(&launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            PlatformError::Io(std::io::Error::new(
                err.kind(),
                format!("could not run {}: {err}", launch.program),
            ))
        })?;
    // The launcher usually exits within milliseconds but may stay for as long
    // as the program it started. Waiting here would block the caller, not
    // waiting would leave a zombie, so a small thread reaps it.
    let _ = std::thread::Builder::new()
        .name("platform-launch-reaper".to_owned())
        .spawn(move || {
            let _ = child.wait();
        });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // None of these tests starts a launcher: they either fail validation
    // before reaching one or run on a platform without any.

    #[test]
    fn process_id_and_executable() {
        assert!(id() > 0);
        let exe = current_exe().unwrap();
        assert!(exe.is_absolute());
        assert!(exe.is_file(), "{exe}");
    }

    #[test]
    fn open_url_rejects_dangerous_urls_before_launching() {
        for url in [
            "file:///etc/passwd",
            "calc.exe",
            "javascript:alert(1)",
            "https://a b",
        ] {
            match open_url(url) {
                Err(PlatformError::Io(io)) => {
                    assert_eq!(io.kind(), std::io::ErrorKind::InvalidInput, "{url}");
                }
                Err(PlatformError::Unsupported { .. }) => assert!(!desktop()),
                other => panic!("{url}: unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn reveal_rejects_a_missing_path_before_launching() {
        let missing = Utf8Path::new("/definitely/not/here/coreverse.cvs");
        match reveal_in_file_manager(missing) {
            Err(PlatformError::Io(io)) => assert_eq!(io.kind(), std::io::ErrorKind::NotFound),
            Err(PlatformError::Unsupported { .. }) => assert!(!desktop()),
            other => panic!("unexpected {other:?}"),
        }
    }
}
