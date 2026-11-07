//! What to run to open a URL or show a file in the file manager.
//!
//! The functions are pure — they build command lines and check input, and run
//! nothing — so they are compiled and unit-tested on every platform even
//! though each OS uses only its own.

#![allow(
    dead_code,
    reason = "each OS uses a different subset; all of them are compiled and tested everywhere"
)]

use std::ffi::OsString;
use std::path::Path;

use ccore::platform::Platform;

use crate::error::{PlatformError, Result};

/// The URL schemes `open_url` accepts.
///
/// The launchers (`ShellExecute`, `open`, `xdg-open`) start *whatever* the
/// system associates with their argument: a program, a document, a script.
/// Restricting the scheme keeps `open_url` from becoming a way to run those
/// when a URL comes from a file, a network message or a chat.
pub(crate) const ALLOWED_URL_SCHEMES: [&str; 3] = ["http", "https", "mailto"];

fn invalid_input(message: String) -> PlatformError {
    PlatformError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    ))
}

/// Checks that `url` is an absolute URL of an allowed scheme without
/// whitespace or control characters.
pub(crate) fn validate_url(url: &str) -> Result<()> {
    let Some((scheme, rest)) = url.split_once(':') else {
        return Err(invalid_input(format!("{url:?} is not an absolute URL")));
    };
    let scheme_ok = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !scheme_ok {
        return Err(invalid_input(format!("{url:?} has no valid URL scheme")));
    }
    if !ALLOWED_URL_SCHEMES
        .iter()
        .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
    {
        return Err(invalid_input(format!(
            "URL scheme {scheme:?} is not allowed (allowed: {})",
            ALLOWED_URL_SCHEMES.join(", ")
        )));
    }
    if rest.is_empty() {
        return Err(invalid_input(format!(
            "{url:?} has nothing after the scheme"
        )));
    }
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid_input(
            "URL contains whitespace or control characters".to_owned(),
        ));
    }

    if scheme.eq_ignore_ascii_case("mailto") {
        if mailto_requests_attachment(rest) {
            return Err(invalid_input(
                "mailto: URLs may not attach files (attach/attachment parameter)".to_owned(),
            ));
        }
    } else {
        // http(s) needs `//` and a host. Without it (`http:C:\x`,
        // `http:/path`) the shell may read the rest as a local path, and a
        // backslash is treated like `/` by Windows URL handling.
        let host = rest.strip_prefix("//").unwrap_or("");
        if host.is_empty() || host.starts_with(['/', '?', '#']) || rest.contains('\\') {
            return Err(invalid_input(format!(
                "{url:?} is not a valid {scheme} URL: expected '{scheme}://<host>...'"
            )));
        }
    }
    Ok(())
}

/// Whether a `mailto:` target (everything after the scheme) asks the mail
/// client to attach a file. Several clients honour a non-standard
/// `attach=<local path>` parameter, which would let a link send local files.
fn mailto_requests_attachment(target: &str) -> bool {
    let Some((_, query)) = target.split_once('?') else {
        return false;
    };
    query.split(['&', ';']).any(|pair| {
        let name = pair.split_once('=').map_or(pair, |(name, _)| name);
        let name = percent_decode_lossy(name).to_ascii_lowercase();
        name == "attach" || name == "attachment"
    })
}

