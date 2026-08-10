use std::{collections::HashMap, sync::Arc};

use canton_paths::Paths;
use canton_types::{NonEmpty, PackageId, errors::PackageIdError};
use daml_lf::v2::sealed::{
    BuiltinType, DefDataType, DefTemplate, FieldWithType, Kind, Module, SelfOrImportedPackageId,
    TemplateChoice, Type, TypeVarWithKind,
    def_data_type::{DataCons, EnumConstructors, Fields},
    type_::{Con, TApp},
};
use quote::quote;
use syn::{Ident, Visibility, token};
use tracing::{debug, trace};

use crate::{
    external_paths::ExternalPaths,
    helpers::{empty_mod, mod_with_items},
    ident, path,
    type_sets::ModuleTypeSet,
    v2::dotted_name_to_owned,
};

#[derive(Debug, thiserror::Error)]
pub enum ModuleGenError {
    #[error(transparent)]
    Syn(#[from] syn::Error),
    #[error(transparent)]
    PackageIdError(#[from] PackageIdError),
}

pub struct ModuleGenerator<'a> {
    package_identifiers: Arc<HashMap<PackageId, Ident>>,
    _external_paths: Arc<ExternalPaths>,
    module: Module<'a>,
    gen_set: ModuleTypeSet,
    paths: Paths,

    /// If data type corresponds to some template definition, it will be here
    template_table: HashMap<DefDataType<'a>, DefTemplate<'a>>,
    /// If data type corresponds to some template choice definition, it will be here
    choice_table: HashMap<DefDataType<'a>, (TemplateChoice<'a>, DefTemplate<'a>)>,

    type_attributes: HashMap<NonEmpty<String>, Vec<syn::Attribute>>,

    /// Defines how deep the module is inside the root package module
    ///
    /// Example:
    /// ```rust,ignore
    /// mod package_XXX {
    ///     const PACKAGE_ID: &str = "123";
    ///     mod A {
    ///         const MODULE_NAME: &str = "A";
    ///         // here depth will be 1
    ///
    ///         mod B {
    ///             const MODULE_NAME: &str = "A.B";
    ///             // Here depth will be 2
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Depth tells how many `super::` you need to apply to get to the "root" of the package
    depth: usize,
}

impl<'a> ModuleGenerator<'a> {
    pub fn new(
        package_identifiers: Arc<HashMap<PackageId, Ident>>,
        external_paths: Arc<ExternalPaths>,
        module: Module<'a>,
        gen_set: ModuleTypeSet,
        type_attributes: HashMap<NonEmpty<String>, Vec<syn::Attribute>>,
    ) -> Self {
        let depth = module.name().segments_count().into();
        let paths = Paths::default();

        let mut template_table = HashMap::new();
        let mut choice_table = HashMap::new();

        let data_types = module.data_types();
        for template in module.templates() {
            let name = template.tycon_name();
            let dt = data_types
                .iter()
                .find(|dt| dt.name() == name)
                .expect("must be a known type");
            template_table.insert(*dt, template);

            for choice in template.choices() {
                let type_ = choice.arg_binder().type_();
                if let Some(dt) = Self::find_def_data_type(module, type_) {
                    choice_table.insert(dt, (choice, template));
                }
            }
        }

        Self {
            package_identifiers,
            _external_paths: external_paths,
            module,
            depth,
            gen_set,
            paths,
            template_table,
            choice_table,
            type_attributes,
        }
    }

    /// Path like `super::super::...` which goes down to the module which is the root of the package
    pub fn package_root_path(&self) -> syn::Path {
        let segments = std::iter::repeat_with(|| syn::PathSegment {
            ident: token::Super::default().into(),
            arguments: syn::PathArguments::None,
        })
        .take(self.depth)
        .collect();

        syn::Path {
            // This is a relative path! No leading ::
            leading_colon: None,
            segments,
        }
    }

    /// One more `super::` than [`Self::package_root_path`]. Useful to access types from other
    /// packages.
    pub fn root_path(&self) -> syn::Path {
        let mut package_root_path = self.package_root_path();
        package_root_path.segments.push(syn::PathSegment {
            ident: token::Super::default().into(),
            arguments: syn::PathArguments::None,
        });
        package_root_path
    }

