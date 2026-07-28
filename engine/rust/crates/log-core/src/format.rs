use crate::diagnostic::Diagnostic;

/// Plain-text rendering, e.g.:
///
/// ```text
/// [Coreverse Engine] [Warning] [CV-ASSET-0001]
/// Asset "example.png" is not referenced.
/// -> Remove the asset or reference it.
/// -> https://docs.coreverse.dev/errors/CV-ASSET-0001
/// ```
pub fn format_plain(diag: &Diagnostic) -> String {
    let mut out = format!(
        "[{}] [{}] [{}]\n{}",
        diag.source_name(),
        diag.severity.as_str(),
        diag.code,
        diag.message
    );
    if let Some(s) = &diag.suggestion {
        out.push_str(&format!("\n-> {s}"));
    }
    if let Some(d) = &diag.documentation {
        out.push_str(&format!("\n-> {d}"));
    }
    out
}

/// Same as `format_plain` but with ANSI color codes for terminal output.
pub fn format_colored(diag: &Diagnostic) -> String {
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";
    let color = diag.severity.ansi_color();
    let mut out = format!(
        "{color}{BOLD}[{}] [{}] [{}]{RESET}\n{}",
        diag.source_name(),
        diag.severity.as_str(),
        diag.code,
        diag.message
    );
    if let Some(s) = &diag.suggestion {
        out.push_str(&format!("\n{color}->{RESET} {s}"));
    }
    if let Some(d) = &diag.documentation {
        out.push_str(&format!("\n{color}->{RESET} {d}"));
    }
    out
}

/// Compact single-line rendering, useful for log files that get grepped.
pub fn format_line(diag: &Diagnostic) -> String {
    format!(
        "[{}] [{}] [{}] [{}] {}",
        diag.timestamp_unix_ms,
        diag.source_name(),
        diag.severity.as_str(),
        diag.code,
        diag.message
    )
}
