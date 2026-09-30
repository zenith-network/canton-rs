use canton_types::PackageId;
use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2::{self as proto, BuiltinType};

use crate::v2::builder::lower::{Lower, Lowerer};

#[derive(Clone, Debug, PartialEq, Eq)]
enum PackageRef {
    SelfPackage,
    Imported(PackageId),
}

/// An owned reference to a type constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeConRef {
    package: PackageRef,
    module: String,
    name: String,
}

impl TypeConRef {
    pub fn local(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            package: PackageRef::SelfPackage,
            module: module.into(),
            name: name.into(),
        }
    }

    pub fn imported(
        package: &PackageId,
        module: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            package: PackageRef::Imported(package.clone()),
            module: module.into(),
            name: name.into(),
        }
    }
}

impl Lower<proto::TypeConId> for TypeConRef {
    fn lower(self, lowerer: &mut Lowerer) -> proto::TypeConId {
        let package_id = match self.package {
            PackageRef::SelfPackage => proto::SelfOrImportedPackageId {
                sum: Some(proto::self_or_imported_package_id::Sum::SelfPackageId(
                    proto::Unit {},
                )),
            },
            PackageRef::Imported(package) => proto::SelfOrImportedPackageId {
                sum: Some(proto::self_or_imported_package_id::Sum::PackageImportId(
                    lowerer.intern_import(package),
                )),
            },
        };

        proto::TypeConId {
            module: Some(proto::ModuleId {
                package_id: Some(package_id),
                module_name_interned_dname: lowerer.intern_dotted(&self.module),
            }),
            name_interned_dname: lowerer.intern_dotted(&self.name),
        }
    }
}

/// An owned kind used by a type parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Star,
    Nat,
    Arrow {
        params: Vec<Kind>,
        result: Box<Kind>,
    },
}

impl Kind {
    pub fn arrow(params: impl IntoIterator<Item = Kind>, result: Kind) -> Self {
        Self::Arrow {
            params: params.into_iter().collect(),
            result: Box::new(result),
        }
    }

    fn into_proto(self) -> proto::Kind {
        let sum = match self {
            Kind::Star => proto::kind::Sum::Star(proto::Unit {}),
            Kind::Nat => proto::kind::Sum::Nat(proto::Unit {}),
            Kind::Arrow { params, result } => {
                proto::kind::Sum::Arrow(Box::new(proto::kind::Arrow {
                    params: params.into_iter().map(Kind::into_proto).collect(),
                    result: Some(Box::new(result.into_proto())),
                }))
            }
        };
        proto::Kind { sum: Some(sum) }
    }
}

/// An owned type parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeParameter {
    name: String,
    kind: Kind,
}

impl TypeParameter {
    pub fn new(name: impl Into<String>, kind: Kind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

impl Lower<proto::TypeVarWithKind> for TypeParameter {
    fn lower(self, lowerer: &mut Lowerer) -> proto::TypeVarWithKind {
        proto::TypeVarWithKind {
            var_interned_str: lowerer.intern_string(self.name),
            kind: Some(self.kind.into_proto()),
        }
    }
}

/// An owned LF type supported by the current sealed v2 representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Var {
        name: String,
        args: Vec<Type>,
    },
    Con {
        reference: TypeConRef,
        args: Vec<Type>,
    },
    Builtin {
        builtin: BuiltinType,
        args: Vec<Type>,
    },
    Nat(i64),
    TApp {
        lhs: Box<Type>,
        rhs: Box<Type>,
    },
}

impl Type {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn var_with_args(name: impl Into<String>, args: impl IntoIterator<Item = Type>) -> Self {
        Self::Var {
            name: name.into(),
            args: args.into_iter().collect(),
        }
    }

    pub fn con(reference: TypeConRef) -> Self {
        Self::Con {
            reference,
            args: Vec::new(),
        }
    }

    pub fn con_with_args(reference: TypeConRef, args: impl IntoIterator<Item = Type>) -> Self {
        Self::Con {
            reference,
            args: args.into_iter().collect(),
        }
    }

    pub fn local(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self::con(TypeConRef::local(module, name))
    }

    pub fn imported(
        package: &PackageId,
        module: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self::con(TypeConRef::imported(package, module, name))
    }

    pub fn builtin(builtin: BuiltinType) -> Self {
        Self::Builtin {
            builtin,
            args: Vec::new(),
        }
    }

    pub fn builtin_with_args(builtin: BuiltinType, args: impl IntoIterator<Item = Type>) -> Self {
        Self::Builtin {
            builtin,
            args: args.into_iter().collect(),
        }
    }

    pub fn nat(value: i64) -> Self {
        Self::Nat(value)
    }

    pub fn tapp(lhs: Type, rhs: Type) -> Self {
        Self::TApp {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn unit() -> Self {
        Self::builtin(BuiltinType::Unit)
    }

    pub fn bool() -> Self {
        Self::builtin(BuiltinType::Bool)
    }

    pub fn int64() -> Self {
        Self::builtin(BuiltinType::Int64)
    }

    pub fn party() -> Self {
        Self::builtin(BuiltinType::Party)
    }

    pub fn text() -> Self {
        Self::builtin(BuiltinType::Text)
    }

    pub fn list(item: Type) -> Self {
        Self::builtin_with_args(BuiltinType::List, [item])
    }

    pub fn optional(item: Type) -> Self {
        Self::builtin_with_args(BuiltinType::Optional, [item])
    }
}

impl Lower<proto::Type> for Type {
    fn lower(self, lowerer: &mut Lowerer) -> proto::Type {
        let sum = match self {
            Type::Var { name, args } => proto::r#type::Sum::Var(proto::r#type::Var {
                var_interned_str: lowerer.intern_string(name),
                args: args.into_iter().map(|type_| type_.lower(lowerer)).collect(),
            }),
            Type::Con { reference, args } => proto::r#type::Sum::Con(proto::r#type::Con {
                tycon: Some(reference.lower(lowerer)),
                args: args.into_iter().map(|type_| type_.lower(lowerer)).collect(),
            }),
            Type::Builtin { builtin, args } => {
                proto::r#type::Sum::Builtin(proto::r#type::Builtin {
                    builtin: builtin as i32,
                    args: args.into_iter().map(|type_| type_.lower(lowerer)).collect(),
                })
            }
            Type::Nat(value) => proto::r#type::Sum::Nat(value),
            Type::TApp { lhs, rhs } => proto::r#type::Sum::Tapp(Box::new(proto::r#type::TApp {
                lhs: Some(Box::new((*lhs).lower(lowerer))),
                rhs: Some(Box::new((*rhs).lower(lowerer))),
            })),
        };
        proto::Type { sum: Some(sum) }
    }
}
