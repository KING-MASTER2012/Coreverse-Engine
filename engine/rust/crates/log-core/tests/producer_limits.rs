//! Runtime producer registration is validated and bounded.
//!
//! Own test binary: the dynamic registry is process-global.

use log_core::producer::{lookup, register_dynamic};

#[test]
fn invalid_codes_are_rejected() {
    for bad in [
        "",
        "has space",
        "new\nline",
        "slash/code",
        "ünïcode",
        &"A".repeat(129),
    ] {
        assert!(
            register_dynamic(bad, "Name", "Org").is_err(),
            "{bad:?} must be rejected"
        );
    }
    assert!(register_dynamic("OK-CODE_1", "Name", "Org").is_ok());
    assert_eq!(lookup("OK-CODE_1").unwrap().display_name, "Name");
    assert!(register_dynamic("OK-CODE_1", "Other", "Org").is_err());
    assert!(register_dynamic("OK-LONG", "n".repeat(129), "Org").is_err());
}

#[test]
fn the_number_of_runtime_producers_is_capped() {
    let mut registered = 0;
    let mut rejected = false;
    for i in 0..1100 {
        match register_dynamic(format!("LIMIT-{i}"), "Limit", "Org") {
            Ok(()) => registered += 1,
            Err(_) => {
                rejected = true;
                break;
            }
        }
    }
    assert!(rejected, "an unbounded registry leaks memory forever");
    assert!(registered <= 1024, "registered {registered}");
}
