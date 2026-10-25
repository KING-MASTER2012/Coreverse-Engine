//! Behavior of the lifecycle, driven through the headless backend.

use std::num::NonZeroUsize;

use super::*;

#[derive(Debug, Clone, PartialEq)]
enum Entry {
    Resumed,
    Suspended,
    AboutToWait,
    Event(WindowId, WindowEvent),
}

type Hook = Box<dyn FnMut(&mut AppContext)>;
type EventHook = Box<dyn FnMut(&mut AppContext, WindowId, &WindowEvent)>;

/// An app that logs every callback and runs optional hooks.
#[derive(Default)]
struct Probe {
    log: Vec<Entry>,
    on_resumed: Option<Hook>,
    on_suspended: Option<Hook>,
    on_wait: Option<Hook>,
    on_event: Option<EventHook>,
}

impl Probe {
    fn with_resumed(mut self, hook: impl FnMut(&mut AppContext) + 'static) -> Self {
        self.on_resumed = Some(Box::new(hook));
        self
    }

    fn with_wait(mut self, hook: impl FnMut(&mut AppContext) + 'static) -> Self {
        self.on_wait = Some(Box::new(hook));
        self
    }

    fn with_event(
        mut self,
        hook: impl FnMut(&mut AppContext, WindowId, &WindowEvent) + 'static,
    ) -> Self {
        self.on_event = Some(Box::new(hook));
        self
    }

    fn take_log(&mut self) -> Vec<Entry> {
        std::mem::take(&mut self.log)
    }
}

impl App for Probe {
    fn resumed(&mut self, ctx: &mut AppContext) {
        self.log.push(Entry::Resumed);
        if let Some(hook) = self.on_resumed.as_mut() {
            hook(ctx);
        }
    }

    fn suspended(&mut self, ctx: &mut AppContext) {
        self.log.push(Entry::Suspended);
        if let Some(hook) = self.on_suspended.as_mut() {
            hook(ctx);
        }
    }

    fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent) {
        if let Some(hook) = self.on_event.as_mut() {
            hook(ctx, window, &event);
        }
        self.log.push(Entry::Event(window, event));
    }

    fn about_to_wait(&mut self, ctx: &mut AppContext) {
        self.log.push(Entry::AboutToWait);
        if let Some(hook) = self.on_wait.as_mut() {
            hook(ctx);
        }
    }
}

fn id(n: u64) -> WindowId {
    WindowId::from_u64(n)
}

fn handle(n: usize) -> NativeWindowHandle {
    NativeWindowHandle::Android {
        native_window: NonZeroUsize::new(n).unwrap(),
    }
}

fn ev(window: u64, event: WindowEvent) -> Entry {
    Entry::Event(id(window), event)
}

fn new_loop(probe: Probe) -> HeadlessLoop<Probe> {
    HeadlessLoop::new(probe, HeadlessConfig::default()).unwrap()
}

/// A loop whose app creates `count` default windows when resumed (ids 1..=count).
fn loop_with_windows(count: usize) -> HeadlessLoop<Probe> {
    let probe = Probe::default().with_resumed(move |ctx| {
        for _ in 0..count {
            ctx.create_window(WindowDesc::default()).unwrap();
        }
    });
    new_loop(probe)
}

/// Steps once and discards the start-up entries, leaving a clean log.
fn started(mut event_loop: HeadlessLoop<Probe>) -> HeadlessLoop<Probe> {
    event_loop.step().unwrap();
    event_loop.app_mut().take_log();
    event_loop
}

fn run(event_loop: &mut HeadlessLoop<Probe>) -> RunOutcome {
    event_loop.run_until_idle(100).unwrap()
}

// --- start-up and idleness -------------------------------------------------

#[test]
fn the_first_step_resumes_then_waits() {
    let mut event_loop = new_loop(Probe::default());
    assert!(
        !event_loop.is_idle(),
        "a loop that has not started has work to do"
    );
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Running);
    assert_eq!(
        event_loop.app_mut().take_log(),
        [Entry::Resumed, Entry::AboutToWait]
    );
    assert!(event_loop.is_idle());
}

