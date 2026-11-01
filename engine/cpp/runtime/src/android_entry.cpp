// The Android entry point of coreverse-runtime.
//
// An Android application has no main(): the NativeActivity starts a thread and
// the Rust `android_main` of the ffi crate (engine/rust/crates/ffi/src/android.rs)
// runs on it, keeps the AndroidApp that the window backend needs, and calls the
// function below. From here on it is the same program as on the desktop:
// cv_ffi::window::Run() starts the event loop on this thread.
//
// There is no console on Android, so errors go to logcat.

#include <cstdio>
#include <exception>
#include <string>

#include "RuntimeApp.hpp"
#include "cv-ffi/Window.hpp"

#if defined(__ANDROID__)
    #include <android/log.h>
#endif

namespace {

    void Log(const std::string& message)
    {
#if defined(__ANDROID__)
        __android_log_print(ANDROID_LOG_ERROR, "coreverse", "%s", message.c_str());
#else
        std::fprintf(stderr, "coreverse: %s\n", message.c_str());
#endif
    }

} // namespace

/// Called by `android_main` of the ffi crate, on the thread the activity library
/// created for it. The return value is only logged: the activity ends when this
/// returns. No exception may escape.
extern "C" int cv_android_main()
{
    try {
        runtime::RuntimeApp app{runtime::Options{}};
        const auto result = cv_ffi::window::Run(app);
        if (!result) {
            Log(result.error());
            return 1;
        }
        return *result;
    } catch (const std::exception& error) {
        Log(std::string("uncaught exception: ") + error.what());
    } catch (...) {
        Log("uncaught unknown exception");
    }
    return 1;
}
