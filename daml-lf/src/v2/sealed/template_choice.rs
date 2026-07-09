use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{Package, Type, VarWithType};

#[derive(Clone, Copy)]
pub struct TemplateChoice<'a> {
    package: Package<'a>,
    unsealed: &'a proto::TemplateChoice,
}

impl<'a> TemplateChoice<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::TemplateChoice, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }

    pub fn as_unsealed(&self) -> &'a proto::TemplateChoice {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn name(&self) -> &'a str {
        self.package
            .get_interned_string(self.unsealed.name_interned_str)
    }

    pub fn consuming(&self) -> bool {
        self.unsealed.consuming
    }

    pub fn arg_binder(&self) -> VarWithType<'a> {
        VarWithType::from_unsealed(self.unsealed.arg_binder.as_ref().unwrap(), self.package)
    }

    pub fn ret_type(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.ret_type.as_ref().unwrap(), self.package)
    }

    pub fn self_binder(&self) -> &'a str {
        self.package()
            .get_interned_string(self.unsealed.self_binder_interned_str)
    }
}

impl fmt::Debug for TemplateChoice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VarWithType")
            .field("package", &format_args!("{:#?}", self.package))
            .field("name", &self.name())
            .field("consuming", &self.consuming())
            .field("arg_binder", &self.arg_binder())
            .field("ret_type", &self.ret_type())
            .field("self_binder", &self.self_binder())
            .finish()
    }
}

impl PartialEq for TemplateChoice<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for TemplateChoice<'_> {}

impl Hash for TemplateChoice<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
