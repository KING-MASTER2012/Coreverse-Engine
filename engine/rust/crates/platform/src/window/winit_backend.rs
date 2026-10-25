//! The desktop window backend, built on `winit`.
//!
//! Enabled by the `winit` cargo feature. It opens real windows on Windows,
//! macOS and Linux (X11 and Wayland) and delivers exactly the callbacks the
//! [`App`] contract describes, so an app written against the headless backend
//! runs unchanged.
//!
//! What it adds to the contract on a real desktop:
//!
//! * A window's surface exists as soon as the window does:
//!   [`SurfaceCreated`](WindowEvent::SurfaceCreated) is delivered right after
//!   the window is created, with the native handle of the window system the
//!   session uses.
//! * Sizes the window system reports are physical pixels at the monitor's scale
//!   factor; when the window moves to a monitor with another scale factor
//!   [`ScaleFactorChanged`](WindowEvent::ScaleFactorChanged) keeps the logical
//!   size.
//! * [`MonitorInfo`] is read from the system (see its documentation for when).
//! * [`Resized`](WindowEvent::Resized) is only delivered when the size really
//!   changed.
//! * A window that cannot be created is fatal: the loop exits and
//!   [`Backend::run`] returns [`WindowError::Backend`].
//!
//! Window creation, the event loop and every callback run on the thread that
//! calls [`run`](Backend::run), which on macOS must be the main thread.

use std::collections::{BTreeMap, HashMap};
use std::num::{NonZeroU64, NonZeroUsize};

use winit::application::ApplicationHandler;
use winit::dpi::{
    LogicalPosition as WinitLogicalPosition, LogicalSize as WinitLogicalSize,
    PhysicalSize as WinitPhysicalSize,
};
use winit::event::WindowEvent as WinitEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow as WinitControlFlow, EventLoop};
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::{
    CursorGrabMode, CursorIcon as WinitCursorIcon, Fullscreen as WinitFullscreen,
    Window as WinitWindow, WindowAttributes, WindowId as WinitWindowId,
};

use super::app::{App, AppContext, Request};
use super::backend::Backend;
use super::cursor::{CursorGrab, CursorIcon};
use super::desc::{Fullscreen, WindowDesc};
use super::error::WindowError;
use super::event::{ControlFlow, WindowEvent, WindowId};
use super::geometry::{PhysicalPosition, PhysicalSize};
use super::handle::NativeWindowHandle;
use super::monitor::MonitorInfo;

/// The desktop backend: real windows through `winit`.
///
/// ```no_run
/// use platform::window::{App, AppContext, Backend, WindowDesc, WindowEvent, WindowId, WinitBackend};
///
/// struct Game;
/// impl App for Game {
///     fn resumed(&mut self, ctx: &mut AppContext) {
///         ctx.create_window(WindowDesc::new("Game")).unwrap();
///     }
///     fn window_event(&mut self, ctx: &mut AppContext, _: WindowId, event: WindowEvent) {
///         if event == WindowEvent::CloseRequested {
///             ctx.exit();
///         }
///     }
/// }
///
/// let exit_code = WinitBackend::new().run(Game).unwrap();
/// std::process::exit(exit_code);
/// ```
#[derive(Debug, Clone, Default)]
pub struct WinitBackend {
    any_thread: bool,
}

impl WinitBackend {
    /// A backend with the default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allows the event loop to run on a thread other than the main one. Only
    /// Windows and Linux (X11, Wayland) support that; elsewhere the setting is
    /// ignored and the loop must run on the main thread. Meant for tests; an
    /// application runs its loop on the main thread.
    #[must_use]
    pub fn with_any_thread(mut self, any_thread: bool) -> Self {
        self.any_thread = any_thread;
        self
    }
}

