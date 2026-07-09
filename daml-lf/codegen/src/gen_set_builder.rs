use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use canton_types::{NonEmpty, PackageId};
use daml_lf::package::{SealedPackage, VersionedSealedPackage};

use crate::{
    ids::IdentifierWithinPackage,
    type_sets::{PackageTypeSet, TypeSet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenMode {
    Full,
    ResolveTemplates,
}

/// Set of types to generate
pub struct GenSetBuilder;

impl GenSetBuilder {
    pub fn build(
        packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        main_package_id: PackageId,
        mode: GenMode,
    ) -> TypeSet {
        match mode {
            GenMode::Full => Self::build_full(packages, main_package_id),
            GenMode::ResolveTemplates => Self::build_resolve_templates(packages, main_package_id),
        }
    }

    fn build_full(
        _packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        _main_package_id: PackageId,
    ) -> TypeSet {
        todo!()
    }

    fn build_resolve_templates(
        packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        main_package_id: PackageId,
    ) -> TypeSet {
        let mut gen_set = TypeSet::new();

        let main_package = &packages[&main_package_id];

        let (main_gen_set, external_deps) =
            PackageGenSetBuilder::build(main_package, PackageGenMode::ResolveTemplates);
        gen_set.0.insert(main_package_id, main_gen_set);

        for (package_id, deps) in external_deps.0 {
            let package = &packages[&package_id];
            let x = PackageGenSetBuilder::build(package, PackageGenMode::FromRoots(todo!()));
        }

        gen_set
    }
}

pub enum PackageGenMode {
    Full,
    ResolveTemplates,
    /// module ID -> type IDs
    FromRoots(PackageTypeSet),
}

/// Set of types to generate within a single package (module ID -> module gen set)
pub struct PackageGenSetBuilder;

impl PackageGenSetBuilder {
    fn build(package: &SealedPackage<'_>, mode: PackageGenMode) -> (PackageTypeSet, TypeSet) {
        match package.versioned() {
            #[cfg(feature = "v2")]
            VersionedSealedPackage::V2(package) => {
                use crate::v2::package_gen_set_builder::PackageGenSetBuilder;

                PackageGenSetBuilder::build(package, mode)
            }
        }
    }
}
