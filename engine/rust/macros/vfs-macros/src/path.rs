use camino::Utf8PathBuf;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr, parse2};
use root::Root;
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
- temp//";

pub(crate) fn resolve(value: &str) -> Result<Utf8PathBuf, &'static str> {
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
        return Err(INVALID_ROOT_MSG);
    };

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
