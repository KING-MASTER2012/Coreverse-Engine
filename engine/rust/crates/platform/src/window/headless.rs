//! A backend that needs no display, no window system and no GPU.
//!
//! It simulates what an operating system does to windows: the *test* plays the
//! role of the OS by queueing [`HostEvent`]s (a resize, a close click, the app
//! going to the background, a new Android surface), and the backend turns
//! them into the same callbacks, in the same order, as a real backend. Requests
//! made by the app (a new size, fullscreen) are carried out the way a window
//! manager would: they come back as events.
//!
//! This is what the window API's tests run on in CI, on every platform; it is
//! also useful for headless tools and for testing application logic.
//!
//! Two ways to use it:
//!
//! * [`HeadlessBackend`] implements [`Backend`]: queue events, call
//!   [`run`](Backend::run), get the exit code.
//! * [`HeadlessLoop`] is the same loop, driven step by step, which hands the
//!   app back so a test can inspect it. Prefer it for tests.
//!
//! The headless loop has no clock: [`ControlFlow::WaitUntil`] behaves like
//! [`ControlFlow::Wait`], and a loop that waits with nothing queued is
//! [idle](RunOutcome::Idle).

use std::collections::{BTreeMap, VecDeque};

use super::app::{App, AppContext, Request};
use super::backend::Backend;
use super::desc::Fullscreen;
use super::error::WindowError;
use super::event::{ControlFlow, WindowEvent, WindowId};
use super::geometry::{PhysicalPosition, PhysicalSize};
use super::handle::NativeWindowHandle;

/// The most events one [`HeadlessLoop::step`] delivers before it gives up.
pub const MAX_EVENTS_PER_STEP: usize = 10_000;

/// The step limit [`HeadlessBackend::run`] uses.
pub const DEFAULT_STEP_LIMIT: usize = 1_000_000;

/// The simulated machine: what the "monitor" looks like.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HeadlessConfig {
    /// The scale factor new windows start with. Default: `1.0`.
    pub scale_factor: f64,
    /// The size of the simulated monitor, which a fullscreen window fills.
    /// Default: 1920 x 1080.
    pub monitor_size: PhysicalSize,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            monitor_size: PhysicalSize::new(1920, 1080),
        }
    }
}

impl HeadlessConfig {
    /// Sets the scale factor new windows start with.
    #[must_use]
    pub fn with_scale_factor(mut self, scale_factor: f64) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    /// Sets the size of the simulated monitor.
    #[must_use]
    pub fn with_monitor_size(mut self, monitor_size: PhysicalSize) -> Self {
        self.monitor_size = monitor_size;
        self
    }

    /// Checks that the scale factor is finite and positive and the monitor has
    /// a size.
    ///
    /// # Errors
    /// [`WindowError::InvalidArgument`].
    pub fn validate(&self) -> Result<(), WindowError> {
        validate_scale_factor(self.scale_factor)?;
        if self.monitor_size.width == 0 || self.monitor_size.height == 0 {
            return Err(WindowError::InvalidArgument(
                "monitor_size must not be zero".to_owned(),
            ));
        }
        Ok(())
    }
}

fn validate_scale_factor(scale_factor: f64) -> Result<(), WindowError> {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        Ok(())
    } else {
        Err(WindowError::InvalidArgument(format!(
            "scale factor must be finite and greater than zero, got {scale_factor}"
        )))
    }
}

