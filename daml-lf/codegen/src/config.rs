use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{self, Path, PathBuf},
};

use canton_types::{DottedName, NonEmpty, PackageId, PackageIdAny, PackageName};
use daml_lf::package::SealedPackage;
use syn::parse::Parser as _;

use crate::external_paths::{ExternalPathsError, UnresolvedExternalPaths};

#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) outdir: Option<PathBuf>,
    pub(crate) external: UnresolvedExternalPaths,
    pub(crate) sdk_types: bool,
    pub(crate) type_attrs: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            outdir: Default::default(),
            external: Default::default(),
            sdk_types: true,
            type_attrs: Default::default(),
        }
    }
}

impl Config {
    /// Create a new default config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set output directory
    pub fn outdir(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.outdir = Some(path.as_ref().to_path_buf());
        self
    }

    /// For Daml SDK types use corresponding Rust std types (instead of code-generating them)
    ///
    /// Enabled by default
    pub fn sdk_types(&mut self, enable: bool) -> &mut Self {
        self.sdk_types = enable;
        self
    }

    /// Supported formats of `package_id_or_name`:
    ///
    /// - `#my_package_name`
    /// - `#my_package_name@v0.1.0`
    /// - `mypackageidffff`
    ///
    /// If provided identifier causes collision, an error will be returned on generation.
    pub fn extern_package(
        &mut self,
        package_id_or_name: impl Into<String>,
        target: impl Into<String>,
    ) -> &mut Self {
        self.external.extern_package(package_id_or_name, target);
        self
    }

    /// `module` must be a dotted name
    ///
    /// See [`Self::extern_package`] for `package_id_or_name` format
    pub fn extern_module(
        &mut self,
        package_id_or_name: impl Into<String>,
        module: impl Into<String>,
        target: impl Into<String>,
    ) -> &mut Self {
        self.external
            .extern_module(package_id_or_name, module, target);
        self
    }

    /// `entity` must be a dotted name
    ///
    /// See [`Self::extern_module`] for other details
    pub fn extern_entity(
        &mut self,
        package_id_or_name: impl Into<String>,
        module: impl Into<String>,
        entity: impl Into<String>,
        target: impl Into<String>,
    ) -> &mut Self {
        self.external
            .extern_entity(package_id_or_name, module, entity, target);
        self
    }

    /// Add specific attribute to a generated type
    ///
    /// # Example
    pub fn type_attribute(
        &mut self,
        package_id_or_name: impl Into<String>,
        module: impl Into<String>,
        entity: impl Into<String>,
        attribute: impl Into<String>,
    ) -> &mut Self {
        let module = module.into();
        let entity = entity.into();
        let attribute = attribute.into();
        self.type_attrs
            .entry(package_id_or_name.into())
            .and_modify(|modules| {
                modules
                    .entry(module.clone())
                    .and_modify(|entities| {
                        entities
                            .entry(entity.clone())
                            .and_modify(|attrs| attrs.push(attribute.clone()))
                            .or_insert_with(|| vec![attribute.clone()]);
                    })
                    .or_insert_with(|| [(entity.clone(), vec![attribute.clone()])].into());
            })
            .or_insert_with(|| [(module, [(entity, vec![attribute])].into())].into());
        self
    }

    pub(crate) fn resolve_type_attributes(
        &self,
        packages: &BTreeMap<PackageId, SealedPackage>,
    ) -> Result<
        HashMap<
            PackageId,
            HashMap<NonEmpty<String>, HashMap<NonEmpty<String>, Vec<syn::Attribute>>>,
        >,
        ConfigError,
    > {
        let mut resolved = HashMap::<
            PackageId,
            HashMap<NonEmpty<String>, HashMap<NonEmpty<String>, Vec<syn::Attribute>>>,
        >::new();

        for (raw_package_id_or_name, modules) in &self.type_attrs {
            let package_id = resolve_package_id(raw_package_id_or_name, packages)?;
            let package_attrs = resolved.entry(package_id).or_default();

            for (raw_module, entities) in modules {
                let module = parse_dotted_name("module", raw_module)?;
                let module_attrs = package_attrs.entry(module).or_default();

                for (raw_entity, raw_attrs) in entities {
                    let entity = parse_dotted_name("entity", raw_entity)?;

                    let entity_attrs = module_attrs.entry(entity).or_default();
                    for raw_attr in raw_attrs {
                        entity_attrs.push(parse_type_attribute(raw_attr)?);
                    }
                }
            }
        }

        Ok(resolved)
    }

    pub(crate) fn get_outdir(&self) -> Result<PathBuf, ConfigError> {
        path::absolute(resolve_outdir(self.outdir.clone())?).map_err(ConfigError::AbsolutePathError)
    }
}

/// If path is not set and `"env"` feature is enabled, try to get the path from `OUT_DIR` env var
pub fn resolve_outdir(
    #[allow(unused_mut)] mut path: Option<PathBuf>,
) -> Result<PathBuf, ConfigError> {
    #[cfg(feature = "env")]
    {
        path = path.or_else(|| std::env::var("OUT_DIR").ok().map(PathBuf::from));
    }
    path.ok_or(ConfigError::OutdirNotSet)
}

