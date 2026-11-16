use std::{collections::BTreeSet, fmt};

use protobuf_utils::UnspecifiedEnumValue;

mod keys;
mod signature;

pub mod error;
pub mod hash_purpose;

// public exports
pub use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
pub use error::ConvertError;
pub use hash_purpose::HashPurpose;
pub use keys::*;
pub use signature::{
    Signature, SignatureDelegation, SignatureDelegationBuilder, SignatureDelegationError,
    SignatureFormat, SigningAlgorithmSpec,
};

/// Encoding format of a key
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CryptoKeyFormat {
    /// ASN.1 + DER encoding
    /// Legacy format no longer used, except for migrations
    Der,

    /// Raw encoding of a key, used for symmetric keys
    Raw,

    /// ASN.1 + DER-encoding of X.509 SubjectPublicKeyInfo structure:
    /// <https://datatracker.ietf.org/doc/html/rfc5280#section-4.1>
    DerX509SubjectPublicKeyInfo,

    /// ASN.1 + DER-encoding of PKCS #8 PrivateKeyInfo structure:
    /// <https://datatracker.ietf.org/doc/html/rfc5208#section-5>
    DerPkcs8PrivateKeyInfo,

    /// Symbolic crypto, must only be used for testing
    Symbolic,
}

impl fmt::Display for CryptoKeyFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CryptoKeyFormat::Der => "ASN.1 + DER",
            CryptoKeyFormat::Raw => "Raw",
            CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                "ASN.1 + DER (X.509 SubjectPublicKeyInfo)"
            }
            CryptoKeyFormat::DerPkcs8PrivateKeyInfo => "ASN.1 + DER (PKCS #8 PrivateKeyInfo)",
            CryptoKeyFormat::Symbolic => "Symbolic",
        })
    }
}

impl From<CryptoKeyFormat> for proto::CryptoKeyFormat {
    fn from(value: CryptoKeyFormat) -> Self {
        match value {
            CryptoKeyFormat::Der => proto::CryptoKeyFormat::Der,
            CryptoKeyFormat::Raw => proto::CryptoKeyFormat::Raw,
            CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                proto::CryptoKeyFormat::DerX509SubjectPublicKeyInfo
            }
            CryptoKeyFormat::DerPkcs8PrivateKeyInfo => {
                proto::CryptoKeyFormat::DerPkcs8PrivateKeyInfo
            }
            CryptoKeyFormat::Symbolic => proto::CryptoKeyFormat::Symbolic,
        }
    }
}

impl TryFrom<proto::CryptoKeyFormat> for CryptoKeyFormat {
    type Error = UnspecifiedEnumValue;