#[test]
fn run_until_idle_reports_idle_and_a_second_run_does_nothing_new() {
    let mut event_loop = new_loop(Probe::default());
    assert_eq!(run(&mut event_loop), RunOutcome::Idle);
    event_loop.app_mut().take_log();
    assert_eq!(run(&mut event_loop), RunOutcome::Idle);
    // Each run steps once, which calls about_to_wait once more.
    assert_eq!(event_loop.app_mut().take_log(), [Entry::AboutToWait]);
}

#[test]
fn poll_is_never_idle_and_hits_the_step_limit() {
    let probe = Probe::default().with_resumed(|ctx| ctx.set_control_flow(ControlFlow::Poll));
    let mut event_loop = new_loop(probe);
    assert_eq!(event_loop.run_until_idle(5).unwrap(), RunOutcome::StepLimit);
    assert!(!event_loop.is_idle());
}

#[test]
fn wait_until_counts_as_idle_without_a_clock() {
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_secs(3600),
        ));
    });
    let mut event_loop = new_loop(probe);
    assert_eq!(run(&mut event_loop), RunOutcome::Idle);
}

#[test]
fn an_invalid_configuration_is_rejected() {
    for config in [
        HeadlessConfig::default().with_scale_factor(0.0),
        HeadlessConfig::default().with_scale_factor(f64::NAN),
        HeadlessConfig::default().with_scale_factor(-1.0),
        HeadlessConfig::default().with_monitor_size(PhysicalSize::new(0, 1080)),
    ] {
        assert!(matches!(
            HeadlessLoop::new(Probe::default(), config),
            Err(WindowError::InvalidArgument(_))
        ));
    }
}

// --- windows ---------------------------------------------------------------

#[test]
fn windows_created_in_resumed_get_ids_from_one() {
    let event_loop = started(loop_with_windows(2));
    let ids: Vec<_> = event_loop.context().windows().map(Window::id).collect();
    assert_eq!(ids, [id(1), id(2)]);
}

#[test]
fn closing_a_window_destroys_it_after_the_handler_and_it_stays_readable_during_destroyed() {
    let seen_title = std::rc::Rc::new(std::cell::RefCell::new(None));
    let sink = seen_title.clone();
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::new("Doomed")).unwrap();
        })
        .with_event(move |ctx, window, event| match event {
            WindowEvent::CloseRequested => ctx.close_window(window).unwrap(),
            WindowEvent::Destroyed => {
                *sink.borrow_mut() = ctx.window(window).map(|w| w.title().to_owned())
            }
            _ => {}
        });
    let mut event_loop = started(new_loop(probe));

    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    event_loop.step().unwrap();

    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::CloseRequested),
            ev(1, WindowEvent::Destroyed),
            Entry::AboutToWait
        ]
    );
    assert_eq!(seen_title.borrow().as_deref(), Some("Doomed"));
    assert!(event_loop.context().window(id(1)).is_none());
}

#[test]
fn ignoring_close_requested_keeps_the_window() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    event_loop.step().unwrap();
    assert!(event_loop.context().window(id(1)).is_some());
}

#[test]
fn closing_twice_destroys_once_and_ids_are_not_reused() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_event(|ctx, window, event| {
            if *event == WindowEvent::CloseRequested {
                ctx.close_window(window).unwrap();
                ctx.close_window(window).unwrap();
            }
        });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    event_loop.step().unwrap();

    let destroyed = event_loop
        .app_mut()
        .take_log()
        .into_iter()
        .filter(|entry| *entry == ev(1, WindowEvent::Destroyed))
        .count();
    assert_eq!(destroyed, 1);

    let fresh = event_loop
        .context_mut()
        .create_window(WindowDesc::default())
        .unwrap();
    assert_eq!(fresh, id(2));
}

#[test]
fn host_events_for_unknown_windows_are_ignored() {
    let mut event_loop = started(loop_with_windows(1));
    for event in [
        HostEvent::CloseRequested { window: id(9) },
        HostEvent::Resized {
            window: id(9),
            size: PhysicalSize::new(1, 1),
        },
        HostEvent::Moved {
            window: id(9),
            position: PhysicalPosition::new(1, 1),
        },
        HostEvent::ScaleFactorChanged {
            window: id(9),
            scale_factor: 2.0,
        },
        HostEvent::Focused {
            window: id(9),
            focused: true,
        },
        HostEvent::Occluded {
            window: id(9),
            occluded: true,
        },
        HostEvent::SurfaceCreated {
            window: id(9),
            handle: handle(1),
        },
        HostEvent::SurfaceLost { window: id(9) },
    ] {
        event_loop.push(event);
    }
    event_loop.step().unwrap();
    assert_eq!(event_loop.app_mut().take_log(), [Entry::AboutToWait]);
}

