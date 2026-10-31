//! Numbers for keys on the C side.
//!
//! The physical key ([`KeyCode`]) and the named logical key ([`NamedKey`])
//! cross the boundary as `u16` values from the `FFI_KEY_*` and
//! `FFI_NAMED_KEY_*` constants below, which cbindgen copies into the header.
//! The numbers are assigned in the order of the `platform` enums at the time
//! of writing and are part of the ABI from now on: a key added to the `platform`
//! crate later gets the next free number, and one this crate does not know yet
//! is reported as [`FFI_KEY_UNKNOWN`] / [`FFI_NAMED_KEY_UNKNOWN`] (`65535`), so
//! the enums can grow without breaking an older C++ caller.
//!
//! The C++ side mirrors them as `cv_ffi::platform::KeyCode` and `NamedKey` and
//! checks every value against these constants with `static_assert`.
//!
//! This file is generated from `platform/src/input/key.rs`; the tests keep the
//! tables complete and free of duplicates.

use platform_crate::input::{KeyCode, NamedKey};

/// Physical key `Unidentified`.
pub const FFI_KEY_UNIDENTIFIED: u16 = 0;
/// Physical key `Backquote`.
pub const FFI_KEY_BACKQUOTE: u16 = 1;
/// Physical key `Backslash`.
pub const FFI_KEY_BACKSLASH: u16 = 2;
/// Physical key `BracketLeft`.
pub const FFI_KEY_BRACKET_LEFT: u16 = 3;
/// Physical key `BracketRight`.
pub const FFI_KEY_BRACKET_RIGHT: u16 = 4;
/// Physical key `Comma`.
pub const FFI_KEY_COMMA: u16 = 5;
/// Physical key `Digit0`.
pub const FFI_KEY_DIGIT_0: u16 = 6;
/// Physical key `Digit1`.
pub const FFI_KEY_DIGIT_1: u16 = 7;
/// Physical key `Digit2`.
pub const FFI_KEY_DIGIT_2: u16 = 8;
/// Physical key `Digit3`.
pub const FFI_KEY_DIGIT_3: u16 = 9;
/// Physical key `Digit4`.
pub const FFI_KEY_DIGIT_4: u16 = 10;
/// Physical key `Digit5`.
pub const FFI_KEY_DIGIT_5: u16 = 11;
/// Physical key `Digit6`.
pub const FFI_KEY_DIGIT_6: u16 = 12;
/// Physical key `Digit7`.
pub const FFI_KEY_DIGIT_7: u16 = 13;
/// Physical key `Digit8`.
pub const FFI_KEY_DIGIT_8: u16 = 14;
/// Physical key `Digit9`.
pub const FFI_KEY_DIGIT_9: u16 = 15;
/// Physical key `Equal`.
pub const FFI_KEY_EQUAL: u16 = 16;
/// Physical key `IntlBackslash`.
pub const FFI_KEY_INTL_BACKSLASH: u16 = 17;
/// Physical key `IntlRo`.
pub const FFI_KEY_INTL_RO: u16 = 18;
/// Physical key `IntlYen`.
pub const FFI_KEY_INTL_YEN: u16 = 19;
/// Physical key `KeyA`.
pub const FFI_KEY_KEY_A: u16 = 20;
/// Physical key `KeyB`.
pub const FFI_KEY_KEY_B: u16 = 21;
/// Physical key `KeyC`.
pub const FFI_KEY_KEY_C: u16 = 22;
/// Physical key `KeyD`.
pub const FFI_KEY_KEY_D: u16 = 23;
/// Physical key `KeyE`.
pub const FFI_KEY_KEY_E: u16 = 24;
/// Physical key `KeyF`.
pub const FFI_KEY_KEY_F: u16 = 25;
/// Physical key `KeyG`.
pub const FFI_KEY_KEY_G: u16 = 26;
/// Physical key `KeyH`.
pub const FFI_KEY_KEY_H: u16 = 27;
/// Physical key `KeyI`.
pub const FFI_KEY_KEY_I: u16 = 28;
/// Physical key `KeyJ`.
pub const FFI_KEY_KEY_J: u16 = 29;
/// Physical key `KeyK`.
pub const FFI_KEY_KEY_K: u16 = 30;
/// Physical key `KeyL`.
pub const FFI_KEY_KEY_L: u16 = 31;
/// Physical key `KeyM`.
pub const FFI_KEY_KEY_M: u16 = 32;
/// Physical key `KeyN`.
pub const FFI_KEY_KEY_N: u16 = 33;
/// Physical key `KeyO`.
pub const FFI_KEY_KEY_O: u16 = 34;
/// Physical key `KeyP`.
pub const FFI_KEY_KEY_P: u16 = 35;
/// Physical key `KeyQ`.
pub const FFI_KEY_KEY_Q: u16 = 36;
/// Physical key `KeyR`.
pub const FFI_KEY_KEY_R: u16 = 37;
/// Physical key `KeyS`.
pub const FFI_KEY_KEY_S: u16 = 38;
/// Physical key `KeyT`.
pub const FFI_KEY_KEY_T: u16 = 39;
/// Physical key `KeyU`.
pub const FFI_KEY_KEY_U: u16 = 40;
/// Physical key `KeyV`.
pub const FFI_KEY_KEY_V: u16 = 41;
/// Physical key `KeyW`.
pub const FFI_KEY_KEY_W: u16 = 42;
/// Physical key `KeyX`.
pub const FFI_KEY_KEY_X: u16 = 43;
/// Physical key `KeyY`.
pub const FFI_KEY_KEY_Y: u16 = 44;
/// Physical key `KeyZ`.
pub const FFI_KEY_KEY_Z: u16 = 45;
/// Physical key `Minus`.
pub const FFI_KEY_MINUS: u16 = 46;
/// Physical key `Period`.
pub const FFI_KEY_PERIOD: u16 = 47;
/// Physical key `Quote`.
pub const FFI_KEY_QUOTE: u16 = 48;
/// Physical key `Semicolon`.
pub const FFI_KEY_SEMICOLON: u16 = 49;
/// Physical key `Slash`.
pub const FFI_KEY_SLASH: u16 = 50;
/// Physical key `AltLeft`.
pub const FFI_KEY_ALT_LEFT: u16 = 51;
/// Physical key `AltRight`.
pub const FFI_KEY_ALT_RIGHT: u16 = 52;
/// Physical key `Backspace`.
pub const FFI_KEY_BACKSPACE: u16 = 53;
/// Physical key `CapsLock`.
pub const FFI_KEY_CAPS_LOCK: u16 = 54;
/// Physical key `ContextMenu`.
pub const FFI_KEY_CONTEXT_MENU: u16 = 55;
/// Physical key `ControlLeft`.
pub const FFI_KEY_CONTROL_LEFT: u16 = 56;
/// Physical key `ControlRight`.
pub const FFI_KEY_CONTROL_RIGHT: u16 = 57;
/// Physical key `Enter`.
pub const FFI_KEY_ENTER: u16 = 58;
/// Physical key `SuperLeft`.
pub const FFI_KEY_SUPER_LEFT: u16 = 59;
/// Physical key `SuperRight`.
pub const FFI_KEY_SUPER_RIGHT: u16 = 60;
/// Physical key `ShiftLeft`.
pub const FFI_KEY_SHIFT_LEFT: u16 = 61;
/// Physical key `ShiftRight`.
pub const FFI_KEY_SHIFT_RIGHT: u16 = 62;
/// Physical key `Space`.
pub const FFI_KEY_SPACE: u16 = 63;
/// Physical key `Tab`.
pub const FFI_KEY_TAB: u16 = 64;
/// Physical key `Convert`.
pub const FFI_KEY_CONVERT: u16 = 65;
/// Physical key `KanaMode`.
pub const FFI_KEY_KANA_MODE: u16 = 66;
/// Physical key `NonConvert`.
pub const FFI_KEY_NON_CONVERT: u16 = 67;
/// Physical key `Delete`.
pub const FFI_KEY_DELETE: u16 = 68;
/// Physical key `End`.
pub const FFI_KEY_END: u16 = 69;
/// Physical key `Help`.
pub const FFI_KEY_HELP: u16 = 70;
/// Physical key `Home`.
pub const FFI_KEY_HOME: u16 = 71;
/// Physical key `Insert`.
pub const FFI_KEY_INSERT: u16 = 72;
/// Physical key `PageDown`.
pub const FFI_KEY_PAGE_DOWN: u16 = 73;
/// Physical key `PageUp`.
pub const FFI_KEY_PAGE_UP: u16 = 74;
/// Physical key `ArrowDown`.
pub const FFI_KEY_ARROW_DOWN: u16 = 75;
/// Physical key `ArrowLeft`.
pub const FFI_KEY_ARROW_LEFT: u16 = 76;
/// Physical key `ArrowRight`.
pub const FFI_KEY_ARROW_RIGHT: u16 = 77;
/// Physical key `ArrowUp`.
pub const FFI_KEY_ARROW_UP: u16 = 78;
/// Physical key `NumLock`.
pub const FFI_KEY_NUM_LOCK: u16 = 79;
/// Physical key `Numpad0`.
pub const FFI_KEY_NUMPAD_0: u16 = 80;
/// Physical key `Numpad1`.
pub const FFI_KEY_NUMPAD_1: u16 = 81;
/// Physical key `Numpad2`.
pub const FFI_KEY_NUMPAD_2: u16 = 82;
/// Physical key `Numpad3`.
pub const FFI_KEY_NUMPAD_3: u16 = 83;
/// Physical key `Numpad4`.
pub const FFI_KEY_NUMPAD_4: u16 = 84;
/// Physical key `Numpad5`.
pub const FFI_KEY_NUMPAD_5: u16 = 85;
/// Physical key `Numpad6`.
pub const FFI_KEY_NUMPAD_6: u16 = 86;
/// Physical key `Numpad7`.
pub const FFI_KEY_NUMPAD_7: u16 = 87;
/// Physical key `Numpad8`.
pub const FFI_KEY_NUMPAD_8: u16 = 88;
/// Physical key `Numpad9`.
pub const FFI_KEY_NUMPAD_9: u16 = 89;
/// Physical key `NumpadAdd`.
pub const FFI_KEY_NUMPAD_ADD: u16 = 90;
/// Physical key `NumpadComma`.
pub const FFI_KEY_NUMPAD_COMMA: u16 = 91;
/// Physical key `NumpadDecimal`.
pub const FFI_KEY_NUMPAD_DECIMAL: u16 = 92;
/// Physical key `NumpadDivide`.
pub const FFI_KEY_NUMPAD_DIVIDE: u16 = 93;
/// Physical key `NumpadEnter`.
pub const FFI_KEY_NUMPAD_ENTER: u16 = 94;
/// Physical key `NumpadEqual`.
pub const FFI_KEY_NUMPAD_EQUAL: u16 = 95;
/// Physical key `NumpadMultiply`.
pub const FFI_KEY_NUMPAD_MULTIPLY: u16 = 96;
/// Physical key `NumpadSubtract`.
pub const FFI_KEY_NUMPAD_SUBTRACT: u16 = 97;
/// Physical key `Escape`.
pub const FFI_KEY_ESCAPE: u16 = 98;
/// Physical key `PrintScreen`.
pub const FFI_KEY_PRINT_SCREEN: u16 = 99;
/// Physical key `ScrollLock`.
pub const FFI_KEY_SCROLL_LOCK: u16 = 100;
/// Physical key `Pause`.
pub const FFI_KEY_PAUSE: u16 = 101;
/// Physical key `AudioVolumeDown`.
pub const FFI_KEY_AUDIO_VOLUME_DOWN: u16 = 102;
/// Physical key `AudioVolumeMute`.
pub const FFI_KEY_AUDIO_VOLUME_MUTE: u16 = 103;
/// Physical key `AudioVolumeUp`.
pub const FFI_KEY_AUDIO_VOLUME_UP: u16 = 104;
/// Physical key `MediaPlayPause`.
pub const FFI_KEY_MEDIA_PLAY_PAUSE: u16 = 105;
/// Physical key `MediaStop`.
pub const FFI_KEY_MEDIA_STOP: u16 = 106;
/// Physical key `MediaTrackNext`.
pub const FFI_KEY_MEDIA_TRACK_NEXT: u16 = 107;
/// Physical key `MediaTrackPrevious`.
pub const FFI_KEY_MEDIA_TRACK_PREVIOUS: u16 = 108;
/// Physical key `F1`.
pub const FFI_KEY_F1: u16 = 109;
/// Physical key `F2`.
pub const FFI_KEY_F2: u16 = 110;
/// Physical key `F3`.
pub const FFI_KEY_F3: u16 = 111;
/// Physical key `F4`.
pub const FFI_KEY_F4: u16 = 112;
/// Physical key `F5`.
pub const FFI_KEY_F5: u16 = 113;
/// Physical key `F6`.
pub const FFI_KEY_F6: u16 = 114;
/// Physical key `F7`.
pub const FFI_KEY_F7: u16 = 115;
/// Physical key `F8`.
pub const FFI_KEY_F8: u16 = 116;
/// Physical key `F9`.
pub const FFI_KEY_F9: u16 = 117;
/// Physical key `F10`.
pub const FFI_KEY_F10: u16 = 118;
/// Physical key `F11`.
pub const FFI_KEY_F11: u16 = 119;
/// Physical key `F12`.
pub const FFI_KEY_F12: u16 = 120;
/// Physical key `F13`.
pub const FFI_KEY_F13: u16 = 121;
/// Physical key `F14`.
pub const FFI_KEY_F14: u16 = 122;
/// Physical key `F15`.
pub const FFI_KEY_F15: u16 = 123;
/// Physical key `F16`.
pub const FFI_KEY_F16: u16 = 124;
/// Physical key `F17`.
pub const FFI_KEY_F17: u16 = 125;
/// Physical key `F18`.
pub const FFI_KEY_F18: u16 = 126;
/// Physical key `F19`.
pub const FFI_KEY_F19: u16 = 127;
/// Physical key `F20`.
pub const FFI_KEY_F20: u16 = 128;
/// Physical key `F21`.
pub const FFI_KEY_F21: u16 = 129;
/// Physical key `F22`.
pub const FFI_KEY_F22: u16 = 130;
/// Physical key `F23`.
pub const FFI_KEY_F23: u16 = 131;
/// Physical key `F24`.
pub const FFI_KEY_F24: u16 = 132;
/// A physical key this header does not know.
pub const FFI_KEY_UNKNOWN: u16 = 65535;

