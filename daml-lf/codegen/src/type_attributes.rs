use std::collections::HashMap;

use canton_types::{DottedName, PackageId, PackageName, errors::DottedNameError};
use daml_lf::package::SealedPackage;

use crate::{
    dispatcher::Packages,
    package_ref::{InvalidPackageRef, PackageRef},
};

#[derive(Debug, thiserror::Error)]
pub enum TypeAttrError {
    #[error(transparent)]
    DottedNameError(#[from] DottedNameError),

    #[error(transparent)]
    InvalidPackageRef(#[from] InvalidPackageRef),

    #[error("package not found: '{0}'")]
    PackageNotFound(PackageRef),

    #[error("ambiguous package reference: '{0}'")]
    AmbiguousPackageRef(PackageRef),

    #[error("invalid attribute: '{content}'")]
    InvalidAttr {
        content: String,

        #[source]
        source: syn::Error,
    },
}

impl TypeAttrError {
    pub fn invalid_attr(content: impl Into<String>, source: syn::Error) -> Self {
        Self::InvalidAttr {
            content: content.into(),
            source,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeAttributes {
    // The order is not needed here, so we use hashmap here
    inner: HashMap<PackageId, HashMap<DottedName, HashMap<DottedName, Vec<syn::Attribute>>>>,
}

impl TypeAttributes {
    /// Create new empty type attributes
    #[allow(
        dead_code,
        reason = "Currently only used in testing, but it's good to have this"
    )]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Resolve type attributes against existing packages.
    ///
    /// Turn names+versions into package IDs. Return error on invalid specification.
    pub fn resolve(
        raw: &HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
        existing: &Packages<'_>,
    ) -> Result<Self, TypeAttrError> {
        TypeAttrsResolver::new(existing).resolve(raw)
    }

    /// Get map of all attributes for given package
    pub fn get_package(
        &self,
        package_id: &PackageId,
    ) -> Option<&HashMap<DottedName, HashMap<DottedName, Vec<syn::Attribute>>>> {
        self.inner.get(package_id)
    }

    /// Get map of all attributes for given module
    pub fn get_module(
        &self,
        package_id: &PackageId,
        module: &DottedName,
    ) -> Option<&HashMap<DottedName, Vec<syn::Attribute>>> {
        self.get_package(package_id).and_then(|p| p.get(module))
    }

    /// Get attributes for given entity
    pub fn get_entity(
        &self,
        package_id: &PackageId,
        module: &DottedName,
        entity: &DottedName,
    ) -> Option<&[syn::Attribute]> {
        self.get_module(package_id, module)
            .and_then(|m| m.get(entity))
            .map(Vec::as_slice)
    }
}

impl AsRef<HashMap<PackageId, HashMap<DottedName, HashMap<DottedName, Vec<syn::Attribute>>>>>
    for TypeAttributes
{
    fn as_ref(
        &self,
    ) -> &HashMap<PackageId, HashMap<DottedName, HashMap<DottedName, Vec<syn::Attribute>>>> {
        &self.inner
    }
}

struct TypeAttrsResolver<'a> {
    packages: &'a Packages<'a>,
}

impl<'a> TypeAttrsResolver<'a> {
    fn new(packages: &'a Packages<'a>) -> Self {
        Self { packages }
    }

    fn resolve(
        &self,
        raw: &HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
    ) -> Result<TypeAttributes, TypeAttrError> {
        let mut inner: HashMap<
            PackageId,
            HashMap<DottedName, HashMap<DottedName, Vec<syn::Attribute>>>,
        > = HashMap::new();

        for (raw_package_id_or_name, modules) in raw {
            let package_ref = PackageRef::parse_str(raw_package_id_or_name)?;
            let package_id = self.resolve_package_ref(package_ref)?;
            let package_attrs = inner.entry(package_id).or_default();

            for (raw_module, entities) in modules {
                let module = DottedName::parse(raw_module)?;
                let module_attrs = package_attrs.entry(module).or_default();

                for (raw_entity, raw_attrs) in entities {
                    let entity = DottedName::parse(raw_entity)?;

                    let entity_attrs = module_attrs.entry(entity).or_default();
                    for raw_attr in raw_attrs {
                        entity_attrs.extend(parse_type_attributes(raw_attr)?);
                    }
                }
            }
        }

        Ok(TypeAttributes { inner })
    }

    fn resolve_package_ref(&self, package_ref: PackageRef) -> Result<PackageId, TypeAttrError> {
        match package_ref {
            PackageRef::Id(package_id) => self.resolve_package_id(package_id),
            PackageRef::Name { name, version } => self.resolve_package_name(name, version),
        }
    }

    fn resolve_package_id(&self, package_id: PackageId) -> Result<PackageId, TypeAttrError> {
        if self.packages.contains_key(&package_id) {
            Ok(package_id)
        } else {
            Err(TypeAttrError::PackageNotFound(PackageRef::Id(package_id)))
        }
    }

    fn resolve_package_name(
        &self,
        package_name: PackageName,
        package_version: Option<String>,
    ) -> Result<PackageId, TypeAttrError> {
        let mut matches = self.packages.iter().filter_map(|(package_id, package)| {
            let (name, version) = get_package_name_and_version(&package.package);

            (name == package_name.as_str()
                && package_version
                    .as_ref()
                    .is_none_or(|package_version| package_version == version))
            .then(|| package_id.clone())
        });

        if let Some(package_id) = matches.next() {
            if matches.next().is_some() {
                Err(TypeAttrError::AmbiguousPackageRef(PackageRef::Name {
                    name: package_name,
                    version: package_version,
                }))
            } else {
                Ok(package_id)
            }
        } else {
            Err(TypeAttrError::PackageNotFound(PackageRef::Name {
                name: package_name,
                version: package_version,
            }))
        }
    }
}

fn get_package_name_and_version<'a>(package: &SealedPackage<'a>) -> (&'a str, &'a str) {
    match package.versioned() {
        #[cfg(feature = "v2")]
        daml_lf::package::VersionedSealedPackage::V2(package) => {
            let metadata = package.metadata();
            (metadata.name(), metadata.version())
        }
    }
}

fn parse_type_attributes(input: &str) -> Result<Vec<syn::Attribute>, TypeAttrError> {
    syn::parse::Parser::parse_str(syn::Attribute::parse_outer, input)
        .map_err(|error| TypeAttrError::invalid_attr(input, error))
}
