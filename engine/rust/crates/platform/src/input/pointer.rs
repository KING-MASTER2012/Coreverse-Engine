//! Positions, scrolling and touches.

/// A point in the client area of a window, in physical pixels, measured from
/// its top-left corner. Fractional, because mice and touch screens report
/// sub-pixel positions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointerPosition {
    /// Horizontal offset in physical pixels.
    pub x: f64,
    /// Vertical offset in physical pixels, growing downwards.
    pub y: f64,
}

impl PointerPosition {
    /// The point `(x, y)`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// How far the mouse wheel (or a touchpad) scrolled.
///
/// Positive values mean the scrolled content moves right and down, the
/// convention of `winit`: rolling a wheel away from you gives a positive `y`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ScrollDelta {
    /// Whole or fractional lines, as a notched wheel reports.
    Lines {
        /// Horizontal amount.
        x: f64,
        /// Vertical amount.
        y: f64,
    },
    /// Physical pixels, as a touchpad reports.
    Pixels {
        /// Horizontal amount.
        x: f64,
        /// Vertical amount.
        y: f64,
    },
}

/// Where a touch is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TouchPhase {
    /// A finger touched the screen.
    Started,
    /// The finger moved.
    Moved,
    /// The finger was lifted.
    Ended,
    /// The system took the touch away (a palm, a system gesture).
    Cancelled,
}

/// One step of one touch.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct TouchEvent {
    /// Identifies the finger from [`Started`](TouchPhase::Started) to its end;
    /// ids may be reused afterwards.
    pub id: u64,
    /// What happened.
    pub phase: TouchPhase,
    /// Where, in physical pixels.
    pub position: PointerPosition,
}

impl TouchEvent {
    /// A touch step of finger `id`.
    #[must_use]
    pub const fn new(id: u64, phase: TouchPhase, position: PointerPosition) -> Self {
        Self {
            id,
            phase,
            position,
        }
    }
}

/// A finger on the screen, as [`InputState`](super::InputState) tracks it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct TouchPoint {
    /// The finger's id.
    pub id: u64,
    /// Where it is now (for a finished touch: where it ended).
    pub position: PointerPosition,
    /// Where it first touched the screen.
    pub start_position: PointerPosition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_fill_the_fields() {
        let p = PointerPosition::new(1.5, 2.5);
        assert_eq!((p.x, p.y), (1.5, 2.5));
        let t = TouchEvent::new(7, TouchPhase::Started, p);
        assert_eq!(t.id, 7);
        assert_eq!(t.phase, TouchPhase::Started);
        assert_eq!(t.position, p);
        assert_eq!(PointerPosition::default(), PointerPosition::new(0.0, 0.0));
    }
}