/// Named logical key `Alt`.
pub const FFI_NAMED_KEY_ALT: u16 = 0;
/// Named logical key `AltGraph`.
pub const FFI_NAMED_KEY_ALT_GRAPH: u16 = 1;
/// Named logical key `CapsLock`.
pub const FFI_NAMED_KEY_CAPS_LOCK: u16 = 2;
/// Named logical key `Control`.
pub const FFI_NAMED_KEY_CONTROL: u16 = 3;
/// Named logical key `Fn`.
pub const FFI_NAMED_KEY_FN: u16 = 4;
/// Named logical key `FnLock`.
pub const FFI_NAMED_KEY_FN_LOCK: u16 = 5;
/// Named logical key `Meta`.
pub const FFI_NAMED_KEY_META: u16 = 6;
/// Named logical key `NumLock`.
pub const FFI_NAMED_KEY_NUM_LOCK: u16 = 7;
/// Named logical key `ScrollLock`.
pub const FFI_NAMED_KEY_SCROLL_LOCK: u16 = 8;
/// Named logical key `Shift`.
pub const FFI_NAMED_KEY_SHIFT: u16 = 9;
/// Named logical key `Super`.
pub const FFI_NAMED_KEY_SUPER: u16 = 10;
/// Named logical key `Enter`.
pub const FFI_NAMED_KEY_ENTER: u16 = 11;
/// Named logical key `Tab`.
pub const FFI_NAMED_KEY_TAB: u16 = 12;
/// Named logical key `Space`.
pub const FFI_NAMED_KEY_SPACE: u16 = 13;
/// Named logical key `ArrowDown`.
pub const FFI_NAMED_KEY_ARROW_DOWN: u16 = 14;
/// Named logical key `ArrowLeft`.
pub const FFI_NAMED_KEY_ARROW_LEFT: u16 = 15;
/// Named logical key `ArrowRight`.
pub const FFI_NAMED_KEY_ARROW_RIGHT: u16 = 16;
/// Named logical key `ArrowUp`.
pub const FFI_NAMED_KEY_ARROW_UP: u16 = 17;
/// Named logical key `End`.
pub const FFI_NAMED_KEY_END: u16 = 18;
/// Named logical key `Home`.
pub const FFI_NAMED_KEY_HOME: u16 = 19;
/// Named logical key `PageDown`.
pub const FFI_NAMED_KEY_PAGE_DOWN: u16 = 20;
/// Named logical key `PageUp`.
pub const FFI_NAMED_KEY_PAGE_UP: u16 = 21;
/// Named logical key `Backspace`.
pub const FFI_NAMED_KEY_BACKSPACE: u16 = 22;
/// Named logical key `Clear`.
pub const FFI_NAMED_KEY_CLEAR: u16 = 23;
/// Named logical key `Copy`.
pub const FFI_NAMED_KEY_COPY: u16 = 24;
/// Named logical key `Cut`.
pub const FFI_NAMED_KEY_CUT: u16 = 25;
/// Named logical key `Delete`.
pub const FFI_NAMED_KEY_DELETE: u16 = 26;
/// Named logical key `Insert`.
pub const FFI_NAMED_KEY_INSERT: u16 = 27;
/// Named logical key `Paste`.
pub const FFI_NAMED_KEY_PASTE: u16 = 28;
/// Named logical key `Redo`.
pub const FFI_NAMED_KEY_REDO: u16 = 29;
/// Named logical key `Undo`.
pub const FFI_NAMED_KEY_UNDO: u16 = 30;
/// Named logical key `ContextMenu`.
pub const FFI_NAMED_KEY_CONTEXT_MENU: u16 = 31;
/// Named logical key `Escape`.
pub const FFI_NAMED_KEY_ESCAPE: u16 = 32;
/// Named logical key `Help`.
pub const FFI_NAMED_KEY_HELP: u16 = 33;
/// Named logical key `Pause`.
pub const FFI_NAMED_KEY_PAUSE: u16 = 34;
/// Named logical key `PrintScreen`.
pub const FFI_NAMED_KEY_PRINT_SCREEN: u16 = 35;
/// Named logical key `Process`.
pub const FFI_NAMED_KEY_PROCESS: u16 = 36;
/// Named logical key `F1`.
pub const FFI_NAMED_KEY_F1: u16 = 37;
/// Named logical key `F2`.
pub const FFI_NAMED_KEY_F2: u16 = 38;
/// Named logical key `F3`.
pub const FFI_NAMED_KEY_F3: u16 = 39;
/// Named logical key `F4`.
pub const FFI_NAMED_KEY_F4: u16 = 40;
/// Named logical key `F5`.
pub const FFI_NAMED_KEY_F5: u16 = 41;
/// Named logical key `F6`.
pub const FFI_NAMED_KEY_F6: u16 = 42;
/// Named logical key `F7`.
pub const FFI_NAMED_KEY_F7: u16 = 43;
/// Named logical key `F8`.
pub const FFI_NAMED_KEY_F8: u16 = 44;
/// Named logical key `F9`.
pub const FFI_NAMED_KEY_F9: u16 = 45;
/// Named logical key `F10`.
pub const FFI_NAMED_KEY_F10: u16 = 46;
/// Named logical key `F11`.
pub const FFI_NAMED_KEY_F11: u16 = 47;
/// Named logical key `F12`.
pub const FFI_NAMED_KEY_F12: u16 = 48;
/// Named logical key `F13`.
pub const FFI_NAMED_KEY_F13: u16 = 49;
/// Named logical key `F14`.
pub const FFI_NAMED_KEY_F14: u16 = 50;
/// Named logical key `F15`.
pub const FFI_NAMED_KEY_F15: u16 = 51;
/// Named logical key `F16`.
pub const FFI_NAMED_KEY_F16: u16 = 52;
/// Named logical key `F17`.
pub const FFI_NAMED_KEY_F17: u16 = 53;
/// Named logical key `F18`.
pub const FFI_NAMED_KEY_F18: u16 = 54;
/// Named logical key `F19`.
pub const FFI_NAMED_KEY_F19: u16 = 55;
/// Named logical key `F20`.
pub const FFI_NAMED_KEY_F20: u16 = 56;
/// Named logical key `F21`.
pub const FFI_NAMED_KEY_F21: u16 = 57;
/// Named logical key `F22`.
pub const FFI_NAMED_KEY_F22: u16 = 58;
/// Named logical key `F23`.
pub const FFI_NAMED_KEY_F23: u16 = 59;
/// Named logical key `F24`.
pub const FFI_NAMED_KEY_F24: u16 = 60;
/// A named logical key this header does not know.
pub const FFI_NAMED_KEY_UNKNOWN: u16 = 65535;