#[test]
fn windows_can_be_created_from_an_event_handler() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_event(|ctx, _, event| {
            if *event == WindowEvent::Focused(true) {
                ctx.create_window(WindowDesc::new("Child")).unwrap();
            }
        });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::Focused {
        window: id(1),
        focused: true,
    });
    event_loop.step().unwrap();
    assert_eq!(event_loop.context().window(id(2)).unwrap().title(), "Child");
}

// --- state reported by the system ------------------------------------------

#[test]
fn focus_occlusion_move_and_resize_update_the_window_and_are_delivered() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::Focused {
        window: id(1),
        focused: true,
    });
    event_loop.push(HostEvent::Occluded {
        window: id(1),
        occluded: true,
    });
    event_loop.push(HostEvent::Moved {
        window: id(1),
        position: PhysicalPosition::new(30, 40),
    });
    event_loop.push(HostEvent::Resized {
        window: id(1),
        size: PhysicalSize::new(640, 480),
    });
    event_loop.step().unwrap();

    let window = event_loop.context().window(id(1)).unwrap();
    assert!(window.is_focused() && window.is_occluded());
    assert_eq!(window.position(), Some(PhysicalPosition::new(30, 40)));
    assert_eq!(window.inner_size(), PhysicalSize::new(640, 480));
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::Focused(true)),
            ev(1, WindowEvent::Occluded(true)),
            ev(1, WindowEvent::Moved(PhysicalPosition::new(30, 40))),
            ev(1, WindowEvent::Resized(PhysicalSize::new(640, 480))),
            Entry::AboutToWait,
        ]
    );
}

#[test]
fn a_scale_factor_change_keeps_the_logical_size() {
    let mut event_loop = started(loop_with_windows(1));
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(1280, 720)
    );
    event_loop.push(HostEvent::ScaleFactorChanged {
        window: id(1),
        scale_factor: 2.0,
    });
    event_loop.step().unwrap();

    let window = event_loop.context().window(id(1)).unwrap();
    assert_eq!(window.scale_factor(), 2.0);
    assert_eq!(window.inner_size(), PhysicalSize::new(2560, 1440));
    assert_eq!(window.logical_inner_size(), LogicalSize::new(1280.0, 720.0));
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(
                1,
                WindowEvent::ScaleFactorChanged {
                    scale_factor: 2.0,
                    inner_size: PhysicalSize::new(2560, 1440)
                }
            ),
            Entry::AboutToWait,
        ]
    );
}

#[test]
fn an_unusable_scale_factor_fails_the_step() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::ScaleFactorChanged {
        window: id(1),
        scale_factor: 0.0,
    });
    assert!(matches!(
        event_loop.step(),
        Err(WindowError::InvalidArgument(_))
    ));
}

#[test]
fn windows_start_at_the_configured_scale_factor() {
    let config = HeadlessConfig::default().with_scale_factor(1.5);
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.create_window(WindowDesc::default()).unwrap();
    });
    let mut event_loop = HeadlessLoop::new(probe, config).unwrap();
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(1920, 1080)
    );
}

// --- requests --------------------------------------------------------------

#[test]
fn a_resize_request_comes_back_as_a_resized_event_in_the_same_step() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop
        .context_mut()
        .set_inner_size(id(1), LogicalSize::new(800.0, 600.0))
        .unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(1280, 720),
        "still the old size until the system reports"
    );
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(800, 600)
    );
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::Resized(PhysicalSize::new(800, 600))),
            Entry::AboutToWait
        ]
    );
}

