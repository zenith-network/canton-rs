use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek},
    path::Path,
};

use zip::{ZipArchive, result::ZipError};

use crate::{
    dalf::{DalfError, DalfFile},
    dar_manifest::{DarManifest, DarManifestError},
};

pub const MANIFEST_FILENAME: &str = "META-INF/MANIFEST.MF";

/// Adapter for a `.dar` file
pub struct DarFile<R = File> {
    manifest: DarManifest,
    reader: ZipArchive<R>,
}

impl<R> fmt::Debug for DarFile<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DarFile")
            .field("manifest", &self.manifest)
            .finish()
    }
}

impl<R> DarFile<R> {
    /// Manifest of a DAR file
    pub fn manifest(&self) -> &DarManifest {
        &self.manifest
    }
}

impl<R: Read + Seek> DarFile<R> {
    /// Get main DALF file
    pub fn main_dalf(&mut self) -> Result<DalfFile, DarError> {
        let reader = self
            .reader
            .by_path(&self.manifest.main_dalf)
            .map_err(|err| DarError::get_dalf_error(err, "failed to get main DALF file"))?;
        DalfFile::read_from(reader).map_err(Into::into)
    }

    /// Get all DALF files (including main DALF)
    pub fn dalfs(&mut self) -> Result<Vec<DalfFile>, DarError> {
        self.manifest
            .dalfs
            .iter()
            .map(|path| {
                let reader = self.reader.by_path(path).map_err(|err| {
                    DarError::get_dalf_error(err, format!("failed to get DALF file: '{path}'"))
                })?;
                DalfFile::read_from(reader).map_err(Into::into)
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

impl DarFile<File> {
    /// Read DAR from file
    pub fn read_from(path: impl AsRef<Path>) -> Result<Self, DarError> {
        let file =
            File::open(path).map_err(|err| DarError::io_error(err, "failed to open DAR file"))?;
        let mut reader = ZipArchive::new(file)?;

        let mut buf = String::new();
        reader
            .by_path(MANIFEST_FILENAME)
            .map_err(|err| match err {
                ZipError::FileNotFound => DarError::MissingManifest(err),
                err => DarError::ZipError(err),
            })?
            .read_to_string(&mut buf)
            .map_err(|err| DarError::io_error(err, "failed to read manifest file"))?;

        let manifest = DarManifest::parse(&buf)?;

        Ok(Self { manifest, reader })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DarError {
    #[error("manifest file is missing inside .dar file")]
    MissingManifest(#[source] ZipError),

    #[error("failed to read .dar file as ZIP archive")]
    ZipError(#[from] ZipError),

    #[error("{message}")]
    GetDalfError {
        message: String,
        #[source]
        source: ZipError,
    },

    #[error("{message}")]
    IOError {
        message: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse DAR manifest")]
    ManifestError(#[from] DarManifestError),

    #[error(transparent)]
    DalfError(#[from] DalfError),
}

impl DarError {
    pub fn io_error(error: io::Error, message: impl Into<String>) -> Self {
        Self::IOError {
            message: message.into(),
            source: error,
        }
    }

    pub fn get_dalf_error(error: ZipError, message: impl Into<String>) -> Self {
        Self::GetDalfError {
            message: message.into(),
            source: error,
        }
    }
}
