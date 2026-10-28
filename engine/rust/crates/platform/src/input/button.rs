//! Button states and mouse buttons.

/// Whether a key or button went down or up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ButtonState {
    /// The key or button went down.
    Pressed,
    /// The key or button went up.
    Released,
}

impl ButtonState {
    /// `true` for [`Pressed`](Self::Pressed).
    #[must_use]
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MouseButton {
    /// The primary button, normally the left one.
    Left,
    /// The secondary button, normally the right one.
    Right,
    /// The wheel button.
    Middle,
    /// The "back" side button.
    Back,
    /// The "forward" side button.
    Forward,
    /// Any other button, by the number the window system gives it.
    Other(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_state_knows_whether_it_is_pressed() {
        assert!(ButtonState::Pressed.is_pressed());
        assert!(!ButtonState::Released.is_pressed());
    }

    #[test]
    fn mouse_buttons_are_ordered_and_hashable() {
        use std::collections::BTreeSet;
        let set: BTreeSet<MouseButton> =
            [MouseButton::Other(9), MouseButton::Left, MouseButton::Right]
                .into_iter()
                .collect();
        assert_eq!(set.len(), 3);
        assert_eq!(set.first(), Some(&MouseButton::Left));
    }
}