#[test]
fn a_resize_request_is_limited_by_the_window_limits_and_ignored_when_nothing_changes() {
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.create_window(
            WindowDesc::default()
                .with_min_inner_size(LogicalSize::new(640.0, 480.0))
                .with_max_inner_size(LogicalSize::new(1920.0, 1080.0)),
        )
            .unwrap();
    });
    let mut event_loop = started(new_loop(probe));

    event_loop
        .context_mut()
        .set_inner_size(id(1), LogicalSize::new(10.0, 10.0))
        .unwrap();
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(640, 480)
    );

    event_loop.app_mut().take_log();
    event_loop
        .context_mut()
        .set_inner_size(id(1), LogicalSize::new(10.0, 10.0))
        .unwrap();
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [Entry::AboutToWait],
        "already at the minimum"
    );
}

#[test]
fn a_move_request_comes_back_as_moved_once() {
    let mut event_loop = started(loop_with_windows(1));
    for _ in 0..2 {
        event_loop
            .context_mut()
            .set_position(id(1), LogicalPosition::new(100.0, 50.0))
            .unwrap();
        event_loop.step().unwrap();
    }
    let moves = event_loop
        .app_mut()
        .take_log()
        .into_iter()
        .filter(|entry| matches!(entry, Entry::Event(_, WindowEvent::Moved(_))))
        .count();
    assert_eq!(moves, 1);
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().position(),
        Some(PhysicalPosition::new(100, 50))
    );
}

#[test]
fn fullscreen_fills_the_monitor_and_leaving_restores_the_windowed_size() {
    let config = HeadlessConfig::default().with_monitor_size(PhysicalSize::new(2560, 1440));
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.create_window(WindowDesc::default()).unwrap();
    });
    let mut event_loop = HeadlessLoop::new(probe, config).unwrap();
    event_loop.step().unwrap();
    event_loop.app_mut().take_log();

    event_loop
        .context_mut()
        .set_fullscreen(id(1), Fullscreen::Borderless)
        .unwrap();
    event_loop.step().unwrap();
    let window = event_loop.context().window(id(1)).unwrap();
    assert_eq!(window.fullscreen(), Fullscreen::Borderless);
    assert_eq!(window.inner_size(), PhysicalSize::new(2560, 1440));

    // A size request is ignored while fullscreen.
    event_loop
        .context_mut()
        .set_inner_size(id(1), LogicalSize::new(300.0, 300.0))
        .unwrap();
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().inner_size(),
        PhysicalSize::new(2560, 1440)
    );

    event_loop
        .context_mut()
        .set_fullscreen(id(1), Fullscreen::None)
        .unwrap();
    event_loop.step().unwrap();
    let window = event_loop.context().window(id(1)).unwrap();
    assert_eq!(window.fullscreen(), Fullscreen::None);
    assert_eq!(window.inner_size(), PhysicalSize::new(1280, 720));

    let resizes: Vec<_> = event_loop
        .app_mut()
        .take_log()
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Event(_, WindowEvent::Resized(size)) => Some(size),
            _ => None,
        })
        .collect();
    assert_eq!(
        resizes,
        [PhysicalSize::new(2560, 1440), PhysicalSize::new(1280, 720)]
    );
}

#[test]
fn a_window_created_fullscreen_is_resized_to_the_monitor() {
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.create_window(WindowDesc::default().with_fullscreen(Fullscreen::Borderless))
            .unwrap();
    });
    let mut event_loop = new_loop(probe);
    event_loop.step().unwrap();
    let window = event_loop.context().window(id(1)).unwrap();
    assert_eq!(window.fullscreen(), Fullscreen::Borderless);
    assert_eq!(window.inner_size(), PhysicalSize::new(1920, 1080));
}

#[test]
fn an_app_that_answers_every_resize_with_another_one_is_stopped() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_event(|ctx, window, event| {
            if let WindowEvent::Resized(size) = event {
                let next = LogicalSize::new(f64::from(size.width) + 1.0, f64::from(size.height));
                ctx.set_inner_size(window, next).unwrap();
            }
        });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::Resized {
        window: id(1),
        size: PhysicalSize::new(700, 500),
    });
    assert!(matches!(
        event_loop.step(),
        Err(WindowError::EventStorm(MAX_EVENTS_PER_STEP))
    ));
}

