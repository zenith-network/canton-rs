use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::builder::{
    DataTypeBuilder, InterfaceBuilder, TemplateBuilder,
    lower::{Lower, Lowerer},
};

/// Builder for one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleBuilder {
    name: String,
    data_types: Vec<DataTypeBuilder>,
    templates: Vec<TemplateBuilder>,
    interfaces: Vec<InterfaceBuilder>,
}

impl ModuleBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_types: Vec::new(),
            templates: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    pub fn data_type(mut self, data_type: DataTypeBuilder) -> Self {
        self.data_types.push(data_type);
        self
    }

    pub fn template(mut self, template: TemplateBuilder) -> Self {
        self.templates.push(template);
        self
    }

    pub fn interface(mut self, interface: InterfaceBuilder) -> Self {
        self.interfaces.push(interface);
        self
    }
}

impl Lower<proto::Module> for ModuleBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::Module {
        proto::Module {
            name_interned_dname: lowerer.intern_dotted(&self.name),
            flags: Some(proto::FeatureFlags::default()),
            synonyms: Vec::new(),
            data_types: self
                .data_types
                .into_iter()
                .map(|data_type| data_type.lower(lowerer))
                .collect(),
            values: Vec::new(),
            templates: self
                .templates
                .into_iter()
                .map(|template| template.lower(lowerer))
                .collect(),
            exceptions: Vec::new(),
            interfaces: self
                .interfaces
                .into_iter()
                .map(|interface| interface.lower(lowerer))
                .collect(),
        }
    }
}

/// Builder for one package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageBuilder {
    name: String,
    version: String,
    modules: Vec<ModuleBuilder>,
}

impl PackageBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            modules: Vec::new(),
        }
    }

    pub fn module(mut self, module: ModuleBuilder) -> Self {
        self.modules.push(module);
        self
    }

    pub fn build(self) -> proto::Package {
        self.lower(&mut Lowerer::default())
    }
}

impl Lower<proto::Package> for PackageBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::Package {
        let name_interned_str = lowerer.intern_string(self.name);
        let version_interned_str = lowerer.intern_string(self.version);
        let modules = self
            .modules
            .into_iter()
            .map(|module| module.lower(lowerer))
            .collect();

        lowerer.finish_package(name_interned_str, version_interned_str, modules)
    }
}
