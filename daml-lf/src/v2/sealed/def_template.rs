use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{DottedName, Module, Package, TemplateChoice, Type, TypeConId};

#[derive(Clone, Copy)]
pub struct DefTemplate<'a> {
    module: Module<'a>,
    index: usize,
}

impl<'a> DefTemplate<'a> {
    pub fn as_unsealed(&self) -> &'a proto::DefTemplate {
        &self.module.as_unsealed().templates[self.index]
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

    pub fn param(&self) -> &'a str {
        self.package()
            .get_interned_string(self.as_unsealed().param_interned_str)
    }

    pub fn choices(&self) -> Vec<TemplateChoice<'a>> {
        self.as_unsealed()
            .choices
            .iter()
            .map(|unsealed| TemplateChoice::from_unsealed(unsealed, self.package()))
            .collect()
    }

    pub fn key(&self) -> Option<DefKey<'a>> {
        self.as_unsealed()
            .key
            .as_ref()
            .map(|unsealed| DefKey::from_unsealed(*self, unsealed))
    }

    pub fn implements(&self) -> Vec<Implements<'a>> {
        self.as_unsealed()
            .implements
            .iter()
            .map(|unsealed| Implements::from_unsealed(*self, unsealed))
            .collect()
    }

    pub(crate) fn from_unsealed(module: Module<'a>, index: usize) -> Self {
        Self { module, index }
    }
}

impl fmt::Debug for DefTemplate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefTemplate")
            .field("module", &format!("{:#?}", self.module))
            .field("tycon_name", &self.tycon_name())
            .field("param", &self.param())
            .field("choices", &self.choices())
            .field("key", &self.key())
            .field("implements", &self.implements())
            .finish()
    }
}

impl<'a, 'b> PartialEq<DefTemplate<'b>> for DefTemplate<'a> {
    fn eq(&self, other: &DefTemplate<'b>) -> bool {
        self.module == other.module && self.index == other.index
    }
}

impl Eq for DefTemplate<'_> {}

impl Hash for DefTemplate<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.index.hash(state);
    }
}

#[derive(Clone, Copy)]
pub struct DefKey<'a> {
    template: DefTemplate<'a>,
    unsealed: &'a proto::def_template::DefKey,
}

impl<'a> DefKey<'a> {
    pub fn as_unsealed(&self) -> &'a proto::def_template::DefKey {
        self.unsealed
    }

    pub fn template(&self) -> DefTemplate<'a> {
        self.template
    }

    pub fn package(&self) -> Package<'a> {
        self.template.package()
    }

    pub fn type_(&self) -> Type<'a> {
        Type::from_unsealed(self.unsealed.r#type.as_ref().unwrap(), self.package())
    }

    fn from_unsealed(template: DefTemplate<'a>, unsealed: &'a proto::def_template::DefKey) -> Self {
        Self { template, unsealed }
    }
}

impl fmt::Debug for DefKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefKey")
            .field("module", &format!("{:#?}", self.template.module()))
            .field("template", &self.template.tycon_name())
            .field("type", &self.type_())
            .finish()
    }
}

impl<'a, 'b> PartialEq<DefKey<'b>> for DefKey<'a> {
    fn eq(&self, other: &DefKey<'b>) -> bool {
        self.template == other.template && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for DefKey<'_> {}

impl Hash for DefKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct Implements<'a> {
    template: DefTemplate<'a>,
    unsealed: &'a proto::def_template::Implements,
}

impl<'a> Implements<'a> {
    pub fn as_unsealed(&self) -> &'a proto::def_template::Implements {
        self.unsealed
    }

    pub fn template(&self) -> DefTemplate<'a> {
        self.template
    }

    pub fn package(&self) -> Package<'a> {
        self.template.package()
    }

    pub fn interface(&self) -> TypeConId<'a> {
        TypeConId::from_unsealed(self.unsealed.interface.as_ref().unwrap(), self.package())
    }

    fn from_unsealed(
        template: DefTemplate<'a>,
        unsealed: &'a proto::def_template::Implements,
    ) -> Self {
        Self { template, unsealed }
    }
}

impl fmt::Debug for Implements<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Implements")
            .field("module", &format!("{:#?}", self.template.module()))
            .field("template", &self.template.tycon_name())
            .field("interface", &self.interface())
            .finish()
    }
}

impl<'a, 'b> PartialEq<Implements<'b>> for Implements<'a> {
    fn eq(&self, other: &Implements<'b>) -> bool {
        self.template == other.template && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Implements<'_> {}

impl Hash for Implements<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.template.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
