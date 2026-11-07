//! Log text must not be able to forge log lines or inject terminal escapes.

use log_core::format::{escape_control, format_colored, format_line, format_plain};
use log_core::{DiagnosticBuilder, Severity};

fn diag(message: &str) -> log_core::Diagnostic {
    DiagnosticBuilder::new("CV", "TEST", 1, Severity::Warning, message).build()
}

#[test]
fn escape_control_leaves_clean_text_untouched() {
    assert!(matches!(
        escape_control("plain text\twith a tab", false),
        std::borrow::Cow::Borrowed(_)
    ));
    assert_eq!(escape_control("héllo wörld ✓", false), "héllo wörld ✓");
}

#[test]
fn escape_control_makes_control_characters_visible() {
    assert_eq!(escape_control("a\nb\r\nc", false), "a\\nb\\r\\nc");
    assert_eq!(escape_control("\u{1b}[31mred", false), "\\x1b[31mred");
    assert_eq!(escape_control("x\u{7f}y\u{0}z", false), "x\\x7fy\\x00z");
    assert_eq!(escape_control("next\u{85}line", false), "next\\u{85}line");

    // Newlines survive for multi-line console output, but a bare CR (cursor
    // return) and escapes do not.
    assert_eq!(escape_control("a\nb", true), "a\nb");
    assert_eq!(escape_control("a\rb\u{1b}", true), "a\\rb\\x1b");
}

#[test]
fn a_log_line_cannot_be_split_by_its_message() {
    let forged = diag("done\n[1] [CV] [Error] [CV-TEST-9999] fake entry");
    let line = format_line(&forged);
    assert_eq!(line.lines().count(), 1, "{line:?}");
    assert!(line.contains("\\n[1] [CV]"), "{line}");
}

#[test]
fn console_output_cannot_carry_terminal_escapes() {
    let hostile = diag("\u{1b}]0;owned\u{7}\u{1b}[2J");
    for text in [format_plain(&hostile), format_colored(&hostile)] {
        assert!(!text.contains('\u{7}'), "{text:?}");
        // The only ESC bytes allowed are the sink's own colour codes.
        let message_part = text.split('\n').nth(1).unwrap();
        assert!(!message_part.contains('\u{1b}'), "{message_part:?}");
    }
}
