//! Smoke test of the desktop backend: opens a real window on a real X server
//! (Xvfb in CI) and walks it through its life.
//!
//! `winit` allows one event loop per process, so everything happens in this one
//! test. It needs a display, so without `DISPLAY` or `WAYLAND_DISPLAY` it is
//! skipped, unless `CV_REQUIRE_DISPLAY` is set (the CI job sets it, so a missing
//! X server cannot turn into a silent pass).
//!
//! Fullscreen can only be checked when a window manager runs next to the X
//! server, because on X11 fullscreen is a request *to the window manager*. The
//! CI job starts one and sets `CV_EXPECT_WM`; without it a missing resize is
//! only noted.

#![cfg(all(feature = "winit", target_os = "linux"))]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use platform::window::{
    App, AppContext, Backend, ControlFlow, CursorGrab, CursorIcon, Fullscreen, LogicalSize,
    NativeWindowHandle, PhysicalSize, WindowDesc, WindowEvent, WindowId, WinitBackend,
};

const TIMEOUT: Duration = Duration::from_secs(30);
const STEP_TIMEOUT: Duration = Duration::from_secs(3);
const EXIT_CODE: i32 = 42;

#[derive(Default)]
struct Report {
    failures: Vec<String>,
    notes: Vec<String>,
    events: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum Stage {
    WaitSurface,
    Resize,
    ClampMax,
    ClampMin,
    CursorCheck,
    Fullscreen(Instant),
    Windowed(Instant),
    Redraw(Instant),
    Close,
}

struct Smoke {
    report: Rc<RefCell<Report>>,
    expect_wm: bool,
    window: Option<WindowId>,
    stage: Stage,
    started: Instant,
    surface: Option<NativeWindowHandle>,
    resized: Option<PhysicalSize>,
    redrawn: bool,
    surface_lost: bool,
    destroyed: bool,
    went_fullscreen: bool,
}

impl Smoke {
    fn fail(&self, message: impl Into<String>) {
        self.report.borrow_mut().failures.push(message.into());
    }

    fn note(&self, message: impl Into<String>) {
        self.report.borrow_mut().notes.push(message.into());
    }

    fn scale(ctx: &AppContext, id: WindowId) -> f64 {
        ctx.window(id)
            .map_or(1.0, platform::window::Window::scale_factor)
    }
}

impl App for Smoke {
    fn resumed(&mut self, ctx: &mut AppContext) {
        if ctx.monitors().is_empty() {
            self.fail("no monitor reported");
        }
        match ctx.primary_monitor() {
            Some(primary) => {
                if primary.size.width == 0
                    || primary.size.height == 0
                    || primary.scale_factor <= 0.0
                {
                    self.fail(format!("implausible primary monitor: {primary:?}"));
                }
            }
            None => self.fail("no primary monitor"),
        }
        let desc = WindowDesc::new("smoke")
            .with_inner_size(LogicalSize::new(640.0, 480.0))
            // Larger than any screen the CI uses: a window manager refuses to
            // make a fullscreen window bigger than its maximum size.
            .with_min_inner_size(LogicalSize::new(200.0, 150.0))
            .with_max_inner_size(LogicalSize::new(4000.0, 3000.0));
        self.window = Some(ctx.create_window(desc).expect("valid description"));
    }

    fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent) {
        self.report.borrow_mut().events.push(format!("{event:?}"));
        if Some(window) != self.window {
            self.fail(format!("event for an unknown window {window}: {event:?}"));
            return;
        }
        match event {
            WindowEvent::SurfaceCreated(handle) => {
                if !matches!(
                    handle,
                    NativeWindowHandle::X11 { .. } | NativeWindowHandle::Wayland { .. }
                ) {
                    self.fail(format!("unexpected handle on Linux: {handle:?}"));
                }
                if ctx
                    .window(window)
                    .and_then(platform::window::Window::native_handle)
                    != Some(handle)
                {
                    self.fail("the window does not report the handle it was just given");
                }
                self.surface = Some(handle);
            }
            WindowEvent::SurfaceLost => {
                if self.destroyed {
                    self.fail("SurfaceLost after Destroyed");
                }
                self.surface_lost = true;
            }
            WindowEvent::Destroyed => {
                if self.surface.is_some() && !self.surface_lost {
                    self.fail("Destroyed without SurfaceLost for a window that had a surface");
                }
                self.destroyed = true;
            }
            WindowEvent::Resized(size) => self.resized = Some(size),
            WindowEvent::RedrawRequested => self.redrawn = true,
            _ => {}
        }
    }