impl Backend for WinitBackend {
    /// Opens the event loop and runs `app` in it until it exits.
    ///
    /// # Errors
    /// [`WindowError::Backend`] when the event loop cannot start (for example
    /// no display is available), when a window cannot be created, or when the
    /// loop fails.
    fn run<A: App>(self, app: A) -> Result<i32, WindowError> {
        let mut builder = EventLoop::builder();
        configure_thread(&mut builder, self.any_thread);
        let event_loop = builder
            .build()
            .map_err(|e| WindowError::Backend(format!("cannot start the event loop: {e}")))?;
        event_loop.set_control_flow(WinitControlFlow::Wait);

        let mut driver = Driver::new(app);
        let result = event_loop.run_app(&mut driver);
        if let Some(fatal) = driver.fatal.take() {
            return Err(fatal);
        }
        result.map_err(|e| WindowError::Backend(format!("the event loop failed: {e}")))?;
        Ok(driver.ctx.exit_code.unwrap_or(0))
    }
}

#[cfg(windows)]
fn configure_thread(builder: &mut winit::event_loop::EventLoopBuilder<()>, any_thread: bool) {
    use winit::platform::windows::EventLoopBuilderExtWindows;
    builder.with_any_thread(any_thread);
}

#[cfg(all(
    unix,
    not(any(target_vendor = "apple", target_os = "android", target_os = "redox"))
))]
fn configure_thread(builder: &mut winit::event_loop::EventLoopBuilder<()>, any_thread: bool) {
    use winit::platform::wayland::EventLoopBuilderExtWayland;
    use winit::platform::x11::EventLoopBuilderExtX11;
    // Both traits have the method; the backend picked at run time reads its own.
    EventLoopBuilderExtX11::with_any_thread(builder, any_thread);
    EventLoopBuilderExtWayland::with_any_thread(builder, any_thread);
}

#[cfg(not(any(
    windows,
    all(
        unix,
        not(any(target_vendor = "apple", target_os = "android", target_os = "redox"))
    )
)))]
fn configure_thread(_builder: &mut winit::event_loop::EventLoopBuilder<()>, _any_thread: bool) {}

/// Drives the app from winit's callbacks.
struct Driver<A: App> {
    app: A,
    ctx: AppContext,
    /// The real windows, by our id.
    windows: BTreeMap<WindowId, WinitWindow>,
    /// Our id for each real window.
    ids: HashMap<WinitWindowId, WindowId>,
    suspended: bool,
    fatal: Option<WindowError>,
}

impl<A: App> Driver<A> {
    fn new(app: A) -> Self {
        Self {
            app,
            ctx: AppContext::new(1.0),
            windows: BTreeMap::new(),
            ids: HashMap::new(),
            suspended: false,
            fatal: None,
        }
    }

    /// `true` once the app asked to exit or something failed: events are
    /// ignored until the loop has wound down.
    fn stopped(&self) -> bool {
        self.ctx.is_exiting() || self.fatal.is_some()
    }

    fn fail(&mut self, error: WindowError) {
        self.fatal.get_or_insert(error);
        self.ctx.exit_with_code(1);
    }

