use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

mod attributes;

use attributes::InterfaceAttributes;

use crate::choice_attributes::ChoiceSpec;

pub fn impl_interface(input: DeriveInput) -> Result<TokenStream, Error> {
    let item_attrs = InterfaceAttributes::parse(&input.attrs)?;
    let generics = input.generics;
    let ident = input.ident;

    let types = item_attrs.paths().types();
    let view = item_attrs.view();
    let choice_impls =
        ChoiceSpec::expand_multiple(item_attrs.choices(), item_attrs.paths(), &ident, &generics);
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let requires_impls = item_attrs.requires().iter().map(|iface| {
        quote! {
            #[automatically_derived]
            impl #impl_generics #types::Requires<#iface> for #ident #type_generics #where_clause {}
        }
    });

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #types::TemplateOrInterface for #ident #type_generics #where_clause {}

        #[automatically_derived]
        impl #impl_generics #types::Interface for #ident #type_generics #where_clause {
            type View = #view;
        }

        #(#choice_impls)*

        #(#requires_impls)*
    })
}
