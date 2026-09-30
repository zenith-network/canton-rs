//! Derive macros for the Canton template, interface, and choice marker traits.
//!
//! [`Template`] can be applied to `struct`s and generates:
//!
//! - `impl ::canton::types::TemplateOrInterface`
//! - `impl ::canton::types::Template`
//! - `impl ::canton::types::TemplateWithKey` when `#[template(key = TYPE)]` is set
//! - `impl ::canton::types::Choice<Self>` for the argument type in every
//!   `choice(...)` entry
//! - `impl ::canton::types::Implements<INTERFACE>` for every
//!   `implements = INTERFACE` entry
//!
//! [`Interface`] can be applied to `struct`s, `enum`s, or `union`s and generates:
//!
//! - `impl ::canton::types::TemplateOrInterface`
//! - `impl ::canton::types::Interface` with the view type from
//!   `#[interface(view = TYPE)]`
//! - `impl ::canton::types::Choice<Self>` for the argument type in every
//!   `choice(...)` entry
//! - `impl ::canton::types::Requires<INTERFACE>` for every
//!   `requires = INTERFACE` entry
//!
//! [`Choice`] can be applied to `struct`s or `enum`s and generates:
//!
//! - `impl ::canton::types::Choice<R>`, where `R` is the template or interface on
//!   which the choice is defined
//!
//! The higher-level `ledger_api_types::v2` traits are provided by blanket impls in
//! `ledger-api-types`, not by these macros directly.
//!
//! `#[template(...)]` supports:
//!
//! - `key = TYPE`
//! - `choice(argument = TYPE, result = TYPE, consuming = EXPR, name = "..." | EXPR)`
//!   (repeatable; all four fields are required)
//! - `implements = INTERFACE` (repeatable)
//! - `crate_path = PATH`
//!
//! `#[interface(...)]` supports:
//!
//! - `view = TYPE` (**required**)
//! - `choice(argument = TYPE, result = TYPE, consuming = EXPR, name = "..." | EXPR)`
//!   (repeatable; all four fields are required)
//! - `requires = INTERFACE` (repeatable)
//! - `crate_path = PATH`
//!
//! `#[choice(...)]` supports:
//!
//! - `template = TYPE` or `interface = TYPE` (**a target is required**)
//! - `result = TYPE` (**required**)
//! - `consuming = EXPR` (**required**)
//! - `name = "..." | EXPR`
//! - `crate_path = PATH`
//!
//! `crate_path` defaults to `::canton`. A choice name defaults to the Rust type
//! name.
//!
//! # Example
//!
//! ```rust
//! # use canton_types as types;
//! use canton_types_derive::{Choice, Interface, Template};
//!
//! struct AssetView {
//!     owner: String,
//! }
//!
//! #[derive(Interface)]
//! #[interface(
//! #     crate_path = crate,
//!     view = AssetView,
//! )]
//! struct Asset;
//!
//! struct AccountView {
//!     owner: String,
//! }
//!
//! struct GetOwner;
//!
//! #[derive(Interface)]
//! #[interface(
//! #     crate_path = crate,
//!     view = AccountView,
//!     choice(
//!         argument = GetOwner,
//!         result = String,
//!         consuming = false,
//!         name = "GetOwner",
//!     ),
//!     requires = Asset,
//! )]
//! struct AccountInterface;
//!
//! struct Archive;
//!
//! #[derive(Template)]
//! #[template(
//! #     crate_path = crate,
//!     key = String,
//!     choice(
//!         argument = Archive,
//!         result = (),
//!         consuming = true,
//!         name = "Archive",
//!     ),
//!     implements = AccountInterface,
//! )]
//! struct Account {
//!     owner: String,
//! }
//!
//! #[derive(Choice)]
//! #[choice(
//! #     crate_path = crate,
//!     template = Account,
//!     result = bool,
//!     consuming = false,
//! )]
//! struct IsActive;
//!
//! # fn assert_asset<I>()
//! # where
//! #     I: crate::types::Interface<View = AccountView> + crate::types::Requires<Asset>,
//! # {}
//! # fn assert_account<T>()
//! # where
//! #     T: crate::types::TemplateOrInterface
//! #         + crate::types::Template
//! #         + crate::types::TemplateWithKey<Key = String>
//! #         + crate::types::Implements<AccountInterface>,
//! # {}
//! # fn assert_template_choice<C>()
//! # where
//! #     C: crate::types::Choice<Account, Result = ()>,
//! # {}
//! # fn assert_interface_choice<C>()
//! # where
//! #     C: crate::types::Choice<AccountInterface, Result = String>,
//! # {}
//! # fn assert_derived_choice<C>()
//! # where
//! #     C: crate::types::Choice<Account, Result = bool>,
//! # {}
//! # fn main() {
//! #     assert_asset::<AccountInterface>();
//! #     assert_account::<Account>();
//! #     assert_template_choice::<Archive>();
//! #     assert_interface_choice::<GetOwner>();
//! #     assert_derived_choice::<IsActive>();
//! # }
//! ```

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod choice;
mod choice_attributes;
mod interface;
mod template;

/// Derive [`canton::types::Template`][canton_types::Template].
///
/// `#[template(key = TYPE)]` also generates
/// [`canton::types::TemplateWithKey<TYPE>`][canton_types::TemplateWithKey].
///
/// Each `choice(argument = TYPE, ...)` entry generates
/// [`canton::types::Choice<Self>`][canton_types::Choice] for the argument type.
///
/// Each `implements = INTERFACE` entry generates [
/// `canton::types::Implements<INTERFACE>`][canton_types::Implements].
///
/// See [crate-level docs](crate) for an example.
#[proc_macro_derive(Template, attributes(template))]
pub fn template(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match template::impl_template(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Derive [`canton::types::Interface`][canton_types::Interface].
///
/// `#[interface(view = TYPE)]` sets [`Interface::View`][canton_types::Interface::View].
///
/// Each `choice(argument = TYPE, ...)` entry generates
/// [`canton::types::Choice<Self>`][canton_types::Choice] for the argument type.
///
/// Each `requires = INTERFACE` entry generates
/// [`canton::types::Requires<INTERFACE>`][canton_types::Requires].
///
/// See [crate-level docs](crate) for an example.
#[proc_macro_derive(Interface, attributes(interface))]
pub fn interface(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match interface::impl_interface(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Derive [`canton::types::Choice<R>`][canton_types::Choice].
///
/// `R` is selected with `#[choice(template = TYPE)]` or
/// `#[choice(interface = TYPE)]`.
///
/// See [crate-level docs](crate) for an example.
#[proc_macro_derive(Choice, attributes(choice))]
pub fn choice(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match choice::impl_choice(input) {
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
