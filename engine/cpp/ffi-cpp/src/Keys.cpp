// GENERATED, see include/cv-ffi/Keys.hpp.

#include "cv-ffi/Keys.hpp"

#include <array>
#include <cstddef>

#include "ffi.h"

namespace cv_ffi::window {

namespace {

static_assert(static_cast<std::uint16_t>(KeyCode::Unidentified) == FFI_KEY_UNIDENTIFIED);
static_assert(static_cast<std::uint16_t>(KeyCode::Backquote) == FFI_KEY_BACKQUOTE);
static_assert(static_cast<std::uint16_t>(KeyCode::Backslash) == FFI_KEY_BACKSLASH);
static_assert(static_cast<std::uint16_t>(KeyCode::BracketLeft) == FFI_KEY_BRACKET_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::BracketRight) == FFI_KEY_BRACKET_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::Comma) == FFI_KEY_COMMA);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit0) == FFI_KEY_DIGIT_0);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit1) == FFI_KEY_DIGIT_1);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit2) == FFI_KEY_DIGIT_2);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit3) == FFI_KEY_DIGIT_3);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit4) == FFI_KEY_DIGIT_4);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit5) == FFI_KEY_DIGIT_5);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit6) == FFI_KEY_DIGIT_6);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit7) == FFI_KEY_DIGIT_7);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit8) == FFI_KEY_DIGIT_8);
static_assert(static_cast<std::uint16_t>(KeyCode::Digit9) == FFI_KEY_DIGIT_9);
static_assert(static_cast<std::uint16_t>(KeyCode::Equal) == FFI_KEY_EQUAL);
static_assert(static_cast<std::uint16_t>(KeyCode::IntlBackslash) == FFI_KEY_INTL_BACKSLASH);
static_assert(static_cast<std::uint16_t>(KeyCode::IntlRo) == FFI_KEY_INTL_RO);
static_assert(static_cast<std::uint16_t>(KeyCode::IntlYen) == FFI_KEY_INTL_YEN);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyA) == FFI_KEY_KEY_A);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyB) == FFI_KEY_KEY_B);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyC) == FFI_KEY_KEY_C);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyD) == FFI_KEY_KEY_D);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyE) == FFI_KEY_KEY_E);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyF) == FFI_KEY_KEY_F);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyG) == FFI_KEY_KEY_G);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyH) == FFI_KEY_KEY_H);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyI) == FFI_KEY_KEY_I);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyJ) == FFI_KEY_KEY_J);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyK) == FFI_KEY_KEY_K);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyL) == FFI_KEY_KEY_L);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyM) == FFI_KEY_KEY_M);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyN) == FFI_KEY_KEY_N);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyO) == FFI_KEY_KEY_O);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyP) == FFI_KEY_KEY_P);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyQ) == FFI_KEY_KEY_Q);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyR) == FFI_KEY_KEY_R);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyS) == FFI_KEY_KEY_S);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyT) == FFI_KEY_KEY_T);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyU) == FFI_KEY_KEY_U);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyV) == FFI_KEY_KEY_V);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyW) == FFI_KEY_KEY_W);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyX) == FFI_KEY_KEY_X);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyY) == FFI_KEY_KEY_Y);
static_assert(static_cast<std::uint16_t>(KeyCode::KeyZ) == FFI_KEY_KEY_Z);
static_assert(static_cast<std::uint16_t>(KeyCode::Minus) == FFI_KEY_MINUS);
static_assert(static_cast<std::uint16_t>(KeyCode::Period) == FFI_KEY_PERIOD);
static_assert(static_cast<std::uint16_t>(KeyCode::Quote) == FFI_KEY_QUOTE);
static_assert(static_cast<std::uint16_t>(KeyCode::Semicolon) == FFI_KEY_SEMICOLON);
static_assert(static_cast<std::uint16_t>(KeyCode::Slash) == FFI_KEY_SLASH);
static_assert(static_cast<std::uint16_t>(KeyCode::AltLeft) == FFI_KEY_ALT_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::AltRight) == FFI_KEY_ALT_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::Backspace) == FFI_KEY_BACKSPACE);
static_assert(static_cast<std::uint16_t>(KeyCode::CapsLock) == FFI_KEY_CAPS_LOCK);
static_assert(static_cast<std::uint16_t>(KeyCode::ContextMenu) == FFI_KEY_CONTEXT_MENU);
static_assert(static_cast<std::uint16_t>(KeyCode::ControlLeft) == FFI_KEY_CONTROL_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::ControlRight) == FFI_KEY_CONTROL_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::Enter) == FFI_KEY_ENTER);
static_assert(static_cast<std::uint16_t>(KeyCode::SuperLeft) == FFI_KEY_SUPER_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::SuperRight) == FFI_KEY_SUPER_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::ShiftLeft) == FFI_KEY_SHIFT_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::ShiftRight) == FFI_KEY_SHIFT_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::Space) == FFI_KEY_SPACE);
static_assert(static_cast<std::uint16_t>(KeyCode::Tab) == FFI_KEY_TAB);
static_assert(static_cast<std::uint16_t>(KeyCode::Convert) == FFI_KEY_CONVERT);
static_assert(static_cast<std::uint16_t>(KeyCode::KanaMode) == FFI_KEY_KANA_MODE);
static_assert(static_cast<std::uint16_t>(KeyCode::NonConvert) == FFI_KEY_NON_CONVERT);
static_assert(static_cast<std::uint16_t>(KeyCode::Delete) == FFI_KEY_DELETE);
static_assert(static_cast<std::uint16_t>(KeyCode::End) == FFI_KEY_END);
static_assert(static_cast<std::uint16_t>(KeyCode::Help) == FFI_KEY_HELP);
static_assert(static_cast<std::uint16_t>(KeyCode::Home) == FFI_KEY_HOME);
static_assert(static_cast<std::uint16_t>(KeyCode::Insert) == FFI_KEY_INSERT);
static_assert(static_cast<std::uint16_t>(KeyCode::PageDown) == FFI_KEY_PAGE_DOWN);
static_assert(static_cast<std::uint16_t>(KeyCode::PageUp) == FFI_KEY_PAGE_UP);
static_assert(static_cast<std::uint16_t>(KeyCode::ArrowDown) == FFI_KEY_ARROW_DOWN);
static_assert(static_cast<std::uint16_t>(KeyCode::ArrowLeft) == FFI_KEY_ARROW_LEFT);
static_assert(static_cast<std::uint16_t>(KeyCode::ArrowRight) == FFI_KEY_ARROW_RIGHT);
static_assert(static_cast<std::uint16_t>(KeyCode::ArrowUp) == FFI_KEY_ARROW_UP);
static_assert(static_cast<std::uint16_t>(KeyCode::NumLock) == FFI_KEY_NUM_LOCK);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad0) == FFI_KEY_NUMPAD_0);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad1) == FFI_KEY_NUMPAD_1);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad2) == FFI_KEY_NUMPAD_2);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad3) == FFI_KEY_NUMPAD_3);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad4) == FFI_KEY_NUMPAD_4);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad5) == FFI_KEY_NUMPAD_5);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad6) == FFI_KEY_NUMPAD_6);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad7) == FFI_KEY_NUMPAD_7);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad8) == FFI_KEY_NUMPAD_8);
static_assert(static_cast<std::uint16_t>(KeyCode::Numpad9) == FFI_KEY_NUMPAD_9);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadAdd) == FFI_KEY_NUMPAD_ADD);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadComma) == FFI_KEY_NUMPAD_COMMA);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadDecimal) == FFI_KEY_NUMPAD_DECIMAL);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadDivide) == FFI_KEY_NUMPAD_DIVIDE);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadEnter) == FFI_KEY_NUMPAD_ENTER);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadEqual) == FFI_KEY_NUMPAD_EQUAL);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadMultiply) == FFI_KEY_NUMPAD_MULTIPLY);
static_assert(static_cast<std::uint16_t>(KeyCode::NumpadSubtract) == FFI_KEY_NUMPAD_SUBTRACT);
static_assert(static_cast<std::uint16_t>(KeyCode::Escape) == FFI_KEY_ESCAPE);
static_assert(static_cast<std::uint16_t>(KeyCode::PrintScreen) == FFI_KEY_PRINT_SCREEN);
static_assert(static_cast<std::uint16_t>(KeyCode::ScrollLock) == FFI_KEY_SCROLL_LOCK);
static_assert(static_cast<std::uint16_t>(KeyCode::Pause) == FFI_KEY_PAUSE);
static_assert(static_cast<std::uint16_t>(KeyCode::AudioVolumeDown) == FFI_KEY_AUDIO_VOLUME_DOWN);
static_assert(static_cast<std::uint16_t>(KeyCode::AudioVolumeMute) == FFI_KEY_AUDIO_VOLUME_MUTE);
static_assert(static_cast<std::uint16_t>(KeyCode::AudioVolumeUp) == FFI_KEY_AUDIO_VOLUME_UP);
static_assert(static_cast<std::uint16_t>(KeyCode::MediaPlayPause) == FFI_KEY_MEDIA_PLAY_PAUSE);
static_assert(static_cast<std::uint16_t>(KeyCode::MediaStop) == FFI_KEY_MEDIA_STOP);
static_assert(static_cast<std::uint16_t>(KeyCode::MediaTrackNext) == FFI_KEY_MEDIA_TRACK_NEXT);
static_assert(static_cast<std::uint16_t>(KeyCode::MediaTrackPrevious) == FFI_KEY_MEDIA_TRACK_PREVIOUS);
static_assert(static_cast<std::uint16_t>(KeyCode::F1) == FFI_KEY_F1);
static_assert(static_cast<std::uint16_t>(KeyCode::F2) == FFI_KEY_F2);
static_assert(static_cast<std::uint16_t>(KeyCode::F3) == FFI_KEY_F3);
static_assert(static_cast<std::uint16_t>(KeyCode::F4) == FFI_KEY_F4);
static_assert(static_cast<std::uint16_t>(KeyCode::F5) == FFI_KEY_F5);
static_assert(static_cast<std::uint16_t>(KeyCode::F6) == FFI_KEY_F6);
static_assert(static_cast<std::uint16_t>(KeyCode::F7) == FFI_KEY_F7);
static_assert(static_cast<std::uint16_t>(KeyCode::F8) == FFI_KEY_F8);
static_assert(static_cast<std::uint16_t>(KeyCode::F9) == FFI_KEY_F9);
static_assert(static_cast<std::uint16_t>(KeyCode::F10) == FFI_KEY_F10);
static_assert(static_cast<std::uint16_t>(KeyCode::F11) == FFI_KEY_F11);
static_assert(static_cast<std::uint16_t>(KeyCode::F12) == FFI_KEY_F12);
static_assert(static_cast<std::uint16_t>(KeyCode::F13) == FFI_KEY_F13);
static_assert(static_cast<std::uint16_t>(KeyCode::F14) == FFI_KEY_F14);
static_assert(static_cast<std::uint16_t>(KeyCode::F15) == FFI_KEY_F15);
static_assert(static_cast<std::uint16_t>(KeyCode::F16) == FFI_KEY_F16);
static_assert(static_cast<std::uint16_t>(KeyCode::F17) == FFI_KEY_F17);
static_assert(static_cast<std::uint16_t>(KeyCode::F18) == FFI_KEY_F18);
static_assert(static_cast<std::uint16_t>(KeyCode::F19) == FFI_KEY_F19);
static_assert(static_cast<std::uint16_t>(KeyCode::F20) == FFI_KEY_F20);
static_assert(static_cast<std::uint16_t>(KeyCode::F21) == FFI_KEY_F21);
static_assert(static_cast<std::uint16_t>(KeyCode::F22) == FFI_KEY_F22);
static_assert(static_cast<std::uint16_t>(KeyCode::F23) == FFI_KEY_F23);
static_assert(static_cast<std::uint16_t>(KeyCode::F24) == FFI_KEY_F24);
static_assert(static_cast<std::uint16_t>(KeyCode::Unknown) == FFI_KEY_UNKNOWN);
static_assert(static_cast<std::uint16_t>(NamedKey::Alt) == FFI_NAMED_KEY_ALT);
static_assert(static_cast<std::uint16_t>(NamedKey::AltGraph) == FFI_NAMED_KEY_ALT_GRAPH);
static_assert(static_cast<std::uint16_t>(NamedKey::CapsLock) == FFI_NAMED_KEY_CAPS_LOCK);
static_assert(static_cast<std::uint16_t>(NamedKey::Control) == FFI_NAMED_KEY_CONTROL);
static_assert(static_cast<std::uint16_t>(NamedKey::Fn) == FFI_NAMED_KEY_FN);
static_assert(static_cast<std::uint16_t>(NamedKey::FnLock) == FFI_NAMED_KEY_FN_LOCK);
static_assert(static_cast<std::uint16_t>(NamedKey::Meta) == FFI_NAMED_KEY_META);
static_assert(static_cast<std::uint16_t>(NamedKey::NumLock) == FFI_NAMED_KEY_NUM_LOCK);
static_assert(static_cast<std::uint16_t>(NamedKey::ScrollLock) == FFI_NAMED_KEY_SCROLL_LOCK);
static_assert(static_cast<std::uint16_t>(NamedKey::Shift) == FFI_NAMED_KEY_SHIFT);
static_assert(static_cast<std::uint16_t>(NamedKey::Super) == FFI_NAMED_KEY_SUPER);
static_assert(static_cast<std::uint16_t>(NamedKey::Enter) == FFI_NAMED_KEY_ENTER);
static_assert(static_cast<std::uint16_t>(NamedKey::Tab) == FFI_NAMED_KEY_TAB);
static_assert(static_cast<std::uint16_t>(NamedKey::Space) == FFI_NAMED_KEY_SPACE);
static_assert(static_cast<std::uint16_t>(NamedKey::ArrowDown) == FFI_NAMED_KEY_ARROW_DOWN);
static_assert(static_cast<std::uint16_t>(NamedKey::ArrowLeft) == FFI_NAMED_KEY_ARROW_LEFT);
static_assert(static_cast<std::uint16_t>(NamedKey::ArrowRight) == FFI_NAMED_KEY_ARROW_RIGHT);
static_assert(static_cast<std::uint16_t>(NamedKey::ArrowUp) == FFI_NAMED_KEY_ARROW_UP);
static_assert(static_cast<std::uint16_t>(NamedKey::End) == FFI_NAMED_KEY_END);
static_assert(static_cast<std::uint16_t>(NamedKey::Home) == FFI_NAMED_KEY_HOME);
static_assert(static_cast<std::uint16_t>(NamedKey::PageDown) == FFI_NAMED_KEY_PAGE_DOWN);
static_assert(static_cast<std::uint16_t>(NamedKey::PageUp) == FFI_NAMED_KEY_PAGE_UP);
static_assert(static_cast<std::uint16_t>(NamedKey::Backspace) == FFI_NAMED_KEY_BACKSPACE);
static_assert(static_cast<std::uint16_t>(NamedKey::Clear) == FFI_NAMED_KEY_CLEAR);
static_assert(static_cast<std::uint16_t>(NamedKey::Copy) == FFI_NAMED_KEY_COPY);
static_assert(static_cast<std::uint16_t>(NamedKey::Cut) == FFI_NAMED_KEY_CUT);
static_assert(static_cast<std::uint16_t>(NamedKey::Delete) == FFI_NAMED_KEY_DELETE);
static_assert(static_cast<std::uint16_t>(NamedKey::Insert) == FFI_NAMED_KEY_INSERT);
static_assert(static_cast<std::uint16_t>(NamedKey::Paste) == FFI_NAMED_KEY_PASTE);
static_assert(static_cast<std::uint16_t>(NamedKey::Redo) == FFI_NAMED_KEY_REDO);
static_assert(static_cast<std::uint16_t>(NamedKey::Undo) == FFI_NAMED_KEY_UNDO);
static_assert(static_cast<std::uint16_t>(NamedKey::ContextMenu) == FFI_NAMED_KEY_CONTEXT_MENU);
static_assert(static_cast<std::uint16_t>(NamedKey::Escape) == FFI_NAMED_KEY_ESCAPE);
static_assert(static_cast<std::uint16_t>(NamedKey::Help) == FFI_NAMED_KEY_HELP);
static_assert(static_cast<std::uint16_t>(NamedKey::Pause) == FFI_NAMED_KEY_PAUSE);
static_assert(static_cast<std::uint16_t>(NamedKey::PrintScreen) == FFI_NAMED_KEY_PRINT_SCREEN);
static_assert(static_cast<std::uint16_t>(NamedKey::Process) == FFI_NAMED_KEY_PROCESS);
static_assert(static_cast<std::uint16_t>(NamedKey::F1) == FFI_NAMED_KEY_F1);
static_assert(static_cast<std::uint16_t>(NamedKey::F2) == FFI_NAMED_KEY_F2);
static_assert(static_cast<std::uint16_t>(NamedKey::F3) == FFI_NAMED_KEY_F3);
static_assert(static_cast<std::uint16_t>(NamedKey::F4) == FFI_NAMED_KEY_F4);
static_assert(static_cast<std::uint16_t>(NamedKey::F5) == FFI_NAMED_KEY_F5);
static_assert(static_cast<std::uint16_t>(NamedKey::F6) == FFI_NAMED_KEY_F6);
static_assert(static_cast<std::uint16_t>(NamedKey::F7) == FFI_NAMED_KEY_F7);
static_assert(static_cast<std::uint16_t>(NamedKey::F8) == FFI_NAMED_KEY_F8);
static_assert(static_cast<std::uint16_t>(NamedKey::F9) == FFI_NAMED_KEY_F9);
static_assert(static_cast<std::uint16_t>(NamedKey::F10) == FFI_NAMED_KEY_F10);
static_assert(static_cast<std::uint16_t>(NamedKey::F11) == FFI_NAMED_KEY_F11);
static_assert(static_cast<std::uint16_t>(NamedKey::F12) == FFI_NAMED_KEY_F12);
static_assert(static_cast<std::uint16_t>(NamedKey::F13) == FFI_NAMED_KEY_F13);
static_assert(static_cast<std::uint16_t>(NamedKey::F14) == FFI_NAMED_KEY_F14);
static_assert(static_cast<std::uint16_t>(NamedKey::F15) == FFI_NAMED_KEY_F15);
static_assert(static_cast<std::uint16_t>(NamedKey::F16) == FFI_NAMED_KEY_F16);
static_assert(static_cast<std::uint16_t>(NamedKey::F17) == FFI_NAMED_KEY_F17);
static_assert(static_cast<std::uint16_t>(NamedKey::F18) == FFI_NAMED_KEY_F18);
static_assert(static_cast<std::uint16_t>(NamedKey::F19) == FFI_NAMED_KEY_F19);
static_assert(static_cast<std::uint16_t>(NamedKey::F20) == FFI_NAMED_KEY_F20);
static_assert(static_cast<std::uint16_t>(NamedKey::F21) == FFI_NAMED_KEY_F21);
static_assert(static_cast<std::uint16_t>(NamedKey::F22) == FFI_NAMED_KEY_F22);
static_assert(static_cast<std::uint16_t>(NamedKey::F23) == FFI_NAMED_KEY_F23);
static_assert(static_cast<std::uint16_t>(NamedKey::F24) == FFI_NAMED_KEY_F24);
static_assert(static_cast<std::uint16_t>(NamedKey::Unknown) == FFI_NAMED_KEY_UNKNOWN);