    /// Generate module content
    ///
    /// Does not include module definition (`mod X { ... }`)
    pub fn gen_module(&self) -> Result<syn::ItemMod, ModuleGenError> {
        let name = self.module.name();

        let ident = ident::generate_snake_ident(name.tail());

        let data_types = self
            .module
            .data_types()
            .into_iter()
            .filter(|dt| {
                self.gen_set
                    .as_ref()
                    .contains(&dotted_name_to_owned(&dt.name()))
            })
            .collect::<Vec<_>>();

        debug!(
            ?name,
            data_types_count = data_types.len(),
            "Entering module"
        );

        if data_types.is_empty() {
            return Ok(empty_mod(ident));
        }

        let mut items = self.gen_header()?;
        items.reserve_exact(data_types.len());

        for dt in data_types {
            items.push(self.gen_item(dt)?);

            // TODO: check if dt is template
            // TODO: check if dt is choice
        }

        Ok(mod_with_items(ident, items))
    }

    fn gen_header(&self) -> Result<Vec<syn::Item>, ModuleGenError> {
        let name = self.module.name().into_iter().collect::<Vec<_>>().join(".");
        let tokens = quote! {
            pub const MODULE_NAME: &str = #name;
        };
        let item = syn::parse2(tokens)?;
        Ok(vec![item])
    }

    /// Generate Rust item (struct, enum, ...) declaration
    fn gen_item(&self, dt: DefDataType<'a>) -> Result<syn::Item, ModuleGenError> {
        let name = dt.name();

        if !name.base().is_empty() {
            todo!("multi-segment name of a data type constructor: {dt:?}")
        }
        let name = name.tail();

        let cons = dt.data_cons();
        let params = dt.params();

        debug!(
            name,
            kind = match cons {
                DataCons::Record(_) => "record",
                DataCons::Variant(_) => "variant",
                DataCons::Enum(_) => "enum",
                DataCons::Interface => "interface",
            },
            ?params,
            "Entering data type deinition"
        );

        let entity_id = ident::generate_camel_ident(name);
        let attrs = self.gen_attrs(name, dt)?;
        let generics = self.gen_generic_params(params);

        match cons {
            DataCons::Record(fields) => self
                .gen_struct(entity_id, fields, generics, attrs)
                .map(Into::into),
            DataCons::Variant(fields) => self
                .gen_enum(entity_id, fields, generics, attrs)
                .map(Into::into),
            DataCons::Enum(enum_ctrs) => self
                .gen_unit_only_enum(entity_id, enum_ctrs, generics, attrs)
                .map(Into::into),
            DataCons::Interface => todo!("interfaces are not supported yet"),
        }
    }

