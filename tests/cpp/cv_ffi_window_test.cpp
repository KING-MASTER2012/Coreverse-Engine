// Phase 10 proof: the window event loop of the Rust `platform` crate through
// cv-ffi's C++ wrapper, on the headless backend, so it runs without a display.
//
// What it shows, against the real Rust library:
//   * Run() calls Resumed, then the window events and AboutToWait, and returns
//     the exit code the application asked for;
//   * a window created through Context exists at once and reads back what was
//     asked (the C++ WindowDesc defaults equal the Rust ones);
//   * a Context kept after its callback is refused with an error, not used;
//   * an unusable description and an unknown window come back as errors;
//   * an exception leaving a callback stops the loop and comes back from Run();
//   * the headless backend reports an application that never exits;
//   * the key tables (KeyName) and the Modifiers helpers.
//
// Real windows are covered by the Xvfb smoke test of the platform crate and by
// running the runtime executable (see docs/src/runtime.md); input events are
// converted and checked in the Rust tests of ffi/src/platform_run.rs.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <new>
#include <functional>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

#include "cv-ffi/Keys.hpp"
#include "cv-ffi/Window.hpp"

namespace window = cv_ffi::window;

namespace {

int g_failures = 0;

void Check(bool condition, const char* what)
{
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        ++g_failures;
    }
}

/// An application whose behaviour is a set of std::function hooks, recording
/// what the loop called.
class ScriptedApp : public window::App
{
public:
    std::function<void(window::Context&)> onResumed;
    std::function<void(window::Context&)> onAboutToWait;
    std::function<void(window::Context&, window::WindowId, const window::Event&)> onEvent;
    std::vector<std::string> log;

    void Resumed(window::Context& ctx) override
    {
        log.emplace_back("resumed");
        if (onResumed) {
            onResumed(ctx);
        }
    }

    void Suspended(window::Context&) override { log.emplace_back("suspended"); }

    void OnEvent(window::Context& ctx, window::WindowId id, const window::Event& event) override
    {
        log.emplace_back(std::holds_alternative<window::Destroyed>(event) ? "destroyed" : "event");
        if (onEvent) {
            onEvent(ctx, id, event);
        }
    }

    void AboutToWait(window::Context& ctx) override
    {
        log.emplace_back("about_to_wait");
        if (onAboutToWait) {
            onAboutToWait(ctx);
        }
    }
};

window::WindowDesc Sized(double width, double height)
{
    window::WindowDesc desc;
    desc.width = width;
    desc.height = height;
    return desc;
}

window::RunConfig Headless()
{
    window::RunConfig config;
    config.backend = window::Backend::Headless;
    return config;
}

