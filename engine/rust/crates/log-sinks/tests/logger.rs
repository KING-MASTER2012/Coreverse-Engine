//! Tests logger system
use log_core::{Diagnostic, DiagnosticBuilder, LogSink, Severity};

use log_sinks::Logger;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CounterSink {
    emitted: Arc<AtomicUsize>,
    shutdown: Arc<AtomicUsize>,
}

impl LogSink for CounterSink {
    fn emit(&self, _: &Diagnostic) {
        self.emitted.fetch_add(1, Ordering::SeqCst);
    }

    fn shutdown(&self) {
        self.shutdown.fetch_add(1, Ordering::SeqCst);
    }
}

fn make_diag(sev: Severity) -> Diagnostic {
    DiagnosticBuilder::new("CV", "TEST", 1, sev, "hello").build()
}

#[test]
fn logger_accepts_sink() {
    let logger = Logger::new();

    logger.add_sink(Box::new(CounterSink {
        emitted: Arc::new(AtomicUsize::new(0)),
        shutdown: Arc::new(AtomicUsize::new(0)),
    }));
}

#[test]
fn emit_calls_every_sink() {
    let logger = Logger::new();

    let counter = Arc::new(AtomicUsize::new(0));

    logger.add_sink(Box::new(CounterSink {
        emitted: counter.clone(),
        shutdown: Arc::new(AtomicUsize::new(0)),
    }));

    logger.emit(make_diag(Severity::Info));

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn emit_respects_minimum_severity() {
    let logger = Logger::new();

    let counter = Arc::new(AtomicUsize::new(0));

    logger.add_sink(Box::new(CounterSink {
        emitted: counter.clone(),
        shutdown: Arc::new(AtomicUsize::new(0)),
    }));

    logger.set_min_severity(Severity::Warning);

    logger.emit(make_diag(Severity::Info));

    assert_eq!(counter.load(Ordering::SeqCst), 0);

    logger.emit(make_diag(Severity::Error));

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn shutdown_calls_every_sink() {
    let logger = Logger::new();

    let shutdown = Arc::new(AtomicUsize::new(0));

    logger.add_sink(Box::new(CounterSink {
        emitted: Arc::new(AtomicUsize::new(0)),
        shutdown: shutdown.clone(),
    }));

    logger.shutdown();

    assert_eq!(shutdown.load(Ordering::SeqCst), 1);
}

#[test]
fn default_creates_logger() {
    let _logger = Logger::default();
}