#[test]
fn the_cursor_grab_is_applied_and_reported() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop
        .context_mut()
        .set_cursor_grab(id(1), CursorGrab::Locked)
        .unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().cursor_grab(),
        CursorGrab::None,
        "a request until the system answers"
    );
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().cursor_grab(),
        CursorGrab::Locked
    );
    event_loop
        .context_mut()
        .set_cursor_grab(id(1), CursorGrab::None)
        .unwrap();
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().cursor_grab(),
        CursorGrab::None
    );
}

#[test]
fn the_simulated_monitor_follows_the_configuration() {
    let config = HeadlessConfig::default()
        .with_scale_factor(2.0)
        .with_monitor_size(PhysicalSize::new(3840, 2160));
    let event_loop = HeadlessLoop::new(Probe::default(), config).unwrap();
    let monitors = event_loop.context().monitors();
    assert_eq!(monitors.len(), 1);
    let monitor = event_loop.context().primary_monitor().unwrap();
    assert_eq!(monitor.size, PhysicalSize::new(3840, 2160));
    assert_eq!(monitor.scale_factor, 2.0);
    assert_eq!(monitor.position, PhysicalPosition::new(0, 0));
    assert_eq!(monitor.refresh_rate_millihertz, Some(60_000));
    assert!(monitor.name.is_some());
}

// --- redraw ----------------------------------------------------------------

#[test]
fn a_redraw_requested_in_about_to_wait_is_delivered_after_it_in_the_same_step() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_wait(|ctx| ctx.request_redraw(id(1)).unwrap());
    let mut event_loop = new_loop(probe);
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            Entry::Resumed,
            Entry::AboutToWait,
            ev(1, WindowEvent::RedrawRequested)
        ]
    );
}

#[test]
fn a_redraw_requested_inside_the_redraw_is_delivered_in_the_next_step() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
            ctx.request_redraw(id(1)).unwrap();
        })
        .with_event(|ctx, window, event| {
            if *event == WindowEvent::RedrawRequested {
                ctx.request_redraw(window).unwrap();
            }
        });
    let mut event_loop = new_loop(probe);
    event_loop.step().unwrap();
    assert!(!event_loop.is_idle(), "the new request is pending");
    event_loop.step().unwrap();
    let redraws = event_loop
        .app_mut()
        .take_log()
        .into_iter()
        .filter(|entry| *entry == ev(1, WindowEvent::RedrawRequested))
        .count();
    assert_eq!(redraws, 2, "one per step");
}

#[test]
fn redraws_wait_while_suspended_and_are_delivered_after_resuming() {
    let probe = Probe::default().with_resumed(|ctx| {
        ctx.create_window(WindowDesc::default()).unwrap();
    });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::Suspend);
    event_loop.step().unwrap();
    event_loop.app_mut().take_log();

    event_loop.context_mut().request_redraw(id(1)).unwrap();
    assert!(
        event_loop.is_idle(),
        "a redraw cannot be delivered while suspended"
    );
    event_loop.step().unwrap();
    assert_eq!(event_loop.app_mut().take_log(), [Entry::AboutToWait]);

    event_loop.push(HostEvent::Resume);
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            Entry::Resumed,
            Entry::AboutToWait,
            ev(1, WindowEvent::RedrawRequested)
        ]
    );
}

// --- suspend, resume and surfaces ------------------------------------------

#[test]
fn a_surface_is_attached_and_replaced_with_a_loss_in_between() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(0x10),
    });
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(0x20),
    });
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::SurfaceCreated(handle(0x10))),
            ev(1, WindowEvent::SurfaceLost),
            ev(1, WindowEvent::SurfaceCreated(handle(0x20))),
            Entry::AboutToWait,
        ]
    );
    assert_eq!(
        event_loop.context().window(id(1)).unwrap().native_handle(),
        Some(handle(0x20))
    );
}

#[test]
fn a_lost_surface_clears_the_handle_and_a_second_loss_is_ignored() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(0x10),
    });
    event_loop.push(HostEvent::SurfaceLost { window: id(1) });
    event_loop.push(HostEvent::SurfaceLost { window: id(1) });
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::SurfaceCreated(handle(0x10))),
            ev(1, WindowEvent::SurfaceLost),
            Entry::AboutToWait
        ]
    );
    assert!(
        event_loop
            .context()
            .window(id(1))
            .unwrap()
            .native_handle()
            .is_none()
    );
}

