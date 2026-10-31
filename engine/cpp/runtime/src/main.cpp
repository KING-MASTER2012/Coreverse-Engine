// coreverse-runtime: opens a window through the Rust platform crate, runs the
// event loop through cv-ffi and presents with the renderer. See
// docs/src/runtime.md.

#include <cstdio>
#include <cstdlib>
#include <string>
#include <string_view>

#include "RuntimeApp.hpp"
#include "cv-ffi/Window.hpp"

namespace {

void PrintUsage(std::FILE* out)
{
    std::fputs(
        "usage: coreverse-runtime [options]\n"
        "  --headless       no window system: windows exist only as state (no display needed)\n"
        "  --frames N       exit after N frames (default: run until the window is closed)\n"
        "  --trace          print every window event\n"
        "  --no-renderer    open the window and run the loop without a device or swapchain\n"
        "  --title TEXT     window title\n"
        "  --help           this text\n"
        "Escape or closing the window exits; F11 toggles borderless fullscreen.\n",
        out
    );
}

} // namespace

int main(int argc, char** argv)
{
    runtime::Options options;
    for (int i = 1; i < argc; ++i) {
        const std::string_view arg = argv[i];
        if (arg == "--headless") {
            options.headless = true;
        } else if (arg == "--trace") {
            options.trace = true;
        } else if (arg == "--no-renderer") {
            options.useRenderer = false;
        } else if (arg == "--frames" && i + 1 < argc) {
            char* end = nullptr;
            const unsigned long long frames = std::strtoull(argv[++i], &end, 10);
            if (end == nullptr || *end != '\0') {
                std::fprintf(stderr, "coreverse-runtime: --frames needs a number\n");
                return 1;
            }
            options.maxFrames = frames;
        } else if (arg == "--title" && i + 1 < argc) {
            options.title = argv[++i];
        } else if (arg == "--help" || arg == "-h") {
            PrintUsage(stdout);
            return 0;
        } else {
            std::fprintf(stderr, "coreverse-runtime: unknown or incomplete option '%s'\n", argv[i]);
            PrintUsage(stderr);
            return 1;
        }
    }

    runtime::RuntimeApp app(options);
    cv_ffi::window::RunConfig config;
    config.backend = options.headless ? cv_ffi::window::Backend::Headless : cv_ffi::window::Backend::Desktop;

    const auto result = cv_ffi::window::Run(app, config);
    if (!result) {
        std::fprintf(stderr, "coreverse-runtime: %s\n", result.error().c_str());
        return 1;
    }
    std::printf("coreverse-runtime: %llu frame(s), exit code %d\n",
                static_cast<unsigned long long>(app.FramesRendered()), *result);
    return *result;
}
