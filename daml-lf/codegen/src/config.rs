use std::path::{self, Path, PathBuf};

use crate::external_paths::{ExternalPathsError, UnresolvedExternalPaths};

#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) outdir: Option<PathBuf>,
    pub(crate) external: UnresolvedExternalPaths,
    pub(crate) sdk_types: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            outdir: Default::default(),
            external: Default::default(),
            sdk_types: true,
        }
    }
}

impl Config {
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

    pub fn get_outdir(&self) -> Result<PathBuf, ConfigError> {
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("output directory is not specified")]
    OutdirNotSet,
    #[error(transparent)]
    InvalidIdentifier(ExternalPathsError),
    #[error(transparent)]
    AbsolutePathError(std::io::Error),
}
