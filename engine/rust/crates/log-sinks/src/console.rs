use log_core::{Diagnostic, LogSink, Severity, format};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Writes diagnostics to stdout/stderr, optionally colored.
///
/// - `Warning` and below go to stdout, `Error` and above go to stderr.
/// - Below `min_severity`, diagnostics are dropped silently by this sink
///   (other sinks, e.g. a persistent file sink, may still record them).
pub struct ConsoleSink {
    color: AtomicBool,
    min_severity: AtomicU8,
}

impl ConsoleSink {
    /// Function what creates new ConsoleSink Object
    pub fn new() -> Self {
        Self {
            color: AtomicBool::new(true),
            min_severity: AtomicU8::new(Severity::Trace.as_u8()),
        }
    }
    /// Function what sets the output color before console's run
    pub fn with_color(color: bool) -> Self {
        let s = Self::new();
        s.set_color(color);
        s
    }
    /// Function what sets output color after console's run
    pub fn set_color(&self, enabled: bool) {
        self.color.store(enabled, Ordering::Relaxed);
    }
    /// The function that sets the minimum severity of the log entry.
    pub fn set_min_severity(&self, severity: Severity) {
        self.min_severity.store(severity.as_u8(), Ordering::Relaxed);
    }
}

impl Default for ConsoleSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink for ConsoleSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        let min =
            Severity::from_u8(self.min_severity.load(Ordering::Relaxed)).unwrap_or(Severity::Trace);
        if diagnostic.severity < min {
            return;
        }
        let text = if self.color.load(Ordering::Relaxed) {
            format::format_colored(diagnostic)
        } else {
            format::format_plain(diagnostic)
        };
        if diagnostic.severity >= Severity::Error {
            let _ = writeln!(std::io::stderr(), "{text}");
        } else {
            let _ = writeln!(std::io::stdout(), "{text}");
        }
    }
}
