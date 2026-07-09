use canton_types::Name;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Error, GenericParam, Generics, Ident, Member, WhereClause};

use crate::{
    attributes::{
        EntityNameAttr, FieldNameAttr, ItemAttributes, MemberAttributes, ModuleNameAttr,
        PackageIdAttr, PackageNameAttr,
    },
    collect_err_chain,
    paths::Paths,
};

/// Generate `Record` (and dependencies) trait impl for struct
///
/// `Value` trait is auto-implemented for `Record`-s
pub fn try_impl_record(
    item_attrs: ItemAttributes,
    ident: Ident,
    ds: DataStruct,
    generics: Generics,
) -> Result<TokenStream, Error> {
    let into_record_impl = into_record_impl(&item_attrs, &ident, &ds, &generics)?;
    let try_from_record_impl = try_from_record_impl(&item_attrs, &ident, &ds, &generics)?;
    let record_impl = record_impl(&item_attrs, &ident, &generics)?;
    let has_identifier_impl = has_identifier_impl(&item_attrs, &ident, &generics)?;
    let template_impl = template_impl(&item_attrs, &ident, &generics);

    let output = quote! {
        #[automatically_derived]
        #into_record_impl

        #[automatically_derived]
        #try_from_record_impl

        #[automatically_derived]
        #record_impl

        #[automatically_derived]
        #has_identifier_impl

        #template_impl
    };

    Ok(output)
}

fn template_impl(item_attrs: &ItemAttributes, ident: &Ident, generics: &Generics) -> TokenStream {
    let types = &item_attrs.paths().types;
    let ledger_api_types = &item_attrs.paths().ledger_api_types;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    item_attrs.template().then(|| {
        quote! {
            #[automatically_derived]
            impl #impl_generics #types::Template for #ident #type_generics #where_clause {}
            #[automatically_derived]
            impl #impl_generics #ledger_api_types::v2::TemplateValue for #ident #type_generics #where_clause {}
        }
    }).unwrap_or_default()
}

fn has_identifier_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let value_v2 = &item_attrs.paths().value_v2;
    let types = &item_attrs.paths().types;

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let package_id = package_id_expr(item_attrs.paths(), item_attrs.package_id());
    let package_name = package_name_expr(item_attrs.paths(), item_attrs.package_name());
    let module_name = module_name_expr(item_attrs.paths(), item_attrs.module_name());
    let entity_name = entity_name_expr(item_attrs.paths(), item_attrs.name(), ident)?;

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

fn record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let record_path = &item_attrs.paths().record_trait;

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    for tp in generics.type_params() {
        let ident = &tp.ident;
        where_clause
            .predicates
            .push(syn::parse_quote! { #ident: #record_path });
    }

    let item_impl = syn::parse_quote! {
        impl #impl_generics #record_path for #ident #type_generics #where_clause {}
    };

    Ok(item_impl)
}

fn try_from_record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let try_from_record_path = &item_attrs.paths().try_from_record_trait;
    let value_v2 = &item_attrs.paths().value_v2;

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    // Pre-render
    let tokens = quote! {
        impl #impl_generics #try_from_record_path for #ident #type_generics #where_clause {
            type Error = #value_v2::errors::ValueKindError;

            fn try_from_record(record: #value_v2::value::Record) -> Result<Self, Self::Error> {
                todo!()
            }
        }
    };

    let item_impl = syn::parse2(tokens)?;

    // TODO: inject generics

    Ok(item_impl)
}

