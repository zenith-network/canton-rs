use canton_paths::Paths;
use canton_types::{DottedName, Name, PackageId, PackageName};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Error, Ident};

mod attributes;

use attributes::IdentifierAttributes;

use crate::{Attr, collect_err_chain};

pub fn impl_has_identifier(input: DeriveInput) -> Result<TokenStream, Error> {
    let attrs = IdentifierAttributes::parse(&input.attrs)?;
    let ident = &input.ident;

    let value_v2 = attrs.paths().value_v2();
    let types = attrs.paths().types();

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let package_id = package_id_expr(attrs.paths(), attrs.package_id());
    let package_name = package_name_expr(attrs.paths(), attrs.package_name());
    let module_name = module_name_expr(attrs.paths(), attrs.module_name());
    let entity_name = entity_name_expr(attrs.paths(), attrs.name(), ident)?;

    let item_impl = syn::parse_quote! {
        impl #impl_generics #value_v2::HasIdentifier for #ident #type_generics #where_clause {
            fn package_id() -> #types::PackageId {
                #package_id
            }
            fn package_name() -> #types::PackageName {
                #package_name
            }
            fn module_name() -> #types::DottedName {
                #module_name
            }
            fn entity_name() -> #types::DottedName {
                #entity_name
            }
        }
    };

    Ok(item_impl)
}

/// Resolve package ID expression, which will be used in the Identifier
///
/// Example outputs:
///
/// - `super::super::PACKAGE_ID`
/// - `::canton::types::PackageId::new_unchecked("1234")`
fn package_id_expr(paths: &Paths, attr: &Attr<PackageId>) -> TokenStream {
    let types = paths.types();

    match attr {
        Attr::Fixed { attr, span } => {
            let package_id = attr.as_str();
            let span = *span;
            let id = quote_spanned!(span=> #package_id);
            quote! { #types::PackageId::new_unchecked(#id) }
        }
        Attr::Expr(expr) => quote! { #expr },
    }
}

/// Resolve package name expression
///
/// Example outputs:
///
/// - `super::super::PACKAGE_NAME`
/// - `::canton::types::PackageName::new_unchecked("abcd")`
fn package_name_expr(paths: &Paths, attr: &Attr<PackageName>) -> TokenStream {
    let types = paths.types();

    match attr {
        Attr::Fixed { attr, span } => {
            let package_name = attr.as_str();
            let span = *span;
            let pname = quote_spanned!(span=> #package_name);
            quote! { #types::PackageName::new_unchecked(#pname) }
        }
        Attr::Expr(expr) => quote! { #expr },
    }
}

/// Resolve module name expression, which will be used in the Identifier
fn module_name_expr(paths: &Paths, attr: &Attr<DottedName>) -> TokenStream {
    let types = paths.types();

    match attr {
        Attr::Fixed { attr, span } => {
            let base = attr.segments().base.iter().map(Name::as_str);
            let tail = attr.segments().tail.as_str();
            let span = *span;

            let base = quote_spanned!(span=>#(#types::Name::new_unchecked(#base.to_string())),*);
            let tail = quote_spanned!(span=>#tail);
            quote! {
                #types::DottedName::from_segments(
                    #types::NonEmpty::new(
                        vec![#base],
                        #types::Name::new_unchecked(#tail.to_string()),
                    )
                )
            }
        }
        Attr::Expr(expr) => quote! { #expr },
    }
}

fn entity_name_expr(
    paths: &Paths,
    attr: Option<&Attr<DottedName>>,
    ident: &Ident,
) -> Result<TokenStream, Error> {
    let types = paths.types();
    if let Some(overwrite) = attr {
        match overwrite {
            Attr::Fixed { attr, span } => {
                let span = *span;
                let base = attr.segments().base.iter().map(Name::as_str);
                let tail = attr.segments().tail.as_str();

                let base =
                    quote_spanned!(span=>#(#types::Name::new_unchecked(#base.to_string())),*);
                let tail = quote_spanned!(span=>#tail);
                Ok(quote! {
                    #types::DottedName::from_segments(
                        #types::NonEmpty::new(
                            vec![#base],
                            #types::Name::new_unchecked(#tail.to_string()),
                        )
                    )
                })
            }
            Attr::Expr(expr) => Ok(quote! { #expr }),
        }
    } else {
        let span = ident.span();
        let name = Name::new(ident.to_string())
            .map_err(|err| Error::new_spanned(ident.clone(), collect_err_chain(&err).join(": ")))?;
        let name = name.as_str();
        let name = quote_spanned!(span=>#name);
        Ok(quote! { #types::DottedName::single(
            #types::Name::new_unchecked(#name.to_string())
        ) })
    }
}
