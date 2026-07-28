//! Diagnostic-first logging core for the Coreverse Engine.
//!
//! A [`Diagnostic`] is the first-class object; text (plain or colored) is
//! just one rendering of it (see [`format`]). Every diagnostic carries a
//! structured [`DiagnosticCode`] of the shape `<Producer>-<Category>-<Number>`,
//! where the producer is always the tool that generated the diagnostic
//! (never a programming language - see [`producer`]).

/// Mod diagnostic.rs
pub mod diagnostic;
/// Mod format.rs
pub mod format;
/// Mod producer.rs
pub mod producer;
/// Mod severity.rs
pub mod severity;

pub use diagnostic::{Diagnostic, DiagnosticBuilder, DiagnosticCode};
pub use severity::Severity;

/// Re-exported so the `register_producer!` macro can expand without forcing
/// downstream crates to depend on `inventory` directly.
pub use inventory;

/// Implemented by every log destination (console, file/VFS, network, ...).
/// Sinks receive fully-formed diagnostics and decide how/whether to persist
/// or render them; they never construct diagnostics themselves.
pub trait LogSink: Send + Sync {
    /// Called once when the logging system creating
    fn emit(&self, diagnostic: &Diagnostic);

    /// Called once when the logging system is shutting down cleanly, giving
    /// the sink a chance to flush or clean up (e.g. deleting non-persistent
    /// log files). Default: no-op.
    fn shutdown(&self) {}
}
