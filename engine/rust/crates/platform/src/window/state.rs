//! The readable state of a window.

use super::cursor::{CursorGrab, CursorIcon};
use super::desc::{Fullscreen, WindowDesc};
use super::event::WindowId;
use super::geometry::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use super::handle::NativeWindowHandle;

/// A window as the application sees it: a snapshot of its current state,
/// obtained from [`AppContext::window`](super::AppContext::window).
///
/// The state is what the window system last *reported*. Changing a window
/// goes through [`AppContext`](super::AppContext); title and visibility are
/// owned by the application and read back immediately, while size, position
/// and fullscreen are requests that the window system may adjust or refuse,
/// so they only change when the matching event
/// ([`Resized`](super::WindowEvent::Resized), [`Moved`](super::WindowEvent::Moved))
/// arrives.
#[derive(Debug, Clone)]
pub struct Window {
    id: WindowId,
    title: String,
    inner_size: PhysicalSize,
    scale_factor: f64,
    position: Option<PhysicalPosition>,
    min_inner_size: Option<LogicalSize>,
    max_inner_size: Option<LogicalSize>,
    resizable: bool,
    visible: bool,
    decorations: bool,
    fullscreen: Fullscreen,
    focused: bool,
    occluded: bool,
    surface: Option<NativeWindowHandle>,
    cursor_icon: CursorIcon,
    cursor_visible: bool,
    cursor_grab: CursorGrab,
}

impl Window {
    /// A window for `desc` at `scale_factor`. Fullscreen starts as
    /// [`Fullscreen::None`]: the backend applies the requested mode.
    pub(crate) fn new(id: WindowId, desc: &WindowDesc, scale_factor: f64) -> Self {
        Self {
            id,
            title: desc.title.clone(),
            inner_size: desc.inner_size.to_physical(scale_factor),
            scale_factor,
            position: desc.position.map(|p| p.to_physical(scale_factor)),
            min_inner_size: desc.min_inner_size,
            max_inner_size: desc.max_inner_size,
            resizable: desc.resizable,
            visible: desc.visible,
            decorations: desc.decorations,
            fullscreen: Fullscreen::None,
            focused: false,
            occluded: false,
            surface: None,
            cursor_icon: CursorIcon::Default,
            cursor_visible: true,
            cursor_grab: CursorGrab::None,
        }
    }

    /// The id of the window.
    #[must_use]
    pub fn id(&self) -> WindowId {
        self.id
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The size of the client area in physical pixels: the size of the
    /// swapchain to create.
    #[must_use]
    pub fn inner_size(&self) -> PhysicalSize {
        self.inner_size
    }

    /// The size of the client area in logical units.
    #[must_use]
    pub fn logical_inner_size(&self) -> LogicalSize {
        self.inner_size.to_logical(self.scale_factor)
    }

    /// The scale factor of the monitor the window is on (`physical = logical
    /// * scale_factor`).
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The position of the top-left corner in physical pixels, or `None` while
    /// the window system has not reported one.
    #[must_use]
    pub fn position(&self) -> Option<PhysicalPosition> {
        self.position
    }

    /// The position in logical units, if known.
    #[must_use]
    pub fn logical_position(&self) -> Option<LogicalPosition> {
        self.position.map(|p| p.to_logical(self.scale_factor))
    }

    /// The smallest client area the window can be given.
    #[must_use]
    pub fn min_inner_size(&self) -> Option<LogicalSize> {
        self.min_inner_size
    }

    /// The largest client area the window can be given.
    #[must_use]
    pub fn max_inner_size(&self) -> Option<LogicalSize> {
        self.max_inner_size
    }

    /// Whether the user can resize the window.
    #[must_use]
    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// Whether the window is shown.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Whether the window has a title bar and borders.
    #[must_use]
    pub fn has_decorations(&self) -> bool {
        self.decorations
    }

    /// The current fullscreen mode.
    #[must_use]
    pub fn fullscreen(&self) -> Fullscreen {
        self.fullscreen
    }

    /// Whether the window has keyboard focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Whether the window is fully hidden (minimized or covered).
    #[must_use]
    pub fn is_occluded(&self) -> bool {
        self.occluded
    }

    /// The handle of the window's presentation surface, or `None` while it has
    /// none: before
    /// [`SurfaceCreated`](super::WindowEvent::SurfaceCreated), after
    /// [`SurfaceLost`](super::WindowEvent::SurfaceLost), and always for a
    /// headless window that no test has attached a surface to.
    #[must_use]
    pub fn native_handle(&self) -> Option<NativeWindowHandle> {
        self.surface
    }

    /// The shape of the mouse cursor over the window.
    #[must_use]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    /// Whether the mouse cursor is shown over the window.
    #[must_use]
    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// The cursor grab mode in effect. A request the platform refused does not
    /// show up here.
    #[must_use]
    pub fn cursor_grab(&self) -> CursorGrab {
        self.cursor_grab
    }

    // Mutators for the application-owned state; used by `AppContext`.

    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub(crate) fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub(crate) fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.cursor_icon = icon;
    }

