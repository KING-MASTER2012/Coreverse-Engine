use serde::{Deserialize, Serialize};

/// Severity of a diagnostic, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Severity {
    /// Trace
    Trace = 0,
    /// Debug
    Debug = 1,
    /// Info
    Info = 2,
    /// Notice
    Notice = 3,
    /// Warning
    Warning = 4,
    /// Error
    Error = 5,
    /// Critical
    Critical = 6,
    /// Fatal
    Fatal = 7,
}

impl Severity {
    /// Severity levels impl
    pub const ALL: [Severity; 8] = [
        Severity::Trace,
        Severity::Debug,
        Severity::Info,
        Severity::Notice,
        Severity::Warning,
        Severity::Error,
        Severity::Critical,
        Severity::Fatal,
    ];
    /// Function what returns severity as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Trace => "Trace",
            Severity::Debug => "Debug",
            Severity::Info => "Info",
            Severity::Notice => "Notice",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
            Severity::Critical => "Critical",
            Severity::Fatal => "Fatal",
        }
    }
    /// Function what returns severity as string lowercase
    pub fn from_str_lossy(s: &str) -> Option<Severity> {
        Some(match s.to_ascii_lowercase().as_str() {
            "trace" => Severity::Trace,
            "debug" => Severity::Debug,
            "info" => Severity::Info,
            "notice" => Severity::Notice,
            "warning" | "warn" => Severity::Warning,
            "error" => Severity::Error,
            "critical" => Severity::Critical,
            "fatal" => Severity::Fatal,
            _ => return None,
        })
    }

    /// ANSI foreground/background escape sequence used for console rendering.
    pub fn ansi_color(&self) -> &'static str {
        match self {
            Severity::Trace => "\x1b[90m",    // gray
            Severity::Debug => "\x1b[36m",    // cyan
            Severity::Info => "\x1b[37m",     // white
            Severity::Notice => "\x1b[34m",   // blue
            Severity::Warning => "\x1b[33m",  // yellow
            Severity::Error => "\x1b[31m",    // red
            Severity::Critical => "\x1b[35m", // magenta
            Severity::Fatal => "\x1b[97;41m", // white on red
        }
    }

    /// Numeric value, primarily useful across the FFI boundary.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
    ///Opposite as_u8 function
    pub fn from_u8(v: u8) -> Option<Severity> {
        Severity::ALL.get(v as usize).copied()
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
