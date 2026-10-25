//! The mouse cursor of a window.

/// The shape of the mouse cursor over a window.
///
/// A small portable set; each backend maps it to what the operating system
/// calls the same thing. The enum is `#[non_exhaustive]`, more shapes can be
/// added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CursorIcon {
    /// The platform's normal arrow.
    #[default]
    Default,
    /// A pointing hand: something can be clicked.
    Pointer,
    /// An I-beam: text can be selected or edited.
    Text,
    /// A crosshair, for precise picking.
    Crosshair,
    /// The system is busy and the app cannot respond (an hourglass or
    /// spinner).
    Wait,
    /// The system is busy but the app still responds.
    Progress,
    /// Help is available.
    Help,
    /// The action under the cursor is not allowed.
    NotAllowed,
    /// An open hand: something can be grabbed.
    Grab,
    /// A closed hand: something is being dragged.
    Grabbing,
    /// Something can be moved.
    Move,
    /// Resizing horizontally (left-right).
    ResizeHorizontal,
    /// Resizing vertically (up-down).
    ResizeVertical,
}

/// How a window holds on to the mouse cursor.
///
/// Support differs by platform: Windows and X11 confine, macOS locks, Wayland
/// does both, and each refuses what it cannot do. A window that was refused
/// keeps its previous mode; [`Window::cursor_grab`](super::Window::cursor_grab)
/// always reports the mode in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CursorGrab {
    /// The cursor moves freely.
    #[default]
    None,
    /// The cursor cannot leave the window but still moves inside it. For
    /// strategy games and editors that must not lose the mouse.
    Confined,
    /// The cursor is fixed in place and only relative motion is reported. For
    /// first-person cameras.
    Locked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_neutral_ones() {
        assert_eq!(CursorIcon::default(), CursorIcon::Default);
        assert_eq!(CursorGrab::default(), CursorGrab::None);
    }
}
