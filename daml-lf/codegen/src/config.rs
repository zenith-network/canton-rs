use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::external_paths::UnresolvedExternalPaths;

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
    /// Supported formats of `package_id_or_name`:
    ///
    /// - `#my_package_name`
    /// - `#my_package_name@v0.1.0`
    /// - `mypackageidffff`
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
}