/// The number of a physical key.
#[must_use]
pub const fn key_code(code: KeyCode) -> u16 {
    match code {
        KeyCode::Unidentified => FFI_KEY_UNIDENTIFIED,
        KeyCode::Backquote => FFI_KEY_BACKQUOTE,
        KeyCode::Backslash => FFI_KEY_BACKSLASH,
        KeyCode::BracketLeft => FFI_KEY_BRACKET_LEFT,
        KeyCode::BracketRight => FFI_KEY_BRACKET_RIGHT,
        KeyCode::Comma => FFI_KEY_COMMA,
        KeyCode::Digit0 => FFI_KEY_DIGIT_0,
        KeyCode::Digit1 => FFI_KEY_DIGIT_1,
        KeyCode::Digit2 => FFI_KEY_DIGIT_2,
        KeyCode::Digit3 => FFI_KEY_DIGIT_3,
        KeyCode::Digit4 => FFI_KEY_DIGIT_4,
        KeyCode::Digit5 => FFI_KEY_DIGIT_5,
        KeyCode::Digit6 => FFI_KEY_DIGIT_6,
        KeyCode::Digit7 => FFI_KEY_DIGIT_7,
        KeyCode::Digit8 => FFI_KEY_DIGIT_8,
        KeyCode::Digit9 => FFI_KEY_DIGIT_9,
        KeyCode::Equal => FFI_KEY_EQUAL,
        KeyCode::IntlBackslash => FFI_KEY_INTL_BACKSLASH,
        KeyCode::IntlRo => FFI_KEY_INTL_RO,
        KeyCode::IntlYen => FFI_KEY_INTL_YEN,
        KeyCode::KeyA => FFI_KEY_KEY_A,
        KeyCode::KeyB => FFI_KEY_KEY_B,
        KeyCode::KeyC => FFI_KEY_KEY_C,
        KeyCode::KeyD => FFI_KEY_KEY_D,
        KeyCode::KeyE => FFI_KEY_KEY_E,
        KeyCode::KeyF => FFI_KEY_KEY_F,
        KeyCode::KeyG => FFI_KEY_KEY_G,
        KeyCode::KeyH => FFI_KEY_KEY_H,
        KeyCode::KeyI => FFI_KEY_KEY_I,
        KeyCode::KeyJ => FFI_KEY_KEY_J,
        KeyCode::KeyK => FFI_KEY_KEY_K,
        KeyCode::KeyL => FFI_KEY_KEY_L,
        KeyCode::KeyM => FFI_KEY_KEY_M,
        KeyCode::KeyN => FFI_KEY_KEY_N,
        KeyCode::KeyO => FFI_KEY_KEY_O,
        KeyCode::KeyP => FFI_KEY_KEY_P,
        KeyCode::KeyQ => FFI_KEY_KEY_Q,
        KeyCode::KeyR => FFI_KEY_KEY_R,
        KeyCode::KeyS => FFI_KEY_KEY_S,
        KeyCode::KeyT => FFI_KEY_KEY_T,
        KeyCode::KeyU => FFI_KEY_KEY_U,
        KeyCode::KeyV => FFI_KEY_KEY_V,
        KeyCode::KeyW => FFI_KEY_KEY_W,
        KeyCode::KeyX => FFI_KEY_KEY_X,
        KeyCode::KeyY => FFI_KEY_KEY_Y,
        KeyCode::KeyZ => FFI_KEY_KEY_Z,
        KeyCode::Minus => FFI_KEY_MINUS,
        KeyCode::Period => FFI_KEY_PERIOD,
        KeyCode::Quote => FFI_KEY_QUOTE,
        KeyCode::Semicolon => FFI_KEY_SEMICOLON,
        KeyCode::Slash => FFI_KEY_SLASH,
        KeyCode::AltLeft => FFI_KEY_ALT_LEFT,
        KeyCode::AltRight => FFI_KEY_ALT_RIGHT,
        KeyCode::Backspace => FFI_KEY_BACKSPACE,
        KeyCode::CapsLock => FFI_KEY_CAPS_LOCK,
        KeyCode::ContextMenu => FFI_KEY_CONTEXT_MENU,
        KeyCode::ControlLeft => FFI_KEY_CONTROL_LEFT,
        KeyCode::ControlRight => FFI_KEY_CONTROL_RIGHT,
        KeyCode::Enter => FFI_KEY_ENTER,
        KeyCode::SuperLeft => FFI_KEY_SUPER_LEFT,
        KeyCode::SuperRight => FFI_KEY_SUPER_RIGHT,
        KeyCode::ShiftLeft => FFI_KEY_SHIFT_LEFT,
        KeyCode::ShiftRight => FFI_KEY_SHIFT_RIGHT,
        KeyCode::Space => FFI_KEY_SPACE,
        KeyCode::Tab => FFI_KEY_TAB,
        KeyCode::Convert => FFI_KEY_CONVERT,
        KeyCode::KanaMode => FFI_KEY_KANA_MODE,
        KeyCode::NonConvert => FFI_KEY_NON_CONVERT,
        KeyCode::Delete => FFI_KEY_DELETE,
        KeyCode::End => FFI_KEY_END,
        KeyCode::Help => FFI_KEY_HELP,
        KeyCode::Home => FFI_KEY_HOME,
        KeyCode::Insert => FFI_KEY_INSERT,
        KeyCode::PageDown => FFI_KEY_PAGE_DOWN,
        KeyCode::PageUp => FFI_KEY_PAGE_UP,
        KeyCode::ArrowDown => FFI_KEY_ARROW_DOWN,
        KeyCode::ArrowLeft => FFI_KEY_ARROW_LEFT,
        KeyCode::ArrowRight => FFI_KEY_ARROW_RIGHT,
        KeyCode::ArrowUp => FFI_KEY_ARROW_UP,
        KeyCode::NumLock => FFI_KEY_NUM_LOCK,
        KeyCode::Numpad0 => FFI_KEY_NUMPAD_0,
        KeyCode::Numpad1 => FFI_KEY_NUMPAD_1,
        KeyCode::Numpad2 => FFI_KEY_NUMPAD_2,
        KeyCode::Numpad3 => FFI_KEY_NUMPAD_3,
        KeyCode::Numpad4 => FFI_KEY_NUMPAD_4,
        KeyCode::Numpad5 => FFI_KEY_NUMPAD_5,
        KeyCode::Numpad6 => FFI_KEY_NUMPAD_6,
        KeyCode::Numpad7 => FFI_KEY_NUMPAD_7,
        KeyCode::Numpad8 => FFI_KEY_NUMPAD_8,
        KeyCode::Numpad9 => FFI_KEY_NUMPAD_9,
        KeyCode::NumpadAdd => FFI_KEY_NUMPAD_ADD,
        KeyCode::NumpadComma => FFI_KEY_NUMPAD_COMMA,
        KeyCode::NumpadDecimal => FFI_KEY_NUMPAD_DECIMAL,
        KeyCode::NumpadDivide => FFI_KEY_NUMPAD_DIVIDE,
        KeyCode::NumpadEnter => FFI_KEY_NUMPAD_ENTER,
        KeyCode::NumpadEqual => FFI_KEY_NUMPAD_EQUAL,
        KeyCode::NumpadMultiply => FFI_KEY_NUMPAD_MULTIPLY,
        KeyCode::NumpadSubtract => FFI_KEY_NUMPAD_SUBTRACT,
        KeyCode::Escape => FFI_KEY_ESCAPE,
        KeyCode::PrintScreen => FFI_KEY_PRINT_SCREEN,
        KeyCode::ScrollLock => FFI_KEY_SCROLL_LOCK,
        KeyCode::Pause => FFI_KEY_PAUSE,
        KeyCode::AudioVolumeDown => FFI_KEY_AUDIO_VOLUME_DOWN,
        KeyCode::AudioVolumeMute => FFI_KEY_AUDIO_VOLUME_MUTE,
        KeyCode::AudioVolumeUp => FFI_KEY_AUDIO_VOLUME_UP,
        KeyCode::MediaPlayPause => FFI_KEY_MEDIA_PLAY_PAUSE,
        KeyCode::MediaStop => FFI_KEY_MEDIA_STOP,
        KeyCode::MediaTrackNext => FFI_KEY_MEDIA_TRACK_NEXT,
        KeyCode::MediaTrackPrevious => FFI_KEY_MEDIA_TRACK_PREVIOUS,
        KeyCode::F1 => FFI_KEY_F1,
        KeyCode::F2 => FFI_KEY_F2,
        KeyCode::F3 => FFI_KEY_F3,
        KeyCode::F4 => FFI_KEY_F4,
        KeyCode::F5 => FFI_KEY_F5,
        KeyCode::F6 => FFI_KEY_F6,
        KeyCode::F7 => FFI_KEY_F7,
        KeyCode::F8 => FFI_KEY_F8,
        KeyCode::F9 => FFI_KEY_F9,
        KeyCode::F10 => FFI_KEY_F10,
        KeyCode::F11 => FFI_KEY_F11,
        KeyCode::F12 => FFI_KEY_F12,
        KeyCode::F13 => FFI_KEY_F13,
        KeyCode::F14 => FFI_KEY_F14,
        KeyCode::F15 => FFI_KEY_F15,
        KeyCode::F16 => FFI_KEY_F16,
        KeyCode::F17 => FFI_KEY_F17,
        KeyCode::F18 => FFI_KEY_F18,
        KeyCode::F19 => FFI_KEY_F19,
        KeyCode::F20 => FFI_KEY_F20,
        KeyCode::F21 => FFI_KEY_F21,
        KeyCode::F22 => FFI_KEY_F22,
        KeyCode::F23 => FFI_KEY_F23,
        KeyCode::F24 => FFI_KEY_F24,
        // `KeyCode` is `#[non_exhaustive]`.
        _ => FFI_KEY_UNKNOWN,
    }
}

