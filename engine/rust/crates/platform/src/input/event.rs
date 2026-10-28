//! The input events a window delivers.

use super::button::{ButtonState, MouseButton};
use super::ime::ImeEvent;
use super::key::{KeyEvent, Modifiers};
use super::pointer::{PointerPosition, ScrollDelta, TouchEvent};

/// Something the user did with an input device, delivered inside
/// [`WindowEvent::Input`](crate::window::WindowEvent::Input).
///
/// Positions are in physical pixels of the window's client area. Feed the
/// events to an [`InputState`](super::InputState) to ask "is W held this
/// frame" instead of tracking presses yourself.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InputEvent {
    /// A key went down or up (auto-repeat included, see
    /// [`KeyEvent::repeat`]).
    Key(KeyEvent),
    /// The set of held modifier keys changed.
    ModifiersChanged(Modifiers),
    /// The mouse cursor moved inside the window. Not delivered while the
    /// cursor is locked, see [`RawMouseMotion`](Self::RawMouseMotion).
    CursorMoved(PointerPosition),
    /// The mouse cursor entered the window.
    CursorEntered,
    /// The mouse cursor left the window.
    CursorLeft,
    /// A mouse button went down or up.
    MouseButton {
        /// Which button.
        button: MouseButton,
        /// Down or up.
        state: ButtonState,
    },
    /// The wheel or touchpad scrolled.
    MouseWheel(ScrollDelta),
    /// Raw, unaccelerated mouse movement in device units, for camera control.
    /// It keeps coming while the cursor is [locked](crate::window::CursorGrab::Locked),
    /// when `CursorMoved` does not. Delivered to the window that has keyboard
    /// focus.
    RawMouseMotion {
        /// Horizontal movement, positive to the right.
        delta_x: f64,
        /// Vertical movement, positive downwards.
        delta_y: f64,
    },
    /// A finger touched, moved on or left the screen.
    Touch(TouchEvent),
    /// An input method composed text, see
    /// [`AppContext::set_ime_allowed`](crate::window::AppContext::set_ime_allowed).
    Ime(ImeEvent),
}
