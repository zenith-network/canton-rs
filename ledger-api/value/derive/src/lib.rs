//! Derive macros for Daml values
//!
//! To use, depend on `canton-types` create with `"derive"` feature enabled.
//!
//! This crate defines [`Value`] macro, which is used to simplify implementation of the
//! `canton::types::Value` trait for Rust types.
//!
//! Can be applied to `struct`-s or `enum`-s.
//!
//! `#[value]` helper attribute can be applied to both item (struct or enum) and members
//! (fields or enum variants). It has the following keys:
//!
//! - `package_id = "..." | EXPR` (**required**) - applied only to structs or enums. Defined package
//!     ID to use in identifier of Ledger API value. Can be a string literal or an expression of
//!     type [`PackageId`][daml_primitives::package_id::PackageId]. If a string literal is passed,
//!     it will be checked at macro runtime to be a valid
//!     [`PackageId`][daml_primitives::package_id::PackageId] and compilation error may be emitted.
//! - `name = "..." | EXPR` - name of the item or member to use on Ledger API value side. This can
//!     be string literal or expression of type
//!     [`NameString`][daml_primitives::name_string::NameString]. If a string literal is passed, it
//!     will be checked at marco runtime to be a valid
//!     [`NameString`][daml_primitives::name_string::NameString] and compilation error may be
//!     emitted. If not set, item/member identifier will be used.
//!
//! TODO: complete docs
//!
//! # Example
//!
//! ```rust,ignore
//! #[derive(Value)]
//! #[value(package_id = "ffffff", name = "MyDataStruct")]
//! struct MyStruct {
//!     #[value(name = "myValue")]
//!     my_value: i64,
//! }
//! ```
//!
//! This will be equivalent to the following Daml code:
//!
//! ```daml
//! data MyDataStruct = MyDataStruct
//!   with
//!     myValue : Int
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{Data, DeriveInput, Error, parse_macro_input};

mod attributes;
mod enum_;
mod paths;
mod struct_;

use attributes::ItemAttributes;

/// Derive macro for `Value` trait. See [crate-level docs](crate).
#[proc_macro_derive(Value, attributes(value))]
pub fn impl_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match try_impl_value(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn try_impl_value(input: DeriveInput) -> Result<TokenStream2, Error> {
    let item_attrs = ItemAttributes::parse(&input.attrs)?;
    let generics = input.generics;

    match input.data {
        Data::Struct(ds) => struct_::try_impl_record(item_attrs, input.ident, ds, generics),
        Data::Enum(de) => enum_::try_impl_value_enum(&item_attrs, input.ident, de, generics),
        Data::Union(_) => Err(Error::new(
            Span::call_site(),
            "LedgerApiValue cannot be applied to union types",
        )),
    }
}

fn collect_err_chain<E: std::error::Error + ?Sized>(error: &E) -> Vec<String> {
    let mut res = vec![error.to_string()];
    if let Some(source) = error.source() {
        res.extend(collect_err_chain(source));
    }
    res
}

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn test_build() {
        let t = TestCases::new();
        t.pass("tests/assets/01-simple-my-type.rs");
        t.compile_fail("tests/assets/02-no-package-id.rs");
        t.compile_fail("tests/assets/03-bad-struct-name.rs");
    }
}
