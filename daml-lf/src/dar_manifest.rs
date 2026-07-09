mod parser;

use parser::{ParseError, parse_manifest};

/// DAR manifest file representation
///
/// Manifest files are normally stored under `META-INF/MANIFEST.MF` path in `.dar`. They are
/// syntactically compatible with JAR manifest files. However they follow their own semantic rules
/// about the attributes names.
///
/// Reference: https://docs.oracle.com/en/java/javase/24/docs/specs/jar/jar.html#jar-manifest
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DarManifest {
    /// Version of the manifest file
    ///
    /// Optional, but normally set to `1.0`.
    pub version: Option<String>,

    /// Name of the entiry, which created the manifest
    ///
    /// For compiled `.dar` files normally set to `damlc`.
    pub created_by: Option<String>,

    /// Name of the archive
    ///
    /// For compiler `.dar` normally set to the name of the archive file.
    pub name: Option<String>,

    /// Version of the Daml SDK
    pub sdk_version: Option<String>,

    /// Path to the main `.dalf` file in the `.dar`
    pub main_dalf: String,

    /// Paths to all `.dalf` files in the `.dar` (including the main one)
    pub dalfs: Vec<String>,
}

impl DarManifest {
    /// Parse DAR manifest file from string slice
    pub fn parse(source: &str) -> Result<Self, DarManifestError> {
        parse_manifest(source).map_err(Into::into)
    }
}

/// Error during manifest parsing
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct DarManifestError {
    #[from]
    inner: ParseError,
}
