use canton_types::PackageId;
use daml_lf::{
    package::{SealedPackage, VersionedSealedPackage},
    v2::sealed::{
        BuiltinType, DefDataType, DefInterface, DefTemplate, DottedName, Module, Package,
        SelfOrImportedPackageId, Type, TypeConId,
        def_data_type::{DataCons, Fields},
        type_::{Builtin, Con, TApp, Var},
    },
};

use crate::{
    generator::GenCtx,
    graph::Edge,
    ir::Definition,
    v2::ir::{Def, Definition as V2Definition},
};

#[derive(Clone, Copy, Debug)]
pub struct DepsResolver<'a> {
    self_package_id: &'a PackageId,
    ctx: &'a GenCtx<'a>,
}

impl<'a> DepsResolver<'a> {
    pub fn new(ctx: &'a GenCtx<'a>, self_package_id: &'a PackageId) -> Self {
        Self {
            self_package_id,
            ctx,
        }
    }

    pub fn discover_templates_and_interfaces<'b>(package: Package<'b>) -> Vec<Def<'b>> {
        package
            .modules()
            .into_iter()
            .flat_map(Self::templates_and_interfaces_of_module)
            .collect()
    }

    fn templates_and_interfaces_of_module(module: Module<'_>) -> Vec<Def<'_>> {
        let data_types = module.data_types();
        let templates = module.templates().into_iter().map(|template| {
            let template_name = template.tycon_name();
            let data_type = *data_types
                .iter()
                .find(|dt| dt.name() == template_name)
                .expect("broken package: data type for {template:?} not found");
            Def::Template {
                template,
                data_type,
            }
        });
        let interfaces = module.interfaces().into_iter().map(|interface| {
            let iface_name = interface.tycon_name();
            let data_type = *data_types
                .iter()
                .find(|dt| dt.name() == iface_name)
                .expect("broken package: data type for {interface:?} not found");
            Def::Interface {
                interface,
                data_type,
            }
        });
        templates.chain(interfaces).collect()
    }

    /// Result doesn't contain input `definition`
    pub fn discover_dependencies(
        ctx: &'a GenCtx<'a>,
        definition: V2Definition<'a>,
    ) -> Vec<(Definition<'a>, Edge)> {
        let self_package_id = definition.package_id();
        let self_ = Self {
            self_package_id,
            ctx,
        };
        self_.deps_of_definition(definition.definition())
    }

    /// Find all dependent definitions of Daml LF v2 type
    pub fn deps_of_type(&self, type_: Type<'a>) -> Vec<(Definition<'a>, Edge)> {
        match type_ {
            Type::Var(var) => self.deps_of_var(var),
            Type::Con(con) => self.deps_of_con(con),
            Type::Builtin(builtin) => self.deps_of_builtin(builtin),
            Type::Nat => Vec::new(),
            Type::Tapp(tapp) => self.deps_of_tapp(tapp),
        }
    }

    fn deps_of_definition(&self, definition: Def<'a>) -> Vec<(Definition<'a>, Edge)> {
        match definition {
            Def::Template {
                template,
                data_type,
            } => self.deps_of_template(template, data_type),
            Def::DataType(data_type) => self.deps_of_data_type(data_type),
            Def::Interface {
                interface,
                data_type,
            } => self.deps_of_interface(interface, data_type),
        }
    }

    fn deps_of_template(
        &self,
        template: DefTemplate<'a>,
        data_type: DefDataType<'a>,
    ) -> Vec<(Definition<'a>, Edge)> {
        let mut deps = self.deps_of_data_type(data_type);

        for choice in template.choices() {
            deps.extend(self.deps_of_type(choice.arg_binder().type_()));
            deps.extend(self.deps_of_type(choice.ret_type()));
        }

        if let Some(key) = template.key() {
            deps.extend(self.deps_of_type(key.type_()));
        }

        deps.extend(
            template
                .implements()
                .into_iter()
                .map(|impl_| (self.ty_con_id_to_definition(impl_.interface()), Edge::Ref)),
        );

        deps
    }

    fn deps_of_interface(
        &self,
        interface: DefInterface<'a>,
        data_type: DefDataType<'a>,
    ) -> Vec<(Definition<'a>, Edge)> {
        let mut deps = self.deps_of_data_type(data_type);
        deps.extend(self.deps_of_type(interface.view()));

        for choice in interface.choices() {
            deps.extend(self.deps_of_type(choice.arg_binder().type_()));
            deps.extend(self.deps_of_type(choice.ret_type()));
        }

        deps.extend(
            interface
                .requires()
                .into_iter()
                .map(|ty_con_id| (self.ty_con_id_to_definition(ty_con_id), Edge::Ref)),
        );

        deps
    }

    fn deps_of_data_type(&self, dt: DefDataType<'a>) -> Vec<(Definition<'a>, Edge)> {
        match dt.data_cons() {
            DataCons::Record(fields) => self.deps_of_fields(fields),
            DataCons::Variant(fields) => self.deps_of_fields(fields),
            DataCons::Enum(_) => Vec::new(),
            DataCons::Interface => Vec::new(),
        }
    }

    fn deps_of_fields(&self, fields: Fields<'a>) -> Vec<(Definition<'a>, Edge)> {
        fields
            .fields()
            .into_iter()
            .flat_map(|field| self.deps_of_type(field.type_()))
            .collect()
    }

    fn deps_of_var(&self, var: Var<'a>) -> Vec<(Definition<'a>, Edge)> {
        self.deps_of_var_with_args(var, Vec::new())
    }

    fn deps_of_var_with_args(
        &self,
        var: Var<'a>,
        args: Vec<Type<'a>>,
    ) -> Vec<(Definition<'a>, Edge)> {
        var.args()
            .into_iter()
            .chain(args)
            .flat_map(|t| self.deps_of_type(t))
            .map(|(def, _)| (def, Edge::Ref))
            .collect()
    }

    fn deps_of_builtin(&self, builtin: Builtin<'a>) -> Vec<(Definition<'a>, Edge)> {
        self.deps_of_builtin_with_args(builtin, Vec::new())
    }

    fn deps_of_builtin_with_args(
        &self,
        builtin: Builtin<'a>,
        applied_args: Vec<Type<'a>>,
    ) -> Vec<(Definition<'a>, Edge)> {
        let args: Vec<_> = builtin
            .args()
            .into_iter()
            .chain(applied_args)
            .flat_map(|t| self.deps_of_type(t))
            .collect();

        match builtin.type_() {
            BuiltinType::Unit
            | BuiltinType::Bool
            | BuiltinType::Int64
            | BuiltinType::Date
            | BuiltinType::Timestamp
            | BuiltinType::Numeric
            | BuiltinType::Party
            | BuiltinType::Text
            | BuiltinType::Bignumeric
            | BuiltinType::RoundingMode => Vec::new(),

            BuiltinType::ContractId
            | BuiltinType::Any
            | BuiltinType::AnyException
            | BuiltinType::Arrow
            | BuiltinType::FailureCategory
            | BuiltinType::TypeRep
            | BuiltinType::List
            | BuiltinType::Genmap
            | BuiltinType::Textmap => args.into_iter().map(|(def, _)| (def, Edge::Ref)).collect(),

            BuiltinType::Optional | BuiltinType::Update => args,
        }
    }

    fn deps_of_con(&self, con: Con<'a>) -> Vec<(Definition<'a>, Edge)> {
        let mut deps: Vec<(Definition, Edge)> = con
            .args()
            .into_iter()
            .flat_map(|t| self.deps_of_type(t))
            .collect();
        deps.push((self.ty_con_id_to_definition(con.tycon()), Edge::Inline));
        deps
    }

    fn deps_of_con_with_args(
        &self,
        con: Con<'a>,
        args: Vec<Type<'a>>,
    ) -> Vec<(Definition<'a>, Edge)> {
        let mut deps = self.deps_of_con(con);
        deps.extend(args.into_iter().flat_map(|t| self.deps_of_type(t)));
        deps
    }

    fn ty_con_id_to_definition(&self, ty_con_id: TypeConId<'a>) -> Definition<'a> {
        let name = ty_con_id.name();
        let module_id = ty_con_id.module();
        let module_name = module_id.module_name();
        let self_or_import = module_id.package_id();

        let (package_id, package) = self
            .ctx
            .packages()
            .iter()
            .find(|(pid, _)| match self_or_import {
                SelfOrImportedPackageId::SelfPackageId => pid == &self.self_package_id,
                SelfOrImportedPackageId::ImportedPackageId(package_id) => *pid == package_id,
            })
            .expect("broken package: package specified in TypeConId wasn't found");

        Self::get_def_from(&package.package, package_id, module_name, name)
    }

    fn deps_of_tapp(&self, tapp: TApp<'a>) -> Vec<(Definition<'a>, Edge)> {
        let mut args = vec![tapp.rhs()];
        let mut lhs = tapp.lhs();
        while let Type::Tapp(tapp) = lhs {
            args.push(tapp.rhs());
            lhs = tapp.lhs();
        }
        args.reverse();

        match lhs {
            Type::Var(var) => self.deps_of_var_with_args(var, args),
            Type::Con(con) => self.deps_of_con_with_args(con, args),
            Type::Builtin(builtin) => self.deps_of_builtin_with_args(builtin, args),
            Type::Nat => args
                .into_iter()
                .flat_map(|t| self.deps_of_type(t))
                .map(|(def, _)| (def, Edge::Ref))
                .collect(),
            Type::Tapp(_) => unreachable!(),
        }
    }

    /// This function is based on how different versions of Daml LF can be referenced from Daml LF v2
    ///
    /// Based on the pointee type, there will be different kind of definitions:
    /// - templates with underlying data type
    /// - interfaces
    /// - data types
    ///
    /// Choices are referenced as bare data types
    fn get_def_from(
        package: &'a SealedPackage<'a>,
        package_id: &'a PackageId,
        module_name: DottedName<'a>,
        name: DottedName<'a>,
    ) -> Definition<'a> {
        match package.versioned() {
            VersionedSealedPackage::V2(package) => {
                let self_modules = package.modules();
                let module = self_modules
                    .into_iter()
                    .find(|m| m.name() == module_name)
                    .expect("broken package: module specified in TypeConId wasn't found");

                let maybe_dt = module.data_types().into_iter().find(|dt| dt.name() == name);
                let maybe_template = module
                    .templates()
                    .into_iter()
                    .find(|t| t.tycon_name() == name);
                let maybe_iface = module
                    .interfaces()
                    .into_iter()
                    .find(|t| t.tycon_name() == name);

                let definition = match (maybe_dt, maybe_template, maybe_iface) {
                    (Some(data_type), None, Some(interface)) => Def::Interface {
                        interface,
                        data_type,
                    },
                    (Some(data_type), Some(template), None) => Def::Template {
                        template,
                        data_type,
                    },
                    (Some(data_type), None, None) => Def::DataType(data_type),
                    (dt, t, i) => {
                        panic!("unexpected reference: data={dt:?}, template={t:?}, iface={i:?}")
                    }
                };

                Definition::V2(V2Definition::new(package_id, definition))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use canton_types::PackageId;
    use daml_lf::{
        dalf::DalfFile,
        package::{Package as RawPackage, VersionedSealedPackage},
        proto::{
            com::digitalasset::daml::lf::archive::{
                Archive, ArchivePayload, HashFunction, archive_payload,
            },
            prost::Message,
        },
        v2::builder::{
            BuiltinType, ChoiceBuilder, DataTypeBuilder, Field, InterfaceBuilder, Kind,
            ModuleBuilder, PackageBuilder, TemplateBuilder, Type as TypeBuilder, TypeConRef,
            TypeParameter,
        },
        v2::sealed::def_data_type::DataCons,
    };
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use sha2::{Digest, Sha256};

    use crate::{
        dispatcher::Packages,
        generator::GenCtx,
        graph::Edge,
        ir::Definition as CodegenDefinition,
        package_ident_generator::PackageIdentGenerator,
        type_attributes::TypeAttributes,
        v2::ir::{Def, Definition as V2Definition},
    };

    use super::DepsResolver;

    fn package(builder: PackageBuilder) -> RawPackage {
        let package = builder.build().encode_to_vec();
        let payload = ArchivePayload {
            minor: "dev".to_owned(),
            patch: 0,
            sum: Some(archive_payload::Sum::DamlLf2(package)),
        }
        .encode_to_vec();
        let archive = Archive {
            hash_function: HashFunction::Sha256 as i32,
            hash: Sha256::digest(&payload)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            payload,
        }
        .encode_to_vec();

        DalfFile::read_from(archive.as_slice())
            .unwrap()
            .to_package()
            .unwrap()
    }

    fn definition<'a>(
        packages: &'a Packages<'a>,
        package_id: &PackageId,
        kind: &str,
        name: &str,
    ) -> V2Definition<'a> {
        let (package_id, package) = packages.get_key_value(package_id).unwrap();
        let VersionedSealedPackage::V2(package) = package.package.versioned();
        let module = package.modules()[0];
        let data_type = || {
            module
                .data_types()
                .into_iter()
                .find(|definition| definition.name() == *[name].as_slice())
                .unwrap()
        };
        let definition = match kind {
            "data" => Def::DataType(data_type()),
            "template" => Def::Template {
                data_type: data_type(),
                template: module
                    .templates()
                    .into_iter()
                    .find(|definition| definition.tycon_name() == *[name].as_slice())
                    .unwrap(),
            },
            "interface" => Def::Interface {
                data_type: data_type(),
                interface: module
                    .interfaces()
                    .into_iter()
                    .find(|definition| definition.tycon_name() == *[name].as_slice())
                    .unwrap(),
            },
            _ => panic!("unknown definition kind: {kind}"),
        };

        V2Definition::new(package_id, definition)
    }

    fn assert_dependencies(
        raw: &[RawPackage],
        package_id: &PackageId,
        kind: &str,
        name: &str,
        expected: &[(&str, &str, &str, &str, Edge)],
    ) {
        let sealed = raw
            .iter()
            .map(|package| (package.package_id().clone(), package.seal().unwrap()))
            .collect::<BTreeMap<_, _>>();
        let packages = PackageIdentGenerator::new(sealed).generate();
        let ctx = GenCtx::new(
            &packages,
            Default::default(),
            Default::default(),
            TypeAttributes::resolve(&Default::default(), &packages).unwrap(),
        );
        let actual = DepsResolver::discover_dependencies(
            &ctx,
            definition(&packages, package_id, kind, name),
        )
        .into_iter()
        .map(|(definition, edge)| {
            let CodegenDefinition::V2(definition) = definition;
            let daml_definition = definition.definition();
            let data_type = daml_definition.data_type();
            assert_eq!(data_type.name(), daml_definition.name());
            let kind = match daml_definition {
                Def::DataType(_) => "data",
                Def::Template { .. } => "template",
                Def::Interface { .. } => {
                    assert!(matches!(data_type.data_cons(), DataCons::Interface));
                    "interface"
                }
            };
            (
                definition.package().metadata().name(),
                format!("{:?}", definition.module().name()),
                format!("{:?}", daml_definition.name()),
                kind,
                edge,
            )
        })
        .collect::<Vec<_>>();
        let expected = expected
            .iter()
            .map(|(package, module, name, kind, edge)| {
                (*package, module.to_string(), name.to_string(), *kind, *edge)
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
    }

    #[fixture]
    fn declarations_package() -> RawPackage {
        let records = [
            "Payload",
            "ChoiceArg",
            "ChoiceResult",
            "Key",
            "View",
            "BaseView",
        ];
        let module = records
            .into_iter()
            .fold(ModuleBuilder::new("Main"), |module, name| {
                module.data_type(DataTypeBuilder::record(name, []))
            })
            .data_type(DataTypeBuilder::enumeration("Status", ["Open", "Closed"]))
            .data_type(DataTypeBuilder::interface("Base").serializable(false))
            .data_type(DataTypeBuilder::interface("Asset").serializable(false))
            .data_type(DataTypeBuilder::record(
                "Contract",
                [Field::new("payload", TypeBuilder::local("Main", "Payload"))],
            ))
            .data_type(DataTypeBuilder::variant(
                "Tree",
                [
                    Field::new("Leaf", TypeBuilder::local("Main", "Payload")),
                    Field::new(
                        "Branch",
                        TypeBuilder::list(TypeBuilder::local("Main", "Tree")),
                    ),
                    Field::new("DirectBranch", TypeBuilder::local("Main", "Tree")),
                    Field::new("OtherLeaf", TypeBuilder::local("Main", "Payload")),
                ],
            ))
            .template(
                TemplateBuilder::new("Contract")
                    .choice(ChoiceBuilder::new(
                        "Act",
                        TypeBuilder::local("Main", "ChoiceArg"),
                        TypeBuilder::local("Main", "ChoiceResult"),
                    ))
                    .key(TypeBuilder::local("Main", "Key"))
                    .implements(TypeConRef::local("Main", "Base"))
                    .implements(TypeConRef::local("Main", "Asset")),
            )
            .interface(InterfaceBuilder::new(
                "Base",
                TypeBuilder::local("Main", "BaseView"),
            ))
            .interface(
                InterfaceBuilder::new("Asset", TypeBuilder::local("Main", "View"))
                    .choice(ChoiceBuilder::new(
                        "Act",
                        TypeBuilder::local("Main", "ChoiceArg"),
                        TypeBuilder::local("Main", "ChoiceResult"),
                    ))
                    .requires(TypeConRef::local("Main", "Base")),
            );

        package(PackageBuilder::new("definitions", "1.0.0").module(module))
    }

    /// Discovers templates and interfaces together with their attached data types.
    ///
    /// ```daml
    /// -- definitions@1.0.0
    /// module Main where
    ///
    /// data Payload = Payload
    /// data ChoiceArg = ChoiceArg
    /// data ChoiceResult = ChoiceResult
    /// data Key = Key
    /// data View = View
    /// data BaseView = BaseView
    /// data Status = Open | Closed
    ///
    /// interface Base where
    ///     viewtype BaseView
    ///
    /// interface Asset requires Base where
    ///     viewtype View
    ///     nonconsuming choice Act : ChoiceResult
    ///         with arg : ChoiceArg
    ///
    /// template Contract
    ///     with payload : Payload
    ///     where
    ///         nonconsuming choice Act : ChoiceResult
    ///             with arg : ChoiceArg
    ///         key Key
    ///         implements Base
    ///         implements Asset
    ///
    /// data Tree
    ///     = Leaf Payload
    ///     | Branch [Tree]
    ///     | DirectBranch Tree
    ///     | OtherLeaf Payload
    /// ```
    #[rstest]
    fn discovers_only_templates_and_interfaces(declarations_package: RawPackage) {
        let sealed = declarations_package.seal().unwrap();
        let VersionedSealedPackage::V2(package) = sealed.versioned();

        let found = DepsResolver::discover_templates_and_interfaces(package)
            .into_iter()
            .map(|definition| {
                let kind = match definition {
                    Def::Template { .. } => "template",
                    Def::Interface { .. } => "interface",
                    Def::DataType(_) => "data",
                };
                let data_kind = match definition.data_type().data_cons() {
                    DataCons::Record(_) => "record",
                    DataCons::Interface => "interface",
                    DataCons::Variant(_) => "variant",
                    DataCons::Enum(_) => "enum",
                };
                (
                    format!("{:?}", definition.module().name()),
                    format!("{:?}", definition.name()),
                    kind,
                    format!("{:?}", definition.data_type().name()),
                    data_kind,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            found,
            [
                (
                    "Main".to_owned(),
                    "Contract".to_owned(),
                    "template",
                    "Contract".to_owned(),
                    "record",
                ),
                (
                    "Main".to_owned(),
                    "Base".to_owned(),
                    "interface",
                    "Base".to_owned(),
                    "interface",
                ),
                (
                    "Main".to_owned(),
                    "Asset".to_owned(),
                    "interface",
                    "Asset".to_owned(),
                    "interface",
                ),
            ]
        );
    }

    /// Resolves every supported type form from record fields and variant constructors.
    ///
    /// case "record":
    ///
    /// ```daml
    /// -- types@1.0.0
    /// module Main where
    ///
    /// data Generic a = Generic with
    ///     value : a
    ///
    /// data InlineTarget = InlineTarget
    /// data RefTarget = RefTarget
    /// data ConArg = ConArg
    /// data TappArg = TappArg
    /// data VarArg = VarArg
    /// data TappVarArg = TappVarArg
    ///
    /// data Subject (f : * -> *) a = Subject with
    ///     primitive : Text
    ///     variable : a
    ///     optional : Optional InlineTarget
    ///     list : [RefTarget]
    ///     constructor : Generic ConArg -- encoded as Con with args
    ///     application : Generic TappArg -- encoded as TApp
    ///     appliedVariable : f VarArg
    ///     tappAppliedVariable : f TappVarArg -- encoded as TApp
    ///     numeric : Numeric 10
    /// ```
    ///
    /// case "variant":
    ///
    /// ```daml
    /// -- types@1.0.0
    /// module Main where
    ///
    /// data Generic a = Generic with
    ///     value : a
    ///
    /// data InlineTarget = InlineTarget
    /// data RefTarget = RefTarget
    /// data ConArg = ConArg
    /// data TappArg = TappArg
    /// data VarArg = VarArg
    /// data TappVarArg = TappVarArg
    ///
    /// data Subject (f : * -> *) a
    ///     = primitive Text
    ///     | variable a
    ///     | optional (Optional InlineTarget)
    ///     | list [RefTarget]
    ///     | constructor (Generic ConArg) -- encoded as Con with args
    ///     | application (Generic TappArg) -- encoded as TApp
    ///     | appliedVariable (f VarArg)
    ///     | tappAppliedVariable (f TappVarArg) -- encoded as TApp
    ///     | numeric (Numeric 10)
    /// ```
    #[rstest]
    #[case::record(false)]
    #[case::variant(true)]
    fn resolves_all_type_forms_in_record_and_variant_fields(#[case] variant: bool) {
        let fields = [
            Field::new("primitive", TypeBuilder::text()),
            Field::new("variable", TypeBuilder::var("a")),
            Field::new(
                "optional",
                TypeBuilder::optional(TypeBuilder::local("Main", "InlineTarget")),
            ),
            Field::new(
                "list",
                TypeBuilder::list(TypeBuilder::local("Main", "RefTarget")),
            ),
            Field::new(
                "constructor",
                TypeBuilder::con_with_args(
                    TypeConRef::local("Main", "Generic"),
                    [TypeBuilder::local("Main", "ConArg")],
                ),
            ),
            Field::new(
                "application",
                TypeBuilder::tapp(
                    TypeBuilder::local("Main", "Generic"),
                    TypeBuilder::local("Main", "TappArg"),
                ),
            ),
            Field::new(
                "appliedVariable",
                TypeBuilder::var_with_args("f", [TypeBuilder::local("Main", "VarArg")]),
            ),
            Field::new(
                "tappAppliedVariable",
                TypeBuilder::tapp(
                    TypeBuilder::var("f"),
                    TypeBuilder::local("Main", "TappVarArg"),
                ),
            ),
            Field::new(
                "numeric",
                TypeBuilder::tapp(
                    TypeBuilder::builtin(BuiltinType::Numeric),
                    TypeBuilder::nat(10),
                ),
            ),
        ];
        let subject = if variant {
            DataTypeBuilder::variant("Subject", fields)
        } else {
            DataTypeBuilder::record("Subject", fields)
        }
        .type_parameter(TypeParameter::new(
            "f",
            Kind::arrow([Kind::Star], Kind::Star),
        ))
        .type_parameter(TypeParameter::new("a", Kind::Star))
        .serializable(false);
        let module = [
            "InlineTarget",
            "RefTarget",
            "ConArg",
            "TappArg",
            "VarArg",
            "TappVarArg",
        ]
        .into_iter()
        .fold(
            ModuleBuilder::new("Main")
                .data_type(
                    DataTypeBuilder::record(
                        "Generic",
                        [Field::new("value", TypeBuilder::var("a"))],
                    )
                    .type_parameter(TypeParameter::new("a", Kind::Star)),
                )
                .data_type(subject),
            |module, name| module.data_type(DataTypeBuilder::record(name, [])),
        );
        let raw = [package(
            PackageBuilder::new("types", "1.0.0").module(module),
        )];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Subject",
            &[
                ("types", "Main", "InlineTarget", "data", Edge::Inline),
                ("types", "Main", "RefTarget", "data", Edge::Ref),
                ("types", "Main", "ConArg", "data", Edge::Inline),
                ("types", "Main", "Generic", "data", Edge::Inline),
                ("types", "Main", "Generic", "data", Edge::Inline),
                ("types", "Main", "TappArg", "data", Edge::Inline),
                ("types", "Main", "VarArg", "data", Edge::Ref),
                ("types", "Main", "TappVarArg", "data", Edge::Ref),
            ],
        );
    }

    /// Assigns dependency strength according to the indirection of each builtin type.
    ///
    /// case "contract_id_template":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : ContractId Target
    /// ```
    ///
    /// case "contract_id_interface":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// data View = View
    /// interface Target where viewtype View
    /// data Subject = Subject with value : ContractId Target
    /// ```
    ///
    /// case "list":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : [Target]
    /// ```
    ///
    /// case "textmap":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : TextMap Target
    /// ```
    ///
    /// case "genmap":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : GenMap Text Target
    /// ```
    ///
    /// case "arrow":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : Text -> Target
    /// ```
    ///
    /// case "optional":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : Optional Target
    /// ```
    ///
    /// case "update":
    ///
    /// ```daml
    /// -- builtin@1.0.0
    /// module Main where
    /// template Target with where
    /// data Subject = Subject with value : Update Target
    /// ```
    #[rstest]
    #[case::contract_id_template(BuiltinType::ContractId, false, "template", Edge::Ref)]
    #[case::contract_id_interface(BuiltinType::ContractId, true, "interface", Edge::Ref)]
    #[case::list(BuiltinType::List, false, "template", Edge::Ref)]
    #[case::textmap(BuiltinType::Textmap, false, "template", Edge::Ref)]
    #[case::genmap(BuiltinType::Genmap, false, "template", Edge::Ref)]
    #[case::arrow(BuiltinType::Arrow, false, "template", Edge::Ref)]
    #[case::optional(BuiltinType::Optional, false, "template", Edge::Inline)]
    #[case::update(BuiltinType::Update, false, "template", Edge::Inline)]
    fn builtin_container_controls_dependency_strength(
        #[case] builtin: BuiltinType,
        #[case] interface_target: bool,
        #[case] target_kind: &str,
        #[case] expected_edge: Edge,
    ) {
        let args = match builtin {
            BuiltinType::Genmap | BuiltinType::Arrow => {
                vec![TypeBuilder::text(), TypeBuilder::local("Main", "Target")]
            }
            _ => vec![TypeBuilder::local("Main", "Target")],
        };
        let module = if interface_target {
            ModuleBuilder::new("Main")
                .data_type(DataTypeBuilder::record("View", []))
                .data_type(DataTypeBuilder::interface("Target").serializable(false))
                .interface(InterfaceBuilder::new(
                    "Target",
                    TypeBuilder::local("Main", "View"),
                ))
        } else {
            ModuleBuilder::new("Main")
                .data_type(DataTypeBuilder::record("Target", []))
                .template(TemplateBuilder::new("Target"))
        };
        let raw = [package(
            PackageBuilder::new("builtin", "1.0.0").module(
                module.data_type(
                    DataTypeBuilder::record(
                        "Subject",
                        [Field::new(
                            "value",
                            TypeBuilder::builtin_with_args(builtin, args),
                        )],
                    )
                    .serializable(false),
                ),
            ),
        )];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Subject",
            &[("builtin", "Main", "Target", target_kind, expected_edge)],
        );
    }

    /// Preserves reference boundaries through nested and explicitly applied containers.
    ///
    /// case "optional_list":
    ///
    /// ```daml
    /// -- nested@1.0.0
    /// module Main where
    /// data Target = Target
    /// data Subject = Subject with value : Optional [Target]
    /// ```
    ///
    /// case "list_optional":
    ///
    /// ```daml
    /// -- nested@1.0.0
    /// module Main where
    /// data Target = Target
    /// data Subject = Subject with value : [Optional Target]
    /// ```
    ///
    /// case "nested_optional":
    ///
    /// ```daml
    /// -- nested@1.0.0
    /// module Main where
    /// data Target = Target
    /// data Subject = Subject with value : Optional (Optional Target)
    /// ```
    ///
    /// case "tapp_optional":
    ///
    /// ```daml
    /// -- nested@1.0.0
    /// module Main where
    /// data Target = Target
    /// data Subject = Subject with value : Optional Target -- encoded as TApp
    /// ```
    ///
    /// case "tapp_list":
    ///
    /// ```daml
    /// -- nested@1.0.0
    /// module Main where
    /// data Target = Target
    /// data Subject = Subject with value : [Target] -- encoded as TApp
    /// ```
    #[rstest]
    #[case::optional_list(
        TypeBuilder::optional(TypeBuilder::list(TypeBuilder::local("Main", "Target"))),
        Edge::Ref
    )]
    #[case::list_optional(
        TypeBuilder::list(TypeBuilder::optional(TypeBuilder::local("Main", "Target"))),
        Edge::Ref
    )]
    #[case::nested_optional(
        TypeBuilder::optional(TypeBuilder::optional(TypeBuilder::local("Main", "Target"))),
        Edge::Inline
    )]
    #[case::tapp_optional(
        TypeBuilder::tapp(
            TypeBuilder::builtin(BuiltinType::Optional),
            TypeBuilder::local("Main", "Target")
        ),
        Edge::Inline
    )]
    #[case::tapp_list(
        TypeBuilder::tapp(
            TypeBuilder::builtin(BuiltinType::List),
            TypeBuilder::local("Main", "Target")
        ),
        Edge::Ref
    )]
    fn container_composition_preserves_reference_boundaries(
        #[case] field_type: TypeBuilder,
        #[case] expected_edge: Edge,
    ) {
        let raw = [package(
            PackageBuilder::new("nested", "1.0.0").module(
                ModuleBuilder::new("Main")
                    .data_type(DataTypeBuilder::record("Target", []))
                    .data_type(DataTypeBuilder::record(
                        "Subject",
                        [Field::new("value", field_type)],
                    )),
            ),
        )];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Subject",
            &[("nested", "Main", "Target", "data", expected_edge)],
        );
    }

    /// Produces no dependencies for enums or interface marker data types.
    ///
    /// ```daml
    /// -- definitions@1.0.0
    /// module Main where
    /// data Status = Open | Closed
    /// data View = View
    /// data BaseView = BaseView
    /// interface Base where viewtype BaseView
    /// interface Asset requires Base where viewtype View
    /// ```
    #[rstest]
    fn enum_and_interface_marker_have_no_dependencies(declarations_package: RawPackage) {
        let raw = [declarations_package];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(&raw, &package_id, "data", "Status", &[]);
        assert_dependencies(&raw, &package_id, "data", "Asset", &[]);
    }

    /// Collects dependencies from a template record, choices, key, and interfaces.
    ///
    /// ```daml
    /// -- definitions@1.0.0
    /// module Main where
    /// data Payload = Payload
    /// data ChoiceArg = ChoiceArg
    /// data ChoiceResult = ChoiceResult
    /// data Key = Key
    /// data View = View
    /// data BaseView = BaseView
    ///
    /// interface Base where
    ///     viewtype BaseView
    ///
    /// interface Asset requires Base where
    ///     viewtype View
    ///
    /// template Contract
    ///     with payload : Payload
    ///     where
    ///         nonconsuming choice Act : ChoiceResult
    ///             with arg : ChoiceArg
    ///         key Key
    ///         implements Base
    ///         implements Asset
    /// ```
    #[rstest]
    fn template_collects_record_choice_key_and_implemented_interface_dependencies(
        declarations_package: RawPackage,
    ) {
        let raw = [declarations_package];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "template",
            "Contract",
            &[
                ("definitions", "Main", "Payload", "data", Edge::Inline),
                ("definitions", "Main", "ChoiceArg", "data", Edge::Inline),
                ("definitions", "Main", "ChoiceResult", "data", Edge::Inline),
                ("definitions", "Main", "Key", "data", Edge::Inline),
                ("definitions", "Main", "Base", "interface", Edge::Ref),
                ("definitions", "Main", "Asset", "interface", Edge::Ref),
            ],
        );
    }

    /// Collects dependencies from an interface view, choices, and required interfaces.
    ///
    /// ```daml
    /// -- definitions@1.0.0
    /// module Main where
    /// data ChoiceArg = ChoiceArg
    /// data ChoiceResult = ChoiceResult
    /// data View = View
    /// data BaseView = BaseView
    ///
    /// interface Base where
    ///     viewtype BaseView
    ///
    /// interface Asset requires Base where
    ///     viewtype View
    ///     nonconsuming choice Act : ChoiceResult
    ///         with arg : ChoiceArg
    /// ```
    #[rstest]
    fn interface_collects_view_and_choice_dependencies(declarations_package: RawPackage) {
        let raw = [declarations_package];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "interface",
            "Asset",
            &[
                ("definitions", "Main", "View", "data", Edge::Inline),
                ("definitions", "Main", "ChoiceArg", "data", Edge::Inline),
                ("definitions", "Main", "ChoiceResult", "data", Edge::Inline),
                ("definitions", "Main", "Base", "interface", Edge::Ref),
            ],
        );
    }

    /// Resolves identically named and dotted types from their exact modules.
    ///
    /// ```daml
    /// -- modules@1.0.0
    /// module Main where
    /// data Payload = Payload
    /// data Subject = Subject with
    ///     main : Payload
    ///     support : Support.Internal.Payload
    ///     nested : Support.Internal.Outer.Payload
    /// ```
    /// ```daml
    /// -- modules@1.0.0
    /// module Support.Internal where
    /// data Payload = Payload
    /// data Outer.Payload = Outer.Payload
    /// ```
    #[test]
    fn resolves_same_named_types_from_different_modules() {
        let raw = [package(
            PackageBuilder::new("modules", "1.0.0")
                .module(
                    ModuleBuilder::new("Main")
                        .data_type(DataTypeBuilder::record("Payload", []))
                        .data_type(DataTypeBuilder::record(
                            "Subject",
                            [
                                Field::new("main", TypeBuilder::local("Main", "Payload")),
                                Field::new(
                                    "support",
                                    TypeBuilder::local("Support.Internal", "Payload"),
                                ),
                                Field::new(
                                    "nested",
                                    TypeBuilder::local("Support.Internal", "Outer.Payload"),
                                ),
                            ],
                        )),
                )
                .module(
                    ModuleBuilder::new("Support.Internal")
                        .data_type(DataTypeBuilder::record("Payload", []))
                        .data_type(DataTypeBuilder::record("Outer.Payload", [])),
                ),
        )];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Subject",
            &[
                ("modules", "Main", "Payload", "data", Edge::Inline),
                (
                    "modules",
                    "Support.Internal",
                    "Payload",
                    "data",
                    Edge::Inline,
                ),
                (
                    "modules",
                    "Support.Internal",
                    "Outer.Payload",
                    "data",
                    Edge::Inline,
                ),
            ],
        );
    }

    /// Resolves local and imported references to the correct definition kinds.
    ///
    /// ```daml
    /// -- dependency@1.0.0
    /// module Main where
    /// data Payload = Payload
    /// template RemoteContract with where
    /// data View = View
    /// interface RemoteInterface where viewtype View
    /// ```
    ///
    /// ```daml
    /// -- main@1.0.0
    /// module Main where
    /// data Payload = Payload
    /// template Contract
    ///     with
    ///         localPayload : Payload
    ///         remotePayload : dependency:Main:Payload
    ///         contractId : ContractId dependency:Main:RemoteContract
    ///     where
    ///         implements dependency:Main:RemoteInterface
    /// ```
    #[test]
    fn imported_references_resolve_to_the_correct_definition_kind() {
        let dependency_package = package(
            PackageBuilder::new("dependency", "1.0.0").module(
                ModuleBuilder::new("Main")
                    .data_type(DataTypeBuilder::record("Payload", []))
                    .data_type(DataTypeBuilder::record("RemoteContract", []))
                    .template(TemplateBuilder::new("RemoteContract"))
                    .data_type(DataTypeBuilder::record("View", []))
                    .data_type(DataTypeBuilder::interface("RemoteInterface").serializable(false))
                    .interface(InterfaceBuilder::new(
                        "RemoteInterface",
                        TypeBuilder::local("Main", "View"),
                    )),
            ),
        );
        let dependency_id = dependency_package.package_id().clone();
        let main =
            package(
                PackageBuilder::new("main", "1.0.0").module(
                    ModuleBuilder::new("Main")
                        .data_type(DataTypeBuilder::record("Payload", []))
                        .data_type(DataTypeBuilder::record(
                            "Contract",
                            [
                                Field::new("localPayload", TypeBuilder::local("Main", "Payload")),
                                Field::new(
                                    "remotePayload",
                                    TypeBuilder::imported(&dependency_id, "Main", "Payload"),
                                ),
                                Field::new(
                                    "contractId",
                                    TypeBuilder::builtin_with_args(
                                        BuiltinType::ContractId,
                                        [TypeBuilder::imported(
                                            &dependency_id,
                                            "Main",
                                            "RemoteContract",
                                        )],
                                    ),
                                ),
                            ],
                        ))
                        .template(TemplateBuilder::new("Contract").implements(
                            TypeConRef::imported(&dependency_id, "Main", "RemoteInterface"),
                        )),
                ),
            );
        let main_id = main.package_id().clone();
        let raw = [dependency_package, main];

        assert_dependencies(
            &raw,
            &main_id,
            "template",
            "Contract",
            &[
                ("main", "Main", "Payload", "data", Edge::Inline),
                ("dependency", "Main", "Payload", "data", Edge::Inline),
                (
                    "dependency",
                    "Main",
                    "RemoteContract",
                    "template",
                    Edge::Ref,
                ),
                (
                    "dependency",
                    "Main",
                    "RemoteInterface",
                    "interface",
                    Edge::Ref,
                ),
            ],
        );
    }

    /// Preserves duplicate dependencies and both inline and referenced recursion.
    ///
    /// ```daml
    /// -- definitions@1.0.0
    /// module Main where
    /// data Payload = Payload
    /// data Tree
    ///     = Leaf Payload
    ///     | Branch [Tree]
    ///     | DirectBranch Tree
    ///     | OtherLeaf Payload
    /// ```
    #[rstest]
    fn preserves_duplicate_and_recursive_references_for_the_graph_builder(
        declarations_package: RawPackage,
    ) {
        let raw = [declarations_package];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Tree",
            &[
                ("definitions", "Main", "Payload", "data", Edge::Inline),
                ("definitions", "Main", "Tree", "data", Edge::Ref),
                ("definitions", "Main", "Tree", "data", Edge::Inline),
                ("definitions", "Main", "Payload", "data", Edge::Inline),
            ],
        );
    }

    /// Distinguishes inline and referenced directions in a mutual recursion cycle.
    ///
    /// ```daml
    /// -- mutual@1.0.0
    /// module Main where
    /// data Parent = Parent with child : Child
    /// data Child = Child with parents : [Parent]
    /// ```
    #[test]
    fn mutual_recursion_preserves_edge_strength_per_direction() {
        let raw = [package(
            PackageBuilder::new("mutual", "1.0.0").module(
                ModuleBuilder::new("Main")
                    .data_type(DataTypeBuilder::record(
                        "Parent",
                        [Field::new("child", TypeBuilder::local("Main", "Child"))],
                    ))
                    .data_type(DataTypeBuilder::record(
                        "Child",
                        [Field::new(
                            "parents",
                            TypeBuilder::list(TypeBuilder::local("Main", "Parent")),
                        )],
                    )),
            ),
        )];
        let package_id = raw[0].package_id().clone();

        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Parent",
            &[("mutual", "Main", "Child", "data", Edge::Inline)],
        );
        assert_dependencies(
            &raw,
            &package_id,
            "data",
            "Child",
            &[("mutual", "Main", "Parent", "data", Edge::Ref)],
        );
    }
}
