use std::collections::BTreeMap;

use canton_types::PackageId;
use daml_lf::package::{SealedPackage, VersionedSealedPackage};

use crate::type_sets::{PackageTypeSet, TypeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenMode {
    ResolveTemplates,
}

pub struct GenSetBuilder;

impl GenSetBuilder {
    pub fn build(
        packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        main_package_id: PackageId,
        mode: GenMode,
    ) -> TypeSet {
        match mode {
            GenMode::ResolveTemplates => Self::build_resolve_templates(packages, main_package_id),
        }
    }

    fn build_resolve_templates(
        packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        main_package_id: PackageId,
    ) -> TypeSet {
        let mut genset = TypeSet::new();

        let main_package = &packages[&main_package_id];

        let PackageGenSetResult {
            genset: main_genset,
            external_deps,
        } = PackageGenSetBuilder::build(main_package, PackageGenMode::ResolveTemplates);
        genset.as_mut().insert(main_package_id, main_genset);

        Self::build_from_deps(packages, &mut genset, external_deps);

        genset
    }

    fn build_from_deps(
        packages: &BTreeMap<PackageId, SealedPackage<'_>>,
        genset: &mut TypeSet,
        deps: TypeSet,
    ) {
        for (package_id, deps) in deps.0 {
            let package = &packages[&package_id];

            let PackageGenSetResult {
                genset: package_genset,
                external_deps: package_external_deps,
            } = PackageGenSetBuilder::build(package, PackageGenMode::FromRoots(deps));

            if let Some(existing_genset) = genset.as_mut().get_mut(&package_id) {
                existing_genset.join(package_genset);
            } else {
                genset.as_mut().insert(package_id, package_genset);
            }

            Self::build_from_deps(packages, genset, package_external_deps);
        }
    }
}

pub enum PackageGenMode {
    ResolveTemplates,
    FromRoots(PackageTypeSet),
}

pub struct PackageGenSetBuilder;

pub struct PackageGenSetResult {
    pub genset: PackageTypeSet,

    /// Dependencies from other packages
    pub external_deps: TypeSet,
}

impl PackageGenSetBuilder {
    fn build(package: &SealedPackage<'_>, mode: PackageGenMode) -> PackageGenSetResult {
        match package.versioned() {
            #[cfg(feature = "v2")]
            VersionedSealedPackage::V2(package) => {
                use crate::v2::package_gen_set_builder::PackageGenSetBuilder;

                PackageGenSetBuilder::build(package, mode)
            }
        }
    }
}