    /// Generate attributes for items (structs, enums)
    ///
    /// Example:
    ///
    /// ```
    /// #[derive(
    ///     Clone,
    ///     Debug,
    ///     ::canton::ledger_api::types::value::v2::HasIdentifier,
    ///     ::canton::ledger_api::types::value::v2::Value,
    /// )]
    /// #[identifier(
    ///     package_id = super::PACKAGE_ID,
    ///     package_name = super::PACKAGE_NAME,
    ///     module = MODULE_NAME,
    ///     name = "MyName",
    /// )]
    /// ```
    fn gen_attrs(
        &self,
        name: &'a str,
        dt: DefDataType<'a>,
    ) -> Result<Vec<syn::Attribute>, ModuleGenError> {
        let root = self.paths.root();
        let value_v2 = self.paths.value_v2();
        let ledger_api_types = self.paths.ledger_api_types_v2();
        let module_name = self.module.name().into_iter().collect::<Vec<_>>().join(".");
        let path = self.package_root_path();

        // TODO: conditionally add: Copy, Eq, Hash, PartialOrd, Ord?

        // #[derive(...)]
        let derive_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: syn::parse2(
                quote! { derive(Clone, Debug, PartialEq, #value_v2::HasIdentifier, #value_v2::Value) },
            )?,
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        // #[value(...)]
        let value_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: syn::parse2(quote! { value(
                crate_path = #root,
            ) })?,
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        // #[identifier(...)]
        let identifier_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: syn::parse2(quote! { identifier(
                package_id = #path::PACKAGE_ID,
                package_name = #path::PACKAGE_NAME,
                module = #module_name,
                name = #name,
                crate_path = #root,
            ) })?,
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        let mut attrs = vec![derive_attr, value_attr, identifier_attr];

        if let Some(template) = self.template_table.get(&dt) {
            // #[derive(Template)]
            attrs.push(syn::Attribute {
                style: syn::AttrStyle::Outer,
                meta: syn::parse2(quote! { derive(#ledger_api_types::Template) })?,
                pound_token: Default::default(),
                bracket_token: Default::default(),
            });

            // #[template(...)]
            let tokens = if let Some(key) = template.key() {
                let key_type = self.gen_type(key.type_())?;
                quote! { template(key = #key_type, crate_path = #root) }
            } else {
                quote! { template(crate_path = #root) }
            };
            attrs.push(syn::Attribute {
                style: syn::AttrStyle::Outer,
                meta: syn::parse2(tokens)?,
                pound_token: Default::default(),
                bracket_token: Default::default(),
            });
        }

        if let Some((choice, template)) = self.choice_table.get(&dt) {
            // Non-interface choices are always defined in the same module, so we can safely
            // use local name here
            let template_name = template.tycon_name();
            assert!(
                template_name.base().is_empty(),
                "multi-segment template type name"
            );
            let template_name = ident::generate_ident(template_name.tail());

            // #[derive(Choice)]
            attrs.push(syn::Attribute {
                style: syn::AttrStyle::Outer,
                meta: syn::parse2(quote! { derive(#ledger_api_types::Choice) })?,
                pound_token: Default::default(),
                bracket_token: Default::default(),
            });

            let result = self.gen_type(choice.ret_type())?;
            let consuming = choice.consuming();
            let choice_name = choice.name();

            // #[choice(...)]
            attrs.push(syn::Attribute {
                style: syn::AttrStyle::Outer,
                meta: syn::parse2(quote! { choice(
                    template = #template_name,
                    crate_path = #root,
                    result = #result,
                    consuming = #consuming,
                    name = #choice_name,
                ) })?,
                pound_token: Default::default(),
                bracket_token: Default::default(),
            });
        }

        if let Some(additional_attrs) = self.type_attributes.get(&dotted_name_to_owned(&dt.name()))
        {
            attrs.extend_from_slice(additional_attrs);
        }

        Ok(attrs)
    }

    /// Generates `#[name = "MyName"]` attribute
    ///
    /// Used for struct fields, enum variants etc.
    fn gen_name_attr(&self, name: &'a str) -> Result<syn::Attribute, ModuleGenError> {
        Ok(syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: syn::parse2(quote! { name = #name })?,
            pound_token: Default::default(),
            bracket_token: Default::default(),
        })
    }

    /// Generates Rust struct declaration `pub struct Name<...> { ... }`
    fn gen_struct(
        &self,
        ident: Ident,
        fields: Fields<'a>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> Result<syn::ItemStruct, ModuleGenError> {
        let field_defs = fields
            .fields()
            .into_iter()
            .map(|field| self.gen_field(field))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(syn::ItemStruct {
            attrs,
            vis: Visibility::Public(Default::default()),
            struct_token: Default::default(),
            ident,
            generics,
            fields: syn::Fields::Named(syn::FieldsNamed {
                brace_token: Default::default(),
                named: field_defs.into_iter().collect(),
            }),
            semi_token: None,
        })
    }

    /// Generate generic params for an item (`<A, B, C>`)
    fn gen_generic_params(&self, params: Vec<TypeVarWithKind<'a>>) -> syn::Generics {
        if params.is_empty() {
            return syn::Generics::default();
        }

        let params = params
            .into_iter()
            .map(|param| {
                let kind = param.kind();
                let ident = ident::generate_camel_ident(param.var());
                match kind {
                    Kind::Star => syn::GenericParam::Type(syn::TypeParam {
                        ident,
                        attrs: Vec::new(),
                        colon_token: None,
                        bounds: Default::default(),
                        eq_token: None,
                        default: None,
                    }),
                    Kind::Arrow(_) => todo!("Arrow kind is not supported yet"),
                    Kind::Nat => todo!("Nat kind is not supported"),
                }
            })
            .collect::<Vec<_>>();
        syn::Generics {
            params: params.into_iter().collect(),
            lt_token: Some(Default::default()),
            gt_token: Some(Default::default()),
            where_clause: None,
        }
    }

    /// Generates a field of a Rust struct
    fn gen_field(&self, field: FieldWithType<'a>) -> Result<syn::Field, ModuleGenError> {
        trace!(?field, "Entering record field");
        let field_name = field.field();
        let value_attribute = self.gen_name_attr(field_name)?;
        let field_id = ident::generate_snake_ident(field_name);
        let field_type = self.gen_type(field.type_())?;

        let field = syn::Field {
            attrs: vec![value_attribute],
            vis: Visibility::Public(Default::default()),
            mutability: syn::FieldMutability::None,
            ident: Some(field_id),
            colon_token: Some(Default::default()),
            ty: field_type,
        };

        Ok(field)
    }

    /// Generates Rust enum declaration `pub enum Name { ... }`
    fn gen_enum(
        &self,
        ident: Ident,
        fields: Fields<'a>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> Result<syn::ItemEnum, ModuleGenError> {
        let variants = fields
            .fields()
            .into_iter()
            .map(|field| self.gen_enum_variant(field))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(syn::ItemEnum {
            attrs,
            vis: Visibility::Public(Default::default()),
            ident,
            generics,
            variants: variants.into_iter().collect(),
            enum_token: Default::default(),
            brace_token: Default::default(),
        })
    }

    /// Generates a Rust enum variant with unnamed field (like `MyVar(X)`)
    fn gen_enum_variant(&self, field: FieldWithType<'a>) -> Result<syn::Variant, ModuleGenError> {
        let field_name = field.field();
        let value_attribute = self.gen_name_attr(field_name)?;
        let ident = ident::generate_camel_ident(field_name);
        let field_type = self.gen_type(field.type_())?;

        let field = syn::Field {
            attrs: Vec::new(),
            vis: Visibility::Inherited,
            mutability: syn::FieldMutability::None,
            ident: None,
            ty: field_type,
            colon_token: Some(Default::default()),
        };

        Ok(syn::Variant {
            attrs: vec![value_attribute],
            ident,
            fields: syn::Fields::Unnamed(syn::FieldsUnnamed {
                paren_token: Default::default(),
                unnamed: [field].into_iter().collect(),
            }),
            discriminant: None,
        })
    }

    /// Generates a Rust enum unit variant (no fields)
    fn gen_enum_unit_variant(&self, variant: &'a str) -> Result<syn::Variant, ModuleGenError> {
        let ident = ident::generate_camel_ident(variant);
        let value_attribute = self.gen_name_attr(variant)?;
        Ok(syn::Variant {
            attrs: vec![value_attribute],
            ident,
            fields: syn::Fields::Unit,
            discriminant: None,
        })
    }

    /// Generates Rust unit-only enum declaration `pub enum Name<...> { A, B, C, ... }`
    fn gen_unit_only_enum(
        &self,
        ident: Ident,
        enum_ctrs: EnumConstructors<'a>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> Result<syn::ItemEnum, ModuleGenError> {
        let variants = enum_ctrs
            .constructors()
            .into_iter()
            .map(|variant| self.gen_enum_unit_variant(variant))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(syn::ItemEnum {
            attrs,
            vis: Visibility::Public(Default::default()),
            enum_token: Default::default(),
            ident,
            generics,
            brace_token: Default::default(),
            variants: variants.into_iter().collect(),
        })
    }

    /// Generate type for Daml type
    fn gen_type(&self, type_: Type<'a>) -> Result<syn::Type, ModuleGenError> {
        match type_ {
            Type::Var(var) => {
                let var = ident::generate_camel_ident(var.var());
                let tokens = quote! { #var };
                syn::parse2(tokens).map_err(Into::into)
            }
            Type::Con(con) => self.gen_con(con).map(Into::into),
            Type::Builtin(builtin) => {
                let args = builtin.args();
                self.gen_builtin_type(builtin.type_(), &args)
            }
            // Type::Forall(_) => todo!(),
            // Type::Struct(_) => todo!(),
            Type::Nat => Ok(syn::Type::Never(syn::TypeNever {
                bang_token: Default::default(),
            })),
            // Type::Syn(_) => todo!(),
            Type::Tapp(tapp) => self.gen_tapp(tapp).map(Into::into),
        }
    }

    /// Generate type for Daml type application
    fn gen_tapp(&self, tapp: TApp<'a>) -> Result<syn::TypePath, ModuleGenError> {
        let (main, params) = self.flatten_tapp(Type::Tapp(tapp));
        trace!(?main, ?params, "Generating TApp");

        let main_type = self.gen_type(main)?;
        let generic_params = params
            .into_iter()
            .map(|param| self.gen_type(param))
            .collect::<Result<Vec<_>, _>>()?;
        let tokens = match main {
            // Manually erase Numeric generic arg
            Type::Builtin(bi) if bi.type_() == BuiltinType::Numeric => quote! { #main_type },
            _ => quote! { #main_type < #(#generic_params),* > },
        };
        trace!(tokens = tokens.to_string(), "Generated TApp");
        Ok(syn::parse2(tokens).unwrap())
    }

    /// Turns `(t1 t2) t3 ... -> t1 <t2, t3, ...>`
    fn flatten_tapp(&self, mut ty: Type<'a>) -> (Type<'a>, Vec<Type<'a>>) {
        let mut args = Vec::new();
        while let Type::Tapp(tapp) = ty {
            args.push(tapp.rhs());
            ty = tapp.lhs();
        }
        args.reverse();
        (ty, args)
    }

    /// Generate type for Daml type constructor application
    fn gen_con(&self, con: Con<'a>) -> Result<syn::TypePath, ModuleGenError> {
        let tycon = con.tycon();

        let name = tycon.name();
        if !name.base().is_empty() {
            todo!("multi-segment tycon name: {con:?}")
        }
        let name = name.tail();
        trace!(name, "Generating type constructor");

        let args = con
            .args()
            .into_iter()
            .map(|arg| self.gen_type(arg))
            .collect::<Result<Vec<_>, _>>()?;

        let module_id = tycon.module();
        let module_name = module_id.module_name();
        let package_id = module_id.package_id();

        let package_path = match package_id {
            SelfOrImportedPackageId::SelfPackageId => self.package_root_path(),
            SelfOrImportedPackageId::ImportedPackageId(package_id) => {
                let package_id = PackageId::new_unchecked_owned(package_id.to_string());
                // FIXME: remove unwrap
                let package_ident = self.package_identifiers.get(&package_id).unwrap();
                let mut path = self.root_path();
                path.segments.push(syn::PathSegment {
                    ident: package_ident.clone(),
                    arguments: syn::PathArguments::None,
                });
                path
            }
        };

        // TODO: ensure package with 'package_id' will be generated

        let type_id = ident::generate_camel_ident(name);
        let module_path = path::generate_module_path(module_name.iter());

        let mut tokens = quote! { #package_path::#module_path::#type_id };
        if !args.is_empty() {
            tokens = quote! { #tokens < #(#args),* > };
        }
        syn::parse2(tokens).map_err(Into::into)
    }

    /// Generate type for Daml built-in type
    fn gen_builtin_type(
        &self,
        builtin: BuiltinType,
        args: &[Type],
    ) -> Result<syn::Type, ModuleGenError> {
        let types = self.paths.types();
        let tokens = match builtin {
            BuiltinType::Unit => quote! { () },
            BuiltinType::Bool => quote! { bool },
            BuiltinType::Int64 => quote! { i64 },
            BuiltinType::Date => todo!(),
            BuiltinType::Timestamp => todo!(),
            BuiltinType::Numeric => quote! { #types::Numeric },
            BuiltinType::Party => quote! { #types::PartyId },
            BuiltinType::Text => quote! { ::std::string::String },
            BuiltinType::ContractId => {
                debug_assert!(
                    args.len() <= 1,
                    "ContractId with type args greater than 1: {args:?}"
                );
                let mut tokens = quote! { #types::ContractId };
                if let Some(arg) = args.first().map(|arg| self.gen_type(*arg)).transpose()? {
                    tokens = quote! { #tokens < #arg > };
                }
                tokens
            }
            BuiltinType::Optional => quote! { ::std::option::Option },
            BuiltinType::List => {
                debug_assert!(
                    args.len() <= 1,
                    "List with type args greater than 1: {args:?}"
                );
                let mut tokens = quote! { ::std::vec::Vec };
                if let Some(arg) = args.first().map(|arg| self.gen_type(*arg)).transpose()? {
                    tokens = quote! { #tokens<#arg> };
                }
                tokens
            }
            BuiltinType::Genmap => {
                debug_assert!(
                    args.len() <= 2,
                    "Gextmap with type args greater than 2: {args:?}"
                );
                let mut tokens = quote! { ::std::collections::BTreeMap };
                if !args.is_empty() {
                    let mut tokens_args = Vec::new();
                    for arg in args {
                        tokens_args.push(self.gen_type(*arg)?);
                    }
                    tokens = quote! { #tokens<#(#tokens_args),*> };
                }
                tokens
            }
            // TODO: is this reachable?
            BuiltinType::Any => quote! { ::std::boxed::Box<dyn ::std::any::Any> },
            BuiltinType::AnyException => todo!(),
            BuiltinType::TypeRep => unreachable!(),
            BuiltinType::Arrow => {
                // TODO: this should be unreachable?
                debug_assert_eq!(
                    args.len(),
                    2,
                    "Arrow type args with length different from 2: {args:?}"
                );
                let a = self.gen_type(args[0])?;
                let b = self.gen_type(args[1])?;
                quote! { fn(#a) -> #b }
            }
            BuiltinType::Update => todo!(),
            BuiltinType::FailureCategory => unreachable!(),
            BuiltinType::Textmap => {
                debug_assert!(
                    args.len() <= 1,
                    "Textmap with type args greater than 1: {args:?}"
                );
                let mut tokens = quote! { ::std::collections::BTreeMap };
                if let Some(arg) = args.first().map(|arg| self.gen_type(*arg)).transpose()? {
                    tokens = quote! { #tokens<#arg> };
                }
                tokens
            }
            BuiltinType::Bignumeric => todo!(),
            BuiltinType::RoundingMode => todo!(),
        };
        syn::parse2(tokens).map_err(Into::into)
    }

    /// If given type is defined in this module, find it's DefDataType
    fn find_def_data_type(module: Module<'a>, type_: Type<'a>) -> Option<DefDataType<'a>> {
        type_
            .type_con_id()
            .map(|type_con_id| {
                let module_id = type_con_id.module();
                let is_local =
                    module_id.package_id().is_self() && module_id.module_name() == module.name();
                is_local.then(|| {
                    let type_name = type_con_id.name();
                    module
                        .data_types()
                        .into_iter()
                        .find(|dt| dt.name() == type_name)
                        .expect("type must be known")
                })
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case::unit(BuiltinType::Unit)]
    #[case::bool(BuiltinType::Bool)]
    #[case::int64(BuiltinType::Int64)]
    #[case::party(BuiltinType::Party)]
    #[case::text(BuiltinType::Text)]
    #[case::contract_id(BuiltinType::ContractId)]
    #[case::optional(BuiltinType::Optional)]
    #[case::list(BuiltinType::List)]
    #[case::genmap(BuiltinType::Genmap)]
    #[case::textmap(BuiltinType::Textmap)]
    fn test_gen_builtin(#[case] _t: BuiltinType) {
        // let _ = ModuleGenerator::gen_builtin_type(t, &[]).unwrap();
    }
}
