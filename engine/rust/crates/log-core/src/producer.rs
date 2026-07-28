use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Static description of a diagnostic **producer** - the actual tool that
/// generated the diagnostic (compiler, linter, IDE inspection engine, engine
/// subsystem, ...). The producer is never a programming language; language is
/// metadata on the `Diagnostic` itself (see `Diagnostic::language`).
///
/// Code layout: `<Organization>-<Product>`, e.g. `"LLVM-CL"`, `"JB-RR"`,
/// `"CV-ENGINE"`. Organization-less standalone tools may use a single token
/// (`"GCC"`, `"LUA"`, `"PG"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProducerInfo {
    /// Canonical, unique code used as the first segment of every diagnostic code.
    pub code: &'static str,
    /// Human readable name shown in the `[Source]` slot of the console format.
    pub display_name: &'static str,
    /// Organization / ecosystem this producer belongs to.
    pub organization: &'static str,
}

inventory::collect!(ProducerInfo);

/// Registers a producer known at compile time (Rust code, engine subsystems).
///
/// ```ignore
/// log_core::register_producer!("CV-ENGINE", "Coreverse Engine", "Coreverse");
/// ```
#[macro_export]
macro_rules! register_producer {
    ($code:expr, $display:expr, $org:expr) => {
        $crate::inventory::submit! {
            $crate::producer::ProducerInfo {
                code: $code,
                display_name: $display,
                organization: $org,
            }
        }
    };
}

fn static_registry() -> &'static HashMap<&'static str, ProducerInfo> {
    static REG: OnceLock<HashMap<&'static str, ProducerInfo>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut map = HashMap::new();
        for info in inventory::iter::<ProducerInfo> {
            if let Some(existing) = map.insert(info.code, *info) {
                panic!(
                    "Duplicate producer code '{}' registered by both '{}' and '{}'. \
                     Producer codes must be globally unique - pick a more specific \
                     <Organization>-<Product> pair.",
                    info.code, existing.display_name, info.display_name
                );
            }
        }
        map
    })
}

/// Runtime-registered producers, for foreign-language callers (scripting
/// languages, dynamically loaded plugins) that can't use the compile-time
/// `inventory` macro.
fn dynamic_registry() -> &'static RwLock<HashMap<String, ProducerInfo>> {
    static REG: OnceLock<RwLock<HashMap<String, ProducerInfo>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Registers a producer at runtime. Returns `Err` if the code collides with
/// an existing static or dynamic registration.
pub fn register_dynamic(
    code: impl Into<String>,
    display_name: impl Into<String>,
    organization: impl Into<String>,
) -> Result<(), String> {
    let code = code.into();
    if static_registry().contains_key(code.as_str()) {
        return Err(format!(
            "Producer code '{code}' is already statically registered"
        ));
    }
    let mut reg = dynamic_registry().write().unwrap();
    if reg.contains_key(&code) {
        return Err(format!("Producer code '{code}' is already registered"));
    }
    // Leak the strings so we can hand back a `&'static ProducerInfo`-shaped
    // value consistent with the static path. Producers are registered once
    // per process lifetime, so this is bounded and acceptable.
    let code_static: &'static str = Box::leak(code.clone().into_boxed_str());
    let display_static: &'static str = Box::leak(display_name.into().into_boxed_str());
    let org_static: &'static str = Box::leak(organization.into().into_boxed_str());
    reg.insert(
        code,
        ProducerInfo {
            code: code_static,
            display_name: display_static,
            organization: org_static,
        },
    );
    Ok(())
}
/// Function what looks up into log
pub fn lookup(code: &str) -> Option<ProducerInfo> {
    if let Some(info) = static_registry().get(code) {
        return Some(*info);
    }
    dynamic_registry().read().unwrap().get(code).copied()
}
/// Function what returns all
pub fn all() -> Vec<ProducerInfo> {
    let mut out: Vec<ProducerInfo> = static_registry().values().copied().collect();
    out.extend(dynamic_registry().read().unwrap().values().copied());
    out
}
