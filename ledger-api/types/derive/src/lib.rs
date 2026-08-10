//! Derive macros for the Ledger API template and choice marker traits.
//!
//! These macros are usually consumed through `ledger-api-types` with the `"derive"`
//! feature, or through `canton` with `"derive"` and `"ledger-api"` enabled.
//!
//! [`Template`] can be applied to `struct`s and generates:
//!
//! - `impl ::canton::types::Template`
//! - `impl ::canton::types::TemplateWithKey` when `#[template(key = TYPE)]` is set
//!
//! [`Choice`] can be applied to `struct`s or `enum`s and generates:
//!
//! - `impl ::canton::types::Choice<T>`
//!
//! The higher-level `ledger_api_types::v2` traits are provided by blanket impls in
//! `ledger-api-types`, not by these macros directly.
//!
//! `#[template(...)]` supports:
//!
//! - `key = TYPE`
//! - `crate_path = PATH`
//!
//! `#[choice(...)]` supports:
//!
//! - `template = TYPE` (**required**)
//! - `result = TYPE` (**required**)
//! - `consuming = EXPR` (**required**)
//! - `name = "..." | EXPR`
//! - `crate_path = PATH`
//!
//! # Example
//!
//! ```rust
//! # use canton_types as types;
//! use ledger_api_types_derive::{Choice, Template};
//!
//! #[derive(Template)]
//! #[template(
//! #     crate_path = crate,
//!     key = String,
//! )]
//! struct Account {
//!     owner: String,
//! }
//!
//! #[derive(Choice)]
//! #[choice(
//! #     crate_path = crate,
//!     template = Account,
//!     result = (),
//!     consuming = true,
//! )]
//! struct Archive;
//!
//! # fn assert_template<T: crate::types::Template>() {}
//! # fn assert_template_with_key<T: crate::types::TemplateWithKey<Key = String>>() {}
//! # fn assert_choice<C>()
//! # where
//! #     C: crate::types::Choice<Account, Result = ()>,
//! # {}
//! # fn main() {
//! #     assert_template::<Account>();
//! #     assert_template_with_key::<Account>();
//! #     assert_choice::<Archive>();
//! # }
//! ```

use proc_macro::TokenStream;
mod choice;
mod template;
use syn::{DeriveInput, parse_macro_input};

/// Derive `canton::types::Template`.
///
/// `#[template(key = TYPE)]` also generates
/// `canton::types::TemplateWithKey<Key = TYPE>`.
///
/// See [crate-level docs](crate) for supported attributes and an example.
#[proc_macro_derive(Template, attributes(template))]
pub fn template(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match template::impl_template_value(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Derive `canton::types::Choice<T>`.
///
/// See [crate-level docs](crate) for supported attributes and an example.
#[proc_macro_derive(Choice, attributes(choice))]
pub fn choice(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match choice::impl_choice_value(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn collect_err_chain<E: std::error::Error + ?Sized>(error: &E) -> Vec<String> {
    let mut res = vec![error.to_string()];
    if let Some(source) = error.source() {
        res.extend(collect_err_chain(source));
    }
    res
}