/// Decodes `%XX` escapes (invalid ones are kept as they are).
fn percent_decode_lossy(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = |byte: u8| char::from(byte).to_digit(16);
        if bytes[index] == b'%'
            && let (Some(high), Some(low)) = (
                bytes.get(index + 1).copied().and_then(hex),
                bytes.get(index + 2).copied().and_then(hex),
            )
        {
            out.push((high * 16 + low) as u8);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A program and its arguments.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Launch {
    pub(crate) program: &'static str,
    pub(crate) args: Vec<OsString>,
}

/// The command that opens an already validated `url`, for platforms that
/// launch through a program (macOS and Linux).
pub(crate) fn open_url_launch(platform: Platform, url: &str) -> Option<Launch> {
    let program = match platform {
        Platform::MacOS => "open",
        Platform::Linux => "xdg-open",
        _ => return None,
    };
    // A validated URL starts with a letter, so it cannot be read as an option.
    Some(Launch {
        program,
        args: vec![url.into()],
    })
}

/// The command that shows the absolute path `path` in the file manager.
///
/// macOS selects the item in a Finder window. Linux has no portable way to
/// select an item, so it opens the folder: the directory itself, or the
/// containing directory of a file.
pub(crate) fn reveal_launch(platform: Platform, path: &Path, is_dir: bool) -> Option<Launch> {
    match platform {
        Platform::MacOS => Some(Launch {
            program: "open",
            // `-R` reveals instead of opening. An absolute path starts with a
            // separator, so it cannot be read as an option.
            args: vec!["-R".into(), path.as_os_str().to_owned()],
        }),
        Platform::Linux => {
            let folder = if is_dir {
                path
            } else {
                path.parent().unwrap_or(path)
            };
            Some(Launch {
                program: "xdg-open",
                args: vec![folder.as_os_str().to_owned()],
            })
        }
        _ => None,
    }
}

/// The `explorer.exe` parameters that select `path`: `/select,"C:\a\b"`.
///
/// Explorer needs backslashes, and the path is quoted because it may contain
/// spaces. `"` is not a legal character in a Windows path, so a path that
/// has one is refused rather than allowed to break out of the quotes.
pub(crate) fn windows_select_params(path: &str) -> Result<String> {
    if path.contains(['"', '\0']) {
        return Err(invalid_input(format!(
            "{path:?} contains a character that is not valid in a Windows path"
        )));
    }
    Ok(format!("/select,\"{}\"", path.replace('/', "\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_web_and_mail_urls() {
        for url in [
            "https://coreverse.example/docs?q=1#top",
            "http://localhost:8080/",
            "HTTPS://EXAMPLE.COM",
            "mailto:team@example.com?subject=Hello%20there",
            "mailto:team@example.com?subject=a&body=b",
            "https://user@example.com:8443/a?b=c",
        ] {
            validate_url(url).unwrap_or_else(|e| panic!("{url}: {e}"));
        }
    }

    #[test]
    fn rejects_everything_that_could_start_a_program_or_open_a_file() {
        for url in [
            "",
            "example.com",
            "calc.exe",
            "C:\\Windows\\System32\\calc.exe",
            "c:/windows/notepad.exe",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ms-msdt:/id PCWDiagnostic",
            "-flag",
            "--help",
            "://nothing",
            "1http://x",
            "ht tp://x",
            "https:",
            "mailto:",
        ] {
            let err = validate_url(url).expect_err(url);
            assert!(
                matches!(&err, PlatformError::Io(io) if io.kind() == std::io::ErrorKind::InvalidInput),
                "{url:?}: {err:?}"
            );
        }
    }

    #[test]
    fn web_urls_need_a_host() {
        for url in [
            "http:C:\\Windows\\notepad.exe",
            "https:/only-one-slash.example",
            "http:relative",
            "https:///no-host",
            "http://?query",
            "https://example.com\\evil",
        ] {
            assert!(validate_url(url).is_err(), "{url:?}");
        }
    }

    #[test]
    fn mailto_cannot_attach_local_files() {
        for url in [
            "mailto:a@b.c?attach=C:/secret.txt",
            "mailto:a@b.c?subject=x&ATTACH=/etc/passwd",
            "mailto:a@b.c?attachment=/etc/passwd",
            "mailto:a@b.c?%61ttach=/etc/passwd",
            "mailto:a@b.c?subject=x;attach=/etc/passwd",
        ] {
            assert!(validate_url(url).is_err(), "{url:?}");
        }
        // A value that merely mentions the word is fine.
        validate_url("mailto:a@b.c?subject=please%20attach%20the%20report").unwrap();
    }

    #[test]
    fn rejects_whitespace_and_control_characters() {
        for url in [
            "https://a b",
            "https://a\nb",
            "https://a\tb",
            "https://a\0b",
            "https://a\u{7f}b",
            "https://a\u{a0}b",
        ] {
            assert!(validate_url(url).is_err(), "{url:?}");
        }
    }

    #[test]
    fn open_command_per_platform() {
        let url = "https://coreverse.example";
        assert_eq!(
            open_url_launch(Platform::MacOS, url),
            Some(Launch {
                program: "open",
                args: vec![url.into()]
            })
        );
        assert_eq!(
            open_url_launch(Platform::Linux, url),
            Some(Launch {
                program: "xdg-open",
                args: vec![url.into()]
            })
        );
        for platform in [
            Platform::Windows,
            Platform::Android,
            Platform::IOS,
            Platform::Unknown,
        ] {
            assert_eq!(open_url_launch(platform, url), None);
        }
    }

    #[test]
    fn reveal_command_per_platform() {
        let file = Path::new("/home/user/project/scene.cvs");
        let launch = reveal_launch(Platform::MacOS, file, false).unwrap();
        assert_eq!(launch.program, "open");
        assert_eq!(
            launch.args,
            vec![OsString::from("-R"), file.as_os_str().to_owned()]
        );

        // Linux cannot select: a file opens its folder, a folder itself.
        let launch = reveal_launch(Platform::Linux, file, false).unwrap();
        assert_eq!(launch.program, "xdg-open");
        assert_eq!(launch.args, vec![OsString::from("/home/user/project")]);
        let dir = Path::new("/home/user/project");
        let launch = reveal_launch(Platform::Linux, dir, true).unwrap();
        assert_eq!(launch.args, vec![OsString::from("/home/user/project")]);

        assert_eq!(reveal_launch(Platform::Windows, file, false), None);
        assert_eq!(reveal_launch(Platform::Android, file, false), None);
    }

    #[test]
    fn windows_select_parameters() {
        assert_eq!(
            windows_select_params("C:\\My Games\\scene.cvs").unwrap(),
            "/select,\"C:\\My Games\\scene.cvs\""
        );
        assert_eq!(
            windows_select_params("C:/My Games/scene.cvs").unwrap(),
            "/select,\"C:\\My Games\\scene.cvs\""
        );
        assert!(windows_select_params("C:\\a\" /e,calc.exe \"b").is_err());
        assert!(windows_select_params("C:\\a\0b").is_err());
    }
}
