//! Keys: the physical key, the logical key, modifiers and the key event.
//!
//! A key press has two identities. The *physical* key ([`KeyCode`]) is where
//! the key sits on the keyboard, whatever the layout: `KeyCode::KeyW` is the
//! key under the left middle finger on QWERTY, AZERTY and Dvorak alike. Use it
//! for game controls (WASD). The *logical* key ([`Key`]) is what the key means
//! under the active layout and modifiers: the character it types or a named
//! key such as Enter. Use it for shortcuts and text.

use std::ops::{BitOr, BitOrAssign};

use super::button::ButtonState;

/// A physical key, named like the W3C UI Events `code` values.
///
/// The variant names are those standard values (`KeyA`, `Digit1`,
/// `ArrowLeft`, `NumpadEnter`, ...), which is why they carry no individual
/// documentation. A key the platform cannot identify is
/// [`Unidentified`](Self::Unidentified); [`InputState`](super::InputState)
/// does not track it, because it stands for many different keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
#[allow(
    missing_docs,
    reason = "the variants are the standard W3C `code` names, documented by that specification"
)]
pub enum KeyCode {
    Unidentified,
    Backquote,
    Backslash,
    BracketLeft,
    BracketRight,
    Comma,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Equal,
    IntlBackslash,
    IntlRo,
    IntlYen,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Minus,
    Period,
    Quote,
    Semicolon,
    Slash,
    AltLeft,
    AltRight,
    Backspace,
    CapsLock,
    ContextMenu,
    ControlLeft,
    ControlRight,
    Enter,
    SuperLeft,
    SuperRight,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    Convert,
    KanaMode,
    NonConvert,
    Delete,
    End,
    Help,
    Home,
    Insert,
    PageDown,
    PageUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadComma,
    NumpadDecimal,
    NumpadDivide,
    NumpadEnter,
    NumpadEqual,
    NumpadMultiply,
    NumpadSubtract,
    Escape,
    PrintScreen,
    ScrollLock,
    Pause,
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    MediaPlayPause,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

impl KeyCode {
    /// `true` for the keys that only modify others: Shift, Control, Alt and
    /// Super, left or right.
    #[must_use]
    pub const fn is_modifier(self) -> bool {
        matches!(
            self,
            Self::ShiftLeft
                | Self::ShiftRight
                | Self::ControlLeft
                | Self::ControlRight
                | Self::AltLeft
                | Self::AltRight
                | Self::SuperLeft
                | Self::SuperRight
        )
    }
}

/// A key with a name rather than a character.
///
/// Keys of the platform that have no variant here are reported as
/// [`Key::Unidentified`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
#[allow(
    missing_docs,
    reason = "the variants are the standard W3C `key` names, documented by that specification"
)]
pub enum NamedKey {
    Alt,
    AltGraph,
    CapsLock,
    Control,
    Fn,
    FnLock,
    Meta,
    NumLock,
    ScrollLock,
    Shift,
    Super,
    Enter,
    Tab,
    Space,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,
    Backspace,
    Clear,
    Copy,
    Cut,
    Delete,
    Insert,
    Paste,
    Redo,
    Undo,
    ContextMenu,
    Escape,
    Help,
    Pause,
    PrintScreen,
    /// The input method is processing the key; the result arrives as an
    /// [`ImeEvent`](super::ImeEvent).
    Process,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
}

/// The logical meaning of a key under the active layout and modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Key {
    /// A key with a name (Enter, an arrow, F5, ...).
    Named(NamedKey),
    /// A key that types this text, usually one character. Shift and the layout
    /// are applied: `"A"` and `"a"` are different keys.
    Character(String),
    /// A dead key (an accent waiting for a letter), with the accent it stands
    /// for if the platform tells.
    Dead(Option<char>),
    /// The platform could not tell what the key means.
    Unidentified,
}

impl Key {
    /// The key that types `text`.
    #[must_use]
    pub fn character(text: impl Into<String>) -> Self {
        Self::Character(text.into())
    }

    /// The typed text of a [`Character`](Self::Character) key.
    #[must_use]
    pub fn as_character(&self) -> Option<&str> {
        match self {
            Self::Character(text) => Some(text),
            _ => None,
        }
    }

    /// The name of a [`Named`](Self::Named) key.
    #[must_use]
    pub fn as_named(&self) -> Option<NamedKey> {
        match self {
            Self::Named(named) => Some(*named),
            _ => None,
        }
    }
}

