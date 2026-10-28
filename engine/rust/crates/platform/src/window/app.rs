//! The application side of the event loop: the [`App`] callbacks and the
//! [`AppContext`] every callback receives.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::cursor::{CursorGrab, CursorIcon};
use super::desc::{Fullscreen, WindowDesc};
use super::error::WindowError;
use super::event::{ControlFlow, WindowEvent, WindowId};
use super::geometry::{LogicalPosition, LogicalSize};
use super::monitor::MonitorInfo;
use super::state::Window;

/// The callbacks of an application, driven by a [`Backend`](super::Backend).
///
/// # Lifecycle
///
/// ```text
///            resumed ──► window_event* / about_to_wait ──► suspended
///               ▲                                              │
///               └──────────────────────────────────────────────┘
/// ```
///
/// * [`resumed`](Self::resumed) is called first, once when the loop starts and
///   again after every [`suspended`](Self::suspended). Create windows here:
///   it is the only point at which mobile platforms guarantee that a window
///   can exist.
/// * [`window_event`](Self::window_event) delivers what happened to a window
///   (see [`WindowEvent`]). Surfaces come and go through
///   `SurfaceCreated`/`SurfaceLost`; when the app is suspended, `SurfaceLost`
///   is delivered for every window that has a surface *before*
///   [`suspended`](Self::suspended).
/// * [`about_to_wait`](Self::about_to_wait) is called once per loop iteration,
///   after all pending events have been delivered and before the loop waits.
///   It is the place for per-frame work: update the simulation and call
///   [`AppContext::request_redraw`].
/// * Redraws (`RedrawRequested`) are delivered after `about_to_wait`, only
///   while the app is resumed.
/// * [`AppContext::exit`] ends the loop once the current callback returns. The
///   remaining windows then receive `SurfaceLost` (if they have a surface) and
///   `Destroyed`.
///
/// All callbacks run on the thread that runs the loop.
pub trait App {
    /// The app is (again) allowed to run and show windows.
    fn resumed(&mut self, _ctx: &mut AppContext) {}

    /// The app is going to the background. Everything tied to a surface has
    /// already been released through `SurfaceLost`; free whatever else must
    /// not live while suspended.
    fn suspended(&mut self, _ctx: &mut AppContext) {}

    /// Something happened to `window`.
    fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent);

    /// All pending events have been delivered; the loop is about to wait.
    fn about_to_wait(&mut self, _ctx: &mut AppContext) {}
}

/// A change the application asked for, for the backend to carry out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Request {
    /// A window was added to the context and must be created for real, from
    /// this description.
    Create(WindowId, Box<WindowDesc>),
    SetTitle(WindowId),
    SetVisible(WindowId),
    SetInnerSize(WindowId, LogicalSize),
    SetPosition(WindowId, LogicalPosition),
    SetFullscreen(WindowId, Fullscreen),
    SetCursorIcon(WindowId),
    SetCursorVisible(WindowId),
    SetCursorGrab(WindowId, CursorGrab),
    SetImeAllowed(WindowId),
    SetImeCursorArea(WindowId, LogicalPosition, LogicalSize),
    /// Close the window: `SurfaceLost` (if it has a surface), `Destroyed`, gone.
    Close(WindowId),
}

/// What a callback can do: create and change windows, ask for redraws, choose
/// how the loop waits and end it.
///
/// Changes are collected while the callback runs and carried out by the
/// backend when it returns. Title and visibility read back immediately;
/// size, position and fullscreen only change when the window system
/// reports them (see [`Window`]).
#[derive(Debug)]
pub struct AppContext {
    pub(super) windows: BTreeMap<WindowId, Window>,
    next_id: u64,
    pub(super) requests: VecDeque<Request>,
    pub(super) redraws: BTreeSet<WindowId>,
    pub(super) control_flow: ControlFlow,
    pub(super) exit_code: Option<i32>,
    /// The scale factor new windows start with, until the window system
    /// reports the real one.
    scale_factor: f64,
    monitors: Vec<MonitorInfo>,
}

impl AppContext {
    /// A context whose new windows start at `scale_factor`.
    pub(crate) fn new(scale_factor: f64) -> Self {
        Self {
            windows: BTreeMap::new(),
            next_id: 1,
            requests: VecDeque::new(),
            redraws: BTreeSet::new(),
            control_flow: ControlFlow::default(),
            exit_code: None,
            scale_factor,
            monitors: Vec::new(),
        }
    }

