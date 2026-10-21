//! Windows: kernel32 / ntdll through `windows-sys`.

use std::time::Duration;

use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GlobalMemoryStatusEx, MEMORYSTATUSEX, OSVERSIONINFOW,
    RelationProcessorCore,
};
use windows_sys::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE, SetWaitableTimer,
    TIMER_ALL_ACCESS, WaitForSingleObject,
};

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;
use crate::parse;

pub(crate) fn os_version() -> Result<OsVersion> {
    // SAFETY: OSVERSIONINFOW is plain old data; all-zero is a valid value.
    let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    info.dwOSVersionInfoSize = u32::try_from(size_of::<OSVERSIONINFOW>())
        .map_err(|e| PlatformError::unusable("RtlGetVersion", e.to_string()))?;
    // SAFETY: `info` is a valid, writable OSVERSIONINFOW whose size field is
    // set. RtlGetVersion (unlike GetVersionExW) is not subject to manifest
    // based version lying and always succeeds with such an argument.
    let status = unsafe { RtlGetVersion(&mut info) };
    if status < 0 {
        return Err(PlatformError::unusable(
            "RtlGetVersion",
            format!("NTSTATUS {status:#010x}"),
        ));
    }
    Ok(OsVersion {
        name: "Windows".to_owned(),
        version: format!(
            "{}.{}.{}",
            info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
        ),
        kernel: None,
    })
}

pub(crate) fn physical_cores() -> Option<usize> {
    let mut len: u32 = 0;
    // SAFETY: a null buffer with length 0 is the documented way to ask for the
    // required size; the call fails with ERROR_INSUFFICIENT_BUFFER and sets `len`.
    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, std::ptr::null_mut(), &mut len)
    };
    if len == 0 {
        return None;
    }
    // u64 elements give the 8-byte alignment the records require.
    let mut buffer = vec![0u64; usize::try_from(len).ok()?.div_ceil(8)];
    // SAFETY: `buffer` is writable for at least `len` bytes (rounded up to a
    // multiple of 8) and suitably aligned; `len` is the size the first call
    // asked for.
    let ok = unsafe {
        GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            buffer.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if ok == 0 {
        return None;
    }
    let bytes: Vec<u8> = buffer
        .iter()
        .flat_map(|word| word.to_ne_bytes())
        .take(usize::try_from(len).ok()?)
        .collect();
    parse::count_windows_core_records(&bytes).filter(|&n| n > 0)
}

pub(crate) fn total_memory_bytes() -> Result<u64> {
    // SAFETY: MEMORYSTATUSEX is plain old data; all-zero is a valid value.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = u32::try_from(size_of::<MEMORYSTATUSEX>())
        .map_err(|e| PlatformError::unusable("GlobalMemoryStatusEx", e.to_string()))?;
    // SAFETY: `status` is a valid, writable MEMORYSTATUSEX whose length field
    // is set, as the API requires.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(PlatformError::last_os_error("GlobalMemoryStatusEx"));
    }
    Ok(status.ullTotalPhys)
}

/// An owned waitable-timer handle.
struct WaitableTimer(HANDLE);

impl Drop for WaitableTimer {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateWaitableTimerExW, is owned by
        // this value alone and is closed exactly once, here.
        unsafe { CloseHandle(self.0) };
    }
}

fn create_high_resolution_timer() -> Option<WaitableTimer> {
    // SAFETY: a null SECURITY_ATTRIBUTES and a null name are both allowed (an
    // unnamed timer with default security); the flag and access rights are
    // the documented constants. The call has no other preconditions.
    let handle = unsafe {
        CreateWaitableTimerExW(
            std::ptr::null(),
            std::ptr::null(),
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
            TIMER_ALL_ACCESS,
        )
    };
    // The high-resolution flag needs Windows 10 1803; older systems fail the
    // call, which is reported as a null handle.
    (!handle.is_null()).then_some(WaitableTimer(handle))
}

thread_local! {
    // One timer per thread: creating a kernel object for every sleep would
    // cost more than the accuracy it buys, and a waitable timer must not be
    // armed from two threads at once.
    static HIGH_RESOLUTION_TIMER: Option<WaitableTimer> = create_high_resolution_timer();
}

/// Sleeps on the high-resolution timer; `false` when it is unavailable or
/// the wait failed, in which case the caller falls back.
fn wait_high_resolution(duration: Duration) -> bool {
    HIGH_RESOLUTION_TIMER.with(|slot| {
        let Some(timer) = slot else {
            return false;
        };
        // A negative due time is relative, in 100 ns units.
        let ticks = i64::try_from(duration.as_nanos().div_ceil(100))
            .unwrap_or(i64::MAX)
            .max(1);
        let due = -ticks;
        // SAFETY: `timer.0` is a valid timer handle owned by this thread's
        // slot; `due` is a valid i64 for the whole call; a zero period makes
        // the timer one-shot, and the null completion routine/argument pair
        // is allowed.
        let armed = unsafe { SetWaitableTimer(timer.0, &due, 0, None, std::ptr::null(), 0) } != 0;
        if !armed {
            return false;
        }
        // SAFETY: `timer.0` is a valid handle that stays open for the call.
        unsafe { WaitForSingleObject(timer.0, INFINITE) == WAIT_OBJECT_0 }
    })
}

/// Sleeps for at least `duration`: on the high-resolution waitable timer
/// when the system has one (about 0.5 ms accuracy), else on
/// `std::thread::sleep` (about 15 ms).
pub(crate) fn sleep_coarse(duration: Duration) {
    if duration.is_zero() || wait_high_resolution(duration) {
        return;
    }
    std::thread::sleep(duration);
}
