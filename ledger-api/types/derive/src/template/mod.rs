use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Error, Generics, Ident};

mod attributes;

use attributes::TemplateAttributes;

pub fn impl_template_value(input: DeriveInput) -> Result<TokenStream, Error> {
    let item_attrs = TemplateAttributes::parse(&input.attrs)?;
    let generics = input.generics;

    match input.data {
        Data::Struct(_) => Ok(inner(&item_attrs, &input.ident, &generics)),
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

fn inner(item_attrs: &TemplateAttributes, ident: &Ident, generics: &Generics) -> TokenStream {
    let types = item_attrs.paths().types();
    let key = item_attrs.key();
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let template_with_key_impl = key.map(|key| {
        quote! {
            #[automatically_derived]
            impl #impl_generics #types::TemplateWithKey for #ident #type_generics #where_clause {
                type Key = #key;
            }
        }
    });
    quote! {
        #[automatically_derived]
        impl #impl_generics #types::Template for #ident #type_generics #where_clause {}

        #template_with_key_impl
    }
}