constexpr std::array<std::string_view, 133> kKeyNames = {
    "Unidentified",
    "Backquote",
    "Backslash",
    "BracketLeft",
    "BracketRight",
    "Comma",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Equal",
    "IntlBackslash",
    "IntlRo",
    "IntlYen",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Minus",
    "Period",
    "Quote",
    "Semicolon",
    "Slash",
    "AltLeft",
    "AltRight",
    "Backspace",
    "CapsLock",
    "ContextMenu",
    "ControlLeft",
    "ControlRight",
    "Enter",
    "SuperLeft",
    "SuperRight",
    "ShiftLeft",
    "ShiftRight",
    "Space",
    "Tab",
    "Convert",
    "KanaMode",
    "NonConvert",
    "Delete",
    "End",
    "Help",
    "Home",
    "Insert",
    "PageDown",
    "PageUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "NumLock",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadComma",
    "NumpadDecimal",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadEqual",
    "NumpadMultiply",
    "NumpadSubtract",
    "Escape",
    "PrintScreen",
    "ScrollLock",
    "Pause",
    "AudioVolumeDown",
    "AudioVolumeMute",
    "AudioVolumeUp",
    "MediaPlayPause",
    "MediaStop",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
};

constexpr std::array<std::string_view, 61> kNamedKeyNames = {
    "Alt",
    "AltGraph",
    "CapsLock",
    "Control",
    "Fn",
    "FnLock",
    "Meta",
    "NumLock",
    "ScrollLock",
    "Shift",
    "Super",
    "Enter",
    "Tab",
    "Space",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "End",
    "Home",
    "PageDown",
    "PageUp",
    "Backspace",
    "Clear",
    "Copy",
    "Cut",
    "Delete",
    "Insert",
    "Paste",
    "Redo",
    "Undo",
    "ContextMenu",
    "Escape",
    "Help",
    "Pause",
    "PrintScreen",
    "Process",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
};

} // namespace

std::string_view KeyName(KeyCode key) noexcept
{
    const auto index = static_cast<std::size_t>(key);
    return index < kKeyNames.size() ? kKeyNames[index] : std::string_view{"Unknown"};
}

std::string_view NamedKeyName(NamedKey key) noexcept
{
    const auto index = static_cast<std::size_t>(key);
    return index < kNamedKeyNames.size() ? kNamedKeyNames[index] : std::string_view{"Unknown"};
}

} // namespace cv_ffi::window
