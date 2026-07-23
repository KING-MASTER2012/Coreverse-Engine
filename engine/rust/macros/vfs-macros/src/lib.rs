use proc_macro::TokenStream;

mod file;
mod path;
mod metadata;

#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    path::path_impl(input.into()).into()
}

#[proc_macro]
pub fn file(input: TokenStream) -> TokenStream {
    file::file_impl(input.into()).into()
}

#[proc_macro]
pub fn file_bytes(input: TokenStream) -> TokenStream {
    file::file_bytes_impl(input.into()).into()
}
 
#[proc_macro]
pub fn metadata(input: TokenStream) -> TokenStream {
    metadata::metadata_impl(input.into()).into()
}