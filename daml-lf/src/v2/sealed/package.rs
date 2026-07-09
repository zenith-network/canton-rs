use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::{
    errors::MalformedPackage,
    sealed::{DottedName, Kind, Module, PackageMetadata, Type},
};

/// Sealed package
#[derive(Clone, Copy)]
pub struct Package<'a> {
    unsealed: &'a proto::Package,
}

impl<'a> Package<'a> {
    /// Seal the package
    ///
    /// Note: sealing doesn't mean that the package is fully correct. It only checks interned indexes,
    /// but doesn't verify that the values are correct. Sealing doesn't mean a full type check.
    pub fn seal(unsealed: &'a proto::Package) -> Result<Self, MalformedPackage> {
        PackageMetadata::seal(unsealed)?;
        Module::seal_modules(unsealed)?;
        Ok(Package { unsealed })
    }

    pub fn as_unsealed(&self) -> &'a proto::Package {
        self.unsealed
    }

    pub fn metadata(&self) -> PackageMetadata<'a> {
        PackageMetadata::from_unsealed(*self)
    }

    pub fn modules(&self) -> Vec<Module<'a>> {
        (0..self.unsealed.modules.len())
            .map(|index| Module::from_unsealed(*self, index))
            .collect()
    }

    pub(crate) fn get_interned_string(&self, idx: i32) -> &'a str {
        self.unsealed.interned_strings.get(idx as usize).unwrap()
    }

    pub(crate) fn get_interned_dotted_names(&self, idx: i32) -> DottedName<'a> {
        DottedName::try_from_iter(
            self.unsealed
                .interned_dotted_names
                .get(idx as usize)
                .unwrap()
                .segments_interned_str
                .iter()
                .map(|idx| {
                    self.unsealed
                        .interned_strings
                        .get(*idx as usize)
                        .unwrap()
                        .as_str()
                }),
        )
        .expect("empty dotted name")
    }

    pub(crate) fn get_interned_kind(&self, idx: i32) -> Kind<'a> {
        // FIXME: add some protection against theoretical infinite recursion
        Kind::from_unsealed(
            self.unsealed.interned_kinds.get(idx as usize).unwrap(),
            *self,
        )
    }

    pub(crate) fn get_interned_type(&self, idx: i32) -> Type<'a> {
        // FIXME: add some protection against theoretical infinite recursion
        Type::from_unsealed(
            self.unsealed.interned_types.get(idx as usize).unwrap(),
            *self,
        )
    }

    pub(crate) fn get_import(&self, idx: i32) -> &'a str {
        use proto::package::ImportsSum;

        if let ImportsSum::PackageImports(imports) = self.unsealed.imports_sum.as_ref().unwrap() {
            imports.imported_packages.get(idx as usize).unwrap()
        } else {
            // Should be impossible on a sealed package
            unreachable!()
        }
    }
}

impl fmt::Debug for Package<'_> {
    /// With alternate flag will print like `my-package-name@0.1.0`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let metadata = self.metadata();
            write!(f, "{}@{}", metadata.name(), metadata.version())
        } else {
            f.debug_struct("Package")
                .field("metadata", &self.metadata())
                .field("modules", &self.modules())
                .finish()
        }
    }
}

impl<'a, 'b> PartialEq<Package<'b>> for Package<'a> {
    fn eq(&self, other: &Package<'b>) -> bool {
        ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Package<'_> {}

impl Hash for Package<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self.unsealed, state);
    }
}
