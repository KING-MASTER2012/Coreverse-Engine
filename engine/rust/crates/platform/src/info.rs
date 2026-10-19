//! System information: operating system, CPU, memory and display server.
//!
//! Everything here is queried at call time and never cached; the calls are
//! cheap except [`SystemInfo::collect`], which touches the OS several times.

use std::fmt;

use ccore::platform::{Architecture, Endian, Platform};

use crate::error::Result;
use crate::{parse, sys};

/// Name and version of the running operating system.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OsVersion {
    /// Product name: `"Windows"`, `"macOS"`, `"iOS"`, `"Android"`, or the
    /// distribution name on Linux (`"Ubuntu"`).
    pub name: String,
    /// Product version: `"10.0.26100"` (Windows, major.minor.build),
    /// `"14.5"` (macOS/iOS), `"14"` (Android), `"24.04"` (Linux
    /// `VERSION_ID`). Falls back to the kernel release when the system does
    /// not report one (rolling Linux distributions, Android without the
    /// property).
    pub version: String,
    /// Kernel release (`uname -r`), where the platform has one.
    pub kernel: Option<String>,
}

impl fmt::Display for OsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

/// CPU core counts.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CpuInfo {
    /// Logical cores (hardware threads) this process may use, at least 1.
    /// This is what the OS schedules on, so it already honours CPU affinity and
    /// container quotas.
    pub logical_cores: usize,
    /// Physical cores of the machine, if the platform reports them. It does not
    /// honour affinity or quotas, so inside a container it can exceed
    /// `logical_cores`.
    pub physical_cores: Option<usize>,
}

/// The windowing system the current session runs on.
///
/// Reported for the session, not for any particular Qt platform plugin: inside
/// a Wayland session with XWayland this says [`Wayland`](Self::Wayland), while
/// the editor may still run as an X11 client.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum DisplayServer {
    /// Windows (Win32 / DWM).
    Win32,
    /// macOS (AppKit).
    Cocoa,
    /// X11 (Linux).
    X11,
    /// Wayland (Linux).
    Wayland,
    /// Android's window system.
    Android,
    /// iOS (UIKit).
    UiKit,
    /// Linux without a reachable X11 or Wayland display (ssh, CI, a server).
    Headless,
    /// The platform is not supported by this crate.
    Unknown,
}

impl fmt::Display for DisplayServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DisplayServer::Win32 => "win32",
            DisplayServer::Cocoa => "cocoa",
            DisplayServer::X11 => "x11",
            DisplayServer::Wayland => "wayland",
            DisplayServer::Android => "android",
            DisplayServer::UiKit => "uikit",
            DisplayServer::Headless => "headless",
            DisplayServer::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// The operating system's name and version.
///
/// # Errors
/// [`PlatformError::Unsupported`](crate::error::PlatformError::Unsupported) on
/// unsupported platforms, or the OS error of the failing call.
pub fn os_version() -> Result<OsVersion> {
    sys::os_version()
}

/// CPU core counts. Never fails: if the logical core count cannot be
/// determined it is reported as 1.
#[must_use]
pub fn cpu_info() -> CpuInfo {
    CpuInfo {
        logical_cores: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
        physical_cores: sys::physical_cores(),
    }
}

/// Installed physical memory in bytes. Inside a container this is the
/// machine's memory, not the container's limit.
///
/// # Errors
/// [`PlatformError::Unsupported`](crate::error::PlatformError::Unsupported) on
/// unsupported platforms, or the OS error of the failing call.
pub fn total_memory_bytes() -> Result<u64> {
    sys::total_memory_bytes()
}

/// The display server of the current session.
#[must_use]
pub fn display_server() -> DisplayServer {
    match Platform::current() {
        Platform::Windows => DisplayServer::Win32,
        Platform::MacOS => DisplayServer::Cocoa,
        Platform::IOS => DisplayServer::UiKit,
        Platform::Android => DisplayServer::Android,
        Platform::Linux => {
            let var = |name: &str| std::env::var(name).ok();
            parse::detect_unix_display_server(
                var("WAYLAND_DISPLAY").as_deref(),
                var("DISPLAY").as_deref(),
                var("XDG_SESSION_TYPE").as_deref(),
            )
        }
        Platform::Unknown => DisplayServer::Unknown,
    }
}

