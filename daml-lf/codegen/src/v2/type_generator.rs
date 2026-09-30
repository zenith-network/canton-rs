use canton_types::PackageId;
use daml_lf::v2::sealed::{
    BuiltinType, SelfOrImportedPackageId, Type, TypeConId,
    type_::{Builtin, Con, TApp},
};
use syn::parse_quote;
use tracing::trace;

use crate::{generator::GenCtx, ident, path};

/// Type generator for Daml LF v2
#[derive(Clone, Debug)]
pub struct TypeGenerator<'a> {
    ctx: &'a GenCtx<'a>,
    self_package_ident: &'a syn::Ident,
    module_path: syn::Path,
}

impl<'a> TypeGenerator<'a> {
    pub fn new(
        ctx: &'a GenCtx,
        self_package_ident: &'a syn::Ident,
        module_path: syn::Path,
    ) -> Self {
        Self {
            ctx,
            self_package_ident,
            module_path,
        }
    }

    /// Generate type
    pub fn generate(&self, type_: Type<'_>) -> syn::Type {
        match type_ {
            Type::Var(var) => {
                let var = ident::generate_camel_ident(var.var());
                parse_quote! { #var }
            }
            Type::Con(con) => syn::Type::Path(self.gen_con(con)),
            Type::Builtin(builtin) => self.gen_builtin(builtin),
            // We emit `!` here because this type will be erased from parent numeric anyway
            Type::Nat => syn::Type::Never(syn::TypeNever {
                attrs: Default::default(),
                bang_token: Default::default(),
            }),
            Type::Tapp(tapp) => syn::Type::Path(self.gen_tapp(tapp)),
        }
    }

    /// Generate type path from `TypeConId`
    ///
    /// Note: generated path always has no path arguments at the end. Generated path is also always
    /// non-empty, because it contains at least the entity identifier.
    ///
    /// ## Example
    ///
    /// For `my-package@1.0.0:Some.Module:SomeType` this generates something like
    ///
    /// ```ignore
    /// super::super::some::module::SomeType
    /// ```
    pub fn generate_type_path(&self, ty_con_id: TypeConId<'_>) -> syn::TypePath {
        let name = ty_con_id.name();
        trace!(?name, "Generating type constructor");

        let module_id = ty_con_id.module();
        let module_name = module_id.module_name();
        let target_module_path =
            path::generate_module_path(module_name.iter().chain(name.base().iter()));

        // This is a relative path from current module to the target type module
        // Using shortest path here simplifies generated code
        let mut path = match module_id.package_id() {
            SelfOrImportedPackageId::SelfPackageId => {
                path::find_shortest_path(&self.module_path, &target_module_path)
            }
            SelfOrImportedPackageId::ImportedPackageId(package_id) => {
                let package_id = PackageId::new_unchecked_owned(package_id.to_string());
                let package_ident = self.ctx.packages()[&package_id].clone().ident;
                let mut target_path = syn::Path::from(package_ident);
                target_path.segments.extend(target_module_path.segments);

                let mut self_path = syn::Path::from(self.self_package_ident.clone());
                self_path.segments.extend(self.module_path.segments.clone());
                path::find_shortest_path(&self_path, &target_path)
            }
        };

        let type_id = ident::generate_camel_ident(name.tail());
        path.segments.push(syn::PathSegment {
            ident: type_id,
            arguments: syn::PathArguments::None,
        });

        syn::TypePath {
            attrs: Default::default(),
            qself: None,
            path,
        }
    }

    /// Generate type path from Daml type application
    fn gen_tapp(&self, tapp: TApp<'_>) -> syn::TypePath {
        let lhs = tapp.lhs();
        let type_path = self.generate(lhs);

        let syn::Type::Path(mut type_path) = type_path else {
            unreachable!("generated an unexpected type for lhs of a type application")
        };

        // We explicitly omit type args of numeric
        if let Type::Builtin(bi) = lhs
            && matches!(bi.type_(), BuiltinType::Numeric)
        {
            return type_path;
        }

        // Safety: Self::generate cannot produce an empty type path
        let last = type_path
            .path
            .segments
            .last_mut()
            .expect("should not generate an empty type path");

        let arg_type = self.generate(tapp.rhs());

        match &mut last.arguments {
            syn::PathArguments::None => {
                last.arguments =
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: Default::default(),
                        args: [syn::GenericArgument::Type(arg_type)].into_iter().collect(),
                        gt_token: Default::default(),
                    });
            }
            syn::PathArguments::AngleBracketed(args) => {
                args.args.push(syn::GenericArgument::Type(arg_type));
            }
            syn::PathArguments::Parenthesized(_) => unreachable!(),
        }

