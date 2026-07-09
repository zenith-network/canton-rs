use std::{
    fmt,
    hash::{Hash, Hasher},
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{
    DottedName, InterfaceMethod, Module, Package, TemplateChoice, Type, TypeConId,
};

#[derive(Clone, Copy)]
pub struct DefInterface<'a> {
    module: Module<'a>,
    index: usize,
}

impl<'a> DefInterface<'a> {
    pub fn as_unsealed(&self) -> &'a proto::DefInterface {
        &self.module.as_unsealed().interfaces[self.index]
    }

    pub fn module(&self) -> Module<'a> {
        self.module
    }

    pub fn package(&self) -> Package<'a> {
        self.module.package()
    }

    pub fn tycon_name(&self) -> DottedName<'a> {
        self.package()
            .get_interned_dotted_names(self.as_unsealed().tycon_interned_dname)
    }

    pub fn methods(&self) -> Vec<InterfaceMethod<'a>> {
        self.as_unsealed()
            .methods
            .iter()
            .map(|m| InterfaceMethod::from_unsealed(m, self.package()))
            .collect()
    }

    pub fn param(&self) -> &'a str {
        self.package()
            .get_interned_string(self.as_unsealed().param_interned_str)
    }

    pub fn choices(&self) -> Vec<TemplateChoice<'a>> {
        self.as_unsealed()
            .choices
            .iter()
            .map(|c| TemplateChoice::from_unsealed(c, self.package()))
            .collect()
    }

    pub fn view(&self) -> Type<'a> {
        Type::from_unsealed(self.as_unsealed().view.as_ref().unwrap(), self.package())
    }

    pub fn requires(&self) -> Vec<TypeConId<'a>> {
        self.as_unsealed()
            .requires
            .iter()
            .map(|r| TypeConId::from_unsealed(r, self.package()))
            .collect()
    }

    pub(crate) fn from_unsealed(module: Module<'a>, index: usize) -> Self {
        Self { module, index }
    }
}

impl fmt::Debug for DefInterface<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefInterface")
            .field("module", &format!("{:#?}", self.module))
            .field("tycon_name", &self.tycon_name())
            .field("methods", &self.methods())
            .field("param", &self.param())
            .field("choices", &self.choices())
            .field("view", &self.view())
            .field("requires", &self.requires())
            .finish()
    }
}

impl<'a, 'b> PartialEq<DefInterface<'b>> for DefInterface<'a> {
    fn eq(&self, other: &DefInterface<'b>) -> bool {
        self.module == other.module && self.index == other.index
    }
}

impl Eq for DefInterface<'_> {}

impl Hash for DefInterface<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.index.hash(state);
    }
}