    pub(crate) fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    // Mutators for state the window system reports; used by the backends.

    pub(crate) fn report_cursor_grab(&mut self, grab: CursorGrab) {
        self.cursor_grab = grab;
    }

    pub(crate) fn report_inner_size(&mut self, size: PhysicalSize) {
        self.inner_size = size;
    }

    pub(crate) fn report_position(&mut self, position: PhysicalPosition) {
        self.position = Some(position);
    }

    pub(crate) fn report_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub(crate) fn report_fullscreen(&mut self, fullscreen: Fullscreen) {
        self.fullscreen = fullscreen;
    }

    pub(crate) fn report_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub(crate) fn report_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
    }

    pub(crate) fn report_surface(&mut self, surface: Option<NativeWindowHandle>) {
        self.surface = surface;
    }

    /// `size` limited to the window's minimum and maximum, which are logical
    /// and converted at the window's scale factor.
    pub(crate) fn clamp_inner_size(&self, size: PhysicalSize) -> PhysicalSize {
        let mut size = size;
        if let Some(min) = self
            .min_inner_size
            .map(|s| s.to_physical(self.scale_factor))
        {
            size = PhysicalSize::new(size.width.max(min.width), size.height.max(min.height));
        }
        if let Some(max) = self
            .max_inner_size
            .map(|s| s.to_physical(self.scale_factor))
        {
            size = PhysicalSize::new(size.width.min(max.width), size.height.min(max.height));
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(desc: &WindowDesc, scale: f64) -> Window {
        Window::new(WindowId::from_u64(1), desc, scale)
    }

    #[test]
    fn new_window_reflects_the_description_at_the_scale_factor() {
        let desc = WindowDesc::new("Hello")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_position(LogicalPosition::new(10.0, 20.0))
            .with_resizable(false)
            .with_visible(false)
            .with_decorations(false);
        let w = window(&desc, 2.0);
        assert_eq!(w.id(), WindowId::from_u64(1));
        assert_eq!(w.title(), "Hello");
        assert_eq!(w.inner_size(), PhysicalSize::new(1600, 1200));
        assert_eq!(w.logical_inner_size(), LogicalSize::new(800.0, 600.0));
        assert_eq!(w.scale_factor(), 2.0);
        assert_eq!(w.position(), Some(PhysicalPosition::new(20, 40)));
        assert_eq!(w.logical_position(), Some(LogicalPosition::new(10.0, 20.0)));
        assert!(!w.is_resizable() && !w.is_visible() && !w.has_decorations());
        assert_eq!(w.fullscreen(), Fullscreen::None);
        assert!(!w.is_focused() && !w.is_occluded());
        assert!(w.native_handle().is_none());
        assert_eq!(w.cursor_icon(), CursorIcon::Default);
        assert!(w.is_cursor_visible());
        assert_eq!(w.cursor_grab(), CursorGrab::None);
    }

    #[test]
    fn a_fullscreen_description_starts_windowed_until_the_backend_applies_it() {
        let desc = WindowDesc::default().with_fullscreen(Fullscreen::Borderless);
        assert_eq!(window(&desc, 1.0).fullscreen(), Fullscreen::None);
    }

    #[test]
    fn clamping_uses_the_logical_limits_at_the_scale_factor() {
        let desc = WindowDesc::default()
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_min_inner_size(LogicalSize::new(400.0, 300.0))
            .with_max_inner_size(LogicalSize::new(1000.0, 700.0));
        let w = window(&desc, 2.0);
        assert_eq!(
            w.clamp_inner_size(PhysicalSize::new(100, 100)),
            PhysicalSize::new(800, 600)
        );
        assert_eq!(
            w.clamp_inner_size(PhysicalSize::new(5000, 5000)),
            PhysicalSize::new(2000, 1400)
        );
        assert_eq!(
            w.clamp_inner_size(PhysicalSize::new(1000, 1000)),
            PhysicalSize::new(1000, 1000)
        );
        let unlimited = window(&WindowDesc::default(), 1.0);
        assert_eq!(
            unlimited.clamp_inner_size(PhysicalSize::new(1, 99_999)),
            PhysicalSize::new(1, 99_999)
        );
    }
}
