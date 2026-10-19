//! Linux and Android: `uname`, `sysconf`, `/etc/os-release` and sysfs.

use std::path::Path;

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;
use crate::parse;

/// Kernel release (`uname -r`).
fn kernel_release() -> Result<String> {
    // SAFETY: `utsname` is a plain array-of-integers struct; all-zero bytes
    // are a valid value for it.
    let mut uts: libc::utsname = unsafe { std::mem::zeroed() };
    // SAFETY: `uts` is a valid, writable `utsname` for the whole call.
    if unsafe { libc::uname(&mut uts) } != 0 {
        return Err(PlatformError::last_os_error("uname"));
    }
    // `c_char` is `i8` or `u8` depending on the target; `to_ne_bytes` is the
    // cast-free way to get the byte out of either.
    let bytes: Vec<u8> = uts
        .release
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.to_ne_bytes()[0])
        .collect();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(target_os = "linux")]
pub(crate) fn os_version() -> Result<OsVersion> {
    let kernel = kernel_release()?;
    let release = ["/etc/os-release", "/usr/lib/os-release"]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| parse::parse_os_release(&text));
    Ok(match release {
        Some(release) => OsVersion {
            name: release.name,
            version: release.version_id.unwrap_or_else(|| kernel.clone()),
            kernel: Some(kernel),
        },
        None => OsVersion {
            name: "Linux".to_owned(),
            version: kernel.clone(),
            kernel: Some(kernel),
        },
    })
}

/// `PROP_VALUE_MAX` of bionic's `<sys/system_properties.h>`.
#[cfg(target_os = "android")]
const PROP_VALUE_MAX: usize = 92;

#[cfg(target_os = "android")]
fn android_release() -> Option<String> {
    let mut value = [0; PROP_VALUE_MAX];
    // SAFETY: the name is a valid NUL-terminated string and `value` is a
    // writable buffer of PROP_VALUE_MAX bytes, which is the size the API
    // documents as the maximum it writes (including the NUL).
    let len = unsafe {
        libc::__system_property_get(c"ro.build.version.release".as_ptr(), value.as_mut_ptr())
    };
    if len <= 0 {
        return None;
    }
    let bytes: Vec<u8> = value
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.to_ne_bytes()[0])
        .collect();
    let release = String::from_utf8_lossy(&bytes).trim().to_owned();
    (!release.is_empty()).then_some(release)
}

#[cfg(target_os = "android")]
pub(crate) fn os_version() -> Result<OsVersion> {
    let kernel = kernel_release()?;
    Ok(OsVersion {
        name: "Android".to_owned(),
        version: android_release().unwrap_or_else(|| kernel.clone()),
        kernel: Some(kernel),
    })
}

pub(crate) fn physical_cores() -> Option<usize> {
    parse::physical_cores_from_sysfs(Path::new("/sys/devices/system/cpu"))
}

pub(crate) fn total_memory_bytes() -> Result<u64> {
    // SAFETY: `sysconf` only reads its integer argument.
    let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
    // SAFETY: as above.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if pages <= 0 || page_size <= 0 {
        return Err(PlatformError::last_os_error("sysconf"));
    }
    let pages =
        u64::try_from(pages).map_err(|e| PlatformError::unusable("sysconf", e.to_string()))?;
    let page_size =
        u64::try_from(page_size).map_err(|e| PlatformError::unusable("sysconf", e.to_string()))?;
    pages
        .checked_mul(page_size)
        .ok_or_else(|| PlatformError::unusable("sysconf", "total memory overflows u64"))
}
