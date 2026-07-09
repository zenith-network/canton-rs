use std::{
    fmt,
    hash::{Hash, Hasher},
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;
use protobuf_utils::InvalidProtoField as _;

use crate::v2::{
    errors::{MalformedPackage, MalformedPackageContext as _},
    seal::seal_interned_dotted_name,
    sealed::{DefDataType, DefInterface, DefTemplate, DottedName, Package},
};

#[derive(Clone, Copy)]
pub struct Module<'a> {
    package: Package<'a>,
    index: usize,
}

impl<'a> Module<'a> {
    pub fn as_unsealed(&self) -> &'a proto::Module {
        &self.package.as_unsealed().modules[self.index]
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn name(&self) -> DottedName<'a> {
        self.package()
            .get_interned_dotted_names(self.as_unsealed().name_interned_dname)
    }

    pub fn data_types(&self) -> Vec<DefDataType<'a>> {
        (0..self.as_unsealed().data_types.len())
            .map(|index| DefDataType::from_unsealed(*self, index))
            .collect()
    }

    pub fn templates(&self) -> Vec<DefTemplate<'a>> {
        (0..self.as_unsealed().templates.len())
            .map(|index| DefTemplate::from_unsealed(*self, index))
            .collect()
    }

    pub fn interfaces(&self) -> Vec<DefInterface<'a>> {
        (0..self.as_unsealed().interfaces.len())
            .map(|index| DefInterface::from_unsealed(*self, index))
            .collect()
    }

    /// Seal all modules in the package
    pub(crate) fn seal_modules(package: &'a proto::Package) -> Result<(), MalformedPackage> {
        for module in &package.modules {
            Self::seal_module(module, package)?;
        }
        Ok(())
    }

    fn seal_module(
        module: &'a proto::Module,
        package: &'a proto::Package,
    ) -> Result<(), MalformedPackage> {
        seal_interned_dotted_name(module.name_interned_dname, package)
            .validated_of::<proto::Module>("name_interned_dname")
            .default_context()?;
        // TODO: seal everything
        Ok(())
    }

    pub(crate) fn from_unsealed(package: Package<'a>, index: usize) -> Self {
        Self { package, index }
    }
}

impl fmt::Debug for Module<'_> {
    /// If alternate flag is set, prints like `my-package-name@0.1.0:My.Module.Name`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#?}:{:?}", self.package, self.name())
        } else {
            f.debug_struct("Module")
                .field("package", &format_args!("{:#?}", self.package))
                .field("name", &self.name())
                .field("data_types", &self.data_types())
                .field("templates", &self.templates())
                .field("interfaces", &self.interfaces())
                .finish()
        }
    }
}

impl<'a, 'b> PartialEq<Module<'b>> for Module<'a> {
    fn eq(&self, other: &Module<'b>) -> bool {
        self.package == other.package && self.index == other.index
    }
}

impl Eq for Module<'_> {}

impl Hash for Module<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        self.index.hash(state);
    }
}