fn into_record_impl(
    item_attrs: &ItemAttributes,
    ident: &Ident,
    ds: &DataStruct,
    generics: &Generics,
) -> Result<syn::ItemImpl, Error> {
    let Paths {
        into_value_trait,
        into_record_trait,
        value,
        value_v2,
        ..
    } = item_attrs.paths();

    let identifier_expr = identifier_expr(item_attrs, ident)?;

    // TODO: What shall we do with the newtypes like `struct S(T)`? Something special?

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
    for tp in generics.type_params() {
        let ident = &tp.ident;
        where_caluse
            .predicates
            .push(syn::parse_quote! { #ident: #into_value_trait });
    }

    let item_impl = syn::parse_quote! {
        impl #impl_generics #into_record_trait for #ident #type_generics #where_caluse {
            fn into_record(self) -> #value_v2::value::Record {
                #value_v2::value::Record {
                    record_id: Some(#identifier_expr),
                    fields: vec![ #(#fields),* ],
                }
            }
        }
    };

    Ok(item_impl)
}

fn field_name_expr(
    paths: &Paths,
    attr: Option<&FieldNameAttr>,
    member: &Member,
) -> Result<TokenStream, Error> {
    let types_path = &paths.types;

    if let Some(overwrite) = attr {
        match overwrite {
            FieldNameAttr::Fixed(name) => {
                let name = name.as_str();
                Ok(quote! { #types_path::Name::new_unchecked(#name.to_string()) })
            }
            FieldNameAttr::Expr(expr) => Ok(quote! { #expr }),
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

/// Generate an expression which initializes Identifier based on given item attributes
fn identifier_expr(item_attrs: &ItemAttributes, ident: &Ident) -> Result<syn::Expr, Error> {
    let value_v2 = &item_attrs.paths().value_v2;

    let package_id = package_id_expr(item_attrs.paths(), item_attrs.package_id());
    let module_name = module_name_expr(item_attrs.paths(), item_attrs.module_name());
    let entity_name = entity_name_expr(item_attrs.paths(), item_attrs.name(), ident)?;

    Ok(syn::parse_quote! {
        #value_v2::Identifier {
            package_id: #package_id,
            module_name: #module_name,
            entity_name: #entity_name,
        }
    })
}

/// Resolve package ID expression, which will be used in the Identifier
///
/// Example outputs:
///
/// - `super::super::PACKAGE_ID`
/// - `::canton::types::PackageId::new_unchecked("1234")`
fn package_id_expr(paths: &Paths, attr: &PackageIdAttr) -> TokenStream {
    let types = &paths.types;
    match attr {
        PackageIdAttr::Fixed(package_id) => {
            let id = package_id.as_str();
            // this id is already checked to be valid, so we can pass it to new_unchecked()
            quote! { #types::PackageId::new_unchecked(#id) }
        }
        PackageIdAttr::Expr(expr) => quote! { #expr }, // use as it is
    }
}

/// Resolve package name expression
///
/// Example outputs:
///
/// - `super::super::PACKAGE_NAME`
/// - `::canton::types::PackageName::new_unchecked("abcd")`
fn package_name_expr(paths: &Paths, attr: &PackageNameAttr) -> TokenStream {
    let types = &paths.types;
    match attr {
        PackageNameAttr::Fixed(package_name) => {
            let id = package_name.as_str();
            // this id is already checked to be valid, so we can pass it to new_unchecked()
            quote! { #types::PackageName::new_unchecked(#id) }
        }
        PackageNameAttr::Expr(expr) => quote! { #expr }, // use as it is
    }
}

/// Resolve module name expression, which will be used in the Identifier
fn module_name_expr(paths: &Paths, attr: &ModuleNameAttr) -> TokenStream {
    let types = &paths.types;
    match attr {
        ModuleNameAttr::Fixed(name) => {
            let base = name.segments().base.iter().map(Name::as_str);
            let tail = name.segments().tail.as_str();
            quote! {
                #types::DottedName::from_segments(
                    #types::NonEmpty::new(
                        vec![#(#types::Name::new_unchecked(#base.to_string())),*],
                        #types::Name::new_unchecked(#tail.to_string()),
                    )
                )
            }
        }
        ModuleNameAttr::Expr(expr) => quote! { #expr },
    }
}

fn entity_name_expr(
    paths: &Paths,
    attr: Option<&EntityNameAttr>,
    ident: &Ident,
) -> Result<TokenStream, Error> {
    let types_path = &paths.types;
    if let Some(overwrite) = attr {
        match overwrite {
            EntityNameAttr::Fixed(name) => {
                let base = name.segments().base.iter().map(Name::as_str);
                let tail = name.segments().tail.as_str();
                Ok(quote! {
                    #types_path::DottedName::from_segments(
                        #types_path::NonEmpty::new(
                            vec![#(#types_path::Name::new_unchecked(#base.to_string())),*],
                            #types_path::Name::new_unchecked(#tail.to_string()),
                        )
                    )
                })
            }
            EntityNameAttr::Expr(expr) => Ok(quote! { #expr }),
        }
    } else {
        let name = Name::new(ident.to_string()).map_err(|err| {
            Error::new_spanned(
                ident.clone(),
                format!("bad struct name: {}", collect_err_chain(&err).join(": ")),
            )
        })?;
        let name = name.as_str();
        Ok(quote! { #types_path::DottedName::single(
            #types_path::Name::new_unchecked(#name.to_string())
        ) })
    }
}

/// `T` -> `T: #bound`
fn bounds_for_type_params(generics: &Generics, bound: &TokenStream) -> Vec<TokenStream> {
    generics
        .params
        .iter()
        .filter_map(|param| {
            if let GenericParam::Type(type_param) = param {
                let ident = &type_param.ident;
                Some(quote! { #ident: #bound })
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
