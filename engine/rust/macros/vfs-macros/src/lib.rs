//! Compile-time helpers for `vfs://`-style root-relative path literals
//! (`assets://`, `build://`, ... — see [`path::resolve`]'s `INVALID_ROOT_MSG`
//! for the full list). Each macro resolves its string literal against
//! `CARGO_MANIFEST_DIR` at compile time, so a typo'd root or a missing file
//! is a build error instead of a runtime one.

use proc_macro::TokenStream;

mod file;
mod metadata;
mod path;

/// Validates a `root://relative/path` literal at compile time and expands
/// to the resolved [`camino::Utf8PathBuf`] as a string constant. Does not
/// touch the filesystem — use [`file!`] or [`file_bytes!`] to also assert
/// the file exists.
#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    path::path_impl(input.into()).into()
}

/// Like [`path!`], but also asserts the file exists at compile time and
/// expands to its contents read as a UTF-8 string (`include_str!`-style).
#[proc_macro]
pub fn file(input: TokenStream) -> TokenStream {
    file::file_impl(input.into()).into()
}

/// Like [`file!`], but expands to the file's raw bytes
/// (`include_bytes!`-style) instead of requiring valid UTF-8.
#[proc_macro]
pub fn file_bytes(input: TokenStream) -> TokenStream {
    file::file_bytes_impl(input.into()).into()
}

/// Like [`path!`], but also asserts the file exists at compile time and
/// expands to its size and last-modified time, captured at build time.
#[proc_macro]
pub fn metadata(input: TokenStream) -> TokenStream {
    metadata::metadata_impl(input.into()).into()
}