    /// Replaces the monitor list (and so the scale factor new windows start
    /// with: the primary monitor's, if there is one).
    pub(crate) fn set_monitors(&mut self, monitors: Vec<MonitorInfo>) {
        if let Some(primary) = monitors.iter().find(|m| m.is_primary) {
            self.scale_factor = primary.scale_factor;
        }
        self.monitors = monitors;
    }

    /// Creates a window from `desc` and returns its id. The window exists in
    /// the context right away; a fullscreen request in `desc` is applied by the
    /// backend afterwards (and reported through `Resized`).
    ///
    /// # Errors
    /// [`WindowError::InvalidDesc`] when [`WindowDesc::validate`] fails.
    pub fn create_window(&mut self, desc: WindowDesc) -> Result<WindowId, WindowError> {
        desc.validate()?;
        let id = WindowId::from_u64(self.next_id);
        self.next_id += 1;
        self.windows
            .insert(id, Window::new(id, &desc, self.scale_factor));
        let fullscreen = desc.fullscreen;
        self.requests.push_back(Request::Create(id, Box::new(desc)));
        if fullscreen != Fullscreen::None {
            self.requests
                .push_back(Request::SetFullscreen(id, fullscreen));
        }
        Ok(id)
    }

    /// The window with this id, or `None` if it does not exist (any more).
    #[must_use]
    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    /// All windows, ordered by id.
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }

    fn window_mut(&mut self, id: WindowId) -> Result<&mut Window, WindowError> {
        self.windows
            .get_mut(&id)
            .ok_or(WindowError::UnknownWindow(id))
    }

    /// Asks for the window to be closed: `Destroyed` follows (after
    /// `SurfaceLost`, if it has a surface), then the window is gone. Closing a
    /// window that is already being closed does nothing.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn close_window(&mut self, id: WindowId) -> Result<(), WindowError> {
        self.window_mut(id)?;
        self.requests.push_back(Request::Close(id));
        Ok(())
    }

    /// Sets the title. The new title is readable right away.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_title(&mut self, id: WindowId, title: impl Into<String>) -> Result<(), WindowError> {
        self.window_mut(id)?.set_title(title.into());
        self.requests.push_back(Request::SetTitle(id));
        Ok(())
    }

    /// Shows or hides the window. The new state is readable right away.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_visible(&mut self, id: WindowId, visible: bool) -> Result<(), WindowError> {
        self.window_mut(id)?.set_visible(visible);
        self.requests.push_back(Request::SetVisible(id));
        Ok(())
    }

    /// Asks for a new client-area size (limited to the window's minimum and
    /// maximum). The window system reports the result with
    /// [`Resized`](WindowEvent::Resized).
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`], or [`WindowError::InvalidArgument`]
    /// when `size` is not finite and positive.
    pub fn set_inner_size(&mut self, id: WindowId, size: LogicalSize) -> Result<(), WindowError> {
        self.window_mut(id)?;
        if !size.is_valid() {
            return Err(WindowError::InvalidArgument(
                "size must be finite and greater than zero".to_owned(),
            ));
        }
        self.requests.push_back(Request::SetInnerSize(id, size));
        Ok(())
    }

    /// Asks for the window to move. The window system reports the result with
    /// [`Moved`](WindowEvent::Moved).
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`], or [`WindowError::InvalidArgument`]
    /// when `position` is not finite.
    pub fn set_position(
        &mut self,
        id: WindowId,
        position: LogicalPosition,
    ) -> Result<(), WindowError> {
        self.window_mut(id)?;
        if !position.is_valid() {
            return Err(WindowError::InvalidArgument(
                "position must be finite".to_owned(),
            ));
        }
        self.requests.push_back(Request::SetPosition(id, position));
        Ok(())
    }

    /// Asks to enter or leave fullscreen. The resulting size arrives as
    /// [`Resized`](WindowEvent::Resized).
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_fullscreen(
        &mut self,
        id: WindowId,
        fullscreen: Fullscreen,
    ) -> Result<(), WindowError> {
        self.window_mut(id)?;
        self.requests
            .push_back(Request::SetFullscreen(id, fullscreen));
        Ok(())
    }

    /// Sets the shape of the mouse cursor over the window. The new shape is
    /// readable right away.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_cursor_icon(&mut self, id: WindowId, icon: CursorIcon) -> Result<(), WindowError> {
        self.window_mut(id)?.set_cursor_icon(icon);
        self.requests.push_back(Request::SetCursorIcon(id));
        Ok(())
    }

    /// Shows or hides the mouse cursor over the window. The new state is
    /// readable right away.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_cursor_visible(&mut self, id: WindowId, visible: bool) -> Result<(), WindowError> {
        self.window_mut(id)?.set_cursor_visible(visible);
        self.requests.push_back(Request::SetCursorVisible(id));
        Ok(())
    }

    /// Asks the window to confine or lock the mouse cursor. Not every
    /// platform supports every mode; a refused request leaves the mode
    /// unchanged, so read [`Window::cursor_grab`] for what is in effect.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_cursor_grab(&mut self, id: WindowId, grab: CursorGrab) -> Result<(), WindowError> {
        self.window_mut(id)?;
        self.requests.push_back(Request::SetCursorGrab(id, grab));
        Ok(())
    }

    /// Allows or forbids input-method (IME) events for the window. Enable it
    /// while a text field has focus and disable it again afterwards: with it
    /// on, composition text arrives as
    /// [`ImeEvent`](crate::input::ImeEvent)s and the system may show its
    /// candidate window. The new state is readable right away through
    /// [`Window::is_ime_allowed`].
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn set_ime_allowed(&mut self, id: WindowId, allowed: bool) -> Result<(), WindowError> {
        self.window_mut(id)?.set_ime_allowed(allowed);
        self.requests.push_back(Request::SetImeAllowed(id));
        Ok(())
    }

    /// Tells the input method where the text caret is, so that it can place its
    /// candidate window next to it and not over it. `position` is the top-left
    /// corner of the caret or selection and `size` its extent, in logical units
    /// of the client area.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`], or [`WindowError::InvalidArgument`]
    /// when `position` is not finite or `size` is negative or not finite.
    pub fn set_ime_cursor_area(
        &mut self,
        id: WindowId,
        position: LogicalPosition,
        size: LogicalSize,
    ) -> Result<(), WindowError> {
        self.window_mut(id)?;
        if !position.is_valid() {
            return Err(WindowError::InvalidArgument(
                "position must be finite".to_owned(),
            ));
        }
        let usable = |extent: f64| extent.is_finite() && extent >= 0.0;
        if !usable(size.width) || !usable(size.height) {
            return Err(WindowError::InvalidArgument(
                "size must be finite and not negative".to_owned(),
            ));
        }
        self.requests
            .push_back(Request::SetImeCursorArea(id, position, size));
        Ok(())
    }

    /// The monitors of the desktop, in no particular order (see
    /// [`MonitorInfo::is_primary`]). Empty on a system without a display. See
    /// [`MonitorInfo`] for when the list is refreshed.
    #[must_use]
    pub fn monitors(&self) -> &[MonitorInfo] {
        &self.monitors
    }

    /// The primary monitor, if the system reports one.
    #[must_use]
    pub fn primary_monitor(&self) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.is_primary)
    }

    /// Asks for a [`RedrawRequested`](WindowEvent::RedrawRequested) event for
    /// the window in this loop iteration (after `about_to_wait`) or, when
    /// already inside the redraw, in the next one. Several requests before the
    /// redraw count as one.
    ///
    /// # Errors
    /// [`WindowError::UnknownWindow`].
    pub fn request_redraw(&mut self, id: WindowId) -> Result<(), WindowError> {
        self.window_mut(id)?;
        self.redraws.insert(id);
        Ok(())
    }

    /// Chooses how the loop waits between iterations.
    pub fn set_control_flow(&mut self, control_flow: ControlFlow) {
        self.control_flow = control_flow;
    }

    /// How the loop waits between iterations.
    #[must_use]
    pub fn control_flow(&self) -> ControlFlow {
        self.control_flow
    }

    /// Ends the loop with exit code `0` once the current callback returns.
    pub fn exit(&mut self) {
        self.exit_with_code(0);
    }

    /// Ends the loop with `code` once the current callback returns. The first
    /// call wins; later ones are ignored.
    pub fn exit_with_code(&mut self, code: i32) {
        self.exit_code.get_or_insert(code);
    }

    /// `true` once [`exit`](Self::exit) has been called.
    #[must_use]
    pub fn is_exiting(&self) -> bool {
        self.exit_code.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> AppContext {
        AppContext::new(1.0)
    }

    #[test]
    fn create_window_assigns_increasing_ids_and_requests_creation() {
        let mut ctx = ctx();
        let a = ctx.create_window(WindowDesc::new("A")).unwrap();
        let b = ctx.create_window(WindowDesc::new("B")).unwrap();
        assert!(a < b);
        assert_eq!(ctx.window(a).unwrap().title(), "A");
        assert_eq!(ctx.windows().map(Window::id).collect::<Vec<_>>(), [a, b]);
        assert_eq!(
            ctx.requests,
            [
                Request::Create(a, Box::new(WindowDesc::new("A"))),
                Request::Create(b, Box::new(WindowDesc::new("B")))
            ]
        );
    }

    #[test]
    fn create_window_rejects_an_invalid_description_without_side_effects() {
        let mut ctx = ctx();
        let bad = WindowDesc::default().with_inner_size(LogicalSize::new(0.0, 0.0));
        assert!(matches!(
            ctx.create_window(bad),
            Err(WindowError::InvalidDesc(_))
        ));
        assert_eq!(ctx.windows().count(), 0);
        assert!(ctx.requests.is_empty());
        // The id of the failed attempt is not consumed.
        assert_eq!(
            ctx.create_window(WindowDesc::default()).unwrap(),
            WindowId::from_u64(1)
        );
    }

    #[test]
    fn a_fullscreen_description_queues_the_fullscreen_request() {
        let mut ctx = ctx();
        let id = ctx
            .create_window(WindowDesc::default().with_fullscreen(Fullscreen::Borderless))
            .unwrap();
        assert_eq!(
            ctx.requests,
            [
                Request::Create(
                    id,
                    Box::new(WindowDesc::default().with_fullscreen(Fullscreen::Borderless))
                ),
                Request::SetFullscreen(id, Fullscreen::Borderless)
            ]
        );
        assert_eq!(ctx.window(id).unwrap().fullscreen(), Fullscreen::None);
    }

    #[test]
    fn title_and_visibility_read_back_immediately() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.set_title(id, "Renamed").unwrap();
        ctx.set_visible(id, false).unwrap();
        let window = ctx.window(id).unwrap();
        assert_eq!(window.title(), "Renamed");
        assert!(!window.is_visible());
    }

    #[test]
    fn size_position_and_fullscreen_are_only_requests() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        let before = ctx.window(id).unwrap().inner_size();
        ctx.requests.clear();
        ctx.set_inner_size(id, LogicalSize::new(500.0, 400.0))
            .unwrap();
        ctx.set_position(id, LogicalPosition::new(5.0, 6.0))
            .unwrap();
        ctx.set_fullscreen(id, Fullscreen::Borderless).unwrap();
        assert_eq!(ctx.window(id).unwrap().inner_size(), before);
        assert_eq!(ctx.window(id).unwrap().fullscreen(), Fullscreen::None);
        assert_eq!(
            ctx.requests,
            [
                Request::SetInnerSize(id, LogicalSize::new(500.0, 400.0)),
                Request::SetPosition(id, LogicalPosition::new(5.0, 6.0)),
                Request::SetFullscreen(id, Fullscreen::Borderless),
            ]
        );
    }

    #[test]
    fn invalid_arguments_are_rejected() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.requests.clear();
        assert!(matches!(
            ctx.set_inner_size(id, LogicalSize::new(-1.0, 10.0)),
            Err(WindowError::InvalidArgument(_))
        ));
        assert!(matches!(
            ctx.set_position(id, LogicalPosition::new(f64::NAN, 0.0)),
            Err(WindowError::InvalidArgument(_))
        ));
        assert!(ctx.requests.is_empty());
    }

    #[test]
    fn unknown_windows_are_reported_everywhere() {
        let mut ctx = ctx();
        let ghost = WindowId::from_u64(99);
        assert!(ctx.window(ghost).is_none());
        for result in [
            ctx.close_window(ghost),
            ctx.set_title(ghost, "x"),
            ctx.set_visible(ghost, true),
            ctx.set_inner_size(ghost, LogicalSize::new(1.0, 1.0)),
            ctx.set_position(ghost, LogicalPosition::new(0.0, 0.0)),
            ctx.set_fullscreen(ghost, Fullscreen::None),
            ctx.request_redraw(ghost),
            ctx.set_cursor_icon(ghost, CursorIcon::Text),
            ctx.set_cursor_visible(ghost, false),
            ctx.set_cursor_grab(ghost, CursorGrab::Locked),
            ctx.set_ime_allowed(ghost, true),
            ctx.set_ime_cursor_area(
                ghost,
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1.0, 1.0),
            ),
        ] {
            assert!(matches!(result, Err(WindowError::UnknownWindow(id)) if id == ghost));
        }
        assert!(ctx.requests.is_empty() && ctx.redraws.is_empty());
    }

    #[test]
    fn cursor_icon_and_visibility_read_back_immediately_and_the_grab_is_only_a_request() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.requests.clear();
        ctx.set_cursor_icon(id, CursorIcon::Text).unwrap();
        ctx.set_cursor_visible(id, false).unwrap();
        ctx.set_cursor_grab(id, CursorGrab::Confined).unwrap();
        let window = ctx.window(id).unwrap();
        assert_eq!(window.cursor_icon(), CursorIcon::Text);
        assert!(!window.is_cursor_visible());
        assert_eq!(window.cursor_grab(), CursorGrab::None);
        assert_eq!(
            ctx.requests,
            [
                Request::SetCursorIcon(id),
                Request::SetCursorVisible(id),
                Request::SetCursorGrab(id, CursorGrab::Confined)
            ]
        );
    }

    #[test]
    fn ime_permission_reads_back_immediately_and_the_cursor_area_is_a_request() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.requests.clear();
        assert!(!ctx.window(id).unwrap().is_ime_allowed());
        ctx.set_ime_allowed(id, true).unwrap();
        assert!(ctx.window(id).unwrap().is_ime_allowed());
        ctx.set_ime_cursor_area(
            id,
            LogicalPosition::new(4.0, 5.0),
            LogicalSize::new(2.0, 0.0),
        )
            .unwrap();
        assert_eq!(
            ctx.requests,
            [
                Request::SetImeAllowed(id),
                Request::SetImeCursorArea(
                    id,
                    LogicalPosition::new(4.0, 5.0),
                    LogicalSize::new(2.0, 0.0)
                )
            ]
        );
    }

    #[test]
    fn an_unusable_ime_cursor_area_is_rejected() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.requests.clear();
        for (position, size) in [
            (
                LogicalPosition::new(f64::NAN, 0.0),
                LogicalSize::new(1.0, 1.0),
            ),
            (LogicalPosition::new(0.0, 0.0), LogicalSize::new(-1.0, 1.0)),
            (
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1.0, f64::INFINITY),
            ),
        ] {
            assert!(matches!(
                ctx.set_ime_cursor_area(id, position, size),
                Err(WindowError::InvalidArgument(_))
            ));
        }
        assert!(ctx.requests.is_empty());
    }

    #[test]
    fn monitors_come_from_the_backend_and_set_the_scale_of_new_windows() {
        use super::super::geometry::{PhysicalPosition, PhysicalSize};
        let mut ctx = ctx();
        assert!(ctx.monitors().is_empty() && ctx.primary_monitor().is_none());
        let secondary = MonitorInfo::new(
            PhysicalPosition::new(-1920, 0),
            PhysicalSize::new(1920, 1080),
            1.0,
        );
        let primary = MonitorInfo::new(
            PhysicalPosition::default(),
            PhysicalSize::new(3840, 2160),
            2.0,
        )
            .with_name(Some("Main".to_owned()))
            .with_primary(true);
        ctx.set_monitors(vec![secondary, primary]);
        assert_eq!(ctx.monitors().len(), 2);
        assert_eq!(ctx.primary_monitor().unwrap().name.as_deref(), Some("Main"));
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        assert_eq!(ctx.window(id).unwrap().scale_factor(), 2.0);
    }

    #[test]
    fn redraw_requests_coalesce() {
        let mut ctx = ctx();
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        ctx.request_redraw(id).unwrap();
        ctx.request_redraw(id).unwrap();
        assert_eq!(ctx.redraws.len(), 1);
    }

    #[test]
    fn the_first_exit_wins() {
        let mut ctx = ctx();
        assert!(!ctx.is_exiting());
        ctx.exit_with_code(3);
        ctx.exit_with_code(9);
        ctx.exit();
        assert!(ctx.is_exiting());
        assert_eq!(ctx.exit_code, Some(3));
    }

    #[test]
    fn control_flow_is_settable() {
        let mut ctx = ctx();
        assert_eq!(ctx.control_flow(), ControlFlow::Wait);
        ctx.set_control_flow(ControlFlow::Poll);
        assert_eq!(ctx.control_flow(), ControlFlow::Poll);
    }

    #[test]
    fn new_windows_start_at_the_context_scale_factor() {
        let mut ctx = AppContext::new(2.0);
        let id = ctx.create_window(WindowDesc::default()).unwrap();
        assert_eq!(ctx.window(id).unwrap().scale_factor(), 2.0);
        assert_eq!(ctx.window(id).unwrap().inner_size().width, 2560);
    }
}