#[test]
fn suspending_loses_every_surface_before_suspended_and_resuming_does_not_bring_them_back() {
    let mut event_loop = started(loop_with_windows(3));
    for (window, address) in [(1, 0x10), (3, 0x30)] {
        event_loop.push(HostEvent::SurfaceCreated {
            window: id(window),
            handle: handle(address),
        });
    }
    event_loop.step().unwrap();
    event_loop.app_mut().take_log();

    event_loop.push(HostEvent::Suspend);
    event_loop.step().unwrap();
    assert!(event_loop.is_suspended());
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::SurfaceLost),
            ev(3, WindowEvent::SurfaceLost),
            Entry::Suspended,
            Entry::AboutToWait
        ]
    );
    assert!(
        event_loop
            .context()
            .windows()
            .all(|w| w.native_handle().is_none())
    );

    event_loop.push(HostEvent::Resume);
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(0x11),
    });
    event_loop.step().unwrap();
    assert!(!event_loop.is_suspended());
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            Entry::Resumed,
            ev(1, WindowEvent::SurfaceCreated(handle(0x11))),
            Entry::AboutToWait
        ]
    );
    assert!(
        event_loop
            .context()
            .window(id(3))
            .unwrap()
            .native_handle()
            .is_none()
    );
}

#[test]
fn suspend_and_resume_are_ignored_in_the_wrong_state() {
    let mut event_loop = started(loop_with_windows(1));
    event_loop.push(HostEvent::Resume); // already resumed
    event_loop.step().unwrap();
    assert_eq!(event_loop.app_mut().take_log(), [Entry::AboutToWait]);

    event_loop.push(HostEvent::Suspend);
    event_loop.push(HostEvent::Suspend); // already suspended
    event_loop.step().unwrap();
    let suspended = event_loop
        .app_mut()
        .take_log()
        .into_iter()
        .filter(|entry| *entry == Entry::Suspended)
        .count();
    assert_eq!(suspended, 1);
}

#[test]
fn closing_a_window_with_a_surface_loses_the_surface_first() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_event(|ctx, window, event| {
            if *event == WindowEvent::CloseRequested {
                ctx.close_window(window).unwrap();
            }
        });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(0x10),
    });
    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    event_loop.step().unwrap();
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(1, WindowEvent::SurfaceCreated(handle(0x10))),
            ev(1, WindowEvent::CloseRequested),
            ev(1, WindowEvent::SurfaceLost),
            ev(1, WindowEvent::Destroyed),
            Entry::AboutToWait,
        ]
    );
}

// --- exit ------------------------------------------------------------------

#[test]
fn exit_destroys_the_remaining_windows_in_id_order_and_stops_delivering() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            for _ in 0..2 {
                ctx.create_window(WindowDesc::default()).unwrap();
            }
        })
        .with_event(|ctx, _, event| {
            if *event == WindowEvent::CloseRequested {
                ctx.exit_with_code(4);
            }
        });
    let mut event_loop = started(new_loop(probe));
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(2),
        handle: handle(0x20),
    });
    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    event_loop.push(HostEvent::Focused {
        window: id(2),
        focused: true,
    }); // never delivered
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(4));

    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            ev(2, WindowEvent::SurfaceCreated(handle(0x20))),
            ev(1, WindowEvent::CloseRequested),
            ev(1, WindowEvent::Destroyed),
            ev(2, WindowEvent::SurfaceLost),
            ev(2, WindowEvent::Destroyed),
        ]
    );
    assert_eq!(event_loop.exit_code(), Some(4));
    assert_eq!(event_loop.context().windows().count(), 0);
    assert!(event_loop.is_idle());
    // A finished loop keeps reporting its exit code and calls nothing.
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(4));
    assert!(event_loop.app_mut().take_log().is_empty());
}

#[test]
fn exiting_while_resuming_skips_the_rest_of_the_step() {
    let probe = Probe::default().with_resumed(|ctx| ctx.exit());
    let mut event_loop = new_loop(probe);
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(0));
    assert_eq!(event_loop.app_mut().take_log(), [Entry::Resumed]);
}

