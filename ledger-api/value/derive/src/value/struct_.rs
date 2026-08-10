use std::debug_assert_matches;

use canton_paths::Paths;
use canton_types::Name;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DataStruct, Error, Fields, Generics, Ident, Member, WhereClause};

use crate::{
    Attr, collect_err_chain,
    value::attributes::{ItemAttributes, MemberAttributes},
};

/// Generate `Record` (and dependencies) trait impl for struct
///
/// `Value` trait is auto-implemented for `Record`-s
pub fn try_impl_record(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    // FIXME: We should explicitly reject tuple structs,
    //        because they don't have viable alternative in Daml.
    debug_assert_matches!(
        ds.fields,
        Fields::Named(_) | Fields::Unit,
        "tuple structs are not allowed"
    );

    let into_record_impl = into_record_impl(&item_attrs, &ident, &ds, &generics)?;
    let try_from_record_impl = try_from_record_impl(&item_attrs, &ident, &ds, &generics)?;
    let record_impl = record_impl(&item_attrs, &ident, &ds, &generics)?;

    let output = quote! {
        #[automatically_derived]
        #into_record_impl

        #[automatically_derived]
        #try_from_record_impl

        #[automatically_derived]
        #record_impl
    };

    Ok(output)
}

fn record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let record_trait = item_attrs.paths().record_trait();
    let try_from_value_trait = item_attrs.paths().try_from_value_trait();
    let into_value_trait = item_attrs.paths().into_value_trait();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    for field in &ds.fields {
        let field_type = &field.ty;
        where_clause
            .predicates
            .push(syn::parse_quote!(#field_type: #try_from_value_trait + #into_value_trait));
    }

    let item_impl = syn::parse_quote! {
        impl #impl_generics #record_trait for #ident #type_generics #where_clause {}
    };

    Ok(item_impl)
}

fn try_from_record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let try_from_record_path = item_attrs.paths().try_from_record_trait();
    let try_from_value_trait = item_attrs.paths().try_from_value_trait();
    let value_v2 = item_attrs.paths().value_v2();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    for field in &ds.fields {
        let field_type = &field.ty;
        where_clause
            .predicates
            .push(syn::parse_quote!(#field_type: #try_from_value_trait));
    }

    let mut labels_check = Vec::new();
    for field in &ds.fields {
        let field_ident = field.ident.as_ref().unwrap();
        let attrs = MemberAttributes::parse(&field.attrs)?;
        let expected = if let Some(attr) = attrs.name() {
            match attr {
                Attr::Fixed { attr, span } => {
                    let span = *span;
                    let name = attr.as_str();
                    quote_spanned!(span=> #name)
                }
                Attr::Expr(expr) => quote! { #expr },
            }
        } else {
            let span = field_ident.span();
            let ident_str = field_ident.to_string();
            quote_spanned!(span=> #ident_str)
        };
        let check = quote! {
            if let Some(label) = #field_ident.label {
                if label != #expected {
                    return Err(
                        UnexpectedLabel::new(
                                #expected.to_string(),
                                label.to_string(),
                            )
                            .into(),
                    );
                }
            }
        };
        labels_check.push(check);
    }

    let fields_count = ds.fields.len();
    let members1 = ds.fields.members();
    let members2 = ds.fields.members();

    let tokens = quote! {
        impl #impl_generics #try_from_record_path for #ident #type_generics #where_clause {
            type Error = #value_v2::errors::TryFromRecordError;

            #[allow(unused_imports)]
            fn try_from_record(record: #value_v2::value::Record) -> Result<Self, Self::Error> {
                use #try_from_value_trait;
                use #value_v2::HasIdentifier;
                use #value_v2::value::RecordField;
                use #value_v2::errors::{
                    TryFromRecordError, UnexpectedIdentifier, UnexpectedRecordSize,
                    UnexpectedLabel,
                };

                if let Some(record_id) = record.record_id {
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

                let [#(#members1),*] = <[RecordField; #fields_count]>::try_from(record.fields)
                    .map_err(|orig| UnexpectedRecordSize::new(#fields_count, orig.len()))?;

                #(#labels_check)*

                Ok(Self {
                    #(#members2: TryFromValue::try_from_value(#members2.value)
                        .map_err(TryFromRecordError::field_error)?),*
                })
            }
        }
    };

    let item_impl = syn::parse2(tokens)?;

    Ok(item_impl)
}

fn into_record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let into_value_trait = item_attrs.paths().into_value_trait();
    let into_record_trait = item_attrs.paths().into_record_trait();
    let value_v2 = item_attrs.paths().value_v2();

    let mut fields = Vec::new();
    for (field, member) in ds.fields.iter().zip(ds.fields.members()) {
        let attrs = MemberAttributes::parse(&field.attrs)?;

        let field_name = field_name_expr(item_attrs.paths(), attrs.name(), &member)?;

        fields.push(quote! {#value_v2::value::RecordField {
            label: Some(#field_name),
            value: #into_value_trait::into_value(self.#member),
        }});
    }

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut where_caluse = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    for field in &ds.fields {
        let field_type = &field.ty;
        where_caluse
            .predicates
            .push(syn::parse_quote!(#field_type: #into_value_trait));
    }

    let item_impl = syn::parse_quote! {
        impl #impl_generics #into_record_trait for #ident #type_generics #where_caluse {
            fn into_record(self) -> #value_v2::value::Record {
                #value_v2::value::Record {
                    record_id: Some(<Self as #value_v2::HasIdentifier>::identifier_with_package_id()),
                    fields: vec![ #(#fields),* ],
                }
            }
        }
    };

    Ok(item_impl)
}

fn field_name_expr(
    paths: &Paths,
    attr: Option<&Attr<Name>>,
    member: &Member,
) -> Result<TokenStream, Error> {
    let types_path = paths.types();

    if let Some(overwrite) = attr {
        match overwrite {
            Attr::Fixed { attr, span } => {
                let name = attr.as_str();
                let span = *span;
                let name = quote_spanned!(span=>#name);
                Ok(quote! { #types_path::Name::new_unchecked(#name.to_string()) })
            }
            Attr::Expr(expr) => Ok(quote! { #expr }),
        }
    } else {
        match &member {
            Member::Named(ident) => {
                let name = Name::new(ident.to_string()).map_err(|err| {
                    Error::new_spanned(
                        ident,
                        format!("bad field name: {}", collect_err_chain(&err).join(": ")),
                    )
                })?;
                let name = name.as_str();
                Ok(quote! { #types_path::Name::new_unchecked(#name.to_string()) })
            }
            Member::Unnamed(index) => {
                // Daml fields will be _1, _2, _3 ...
                let name = Name::new_unchecked(format!("_{}", index.index + 1));
                let name = name.as_str();
                Ok(quote! { #types_path::Name::new_unchecked(#name.to_string()) })
            }
        }
    }
}
