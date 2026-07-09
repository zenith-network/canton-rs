use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Error, Fields, Generics, Ident};

use crate::ItemAttributes;

pub fn try_impl_value_enum(
    item_attrs: &ItemAttributes,
    ident: Ident,
    de: DataEnum,
    generics: Generics,
) -> Result<TokenStream, Error> {
    let is_unit_only = de
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit));

    if is_unit_only {
        // This is a Enum in Daml LF
        try_impl_value_unit_only_enum(item_attrs, ident, de)
    } else {
        // This is a Variant in Daml LF
        todo!("non-unit enums are not suppoted yet")
    }
}

fn try_impl_value_unit_only_enum(
    item_attrs: &ItemAttributes,
    ident: Ident,
    de: DataEnum,
) -> Result<TokenStream, Error> {
    let into_value_trait = &item_attrs.paths().into_value_trait;
    let try_from_value_trait = &item_attrs.paths().try_from_value_trait;
    let value_trait = &item_attrs.paths().value_trait;
    let value_v2 = &item_attrs.paths().value_v2;

    let mut match_arms = Vec::new();
    for _variant in de.variants {
        // let Variant {
        //     attrs,
        //     ident,
        //     fields,
        //     discriminant,
        // } = variant;

        // let name = attrs.iter().find(|attr| attr.path());

        match_arms.push(quote! { Self::#ident => todo!() })
    }

    Ok(quote! {
        impl #into_value_trait for #ident {
            fn into_value(self) -> #value_v2::value::Value {
                // let constructor = #path::types::primitives::name_string::Name::new_unchecked("");
                #value_v2::value::Value::Enum(#value_v2::value::Enum {
                    enum_id: None,
                    constructor: match self {
                        #(#match_arms),*
                    },
                })
            }
        }

        impl #try_from_value_trait for #ident {
            type Error = #value_v2::errors::ValueKindError;

            fn try_from_value(value: #value_v2::value::Value) -> Result<Self, Self::Error> {
                todo!()
            }
        }

        impl #value_trait for #ident {}
    })
}