#[test]
fn exit_from_about_to_wait_and_from_a_redraw_both_end_the_loop() {
    let probe = Probe::default().with_wait(|ctx| ctx.exit_with_code(1));
    let mut event_loop = new_loop(probe);
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(1));

    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
            ctx.request_redraw(id(1)).unwrap();
        })
        .with_event(|ctx, _, event| {
            if *event == WindowEvent::RedrawRequested {
                ctx.exit_with_code(2);
            }
        });
    let mut event_loop = new_loop(probe);
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(2));
    assert_eq!(
        event_loop.app_mut().take_log().last(),
        Some(&ev(1, WindowEvent::Destroyed))
    );
}

#[test]
fn requests_made_while_exiting_are_ignored() {
    let probe = Probe::default()
        .with_resumed(|ctx| {
            ctx.create_window(WindowDesc::default()).unwrap();
        })
        .with_event(|ctx, window, event| {
            if *event == WindowEvent::Destroyed {
                // The loop is already shutting down.
                ctx.create_window(WindowDesc::default()).unwrap();
                let _ = ctx.close_window(window);
            }
        })
        .with_wait(|ctx| ctx.exit());
    let mut event_loop = new_loop(probe);
    assert_eq!(event_loop.step().unwrap(), StepOutcome::Exited(0));
    assert_eq!(event_loop.context().windows().count(), 0);
}

// --- the Backend contract --------------------------------------------------

struct QuitOnClose(i32);

impl App for QuitOnClose {
    fn resumed(&mut self, ctx: &mut AppContext) {
        ctx.create_window(WindowDesc::default()).unwrap();
    }

    fn window_event(&mut self, ctx: &mut AppContext, _: WindowId, event: WindowEvent) {
        if event == WindowEvent::CloseRequested {
            ctx.exit_with_code(self.0);
        }
    }
}

#[test]
fn backend_run_returns_the_exit_code() {
    let mut backend = HeadlessBackend::default();
    backend.push(HostEvent::CloseRequested { window: id(1) });
    assert_eq!(backend.run(QuitOnClose(5)).unwrap(), 5);
}

#[test]
fn backend_run_reports_a_stalled_loop() {
    let backend = HeadlessBackend::default();
    assert!(matches!(
        backend.run(QuitOnClose(0)),
        Err(WindowError::Stalled)
    ));
}

#[test]
fn backend_run_rejects_an_invalid_configuration() {
    let backend = HeadlessBackend::new(HeadlessConfig::default().with_scale_factor(0.0));
    assert!(matches!(
        backend.run(QuitOnClose(0)),
        Err(WindowError::InvalidArgument(_))
    ));
}

#[test]
fn the_lifecycle_is_the_same_for_a_whole_session() {
    // A condensed mobile-style session: start, get a surface, go to the
    // background, come back with a new surface, close.
    let mut event_loop = new_loop(
        Probe::default()
            .with_resumed(|ctx| {
                if ctx.windows().next().is_none() {
                    ctx.create_window(WindowDesc::default()).unwrap();
                }
            })
            .with_event(|ctx, window, event| {
                if *event == WindowEvent::CloseRequested {
                    ctx.close_window(window).unwrap();
                    ctx.exit();
                }
            }),
    );
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(1),
    });
    event_loop.push(HostEvent::Suspend);
    event_loop.push(HostEvent::Resume);
    event_loop.push(HostEvent::SurfaceCreated {
        window: id(1),
        handle: handle(2),
    });
    event_loop.push(HostEvent::CloseRequested { window: id(1) });
    assert_eq!(run(&mut event_loop), RunOutcome::Exited(0));

    // The window exists from the first resume on, so the first surface is
    // attached right away; it is lost before `suspended`.
    assert_eq!(
        event_loop.app_mut().take_log(),
        [
            Entry::Resumed,
            ev(1, WindowEvent::SurfaceCreated(handle(1))),
            ev(1, WindowEvent::SurfaceLost),
            Entry::Suspended,
            Entry::Resumed,
            ev(1, WindowEvent::SurfaceCreated(handle(2))),
            ev(1, WindowEvent::CloseRequested),
            ev(1, WindowEvent::SurfaceLost),
            ev(1, WindowEvent::Destroyed),
        ]
    );
}
