//! Read-only access to the process environment.
//!
//! There is deliberately no way to *write* the environment. Since Rust's 2024
//! edition `std::env::set_var` is `unsafe` (another thread may be reading the
//! environment through C's `getenv` at the same moment), and the workspace
//! forbids unsafe code outside the OS backends. Code that needs to hand
//! variables to a child process should set them on the
//! [`Command`](std::process::Command) instead.
//!
//! A name that can never be a variable (empty, or containing `=` or NUL) is
//! treated like a variable that is not set.

use std::ffi::OsString;

use camino::Utf8PathBuf;

use crate::error::{PlatformError, Result};
use crate::paths::utf8;

fn invalid_data(message: String) -> PlatformError {
    PlatformError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['=', '\0'])
}

/// The raw value of `name`, if it is set.
#[must_use]
pub fn var_os(name: &str) -> Option<OsString> {
    if valid_name(name) {
        std::env::var_os(name)
    } else {
        None
    }
}

/// The value of `name` as text; `None` when it is not set.
///
/// # Errors
///
/// [`PlatformError::Io`] with [`InvalidData`](std::io::ErrorKind::InvalidData)
/// when the value is set but is not valid Unicode.
pub fn var(name: &str) -> Result<Option<String>> {
    match var_os(name) {
        None => Ok(None),
        Some(value) => value
            .into_string()
            .map(Some)
            .map_err(|_| invalid_data(format!("environment variable {name} is not valid Unicode"))),
    }
}

/// The value of `name` as a path; `None` when it is not set or empty.
///
/// # Errors
///
/// [`PlatformError::NonUtf8Path`] when the value is not valid UTF-8.
pub fn var_path(name: &str) -> Result<Option<Utf8PathBuf>> {
    match var_os(name) {
        Some(value) if !value.is_empty() => utf8(value.into()).map(Some),
        _ => Ok(None),
    }
}

/// The value of `name` as a flag; `None` when it is not set or empty.
///
/// `1`, `true`, `yes` and `on` are true, `0`, `false`, `no` and `off` are
/// false, in any letter case and ignoring surrounding whitespace.
///
/// # Errors
///
/// [`PlatformError::Io`] with [`InvalidData`](std::io::ErrorKind::InvalidData)
/// when the value is none of those (a typo should not silently mean "off").
pub fn var_bool(name: &str) -> Result<Option<bool>> {
    let Some(text) = var(name)? else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    parse_bool(&text).map(Some).ok_or_else(|| {
        invalid_data(format!(
            "environment variable {name}={text:?} is not a flag"
        ))
    })
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// The command-line arguments, program name first.
///
/// Unlike [`std::env::args`] this does not panic on arguments that are not
/// valid Unicode.
///
/// # Errors
///
/// [`PlatformError::Io`] with [`InvalidData`](std::io::ErrorKind::InvalidData)
/// when an argument is not valid Unicode.
pub fn args() -> Result<Vec<String>> {
    std::env::args_os()
        .map(|arg| {
            arg.into_string().map_err(|arg| {
                invalid_data(format!(
                    "command-line argument {arg:?} is not valid Unicode"
                ))
            })
        })
        .collect()
}

/// The current working directory.
///
/// # Errors
///
/// [`PlatformError::Io`] when it cannot be determined, and
/// [`PlatformError::NonUtf8Path`] when it is not UTF-8.
pub fn current_dir() -> Result<Utf8PathBuf> {
    utf8(std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The tests read variables that exist before the test process starts or
    // that cannot exist, never set any (that is the point of this module).

    #[test]
    fn unset_variables_are_none() {
        let name = "COREVERSE_PLATFORM_TEST_SURELY_UNSET";
        assert_eq!(var_os(name), None);
        assert_eq!(var(name).unwrap(), None);
        assert_eq!(var_path(name).unwrap(), None);
        assert_eq!(var_bool(name).unwrap(), None);
    }

    #[test]
    fn impossible_names_are_unset_not_panics() {
        for name in ["", "A=B", "A\0B", "="] {
            assert_eq!(var_os(name), None, "{name:?}");
            assert_eq!(var(name).unwrap(), None, "{name:?}");
        }
    }

    #[test]
    fn existing_variable_is_found() {
        // Cargo sets this for every test run.
        let manifest_dir = var("CARGO_MANIFEST_DIR").unwrap().expect("set by cargo");
        assert!(!manifest_dir.is_empty());
        let as_path = var_path("CARGO_MANIFEST_DIR").unwrap().unwrap();
        assert_eq!(as_path.as_str(), manifest_dir);
        assert!(as_path.is_absolute());
    }

    #[test]
    fn bool_parsing() {
        for text in ["1", "true", "TRUE", " Yes ", "on", "On"] {
            assert_eq!(parse_bool(text), Some(true), "{text:?}");
        }
        for text in ["0", "false", "False", "no", " OFF"] {
            assert_eq!(parse_bool(text), Some(false), "{text:?}");
        }
        for text in ["", "2", "y", "enabled", "tru"] {
            assert_eq!(parse_bool(text), None, "{text:?}");
        }
    }

    #[test]
    fn args_start_with_the_program() {
        let args = args().unwrap();
        assert!(!args.is_empty());
    }

    #[test]
    fn current_dir_is_absolute() {
        assert!(current_dir().unwrap().is_absolute());
    }
}
