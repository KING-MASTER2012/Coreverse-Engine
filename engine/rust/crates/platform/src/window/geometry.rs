//! Sizes and positions, in physical pixels and in logical (DPI-independent)
//! units.
//!
//! A *logical* unit is what the application asks for ("a 1280 x 720 window");
//! a *physical* unit is a real pixel of the display. They differ by the
//! window's scale factor (`1.0` on a classic display, `2.0` on a typical
//! high-DPI one): `physical = logical * scale_factor`. Requests to the window
//! system are in logical units, the events and sizes it reports are physical,
//! because that is what a swapchain needs.
//!
//! Every conversion needs a finite, positive scale factor; callers are
//! expected to have validated it (the window system never reports anything
//! else).

/// A size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PhysicalSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// A size of `width` x `height` physical pixels.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// This size in logical units at `scale_factor`.
    #[must_use]
    pub fn to_logical(self, scale_factor: f64) -> LogicalSize {
        LogicalSize::new(
            f64::from(self.width) / scale_factor,
            f64::from(self.height) / scale_factor,
        )
    }
}

/// A size in logical (DPI-independent) units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LogicalSize {
    /// Width in logical units.
    pub width: f64,
    /// Height in logical units.
    pub height: f64,
}

impl LogicalSize {
    /// A size of `width` x `height` logical units.
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// This size in physical pixels at `scale_factor`, rounded to the nearest
    /// pixel (and saturated at the `u32` range).
    #[must_use]
    pub fn to_physical(self, scale_factor: f64) -> PhysicalSize {
        // `as` from f64 to u32 saturates (and maps NaN to 0), which is the
        // wanted behavior for out-of-range input.
        PhysicalSize::new(
            (self.width * scale_factor).round() as u32,
            (self.height * scale_factor).round() as u32,
        )
    }

    /// `true` when both extents are finite and greater than zero.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.width.is_finite() && self.height.is_finite() && self.width > 0.0 && self.height > 0.0
    }
}

/// A position in physical pixels, relative to the top-left of the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PhysicalPosition {
    /// Horizontal offset in physical pixels.
    pub x: i32,
    /// Vertical offset in physical pixels.
    pub y: i32,
}

impl PhysicalPosition {
    /// The position (`x`, `y`) in physical pixels.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// This position in logical units at `scale_factor`.
    #[must_use]
    pub fn to_logical(self, scale_factor: f64) -> LogicalPosition {
        LogicalPosition::new(
            f64::from(self.x) / scale_factor,
            f64::from(self.y) / scale_factor,
        )
    }
}

/// A position in logical (DPI-independent) units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LogicalPosition {
    /// Horizontal offset in logical units.
    pub x: f64,
    /// Vertical offset in logical units.
    pub y: f64,
}

impl LogicalPosition {
    /// The position (`x`, `y`) in logical units.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// This position in physical pixels at `scale_factor`, rounded to the
    /// nearest pixel (and saturated at the `i32` range).
    #[must_use]
    pub fn to_physical(self, scale_factor: f64) -> PhysicalPosition {
        PhysicalPosition::new(
            (self.x * scale_factor).round() as i32,
            (self.y * scale_factor).round() as i32,
        )
    }

    /// `true` when both coordinates are finite.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_to_physical_scales_and_rounds() {
        assert_eq!(
            LogicalSize::new(1280.0, 720.0).to_physical(1.0),
            PhysicalSize::new(1280, 720)
        );
        assert_eq!(
            LogicalSize::new(1280.0, 720.0).to_physical(1.5),
            PhysicalSize::new(1920, 1080)
        );
        assert_eq!(
            LogicalSize::new(100.0, 100.0).to_physical(1.25),
            PhysicalSize::new(125, 125)
        );
        // 101 * 1.25 = 126.25 rounds down, 103 * 1.25 = 128.75 rounds up.
        assert_eq!(
            LogicalSize::new(101.0, 103.0).to_physical(1.25),
            PhysicalSize::new(126, 129)
        );
    }

    #[test]
    fn physical_to_logical_divides() {
        assert_eq!(
            PhysicalSize::new(1920, 1080).to_logical(1.5),
            LogicalSize::new(1280.0, 720.0)
        );
        assert_eq!(
            PhysicalPosition::new(300, -150).to_logical(2.0),
            LogicalPosition::new(150.0, -75.0)
        );
    }

    #[test]
    fn round_trip_is_exact_for_common_scale_factors() {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let physical = PhysicalSize::new(1920, 1080);
            assert_eq!(
                physical.to_logical(scale).to_physical(scale),
                physical,
                "scale {scale}"
            );
        }
    }

    #[test]
    fn positions_round_and_keep_their_sign() {
        assert_eq!(
            LogicalPosition::new(10.0, -10.0).to_physical(2.0),
            PhysicalPosition::new(20, -20)
        );
        assert_eq!(
            LogicalPosition::new(0.4, -0.4).to_physical(1.0),
            PhysicalPosition::new(0, 0)
        );
        assert_eq!(
            LogicalPosition::new(0.6, -0.6).to_physical(1.0),
            PhysicalPosition::new(1, -1)
        );
    }

    #[test]
    fn out_of_range_values_saturate() {
        assert_eq!(
            LogicalSize::new(1e20, -5.0).to_physical(1.0),
            PhysicalSize::new(u32::MAX, 0)
        );
        assert_eq!(
            LogicalPosition::new(1e20, -1e20).to_physical(1.0),
            PhysicalPosition::new(i32::MAX, i32::MIN)
        );
    }

    #[test]
    fn validity_requires_finite_positive_sizes() {
        assert!(LogicalSize::new(1.0, 1.0).is_valid());
        assert!(!LogicalSize::new(0.0, 1.0).is_valid());
        assert!(!LogicalSize::new(1.0, -1.0).is_valid());
        assert!(!LogicalSize::new(f64::NAN, 1.0).is_valid());
        assert!(!LogicalSize::new(1.0, f64::INFINITY).is_valid());
        assert!(LogicalPosition::new(-5.0, 5.0).is_valid());
        assert!(!LogicalPosition::new(f64::NAN, 0.0).is_valid());
    }
}