void TestLifecycleAndExitCode()
{
    ScriptedApp app;
    window::WindowId created = 0;
    window::Context* kept = nullptr; // only to compare pointers below, never used
    app.onResumed = [&](window::Context& ctx) {
        kept = &ctx;
        auto id = ctx.CreateWindow({});
        Check(id.has_value(), "CreateWindow with the default description succeeds");
        if (id) {
            created = *id;
            // The window exists at once and carries the Rust defaults.
            auto info = ctx.GetWindowInfo(*id);
            Check(info.has_value(), "GetWindowInfo of a new window succeeds");
            if (info) {
                Check(info->innerWidth == 1280 && info->innerHeight == 720, "the C++ default size is the Rust one");
                Check(info->scaleFactor == 1.0, "the headless scale factor is 1");
                Check(info->visible && !info->imeAllowed && !info->focused, "a new window is visible, unfocused");
                Check(info->fullscreen == window::Fullscreen::None, "a new window is not fullscreen");
                Check(info->cursorGrab == window::CursorGrab::None, "a new window has a free cursor");
            }
        }
    };
    app.onAboutToWait = [&](window::Context& ctx) {
        Check(ctx.SetTitle(created, "Renamed").has_value(), "SetTitle");
        Check(ctx.SetImeAllowed(created, true).has_value(), "SetImeAllowed");
        Check(ctx.SetImeCursorArea(created, 1.0, 2.0, 3.0, 4.0).has_value(), "SetImeCursorArea");
        Check(ctx.SetCursorVisible(created, false).has_value(), "SetCursorVisible");
        Check(ctx.SetControlFlow(window::ControlFlow::Poll).has_value(), "SetControlFlow Poll");
        Check(ctx.SetControlFlow(window::ControlFlow::WaitUntil, std::chrono::milliseconds{5}).has_value(),
              "SetControlFlow WaitUntil");
        auto info = ctx.GetWindowInfo(created);
        Check(info && info->imeAllowed, "the IME permission reads back");
        Check(ctx.Exit(42).has_value(), "Exit");
    };

    auto code = window::Run(app, Headless());
    Check(code.has_value(), "Run succeeds when the application exits");
    Check(code && *code == 42, "Run returns the exit code of Exit()");
    Check(created == 1, "the first window has id 1");
    Check(kept != nullptr, "Resumed was called");
    Check(!app.log.empty() && app.log.front() == "resumed", "Resumed is the first callback");
    Check(!app.log.empty() && app.log.back() == "destroyed", "the window is destroyed when the loop ends");
    bool sawWait = false;
    for (const std::string& entry : app.log) {
        sawWait = sawWait || entry == "about_to_wait";
    }
    Check(sawWait, "AboutToWait was called");
}

void TestScaleAndMonitorConfig()
{
    ScriptedApp app;
    app.onResumed = [](window::Context& ctx) {
        auto id = ctx.CreateWindow(Sized(100.0, 50.0));
        Check(id.has_value(), "CreateWindow");
        if (!id) {
            return;
        }
        auto info = ctx.GetWindowInfo(*id);
        Check(info && info->scaleFactor == 2.0, "the headless scale factor comes from RunConfig");
        // 100 x 50 logical units at scale 2.
        Check(info && info->innerWidth == 200 && info->innerHeight == 100, "logical size times scale is physical");
        (void)ctx.Exit(0);
    };
    window::RunConfig config = Headless();
    config.headlessScaleFactor = 2.0;
    config.headlessMonitorWidth = 1920;
    config.headlessMonitorHeight = 1080;
    auto code = window::Run(app, config);
    Check(code && *code == 0, "Run with a scale factor");
}

void TestErrorsComeBackAsMessages()
{
    ScriptedApp app;
    app.onResumed = [](window::Context& ctx) {
        auto bad = ctx.CreateWindow(Sized(0.0, 0.0));
        Check(!bad.has_value() && !bad.error().empty(), "an unusable description is an error with a message");

        auto unknown = ctx.CloseWindow(99);
        Check(!unknown.has_value() && !unknown.error().empty(), "an unknown window is an error with a message");
        Check(!ctx.GetWindowInfo(99).has_value(), "GetWindowInfo of an unknown window fails");
        Check(!ctx.SetTitle(99, "x").has_value(), "SetTitle of an unknown window fails");
        Check(!ctx.SetImeCursorArea(99, 0, 0, 1, 1).has_value(), "SetImeCursorArea of an unknown window fails");
        (void)ctx.Exit(0);
    };
    auto code = window::Run(app, Headless());
    Check(code && *code == 0, "errors inside callbacks do not stop the loop by themselves");
}