    fn refresh_monitors(&mut self, event_loop: &ActiveEventLoop) {
        let primary = event_loop.primary_monitor();
        let mut monitors: Vec<MonitorInfo> = event_loop
            .available_monitors()
            .map(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                MonitorInfo::new(
                    PhysicalPosition::new(position.x, position.y),
                    PhysicalSize::new(size.width, size.height),
                    monitor.scale_factor(),
                )
                    .with_name(monitor.name())
                    .with_refresh_rate(monitor.refresh_rate_millihertz())
                    .with_primary(primary.as_ref() == Some(&monitor))
            })
            .collect();
        // Wayland has no notion of a primary monitor; report the first one so
        // that `primary_monitor()` is never empty on a system with a display.
        if !monitors.iter().any(|m| m.is_primary)
            && let Some(first) = monitors.first_mut()
        {
            first.is_primary = true;
        }
        self.ctx.set_monitors(monitors);
    }

    /// Carries out the requests the app made, then hands what is left to
    /// winit: redraws, the control flow and a requested exit.
    fn end_callback(&mut self, event_loop: &ActiveEventLoop) {
        self.settle(event_loop);
        for id in std::mem::take(&mut self.ctx.redraws) {
            if let Some(window) = self.windows.get(&id) {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(match self.ctx.control_flow {
            ControlFlow::Poll => WinitControlFlow::Poll,
            ControlFlow::Wait => WinitControlFlow::Wait,
            ControlFlow::WaitUntil(instant) => WinitControlFlow::WaitUntil(instant),
        });
        if self.stopped() {
            event_loop.exit();
        }
    }

    fn deliver_raw(&mut self, id: WindowId, event: WindowEvent) {
        self.app.window_event(&mut self.ctx, id, event);
    }

    fn settle(&mut self, event_loop: &ActiveEventLoop) {
        while let Some(request) = self.ctx.requests.pop_front() {
            if self.stopped() {
                self.ctx.requests.clear();
                break;
            }
            self.apply(event_loop, request);
        }
    }

    fn apply(&mut self, event_loop: &ActiveEventLoop, request: Request) {
        match request {
            Request::Create(id, desc) => self.create(event_loop, id, &desc),
            Request::SetTitle(id) => {
                if let (Some(state), Some(window)) =
                    (self.ctx.windows.get(&id), self.windows.get(&id))
                {
                    window.set_title(state.title());
                }
            }
            Request::SetVisible(id) => {
                if let (Some(state), Some(window)) =
                    (self.ctx.windows.get(&id), self.windows.get(&id))
                {
                    window.set_visible(state.is_visible());
                }
            }
            Request::SetInnerSize(id, size) => {
                let physical = self
                    .ctx
                    .window(id)
                    .map(|w| size.to_physical(w.scale_factor()));
                self.set_inner_size(id, physical);
            }
            Request::SetPosition(id, position) => {
                if let Some(window) = self.windows.get(&id) {
                    window.set_outer_position(WinitLogicalPosition::new(position.x, position.y));
                }
            }
            Request::SetFullscreen(id, fullscreen) => {
                if let Some(window) = self.windows.get(&id) {
                    window.set_fullscreen(match fullscreen {
                        Fullscreen::None => None,
                        _ => Some(WinitFullscreen::Borderless(None)),
                    });
                    if let Some(state) = self.ctx.windows.get_mut(&id) {
                        state.report_fullscreen(fullscreen);
                    }
                }
            }
            Request::SetCursorIcon(id) => {
                if let (Some(state), Some(window)) =
                    (self.ctx.windows.get(&id), self.windows.get(&id))
                {
                    window.set_cursor(cursor_icon(state.cursor_icon()));
                }
            }
            Request::SetCursorVisible(id) => {
                if let (Some(state), Some(window)) =
                    (self.ctx.windows.get(&id), self.windows.get(&id))
                {
                    window.set_cursor_visible(state.is_cursor_visible());
                }
            }
            Request::SetCursorGrab(id, grab) => {
                if let Some(window) = self.windows.get(&id) {
                    let mode = match grab {
                        CursorGrab::Confined => CursorGrabMode::Confined,
                        CursorGrab::Locked => CursorGrabMode::Locked,
                        _ => CursorGrabMode::None,
                    };
                    // A refusal leaves the previous mode in place; the window
                    // reports what is in effect.
                    if window.set_cursor_grab(mode).is_ok()
                        && let Some(state) = self.ctx.windows.get_mut(&id)
                    {
                        state.report_cursor_grab(grab);
                    }
                }
            }
            Request::Close(id) => self.close(id),
        }
    }

    fn create(&mut self, event_loop: &ActiveEventLoop, id: WindowId, desc: &WindowDesc) {
        let mut attributes = WindowAttributes::default()
            .with_title(desc.title.as_str())
            .with_inner_size(WinitLogicalSize::new(
                desc.inner_size.width,
                desc.inner_size.height,
            ))
            .with_resizable(desc.resizable)
            .with_visible(desc.visible)
            .with_decorations(desc.decorations);
        if let Some(position) = desc.position {
            attributes =
                attributes.with_position(WinitLogicalPosition::new(position.x, position.y));
        }
        if let Some(min) = desc.min_inner_size {
            attributes =
                attributes.with_min_inner_size(WinitLogicalSize::new(min.width, min.height));
        }
        if let Some(max) = desc.max_inner_size {
            attributes =
                attributes.with_max_inner_size(WinitLogicalSize::new(max.width, max.height));
        }

        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(error) => {
                self.ctx.windows.remove(&id);
                self.fail(WindowError::Backend(format!(
                    "cannot create window {id}: {error}"
                )));
                return;
            }
        };

        // What the system really gave us replaces what the context assumed.
        let inner = window.inner_size();
        let position = window.outer_position().ok();
        if let Some(state) = self.ctx.windows.get_mut(&id) {
            state.report_scale_factor(window.scale_factor());
            state.report_inner_size(PhysicalSize::new(inner.width, inner.height));
            if let Some(position) = position {
                state.report_position(PhysicalPosition::new(position.x, position.y));
            }
            state.report_focused(window.has_focus());
        }
        let handle = native_handle(&window);
        self.ids.insert(window.id(), id);
        self.windows.insert(id, window);
        self.refresh_monitors(event_loop);

        match handle {
            Some(handle) => self.attach_surface(id, handle),
            // Desktop windows always have a handle; mobile ones get theirs
            // when the system provides the surface.
            None if cfg!(not(any(target_os = "android", target_os = "ios"))) => {
                self.fail(WindowError::Backend(format!(
                    "window {id} has no native handle this crate understands"
                )));
            }
            None => {}
        }
    }

    fn attach_surface(&mut self, id: WindowId, handle: NativeWindowHandle) {
        if let Some(state) = self.ctx.windows.get_mut(&id) {
            state.report_surface(Some(handle));
            self.deliver_raw(id, WindowEvent::SurfaceCreated(handle));
        }
    }

    /// Gives a surface to every window that lost its own while suspended
    /// (Android, iOS); a no-op on the desktop, where nothing is lost.
    fn attach_missing_surfaces(&mut self) {
        let missing: Vec<WindowId> = self
            .ctx
            .windows
            .values()
            .filter(|w| w.native_handle().is_none())
            .map(super::state::Window::id)
            .collect();
        for id in missing {
            let handle = self.windows.get(&id).and_then(native_handle);
            if let Some(handle) = handle {
                self.attach_surface(id, handle);
            }
        }
    }

    fn lose_all_surfaces(&mut self) {
        let with_surface: Vec<WindowId> = self
            .ctx
            .windows
            .values()
            .filter(|w| w.native_handle().is_some())
            .map(super::state::Window::id)
            .collect();
        for id in with_surface {
            if let Some(state) = self.ctx.windows.get_mut(&id) {
                state.report_surface(None);
                self.deliver_raw(id, WindowEvent::SurfaceLost);
            }
        }
    }

    fn close(&mut self, id: WindowId) {
        let Some(state) = self.ctx.windows.get_mut(&id) else {
            return;
        };
        if state.native_handle().is_some() {
            state.report_surface(None);
            self.deliver_raw(id, WindowEvent::SurfaceLost);
        }
        self.deliver_raw(id, WindowEvent::Destroyed);
        self.ctx.windows.remove(&id);
        self.ctx.redraws.remove(&id);
        if let Some(window) = self.windows.remove(&id) {
            self.ids.remove(&window.id());
        }
    }

    /// Asks for `size` (limited to the window's minimum and maximum). A system
    /// that applies it at once is reported like one that answers later.
    fn set_inner_size(&mut self, id: WindowId, size: Option<PhysicalSize>) {
        let (Some(size), Some(state), Some(window)) =
            (size, self.ctx.windows.get(&id), self.windows.get(&id))
        else {
            return;
        };
        if state.fullscreen() != Fullscreen::None {
            return;
        }
        let target = state.clamp_inner_size(size);
        if let Some(applied) =
            window.request_inner_size(WinitPhysicalSize::new(target.width, target.height))
        {
            self.report_resize(id, PhysicalSize::new(applied.width, applied.height));
        }
    }

    /// Records a new client-area size and delivers `Resized`, unless the
    /// window already has that size.
    fn report_resize(&mut self, id: WindowId, size: PhysicalSize) {
        let fullscreen = self.windows.get(&id).map(|w| w.fullscreen().is_some());
        let Some(state) = self.ctx.windows.get_mut(&id) else {
            return;
        };
        // The system is the authority on fullscreen: a refused or undone
        // request shows up here.
        if let Some(fullscreen) = fullscreen {
            state.report_fullscreen(if fullscreen {
                Fullscreen::Borderless
            } else {
                Fullscreen::None
            });
        }
        if state.inner_size() == size {
            return;
        }
        state.report_inner_size(size);
        self.deliver_raw(id, WindowEvent::Resized(size));
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WinitEvent,
    ) {
        match event {
            WinitEvent::CloseRequested => self.deliver_raw(id, WindowEvent::CloseRequested),
            WinitEvent::Resized(size) => {
                self.report_resize(id, PhysicalSize::new(size.width, size.height))
            }
            WinitEvent::Moved(position) => {
                let position = PhysicalPosition::new(position.x, position.y);
                if let Some(state) = self.ctx.windows.get_mut(&id)
                    && state.position() != Some(position)
                {
                    state.report_position(position);
                    self.deliver_raw(id, WindowEvent::Moved(position));
                }
            }
            WinitEvent::ScaleFactorChanged {
                scale_factor,
                mut inner_size_writer,
            } => {
                let Some(state) = self.ctx.windows.get_mut(&id) else {
                    return;
                };
                // Keep the logical size: the physical one follows the scale.
                let inner_size = state.logical_inner_size().to_physical(scale_factor);
                state.report_scale_factor(scale_factor);
                state.report_inner_size(inner_size);
                let _ = inner_size_writer.request_inner_size(WinitPhysicalSize::new(
                    inner_size.width,
                    inner_size.height,
                ));
                self.refresh_monitors(event_loop);
                self.deliver_raw(
                    id,
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        inner_size,
                    },
                );
            }
            WinitEvent::Focused(focused) => {
                if let Some(state) = self.ctx.windows.get_mut(&id) {
                    state.report_focused(focused);
                    self.deliver_raw(id, WindowEvent::Focused(focused));
                }
            }
            WinitEvent::Occluded(occluded) => {
                if let Some(state) = self.ctx.windows.get_mut(&id) {
                    state.report_occluded(occluded);
                    self.deliver_raw(id, WindowEvent::Occluded(occluded));
                }
            }
            // Not while suspended: there is no surface to draw into.
            WinitEvent::RedrawRequested if !self.suspended => {
                self.deliver_raw(id, WindowEvent::RedrawRequested);
            }
            // Input arrives with its own phase; everything else is not part of
            // the window contract.
            _ => {}
        }
    }

    /// Ends the app: every remaining window loses its surface and is
    /// destroyed, in id order.
    fn shut_down(&mut self) {
        let ids: Vec<WindowId> = self.ctx.windows.keys().copied().collect();
        for id in ids {
            if let Some(state) = self.ctx.windows.get_mut(&id)
                && state.native_handle().is_some()
            {
                state.report_surface(None);
                self.deliver_raw(id, WindowEvent::SurfaceLost);
            }
            self.deliver_raw(id, WindowEvent::Destroyed);
        }
        self.ctx.windows.clear();
        self.ctx.redraws.clear();
        self.ctx.requests.clear();
        self.ids.clear();
        self.windows.clear();
    }
}

