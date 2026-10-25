//! Describing the monitors the application can use.

use super::geometry::{PhysicalPosition, PhysicalSize};

/// One monitor, as reported by the window system.
///
/// Positions and sizes are physical pixels in the desktop's coordinate space,
/// where the primary monitor usually sits at the origin and the others around
/// it (a monitor to the left of the primary one has a negative `x`).
///
/// The list of monitors ([`AppContext::monitors`](super::AppContext::monitors))
/// is read when the app starts (and resumes), when a window is created and
/// whenever a window's scale factor changes; plugging a monitor in or out is
/// picked up at the next of those moments.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MonitorInfo {
    /// A human-readable name ("DELL U2720Q", "Built-in Retina Display"), if
    /// the system gives one.
    pub name: Option<String>,
    /// The top-left corner of the monitor in the desktop's coordinate space.
    pub position: PhysicalPosition,
    /// The size of the monitor in physical pixels.
    pub size: PhysicalSize,
    /// The scale factor of the monitor (`physical = logical * scale_factor`).
    pub scale_factor: f64,
    /// The current refresh rate in millihertz (60 Hz is `60_000`), if known.
    pub refresh_rate_millihertz: Option<u32>,
    /// Whether this is the primary monitor. Systems without that notion
    /// (Wayland) report the first monitor as primary, so a system with a
    /// display always has one.
    pub is_primary: bool,
}

impl MonitorInfo {
    /// A monitor with the given geometry; the name and refresh rate are
    /// unknown and it is not the primary one until set.
    #[must_use]
    pub(crate) fn new(position: PhysicalPosition, size: PhysicalSize, scale_factor: f64) -> Self {
        Self {
            name: None,
            position,
            size,
            scale_factor,
            refresh_rate_millihertz: None,
            is_primary: false,
        }
    }

    /// Sets the name.
    #[must_use]
    pub(crate) fn with_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Sets the refresh rate.
    #[must_use]
    pub(crate) fn with_refresh_rate(mut self, millihertz: Option<u32>) -> Self {
        self.refresh_rate_millihertz = millihertz;
        self
    }

    /// Marks the monitor as the primary one (or not).
    #[must_use]
    pub(crate) fn with_primary(mut self, is_primary: bool) -> Self {
        self.is_primary = is_primary;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_monitor_has_no_optional_data_and_is_not_primary() {
        let monitor = MonitorInfo::new(
            PhysicalPosition::new(-1920, 0),
            PhysicalSize::new(1920, 1080),
            1.0,
        );
        assert_eq!(monitor.name, None);
        assert_eq!(monitor.refresh_rate_millihertz, None);
        assert!(!monitor.is_primary);
        assert_eq!(monitor.position, PhysicalPosition::new(-1920, 0));
    }

    #[test]
    fn builders_fill_the_optional_fields() {
        let monitor = MonitorInfo::new(
            PhysicalPosition::default(),
            PhysicalSize::new(2560, 1440),
            2.0,
        )
            .with_name(Some("Main".to_owned()))
            .with_refresh_rate(Some(143_981))
            .with_primary(true);
        assert_eq!(monitor.name.as_deref(), Some("Main"));
        assert_eq!(monitor.refresh_rate_millihertz, Some(143_981));
        assert!(monitor.is_primary);
    }
}
