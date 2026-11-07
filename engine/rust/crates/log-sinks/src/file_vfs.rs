use log_core::{Diagnostic, LogSink, format};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Abstraction over the `logs://` virtual filesystem. The engine's own VFS
/// crate can implement this directly against its dual-backend (loose files
/// in dev, packed `.coreproject` at export); [`NativeLogVfs`] is a
/// self-contained fallback for standalone use of the logger.
pub trait LogVfs: Send + Sync {
    /// Appends a line (plus trailing newline) to `logs://<virtual_path>`,
    /// creating the file/directory if necessary.
    fn append_line(&self, virtual_path: &str, line: &str) -> io::Result<()>;

    /// Truncates `logs://<virtual_path>` to empty, creating it if necessary.
    fn clear(&self, virtual_path: &str) -> io::Result<()>;

    /// Deletes `logs://<virtual_path>` if it exists.
    fn delete(&self, virtual_path: &str) -> io::Result<()>;

    /// Prepares `logs://<virtual_path>` for a new session: afterwards the
    /// file exists and is empty. Implementations should keep the previous
    /// session's content around (see [`NativeLogVfs`]) - it is the only
    /// record of what happened if that session crashed. The default just
    /// [`clear`](Self::clear)s the file, i.e. discards it.
    fn archive_previous(&self, virtual_path: &str) -> io::Result<()> {
        self.clear(virtual_path)
    }
}

/// Maps `logs://<name>` to `<project_root>/Logs/<name>` on the real
/// filesystem. This is what a standalone build of the logger (or the engine,
/// until its own VFS crate wires in a richer backend) uses by default.
pub struct NativeLogVfs {
    logs_dir: PathBuf,
}

impl NativeLogVfs {
    /// Function what creates NativeLogVfs Object
    pub fn new(project_root: impl AsRef<Path>) -> io::Result<Self> {
        let logs_dir = project_root.as_ref().join("Logs");
        fs::create_dir_all(&logs_dir)?;
        Ok(Self { logs_dir })
    }

    /// Maps a `logs://` name to a file inside the logs directory. Only
    /// plain relative names are accepted: `..`, absolute paths and drive
    /// prefixes would let a caller reach files outside `Logs/`.
    fn resolve(&self, virtual_path: &str) -> io::Result<PathBuf> {
        let relative = Path::new(virtual_path);
        let escapes = virtual_path.is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir));
        if escapes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("log path '{virtual_path}' must be a plain relative name inside Logs/"),
            ));
        }
        Ok(self.logs_dir.join(relative))
    }
}

impl LogVfs for NativeLogVfs {
    fn append_line(&self, virtual_path: &str, line: &str) -> io::Result<()> {
        let path = self.resolve(virtual_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{line}")
    }

    fn clear(&self, virtual_path: &str) -> io::Result<()> {
        let path = self.resolve(virtual_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(())
    }

    fn delete(&self, virtual_path: &str) -> io::Result<()> {
        let path = self.resolve(virtual_path)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Keeps the previous session's log as `<name>.previous.log` (replacing
    /// the one before it) and starts a fresh, empty `<name>`. Because a
    /// clean shutdown deletes non-persistent logs, a log that is still
    /// here at startup means the last session ended abnormally - exactly
    /// the log someone needs to find out why.
    fn archive_previous(&self, virtual_path: &str) -> io::Result<()> {
        let path = self.resolve(virtual_path)?;
        if fs::metadata(&path).is_ok_and(|meta| meta.len() > 0) {
            let mut previous = path.clone().into_os_string();
            previous.push(".previous.log");
            fs::rename(&path, PathBuf::from(previous))?;
        }
        self.clear(virtual_path)
    }
}

/// Decides which `logs://` file a diagnostic is routed to, based on its
/// producer code and category. `"latest.log"` always receives a mirror copy
/// on top of whatever this returns.
pub fn default_route(diagnostic: &Diagnostic) -> &'static str {
    let producer = diagnostic.code.producer.as_str();
    let category = diagnostic.category().to_ascii_uppercase();

    if producer.starts_with("CV-EDITOR") {
        "editor.log"
    } else if producer.starts_with("CV-BUILD") || producer.starts_with("CV-PACKAGER") {
        "build.log"
    } else if producer.starts_with("CV-") {
        if category == "RUNTIME"
            || category == "SCRIPT"
            || category == "PHYSICS"
            || category == "AUDIO"
        {
            "runtime.log"
        } else {
            "engine.log"
        }
    } else {
        // Any external tool (LLVM-CL, RS-RUSTC, JB-RR, ...) is a compiler /
        // external-analysis diagnostic from the engine's point of view.
        "compiler.log"
    }
}

/// Locks `mutex`, ignoring poisoning. A panic in some other thread while it
/// held the lock must not make every later log call panic too: the sets
/// guarded here stay valid after any panic.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A [`LogSink`] that writes every diagnostic into the project's
/// `Logs/` folder via a [`LogVfs`].
///
/// By default, log files are **session-scoped**: at startup the previous
/// session's file is kept as `<name>.previous.log` (see
/// [`LogVfs::archive_previous`]) and a fresh one is started; on clean
/// [`LogSink::shutdown`] they are deleted, except files explicitly marked
/// persistent via [`FileSink::mark_persistent`] or diagnostics with
/// `persistent: true` (whose target file becomes persistent for the rest of
/// the session).
pub struct FileSink {
    vfs: Box<dyn LogVfs>,
    route: Box<dyn Fn(&Diagnostic) -> &'static str + Send + Sync>,
    persistent_files: Mutex<HashSet<String>>,
    known_files: Mutex<HashSet<String>>,
}

impl FileSink {
    /// Creates a sink using the default routing rules, starting a new
    /// session for the standard set of log files up front (`engine.log`, `editor.log`, `compiler.log`,
    /// `runtime.log`, `build.log`, `latest.log`).
    pub fn new(vfs: impl LogVfs + 'static) -> io::Result<Self> {
        Self::with_router(vfs, default_route)
    }
    /// Function what runs router
    pub fn with_router(
        vfs: impl LogVfs + 'static,
        route: impl Fn(&Diagnostic) -> &'static str + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let sink = Self {
            vfs: Box::new(vfs),
            route: Box::new(route),
            persistent_files: Mutex::new(HashSet::new()),
            known_files: Mutex::new(HashSet::new()),
        };
        for name in [
            "engine.log",
            "editor.log",
            "compiler.log",
            "runtime.log",
            "build.log",
            "latest.log",
        ] {
            sink.vfs.archive_previous(name)?;
            lock(&sink.known_files).insert(name.to_string());
        }
        Ok(sink)
    }

    /// Marks a specific `logs://` file as persistent: it survives clean
    /// shutdown instead of being deleted.
    pub fn mark_persistent(&self, virtual_path: impl Into<String>) {
        lock(&self.persistent_files).insert(virtual_path.into());
    }

    fn record(&self, virtual_path: &str, diagnostic: &Diagnostic) {
        lock(&self.known_files).insert(virtual_path.to_string());
        let _ = self
            .vfs
            .append_line(virtual_path, &format::format_line(diagnostic));
    }
}

impl LogSink for FileSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        if diagnostic.persistent {
            let target = (self.route)(diagnostic);
            self.mark_persistent(target);
        }
        let target = (self.route)(diagnostic);
        self.record(target, diagnostic);
        self.record("latest.log", diagnostic);
    }

    fn shutdown(&self) {
        let persistent = lock(&self.persistent_files);
        let known = lock(&self.known_files);
        for file in known.iter() {
            if !persistent.contains(file) {
                let _ = self.vfs.delete(file);
            }
        }
    }
}