    /// Returns error if value is `UNSPECIFIED`.
    fn try_from(value: proto::CryptoKeyFormat) -> Result<Self, Self::Error> {
        match value {
            proto::CryptoKeyFormat::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::CryptoKeyFormat::Der => Ok(Self::Der),
            proto::CryptoKeyFormat::Raw => Ok(Self::Raw),
            proto::CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                Ok(Self::DerX509SubjectPublicKeyInfo)
            }
            proto::CryptoKeyFormat::DerPkcs8PrivateKeyInfo => Ok(Self::DerPkcs8PrivateKeyInfo),
            proto::CryptoKeyFormat::Symbolic => Ok(Self::Symbolic),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SigningKeySpec {
    /// Elliptic Curve Key from Curve25519
    /// as defined in <http://ed25519.cr.yp.to/>
    EcCurve25519,

    /// Elliptic Curve Key from the NIST P-256 curve (aka secp256r1)
    /// as defined in <https://doi.org/10.6028/NIST.FIPS.186-4>
    EcP256,

    /// Elliptic Curve Key from the NIST P-384 curve (aka secp384r1)
    /// as defined in <https://doi.org/10.6028/NIST.FIPS.186-4>
    EcP384,

    /// Elliptic Curve Key from SECG P256k1 curve (aka secp256k1)
    /// commonly used in bitcoin and ethereum
    /// as defined in <https://www.secg.org/sec2-v2.pdf>
    EcSecp256k1,

    /// PQC Signing key spec for ML-DSA-65 as defined in <https://doi.org/10.6028/NIST.FIPS.204>
    MlDsa65,
}

impl From<SigningKeySpec> for proto::SigningKeySpec {
    fn from(value: SigningKeySpec) -> Self {
        match value {
            SigningKeySpec::EcCurve25519 => Self::EcCurve25519,
            SigningKeySpec::EcP256 => Self::EcP256,
            SigningKeySpec::EcP384 => Self::EcP384,
            SigningKeySpec::EcSecp256k1 => Self::EcSecp256k1,
            SigningKeySpec::MlDsa65 => Self::MlDsa65,
        }
    }
}

impl TryFrom<proto::SigningKeySpec> for SigningKeySpec {
    type Error = UnspecifiedEnumValue;

    /// Returns error if value is `UNSPECIFIED`.
    fn try_from(value: proto::SigningKeySpec) -> Result<Self, Self::Error> {
        match value {
            proto::SigningKeySpec::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::SigningKeySpec::EcCurve25519 => Ok(Self::EcCurve25519),
            proto::SigningKeySpec::EcP256 => Ok(Self::EcP256),
            proto::SigningKeySpec::EcP384 => Ok(Self::EcP384),
            proto::SigningKeySpec::EcSecp256k1 => Ok(Self::EcSecp256k1),
            proto::SigningKeySpec::MlDsa65 => Ok(Self::MlDsa65),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum EncryptionKeySpec {
    /// Elliptic Curve Key from the NIST P-256 curve (aka Secp256r1)
    /// as defined in <https://doi.org/10.6028/NIST.FIPS.186-4>
    EcP256,

    /// RSA with 2048 bits
    Rsa2048,
}

impl From<EncryptionKeySpec> for proto::EncryptionKeySpec {
    fn from(value: EncryptionKeySpec) -> Self {
        match value {
            EncryptionKeySpec::EcP256 => Self::EcP256,
            EncryptionKeySpec::Rsa2048 => Self::Rsa2048,
        }
    }
}

impl TryFrom<proto::EncryptionKeySpec> for EncryptionKeySpec {
    type Error = UnspecifiedEnumValue;

    /// Returns error if value is `UNSPECIFIED`.
    fn try_from(value: proto::EncryptionKeySpec) -> Result<Self, Self::Error> {
        match value {
            proto::EncryptionKeySpec::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::EncryptionKeySpec::EcP256 => Ok(Self::EcP256),
            proto::EncryptionKeySpec::Rsa2048 => Ok(Self::Rsa2048),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SigningKeyUsage {
    /// The root namespace key that defines a node's identity and signs topology requests
    Namespace,

    #[deprecated(note = "no longer used (only kept for backwards compatibility)")]
    IdentityDelegation,

    /// Keys that authenticate members of the network towards a sequencer
    SequencerAuthentication,

    /// Keys that deal with all the signing that happens as part of the protocol
    Protocol,

    /// Used internally to identify keys that can self-sign to prove ownership
    ProofOfOwnership,
}

impl From<SigningKeyUsage> for proto::SigningKeyUsage {
    fn from(value: SigningKeyUsage) -> Self {
        match value {
            SigningKeyUsage::Namespace => Self::Namespace,
            #[allow(deprecated)]
            SigningKeyUsage::IdentityDelegation => Self::IdentityDelegation,
            SigningKeyUsage::SequencerAuthentication => Self::SequencerAuthentication,
            SigningKeyUsage::Protocol => Self::Protocol,
            SigningKeyUsage::ProofOfOwnership => Self::ProofOfOwnership,
        }
    }
}

impl TryFrom<proto::SigningKeyUsage> for SigningKeyUsage {
    type Error = UnspecifiedEnumValue;

    /// Returns error if value is `UNSPECIFIED`.
    fn try_from(value: proto::SigningKeyUsage) -> Result<Self, Self::Error> {
        match value {
            proto::SigningKeyUsage::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::SigningKeyUsage::Namespace => Ok(Self::Namespace),
            #[allow(deprecated)]
            proto::SigningKeyUsage::IdentityDelegation => Ok(Self::IdentityDelegation),
            proto::SigningKeyUsage::SequencerAuthentication => Ok(Self::SequencerAuthentication),
            proto::SigningKeyUsage::Protocol => Ok(Self::Protocol),
            proto::SigningKeyUsage::ProofOfOwnership => Ok(Self::ProofOfOwnership),
        }
    }
}

/// Specified usages of a signing key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SigningKeyUsages {
    usages: BTreeSet<SigningKeyUsage>,
}

impl SigningKeyUsages {
    fn validate(usages: &BTreeSet<SigningKeyUsage>) -> Result<(), SigningKeyUsageError> {
        // These two cases are explicitly rejected by Canton, so we reject them on SDK side too
        if usages == &BTreeSet::from([SigningKeyUsage::ProofOfOwnership]) {
            return Err(SigningKeyUsageError::OnlyProofOfOwnership);
        }

        if usages
            == &BTreeSet::from([
                SigningKeyUsage::Namespace,
                SigningKeyUsage::ProofOfOwnership,
            ])
        {
            return Err(SigningKeyUsageError::NamespaceWithProof);
        }

        Ok(())
    }

    /// Only [`Namespace`][SigningKeyUsage::Namespace] usage
    pub fn namespace() -> Self {
        Self {
            usages: [SigningKeyUsage::Namespace].into(),
        }
    }

    /// Only [`Protocol`][SigningKeyUsage::Protocol] usage
    pub fn protocol() -> Self {
        Self {
            usages: [SigningKeyUsage::Protocol].into(),
        }
    }

    /// Only [`SequencerAuthentication`][SigningKeyUsage::SequencerAuthentication] usage
    pub fn sequencer_authentication() -> Self {
        Self {
            usages: [SigningKeyUsage::SequencerAuthentication].into(),
        }
    }

    /// Iterator over usages
    pub fn iter(&self) -> impl Iterator<Item = &SigningKeyUsage> {
        self.usages.iter()
    }

    /// Try to create usages from an iterable collection
    ///
    /// Note that duplicating entries will be silently ignored.
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = SigningKeyUsage>,
    ) -> Result<Self, SigningKeyUsageError> {
        BTreeSet::from_iter(iter).try_into()
    }
}

impl TryFrom<SigningKeyUsage> for SigningKeyUsages {
    type Error = SigningKeyUsageError;

    fn try_from(value: SigningKeyUsage) -> Result<Self, Self::Error> {
        BTreeSet::from([value]).try_into()
    }
}

impl TryFrom<BTreeSet<SigningKeyUsage>> for SigningKeyUsages {
    type Error = SigningKeyUsageError;

    fn try_from(value: BTreeSet<SigningKeyUsage>) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self { usages: value })
    }
}

impl TryFrom<Vec<SigningKeyUsage>> for SigningKeyUsages {
    type Error = SigningKeyUsageError;

    /// This function silently absorbs duplications on purpose
    fn try_from(value: Vec<SigningKeyUsage>) -> Result<Self, Self::Error> {
        value.into_iter().collect::<BTreeSet<_>>().try_into()
    }
}

impl From<SigningKeyUsages> for Vec<SigningKeyUsage> {
    fn from(value: SigningKeyUsages) -> Self {
        value.usages.into_iter().collect()
    }
}

impl IntoIterator for SigningKeyUsages {
    type Item = <BTreeSet<SigningKeyUsage> as IntoIterator>::Item;

    type IntoIter = <BTreeSet<SigningKeyUsage> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.usages.into_iter()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SigningKeyUsageError {
    #[error("proof-of-ownership usage must be accompanied by non-internal usage")]
    OnlyProofOfOwnership,
    #[error("namespace key cannot be used for proof of ownership")]
    NamespaceWithProof,
}