/// Something the simulated operating system does. Events for a window that
/// does not exist (any more) are ignored, as a real system sends them for
/// windows that were just closed.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum HostEvent {
    /// The app returns to the foreground. Ignored unless it is suspended: the
    /// loop starts resumed. Surfaces are not restored; queue
    /// [`SurfaceCreated`](Self::SurfaceCreated) as the system would.
    Resume,
    /// The app goes to the background: every surface is lost, then
    /// `suspended` is called. Ignored unless the app is resumed.
    Suspend,
    /// The user clicked the close button.
    CloseRequested {
        /// The window.
        window: WindowId,
    },
    /// The client area got this size (the user dragged an edge, or the app's
    /// request was carried out).
    Resized {
        /// The window.
        window: WindowId,
        /// The new size in physical pixels.
        size: PhysicalSize,
    },
    /// The window moved.
    Moved {
        /// The window.
        window: WindowId,
        /// The new position in physical pixels.
        position: PhysicalPosition,
    },
    /// The window moved to a monitor with another scale factor. The window
    /// keeps its logical size. A scale factor that is not finite and positive
    /// makes the step fail with [`WindowError::InvalidArgument`].
    ScaleFactorChanged {
        /// The window.
        window: WindowId,
        /// The new scale factor.
        scale_factor: f64,
    },
    /// The window gained or lost keyboard focus.
    Focused {
        /// The window.
        window: WindowId,
        /// `true` when focus was gained.
        focused: bool,
    },
    /// The window became fully hidden or visible again.
    Occluded {
        /// The window.
        window: WindowId,
        /// `true` when it became hidden.
        occluded: bool,
    },
    /// The system gave the window a surface. If it already had one, that one
    /// is lost first, as when Android replaces its `ANativeWindow`.
    SurfaceCreated {
        /// The window.
        window: WindowId,
        /// The handle to hand to the app. The headless backend never looks
        /// inside it, so a test can use any non-zero value.
        handle: NativeWindowHandle,
    },
    /// The system took the window's surface away. Ignored when it has none.
    SurfaceLost {
        /// The window.
        window: WindowId,
    },
}

/// What one [`HeadlessLoop::step`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    /// The loop ran one iteration and can continue.
    Running,
    /// The app exited with this code; the loop is finished.
    Exited(i32),
}

/// How [`HeadlessLoop::run_until_idle`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// The app exited with this code.
    Exited(i32),
    /// Nothing is left to do: no queued event, no pending redraw, and a
    /// control flow that waits. Queue more events and run again.
    Idle,
    /// The step limit was reached first (typically [`ControlFlow::Poll`]).
    StepLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    NotStarted,
    Resumed,
    Suspended,
    Exited,
}

/// The headless event loop, driven one [`step`](Self::step) at a time.
///
/// One step is one loop iteration:
///
/// 1. The first step starts the app: `resumed` is called.
/// 2. Every queued [`HostEvent`] is delivered (events produced meanwhile, for
///    example the `Resized` answering a resize request, join the queue and are
///    delivered in the same step).
/// 3. `about_to_wait` is called.
/// 4. `RedrawRequested` is delivered to every window that asked for one, unless
///    the app is suspended.
///
/// After every callback the requests the app made are carried out. As soon as
/// the app calls [`AppContext::exit`], the step ends: the remaining windows
/// get `SurfaceLost` (if they have a surface) and `Destroyed`, and the loop is
/// finished.
#[derive(Debug)]
pub struct HeadlessLoop<A: App> {
    app: A,
    ctx: AppContext,
    config: HeadlessConfig,
    queue: VecDeque<HostEvent>,
    phase: Phase,
    exit_code: Option<i32>,
    /// The size a window had before it went fullscreen, to restore it.
    windowed_sizes: BTreeMap<WindowId, PhysicalSize>,
}

impl<A: App> HeadlessLoop<A> {
    /// A loop for `app` on the simulated machine `config`. Nothing runs until
    /// the first [`step`](Self::step).
    ///
    /// # Errors
    /// [`WindowError::InvalidArgument`] when [`HeadlessConfig::validate`]
    /// fails.
    pub fn new(app: A, config: HeadlessConfig) -> Result<Self, WindowError> {
        config.validate()?;
        Ok(Self {
            app,
            ctx: AppContext::new(config.scale_factor),
            config,
            queue: VecDeque::new(),
            phase: Phase::NotStarted,
            exit_code: None,
            windowed_sizes: BTreeMap::new(),
        })
    }

    /// Queues an event for the next step.
    pub fn push(&mut self, event: HostEvent) {
        self.queue.push_back(event);
    }

    /// The app.
    #[must_use]
    pub fn app(&self) -> &A {
        &self.app
    }

    /// The app, mutably.
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// The context the app sees: its windows and settings.
    #[must_use]
    pub fn context(&self) -> &AppContext {
        &self.ctx
    }

    /// The context, mutably. Lets a test do what the app could do in a
    /// callback; requests made this way are carried out in the next step.
    pub fn context_mut(&mut self) -> &mut AppContext {
        &mut self.ctx
    }

    /// The exit code, once the app has exited.
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// `true` while the app is in the background.
    #[must_use]
    pub fn is_suspended(&self) -> bool {
        self.phase == Phase::Suspended
    }

    /// Ends the loop and returns the app.
    #[must_use]
    pub fn into_app(self) -> A {
        self.app
    }