impl<A: App> ApplicationHandler for Driver<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.stopped() {
            return;
        }
        self.suspended = false;
        self.refresh_monitors(event_loop);
        self.app.resumed(&mut self.ctx);
        self.end_callback(event_loop);
        self.attach_missing_surfaces();
        self.end_callback(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if self.stopped() {
            return;
        }
        self.lose_all_surfaces();
        self.suspended = true;
        self.app.suspended(&mut self.ctx);
        self.end_callback(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WinitWindowId,
        event: WinitEvent,
    ) {
        if self.stopped() {
            return;
        }
        let Some(&id) = self.ids.get(&window_id) else {
            return;
        };
        self.handle_window_event(event_loop, id, event);
        self.end_callback(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.stopped() {
            return;
        }
        self.app.about_to_wait(&mut self.ctx);
        self.end_callback(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shut_down();
    }
}

fn cursor_icon(icon: CursorIcon) -> WinitCursorIcon {
    match icon {
        CursorIcon::Pointer => WinitCursorIcon::Pointer,
        CursorIcon::Text => WinitCursorIcon::Text,
        CursorIcon::Crosshair => WinitCursorIcon::Crosshair,
        CursorIcon::Wait => WinitCursorIcon::Wait,
        CursorIcon::Progress => WinitCursorIcon::Progress,
        CursorIcon::Help => WinitCursorIcon::Help,
        CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
        CursorIcon::Grab => WinitCursorIcon::Grab,
        CursorIcon::Grabbing => WinitCursorIcon::Grabbing,
        CursorIcon::Move => WinitCursorIcon::Move,
        CursorIcon::ResizeHorizontal => WinitCursorIcon::EwResize,
        CursorIcon::ResizeVertical => WinitCursorIcon::NsResize,
        // `Default` and any shape added later without a mapping yet.
        _ => WinitCursorIcon::Default,
    }
}

/// An Xlib `Window` as the 64-bit value our handle carries.
fn xlib_window(window: std::ffi::c_ulong) -> Option<NonZeroU64> {
    #[allow(
        clippy::useless_conversion,
        reason = "`c_ulong` is 64 bits on 64-bit Unix but 32 bits elsewhere"
    )]
    NonZeroU64::new(u64::from(window))
}

