//! macOS and iOS: everything comes from `sysctlbyname`.

use std::ffi::CStr;

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;
use crate::thread::ThreadPriority;

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

/// Darwin accepts thread names up to 63 bytes.
const MAX_THREAD_NAME_BYTES: usize = 63;

/// Names the calling thread with `pthread_setname_np`.
pub(crate) fn set_thread_name(name: &str) -> Result<()> {
    let name = crate::parse::truncate_utf8(name, MAX_THREAD_NAME_BYTES);
    let name = std::ffi::CString::new(name).map_err(|err| {
        PlatformError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
    })?;
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call;
    // the Darwin variant of the function names the calling thread.
    let rc = unsafe { libc::pthread_setname_np(name.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(PlatformError::from_errno("pthread_setname_np", rc))
    }
}

// The libc crate does not export the QoS API, so it is declared here. The
// declaration matches `<pthread/qos.h>`: `qos_class_t` is an `unsigned int`.
unsafe extern "C" {
    fn pthread_set_qos_class_self_np(
        qos_class: libc::c_uint,
        relative_priority: libc::c_int,
    ) -> libc::c_int;
}

// The `QOS_CLASS_*` values of `<sys/qos.h>`.
const QOS_CLASS_USER_INTERACTIVE: libc::c_uint = 0x21;
const QOS_CLASS_USER_INITIATED: libc::c_uint = 0x19;
const QOS_CLASS_DEFAULT: libc::c_uint = 0x15;
const QOS_CLASS_UTILITY: libc::c_uint = 0x11;

/// Maps the abstract priority to a Quality-of-Service class and applies it to
/// the calling thread.
pub(crate) fn set_thread_priority(priority: ThreadPriority) -> Result<()> {
    let class = match priority {
        ThreadPriority::Low => QOS_CLASS_UTILITY,
        ThreadPriority::Normal => QOS_CLASS_DEFAULT,
        ThreadPriority::High => QOS_CLASS_USER_INITIATED,
        ThreadPriority::TimeCritical => QOS_CLASS_USER_INTERACTIVE,
    };
    // SAFETY: `class` is one of the documented QoS classes and 0 is the
    // neutral relative priority; the function only changes the scheduling of
    // the calling thread and takes no pointers.
    let rc = unsafe { pthread_set_qos_class_self_np(class, 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(PlatformError::from_errno(
            "pthread_set_qos_class_self_np",
            rc,
        ))
    }
}
