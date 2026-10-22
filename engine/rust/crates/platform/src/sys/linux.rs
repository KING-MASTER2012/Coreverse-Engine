//! Linux and Android: `uname`, `sysconf`, `/etc/os-release` and sysfs.

use std::path::Path;

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;
use crate::parse;
use crate::thread::ThreadPriority;

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

/// Linux limits a thread name to 15 bytes plus the terminating NUL.
const MAX_THREAD_NAME_BYTES: usize = 15;

/// Names the calling thread (`prctl(PR_SET_NAME)`), cutting the name to what
/// the kernel accepts. `prctl` works on every Android API level, unlike
/// `pthread_setname_np`, which bionic only has since API 26.
pub(crate) fn set_thread_name(name: &str) -> Result<()> {
    let name = parse::truncate_utf8(name, MAX_THREAD_NAME_BYTES);
    let name = std::ffi::CString::new(name).map_err(|err| {
        PlatformError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
    })?;
    // SAFETY: PR_SET_NAME reads a NUL-terminated string of at most 16 bytes
    // (including the NUL) at the address in the second argument; `name` is at
    // most 15 bytes plus its NUL and outlives the call. The remaining
    // arguments are unused by this option and passed as zero.
    let rc = unsafe {
        libc::prctl(
            libc::PR_SET_NAME,
            name.as_ptr() as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(PlatformError::last_os_error("prctl"))
    }
}

/// The nice value of an abstract priority. Negative values need privileges.
fn nice_value(priority: ThreadPriority) -> libc::c_int {
    match priority {
        ThreadPriority::Low => 10,
        ThreadPriority::Normal => 0,
        ThreadPriority::High => -5,
        ThreadPriority::TimeCritical => -10,
    }
}

/// Sets the nice value of the calling thread. Linux keeps a nice value per
/// thread (a deviation from POSIX that `setpriority(PRIO_PROCESS, tid, ..)`
/// relies on). Without `CAP_SYS_NICE` or a suitable `RLIMIT_NICE`, lowering
/// the nice value is refused: that includes going back from `Low` to
/// `Normal`.
pub(crate) fn set_thread_priority(priority: ThreadPriority) -> Result<()> {
    // SAFETY: `SYS_gettid` takes no arguments and cannot fail.
    let tid = unsafe { libc::syscall(libc::SYS_gettid) };
    let tid = libc::id_t::try_from(tid)
        .map_err(|_| PlatformError::unusable("gettid", "returned an invalid thread id"))?;
    // SAFETY: all arguments are plain integers.
    let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, tid, nice_value(priority)) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    match err.raw_os_error() {
        // EPERM and EACCES both map to `ErrorKind::PermissionDenied`.
        Some(code) if code == libc::EPERM || code == libc::EACCES => Err(PlatformError::Io(err)),
        _ => Err(PlatformError::last_os_error("setpriority")),
    }
}
