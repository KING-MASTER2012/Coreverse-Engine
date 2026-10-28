//! Conversions from `winit` input events to the types of [`crate::input`].

use winit::event::{
    ElementState, Ime, KeyEvent as WinitKeyEvent, MouseButton as WinitMouseButton,
    MouseScrollDelta, Touch as WinitTouch, TouchPhase as WinitTouchPhase,
};
use winit::keyboard::{Key as WinitKey, KeyCode as W, ModifiersState, NamedKey as WN, PhysicalKey};

use crate::input::{
    ButtonState, ImeEvent, Key, KeyCode, KeyEvent, Modifiers, MouseButton, NamedKey,
    PointerPosition, ScrollDelta, TouchEvent, TouchPhase,
};

/// Maps the variants that carry the same name on both sides one to one; every
/// other value becomes `$fallback`.
macro_rules! same_name {
    ($value:expr, $from:ident => $to:ident, $fallback:expr, [$($name:ident),+ $(,)?]) => {
        match $value {
            $($from::$name => $to::$name,)+
            _ => $fallback,
        }
    };
}

pub(super) fn button_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

pub(super) fn mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(number) => MouseButton::Other(number),
    }
}

pub(super) fn modifiers(state: ModifiersState) -> Modifiers {
    let mut modifiers = Modifiers::NONE;
    for (flag, modifier) in [
        (ModifiersState::SHIFT, Modifiers::SHIFT),
        (ModifiersState::CONTROL, Modifiers::CONTROL),
        (ModifiersState::ALT, Modifiers::ALT),
        (ModifiersState::SUPER, Modifiers::SUPER),
    ] {
        if state.contains(flag) {
            modifiers |= modifier;
        }
    }
    modifiers
}

pub(super) fn scroll_delta(delta: MouseScrollDelta) -> ScrollDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines {
            x: f64::from(x),
            y: f64::from(y),
        },
        MouseScrollDelta::PixelDelta(position) => ScrollDelta::Pixels {
            x: position.x,
            y: position.y,
        },
    }
}

pub(super) fn touch(touch: &WinitTouch) -> TouchEvent {
    let phase = match touch.phase {
        WinitTouchPhase::Started => TouchPhase::Started,
        WinitTouchPhase::Moved => TouchPhase::Moved,
        WinitTouchPhase::Ended => TouchPhase::Ended,
        WinitTouchPhase::Cancelled => TouchPhase::Cancelled,
    };
    TouchEvent::new(
        touch.id,
        phase,
        PointerPosition::new(touch.location.x, touch.location.y),
    )
}

pub(super) fn ime(event: Ime) -> ImeEvent {
    match event {
        Ime::Enabled => ImeEvent::Enabled,
        Ime::Preedit(text, cursor) => ImeEvent::Preedit { text, cursor },
        Ime::Commit(text) => ImeEvent::Commit(text),
        Ime::Disabled => ImeEvent::Disabled,
    }
}

pub(super) fn key_event(event: &WinitKeyEvent, synthetic: bool) -> KeyEvent {
    let physical = match event.physical_key {
        PhysicalKey::Code(code) => key_code(code),
        PhysicalKey::Unidentified(_) => KeyCode::Unidentified,
    };
    KeyEvent {
        physical_key: physical,
        logical_key: logical_key(&event.logical_key),
        text: event.text.as_ref().map(|text| text.to_string()),
        state: button_state(event.state),
        repeat: event.repeat,
        synthetic,
    }
}

fn logical_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(named) => named_key(*named).map_or(Key::Unidentified, Key::Named),
        WinitKey::Character(text) => Key::Character(text.to_string()),
        WinitKey::Dead(accent) => Key::Dead(*accent),
        WinitKey::Unidentified(_) => Key::Unidentified,
    }
}

fn named_key(key: WN) -> Option<NamedKey> {
    // `Super` is also reported as `Meta` on some systems; both map to their own
    // variant.
    Some(same_name!(key, WN => NamedKey, return None, [
            Alt, AltGraph, CapsLock, Control, Fn, FnLock, Meta, NumLock, ScrollLock, Shift,
            Super, Enter, Tab, Space, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Home,
            PageDown, PageUp, Backspace, Clear, Copy, Cut, Delete, Insert, Paste, Redo, Undo,
            ContextMenu, Escape, Help, Pause, PrintScreen, Process, F1, F2, F3, F4, F5, F6, F7,
            F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
    ]))
}