/// The presentation-surface handle of `window`, in our own type; `None` when
/// the window has no surface (yet) or the window system is not one we know.
fn native_handle(window: &WinitWindow) -> Option<NativeWindowHandle> {
    let window_handle = window.window_handle().ok()?;
    let display_handle = window.display_handle().ok()?;
    match (window_handle.as_raw(), display_handle.as_raw()) {
        (RawWindowHandle::Win32(handle), _) => Some(NativeWindowHandle::Win32 {
            hwnd: NonZeroUsize::new(handle.hwnd.get().cast_unsigned())?,
            hinstance: handle
                .hinstance
                .and_then(|instance| NonZeroUsize::new(instance.get().cast_unsigned())),
        }),
        (RawWindowHandle::Xlib(handle), RawDisplayHandle::Xlib(display)) => {
            Some(NativeWindowHandle::X11 {
                display: display.display.map(std::ptr::NonNull::addr),
                window: xlib_window(handle.window)?,
            })
        }
        (RawWindowHandle::Wayland(handle), RawDisplayHandle::Wayland(display)) => {
            Some(NativeWindowHandle::Wayland {
                display: display.display.addr(),
                surface: handle.surface.addr(),
            })
        }
        (RawWindowHandle::AndroidNdk(handle), _) => Some(NativeWindowHandle::Android {
            native_window: handle.a_native_window.addr(),
        }),
        #[cfg(target_os = "macos")]
        (RawWindowHandle::AppKit(_), _) => crate::sys::metal_layer(&window_handle)
            .map(|metal_layer| NativeWindowHandle::Cocoa { metal_layer }),
        #[cfg(target_os = "ios")]
        (RawWindowHandle::UiKit(_), _) => crate::sys::metal_layer(&window_handle)
            .map(|metal_layer| NativeWindowHandle::UiKit { metal_layer }),
        _ => None,
    }
}
