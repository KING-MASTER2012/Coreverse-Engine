//! Definition and declaration of logger struct. Exports modules
/// Exports Console module
pub mod console;
/// Exports Console module
pub mod file_vfs;

pub use console::ConsoleSink;
pub use file_vfs::{FileSink, LogVfs, NativeLogVfs, default_route};

use log_core::{Diagnostic, LogSink, Severity};
use std::sync::{Arc, PoisonError, RwLock};

/// Fans a diagnostic out to every registered [`LogSink`]. This is the object
/// FFI bindings hold a handle to.
pub struct Logger {
    sinks: RwLock<Vec<Arc<dyn LogSink>>>,
    min_severity: RwLock<Severity>,
}

impl Logger {
    /// The function what creates a logger system
    pub fn new() -> Self {
        Self {
            sinks: RwLock::new(Vec::new()),
            min_severity: RwLock::new(Severity::Trace),
        }
    }
    /// The function what adds a shink
    pub fn add_sink(&self, sink: Box<dyn LogSink>) {
        self.sinks
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Arc::from(sink));
    }
    /// The function that sets the minimum severity of the logs
    pub fn set_min_severity(&self, severity: Severity) {
        *self
            .min_severity
            .write()
            .unwrap_or_else(PoisonError::into_inner) = severity;
    }
    /// The emit function
    pub fn emit(&self, diagnostic: Diagnostic) {
        let min = *self
            .min_severity
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        if diagnostic.severity < min {
            return;
        }
        for sink in self.sink_snapshot() {
            sink.emit(&diagnostic);
        }
    }

    /// Flushes/cleans up every sink. Call this once, on clean engine
    /// shutdown, so non-persistent log files get removed.
    pub fn shutdown(&self) {
        for sink in self.sink_snapshot() {
            sink.shutdown();
        }
    }

    /// The current sinks, cloned out so no lock is held while they run: a
    /// sink may log or add another sink from inside `emit` without
    /// deadlocking on the `RwLock`.
    fn sink_snapshot(&self) -> Vec<Arc<dyn LogSink>> {
        self.sinks
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
