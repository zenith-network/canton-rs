use canton_types::{Name, NonEmpty, PackageId};
use daml_lf::v2::sealed::{
    DefDataType, DottedName, FieldWithType, Kind, Module, Package, Type, TypeVarWithKind,
    def_data_type::{DataCons, EnumConstructors, Fields},
};
use quote::quote;
use syn::parse_quote;
use tracing::{debug, trace};

use crate::{
    generator::GenCtx,
    ident,
    ir::Edge,
    path,
    v2::{
        deps_resolver::DepsResolver,
        ir::{Def, Definition},
        type_generator::TypeGenerator,
    },
};

/// Syntax item generator for Daml LF v2
#[derive(Clone, Debug)]
pub struct ItemGenerator<'a> {
    ctx: &'a GenCtx<'a>,
    deps_resolver: DepsResolver<'a>,
    type_generator: TypeGenerator<'a>,
    definition: Definition<'a>,
    package_id: &'a PackageId,
    module_path: syn::Path,
}

impl<'a> ItemGenerator<'a> {
    /// Generate header for Daml LF v2 package
    pub fn generate_package_header(package_id: &PackageId, package: Package<'_>) -> Vec<syn::Item> {
        let package_id = package_id.as_str();

        let metadata = package.metadata();
        let name = metadata.name();
        let version = metadata.version();

        let package_id = syn::parse_quote! {
            pub const PACKAGE_ID: ::canton::types::PackageId =
                ::canton::types::PackageId::new_unchecked(#package_id);
        };
        let package_name = syn::parse_quote! {
            pub const PACKAGE_NAME: ::canton::types::PackageName =
                ::canton::types::PackageName::new_unchecked(#name);
        };
        let package_version = syn::parse_quote! {
            pub const PACKAGE_VERSION: &str = #version;
        };
        vec![package_id, package_name, package_version]
    }

    /// Generate header for Daml LF v2 module
    pub fn generate_module_header(module: Module<'_>) -> Vec<syn::Item> {
        let name = module.name().into_iter().collect::<Vec<_>>().join(".");
        let module_name = syn::parse_quote! {
            pub const MODULE_NAME: &str = #name;
        };
        vec![module_name]
    }

    /// Generate item from a Daml LF v2 definition
    pub fn generate(definition: Definition<'a>, ctx: &'a GenCtx) -> syn::Item {
        let module_name = definition.module().name();
        let def = definition.definition();
        let definition_name = def.name();
        let package_id = definition.package_id();
        let package_ident = &ctx.packages()[package_id].ident;
        let module_path =
            path::generate_module_path(module_name.iter().chain(definition_name.base().iter()));
        let type_generator = TypeGenerator::new(ctx, package_ident, module_path.clone());
        let deps_resolver = DepsResolver::new(ctx, package_id);

        let generator = Self {
            ctx,
            deps_resolver,
            type_generator,
            definition,
            package_id,
            module_path,
        };

        generator.generate_from_definition(def)
    }

    /// Generate item from definition
    fn generate_from_definition(&self, definition: Def<'_>) -> syn::Item {
        let attrs = self.generate_attrs(definition);
        self.generate_from_data_type(definition.data_type(), attrs)
    }

    /// Generate item from data type definition
    fn generate_from_data_type(
        &self,
        dt: DefDataType<'_>,
        attrs: Vec<syn::Attribute>,
    ) -> syn::Item {
        let name = dt.name().tail();

        let cons = dt.data_cons();
        let params = dt.params();

        debug!(name, ?cons, ?params, "Entering data type definition");

        let entity_id = ident::generate_camel_ident(name);
        let generics = Self::generate_generics(params);

        match cons {
            DataCons::Record(fields) => {
                syn::Item::Struct(self.generate_struct(entity_id, fields, generics, attrs))
            }
            DataCons::Variant(fields) => {
                syn::Item::Enum(self.generate_enum(entity_id, fields, generics, attrs))
            }
            DataCons::Enum(ctors) => {
                syn::Item::Enum(self.generate_enum_unit_only(entity_id, ctors, generics, attrs))
            }
            DataCons::Interface => {
                syn::Item::Struct(self.generate_interface_struct(entity_id, generics, attrs))
            }
        }
    }

    /// Generate Rust enum (unit-only) declaration `pub enum Name { Variant, Variant1 ... }`
    fn generate_enum_unit_only(
        &self,
        ident: syn::Ident,
        enum_ctrs: EnumConstructors<'a>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> syn::ItemEnum {
        syn::ItemEnum {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            enum_token: Default::default(),
            ident,
            generics,
            brace_token: Default::default(),
            variants: enum_ctrs
                .constructors()
                .into_iter()
                .map(|variant| self.generate_enum_unit_only_variant(variant))
                .collect(),
        }
    }

    /// Generates a Rust enum unit variant (no fields)
    fn generate_enum_unit_only_variant(&self, variant: &'a str) -> syn::Variant {
        let ident = ident::generate_camel_ident(variant);
        let value_attribute = Self::generate_name_attr(variant);
        syn::Variant {
            attrs: vec![value_attribute],
            ident,
            fields: syn::Fields::Unit,
            discriminant: None,
        }
    }

    /// Generate Rust enum declaration `pub enum Name { Variant(Type), ... }`
    fn generate_enum(
        &self,
        ident: syn::Ident,
        fields: Fields<'_>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> syn::ItemEnum {
        let variants = fields
            .fields()
            .into_iter()
            .map(|field| self.generate_enum_variant(field, &generics))
            .collect();
        syn::ItemEnum {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            ident,
            generics,
            variants,
            enum_token: Default::default(),
            brace_token: Default::default(),
        }
    }

    /// Generate Rust enum variant with a single unnamed field: `#[name = "Variant"] Variant(Type)`
    fn generate_enum_variant(
        &self,
        field: FieldWithType<'_>,
        generics: &syn::Generics,
    ) -> syn::Variant {
        let field_name = field.field();
        let mut attrs = vec![Self::generate_name_attr(field_name)];
        let maybe_edge = self.type_references_definition(field.type_());
        if maybe_edge.is_some() {
            attrs.push(parse_quote! { #[value(omit_bound)] });
        }
        let ident = ident::generate_camel_ident(field_name);
        let fields = self.generate_enum_fields(field, generics, maybe_edge);
        syn::Variant {
            attrs,
            ident,
            fields,
            discriminant: None,
        }
    }

    /// Generate single unnamed field `Variant(Type)`
    fn generate_enum_fields(
        &self,
        field: FieldWithType<'_>,
        generics: &syn::Generics,
        edge: Option<Edge>,
    ) -> syn::Fields {
        let type_ = field.type_();
        let mut field_type = self.type_generator.generate(type_);
        if !matches!(type_, Type::Var(_)) {
            Self::protect_from_collision_with_generics(&mut field_type, generics);
        }
        if edge == Some(Edge::Inline) {
            field_type = parse_quote! { ::std::boxed::Box<#field_type> };
        }

        let field = syn::Field {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            modifiers: Default::default(),
            ident: None,
            ty: field_type,
            colon_token: Some(Default::default()),
            default: Default::default(),
        };

        syn::Fields::Unnamed(syn::FieldsUnnamed {
            paren_token: Default::default(),
            unnamed: [field].into_iter().collect(),
        })
    }

    /// Generate Rust struct declaration `pub struct Name<...> { name: Type, ... }`
    fn generate_struct(
        &self,
        ident: syn::Ident,
        fields: Fields<'_>,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> syn::ItemStruct {
        let fields = self.generate_struct_fields(fields, &generics);
        syn::ItemStruct {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            struct_token: Default::default(),
            ident,
            generics,
            fields,
            semi_token: None,
        }
    }

    /// Generate names fields for struct `pub name1: Type1, pub name2: Type2, ...`
    fn generate_struct_fields(&self, fields: Fields<'_>, generics: &syn::Generics) -> syn::Fields {
        syn::Fields::Named(syn::FieldsNamed {
            brace_token: Default::default(),
            named: fields
                .fields()
                .into_iter()
                .map(|field| self.generate_struct_field(field, generics))
                .collect(),
        })
    }

    /// Generate a field in struct: `#[name = "Name"] pub name: Type,`
    fn generate_struct_field(
        &self,
        field: FieldWithType<'_>,
        generics: &syn::Generics,
    ) -> syn::Field {
        trace!(?field, "Entering field");
        let field_name = field.field();
        let mut attrs = vec![Self::generate_name_attr(field_name)];
        let maybe_edge = self.type_references_definition(field.type_());
        if maybe_edge.is_some() {
            attrs.push(parse_quote! { #[value(omit_bound)] });
        }
        let field_id = ident::generate_snake_ident(field_name);
        let type_ = field.type_();
        let mut field_type = self.type_generator.generate(type_);
        if !matches!(type_, Type::Var(_)) {
            Self::protect_from_collision_with_generics(&mut field_type, generics);
        }

        if maybe_edge == Some(Edge::Inline) {
            field_type = parse_quote! { ::std::boxed::Box<#field_type> };
        }

        syn::Field {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            modifiers: Default::default(),
            ident: Some(field_id),
            colon_token: Some(Default::default()),
            ty: field_type,
            default: None,
        }
    }

    /// Generate unit struct representing interface type `pub struct MyInterface;`
    fn generate_interface_struct(
        &self,
        ident: syn::Ident,
        generics: syn::Generics,
        attrs: Vec<syn::Attribute>,
    ) -> syn::ItemStruct {
        syn::ItemStruct {
            attrs,
            vis: syn::Visibility::Public(Default::default()),
            struct_token: Default::default(),
            ident,
            generics,
            fields: syn::Fields::Unit,
            semi_token: Some(Default::default()),
        }
    }

    /// Generate name attribute `#[name = "ABC"]`
    fn generate_name_attr(name: &str) -> syn::Attribute {
        syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: syn::parse_quote! { name = #name },
            pound_token: Default::default(),
            bracket_token: Default::default(),
        }
    }

    /// Note: this doesn't account multi-level recursion. Multi-level recursion is currently not
    /// supported
    fn type_references_definition(&self, type_: Type<'a>) -> Option<Edge> {
        self.deps_resolver
            .deps_of_type(type_)
            .into_iter()
            .filter_map(|(def, edge)| {
                if def == crate::ir::Definition::V2(self.definition) {
                    Some(edge)
                } else {
                    None
                }
            })
            .max()
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
    fn generate_attrs(&self, definition: Def<'_>) -> Vec<syn::Attribute> {
        let name = definition.name().into_iter().collect::<Vec<_>>().join(".");

        let krate = self.ctx.paths().root();
        let value_v2 = self.ctx.paths().value_v2();
        let types = self.ctx.paths().types();
        let module_name = definition
            .module()
            .name()
            .into_iter()
            .collect::<Vec<_>>()
            .join(".");
        let package_root_path = path::super_repeat(self.module_path.segments.len());

        // TODO: conditionally add: Copy, Eq, Hash, PartialOrd, Ord?

        // #[derive(HasIdentifier, Value, ...)]
        let derive_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: parse_quote! {
                derive(Clone, Debug, PartialEq, #value_v2::HasIdentifier, #value_v2::Value)
            },
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        // #[value(...)]
        let value_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: parse_quote! { value(crate_path = #krate) },
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        // #[identifier(...)]
        let identifier_attr = syn::Attribute {
            style: syn::AttrStyle::Outer,
            meta: parse_quote! { identifier(
                package_id = #package_root_path::PACKAGE_ID,
                package_name = #package_root_path::PACKAGE_NAME,
                module = #module_name,
                name = #name,
                crate_path = #krate,
            ) },
            pound_token: Default::default(),
            bracket_token: Default::default(),
        };

        let mut attrs = vec![derive_attr, value_attr, identifier_attr];

        match definition {
            Def::Template { template, .. } => {
                // #[derive(Template)]
                attrs.push(syn::Attribute {
                    style: syn::AttrStyle::Outer,
                    meta: parse_quote! { derive(#types::Template) },
                    pound_token: Default::default(),
                    bracket_token: Default::default(),
                });

                // #[template(...)]
                let mut args = vec![quote! { crate_path = #krate }];

                args.extend(template.implements().into_iter().map(|impl_| {
                    let iface = self.type_generator.generate_type_path(impl_.interface());
                    quote! { implements = #iface }
                }));

                if let Some(key) = template.key() {
                    let key_type = self.type_generator.generate(key.type_());
                    args.push(quote! { key = #key_type })
                }

                // #[template(choice(...))]
                args.extend(template.choices().into_iter().map(|choice| {
                    let argument = self.type_generator.generate(choice.arg_binder().type_());
                    let result = self.type_generator.generate(choice.ret_type());
                    let consuming = choice.consuming();
                    let name = choice.name();
                    quote! { choice(
                        argument = #argument,
                        result = #result,
                        consuming = #consuming,
                        name = #name,
                    ) }
                }));

                let meta = parse_quote! { template(#(#args),*) };
                attrs.push(syn::Attribute {
                    style: syn::AttrStyle::Outer,
                    meta,
                    pound_token: Default::default(),
                    bracket_token: Default::default(),
                });
            }

            Def::Interface { interface, .. } => {
                // #[derive(Interface)]
                attrs.push(syn::Attribute {
                    style: syn::AttrStyle::Outer,
                    meta: parse_quote! { derive(#types::Interface) },
                    pound_token: Default::default(),
                    bracket_token: Default::default(),
                });

                // #[interface(...)]
                let view = self.type_generator.generate(interface.view());
                let mut args = vec![quote! { crate_path = #krate }, quote! { view = #view }];

                args.extend(interface.requires().into_iter().map(|ty_con_id| {
                    let iface = self.type_generator.generate_type_path(ty_con_id);
                    quote! { requires = #iface }
                }));

                // #[interface(choice(...))]
                args.extend(interface.choices().into_iter().map(|choice| {
                    let argument = self.type_generator.generate(choice.arg_binder().type_());
                    let result = self.type_generator.generate(choice.ret_type());
                    let consuming = choice.consuming();
                    let name = choice.name();
                    quote! { choice(
                        argument = #argument,
                        result = #result,
                        consuming = #consuming,
                        name = #name,
                    ) }
                }));

                let meta = parse_quote! { interface(#(#args),*) };
                attrs.push(syn::Attribute {
                    style: syn::AttrStyle::Outer,
                    meta,
                    pound_token: Default::default(),
                    bracket_token: Default::default(),
                });
            }

            // No special attrs needed for data types
            _ => {}
        };

        if let Some(additional_attrs) = self.ctx.type_attributes().get_entity(
            self.package_id,
            &dotted_name_to_owned(&definition.module().name()),
            &dotted_name_to_owned(&definition.name()),
        ) {
            attrs.extend_from_slice(additional_attrs);
        }

        attrs
    }

    /// Generate generic type parameters for an item (`<A, B, C>`)
    fn generate_generics(params: Vec<TypeVarWithKind<'_>>) -> syn::Generics {
        let params = params
            .into_iter()
            .map(Self::generate_generic_param)
            .collect::<Vec<_>>();

        syn::Generics {
            params: params.into_iter().collect(),
            lt_token: Some(Default::default()),
            gt_token: Some(Default::default()),
            where_clause: None,
        }
    }

    /// Generate generic type parameter
    fn generate_generic_param(param: TypeVarWithKind<'_>) -> syn::GenericParam {
        let kind = param.kind();
        let ident = ident::generate_camel_ident(param.var());
        match kind {
            Kind::Star => syn::GenericParam::Type(syn::TypeParam {
                ident,
                attrs: Vec::new(),
                colon_token: None,
                bounds: Default::default(),
                default: None,
            }),
            Kind::Arrow(arrow) => todo!("Arrow kind is not supported yet: {arrow:?}"),
            Kind::Nat => todo!("Nat kind is not supported yet"),
        }
    }

    /// If type was resolved as just an identifier, but it's not a type var, it may collide with
    /// generic type param with the same identifier. This function inserts `self` before the type
    /// identifier if needed.
    fn protect_from_collision_with_generics(field_type: &mut syn::Type, generics: &syn::Generics) {
        if let syn::Type::Path(type_path) = field_type
            && type_path.path.segments.len() == 1
        {
            let type_ident = &type_path.path.segments[0].ident;
            if generics
                .type_params()
                .find(|tp| &tp.ident == type_ident)
                .is_some()
            {
                // if there is a colliding type parameter, prepend `self`
                type_path
                    .path
                    .segments
                    .insert(0, syn::token::SelfValue::default().into());
            }
        }
    }
}

fn dotted_name_to_owned(name: &DottedName<'_>) -> canton_types::DottedName {
    let base = name
        .base()
        .iter()
        .map(ToString::to_string)
        .map(Name::new_unchecked)
        .collect();
    let tail = Name::new_unchecked(name.tail().to_string());
    canton_types::DottedName::from_segments(NonEmpty { base, tail })
}
