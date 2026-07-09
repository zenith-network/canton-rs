use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{Package, Type};

#[derive(Clone, Copy)]
pub struct InterfaceMethod<'a> {
    package: Package<'a>,
    unsealed: &'a proto::InterfaceMethod,
}

impl<'a> InterfaceMethod<'a> {
    pub fn as_unsealed(&self) -> &'a proto::InterfaceMethod {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn name(&self) -> &'a str {
        self.package()
            .get_interned_string(self.unsealed.method_interned_name)
    }

    pub fn type_(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.r#type.as_ref().unwrap(), self.package())
    }

    pub(crate) fn from_unsealed(
        unsealed: &'a proto::InterfaceMethod,
        package: Package<'a>,
    ) -> Self {
        Self { package, unsealed }
    }
}

impl fmt::Debug for InterfaceMethod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterfaceMethod")
            .field("package", &format_args!("{:#?}", self.package))
            .field("name", &self.name())
            .field("type", &self.type_())
            .finish()
    }
}

impl PartialEq for InterfaceMethod<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for InterfaceMethod<'_> {}

impl Hash for InterfaceMethod<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
