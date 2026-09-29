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
        try_impl_value_variant(item_attrs, ident, generics, de)
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

/// A Daml LF variant: every constructor carries exactly one payload value (the
/// codegen emits `()` for a constructor without data).
fn try_impl_value_variant(
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
    // Each payload that mentions a type parameter must itself convert (as the record
    // derive requires per field). Payloads without one are left out, so a recursive
    // variant (`AnyValue` through `Vec<AnyValue>`) adds no cyclic bound.
    let type_params: Vec<String> = generics
        .type_params()
        .map(|p| p.ident.to_string())
        .collect();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    for variant in &de.variants {
        if let Fields::Unnamed(fields) = &variant.fields {
            for field in &fields.unnamed {
                let ty = &field.ty;
                let mentions_param = quote!(#ty)
                    .to_string()
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| type_params.iter().any(|param| param == word));
                if mentions_param {
                    where_clause
                        .predicates
                        .push(syn::parse_quote!(#ty: #try_from_value_trait + #into_value_trait));
                }
            }
        }
    }
    let where_clause = &where_clause;

    let mut into_match_arms = Vec::new();
    let mut from_match_arms = Vec::new();
    for variant in &de.variants {
        let variant_ident = &variant.ident;
        let payload_ty = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => &fields.unnamed[0].ty,
            _ => {
                return Err(Error::new_spanned(
                    variant,
                    "a Daml variant constructor must carry exactly one unnamed payload \
                     (use `()` for a constructor without data)",
                ));
            }
        };
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

        into_match_arms.push(quote! {
            Self::#variant_ident(payload) => (
                #types::Name::new_static_unchecked(#name),
                #into_value_trait::into_value(payload),
            )
        });
        from_match_arms.push(quote! {
            #name => Ok(Self::#variant_ident(
                <#payload_ty as #try_from_value_trait>::try_from_value(variant.value)
                    .map_err(|e| TryFromVariantError::PayloadError(Box::new(e)))?,
            ))
        });
    }
    from_match_arms.push(quote! { t => Err(UnexpectedConstructorName::new(t.to_string()).into()) });

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #into_value_trait for #ident #ty_generics #where_clause {
            fn into_value(self) -> #value_v2::value::Value {
                use #value_v2::HasIdentifier;
                let (constructor, value) = match self {
                    #(#into_match_arms),*
                };
                #value_v2::value::Value::Variant(Box::new(#value_v2::value::Variant {
                    variant_id: Some(<Self as HasIdentifier>::identifier_with_package_id()),
                    constructor,
                    value,
                }))
            }
        }

        #[automatically_derived]
        impl #impl_generics #try_from_value_trait for #ident #ty_generics #where_clause {
            type Error = #value_v2::errors::TryFromVariantError;

            #[allow(unused_imports)]
            fn try_from_value(value: #value_v2::value::Value) -> Result<Self, Self::Error> {
                use #value_v2::HasIdentifier;
                use #value_v2::errors::{TryFromVariantError, UnexpectedIdentifier, UnexpectedConstructorName};

                let variant = value.into_variant()?;

                if let Some(variant_id) = &variant.variant_id {
                    let expected = <Self as HasIdentifier>::identifier_with_package_id();
                    if *variant_id != expected {
                        return Err(
                            UnexpectedIdentifier::new(expected.to_string(), variant_id.to_string()).into(),
                        );
                    }
                }

                match variant.constructor.as_str() {
                    #(#from_match_arms),*
                }
            }
        }

        #[automatically_derived]
        impl #impl_generics #value_trait for #ident #ty_generics #where_clause {}
    })
}
