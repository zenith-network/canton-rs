use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{BuiltinType, Package, TypeConId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type<'a> {
    Var(Var<'a>),
    Con(Con<'a>),
    Builtin(Builtin<'a>),
    // Forall(::prost::alloc::boxed::Box<Forall>),
    // Struct(Struct),
    Nat,
    // Syn(Syn),
    Tapp(TApp<'a>),
    // TODO: complete variants
}

impl<'a> Type<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::Type, package: Package<'a>) -> Self {
        use proto::r#type::Sum;

        match unsealed.sum.as_ref().unwrap() {
            Sum::Var(unsealed) => Self::Var(Var { unsealed, package }),
            Sum::Con(unsealed) => Self::Con(Con { package, unsealed }),
            Sum::Builtin(unsealed) => Self::Builtin(Builtin {
                package,
                unsealed,
                type_: proto::BuiltinType::try_from(unsealed.builtin)
                    .unwrap()
                    .into(),
            }),
            Sum::Forall(_) => todo!(),
            Sum::Struct(_) => todo!(),
            Sum::Nat(_) => Self::Nat,
            Sum::Syn(_) => todo!(),
            Sum::InternedType(idx) => package.get_interned_type(*idx),
            Sum::Tapp(unsealed) => Self::Tapp(TApp { package, unsealed }),
        }
    }

    /// Return type constructor ID if some
    pub fn type_con_id(&self) -> Option<TypeConId<'a>> {
        match self {
            Type::Con(con) => Some(con.tycon()),
            Type::Tapp(tapp) => tapp.lhs().type_con_id(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Var<'a> {
    package: Package<'a>,
    unsealed: &'a proto::r#type::Var,
}

impl<'a> Var<'a> {
    pub fn as_unsealed(&self) -> &'a proto::r#type::Var {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn var(&self) -> &'a str {
        self.package
            .get_interned_string(self.unsealed.var_interned_str)
    }

    pub fn args(&self) -> Vec<Type<'a>> {
        self.unsealed
            .args
            .iter()
            .map(|t| Type::from_unsealed(t, self.package))
            .collect()
    }
}

impl fmt::Debug for Var<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Var")
            .field("package", &format_args!("{:#?}", self.package))
            .field("var", &self.var())
            .field("args", &self.args())
            .finish()
    }
}

impl PartialEq for Var<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Var<'_> {}

impl Hash for Var<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct Con<'a> {
    package: Package<'a>,
    unsealed: &'a proto::r#type::Con,
}

impl<'a> Con<'a> {
    pub fn as_unsealed(&self) -> &'a proto::r#type::Con {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn tycon(&self) -> TypeConId<'a> {
        TypeConId::from_unsealed(self.unsealed.tycon.as_ref().unwrap(), self.package)
    }

    pub fn args(&self) -> Vec<Type<'a>> {
        self.unsealed
            .args
            .iter()
            .map(|t| Type::from_unsealed(t, self.package))
            .collect()
    }
}

impl fmt::Debug for Con<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Con")
            .field("package", &format_args!("{:#?}", self.package))
            .field("tycon", &self.tycon())
            .field("args", &self.args())
            .finish()
    }
}

impl PartialEq for Con<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Con<'_> {}

impl Hash for Con<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct Builtin<'a> {
    package: Package<'a>,
    unsealed: &'a proto::r#type::Builtin,
    type_: BuiltinType,
}

impl<'a> Builtin<'a> {
    pub fn as_unsealed(&self) -> &'a proto::r#type::Builtin {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn type_(&self) -> BuiltinType {
        self.type_
    }

    pub fn args(&self) -> Vec<Type<'a>> {
        if self.type_ == BuiltinType::Numeric {
            Vec::new()
        } else {
            self.unsealed
                .args
                .iter()
                .map(|t| Type::from_unsealed(t, self.package))
                .collect()
        }
    }
}

impl fmt::Debug for Builtin<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builtin")
            .field("package", &format_args!("{:#?}", self.package))
            .field("type", &self.type_)
            .field("args", &self.args())
            .finish()
    }
}

impl PartialEq for Builtin<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Builtin<'_> {}

impl Hash for Builtin<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct TApp<'a> {
    package: Package<'a>,
    unsealed: &'a proto::r#type::TApp,
}

impl<'a> TApp<'a> {
    pub fn as_unsealed(&self) -> &'a proto::r#type::TApp {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn lhs(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.lhs.as_ref().unwrap(), self.package)
    }

    pub fn rhs(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.rhs.as_ref().unwrap(), self.package)
    }
}

impl fmt::Debug for TApp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TApp")
            .field("package", &format_args!("{:#?}", self.package))
            .field("lhs", &self.lhs())
            .field("rhs", &self.rhs())
            .finish()
    }
}

impl PartialEq for TApp<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for TApp<'_> {}

impl Hash for TApp<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
