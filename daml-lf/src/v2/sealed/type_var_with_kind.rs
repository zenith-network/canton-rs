use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{Kind, Package};

// TODO: is it actually a part of a module?

#[derive(Clone, Copy)]
pub struct TypeVarWithKind<'a> {
    package: Package<'a>,
    type_var_with_kind: &'a proto::TypeVarWithKind,
}

impl<'a> TypeVarWithKind<'a> {
    pub fn as_unsealed(&self) -> &'a proto::TypeVarWithKind {
        self.type_var_with_kind
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn var(&self) -> &'a str {
        self.package
            .get_interned_string(self.as_unsealed().var_interned_str)
    }

    pub fn kind(&self) -> Kind<'a> {
        Kind::from_unsealed(self.type_var_with_kind.kind.as_ref().unwrap(), self.package)
    }

    pub(crate) fn from_unsealed(
        type_var_with_kind: &'a proto::TypeVarWithKind,
        package: Package<'a>,
    ) -> Self {
        Self {
            package,
            type_var_with_kind,
        }
    }
}

impl fmt::Debug for TypeVarWithKind<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeVarWithKind")
            .field("package", &format_args!("{:#?}", self.package))
            .field("var", &self.var())
            .field("kind", &self.kind())
            .finish()
    }
}

impl PartialEq for TypeVarWithKind<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.type_var_with_kind, other.type_var_with_kind)
    }
}

impl Eq for TypeVarWithKind<'_> {}

impl Hash for TypeVarWithKind<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.type_var_with_kind, state);
    }
}