fn resolve_package_id(
    package_id_or_name: &str,
    packages: &BTreeMap<PackageId, SealedPackage<'_>>,
) -> Result<PackageId, TypeAttrError> {
    if let Some(raw_name) = package_id_or_name.strip_prefix('#') {
        if let Some((raw_name, raw_version)) = raw_name.split_once('@') {
            if raw_version.is_empty() {
                return Err(TypeAttrError::package_reference(
                    package_id_or_name,
                    "missing version after `@`",
                ));
            }

            let package_name = PackageName::new(raw_name.to_owned())
                .map_err(|source| TypeAttrError::package_reference(package_id_or_name, source))?;

            return resolve_package_name(
                package_id_or_name,
                &package_name,
                Some(raw_version),
                packages,
            );
        }

        let package_name = PackageName::new(raw_name.to_owned())
            .map_err(|source| TypeAttrError::package_reference(package_id_or_name, source))?;

        return resolve_package_name(package_id_or_name, &package_name, None, packages);
    }

    let package_id_any = PackageIdAny::parse(package_id_or_name)
        .map_err(|source| TypeAttrError::package_reference(package_id_or_name, source))?;

    match package_id_any {
        PackageIdAny::Id(package_id) => packages
            .contains_key(&package_id)
            .then_some(package_id)
            .ok_or_else(|| TypeAttrError::unknown_package_reference(package_id_or_name)),
        PackageIdAny::Name(package_name) => {
            resolve_package_name(package_id_or_name, &package_name, None, packages)
        }
    }
}

fn resolve_package_name(
    raw_package_id_or_name: &str,
    package_name: &PackageName,
    package_version: Option<&str>,
    packages: &BTreeMap<PackageId, SealedPackage<'_>>,
) -> Result<PackageId, TypeAttrError> {
    let matches = packages
        .iter()
        .filter_map(|(package_id, package)| {
            let (name, version) = package_name_and_version(package);
            (name == package_name.as_str()
                && package_version.is_none_or(|package_version| package_version == version))
            .then(|| package_id.clone())
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(TypeAttrError::unknown_package_reference(
            raw_package_id_or_name,
        )),
        [package_id] => Ok(package_id.clone()),
        _ => Err(TypeAttrError::ambiguous_package_reference(
            raw_package_id_or_name,
            &matches,
        )),
    }
}

fn package_name_and_version<'a>(package: &SealedPackage<'a>) -> (&'a str, &'a str) {
    match package.versioned() {
        daml_lf::package::VersionedSealedPackage::V2(package) => {
            let metadata = package.metadata();
            (metadata.name(), metadata.version())
        }
    }
}

fn parse_dotted_name(kind: &'static str, input: &str) -> Result<NonEmpty<String>, ConfigError> {
    let dotted_name = DottedName::parse(input)
        .map_err(|source| TypeAttrError::invalid(kind, input, source))?;
    let segments = dotted_name.segments();
    Ok(NonEmpty {
        base: segments.base.iter().map(ToString::to_string).collect(),
        tail: segments.tail.to_string(),
    })
}

fn parse_type_attribute(input: &str) -> Result<syn::Attribute, TypeAttrError> {
    let input = input.trim();
    parse_single_attribute(input, syn::Attribute::parse_outer).or_else(|outer_error| {
        parse_single_attribute(input, syn::Attribute::parse_inner).or(Err(outer_error))
    })
}

fn parse_single_attribute(
    input: &str,
    parser: fn(syn::parse::ParseStream<'_>) -> syn::Result<Vec<syn::Attribute>>,
) -> Result<syn::Attribute, TypeAttrError> {
    let parsed = parser
        .parse_str(input)
        .map_err(|source| TypeAttrError::invalid("attribute", input, source))?;

    match parsed.as_slice() {
        [attribute] => Ok(attribute.clone()),
        _ => Err(TypeAttrError::invalid(
            "attribute",
            input,
            format!("expected a single attribute, found {}", parsed.len()),
        )),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("output directory is not specified")]
    OutdirNotSet,
    #[error(transparent)]
    TypeAttrError(#[from] TypeAttrError),
    #[error(transparent)]
    InvalidIdentifier(ExternalPathsError),
    #[error(transparent)]
    AbsolutePathError(std::io::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TypeAttrError(String);

impl TypeAttrError {
    fn invalid(kind: &'static str, value: &str, message: impl fmt::Display) -> Self {
        Self(format!(
            "invalid type attribute {kind} {value:?}: {message}"
        ))
    }

    fn package_reference(input: &str, message: impl fmt::Display) -> Self {
        Self(format!(
            "invalid type attribute package reference {input:?}: {message}"
        ))
    }

    fn unknown_package_reference(input: &str) -> Self {
        Self(format!(
            "type attribute package reference {input:?} does not match any package in the DAR"
        ))
    }

    fn ambiguous_package_reference(input: &str, matches: &[PackageId]) -> Self {
        Self(format!(
            "type attribute package reference {input:?} is ambiguous and matches packages {matches:?}"
        ))
    }
}