/// Everything above in one value.
///
/// Fields the platform cannot report are `None`; collecting never fails.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SystemInfo {
    /// The operating system the crate was compiled for.
    pub platform: Platform,
    /// The CPU architecture the crate was compiled for.
    pub architecture: Architecture,
    /// Byte order of the target.
    pub endian: Endian,
    /// Pointer width of the target in bits (32 or 64).
    pub pointer_width: u32,
    /// Name and version of the running OS.
    pub os: Option<OsVersion>,
    /// CPU core counts.
    pub cpu: CpuInfo,
    /// Installed physical memory in bytes.
    pub total_memory_bytes: Option<u64>,
    /// The session's display server.
    pub display_server: DisplayServer,
}

impl SystemInfo {
    /// Queries the OS for every field.
    #[must_use]
    pub fn collect() -> Self {
        Self {
            platform: Platform::current(),
            architecture: Architecture::current(),
            endian: Endian::current(),
            pointer_width: usize::BITS,
            os: os_version().ok(),
            cpu: cpu_info(),
            total_memory_bytes: total_memory_bytes().ok(),
            display_server: display_server(),
        }
    }
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({}-endian, {}-bit)",
            self.platform, self.architecture, self.endian, self.pointer_width
        )?;
        if let Some(os) = &self.os {
            write!(f, ", {os}")?;
        }
        write!(f, ", {} logical", self.cpu.logical_cores)?;
        if let Some(physical) = self.cpu.physical_cores {
            write!(f, " / {physical} physical")?;
        }
        write!(f, " cores")?;
        if let Some(bytes) = self.total_memory_bytes {
            // Display only: the precision loss of the cast is irrelevant here.
            write!(f, ", {:.1} GiB RAM", bytes as f64 / (1u64 << 30) as f64)?;
        }
        write!(f, ", display: {}", self.display_server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_info_has_at_least_one_logical_core() {
        let cpu = cpu_info();
        assert!(cpu.logical_cores >= 1);
        if let Some(physical) = cpu.physical_cores {
            assert!(physical >= 1);
        }
    }

    #[test]
    fn os_version_is_readable() {
        let os = os_version().expect("os_version works on every supported test platform");
        assert!(!os.name.is_empty());
        assert!(!os.version.is_empty());
        assert_eq!(os.to_string(), format!("{} {}", os.name, os.version));
    }

    #[test]
    fn total_memory_is_plausible() {
        let bytes =
            total_memory_bytes().expect("total memory works on every supported test platform");
        assert!(
            bytes >= 16 * 1024 * 1024,
            "{bytes} bytes is implausibly small"
        );
    }

    #[test]
    fn display_server_matches_platform() {
        let server = display_server();
        match Platform::current() {
            Platform::Windows => assert_eq!(server, DisplayServer::Win32),
            Platform::MacOS => assert_eq!(server, DisplayServer::Cocoa),
            Platform::Linux => assert!(matches!(
                server,
                DisplayServer::X11 | DisplayServer::Wayland | DisplayServer::Headless
            )),
            _ => {}
        }
    }

    #[test]
    fn system_info_matches_compile_time_constants() {
        let info = SystemInfo::collect();
        assert_eq!(info.platform, Platform::current());
        assert_eq!(info.architecture, Architecture::current());
        assert_eq!(info.endian, Endian::current());
        assert_eq!(info.pointer_width, usize::BITS);
        assert!(info.os.is_some());
        assert!(info.total_memory_bytes.is_some());
    }

    #[test]
    fn system_info_display_is_one_line() {
        let text = SystemInfo::collect().to_string();
        assert!(!text.contains('\n'));
        assert!(text.contains("logical"));
        assert!(text.contains("display:"));
    }

    #[test]
    fn display_server_names() {
        assert_eq!(DisplayServer::Wayland.to_string(), "wayland");
        assert_eq!(DisplayServer::Headless.to_string(), "headless");
        assert_eq!(DisplayServer::UiKit.to_string(), "uikit");
    }
}