    /// `true` when another [`step`](Self::step) would have nothing to do:
    /// the app has started, no event or request is queued, no redraw is due (a redraw
    /// requested while suspended is not due) and the control flow is not
    /// [`ControlFlow::Poll`]. A finished loop is idle too.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        match self.phase {
            Phase::NotStarted => false,
            Phase::Exited => true,
            Phase::Resumed | Phase::Suspended => {
                let redraw_due = self.phase == Phase::Resumed && !self.ctx.redraws.is_empty();
                self.queue.is_empty()
                    && self.ctx.requests.is_empty()
                    && !redraw_due
                    && self.ctx.control_flow != ControlFlow::Poll
            }
        }
    }

    /// Runs one loop iteration (see the type documentation).
    ///
    /// # Errors
    /// [`WindowError::EventStorm`] when more than [`MAX_EVENTS_PER_STEP`]
    /// events were delivered, and [`WindowError::InvalidArgument`] for a
    /// [`HostEvent::ScaleFactorChanged`] with an unusable scale factor.
    pub fn step(&mut self) -> Result<StepOutcome, WindowError> {
        if self.phase == Phase::Exited {
            return Ok(StepOutcome::Exited(self.exit_code.unwrap_or(0)));
        }

        if self.phase == Phase::NotStarted {
            self.phase = Phase::Resumed;
            self.app.resumed(&mut self.ctx);
            self.settle();
            if self.ctx.is_exiting() {
                return Ok(self.finish());
            }
        }

        // Requests made outside a callback (through `context_mut`) are carried
        // out before the events are delivered.
        self.settle();
        if self.ctx.is_exiting() {
            return Ok(self.finish());
        }

        let mut delivered = 0usize;
        while let Some(event) = self.queue.pop_front() {
            delivered += 1;
            if delivered > MAX_EVENTS_PER_STEP {
                return Err(WindowError::EventStorm(MAX_EVENTS_PER_STEP));
            }
            self.handle_host(event)?;
            if self.ctx.is_exiting() {
                return Ok(self.finish());
            }
        }

        self.app.about_to_wait(&mut self.ctx);
        self.settle();
        if self.ctx.is_exiting() {
            return Ok(self.finish());
        }

        if self.phase == Phase::Resumed {
            // Taken first, so a redraw requested inside a redraw handler is
            // delivered in the next step.
            let due = std::mem::take(&mut self.ctx.redraws);
            for id in due {
                if self.ctx.windows.contains_key(&id) {
                    self.deliver(id, WindowEvent::RedrawRequested);
                    if self.ctx.is_exiting() {
                        return Ok(self.finish());
                    }
                }
            }
        }

        Ok(StepOutcome::Running)
    }

    /// Steps until the app exits, the loop is [idle](Self::is_idle), or
    /// `max_steps` steps have run.
    ///
    /// # Errors
    /// The errors of [`step`](Self::step).
    pub fn run_until_idle(&mut self, max_steps: usize) -> Result<RunOutcome, WindowError> {
        for _ in 0..max_steps {
            if let StepOutcome::Exited(code) = self.step()? {
                return Ok(RunOutcome::Exited(code));
            }
            if self.is_idle() {
                return Ok(RunOutcome::Idle);
            }
        }
        Ok(RunOutcome::StepLimit)
    }

    /// Calls the app's `window_event` and carries out the requests it made.
    fn deliver(&mut self, id: WindowId, event: WindowEvent) {
        self.deliver_raw(id, event);
        self.settle();
    }

    fn deliver_raw(&mut self, id: WindowId, event: WindowEvent) {
        self.app.window_event(&mut self.ctx, id, event);
    }

    /// Carries out the pending requests, which may deliver events (closing a
    /// window does) that make more requests; stops when none are left or the
    /// app has exited (`finish` then closes everything).
    fn settle(&mut self) {
        while let Some(request) = self.ctx.requests.pop_front() {
            if self.ctx.is_exiting() {
                self.ctx.requests.clear();
                break;
            }
            self.apply(request);
        }
    }

    fn apply(&mut self, request: Request) {
        match request {
            // Nothing to create, show or retitle without a window system.
            Request::Create(_) | Request::SetTitle(_) | Request::SetVisible(_) => {}
            Request::SetInnerSize(id, size) => {
                let Some(window) = self.ctx.windows.get(&id) else {
                    return;
                };
                // Like most window managers, ignore it while fullscreen.
                if window.fullscreen() != Fullscreen::None {
                    return;
                }
                let target = window.clamp_inner_size(size.to_physical(window.scale_factor()));
                if target != window.inner_size() {
                    self.queue.push_back(HostEvent::Resized {
                        window: id,
                        size: target,
                    });
                }
            }
            Request::SetPosition(id, position) => {
                let Some(window) = self.ctx.windows.get(&id) else {
                    return;
                };
                let target = position.to_physical(window.scale_factor());
                if window.position() != Some(target) {
                    self.queue.push_back(HostEvent::Moved {
                        window: id,
                        position: target,
                    });
                }
            }
            Request::SetFullscreen(id, fullscreen) => self.apply_fullscreen(id, fullscreen),
            Request::Close(id) => {
                let Some(window) = self.ctx.windows.get_mut(&id) else {
                    return;
                };
                let had_surface = window.native_handle().is_some();
                if had_surface {
                    window.report_surface(None);
                    self.deliver_raw(id, WindowEvent::SurfaceLost);
                }
                self.deliver_raw(id, WindowEvent::Destroyed);
                self.ctx.windows.remove(&id);
                self.ctx.redraws.remove(&id);
                self.windowed_sizes.remove(&id);
            }
        }
    }

    fn apply_fullscreen(&mut self, id: WindowId, fullscreen: Fullscreen) {
        let Some(window) = self.ctx.windows.get_mut(&id) else {
            return;
        };
        if window.fullscreen() == fullscreen {
            return;
        }
        let current = window.inner_size();
        window.report_fullscreen(fullscreen);
        let target = if fullscreen == Fullscreen::None {
            self.windowed_sizes.remove(&id)
        } else {
            self.windowed_sizes.entry(id).or_insert(current);
            Some(self.config.monitor_size)
        };
        if let Some(size) = target
            && size != current
        {
            self.queue
                .push_back(HostEvent::Resized { window: id, size });
        }
    }

    fn handle_host(&mut self, event: HostEvent) -> Result<(), WindowError> {
        match event {
            HostEvent::Resume => {
                if self.phase == Phase::Suspended {
                    self.phase = Phase::Resumed;
                    self.app.resumed(&mut self.ctx);
                    self.settle();
                }
            }
            HostEvent::Suspend => {
                if self.phase == Phase::Resumed {
                    self.lose_all_surfaces();
                    self.phase = Phase::Suspended;
                    self.app.suspended(&mut self.ctx);
                    self.settle();
                }
            }
            HostEvent::CloseRequested { window } => {
                if self.ctx.windows.contains_key(&window) {
                    self.deliver(window, WindowEvent::CloseRequested);
                }
            }
            HostEvent::Resized { window, size } => {
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    w.report_inner_size(size);
                    self.deliver(window, WindowEvent::Resized(size));
                }
            }
            HostEvent::Moved { window, position } => {
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    w.report_position(position);
                    self.deliver(window, WindowEvent::Moved(position));
                }
            }
            HostEvent::ScaleFactorChanged {
                window,
                scale_factor,
            } => {
                validate_scale_factor(scale_factor)?;
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    // The logical size is kept, so the physical size follows.
                    let logical = w.logical_inner_size();
                    let inner_size = logical.to_physical(scale_factor);
                    w.report_scale_factor(scale_factor);
                    w.report_inner_size(inner_size);
                    self.deliver(
                        window,
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            inner_size,
                        },
                    );
                }
            }
            HostEvent::Focused { window, focused } => {
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    w.report_focused(focused);
                    self.deliver(window, WindowEvent::Focused(focused));
                }
            }
            HostEvent::Occluded { window, occluded } => {
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    w.report_occluded(occluded);
                    self.deliver(window, WindowEvent::Occluded(occluded));
                }
            }
            HostEvent::SurfaceCreated { window, handle } => {
                if let Some(w) = self.ctx.windows.get_mut(&window) {
                    let replaced = w.native_handle().is_some();
                    w.report_surface(None);
                    if replaced {
                        self.deliver(window, WindowEvent::SurfaceLost);
                    }
                    if self.ctx.is_exiting() {
                        return Ok(());
                    }
                    // The app may have closed the window while handling the loss.
                    if let Some(w) = self.ctx.windows.get_mut(&window) {
                        w.report_surface(Some(handle));
                        self.deliver(window, WindowEvent::SurfaceCreated(handle));
                    }
                }
            }
            HostEvent::SurfaceLost { window } => {
                if let Some(w) = self.ctx.windows.get_mut(&window)
                    && w.native_handle().is_some()
                {
                    w.report_surface(None);
                    self.deliver(window, WindowEvent::SurfaceLost);
                }
            }
        }
        Ok(())
    }

    /// `SurfaceLost` for every window that has a surface, in id order.
    fn lose_all_surfaces(&mut self) {
        let with_surface: Vec<WindowId> = self
            .ctx
            .windows
            .values()
            .filter(|w| w.native_handle().is_some())
            .map(super::state::Window::id)
            .collect();
        for id in with_surface {
            if self.ctx.is_exiting() {
                break;
            }
            if let Some(w) = self.ctx.windows.get_mut(&id) {
                w.report_surface(None);
                self.deliver(id, WindowEvent::SurfaceLost);
            }
        }
    }

    /// Ends the loop after `exit`: closes every remaining window (their
    /// handlers' requests are ignored) and records the exit code.
    fn finish(&mut self) -> StepOutcome {
        let code = self.ctx.exit_code.unwrap_or(0);
        self.queue.clear();
        self.ctx.requests.clear();
        let ids: Vec<WindowId> = self.ctx.windows.keys().copied().collect();
        for id in ids {
            if let Some(w) = self.ctx.windows.get_mut(&id)
                && w.native_handle().is_some()
            {
                w.report_surface(None);
                self.deliver_raw(id, WindowEvent::SurfaceLost);
            }
            self.deliver_raw(id, WindowEvent::Destroyed);
        }
        self.ctx.windows.clear();
        self.ctx.redraws.clear();
        self.ctx.requests.clear();
        self.windowed_sizes.clear();
        self.phase = Phase::Exited;
        self.exit_code = Some(code);
        StepOutcome::Exited(code)
    }
}