        type_path
    }

    /// Generate type path from Daml type constructor application
    fn gen_con(&self, con: Con<'_>) -> syn::TypePath {
        let tycon = con.tycon();

        let mut path = self.generate_type_path(tycon);

        let args = con.args();
        if !args.is_empty() {
            let generic_args = self.gen_generic_args(&args);
            if let Some(segment) = path.path.segments.last_mut() {
                segment.arguments = syn::PathArguments::AngleBracketed(generic_args);
            }
        }

        path
    }

    /// Generate generic argument from a type (for path segment)
    fn gen_generic_arg(&self, type_: Type<'_>) -> syn::GenericArgument {
        syn::GenericArgument::Type(self.generate(type_))
    }

    /// Generate generic arguments of a path segment: `<my::Type, other::Type>`
    fn gen_generic_args(&self, types: &[Type<'_>]) -> syn::AngleBracketedGenericArguments {
        syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: types
                .iter()
                .map(|type_| self.gen_generic_arg(*type_))
                .collect(),
            gt_token: Default::default(),
        }
    }

    /// Generate type from Daml built-in type
    fn gen_builtin(&self, builtin: Builtin<'_>) -> syn::Type {
        let types = self.ctx.paths().types();
        let type_ = builtin.type_();

        let mut result = match type_ {
            // Rust built-in types
            BuiltinType::Unit => parse_quote! { () },
            BuiltinType::Bool => parse_quote! { bool },
            BuiltinType::Int64 => parse_quote! { i64 },

            // std types
            // TODO: it's probably better to use core + alloc and re-export those from runtime crate
            BuiltinType::Text => parse_quote! { ::std::string::String },
            BuiltinType::Optional => parse_quote! { ::std::option::Option },
            BuiltinType::List => parse_quote! { ::std::vec::Vec },
            BuiltinType::Genmap => parse_quote! { ::std::collections::BTreeMap },

            // custom runtime types
            BuiltinType::Date => parse_quote! { #types::Date },
            BuiltinType::Timestamp => parse_quote! { #types::Timestamp },
            BuiltinType::Numeric => parse_quote! { #types::Numeric },
            BuiltinType::Party => parse_quote! { #types::PartyId },
            BuiltinType::ContractId => parse_quote! { #types::ContractId },
            BuiltinType::Textmap => parse_quote! { #types::TextMap },
            BuiltinType::Bignumeric => todo!("BuiltinType::Bignumeric"),
            BuiltinType::RoundingMode => todo!("BuiltinType::RoundingMode"),

            BuiltinType::Any => unreachable!("BuiltinType::Any"),
            BuiltinType::AnyException => unreachable!("BuiltinType::AnyException"),
            BuiltinType::TypeRep => unreachable!("BuiltinType::TypeRep"),
            BuiltinType::Arrow => unreachable!("BuiltinType::Arrow"),
            BuiltinType::Update => unreachable!("BuiltinType::Update"),
            BuiltinType::FailureCategory => unreachable!("BuiltinType::FailureCategory"),
        };

        let args = builtin.args();

        if !args.is_empty() {
            // We assume that the package is valid. Then if there are some type args to apply,
            // this type cannot be `()`, `bool` or `u64`. In all other cases it is a type path.
            // Additionally we intentionally skip numerics, because their args must be skipped - in
            // Rust we represent them with a type with no generic args.
            if let syn::Type::Path(type_path) = &mut result
                && !matches!(type_, BuiltinType::Numeric)
            {
                let generic_args = self.gen_generic_args(&args);
                // Safety: we just generated this path - in all cases it's not empty
                let last = type_path.path.segments.last_mut().unwrap();
                match &mut last.arguments {
                    syn::PathArguments::None => {
                        last.arguments = syn::PathArguments::AngleBracketed(generic_args);
                    }
                    syn::PathArguments::AngleBracketed(angle_brackets) => {
                        // If there are already some args there (currently it's only for Textmap),
                        // we extend them
                        angle_brackets.args.extend(generic_args.args);
                    }
                    // None of out paths has this
                    syn::PathArguments::Parenthesized(_) => unreachable!(),
                }
            } else {
                unreachable!("attempt to apply type args to wrong builtin type {builtin:?}")
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canton_types::PackageId;
    use daml_lf::{
        package::{SealedPackage as OuterSealedPackage, VersionedSealedPackage},
        proto::com::digitalasset::daml::lf::archive::v2::Package as ProtoPackage,
        v2::{
            builder::{
                BuiltinType, DataTypeBuilder, Field, Kind, ModuleBuilder, PackageBuilder,
                Type as TypeBuilder, TypeConRef, TypeParameter,
            },
            sealed::{Package as SealedPackage, def_data_type::DataCons},
        },
    };
    use daml_lf_version::{MinorVersion, Version};
    use pretty_assertions::assert_eq;
    use quote::ToTokens;
    use rstest::{fixture, rstest};

    use crate::{
        dispatcher::Packages, generator::GenCtx, package_ident_generator::PackageWithIdent,
        type_attributes::TypeAttributes,
    };

    use super::TypeGenerator;

    /// Builds packages equivalent to the following Daml declarations:
    ///
    /// - `dependency`:
    ///
    ///   ```daml
    ///   module Dep where
    ///
    ///   data Payload = Payload
    ///   data Outer.Payload = Payload
    ///   data Generic a b = Generic with
    ///     first : a
    ///     second : b
    ///   ```
    ///
    /// - `main-package`:
    ///
    ///   ```daml
    ///   module Main where
    ///
    ///   data Generic a b = Generic with
    ///     first : a
    ///     second : b
    ///
    ///   data Target = Target
    ///
    ///   data Subject a = Subject with
    ///     variable : a
    ///     constructor : Target
    ///     generic_inline : Generic Text Int
    ///     generic_tapp : Generic Text Int
    ///     generic_variable_tapp : Generic a (Optional a)
    ///     nested_inline : Optional (Generic Text Int)
    ///     nested_tapp : Optional (Generic Text Int)
    ///     unit : ()
    ///     bool : Bool
    ///     int64 : Int
    ///     date : Date
    ///     timestamp : Time
    ///     party : Party
    ///     text : Text
    ///     numeric_inline : Numeric 10
    ///     numeric_tapp : Numeric 10
    ///     contract_id_inline : ContractId Target
    ///     contract_id_tapp : ContractId Target
    ///     contract_id_unit_tapp : ContractId ()
    ///     contract_id_variable_tapp : ContractId a
    ///     contract_id_function_tapp : ContractId (Int -> Text)
    ///     optional_inline : Optional Target
    ///     optional_tapp : Optional Target
    ///     optional_numeric_tapp : Optional (Numeric 10)
    ///     list_inline : [Target]
    ///     list_tapp : [Target]
    ///     list_optional_tapp : [Optional Target]
    ///     textmap_inline : TextMap Int
    ///     textmap_tapp : TextMap Int
    ///     textmap_generic_tapp : TextMap (Generic Text Int)
    ///     genmap_inline : GenMap Text Int
    ///     genmap_tapp : GenMap Text Int
    ///     genmap_nested_tapp : GenMap Text [Target]
    ///     imported : Dep.Payload
    ///     imported_generic_inline : Dep.Generic Text Int
    ///     imported_generic_tapp : Dep.Generic Text Int
    ///
    ///   data Outer.Inner = Inner
    ///   data PathSubject = PathSubject with
    ///     same_module : Main.Target
    ///     other_module : Other.Target
    ///     nested_module : Nested.Module.Target
    ///     nested_type : Main.Outer.Inner
    ///     imported : Dep.Payload
    ///     imported_nested : Dep.Outer.Payload
    ///   ```
    ///
    ///   ```daml
    ///   module Other where
    ///
    ///   data Target = Target
    ///   ```
    ///
    ///   ```daml
    ///   module Nested.Module where
    ///
    ///   data Target = Target
    ///   ```
    ///
    /// The `_inline` fields use the pre-LF 2.2 inline argument encoding. The `_tapp` fields use
    /// explicit `TApp` nodes for the same source-level types.
    #[fixture]
    fn packages() -> (ProtoPackage, ProtoPackage) {
        let dependency_id = PackageId::new_unchecked("dependency");
        let dependency_generic = DataTypeBuilder::record(
            "Generic",
            [
                Field::new("first", TypeBuilder::var("a")),
                Field::new("second", TypeBuilder::var("b")),
            ],
        )
        .type_parameter(TypeParameter::new("a", Kind::Star))
        .type_parameter(TypeParameter::new("b", Kind::Star));
        let dependency = PackageBuilder::new("dependency", "1.0.0")
            .module(
                ModuleBuilder::new("Dep")
                    .data_type(DataTypeBuilder::record("Payload", []))
                    .data_type(DataTypeBuilder::record("Outer.Payload", []))
                    .data_type(dependency_generic),
            )
            .build();

        let generic = DataTypeBuilder::record(
            "Generic",
            [
                Field::new("first", TypeBuilder::var("a")),
                Field::new("second", TypeBuilder::var("b")),
            ],
        )
        .type_parameter(TypeParameter::new("a", Kind::Star))
        .type_parameter(TypeParameter::new("b", Kind::Star));
        let target = DataTypeBuilder::record("Target", []);
        let generic_inline = TypeBuilder::con_with_args(
            TypeConRef::local("Main", "Generic"),
            [TypeBuilder::text(), TypeBuilder::int64()],
        );
        let generic_tapp = TypeBuilder::tapp(
            TypeBuilder::tapp(TypeBuilder::local("Main", "Generic"), TypeBuilder::text()),
            TypeBuilder::int64(),
        );
        let optional_target_tapp = TypeBuilder::tapp(
            TypeBuilder::builtin(BuiltinType::Optional),
            TypeBuilder::local("Main", "Target"),
        );
        let numeric_tapp = TypeBuilder::tapp(
            TypeBuilder::builtin(BuiltinType::Numeric),
            TypeBuilder::nat(10),
        );
        let function_tapp = TypeBuilder::tapp(
            TypeBuilder::tapp(
                TypeBuilder::builtin(BuiltinType::Arrow),
                TypeBuilder::int64(),
            ),
            TypeBuilder::text(),
        );
        let imported_generic_inline = TypeBuilder::con_with_args(
            TypeConRef::imported(&dependency_id, "Dep", "Generic"),
            [TypeBuilder::text(), TypeBuilder::int64()],
        );
        let imported_generic_tapp = TypeBuilder::tapp(
            TypeBuilder::tapp(
                TypeBuilder::imported(&dependency_id, "Dep", "Generic"),
                TypeBuilder::text(),
            ),
            TypeBuilder::int64(),
        );
        let subject = DataTypeBuilder::record(
            "Subject",
            [
                Field::new("variable", TypeBuilder::var("a")),
                Field::new("constructor", TypeBuilder::local("Main", "Target")),
                Field::new("generic_inline", generic_inline.clone()),
                Field::new("generic_tapp", generic_tapp.clone()),
                Field::new(
                    "generic_variable_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::tapp(
                            TypeBuilder::local("Main", "Generic"),
                            TypeBuilder::var("a"),
                        ),
                        TypeBuilder::tapp(
                            TypeBuilder::builtin(BuiltinType::Optional),
                            TypeBuilder::var("a"),
                        ),
                    ),
                ),
                Field::new(
                    "nested_inline",
                    TypeBuilder::builtin_with_args(BuiltinType::Optional, [generic_inline.clone()]),
                ),
                Field::new(
                    "nested_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::Optional),
                        generic_tapp.clone(),
                    ),
                ),
                Field::new("unit", TypeBuilder::unit()),
                Field::new("bool", TypeBuilder::bool()),
                Field::new("int64", TypeBuilder::int64()),
                Field::new("date", TypeBuilder::builtin(BuiltinType::Date)),
                Field::new("timestamp", TypeBuilder::builtin(BuiltinType::Timestamp)),
                Field::new("party", TypeBuilder::party()),
                Field::new("text", TypeBuilder::text()),
                Field::new(
                    "numeric_inline",
                    TypeBuilder::builtin_with_args(BuiltinType::Numeric, [TypeBuilder::nat(10)]),
                ),
                Field::new("numeric_tapp", numeric_tapp.clone()),
                Field::new(
                    "contract_id_inline",
                    TypeBuilder::builtin_with_args(
                        BuiltinType::ContractId,
                        [TypeBuilder::local("Main", "Target")],
                    ),
                ),
                Field::new(
                    "contract_id_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::ContractId),
                        TypeBuilder::local("Main", "Target"),
                    ),
                ),
                Field::new(
                    "contract_id_unit_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::ContractId),
                        TypeBuilder::unit(),
                    ),
                ),
                Field::new(
                    "contract_id_variable_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::ContractId),
                        TypeBuilder::var("a"),
                    ),
                ),
                Field::new(
                    "contract_id_function_tapp",
                    TypeBuilder::tapp(TypeBuilder::builtin(BuiltinType::ContractId), function_tapp),
                ),
                Field::new(
                    "optional_inline",
                    TypeBuilder::builtin_with_args(
                        BuiltinType::Optional,
                        [TypeBuilder::local("Main", "Target")],
                    ),
                ),
                Field::new("optional_tapp", optional_target_tapp.clone()),
                Field::new(
                    "optional_numeric_tapp",
                    TypeBuilder::tapp(TypeBuilder::builtin(BuiltinType::Optional), numeric_tapp),
                ),
                Field::new(
                    "list_inline",
                    TypeBuilder::builtin_with_args(
                        BuiltinType::List,
                        [TypeBuilder::local("Main", "Target")],
                    ),
                ),
                Field::new(
                    "list_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::List),
                        TypeBuilder::local("Main", "Target"),
                    ),
                ),
                Field::new(
                    "list_optional_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::List),
                        optional_target_tapp,
                    ),
                ),
                Field::new(
                    "textmap_inline",
                    TypeBuilder::builtin_with_args(BuiltinType::Textmap, [TypeBuilder::int64()]),
                ),
                Field::new(
                    "textmap_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::builtin(BuiltinType::Textmap),
                        TypeBuilder::int64(),
                    ),
                ),
                Field::new(
                    "textmap_generic_tapp",
                    TypeBuilder::tapp(TypeBuilder::builtin(BuiltinType::Textmap), generic_tapp),
                ),
                Field::new(
                    "genmap_inline",
                    TypeBuilder::builtin_with_args(
                        BuiltinType::Genmap,
                        [TypeBuilder::text(), TypeBuilder::int64()],
                    ),
                ),
                Field::new(
                    "genmap_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::tapp(
                            TypeBuilder::builtin(BuiltinType::Genmap),
                            TypeBuilder::text(),
                        ),
                        TypeBuilder::int64(),
                    ),
                ),
                Field::new(
                    "genmap_nested_tapp",
                    TypeBuilder::tapp(
                        TypeBuilder::tapp(
                            TypeBuilder::builtin(BuiltinType::Genmap),
                            TypeBuilder::text(),
                        ),
                        TypeBuilder::tapp(
                            TypeBuilder::builtin(BuiltinType::List),
                            TypeBuilder::local("Main", "Target"),
                        ),
                    ),
                ),
                Field::new(
                    "imported",
                    TypeBuilder::imported(&dependency_id, "Dep", "Payload"),
                ),
                Field::new("imported_generic_inline", imported_generic_inline),
                Field::new("imported_generic_tapp", imported_generic_tapp),
            ],
        )
        .type_parameter(TypeParameter::new("a", Kind::Star));

        let path_subject = DataTypeBuilder::record(
            "PathSubject",
            [
                Field::new("same_module", TypeBuilder::local("Main", "Target")),
                Field::new("other_module", TypeBuilder::local("Other", "Target")),
                Field::new(
                    "nested_module",
                    TypeBuilder::local("Nested.Module", "Target"),
                ),
                Field::new("nested_type", TypeBuilder::local("Main", "Outer.Inner")),
                Field::new(
                    "imported",
                    TypeBuilder::imported(&dependency_id, "Dep", "Payload"),
                ),
                Field::new(
                    "imported_nested",
                    TypeBuilder::imported(&dependency_id, "Dep", "Outer.Payload"),
                ),
            ],
        );
        let main = PackageBuilder::new("main-package", "1.0.0")
            .module(
                ModuleBuilder::new("Main")
                    .data_type(generic)
                    .data_type(target)
                    .data_type(DataTypeBuilder::record("Outer.Inner", []))
                    .data_type(subject)
                    .data_type(path_subject),
            )
            .module(ModuleBuilder::new("Other").data_type(DataTypeBuilder::record("Target", [])))
            .module(
                ModuleBuilder::new("Nested.Module")
                    .data_type(DataTypeBuilder::record("Target", [])),
            )
            .build();

        (dependency, main)
    }

    #[rstest]
    #[case::variable("variable", "A")]
    #[case::constructor("constructor", "Target")]
    #[case::imported("imported", "super::super::dependency::dep::Payload")]
    #[case::imported_generic_inline(
        "imported_generic_inline",
        "super::super::dependency::dep::Generic<::std::string::String, i64>"
    )]
    #[case::imported_generic_tapp(
        "imported_generic_tapp",
        "super::super::dependency::dep::Generic<::std::string::String, i64>"
    )]
    #[case::generic_inline("generic_inline", "Generic<::std::string::String, i64>")]
    #[case::generic_tapp("generic_tapp", "Generic<::std::string::String, i64>")]
    #[case::generic_variable_tapp("generic_variable_tapp", "Generic<A, ::std::option::Option<A>>")]
    #[case::nested_inline(
        "nested_inline",
        "::std::option::Option<Generic<::std::string::String, i64>>"
    )]
    #[case::nested_tapp(
        "nested_tapp",
        "::std::option::Option<Generic<::std::string::String, i64>>"
    )]
    #[case::unit("unit", "()")]
    #[case::bool("bool", "bool")]
    #[case::int64("int64", "i64")]
    #[case::date("date", "::canton::types::Date")]
    #[case::timestamp("timestamp", "::canton::types::Timestamp")]
    #[case::party("party", "::canton::types::PartyId")]
    #[case::text("text", "::std::string::String")]
    #[case::numeric_inline("numeric_inline", "::canton::types::Numeric")]
    #[case::numeric_tapp("numeric_tapp", "::canton::types::Numeric")]
    #[case::contract_id_inline("contract_id_inline", "::canton::types::ContractId<Target>")]
    #[case::contract_id_tapp("contract_id_tapp", "::canton::types::ContractId<Target>")]
    #[case::contract_id_unit_tapp("contract_id_unit_tapp", "::canton::types::ContractId<()>")]
    #[case::contract_id_variable_tapp(
        "contract_id_variable_tapp",
        "::canton::types::ContractId<A>"
    )]
    // TODO: Revisit whether codegen should support this legal but unusual phantom
    //       ContractId argument. Currently it's a valid Daml code but doesn't make any sense to me.
    #[ignore]
    #[case::contract_id_function_tapp(
        "contract_id_function_tapp",
        "::canton::types::ContractId<fn(i64) -> ::std::string::String>"
    )]
    #[case::optional_inline("optional_inline", "::std::option::Option<Target>")]
    #[case::optional_tapp("optional_tapp", "::std::option::Option<Target>")]
    #[case::optional_numeric_tapp(
        "optional_numeric_tapp",
        "::std::option::Option<::canton::types::Numeric>"
    )]
    #[case::list_inline("list_inline", "::std::vec::Vec<Target>")]
    #[case::list_tapp("list_tapp", "::std::vec::Vec<Target>")]
    #[case::list_optional_tapp(
        "list_optional_tapp",
        "::std::vec::Vec<::std::option::Option<Target>>"
    )]
    #[case::textmap_inline("textmap_inline", "::canton::types::TextMap<i64>")]
    #[case::textmap_tapp("textmap_tapp", "::canton::types::TextMap<i64>")]
    #[case::textmap_generic_tapp(
        "textmap_generic_tapp",
        "::canton::types::TextMap<Generic<::std::string::String, i64>>"
    )]
    #[case::genmap_inline(
        "genmap_inline",
        "::std::collections::BTreeMap<::std::string::String, i64>"
    )]
    #[case::genmap_tapp(
        "genmap_tapp",
        "::std::collections::BTreeMap<::std::string::String, i64>"
    )]
    #[case::genmap_nested_tapp(
        "genmap_nested_tapp",
        "::std::collections::BTreeMap<::std::string::String, ::std::vec::Vec<Target>>"
    )]
    fn generate(
        packages: (ProtoPackage, ProtoPackage),
        #[case] field: &str,
        #[case] expected: &str,
    ) {
        let (dependency_package, main_package) = packages;
        let dependency_package =
            SealedPackage::seal(&dependency_package).expect("dependency package should be valid");
        let main_package =
            SealedPackage::seal(&main_package).expect("main package should be valid");
        let dependency_id = PackageId::new_unchecked("dependency");
        let main_id = PackageId::new_unchecked("main-package");
        let version = Version {
            major: 2,
            minor: MinorVersion::Dev,
        };
        let packages: Packages<'_> = BTreeMap::from([
            (
                dependency_id.clone(),
                PackageWithIdent {
                    package: OuterSealedPackage::new(
                        version,
                        dependency_id,
                        VersionedSealedPackage::V2(dependency_package),
                    ),
                    ident: syn::parse_quote!(dependency),
                },
            ),
            (
                main_id.clone(),
                PackageWithIdent {
                    package: OuterSealedPackage::new(
                        version,
                        main_id.clone(),
                        VersionedSealedPackage::V2(main_package),
                    ),
                    ident: syn::parse_quote!(main_package),
                },
            ),
        ]);
        let ctx = GenCtx::new(
            &packages,
            Default::default(),
            Default::default(),
            TypeAttributes::new(),
        );
        let generator = TypeGenerator {
            ctx: &ctx,
            self_package_ident: &packages[&main_id].ident,
            module_path: syn::parse_quote!(main),
        };

        let module = main_package
            .modules()
            .into_iter()
            .find(|module| module.name() == *["Main"].as_slice())
            .expect("Main module should exist");
        let subject = module
            .data_types()
            .into_iter()
            .find(|data_type| data_type.name() == *["Subject"].as_slice())
            .expect("Subject data type should exist");
        let DataCons::Record(fields) = subject.data_cons() else {
            panic!("Subject should be a record");
        };
        let type_ = fields
            .fields()
            .into_iter()
            .find(|candidate| candidate.field() == field)
            .unwrap_or_else(|| panic!("Subject should contain field {field}"))
            .type_();

        let actual = generator.generate(type_).to_token_stream().to_string();
        let expected = syn::parse_str::<syn::Type>(expected)
            .expect("expected Rust type should parse")
            .to_token_stream()
            .to_string();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::same_module("main", "same_module", "Target")]
    #[case::same_module_from_child("main::child", "same_module", "super::Target")]
    #[case::other_module("main", "other_module", "super::other::Target")]
    #[case::nested_module("main", "nested_module", "super::nested::module::Target")]
    #[case::nested_module_from_own_module("nested::module", "nested_module", "Target")]
    #[case::nested_type("main", "nested_type", "outer::Inner")]
    #[case::nested_type_from_own_module("main::outer", "nested_type", "Inner")]
    #[case::imported("main", "imported", "super::super::dependency::dep::Payload")]
    #[case::imported_from_child(
        "main::child",
        "imported",
        "super::super::super::dependency::dep::Payload"
    )]
    #[case::imported_nested(
        "main",
        "imported_nested",
        "super::super::dependency::dep::outer::Payload"
    )]
    fn generate_type_path(
        packages: (ProtoPackage, ProtoPackage),
        #[case] module_path: &str,
        #[case] field: &str,
        #[case] expected: &str,
    ) {
        let (dependency_package, main_package) = packages;
        let dependency_package =
            SealedPackage::seal(&dependency_package).expect("dependency package should be valid");
        let main_package =
            SealedPackage::seal(&main_package).expect("main package should be valid");
        let dependency_id = PackageId::new_unchecked("dependency");
        let main_id = PackageId::new_unchecked("main-package");
        let version = Version {
            major: 2,
            minor: MinorVersion::Dev,
        };
        let packages: Packages<'_> = BTreeMap::from([
            (
                dependency_id.clone(),
                PackageWithIdent {
                    package: OuterSealedPackage::new(
                        version,
                        dependency_id,
                        VersionedSealedPackage::V2(dependency_package),
                    ),
                    ident: syn::parse_quote!(dependency),
                },
            ),
            (
                main_id.clone(),
                PackageWithIdent {
                    package: OuterSealedPackage::new(
                        version,
                        main_id.clone(),
                        VersionedSealedPackage::V2(main_package),
                    ),
                    ident: syn::parse_quote!(main_package),
                },
            ),
        ]);
        let ctx = GenCtx::new(
            &packages,
            Default::default(),
            Default::default(),
            TypeAttributes::new(),
        );
        let generator = TypeGenerator {
            ctx: &ctx,
            self_package_ident: &packages[&main_id].ident,
            module_path: syn::parse_str(module_path).expect("module path should parse"),
        };

        let module = main_package
            .modules()
            .into_iter()
            .find(|module| module.name() == *["Main"].as_slice())
            .expect("Main module should exist");
        let subject = module
            .data_types()
            .into_iter()
            .find(|data_type| data_type.name() == *["PathSubject"].as_slice())
            .expect("PathSubject data type should exist");
        let DataCons::Record(fields) = subject.data_cons() else {
            panic!("PathSubject should be a record");
        };
        let type_con_id = fields
            .fields()
            .into_iter()
            .find(|candidate| candidate.field() == field)
            .unwrap_or_else(|| panic!("PathSubject should contain field {field}"))
            .type_()
            .type_con_id()
            .unwrap_or_else(|| panic!("PathSubject.{field} should be a type constructor"));

        let actual = generator
            .generate_type_path(type_con_id)
            .to_token_stream()
            .to_string();
        let expected = syn::parse_str::<syn::TypePath>(expected)
            .expect("expected Rust type path should parse")
            .to_token_stream()
            .to_string();
        assert_eq!(actual, expected);
    }
}
