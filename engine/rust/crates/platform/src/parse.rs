//! Pure parsing and detection helpers used by the OS backends.
//!
//! They contain no OS calls, so they are compiled and unit-tested on every
//! platform even though each backend only uses some of them.

#![allow(
    dead_code,
    reason = "each OS backend uses a different subset; all of them are compiled and tested everywhere"
)]

use std::collections::HashSet;
use std::path::Path;

use crate::info::DisplayServer;

/// `NAME` and `VERSION_ID` of an `os-release` file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct OsRelease {
    pub(crate) name: String,
    pub(crate) version_id: Option<String>,
}

/// Parses the contents of `/etc/os-release`. Returns `None` when there is no
/// `NAME` entry.
pub(crate) fn parse_os_release(text: &str) -> Option<OsRelease> {
    let mut name = None;
    let mut version_id = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unquote(value.trim());
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "NAME" => name = Some(value.to_owned()),
            "VERSION_ID" => version_id = Some(value.to_owned()),
            _ => {}
        }
    }
    name.map(|name| OsRelease { name, version_id })
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

/// Decides between X11 and Wayland from the session's environment.
///
/// A server counts as reachable only if its variable is set and non-empty. When
/// both are (a Wayland session with XWayland), `XDG_SESSION_TYPE` decides, and
/// anything but `x11` means Wayland. Neither set means a headless session.
pub(crate) fn detect_unix_display_server(
    wayland_display: Option<&str>,
    x11_display: Option<&str>,
    session_type: Option<&str>,
) -> DisplayServer {
    let set = |value: Option<&str>| value.is_some_and(|v| !v.is_empty());
    match (set(wayland_display), set(x11_display)) {
        (true, false) => DisplayServer::Wayland,
        (false, true) => DisplayServer::X11,
        (true, true) => {
            if session_type.is_some_and(|t| t.eq_ignore_ascii_case("x11")) {
                DisplayServer::X11
            } else {
                DisplayServer::Wayland
            }
        }
        (false, false) => DisplayServer::Headless,
    }
}

/// `RelationProcessorCore` in `LOGICAL_PROCESSOR_RELATIONSHIP`.
const RELATION_PROCESSOR_CORE: i32 = 0;

/// Counts the `RelationProcessorCore` records in the buffer filled by
/// `GetLogicalProcessorInformationEx`. Every record starts with its
/// relationship (`i32`) and its own size in bytes (`u32`). `None` means the
/// buffer is malformed.
pub(crate) fn count_windows_core_records(buffer: &[u8]) -> Option<usize> {
    let mut offset = 0usize;
    let mut cores = 0usize;
    while offset < buffer.len() {
        let header = buffer.get(offset..offset.checked_add(8)?)?;
        let relationship = i32::from_ne_bytes(header[0..4].try_into().ok()?);
        let size = usize::try_from(u32::from_ne_bytes(header[4..8].try_into().ok()?)).ok()?;
        if size < 8 || offset.checked_add(size)? > buffer.len() {
            return None;
        }
        if relationship == RELATION_PROCESSOR_CORE {
            cores += 1;
        }
        offset += size;
    }
    Some(cores)
}

