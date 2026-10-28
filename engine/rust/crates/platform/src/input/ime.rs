//! Input-method (IME) events.

/// What an input method editor reports while the user composes text, for
/// example Chinese, Japanese or Korean.
///
/// IME events are only delivered while the window allows them, see
/// [`AppContext::set_ime_allowed`](crate::window::AppContext::set_ime_allowed).
/// Text typed without composition arrives as [`KeyEvent::text`](super::KeyEvent::text).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImeEvent {
    /// Composition became available: show the preedit text when it comes.
    Enabled,
    /// The text being composed changed. An empty `text` clears the preedit.
    Preedit {
        /// The text being composed, not yet part of the document.
        text: String,
        /// The caret or selection inside `text` as byte offsets, if the input
        /// method shows one.
        cursor: Option<(usize, usize)>,
    },
    /// Composition finished: insert this text. Any preedit is cleared first.
    Commit(String),
    /// Composition is no longer available.
    Disabled,
}
