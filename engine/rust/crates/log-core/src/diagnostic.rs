use crate::producer;
use crate::severity::Severity;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// `<Producer>-<Category>-<Number>`. The producer segment may itself contain
/// hyphens (e.g. `"LLVM-CL"`), so codes are always composed from structured
/// fields rather than parsed back out of a single string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagnosticCode {
    /// Producer
    pub producer: String,
    /// Category
    pub category: String,
    /// Log Number
    pub number: u32,
}

impl DiagnosticCode {
    /// Function what create new Diagnostic Code
    pub fn new(producer: impl Into<String>, category: impl Into<String>, number: u32) -> Self {
        Self {
            producer: producer.into(),
            category: category.into(),
            number,
        }
    }
    /// Function what code converts into string
    pub fn as_string(&self) -> String {
        format!("{}-{}-{:04}", self.producer, self.category, self.number)
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_string())
    }
}

/// A diagnostic is a first-class object, not a line of text. Console/file
/// text is just one possible rendering of it (see `crate::format`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The Diagnostic Code
    pub code: DiagnosticCode,
    /// The Severity
    pub severity: Severity,
    /// The Message
    pub message: String,
    /// Programming language the diagnostic relates to, if any. This is
    /// metadata only - it never influences the producer code.
    pub language: Option<String>,
    /// The Module
    pub module: Option<String>,
    /// The File
    pub file: Option<String>,
    /// The Line
    pub line: Option<u32>,
    /// The Column
    pub column: Option<u32>,
    /// The Suggestion
    pub suggestion: Option<String>,
    /// The Documentation
    pub documentation: Option<String>,
    /// The Thread
    pub thread: String,
    /// The Timestamp
    pub timestamp_unix_ms: u128,
    /// If false (default), this diagnostic's log file is cleared when the
    /// logging system shuts down cleanly. If true, it survives.
    pub persistent: bool,
    /// Extra
    pub extra: Vec<(String, String)>,
}

impl Diagnostic {
    /// Resolves the human-readable producer name via the registry, falling
    /// back to the raw code if the producer was never registered.
    pub fn source_name(&self) -> String {
        producer::lookup(&self.code.producer)
            .map(|p| p.display_name.to_string())
            .unwrap_or_else(|| self.code.producer.clone())
    }
    /// Function what returns category
    pub fn category(&self) -> &str {
        &self.code.category
    }
    /// Function what log converts into json
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
    /// Function what json converts into log
    pub fn from_json(s: &str) -> serde_json::Result<Diagnostic> {
        serde_json::from_str(s)
    }
}
/// Diagnostic Builder Struct
pub struct DiagnosticBuilder {
    code: DiagnosticCode,
    severity: Severity,
    message: String,
    language: Option<String>,
    module: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    suggestion: Option<String>,
    documentation: Option<String>,
    persistent: bool,
    extra: Vec<(String, String)>,
}

impl DiagnosticBuilder {
    /// Function what creates diagnostic builder
    pub fn new(
        producer: impl Into<String>,
        category: impl Into<String>,
        number: u32,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: DiagnosticCode::new(producer, category, number),
            severity,
            message: message.into(),
            language: None,
            module: None,
            file: None,
            line: None,
            column: None,
            suggestion: None,
            documentation: None,
            persistent: false,
            extra: Vec::new(),
        }
    }
    /// Function what changes language
    pub fn language(mut self, v: impl Into<String>) -> Self {
        self.language = Some(v.into());
        self
    }
    /// Function what changes module
    pub fn module(mut self, v: impl Into<String>) -> Self {
        self.module = Some(v.into());
        self
    }
    /// Function what changes file
    pub fn file(mut self, v: impl Into<String>) -> Self {
        self.file = Some(v.into());
        self
    }
    /// Function what changes line
    pub fn line(mut self, v: u32) -> Self {
        self.line = Some(v);
        self
    }
    /// Function what changes column
    pub fn column(mut self, v: u32) -> Self {
        self.column = Some(v);
        self
    }
    /// Function what changes suggestion
    pub fn suggestion(mut self, v: impl Into<String>) -> Self {
        self.suggestion = Some(v.into());
        self
    }
    /// Function what changes documentation
    pub fn documentation(mut self, v: impl Into<String>) -> Self {
        self.documentation = Some(v.into());
        self
    }
    /// Function what changes persistent
    pub fn persistent(mut self, v: bool) -> Self {
        self.persistent = v;
        self
    }
    /// Function what changes extra
    pub fn extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.push((key.into(), value.into()));
        self
    }
    /// Function what builds
    pub fn build(self) -> Diagnostic {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        Diagnostic {
            code: self.code,
            severity: self.severity,
            message: self.message,
            language: self.language,
            module: self.module,
            file: self.file,
            line: self.line,
            column: self.column,
            suggestion: self.suggestion,
            documentation: self.documentation,
            thread,
            timestamp_unix_ms: now,
            persistent: self.persistent,
            extra: self.extra,
        }
    }
}
