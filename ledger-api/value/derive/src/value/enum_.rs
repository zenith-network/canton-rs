use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DataEnum, Error, Fields, Generics, Ident};

use crate::{
    Attr,
    value::attributes::{ItemAttributes, MemberAttributes},
};

pub fn try_impl_value_enum(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    de: &DataEnum,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    let is_unit_only = de
        .variants
        .iter()
        .all(|variant| matches!(variant.fields, Fields::Unit));

    if is_unit_only {
        // This is a Enum in Daml LF
        try_impl_value_unit_only_enum(item_attrs, ident, generics, de)
    } else {
        // This is a Variant in Daml LF
        todo!("non-unit enums are not suppoted yet")
    }
}

fn try_impl_value_unit_only_enum(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    generics: &Generics,
    de: &DataEnum,
) -> Result<TokenStream, Error> {
    let types = item_attrs.paths().types();
    let into_value_trait = item_attrs.paths().into_value_trait();
    let try_from_value_trait = item_attrs.paths().try_from_value_trait();
    let value_trait = item_attrs.paths().value_trait();
    let value_v2 = item_attrs.paths().value_v2();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut into_match_arms = Vec::new();
    let mut from_match_arms = Vec::new();
    for variant in &de.variants {
        let variant_ident = &variant.ident;
        let attrs = MemberAttributes::parse(&variant.attrs)?;

        let name = if let Some(attr) = attrs.name() {
            match attr {
                Attr::Fixed { attr, span } => {
                    let span = *span;
                    let name = attr.as_str();
                    quote_spanned!(span=> #name)
                }
                Attr::Expr(expr) => quote! { #expr },
            }
        } else {
            let span = variant_ident.span();
            let ident_str = variant_ident.to_string();
            quote_spanned!(span=> #ident_str)
        };

        let variant_ident = &variant.ident;

        into_match_arms
            .push(quote! { Self::#variant_ident => #types::Name::new_static_unchecked(#name) });
        from_match_arms.push(quote! { #name => Ok(Self::#variant_ident) });
    }
    from_match_arms.push(quote! { t => Err(UnexpectedConstructorName::new(t.to_string()).into()) });

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #into_value_trait for #ident #ty_generics #where_clause {
            fn into_value(self) -> #value_v2::value::Value {
                use #value_v2::HasIdentifier;
                #value_v2::value::Value::Enum(#value_v2::value::Enum {
                    enum_id: Some(<Self as HasIdentifier>::identifier_with_package_id()),
                    constructor: match self {
                        #(#into_match_arms),*
                    },
                })
            }
        }

        #[automatically_derived]
        impl #impl_generics #try_from_value_trait for #ident #ty_generics #where_clause {
            type Error = #value_v2::errors::TryFromEnumError;

            #[allow(unused_imports)]
            fn try_from_value(value: #value_v2::value::Value) -> Result<Self, Self::Error> {
                use #value_v2::HasIdentifier;
                use #value_v2::errors::{UnexpectedIdentifier, UnexpectedConstructorName};

                let enum_ = value.into_enum()?;

                if let Some(record_id) = enum_.enum_id {
                    let expected = <Self as HasIdentifier>::identifier_with_package_id();
                    if record_id != expected {
                        return Err(
                            UnexpectedIdentifier::new(
                                    expected.to_string(),
                                    record_id.to_string(),
                                )
                                .into(),
                        );
                    }
                }

                let constructor = enum_.constructor.as_str();

                match constructor {
                    #(#from_match_arms),*
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics #value_trait for #ident #ty_generics #where_clause {}
    })
}