/// Counts physical cores below `/sys/devices/system/cpu`: every distinct
/// `topology/thread_siblings_list` is one core, however many hardware threads
/// it has. Offline CPUs have no `topology` directory and are skipped. `None`
/// when no CPU exposes its topology (containers, some Android builds).
pub(crate) fn physical_cores_from_sysfs(cpu_root: &Path) -> Option<usize> {
    let mut sibling_sets = HashSet::new();
    for entry in std::fs::read_dir(cpu_root).ok()?.flatten() {
        let name = entry.file_name();
        let is_cpu_dir = name
            .to_str()
            .and_then(|n| n.strip_prefix("cpu"))
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()));
        if !is_cpu_dir {
            continue;
        }
        if let Ok(list) =
            std::fs::read_to_string(entry.path().join("topology/thread_siblings_list"))
        {
            let list = list.trim();
            if !list.is_empty() {
                sibling_sets.insert(list.to_owned());
            }
        }
    }
    (!sibling_sets.is_empty()).then_some(sibling_sets.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_with_quotes_and_comments() {
        let text = "# comment\nNAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nID=ubuntu\n";
        assert_eq!(
            parse_os_release(text),
            Some(OsRelease {
                name: "Ubuntu".into(),
                version_id: Some("24.04".into())
            })
        );
    }

    #[test]
    fn os_release_rolling_distro_has_no_version() {
        let text = "NAME='Arch Linux'\nPRETTY_NAME=\"Arch Linux\"\nID=arch\n";
        let release = parse_os_release(text).unwrap();
        assert_eq!(release.name, "Arch Linux");
        assert_eq!(release.version_id, None);
    }

    #[test]
    fn os_release_without_name_is_none() {
        assert_eq!(parse_os_release("ID=x\nVERSION_ID=1\n"), None);
        assert_eq!(parse_os_release(""), None);
        assert_eq!(parse_os_release("NAME=\n"), None);
    }

    #[test]
    fn display_server_decision_table() {
        use DisplayServer::*;
        let d = detect_unix_display_server;
        assert_eq!(d(Some("wayland-0"), None, Some("wayland")), Wayland);
        assert_eq!(d(None, Some(":0"), Some("x11")), X11);
        assert_eq!(d(None, None, None), Headless);
        assert_eq!(d(Some(""), Some(""), Some("wayland")), Headless);
        // XWayland session: both reachable, the session type decides.
        assert_eq!(d(Some("wayland-0"), Some(":0"), Some("wayland")), Wayland);
        assert_eq!(d(Some("wayland-0"), Some(":0"), Some("X11")), X11);
        assert_eq!(d(Some("wayland-0"), Some(":0"), None), Wayland);
        // The session type alone is not enough: nothing to connect to.
        assert_eq!(d(None, None, Some("wayland")), Headless);
    }

    fn record(relationship: i32, size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&relationship.to_ne_bytes());
        bytes.extend_from_slice(&size.to_ne_bytes());
        bytes.resize(usize::try_from(size).unwrap(), 0);
        bytes
    }

    #[test]
    fn counts_only_core_records() {
        let mut buffer = record(0, 48);
        buffer.extend(record(0, 48));
        buffer.extend(record(2, 64)); // a cache record
        buffer.extend(record(0, 48));
        assert_eq!(count_windows_core_records(&buffer), Some(3));
        assert_eq!(count_windows_core_records(&[]), Some(0));
    }

    #[test]
    fn rejects_malformed_core_buffers() {
        // Size smaller than the header would loop forever.
        let mut tiny = Vec::new();
        tiny.extend_from_slice(&0i32.to_ne_bytes());
        tiny.extend_from_slice(&4u32.to_ne_bytes());
        assert_eq!(count_windows_core_records(&tiny), None);
        // Record claims more bytes than the buffer has.
        let mut truncated = record(0, 48);
        truncated.truncate(40);
        assert_eq!(count_windows_core_records(&truncated), None);
        // Trailing bytes too short for a header.
        let mut trailing = record(0, 48);
        trailing.extend_from_slice(&[1, 2, 3]);
        assert_eq!(count_windows_core_records(&trailing), None);
    }

    fn fake_cpu(root: &Path, dir: &str, siblings: Option<&str>) {
        let cpu = root.join(dir);
        std::fs::create_dir_all(cpu.join("topology")).unwrap();
        if let Some(list) = siblings {
            std::fs::write(cpu.join("topology/thread_siblings_list"), list).unwrap();
        }
    }

    #[test]
    fn sysfs_counts_distinct_sibling_sets() {
        let tmp = tempfile::tempdir().unwrap();
        // 2 cores with 2 hardware threads each, one offline cpu without
        // topology files, and entries that are not cpu directories.
        fake_cpu(tmp.path(), "cpu0", Some("0,2\n"));
        fake_cpu(tmp.path(), "cpu1", Some("1,3\n"));
        fake_cpu(tmp.path(), "cpu2", Some("0,2\n"));
        fake_cpu(tmp.path(), "cpu3", Some("1,3\n"));
        fake_cpu(tmp.path(), "cpu4", None);
        std::fs::create_dir_all(tmp.path().join("cpufreq")).unwrap();
        std::fs::create_dir_all(tmp.path().join("cpuidle")).unwrap();
        std::fs::write(tmp.path().join("online"), "0-3").unwrap();
        assert_eq!(physical_cores_from_sysfs(tmp.path()), Some(2));
    }

    #[test]
    fn sysfs_without_topology_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(physical_cores_from_sysfs(tmp.path()), None);
        fake_cpu(tmp.path(), "cpu0", None);
        assert_eq!(physical_cores_from_sysfs(tmp.path()), None);
        assert_eq!(physical_cores_from_sysfs(&tmp.path().join("missing")), None);
    }
}