/// The headless [`Backend`]: queue [`HostEvent`]s, then [`run`](Backend::run).
///
/// ```
/// use platform::window::{
///     App, AppContext, Backend, HeadlessBackend, HostEvent, WindowDesc, WindowEvent, WindowId,
/// };
///
/// struct Quit;
/// impl App for Quit {
///     fn resumed(&mut self, ctx: &mut AppContext) {
///         ctx.create_window(WindowDesc::new("Hello")).unwrap();
///     }
///     fn window_event(&mut self, ctx: &mut AppContext, _: WindowId, event: WindowEvent) {
///         if event == WindowEvent::CloseRequested {
///             ctx.exit_with_code(7);
///         }
///     }
/// }
///
/// let mut backend = HeadlessBackend::default();
/// backend.push(HostEvent::CloseRequested { window: WindowId::from_u64(1) }); // the first window
/// assert_eq!(backend.run(Quit).unwrap(), 7);
/// ```
#[derive(Debug, Clone, Default)]
pub struct HeadlessBackend {
    config: HeadlessConfig,
    events: Vec<HostEvent>,
}

impl HeadlessBackend {
    /// A backend on the simulated machine `config`.
    #[must_use]
    pub fn new(config: HeadlessConfig) -> Self {
        Self {
            config,
            events: Vec::new(),
        }
    }

    /// Queues an event for the run, delivered in the order pushed.
    pub fn push(&mut self, event: HostEvent) {
        self.events.push(event);
    }
}

impl Backend for HeadlessBackend {
    /// Runs the app on a [`HeadlessLoop`] until it exits.
    ///
    /// # Errors
    /// [`WindowError::Stalled`] when the loop goes idle without the app
    /// exiting, [`WindowError::StepLimit`] after [`DEFAULT_STEP_LIMIT`] steps,
    /// plus the errors of [`HeadlessLoop::new`] and [`HeadlessLoop::step`].
    fn run<A: App>(self, app: A) -> Result<i32, WindowError> {
        let mut event_loop = HeadlessLoop::new(app, self.config)?;
        for event in self.events {
            event_loop.push(event);
        }
        match event_loop.run_until_idle(DEFAULT_STEP_LIMIT)? {
            RunOutcome::Exited(code) => Ok(code),
            RunOutcome::Idle => Err(WindowError::Stalled),
            RunOutcome::StepLimit => Err(WindowError::StepLimit(DEFAULT_STEP_LIMIT)),
        }
    }
}
