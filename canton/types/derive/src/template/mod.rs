use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Error, Generics, Ident};

mod attributes;

use attributes::TemplateAttributes;

use crate::choice_attributes::ChoiceSpec;

pub fn impl_template(input: DeriveInput) -> Result<TokenStream, Error> {
    let item_attrs = TemplateAttributes::parse(&input.attrs)?;
    let generics = input.generics;

    match input.data {
        Data::Struct(_) => inner(&item_attrs, &input.ident, &generics),
        Data::Enum(_) => Err(Error::new(
            Span::call_site(),
            "Template macro cannot be applied to enum types",
        )),
        Data::Union(_) => Err(Error::new(
            Span::call_site(),
            "Template macro cannot be applied to union types",
        )),
    }
}

fn inner(
    item_attrs: &TemplateAttributes,
    ident: &Ident,
    generics: &Generics,
) -> Result<TokenStream, Error> {
    let types = item_attrs.paths().types();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let template_with_key_impl = item_attrs.key().map(|key| {
        quote! {
            #[automatically_derived]
            impl #impl_generics #types::TemplateWithKey for #ident #type_generics #where_clause {
                type Key = #key;
            }
        }
    });

    let choice_impls =
        ChoiceSpec::expand_multiple(item_attrs.choices(), item_attrs.paths(), ident, generics);

    let implements_impls = item_attrs.implements().iter().map(|iface| {
        quote! {
            #[automatically_derived]
            impl #impl_generics #types::Implements<#iface> for #ident #type_generics #where_clause {}
        }
    });

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #types::TemplateOrInterface for #ident #type_generics #where_clause {}

        #[automatically_derived]
        impl #impl_generics #types::Template for #ident #type_generics #where_clause {}

        #template_with_key_impl

        #(#choice_impls)*

        #(#implements_impls)*
    })
}
