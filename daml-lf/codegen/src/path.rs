//! Utilities to generate Rust paths (e.g. `foo::bar::baz`)
//!
//! TODO: this may be way more sofisticated
//!
//! Reference: https://doc.rust-lang.org/reference/paths.html

use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;

use crate::ident;

/// Generate a module path
///
/// All segments of the path will be converted to snake_case
///
/// # Example
///
/// `["MyMod", "MySubmod"]` -> `my_mod::my_submod`
pub fn generate_module_path(path: impl IntoIterator<Item = impl AsRef<str>>) -> TokenStream {
    generate_path(path.into_iter().map(|s| s.as_ref().to_snake_case()))
}

/// Generate a path
///
/// All segments are left unmodified (not converted to snake_case or CamelCase)
pub fn generate_path(path: impl IntoIterator<Item = impl AsRef<str>>) -> TokenStream {
    let segments = path
        .into_iter()
        .map(|segment| ident::generate_ident(segment));
    quote! { #(#segments)::* }
}
