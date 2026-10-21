//! macOS and iOS: everything comes from `sysctlbyname`.

use std::ffi::CStr;

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;

/// Reads the sysctl `name` into `out` and returns the number of bytes written.
fn sysctl_raw(name: &CStr, out: &mut [u8]) -> Result<usize> {
    let mut len = out.len();
    // SAFETY: `name` is a valid NUL-terminated string, `out` is writable for
    // `len` bytes and `len` is a valid in/out size; no new value is set
    // (null pointer, length 0).
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            out.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc == 0 {
        Ok(len)
    } else {
        Err(PlatformError::last_os_error("sysctlbyname"))
    }
}

fn sysctl_string(name: &CStr) -> Result<String> {
    let mut buffer = [0u8; 128];
    let len = sysctl_raw(name, &mut buffer)?;
    let text = &buffer[..len.min(buffer.len())];
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    Ok(String::from_utf8_lossy(&text[..end]).into_owned())
}

/// Integer sysctls are 4 or 8 bytes wide depending on the key.
fn sysctl_u64(name: &CStr) -> Result<u64> {
    let mut buffer = [0u8; 8];
    match sysctl_raw(name, &mut buffer)? {
        4 => Ok(u64::from(u32::from_ne_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3],
        ]))),
        8 => Ok(u64::from_ne_bytes(buffer)),
        other => Err(PlatformError::unusable(
            "sysctlbyname",
            format!("unexpected result size of {other} bytes"),
        )),
    }
}

pub(crate) fn os_version() -> Result<OsVersion> {
    Ok(OsVersion {
        name: if cfg!(target_os = "ios") {
            "iOS"
        } else {
            "macOS"
        }
        .to_owned(),
        version: sysctl_string(c"kern.osproductversion")?,
        kernel: sysctl_string(c"kern.osrelease").ok(),
    })
}

pub(crate) fn physical_cores() -> Option<usize> {
    let cores = sysctl_u64(c"hw.physicalcpu").ok()?;
    usize::try_from(cores).ok().filter(|&n| n > 0)
}

pub(crate) fn total_memory_bytes() -> Result<u64> {
    sysctl_u64(c"hw.memsize")
}
