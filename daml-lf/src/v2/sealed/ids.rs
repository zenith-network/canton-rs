use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{DottedName, Package};

#[derive(Clone, Copy)]
pub struct TypeConId<'a> {
    package: Package<'a>,
    unsealed: &'a proto::TypeConId,
}

impl<'a> TypeConId<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::TypeConId, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }

    pub fn module(&self) -> ModuleId<'a> {
        ModuleId::from_unsealed(self.unsealed.module.as_ref().unwrap(), self.package)
    }

    pub fn name(&self) -> DottedName<'a> {
        self.package
            .get_interned_dotted_names(self.unsealed.name_interned_dname)
    }
}

impl fmt::Debug for TypeConId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeConId")
            .field("package", &format_args!("{:#?}", self.package))
            .field("module", &self.module())
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for TypeConId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for TypeConId<'_> {}

impl Hash for TypeConId<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct ModuleId<'a> {
    package: Package<'a>,
    unsealed: &'a proto::ModuleId,
}

impl<'a> ModuleId<'a> {
    pub(crate) fn from_unsealed(unsealed: &'a proto::ModuleId, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn package_id(&self) -> SelfOrImportedPackageId<'a> {
        SelfOrImportedPackageId::from_unsealed(self.unsealed.package_id.unwrap(), self.package)
    }

    pub fn module_name(&self) -> DottedName<'a> {
        self.package
            .get_interned_dotted_names(self.unsealed.module_name_interned_dname)
    }
}

impl fmt::Debug for ModuleId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleId")
            .field("package", &format_args!("{:#?}", self.package))
            .field("package_id", &self.package_id())
            .field("module_name", &self.module_name())
            .finish()
    }
}

impl PartialEq for ModuleId<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for ModuleId<'_> {}

impl Hash for ModuleId<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelfOrImportedPackageId<'a> {
    SelfPackageId,
    ImportedPackageId(&'a str),
}

impl<'a> SelfOrImportedPackageId<'a> {
    pub fn is_self(&self) -> bool {
        matches!(self, Self::SelfPackageId)
    }

    pub(crate) fn from_unsealed(
        unsealed: proto::SelfOrImportedPackageId,
        package: Package<'a>,
    ) -> Self {
        use proto::self_or_imported_package_id::Sum;

        match unsealed.sum.unwrap() {
            Sum::SelfPackageId(_) => Self::SelfPackageId,
            Sum::ImportedPackageIdInternedStr(idx) => {
                Self::ImportedPackageId(package.get_interned_string(idx))
            }
            Sum::PackageImportId(idx) => Self::ImportedPackageId(package.get_import(idx)),
        }
    }
}
