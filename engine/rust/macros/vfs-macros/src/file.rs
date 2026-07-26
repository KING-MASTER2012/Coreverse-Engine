use std::env;

use camino::Utf8PathBuf;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr, parse2};

use crate::path::resolve;

pub fn resolve_full_path(input: TokenStream) -> Result<Utf8PathBuf, TokenStream> {
    let path: LitStr = match parse2(input) {
        Ok(lit) => lit,
        Err(err) => return Err(err.to_compile_error()),
    };
    let value = path.value();
    let relative = match resolve(&value) {
        Ok(resolved) => resolved,
        Err(msg) => return Err(Error::new_spanned(&path, msg).to_compile_error()),
    };
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return Err(Error::new_spanned(
                &path,
                "CARGO_MANIFEST_DIR is not set; cannot resolve VFS path at compile time.",
            )
            .to_compile_error());
        }
    };
    let full_path = Utf8PathBuf::from(manifest_dir).join(&relative);
    Ok(full_path)
}

pub fn file_impl(input: TokenStream) -> TokenStream {
    let full_path = match resolve_full_path(input) {
        Ok(full_path) => full_path,
        Err(err) => return err,
    };
    let full_path = full_path.as_str();
    quote! { include_str!(#full_path) }
}

pub fn file_bytes_impl(input: TokenStream) -> TokenStream {
    let full_path = match resolve_full_path(input) {
        Ok(full_path) => full_path,
        Err(err) => return err,
    };
    let full_path = full_path.as_str();
    quote! { include_bytes!(#full_path) }
}
