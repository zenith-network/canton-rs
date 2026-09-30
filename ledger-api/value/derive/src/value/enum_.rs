use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DataEnum, Error, Fields, Generics, Ident};

use crate::{
    Attr,
    value::{
        attributes::{ItemAttributes, MemberAttributes},
        bounds::apply_bounds,
    },
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

    let mut field_types = Vec::new();
    let mut into_match_arms = Vec::new();
    let mut from_match_arms = Vec::new();
    for variant in &de.variants {
        let Fields::Unnamed(fields) = &variant.fields else {
            return Err(Error::new_spanned(
                &variant.fields,
                "Daml LF variant constructors must contain exactly one unnamed field",
            ));
        };
        if fields.unnamed.len() != 1 {
            return Err(Error::new_spanned(
                &variant.fields,
                "Daml LF variant constructors must contain exactly one unnamed field",
            ));
        }

        let variant_ident = &variant.ident;
        let attrs = MemberAttributes::parse(&variant.attrs)?;
        let field_type = &fields.unnamed.first().unwrap().ty;
        if !attrs.omit_bound() {
            field_types.push(field_type);
        }
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
            Self::#variant_ident(value) => #value_v2::value::Variant {
                variant_id: Some(<Self as HasIdentifier>::identifier_with_package_id()),
                constructor: #types::Name::new_static_unchecked(#name),
                value: #into_value_trait::into_value(value),
            }
        });
        from_match_arms.push(quote! {
            #name => Ok(Self::#variant_ident(
                #try_from_value_trait::try_from_value(variant.value)
                    .map_err(TryFromVariantError::value_error)?,
            ))
        });
    }
    from_match_arms.push(quote! {
        t => Err(UnexpectedConstructorName::new(t.to_string()).into())
    });

    let into_generics = apply_bounds(
        generics,
        field_types.iter().copied(),
        &[&into_value_trait],
        item_attrs.bounds().iter().chain(item_attrs.into_bounds()),
    );
    let (into_impl_generics, into_ty_generics, into_where_clause) = into_generics.split_for_impl();

    let from_generics = apply_bounds(
        generics,
        field_types.iter().copied(),
        &[&try_from_value_trait],
        item_attrs.bounds().iter().chain(item_attrs.from_bounds()),
    );
    let (from_impl_generics, from_ty_generics, from_where_clause) = from_generics.split_for_impl();

    let value_generics = apply_bounds(
        generics,
        field_types.iter().copied(),
        &[&into_value_trait, &try_from_value_trait],
        item_attrs
            .bounds()
            .iter()
            .chain(item_attrs.into_bounds())
            .chain(item_attrs.from_bounds()),
    );
    let (value_impl_generics, value_ty_generics, value_where_clause) =
        value_generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #into_impl_generics #into_value_trait for #ident #into_ty_generics #into_where_clause {
            fn into_value(self) -> #value_v2::value::Value {
                use #value_v2::HasIdentifier;

                #value_v2::value::Value::Variant(Box::new(match self {
                    #(#into_match_arms),*
                }))
            }
        }

        #[automatically_derived]
        impl #into_impl_generics #into_value_trait for ::std::boxed::Box<#ident #into_ty_generics> #into_where_clause {
            fn into_value(self) -> #value_v2::value::Value {
                use #value_v2::HasIdentifier;

                let self_ = *self;
                self_.into_value()
            }
        }

        #[automatically_derived]
        impl #from_impl_generics #try_from_value_trait for #ident #from_ty_generics #from_where_clause {
            type Error = #value_v2::errors::TryFromVariantError;

            #[allow(unused_imports)]
            fn try_from_value(value: #value_v2::value::Value) -> Result<Self, Self::Error> {
                use #value_v2::HasIdentifier;
                use #value_v2::errors::{
                    TryFromVariantError, UnexpectedConstructorName, UnexpectedIdentifier,
                };

                let variant = value.into_variant()?;

                if let Some(variant_id) = variant.variant_id {
                    let expected = <Self as HasIdentifier>::identifier_with_package_id();
                    if variant_id != expected {
                        return Err(
                            UnexpectedIdentifier::new(
                                expected.to_string(),
                                variant_id.to_string(),
                            )
                            .into(),
                        );
                    }
                }

                let constructor = variant.constructor.as_str();

                match constructor {
                    #(#from_match_arms),*
                }
            }
        }

        #[automatically_derived]
        impl #from_impl_generics #try_from_value_trait for ::std::boxed::Box<#ident #from_ty_generics> #from_where_clause {
            type Error = <#ident #from_ty_generics as #try_from_value_trait>::Error;

            fn try_from_value(value: #value_v2::value::Value) -> Result<Self, Self::Error> {
                Ok(std::boxed::Box::new(<#ident #from_ty_generics as #try_from_value_trait>::try_from_value(value)?))
            }
        }

        #[automatically_derived]
        impl #value_impl_generics #value_trait for #ident #value_ty_generics #value_where_clause {}

        #[automatically_derived]
        impl #value_impl_generics #value_trait for ::std::boxed::Box<#ident #value_ty_generics> #value_where_clause {}
    })
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

    let into_generics = apply_bounds(
        generics,
        std::iter::empty(),
        &[],
        item_attrs.bounds().iter().chain(item_attrs.into_bounds()),
    );
    let (into_impl_generics, into_ty_generics, into_where_clause) = into_generics.split_for_impl();

    let from_generics = apply_bounds(
        generics,
        std::iter::empty(),
        &[],
        item_attrs.bounds().iter().chain(item_attrs.from_bounds()),
    );
    let (from_impl_generics, from_ty_generics, from_where_clause) = from_generics.split_for_impl();

    let value_generics = apply_bounds(
        generics,
        std::iter::empty(),
        &[],
        item_attrs
            .bounds()
            .iter()
            .chain(item_attrs.into_bounds())
            .chain(item_attrs.from_bounds()),
    );
    let (value_impl_generics, value_ty_generics, value_where_clause) =
        value_generics.split_for_impl();

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
        impl #into_impl_generics #into_value_trait for #ident #into_ty_generics #into_where_clause {
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
        impl #from_impl_generics #try_from_value_trait for #ident #from_ty_generics #from_where_clause {
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
        impl #value_impl_generics #value_trait for #ident #value_ty_generics #value_where_clause {}
    })
}
