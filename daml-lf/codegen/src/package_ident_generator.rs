use std::collections::{BTreeMap, HashMap};

use canton_types::PackageId;
use daml_lf::package::{SealedPackage, VersionedSealedPackage};

/// Sealed package with generated Rust identifier
#[derive(Clone, Debug)]
pub struct PackageWithIdent<'a> {
    pub package: SealedPackage<'a>,
    pub ident: syn::Ident,
}

/// Generator of package identifiers
#[derive(Clone, Debug)]
pub struct PackageIdentGenerator<'a> {
    packages: BTreeMap<PackageId, SealedPackage<'a>>,
}

impl<'a> PackageIdentGenerator<'a> {
    pub fn new(packages: BTreeMap<PackageId, SealedPackage<'a>>) -> Self {
        Self { packages }
    }

    pub fn generate(self) -> BTreeMap<PackageId, PackageWithIdent<'a>> {
        let packages = self
            .packages
            .into_iter()
            .map(|(package_id, package)| {
                let (name, version) = Self::package_name_and_version(&package);
                (package_id, package, name.to_owned(), version.to_owned())
            })
            .collect::<Vec<_>>();

        let mut name_counts = HashMap::<String, usize>::new();
        let mut name_version_counts = HashMap::<(String, String), usize>::new();
        for (_, _, name, version) in &packages {
            *name_counts.entry(name.clone()).or_default() += 1;
            *name_version_counts
                .entry((name.clone(), version.clone()))
                .or_default() += 1;
        }

        packages
            .into_iter()
            .map(|(package_id, package, name, version)| {
                let name_version_count = name_version_counts[&(name.clone(), version.clone())];
                let ident = if name_counts[&name] == 1 {
                    crate::ident::generate_snake_ident(&name)
                } else if name_version_count == 1 {
                    crate::ident::generate_snake_ident(format!("{name}_{version}"))
                } else {
                    crate::ident::generate_snake_ident(format!(
                        "{name}_{version}_{}",
                        Self::short_package_id(&package_id),
                    ))
                };
                (package_id, PackageWithIdent { package, ident })
            })
            .collect::<BTreeMap<_, _>>()
    }

    fn package_name_and_version(sealed_package: &SealedPackage<'a>) -> (&'a str, &'a str) {
        match sealed_package.versioned() {
            #[cfg(feature = "v2")]
            VersionedSealedPackage::V2(package) => {
                let metadata = package.metadata();
                (metadata.name(), metadata.version())
            }
        }
    }

    fn short_package_id(package_id: &PackageId) -> &str {
        let package_id = package_id.as_str();
        match package_id.char_indices().nth(8) {
            Some((idx, _)) => &package_id[..idx],
            None => package_id,
        }
    }
}
