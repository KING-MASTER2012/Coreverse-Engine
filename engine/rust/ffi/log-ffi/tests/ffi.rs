//! Log system FFI test
use std::ffi::CString;

use log_ffi::*;

#[test]
fn init_and_shutdown() {
    unsafe {
        let root = CString::new(".").unwrap();

        let logger = cv_log_init(root.as_ptr());

        assert!(!logger.is_null());

        assert_eq!(cv_log_shutdown(logger), 0);
    }
}

#[test]
fn shutdown_null_returns_error() {
    unsafe {
        assert_eq!(cv_log_shutdown(std::ptr::null_mut()), -1);
    }
}

#[test]
fn set_min_severity_null_handle() {
    unsafe {
        assert_eq!(cv_log_set_min_severity(std::ptr::null_mut(), 0), -1);
    }
}

#[test]
fn invalid_severity_returns_error() {
    unsafe {
        let root = CString::new(".").unwrap();

        let logger = cv_log_init(root.as_ptr());

        assert_eq!(cv_log_set_min_severity(logger, 100), -3);

        cv_log_shutdown(logger);
    }
}

#[test]
fn format_json_null_returns_null() {
    unsafe {
        let ptr = cv_log_format_json(std::ptr::null(), 0);

        assert!(ptr.is_null());
    }
}

#[test]
fn free_null_is_safe() {
    unsafe {
        cv_log_free_string(std::ptr::null_mut());
    }
}
