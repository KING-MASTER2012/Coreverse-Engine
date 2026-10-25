//! The operating-system backends.
//!
//! Exactly one backend is compiled; each exports the same system-info and
//! thread functions. `sleep_coarse` is an extra that only Windows needs more than
//! `std::thread::sleep` for, and `known_folder` and `shell_execute` exist on Windows alone. All
//! `unsafe` code of the crate lives in this module tree (see the `allow` on its
//! declaration in `lib.rs`), keeps its unsafe blocks tiny, and documents why
//! each is sound.

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::{
    os_version, physical_cores, set_thread_name, set_thread_priority, total_memory_bytes,
};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use apple::{
    os_version, physical_cores, set_thread_name, set_thread_priority, total_memory_bytes,
};

#[cfg(all(feature = "winit", any(target_os = "macos", target_os = "ios")))]
mod metal;
#[cfg(all(feature = "winit", any(target_os = "macos", target_os = "ios")))]
pub(crate) use metal::metal_layer;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{
    known_folder, os_version, physical_cores, set_thread_name, set_thread_priority, shell_execute,
    sleep_coarse, total_memory_bytes,
};

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
)))]
mod fallback;
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    windows
)))]
pub(crate) use fallback::{
    os_version, physical_cores, set_thread_name, set_thread_priority, total_memory_bytes,
};

/// Sleeps for at least `duration`, with whatever accuracy the OS scheduler
/// gives. Windows overrides this with a high-resolution timer.
#[cfg(not(windows))]
pub(crate) fn sleep_coarse(duration: std::time::Duration) {
    std::thread::sleep(duration);
}
