//! Derive macros for Ledger API value traits.
//!
//! These macros are usually consumed through `ledger-api-value` with the `"derive"`
//! feature, or through `canton` with `"derive"` and `"ledger-api"` enabled.
//!
//! This crate defines:
//!
//! - [`HasIdentifier`] for static Ledger API identifiers
//! - [`Value`] for record-based value conversions
//!
//! `#[identifier(...)]` supports:
//!
//! - `package_id = "..." | EXPR` (**required**)
//! - `package_name = "..." | EXPR` (**required**)
//! - `module = "..." | EXPR` (**required**)
//! - `name = "..." | EXPR`
//! - `crate_path = PATH`
//!
//! `#[value(...)]` currently supports only:
//!
//! - `crate_path = PATH`
//!
//! `#[name = "..." | EXPR]` can be used on struct fields to override the Daml field
//! name.
//!
//! `Value` requires the type to implement `HasIdentifier` and is currently intended
//! for named-field structs.
//!
//! # Example
//!
//! ```rust
//! use canton::ledger_api::types::value::v2::{HasIdentifier, Value};
//!
//! #[derive(HasIdentifier, Value)]
//! #[identifier(package_id = "ffff", package_name = "my-pack", module = "My.Module")]
//! struct MyType {
//!     value: i64,
//!     #[name = "otherValue"]
//!     other_value: String,
//! }
//!
//! # fn assert_identifier<T: canton::ledger_api::types::value::v2::HasIdentifier>() {}
//! # fn assert_value<T: canton::ledger_api::types::value::v2::Value>() {}
//! # fn main() {
//! #     assert_identifier::<MyType>();
//! #     assert_value::<MyType>();
//! # }
//! ```

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Expr, parse_macro_input};

mod has_identifier;
mod value;

/// Derive `ledger_api_value::v2::HasIdentifier`.
///
/// Required item attribute:
/// `#[identifier(package_id = ..., package_name = ..., module = ...)]`.
///
/// `name` and `crate_path` are optional.
///
/// See [crate-level docs](crate) for an example.
#[proc_macro_derive(HasIdentifier, attributes(identifier))]
pub fn has_identifier(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match has_identifier::impl_has_identifier(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Derive the record-based value traits from `ledger_api_value::v2`.
///
/// This macro requires the type to implement `HasIdentifier`.
/// `#[name = ...]` may be used on fields to override the Daml field name.
///
/// See [crate-level docs](crate) for supported attributes and an example.
#[proc_macro_derive(Value, attributes(name, value))]
pub fn value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match value::impl_value(input) {
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

enum Attr<T> {
    Fixed { attr: T, span: Span },
    Expr(Expr),
}

impl<T> Attr<T> {
    pub fn fixed(attr: T, span: Span) -> Self {
        Self::Fixed { attr, span }
    }

    pub fn expr(expr: Expr) -> Self {
        Self::Expr(expr)
    }
}

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn test_build() {
        let t = TestCases::new();
        t.pass("tests/assets/01-simple-my-type.rs");
    }
}
