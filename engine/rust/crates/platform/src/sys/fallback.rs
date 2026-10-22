//! Platforms without a backend: everything reports `Unsupported`.

use crate::error::{PlatformError, Result};
use crate::info::OsVersion;
use crate::thread::ThreadPriority;

pub(crate) fn os_version() -> Result<OsVersion> {
    Err(PlatformError::unsupported("OS version"))
}

pub(crate) fn physical_cores() -> Option<usize> {
    None
}

pub(crate) fn total_memory_bytes() -> Result<u64> {
    Err(PlatformError::unsupported("total memory"))
}

pub(crate) fn set_thread_name(_name: &str) -> Result<()> {
    Err(PlatformError::unsupported("thread names"))
}

pub(crate) fn set_thread_priority(_priority: ThreadPriority) -> Result<()> {
    Err(PlatformError::unsupported("thread priorities"))
}
