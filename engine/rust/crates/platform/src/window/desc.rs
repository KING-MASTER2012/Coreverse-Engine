//! Describing a window to create.

use super::error::WindowError;
use super::geometry::{LogicalPosition, LogicalSize};

/// Whether and how a window covers its monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Fullscreen {
    /// A normal window.
    #[default]
    None,
    /// A borderless window the size of its monitor. Switching to and from it
    /// never changes the video mode.
    Borderless,
}

/// Everything needed to create a window, built with chained `with_*` calls:
///
/// ```
/// use platform::window::{LogicalSize, WindowDesc};
///
/// let desc = WindowDesc::new("Editor").with_inner_size(LogicalSize::new(1600.0, 900.0)).with_resizable(false);
/// assert_eq!(desc.title, "Editor");
/// assert!(desc.validate().is_ok());
/// ```
///
/// Sizes and positions are logical (DPI-independent) units; the window system
/// converts them with the monitor's scale factor. The struct is
/// `#[non_exhaustive]`, so fields can be added later without breaking
/// callers; use [`WindowDesc::default`] or [`WindowDesc::new`] and the
/// builder methods.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct WindowDesc {
    /// The window title. Default: `"Coreverse"`.
    pub title: String,
    /// The size of the client area. Default: 1280 x 720.
    pub inner_size: LogicalSize,
    /// The position of the window's top-left corner, or `None` to let the
    /// window system choose. Default: `None`.
    pub position: Option<LogicalPosition>,
    /// The smallest client area the window can be given. Default: none.
    pub min_inner_size: Option<LogicalSize>,
    /// The largest client area the window can be given. Default: none.
    pub max_inner_size: Option<LogicalSize>,
    /// Whether the user can resize the window. Default: `true`.
    pub resizable: bool,
    /// Whether the window is shown. Default: `true`.
    pub visible: bool,
    /// Whether the window has a title bar and borders. Default: `true`.
    pub decorations: bool,
    /// Whether the window starts fullscreen. Default: [`Fullscreen::None`].
    pub fullscreen: Fullscreen,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            title: String::from("Coreverse"),
            inner_size: LogicalSize::new(1280.0, 720.0),
            position: None,
            min_inner_size: None,
            max_inner_size: None,
            resizable: true,
            visible: true,
            decorations: true,
            fullscreen: Fullscreen::None,
        }
    }
}

impl WindowDesc {
    /// A description with `title` and the defaults for everything else.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Sets the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the client-area size.
    #[must_use]
    pub fn with_inner_size(mut self, size: LogicalSize) -> Self {
        self.inner_size = size;
        self
    }

    /// Sets the initial position.
    #[must_use]
    pub fn with_position(mut self, position: LogicalPosition) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the smallest client-area size.
    #[must_use]
    pub fn with_min_inner_size(mut self, size: LogicalSize) -> Self {
        self.min_inner_size = Some(size);
        self
    }

    /// Sets the largest client-area size.
    #[must_use]
    pub fn with_max_inner_size(mut self, size: LogicalSize) -> Self {
        self.max_inner_size = Some(size);
        self
    }

