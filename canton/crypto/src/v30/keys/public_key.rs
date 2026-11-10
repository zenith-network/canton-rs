use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
use canton_types::topology::Fingerprint;
use protobuf_utils::RequiredProtoField as _;

use crate::v30::{
    ConvertError, EncodedEncryptionPublicKey, EncodedSigningPublicKey, SigningPublicKeyError,
};

/// Encoded public key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EncodedPublicKey {
    /// Signing public key
    Signing(EncodedSigningPublicKey),

    /// Encryption public key
    Encryption(EncodedEncryptionPublicKey),
}

impl EncodedPublicKey {
    /// Get the fingerprint of the key
    pub fn fingerprint(&self) -> Result<Fingerprint, SigningPublicKeyError> {
        match self {
            EncodedPublicKey::Signing(key) => key.fingerprint(),
            EncodedPublicKey::Encryption(key) => Ok(key.fingerprint()),
        }
    }
}

impl From<EncodedSigningPublicKey> for EncodedPublicKey {
    fn from(value: EncodedSigningPublicKey) -> Self {
        Self::Signing(value)
    }
}

impl From<EncodedEncryptionPublicKey> for EncodedPublicKey {
    fn from(value: EncodedEncryptionPublicKey) -> Self {
        Self::Encryption(value)
    }
}

impl From<EncodedPublicKey> for proto::PublicKey {
    fn from(value: EncodedPublicKey) -> Self {
        use proto::public_key::Key;

        match value {
            EncodedPublicKey::Signing(key) => Self {
                key: Some(Key::SigningPublicKey(key.into())),
            },
            EncodedPublicKey::Encryption(key) => Self {
                key: Some(Key::EncryptionPublicKey(key.into())),
            },
        }
    }
}

impl TryFrom<proto::PublicKey> for EncodedPublicKey {
    type Error = ConvertError;

    fn try_from(value: proto::PublicKey) -> Result<Self, Self::Error> {
        use proto::public_key::Key;

        match value.key.required_of::<proto::PublicKey>("key")? {
            Key::SigningPublicKey(key) => key.try_into().map(Self::Signing),
            Key::EncryptionPublicKey(key) => key.try_into().map(Self::Encryption),
        }
    }
}
