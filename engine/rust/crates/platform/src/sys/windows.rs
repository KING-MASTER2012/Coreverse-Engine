//! Windows: kernel32 / ntdll through `windows-sys`.

use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
use windows_sys::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GlobalMemoryStatusEx, MEMORYSTATUSEX, OSVERSIONINFOW,
    RelationProcessorCore,
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
