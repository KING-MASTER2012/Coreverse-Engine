use std::env;
use std::time::UNIX_EPOCH;

use camino::Utf8PathBuf;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr, parse2};

use crate::path::resolve;

pub fn metadata_impl(input: TokenStream) -> TokenStream {
    let path: LitStr = match parse2(input) {
        Ok(lit) => lit,
        Err(err) => return err.to_compile_error(),
    };
    let value = path.value();
    let relative = match resolve(&value) {
        Ok(resolved) => resolved,
        Err(msg) => return Error::new_spanned(&path, msg).to_compile_error(),
    };
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return Error::new_spanned(
                &path,
                "CARGO_MANIFEST_DIR is not set; cannot resolve VFS path at compile time.",
            )
            .to_compile_error();
        }
    };
    let full_path: Utf8PathBuf = Utf8PathBuf::from(manifest_dir).join(&relative);
    let meta = match std::fs::metadata(&full_path) {
        Ok(meta) => meta,
        Err(err) => {
            return Error::new_spanned(
                &path,
                format!("Failed to read metadata for '{full_path}': {err}"),
            )
            .to_compile_error();
        }
    };
    let size = meta.len();
    let modified_unix: u64 = match meta.modified() {
        Ok(time) => time
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        Err(_) => {
            // Platform doesn't support mtime; fall back to 0 rather than
            // failing the whole build over a non-essential field.
            0
        }
    };
    let readonly = meta.permissions().readonly();

    quote! {
        {
            #[derive(Debug, Clone, Copy)]
            #[allow(dead_code)]
            struct VfsFileMetadata {
                pub size: u64,
                pub modified_unix: u64,
                pub readonly: bool,
            }

            VfsFileMetadata {
                size: #size,
                modified_unix: #modified_unix,
                readonly: #readonly,
            }
        }
    }
}
