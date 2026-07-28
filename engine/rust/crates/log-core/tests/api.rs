//! Tests Log-Core crate for API
use log_core::{DiagnosticBuilder, LogSink, Severity};

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct DummySink {
    called: Arc<AtomicBool>,
}

impl LogSink for DummySink {
    fn emit(&self, _: &log_core::Diagnostic) {
        self.called.store(true, Ordering::SeqCst);
    }

    fn shutdown(&self) {
        self.called.store(true, Ordering::SeqCst);
    }
}

#[test]
fn public_api_is_usable() {
    let diag = DiagnosticBuilder::new("CV", "TEST", 1, Severity::Info, "hello").build();

    assert_eq!(diag.severity, Severity::Info);
}

#[test]
fn logsink_trait_can_be_implemented() {
    let flag = Arc::new(AtomicBool::new(false));

    let sink = DummySink {
        called: flag.clone(),
    };

    let diag = DiagnosticBuilder::new("CV", "TEST", 1, Severity::Info, "hello").build();

    sink.emit(&diag);

    assert!(flag.load(Ordering::SeqCst));
}

#[test]
fn logsink_shutdown_default_is_callable() {
    struct EmptySink;

    impl LogSink for EmptySink {
        fn emit(&self, _: &log_core::Diagnostic) {}
    }

    let sink = EmptySink;
    sink.shutdown();
}