/// The number of a named logical key.
#[must_use]
pub const fn named_key(key: NamedKey) -> u16 {
    match key {
        NamedKey::Alt => FFI_NAMED_KEY_ALT,
        NamedKey::AltGraph => FFI_NAMED_KEY_ALT_GRAPH,
        NamedKey::CapsLock => FFI_NAMED_KEY_CAPS_LOCK,
        NamedKey::Control => FFI_NAMED_KEY_CONTROL,
        NamedKey::Fn => FFI_NAMED_KEY_FN,
        NamedKey::FnLock => FFI_NAMED_KEY_FN_LOCK,
        NamedKey::Meta => FFI_NAMED_KEY_META,
        NamedKey::NumLock => FFI_NAMED_KEY_NUM_LOCK,
        NamedKey::ScrollLock => FFI_NAMED_KEY_SCROLL_LOCK,
        NamedKey::Shift => FFI_NAMED_KEY_SHIFT,
        NamedKey::Super => FFI_NAMED_KEY_SUPER,
        NamedKey::Enter => FFI_NAMED_KEY_ENTER,
        NamedKey::Tab => FFI_NAMED_KEY_TAB,
        NamedKey::Space => FFI_NAMED_KEY_SPACE,
        NamedKey::ArrowDown => FFI_NAMED_KEY_ARROW_DOWN,
        NamedKey::ArrowLeft => FFI_NAMED_KEY_ARROW_LEFT,
        NamedKey::ArrowRight => FFI_NAMED_KEY_ARROW_RIGHT,
        NamedKey::ArrowUp => FFI_NAMED_KEY_ARROW_UP,
        NamedKey::End => FFI_NAMED_KEY_END,
        NamedKey::Home => FFI_NAMED_KEY_HOME,
        NamedKey::PageDown => FFI_NAMED_KEY_PAGE_DOWN,
        NamedKey::PageUp => FFI_NAMED_KEY_PAGE_UP,
        NamedKey::Backspace => FFI_NAMED_KEY_BACKSPACE,
        NamedKey::Clear => FFI_NAMED_KEY_CLEAR,
        NamedKey::Copy => FFI_NAMED_KEY_COPY,
        NamedKey::Cut => FFI_NAMED_KEY_CUT,
        NamedKey::Delete => FFI_NAMED_KEY_DELETE,
        NamedKey::Insert => FFI_NAMED_KEY_INSERT,
        NamedKey::Paste => FFI_NAMED_KEY_PASTE,
        NamedKey::Redo => FFI_NAMED_KEY_REDO,
        NamedKey::Undo => FFI_NAMED_KEY_UNDO,
        NamedKey::ContextMenu => FFI_NAMED_KEY_CONTEXT_MENU,
        NamedKey::Escape => FFI_NAMED_KEY_ESCAPE,
        NamedKey::Help => FFI_NAMED_KEY_HELP,
        NamedKey::Pause => FFI_NAMED_KEY_PAUSE,
        NamedKey::PrintScreen => FFI_NAMED_KEY_PRINT_SCREEN,
        NamedKey::Process => FFI_NAMED_KEY_PROCESS,
        NamedKey::F1 => FFI_NAMED_KEY_F1,
        NamedKey::F2 => FFI_NAMED_KEY_F2,
        NamedKey::F3 => FFI_NAMED_KEY_F3,
        NamedKey::F4 => FFI_NAMED_KEY_F4,
        NamedKey::F5 => FFI_NAMED_KEY_F5,
        NamedKey::F6 => FFI_NAMED_KEY_F6,
        NamedKey::F7 => FFI_NAMED_KEY_F7,
        NamedKey::F8 => FFI_NAMED_KEY_F8,
        NamedKey::F9 => FFI_NAMED_KEY_F9,
        NamedKey::F10 => FFI_NAMED_KEY_F10,
        NamedKey::F11 => FFI_NAMED_KEY_F11,
        NamedKey::F12 => FFI_NAMED_KEY_F12,
        NamedKey::F13 => FFI_NAMED_KEY_F13,
        NamedKey::F14 => FFI_NAMED_KEY_F14,
        NamedKey::F15 => FFI_NAMED_KEY_F15,
        NamedKey::F16 => FFI_NAMED_KEY_F16,
        NamedKey::F17 => FFI_NAMED_KEY_F17,
        NamedKey::F18 => FFI_NAMED_KEY_F18,
        NamedKey::F19 => FFI_NAMED_KEY_F19,
        NamedKey::F20 => FFI_NAMED_KEY_F20,
        NamedKey::F21 => FFI_NAMED_KEY_F21,
        NamedKey::F22 => FFI_NAMED_KEY_F22,
        NamedKey::F23 => FFI_NAMED_KEY_F23,
        NamedKey::F24 => FFI_NAMED_KEY_F24,
        // `NamedKey` is `#[non_exhaustive]`.
        _ => FFI_NAMED_KEY_UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KEY_CODES: &[KeyCode] = &[
        KeyCode::Unidentified,
        KeyCode::Backquote,
        KeyCode::Backslash,
        KeyCode::BracketLeft,
        KeyCode::BracketRight,
        KeyCode::Comma,
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Equal,
        KeyCode::IntlBackslash,
        KeyCode::IntlRo,
        KeyCode::IntlYen,
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Minus,
        KeyCode::Period,
        KeyCode::Quote,
        KeyCode::Semicolon,
        KeyCode::Slash,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::Backspace,
        KeyCode::CapsLock,
        KeyCode::ContextMenu,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::Enter,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::Space,
        KeyCode::Tab,
        KeyCode::Convert,
        KeyCode::KanaMode,
        KeyCode::NonConvert,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::Help,
        KeyCode::Home,
        KeyCode::Insert,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::NumLock,
        KeyCode::Numpad0,
        KeyCode::Numpad1,
        KeyCode::Numpad2,
        KeyCode::Numpad3,
        KeyCode::Numpad4,
        KeyCode::Numpad5,
        KeyCode::Numpad6,
        KeyCode::Numpad7,
        KeyCode::Numpad8,
        KeyCode::Numpad9,
        KeyCode::NumpadAdd,
        KeyCode::NumpadComma,
        KeyCode::NumpadDecimal,
        KeyCode::NumpadDivide,
        KeyCode::NumpadEnter,
        KeyCode::NumpadEqual,
        KeyCode::NumpadMultiply,
        KeyCode::NumpadSubtract,
        KeyCode::Escape,
        KeyCode::PrintScreen,
        KeyCode::ScrollLock,
        KeyCode::Pause,
        KeyCode::AudioVolumeDown,
        KeyCode::AudioVolumeMute,
        KeyCode::AudioVolumeUp,
        KeyCode::MediaPlayPause,
        KeyCode::MediaStop,
        KeyCode::MediaTrackNext,
        KeyCode::MediaTrackPrevious,
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
        KeyCode::F13,
        KeyCode::F14,
        KeyCode::F15,
        KeyCode::F16,
        KeyCode::F17,
        KeyCode::F18,
        KeyCode::F19,
        KeyCode::F20,
        KeyCode::F21,
        KeyCode::F22,
        KeyCode::F23,
        KeyCode::F24,
    ];

    const ALL_NAMED_KEYS: &[NamedKey] = &[
        NamedKey::Alt,
        NamedKey::AltGraph,
        NamedKey::CapsLock,
        NamedKey::Control,
        NamedKey::Fn,
        NamedKey::FnLock,
        NamedKey::Meta,
        NamedKey::NumLock,
        NamedKey::ScrollLock,
        NamedKey::Shift,
        NamedKey::Super,
        NamedKey::Enter,
        NamedKey::Tab,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::ArrowUp,
        NamedKey::End,
        NamedKey::Home,
        NamedKey::PageDown,
        NamedKey::PageUp,
        NamedKey::Backspace,
        NamedKey::Clear,
        NamedKey::Copy,
        NamedKey::Cut,
        NamedKey::Delete,
        NamedKey::Insert,
        NamedKey::Paste,
        NamedKey::Redo,
        NamedKey::Undo,
        NamedKey::ContextMenu,
        NamedKey::Escape,
        NamedKey::Help,
        NamedKey::Pause,
        NamedKey::PrintScreen,
        NamedKey::Process,
        NamedKey::F1,
        NamedKey::F2,
        NamedKey::F3,
        NamedKey::F4,
        NamedKey::F5,
        NamedKey::F6,
        NamedKey::F7,
        NamedKey::F8,
        NamedKey::F9,
        NamedKey::F10,
        NamedKey::F11,
        NamedKey::F12,
        NamedKey::F13,
        NamedKey::F14,
        NamedKey::F15,
        NamedKey::F16,
        NamedKey::F17,
        NamedKey::F18,
        NamedKey::F19,
        NamedKey::F20,
        NamedKey::F21,
        NamedKey::F22,
        NamedKey::F23,
        NamedKey::F24,
    ];

    #[test]
    fn every_physical_key_has_its_own_known_number() {
        let mut seen = std::collections::BTreeSet::new();
        for &code in ALL_KEY_CODES {
            let number = key_code(code);
            assert_ne!(number, FFI_KEY_UNKNOWN, "{code:?} is not mapped");
            assert!(seen.insert(number), "{code:?} shares its number");
        }
        assert_eq!(seen.len(), ALL_KEY_CODES.len());
    }

    #[test]
    fn every_named_key_has_its_own_known_number() {
        let mut seen = std::collections::BTreeSet::new();
        for &key in ALL_NAMED_KEYS {
            let number = named_key(key);
            assert_ne!(number, FFI_NAMED_KEY_UNKNOWN, "{key:?} is not mapped");
            assert!(seen.insert(number), "{key:?} shares its number");
        }
        assert_eq!(seen.len(), ALL_NAMED_KEYS.len());
    }

    #[test]
    fn numbers_follow_the_documented_order() {
        assert_eq!(key_code(KeyCode::Unidentified), 0);
        assert_eq!(key_code(KeyCode::Backquote), 1);
        assert_eq!(
            u16::try_from(ALL_KEY_CODES.len()).unwrap() - 1,
            key_code(KeyCode::F24)
        );
        assert_eq!(named_key(NamedKey::Alt), 0);
        assert_eq!(
            u16::try_from(ALL_NAMED_KEYS.len()).unwrap() - 1,
            named_key(NamedKey::F24)
        );
    }

    #[test]
    fn the_tables_list_every_key_of_the_platform_crate() {
        // Compared with the enum source, so a key added there without a
        // number here fails this test.
        let source = include_str!("../../platform/src/input/key.rs");
        let count = |name: &str| {
            let start = source.find(&format!("pub enum {name} {{")).unwrap();
            let body = &source[start..];
            let body = &body[..body.find("\n}").unwrap()];
            body.lines()
                .filter(|l| {
                    l.starts_with("    ")
                        && !l.trim_start().starts_with("//")
                        && l.trim_end().ends_with(',')
                })
                .filter(|l| !l.trim_start().starts_with("#["))
                .count()
        };
        assert_eq!(count("KeyCode"), ALL_KEY_CODES.len());
        assert_eq!(count("NamedKey"), ALL_NAMED_KEYS.len());
    }
}