/// The modifier keys that are held: a set of Shift, Control, Alt and Super
/// (the Windows or Command key). Left and right are not told apart; track
/// [`KeyCode::ShiftLeft`] and friends for that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifier.
    pub const NONE: Self = Self(0);
    /// Shift.
    pub const SHIFT: Self = Self(1);
    /// Control (Ctrl).
    pub const CONTROL: Self = Self(1 << 1);
    /// Alt (Option on macOS).
    pub const ALT: Self = Self(1 << 2);
    /// The Windows key, or Command on macOS.
    pub const SUPER: Self = Self(1 << 3);

    /// `true` when every modifier of `other` is held.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// `true` when no modifier is held.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// `true` while Shift is held.
    #[must_use]
    pub const fn shift(self) -> bool {
        self.contains(Self::SHIFT)
    }

    /// `true` while Control is held.
    #[must_use]
    pub const fn control(self) -> bool {
        self.contains(Self::CONTROL)
    }

    /// `true` while Alt is held.
    #[must_use]
    pub const fn alt(self) -> bool {
        self.contains(Self::ALT)
    }

    /// `true` while Super is held.
    #[must_use]
    pub const fn super_key(self) -> bool {
        self.contains(Self::SUPER)
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// A key going down or up.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeyEvent {
    /// Where the key is on the keyboard, independent of the layout.
    pub physical_key: KeyCode,
    /// What the key means under the active layout and modifiers.
    pub logical_key: Key,
    /// The text the press produces (with Shift and the layout applied), if
    /// any. Key repeat produces it again. Enter, Tab and Backspace report
    /// their control characters here; filter with `char::is_control` if you
    /// want printable text only.
    pub text: Option<String>,
    /// Whether the key went down or up.
    pub state: ButtonState,
    /// `true` for the auto-repeat of a key that is held.
    pub repeat: bool,
    /// `true` when the system made the event up, for example the release of a
    /// key that was still held when the window lost focus.
    pub synthetic: bool,
}

impl KeyEvent {
    /// A key event without text, repeat or synthetic flag.
    #[must_use]
    pub fn new(physical_key: KeyCode, logical_key: Key, state: ButtonState) -> Self {
        Self {
            physical_key,
            logical_key,
            text: None,
            state,
            repeat: false,
            synthetic: false,
        }
    }

    /// Sets the text the event produces.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Marks the event as an auto-repeat.
    #[must_use]
    pub fn with_repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }

    /// Marks the event as made up by the system.
    #[must_use]
    pub fn with_synthetic(mut self, synthetic: bool) -> Self {
        self.synthetic = synthetic;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_combine_and_are_queried() {
        let m = Modifiers::SHIFT | Modifiers::CONTROL;
        assert!(m.shift() && m.control());
        assert!(!m.alt() && !m.super_key());
        assert!(m.contains(Modifiers::SHIFT));
        assert!(!m.contains(Modifiers::SHIFT | Modifiers::ALT));
        assert!(Modifiers::NONE.is_empty() && Modifiers::default().is_empty());
        let mut n = Modifiers::NONE;
        n |= Modifiers::SUPER;
        assert!(n.super_key() && !n.is_empty());
    }

    #[test]
    fn modifier_keys_are_recognised() {
        assert!(KeyCode::ShiftRight.is_modifier());
        assert!(KeyCode::SuperLeft.is_modifier());
        assert!(!KeyCode::KeyA.is_modifier());
        assert!(!KeyCode::CapsLock.is_modifier());
    }

    #[test]
    fn logical_keys_expose_their_payload() {
        let a = Key::character("a");
        assert_eq!(a.as_character(), Some("a"));
        assert_eq!(a.as_named(), None);
        let enter = Key::Named(NamedKey::Enter);
        assert_eq!(enter.as_named(), Some(NamedKey::Enter));
        assert_eq!(enter.as_character(), None);
        assert_ne!(Key::character("A"), Key::character("a"));
    }

    #[test]
    fn key_events_are_built_step_by_step() {
        let e = KeyEvent::new(KeyCode::KeyA, Key::character("a"), ButtonState::Pressed)
            .with_text("a")
            .with_repeat(true)
            .with_synthetic(true);
        assert_eq!(e.physical_key, KeyCode::KeyA);
        assert_eq!(e.text.as_deref(), Some("a"));
        assert!(e.repeat && e.synthetic && e.state.is_pressed());
        let plain = KeyEvent::new(
            KeyCode::Escape,
            Key::Named(NamedKey::Escape),
            ButtonState::Released,
        );
        assert!(plain.text.is_none() && !plain.repeat && !plain.synthetic);
    }
}
