use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
use canton_types::topology::Fingerprint;
use multihash_codetable::{Code, MultihashDigest as _};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::v30::{
    ConvertError, CryptoKeyFormat, EncryptionKeySpec, HashPurpose as _,
    hash_purpose::PublicKeyFingerprint,
};

/// Encoded encryption public key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EncodedEncryptionPublicKey {
    format: CryptoKeyFormat,
    public_key: Vec<u8>,
    key_spec: EncryptionKeySpec,
}

impl EncodedEncryptionPublicKey {
    /// Create new encryption public key from raw bytes
    ///
    /// This function does not check, that `public_key` is actually a `key_spec` key encoded with
    /// `format`. User is responsible for doing that.
    pub fn new(
        format: CryptoKeyFormat,
        public_key: Vec<u8>,
        key_spec: EncryptionKeySpec,
    ) -> Result<Self, EncryptionPublicKeyError> {
        if format == CryptoKeyFormat::DerPkcs8PrivateKeyInfo {
            return Err(EncryptionPublicKeyError {
                inner: ErrorInner::BadKeyFormat { format },
            });
        }
        if public_key.is_empty() {
            return Err(EncryptionPublicKeyError {
                inner: ErrorInner::EmptyKey,
            });
        }

        Ok(Self {
            format,
            public_key,
            key_spec,
        })
    }

    /// Encoding format of the key
    pub fn format(&self) -> CryptoKeyFormat {
        self.format
    }

    /// Public key serialized according to `format`
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// The key specification that was used to generate the key pair
    pub fn key_spec(&self) -> EncryptionKeySpec {
        self.key_spec
    }

    /// Get the fingerprint of the key
    pub fn fingerprint(&self) -> Fingerprint {
        let mut input = PublicKeyFingerprint::ID.to_be_bytes().to_vec();
        input.extend_from_slice(&self.public_key);

        let multihash = Code::Sha2_256.digest(&input);
        let hex_str = hex::encode(multihash.to_bytes());

        debug_assert_eq!(hex_str.len(), 68);
        // 68-digit hex string is always a valid fingerprint, so it's safe to use new_unchecked here
        Fingerprint::new_unchecked(hex_str)
    }
}

impl From<EncodedEncryptionPublicKey> for proto::EncryptionPublicKey {
    fn from(value: EncodedEncryptionPublicKey) -> Self {
        Self {
            format: proto::CryptoKeyFormat::from(value.format).into(),
            public_key: value.public_key,
            #[allow(deprecated)]
            scheme: proto::EncryptionKeyScheme::Unspecified.into(),
            key_spec: proto::EncryptionKeySpec::from(value.key_spec).into(),
        }
    }
}

impl TryFrom<proto::EncryptionPublicKey> for EncodedEncryptionPublicKey {
    type Error = ConvertError;

    fn try_from(value: proto::EncryptionPublicKey) -> Result<Self, Self::Error> {
        let format = proto::CryptoKeyFormat::try_from(value.format)
            .validated_of::<proto::EncryptionPublicKey>("format")?
            .try_into()
            .validated_of::<proto::EncryptionPublicKey>("format")?;

        let key_spec = proto::EncryptionKeySpec::try_from(value.key_spec)
            .validated_of::<proto::EncryptionPublicKey>("key_spec")?
            .try_into()
            // If key specification is UNSPECIFIED, fallback to checking the scheme
            .or_else(|error| {
                #[allow(deprecated, reason = "support old keys")]
                let scheme = proto::EncryptionKeyScheme::try_from(value.scheme)
                    .validated_of::<proto::EncryptionPublicKey>("scheme")?;
                match scheme {
                    proto::EncryptionKeyScheme::Unspecified => {
                        // Reporting the original error here, so that the error message
                        // won't refer to a deprecated field. Basically this error means,
                        // that key_spec is unspecified and there is nothing to fallback to.
                        Err(error).validated_of::<proto::EncryptionPublicKey>("key_spec")
                    }
                    proto::EncryptionKeyScheme::EciesP256HkdfHmacSha256Aes128gcm => {
                        Ok(EncryptionKeySpec::EcP256)
                    }
                    proto::EncryptionKeyScheme::EciesP256HmacSha256aEs128cbc => {
                        Ok(EncryptionKeySpec::EcP256)
                    }
                    proto::EncryptionKeyScheme::Rsa2048OaepSha256 => Ok(EncryptionKeySpec::Rsa2048),
                }
            })?;

        Self::new(format, value.public_key, key_spec)
            .map_err(|error| {
                let field_name = match error.inner {
                    ErrorInner::EmptyKey => "public_key",
                    ErrorInner::BadKeyFormat { .. } => "format",
                };
                InvalidProtoFieldValue::new_of::<proto::EncryptionPublicKey>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Error constructing, decoding, or encoding an encryption public key
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct EncryptionPublicKeyError {
    inner: ErrorInner,
}

#[derive(Debug, thiserror::Error)]
enum ErrorInner {
    #[error("key is empty")]
    EmptyKey,

    #[error("bad crypto key format for a public encryption key: '{format}'")]
    BadKeyFormat { format: CryptoKeyFormat },
}
