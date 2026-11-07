use std::borrow::Cow;

use crate::diagnostic::Diagnostic;

/// Plain-text rendering.md, e.g.:
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
        escape_control(&diag.source_name(), false),
        diag.severity.as_str(),
        diag.code,
        escape_control(&diag.message, true)
    );
    if let Some(s) = &diag.suggestion {
        out.push_str(&format!("\n-> {}", escape_control(s, true)));
    }
    if let Some(d) = &diag.documentation {
        out.push_str(&format!("\n-> {}", escape_control(d, true)));
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
        escape_control(&diag.source_name(), false),
        diag.severity.as_str(),
        diag.code,
        escape_control(&diag.message, true)
    );
    if let Some(s) = &diag.suggestion {
        out.push_str(&format!("\n{color}->{RESET} {}", escape_control(s, true)));
    }
    if let Some(d) = &diag.documentation {
        out.push_str(&format!("\n{color}->{RESET} {}", escape_control(d, true)));
    }
    out
}

/// Compact single-line rendering.md, useful for log files that get grepped.
pub fn format_line(diag: &Diagnostic) -> String {
    format!(
        "[{}] [{}] [{}] [{}] {}",
        diag.timestamp_unix_ms,
        escape_control(&diag.source_name(), false),
        diag.severity.as_str(),
        diag.code,
        escape_control(&diag.message, false)
    )
}

/// Makes `text` safe to print or write to a log.
///
/// Messages can carry data the engine does not control (file names, script
/// output, FFI strings). Left as is, a `\n` forges extra log lines (so a
/// one-line-per-entry file can be made to contain entries that never
/// happened) and an ESC byte injects terminal control sequences when the
/// text is shown in a console. Control characters are therefore replaced by
/// a visible escape (`\n`, `\x1b`, `\u{85}`); tabs always stay, and
/// newlines stay when `keep_newlines` is set (multi-line console output).
pub fn escape_control(text: &str, keep_newlines: bool) -> Cow<'_, str> {
    let needs_escape = |c: char| match c {
        '\t' => false,
        '\n' => !keep_newlines,
        c => c.is_control(),
    };
    if !text.chars().any(needs_escape) {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        if !needs_escape(c) {
            out.push(c);
            continue;
        }
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x80 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
        }
    }
    Cow::Owned(out)
}