    /// Sets whether the user can resize the window.
    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets whether the window is shown.
    #[must_use]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets whether the window has a title bar and borders.
    #[must_use]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Sets whether the window starts fullscreen.
    #[must_use]
    pub fn with_fullscreen(mut self, fullscreen: Fullscreen) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Checks that the description can be used: every size is finite and
    /// positive, the position is finite, the minimum does not exceed the
    /// maximum, and the initial size lies between them.
    ///
    /// # Errors
    /// [`WindowError::InvalidDesc`] naming the first problem found.
    pub fn validate(&self) -> Result<(), WindowError> {
        let bad = |what: &str| Err(WindowError::InvalidDesc(what.to_owned()));
        if !self.inner_size.is_valid() {
            return bad("inner_size must be finite and greater than zero");
        }
        if self.position.is_some_and(|p| !p.is_valid()) {
            return bad("position must be finite");
        }
        if self.min_inner_size.is_some_and(|s| !s.is_valid()) {
            return bad("min_inner_size must be finite and greater than zero");
        }
        if self.max_inner_size.is_some_and(|s| !s.is_valid()) {
            return bad("max_inner_size must be finite and greater than zero");
        }
        if let (Some(min), Some(max)) = (self.min_inner_size, self.max_inner_size)
            && (min.width > max.width || min.height > max.height)
        {
            return bad("min_inner_size must not exceed max_inner_size");
        }
        if let Some(min) = self.min_inner_size
            && (self.inner_size.width < min.width || self.inner_size.height < min.height)
        {
            return bad("inner_size is smaller than min_inner_size");
        }
        if let Some(max) = self.max_inner_size
            && (self.inner_size.width > max.width || self.inner_size.height > max.height)
        {
            return bad("inner_size is larger than max_inner_size");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invalid_message(desc: &WindowDesc) -> String {
        match desc.validate() {
            Err(WindowError::InvalidDesc(message)) => message,
            other => panic!("expected InvalidDesc, got {other:?}"),
        }
    }

    #[test]
    fn defaults_are_valid_and_documented() {
        let desc = WindowDesc::default();
        assert_eq!(desc.title, "Coreverse");
        assert_eq!(desc.inner_size, LogicalSize::new(1280.0, 720.0));
        assert!(desc.resizable && desc.visible && desc.decorations);
        assert_eq!(desc.fullscreen, Fullscreen::None);
        assert!(
            desc.position.is_none()
                && desc.min_inner_size.is_none()
                && desc.max_inner_size.is_none()
        );
        desc.validate().unwrap();
    }

    #[test]
    fn builder_sets_every_field() {
        let desc = WindowDesc::new("A")
            .with_title("B")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_position(LogicalPosition::new(10.0, 20.0))
            .with_min_inner_size(LogicalSize::new(400.0, 300.0))
            .with_max_inner_size(LogicalSize::new(1600.0, 1200.0))
            .with_resizable(false)
            .with_visible(false)
            .with_decorations(false)
            .with_fullscreen(Fullscreen::Borderless);
        assert_eq!(desc.title, "B");
        assert_eq!(desc.inner_size, LogicalSize::new(800.0, 600.0));
        assert_eq!(desc.position, Some(LogicalPosition::new(10.0, 20.0)));
        assert_eq!(desc.min_inner_size, Some(LogicalSize::new(400.0, 300.0)));
        assert_eq!(desc.max_inner_size, Some(LogicalSize::new(1600.0, 1200.0)));
        assert!(!desc.resizable && !desc.visible && !desc.decorations);
        assert_eq!(desc.fullscreen, Fullscreen::Borderless);
        desc.validate().unwrap();
    }

    #[test]
    fn rejects_unusable_sizes() {
        for size in [
            LogicalSize::new(0.0, 100.0),
            LogicalSize::new(100.0, -1.0),
            LogicalSize::new(f64::NAN, 100.0),
        ] {
            let message = invalid_message(&WindowDesc::default().with_inner_size(size));
            assert!(message.contains("inner_size"), "{message}");
        }
        assert!(
            invalid_message(&WindowDesc::default().with_min_inner_size(LogicalSize::new(0.0, 1.0)))
                .contains("min_inner_size")
        );
        assert!(
            invalid_message(
                &WindowDesc::default().with_max_inner_size(LogicalSize::new(1.0, f64::INFINITY))
            )
                .contains("max_inner_size")
        );
        assert!(
            invalid_message(
                &WindowDesc::default().with_position(LogicalPosition::new(f64::NAN, 0.0))
            )
                .contains("position")
        );
    }

    #[test]
    fn rejects_inconsistent_limits() {
        let min_over_max = WindowDesc::default()
            .with_min_inner_size(LogicalSize::new(900.0, 100.0))
            .with_max_inner_size(LogicalSize::new(800.0, 800.0));
        assert!(invalid_message(&min_over_max).contains("must not exceed"));

        let below_min = WindowDesc::default().with_min_inner_size(LogicalSize::new(2000.0, 100.0));
        assert!(invalid_message(&below_min).contains("smaller than"));

        let above_max = WindowDesc::default().with_max_inner_size(LogicalSize::new(1000.0, 1000.0));
        assert!(invalid_message(&above_max).contains("larger than"));
    }
}
