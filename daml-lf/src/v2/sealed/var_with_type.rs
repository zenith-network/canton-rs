use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{Package, Type};

#[derive(Clone, Copy)]
pub struct VarWithType<'a> {
    package: Package<'a>,
    unsealed: &'a proto::VarWithType,
}

impl<'a> VarWithType<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::VarWithType, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }

    pub fn as_unsealed(&self) -> &'a proto::VarWithType {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn var(&self) -> &'a str {
        self.package
            .get_interned_string(self.unsealed.var_interned_str)
    }

    pub fn type_(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.r#type.as_ref().unwrap(), self.package)
    }
}

impl fmt::Debug for VarWithType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VarWithType")
            .field("package", &format_args!("{:#?}", self.package))
            .field("var", &self.var())
            .field("type", &self.type_())
            .finish()
    }
}

impl PartialEq for VarWithType<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for VarWithType<'_> {}

impl Hash for VarWithType<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
