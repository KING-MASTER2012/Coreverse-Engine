//! `FileSink` / `NativeLogVfs`: previous-session logs, path validation and
//! re-entrancy of the logger.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use log_core::{Diagnostic, DiagnosticBuilder, LogSink, Severity};
use log_sinks::{FileSink, LogVfs, Logger, NativeLogVfs};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cv_logs_{name}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn diag(message: &str) -> Diagnostic {
    DiagnosticBuilder::new("CV", "TEST", 1, Severity::Error, message).build()
}

#[test]
fn native_log_vfs_only_accepts_plain_relative_names() {
    let dir = TempDir::new("paths");
    let vfs = NativeLogVfs::new(&dir.0).unwrap();

    assert!(vfs.append_line("engine.log", "fine").is_ok());
    assert!(vfs.append_line("sub/dir.log", "fine").is_ok());
    for bad in ["../escape.log", "a/../../escape.log", "/etc/cron.d/x", ""] {
        assert!(vfs.append_line(bad, "x").is_err(), "append {bad:?}");
        assert!(vfs.clear(bad).is_err(), "clear {bad:?}");
        assert!(vfs.delete(bad).is_err(), "delete {bad:?}");
    }
    assert!(!dir.0.join("escape.log").exists());
}

#[test]
fn the_previous_sessions_log_is_kept_not_discarded() {
    let dir = TempDir::new("rotate");
    let logs = dir.0.join("Logs");
    std::fs::create_dir_all(&logs).unwrap();
    // What an unclean exit leaves behind: logs that were never deleted.
    std::fs::write(logs.join("engine.log"), "crash trail\n").unwrap();
    std::fs::write(logs.join("editor.log"), "").unwrap();

    let sink = FileSink::new(NativeLogVfs::new(&dir.0).unwrap()).unwrap();

    assert_eq!(
        std::fs::read_to_string(logs.join("engine.log.previous.log")).unwrap(),
        "crash trail\n"
    );
    assert_eq!(
        std::fs::read_to_string(logs.join("engine.log")).unwrap(),
        ""
    );
    // An empty old log is not worth keeping.
    assert!(!logs.join("editor.log.previous.log").exists());

    // The new session writes into the fresh file...
    sink.emit(&diag("new session"));
    assert!(
        std::fs::read_to_string(logs.join("latest.log"))
            .unwrap()
            .contains("new session")
    );
    // ...and the next start replaces the older "previous" copy.
    drop(sink);
    let _again = FileSink::new(NativeLogVfs::new(&dir.0).unwrap()).unwrap();
    assert!(
        std::fs::read_to_string(logs.join("latest.log.previous.log"))
            .unwrap()
            .contains("new session")
    );
}

#[test]
fn a_message_cannot_forge_extra_lines_in_a_log_file() {
    let dir = TempDir::new("forge");
    let sink = FileSink::new(NativeLogVfs::new(&dir.0).unwrap()).unwrap();
    sink.emit(&diag("one\n[0] [CV] [Error] [CV-TEST-0002] forged"));

    let latest = std::fs::read_to_string(dir.0.join("Logs/latest.log")).unwrap();
    assert_eq!(latest.lines().count(), 1, "{latest:?}");
}

/// A sink that adds another sink from inside `emit`.
struct AddsASink {
    logger: Arc<Logger>,
}

impl LogSink for AddsASink {
    fn emit(&self, _: &Diagnostic) {
        struct Quiet;
        impl LogSink for Quiet {
            fn emit(&self, _: &Diagnostic) {}
        }
        // Used to deadlock: `emit` ran under the sinks' read lock, and this
        // wants the write lock.
        self.logger.add_sink(Box::new(Quiet));
    }
}

#[test]
fn a_sink_may_add_a_sink_while_emitting() {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let logger = Arc::new(Logger::new());
        logger.add_sink(Box::new(AddsASink {
            logger: Arc::clone(&logger),
        }));
        logger.emit(diag("trigger"));
        logger.emit(diag("second emit sees the added sink"));
        let _ = done.send(());
    });
    assert!(
        finished.recv_timeout(Duration::from_secs(10)).is_ok(),
        "Logger::emit deadlocked"
    );
}
