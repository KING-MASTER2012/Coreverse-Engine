use camino::Utf8PathBuf;
use proc_macro2::TokenStream;
use quote::quote;
use root::Root;
use syn::{Error, LitStr, parse2};
const INVALID_ROOT_MSG: &str = "Invalid VFS Root.\n\
Supported Roots:\n\
- assets://\n\
- build://\n\
- cache://\n\
- config://\n\
- logs://\n\
- mods://\n\
- packages://\n\
- source://\n\
- temp://";

pub(crate) fn resolve(value: &str) -> Result<Utf8PathBuf, String> {
    let (root, relative_path) = if let Some(rest) = value.strip_prefix("assets://") {
        (Root::Assets, rest)
    } else if let Some(rest) = value.strip_prefix("build://") {
        (Root::Build, rest)
    } else if let Some(rest) = value.strip_prefix("cache://") {
        (Root::Cache, rest)
    } else if let Some(rest) = value.strip_prefix("config://") {
        (Root::Config, rest)
    } else if let Some(rest) = value.strip_prefix("logs://") {
        (Root::Logs, rest)
    } else if let Some(rest) = value.strip_prefix("mods://") {
        (Root::Mods, rest)
    } else if let Some(rest) = value.strip_prefix("packages://") {
        (Root::Packages, rest)
    } else if let Some(rest) = value.strip_prefix("source://") {
        (Root::Source, rest)
    } else if let Some(rest) = value.strip_prefix("temp://") {
        (Root::Temp, rest)
    } else {
        return Err(INVALID_ROOT_MSG.to_string());
    };
    let relative_path = validate_relative(relative_path)?;
    let relative_path = relative_path.as_str();

    let resolved = match root {
        Root::Assets => format!("assets/{relative_path}"),
        Root::Build => format!("build/{relative_path}"),
        Root::Cache => format!("cache/{relative_path}"),
        Root::Config => format!("config/{relative_path}"),
        Root::Logs => format!("logs/{relative_path}"),
        Root::Mods => format!("mods/{relative_path}"),
        Root::Packages => format!("packages/{relative_path}"),
        Root::Source => format!("source/{relative_path}"),
        Root::Temp => format!("temp/{relative_path}"),
    };

    Ok(Utf8PathBuf::from(resolved))
}

/// Checks the part after `<root>://` and returns it with `\` turned into `/`.
///
/// `file!`/`file_bytes!`/`metadata!` join this onto the crate's directory and
/// read the result at compile time, so a `..` segment (or an absolute path)
/// would embed any file the build machine can read. The rules match
/// `vfs::VfsPath::new`, so a path accepted here is also valid at runtime.
fn validate_relative(relative: &str) -> Result<String, String> {
    let relative = relative.replace('\\', "/");
    let invalid = |why: &str| Err(format!("Invalid VFS path '{relative}': {why}."));

    if relative.contains('\0') {
        return invalid("it contains a NUL byte");
    }
    if relative.starts_with('/') {
        return invalid("it must be relative to its root, not absolute");
    }
    for segment in relative.split('/') {
        if segment == ".." {
            return invalid("'..' would escape the root");
        }
        if segment.contains(':') {
            return invalid("':' (drive letters, alternate data streams) is not allowed");
        }
    }
    Ok(relative)
}

pub fn path_impl(input: TokenStream) -> TokenStream {
    let path: LitStr = match parse2(input) {
        Ok(lit) => lit,
        Err(err) => return err.to_compile_error(),
    };
    let value = path.value();
    let resolved = match resolve(&value) {
        Ok(resolved) => resolved,
        Err(msg) => return Error::new_spanned(path, msg).to_compile_error(),
    };
    let resolved = resolved.as_str();
    quote! { #resolved }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_every_root() {
        assert_eq!(resolve("assets://a/b.txt").unwrap(), "assets/a/b.txt");
        assert_eq!(resolve("temp://x").unwrap(), "temp/x");
        assert_eq!(
            resolve("config://dir\\file.toml").unwrap(),
            "config/dir/file.toml"
        );
    }

    #[test]
    fn rejects_paths_that_leave_the_root() {
        for bad in [
            "assets://../secret",
            "assets://a/../../secret",
            "assets://a\\..\\..\\secret",
            "assets:///etc/passwd",
            "assets://C:/Windows",
            "assets://file.txt:stream",
            "assets://a\0b",
        ] {
            assert!(resolve(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn the_root_error_lists_temp_correctly() {
        let message = resolve("nope://x").unwrap_err();
        assert!(message.contains("- temp://"), "{message}");
        assert!(!message.contains("temp//"), "{message}");
    }
}
