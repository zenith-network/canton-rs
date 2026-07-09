use std::{
    io::{self, Read},
    num::ParseIntError,
};

use canton_types::errors::PackageIdError;
use daml_lf_archive_proto::{
    com::digitalasset::daml::lf::archive::{Archive, ArchivePayload, archive_payload},
    prost::{DecodeError, Message as _},
};
use daml_lf_version::{MinorVersion, Version};

use crate::hash::{PayloadHash, PayloadHashError};

#[derive(Debug, thiserror::Error)]
pub enum DalfError {
    #[error("{message}")]
    IOError {
        message: String,
        #[source]
        source: io::Error,
    },

    #[error("{message}")]
    DecodeError {
        message: String,
        source: DecodeError,
    },

    #[error("archive payload not found")]
    PayloadNotFound,

    #[error("invalid Daml LF version: '{major}.{input}'")]
    InvalidVersion {
        major: u32,
        input: String,
        #[source]
        source: ParseIntError,
    },

    #[error("unsupported Daml LF version: '{0}'")]
    UnsupportedVersion(Version),

    #[error(transparent)]
    HashError(#[from] PayloadHashError),

    #[error(transparent)]
    PackageIdError(#[from] PackageIdError),
}

impl DalfError {
    pub fn io_error(error: io::Error, message: impl Into<String>) -> Self {
        Self::IOError {
            message: message.into(),
            source: error,
        }
    }

    pub fn decode_error(error: DecodeError, message: impl Into<String>) -> Self {
        Self::DecodeError {
            message: message.into(),
            source: error,
        }
    }

    pub fn invalid_version(error: ParseIntError, major: u32, input: String) -> Self {
        Self::InvalidVersion {
            major,
            input,
            source: error,
        }
    }
}

/// Versions of the DALF payload
pub enum Payload {
    DamlLf1(Vec<u8>),
    DamlLf2(Vec<u8>),
}

/// Adapter for a `.dalf` file
pub struct DalfFile {
    hash: PayloadHash,
    version: Version,
    payload: Payload,
}

impl DalfFile {
    /// Hash of the payload (interpreted as package ID)
    pub fn hash(&self) -> &PayloadHash {
        &self.hash
    }

    /// Daml LF version
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Reference to payload
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// Read DALF file from reader
    pub fn read_from<R: Read>(mut reader: R) -> Result<Self, DalfError> {
        let mut buf = Vec::new();

        reader
            .read_to_end(&mut buf)
            .map_err(|err| DalfError::io_error(err, "failed to read DALF file"))?;

        let archive = Archive::decode(buf.as_slice()).map_err(|err| {
            DalfError::decode_error(err, "failed to decode Daml archive from protobuf")
        })?;

        let hash = PayloadHash::try_from_proto(archive.hash_function, archive.hash)?;
        hash.validate(&archive.payload)?;

        let archive_payload =
            ArchivePayload::decode(archive.payload.as_slice()).map_err(|err| {
                DalfError::decode_error(err, "failed to decode Daml archive payload from protobuf")
            })?;

        let sum = archive_payload.sum.ok_or(DalfError::PayloadNotFound)?;

        let version = Self::parse_version(&sum, &archive_payload.minor)?;

        let payload = match sum {
            archive_payload::Sum::DamlLf1(bytes) => Payload::DamlLf1(bytes),
            archive_payload::Sum::DamlLf2(bytes) => Payload::DamlLf2(bytes),
        };

        Ok(Self {
            hash,
            version,
            payload,
        })
    }

    fn parse_version(sum: &archive_payload::Sum, minor: &str) -> Result<Version, DalfError> {
        let major = match sum {
            archive_payload::Sum::DamlLf1(_) => 1,
            archive_payload::Sum::DamlLf2(_) => 2,
        };

        let minor = match minor {
            "dev" => MinorVersion::Dev,
            v if let Some((v, rc)) = v.split_once("-rc") => MinorVersion::Staging {
                version: v
                    .parse()
                    .map_err(|err| DalfError::invalid_version(err, major, minor.to_owned()))?,
                revision: rc
                    .parse()
                    .map_err(|err| DalfError::invalid_version(err, major, minor.to_owned()))?,
            },
            v => MinorVersion::Stable {
                version: v
                    .parse()
                    .map_err(|err| DalfError::invalid_version(err, major, minor.to_owned()))?,
            },
        };

        Ok(Version { major, minor })
    }

    /// Parse payload of the DALF file
    #[allow(clippy::non_minimal_cfg)]
    #[cfg(any(feature = "v2"))]
    pub fn to_package(&self) -> Result<crate::package::Package, DalfError> {
        // This checks the support on protobuf side
        if !daml_lf_archive_proto::is_supported(&self.version) {
            return Err(DalfError::UnsupportedVersion(self.version));
        }

        match &self.payload {
            #[cfg(feature = "v2")]
            Payload::DamlLf2(bytes) => {
                use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2::Package;
                let package_id = self.hash().to_package_id();
                let proto = Package::decode(bytes.as_slice())
                    .map_err(|err| DalfError::decode_error(err, "failed to decode the package"))?;
                Ok(crate::package::Package::new(
                    self.version,
                    package_id,
                    proto.into(),
                ))
            }
            _ => Err(DalfError::UnsupportedVersion(self.version)),
        }
    }
}