void TestStaleContextIsRefused()
{
    // A context used when no callback is running must be refused, not used. The
    // wrapper does not allow copying a Context, so a second Context is built from
    // the same raw pointer -- exactly the mistake an application could make with
    // the C ABI (keeping the pointer of a callback and using it later).
    ScriptedApp app;
    window::Context* stale = nullptr;
    alignas(window::Context) unsigned char storage[sizeof(window::Context)];
    app.onResumed = [&](window::Context& ctx) {
        void* raw = nullptr;
        static_assert(sizeof(window::Context) == sizeof(void*), "Context wraps exactly the raw pointer");
        std::memcpy(&raw, &ctx, sizeof raw);
        stale = new (storage) window::Context(raw);
    };
    app.onAboutToWait = [&](window::Context& ctx) { (void)ctx.Exit(0); };
    auto code = window::Run(app, Headless());
    Check(code && *code == 0, "the loop ran and exited normally");

    // The loop is gone: the pointer kept from Resumed must be refused.
    Check(stale != nullptr, "Resumed ran");
    if (stale != nullptr) {
        auto result = stale->Exit(7);
        Check(!result.has_value(), "a context used outside a running callback is refused");
        Check(
            !result && result.error().find("running") != std::string::npos,
            "the refusal says the context is not that of a running callback"
        );
        Check(!stale->CreateWindow({}).has_value(), "creating a window with it is refused too");
        stale->~Context();
    }
}

void TestAnExceptionStopsTheLoop()
{
    ScriptedApp app;
    int callbacksAfterThrow = 0;
    app.onResumed = [](window::Context&) { throw std::runtime_error("boom"); };
    app.onAboutToWait = [&](window::Context&) { ++callbacksAfterThrow; };
    auto code = window::Run(app, Headless());
    Check(!code.has_value(), "a callback that throws makes Run fail");
    Check(!code && code.error().find("boom") != std::string::npos, "the message of the exception is returned");
    Check(callbacksAfterThrow == 0, "callbacks after the throwing one are skipped");

    ScriptedApp other;
    other.onResumed = [](window::Context&) { throw 42; };
    auto unknown = window::Run(other, Headless());
    Check(!unknown && unknown.error().find("unknown exception") != std::string::npos,
          "a non-std exception is reported too");
}

void TestHeadlessReportsAnApplicationThatNeverExits()
{
    ScriptedApp app; // no hooks: nothing ever calls Exit()
    auto code = window::Run(app, Headless());
    Check(!code.has_value(), "the headless backend stops a loop that can never be woken");
    Check(!code && !code.error().empty(), "with a message");
}

void TestKeyTablesAndModifiers()
{
    Check(window::KeyName(window::KeyCode::KeyW) == "KeyW", "KeyName(KeyW)");
    Check(window::KeyName(window::KeyCode::ArrowLeft) == "ArrowLeft", "KeyName(ArrowLeft)");
    Check(window::KeyName(window::KeyCode::F24) == "F24", "KeyName(F24)");
    Check(window::KeyName(window::KeyCode::Unidentified) == "Unidentified", "KeyName(Unidentified)");
    Check(window::KeyName(window::KeyCode::Unknown) == "Unknown", "KeyName of an unknown value");
    Check(window::KeyName(static_cast<window::KeyCode>(5000)) == "Unknown", "KeyName of a number outside the table");
    Check(window::NamedKeyName(window::NamedKey::Enter) == "Enter", "NamedKeyName(Enter)");
    Check(window::NamedKeyName(window::NamedKey::Process) == "Process", "NamedKeyName(Process)");
    Check(window::NamedKeyName(window::NamedKey::Unknown) == "Unknown", "NamedKeyName of an unknown value");

    using window::Modifiers;
    const Modifiers held = Modifiers::Shift | Modifiers::Control;
    Check(window::Has(held, Modifiers::Shift), "Has(Shift)");
    Check(window::Has(held, Modifiers::Shift | Modifiers::Control), "Has(Shift|Control)");
    Check(!window::Has(held, Modifiers::Alt), "!Has(Alt)");
    Check(window::Has(held, Modifiers::None), "every set has None");
}

} // namespace

int main()
{
    TestLifecycleAndExitCode();
    TestScaleAndMonitorConfig();
    TestErrorsComeBackAsMessages();
    TestStaleContextIsRefused();
    TestAnExceptionStopsTheLoop();
    TestHeadlessReportsAnApplicationThatNeverExits();
    TestKeyTablesAndModifiers();

    if (g_failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", g_failures);
        return EXIT_FAILURE;
    }
    std::puts("cv_ffi_window_test: all checks passed");
    return EXIT_SUCCESS;
}