    fn about_to_wait(&mut self, ctx: &mut AppContext) {
        ctx.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(10),
        ));
        if self.started.elapsed() > TIMEOUT {
            self.fail(format!("timed out in stage {:?}", self.stage));
            ctx.exit_with_code(99);
            return;
        }
        let Some(id) = self.window else { return };
        if ctx.window(id).is_none() && !self.destroyed {
            return;
        }
        let scale = Self::scale(ctx, id);

        match self.stage {
            Stage::WaitSurface => {
                if self.surface.is_some() {
                    self.resized = None;
                    ctx.set_inner_size(id, LogicalSize::new(800.0, 600.0))
                        .unwrap();
                    self.stage = Stage::Resize;
                }
            }
            Stage::Resize => {
                if let Some(size) = self.resized.take() {
                    let expected = LogicalSize::new(800.0, 600.0).to_physical(scale);
                    if size != expected {
                        self.fail(format!("resize: got {size:?}, expected {expected:?}"));
                    }
                    if ctx.window(id).map(platform::window::Window::inner_size) != Some(size) {
                        self.fail("the window state does not match the Resized event");
                    }
                    // Far beyond the maximum: the request must be limited.
                    ctx.set_inner_size(id, LogicalSize::new(9000.0, 9000.0))
                        .unwrap();
                    self.stage = Stage::ClampMax;
                }
            }
            Stage::ClampMax => {
                if let Some(size) = self.resized.take() {
                    let expected = LogicalSize::new(4000.0, 3000.0).to_physical(scale);
                    if size != expected {
                        self.fail(format!(
                            "clamp to the maximum: got {size:?}, expected {expected:?}"
                        ));
                    }
                    ctx.set_inner_size(id, LogicalSize::new(10.0, 10.0))
                        .unwrap();
                    self.stage = Stage::ClampMin;
                }
            }
            Stage::ClampMin => {
                if let Some(size) = self.resized.take() {
                    let expected = LogicalSize::new(200.0, 150.0).to_physical(scale);
                    if size != expected {
                        self.fail(format!(
                            "clamp to the minimum: got {size:?}, expected {expected:?}"
                        ));
                    }
                    ctx.set_title(id, "renamed").unwrap();
                    ctx.set_cursor_icon(id, CursorIcon::Text).unwrap();
                    ctx.set_cursor_visible(id, false).unwrap();
                    ctx.set_cursor_grab(id, CursorGrab::Confined).unwrap();
                    self.stage = Stage::CursorCheck;
                }
            }
            Stage::CursorCheck => {
                let window = ctx.window(id).expect("window exists");
                if window.title() != "renamed"
                    || window.cursor_icon() != CursorIcon::Text
                    || window.is_cursor_visible()
                {
                    self.fail("title or cursor state does not read back");
                }
                if window.cursor_grab() != CursorGrab::Confined {
                    self.note(format!(
                        "the X server refused to confine the cursor: {:?}",
                        window.cursor_grab()
                    ));
                }
                ctx.set_cursor_grab(id, CursorGrab::None).unwrap();
                ctx.set_cursor_visible(id, true).unwrap();
                self.resized = None;
                ctx.set_fullscreen(id, Fullscreen::Borderless).unwrap();
                self.stage = Stage::Fullscreen(Instant::now());
            }
            Stage::Fullscreen(since) => {
                if let Some(size) = self.resized.take() {
                    self.went_fullscreen = true;
                    let monitor = ctx.primary_monitor().map(|m| m.size);
                    if Some(size) != monitor {
                        self.fail(format!(
                            "fullscreen size {size:?} is not the monitor size {monitor:?}"
                        ));
                    }
                    if ctx.window(id).map(platform::window::Window::fullscreen)
                        != Some(Fullscreen::Borderless)
                    {
                        self.fail("the window does not report being fullscreen");
                    }
                    self.leave_fullscreen(ctx, id);
                } else if since.elapsed() > STEP_TIMEOUT {
                    if self.expect_wm {
                        self.fail("no Resized after the fullscreen request although a window manager runs");
                    } else {
                        self.note("no window manager: the fullscreen request had no effect");
                    }
                    self.leave_fullscreen(ctx, id);
                }
            }
            Stage::Windowed(since) => {
                let back = self.resized.take();
                if let Some(size) = back {
                    let expected = LogicalSize::new(200.0, 150.0).to_physical(scale);
                    if size != expected {
                        self.fail(format!(
                            "leaving fullscreen: got {size:?}, expected {expected:?}"
                        ));
                    }
                }
                if back.is_some() || !self.went_fullscreen || since.elapsed() > STEP_TIMEOUT {
                    if self.went_fullscreen && back.is_none() {
                        self.fail("no Resized after leaving fullscreen");
                    }
                    if ctx.window(id).map(platform::window::Window::fullscreen)
                        != Some(Fullscreen::None)
                    {
                        self.fail("the window still reports being fullscreen");
                    }
                    ctx.request_redraw(id).unwrap();
                    self.stage = Stage::Redraw(Instant::now());
                }
            }
            Stage::Redraw(since) => {
                if self.redrawn {
                    ctx.close_window(id).unwrap();
                    self.stage = Stage::Close;
                } else if since.elapsed() > STEP_TIMEOUT {
                    self.fail("no RedrawRequested after request_redraw");
                    ctx.close_window(id).unwrap();
                    self.stage = Stage::Close;
                }
            }
            Stage::Close => {
                if self.destroyed {
                    ctx.exit_with_code(EXIT_CODE);
                }
            }
        }
    }
}

