use std::fmt;

use canton_types::PackageId;
use daml_lf_archive_proto::{
    com::digitalasset::daml::lf::archive::HashFunction, prost::UnknownEnumValue,
};
use hex::FromHexError;
use sha2::{Digest as _, Sha256};

pub enum PayloadHash {
    Sha256([u8; 32]),
}

impl PayloadHash {
    pub fn try_from_proto(
        hash_function: i32,
        hash: impl AsRef<str>,
    ) -> Result<Self, PayloadHashError> {
        let hash_function =
            HashFunction::try_from(hash_function).map_err(PayloadHashError::UnknownHashFunction)?;

        match hash_function {
            HashFunction::Sha256 => {
                let bytes = hex::decode(hash.as_ref())?;
                let hash = <[u8; 32]>::try_from(bytes).map_err(|orig| {
                    PayloadHashError::InvalidHashLengh {
                        expected: 32,
                        got: orig.len(),
                    }
                })?;
                Ok(Self::Sha256(hash))
            }
        }
    }

    /// Validate that this hash matches the data
    pub fn validate(&self, data: impl AsRef<[u8]>) -> Result<(), PayloadHashError> {
        match self {
            PayloadHash::Sha256(value) => {
                let calculated_hash: [u8; 32] = Sha256::digest(data).into();
                if value != &calculated_hash {
                    Err(PayloadHashError::HashMismatch)
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn to_package_id(&self) -> PackageId {
        match self {
            PayloadHash::Sha256(v) => PackageId::new_unchecked_owned(hex::encode(v)),
        }
    }
}

impl fmt::Display for PayloadHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl fmt::LowerHex for PayloadHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadHash::Sha256(value) => write!(f, "{}", hex::encode(value)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PayloadHashError {
    #[error("unknown hash function")]
    UnknownHashFunction(#[source] UnknownEnumValue),

    #[error("hash mismatch")]
    HashMismatch,

    #[error("failed to decode hash from hex")]
    FromHexError(#[from] FromHexError),

    #[error("invalid hash lenght (expected {expected} bytes, got {got} bytes)")]
    InvalidHashLengh { expected: usize, got: usize },
}
