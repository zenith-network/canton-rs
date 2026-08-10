use canton_paths::Paths;
use canton_types::Name;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Error, Generics, Ident};

mod attributes;

use attributes::ChoiceAttributes;

use crate::{choice::attributes::NameAttr, collect_err_chain};

pub fn impl_choice_value(input: DeriveInput) -> Result<TokenStream, Error> {
    let item_attrs = ChoiceAttributes::parse(&input.attrs)?;
    let generics = input.generics;

    match input.data {
        Data::Union(_) => Err(Error::new(
            Span::call_site(),
            "Choice macro cannot be applied to union types",
        )),
        _ => impl_choice_value_inner(&item_attrs, &input.ident, &generics),
    }
}

fn impl_choice_value_inner(
    item_attrs: &ChoiceAttributes,
    ident: &Ident,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    let types = item_attrs.paths().types();
    let template = item_attrs.template();
    let result = item_attrs.result();
    let consuming = item_attrs.consuming();
    let name = choice_name_expr(item_attrs.paths(), item_attrs.name(), ident)?;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #types::Choice<#template> for #ident #type_generics #where_clause {
            const CONSUMING: bool = #consuming;
            const NAME: #types::Name = #name;
            type Result = #result;
        }
    })
}

fn choice_name_expr(
    paths: &Paths,
    attr: Option<&NameAttr>,
    ident: &Ident,
) -> Result<TokenStream, Error> {
    let types = paths.types();

    if let Some(attr) = attr {
        match attr {
            NameAttr::Fixed(name) => {
                let name = name.as_str();
                Ok(quote! { #types::Name::new_static_unchecked(#name) })
            }
            NameAttr::Expr(expr) => Ok(quote! { #expr }),
        }
    } else {
        let name = Name::new(ident.to_string())
            .map_err(|err| Error::new_spanned(ident.clone(), collect_err_chain(&err).join(": ")))?;
        let name = name.as_str();
        Ok(quote! { #types::Name::new_static_unchecked(#name) })
    }
}