impl Smoke {
    fn leave_fullscreen(&mut self, ctx: &mut AppContext, id: WindowId) {
        self.resized = None;
        ctx.set_fullscreen(id, Fullscreen::None).unwrap();
        self.stage = Stage::Windowed(Instant::now());
    }
}

#[test]
fn a_real_window_goes_through_its_whole_life() {
    let has_display =
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    if !has_display {
        assert!(
            std::env::var_os("CV_REQUIRE_DISPLAY").is_none(),
            "CV_REQUIRE_DISPLAY is set but there is no display (start an X server, e.g. xvfb-run)"
        );
        eprintln!(
            "skipping: no DISPLAY or WAYLAND_DISPLAY (set CV_REQUIRE_DISPLAY=1 to make this an error)"
        );
        return;
    }

    let report = Rc::new(RefCell::new(Report::default()));
    let app = Smoke {
        report: Rc::clone(&report),
        expect_wm: std::env::var_os("CV_EXPECT_WM").is_some(),
        window: None,
        stage: Stage::WaitSurface,
        started: Instant::now(),
        surface: None,
        resized: None,
        redrawn: false,
        surface_lost: false,
        destroyed: false,
        went_fullscreen: false,
    };

    let exit_code = WinitBackend::new()
        .with_any_thread(true)
        .run(app)
        .expect("the event loop runs");

    let report = report.borrow();
    for note in &report.notes {
        eprintln!("note: {note}");
    }
    assert!(
        report.failures.is_empty(),
        "failures: {:#?}\nevents: {:#?}",
        report.failures,
        report.events
    );
    assert_eq!(exit_code, EXIT_CODE, "events: {:#?}", report.events);

    let position = |name: &str| report.events.iter().position(|e| e.starts_with(name));
    let created = position("SurfaceCreated").expect("a surface was created");
    let lost = position("SurfaceLost").expect("the surface was lost");
    let destroyed = position("Destroyed").expect("the window was destroyed");
    assert!(
        created < lost && lost < destroyed,
        "order: {:#?}",
        report.events
    );
}