fn key_code(code: W) -> KeyCode {
    same_name!(code, W => KeyCode, KeyCode::Unidentified, [
            Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2,
            Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, IntlBackslash,
            IntlRo, IntlYen, KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK,
            KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY,
            KeyZ, Minus, Period, Quote, Semicolon, Slash, AltLeft, AltRight, Backspace,
            CapsLock, ContextMenu, ControlLeft, ControlRight, Enter, SuperLeft, SuperRight,
            ShiftLeft, ShiftRight, Space, Tab, Convert, KanaMode, NonConvert, Delete, End,
            Help, Home, Insert, PageDown, PageUp, ArrowDown, ArrowLeft, ArrowRight, ArrowUp,
            NumLock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7,
            Numpad8, Numpad9, NumpadAdd, NumpadComma, NumpadDecimal, NumpadDivide, NumpadEnter,
            NumpadEqual, NumpadMultiply, NumpadSubtract, Escape, PrintScreen, ScrollLock,
            Pause, AudioVolumeDown, AudioVolumeMute, AudioVolumeUp, MediaPlayPause, MediaStop,
            MediaTrackNext, MediaTrackPrevious, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11,
            F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_flags_map_one_to_one() {
        assert_eq!(modifiers(ModifiersState::empty()), Modifiers::NONE);
        let all = ModifiersState::SHIFT
            | ModifiersState::CONTROL
            | ModifiersState::ALT
            | ModifiersState::SUPER;
        assert_eq!(
            modifiers(all),
            Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER
        );
        assert_eq!(modifiers(ModifiersState::ALT), Modifiers::ALT);
    }

    #[test]
    fn mouse_buttons_map_one_to_one() {
        assert_eq!(mouse_button(WinitMouseButton::Left), MouseButton::Left);
        assert_eq!(
            mouse_button(WinitMouseButton::Forward),
            MouseButton::Forward
        );
        assert_eq!(
            mouse_button(WinitMouseButton::Other(9)),
            MouseButton::Other(9)
        );
    }

    #[test]
    fn scrolling_keeps_its_unit() {
        assert_eq!(
            scroll_delta(MouseScrollDelta::LineDelta(0.5, -2.0)),
            ScrollDelta::Lines { x: 0.5, y: -2.0 }
        );
        assert_eq!(
            scroll_delta(MouseScrollDelta::PixelDelta(
                winit::dpi::PhysicalPosition::new(3.0, 4.0)
            )),
            ScrollDelta::Pixels { x: 3.0, y: 4.0 }
        );
    }

    #[test]
    fn physical_and_logical_keys_map_by_name() {
        assert_eq!(key_code(W::KeyW), KeyCode::KeyW);
        assert_eq!(key_code(W::Numpad7), KeyCode::Numpad7);
        assert_eq!(key_code(W::F24), KeyCode::F24);
        assert_eq!(key_code(W::BrowserBack), KeyCode::Unidentified);
        assert_eq!(
            logical_key(&WinitKey::Named(WN::Enter)),
            Key::Named(NamedKey::Enter)
        );
        assert_eq!(
            logical_key(&WinitKey::Named(WN::BrowserBack)),
            Key::Unidentified
        );
        assert_eq!(
            logical_key(&WinitKey::Character("a".into())),
            Key::character("a")
        );
        assert_eq!(
            logical_key(&WinitKey::Dead(Some('`'))),
            Key::Dead(Some('`'))
        );
    }

    #[test]
    fn ime_events_map_one_to_one() {
        assert_eq!(ime(Ime::Enabled), ImeEvent::Enabled);
        assert_eq!(
            ime(Ime::Commit("x".to_owned())),
            ImeEvent::Commit("x".to_owned())
        );
        assert_eq!(
            ime(Ime::Preedit("ab".to_owned(), Some((0, 1)))),
            ImeEvent::Preedit {
                text: "ab".to_owned(),
                cursor: Some((0, 1))
            }
        );
    }
}
