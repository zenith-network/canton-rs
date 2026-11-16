use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
use canton_types::topology::Fingerprint;
use multihash_codetable::{Code, MultihashDigest as _};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::v30::{
    ConvertError, CryptoKeyFormat, DecodedKeyOwned, DecodedKeyRef, HashPurpose as _,
    SigningKeySpec, SigningKeyUsage, SigningKeyUsages, hash_purpose::PublicKeyFingerprint,
};

/// Encoded signing public key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EncodedSigningPublicKey {
    format: CryptoKeyFormat,
    public_key: Vec<u8>,
    key_spec: SigningKeySpec,
    usage: SigningKeyUsages,
}

impl EncodedSigningPublicKey {
    /// Create new signing public key from raw bytes
    ///
    /// This function does not check, that `public_key` is actually a `key_spec` key encoded with
    /// `format`. User is responsible for doing that.
    pub fn new(
        format: CryptoKeyFormat,
        public_key: Vec<u8>,
        key_spec: SigningKeySpec,
        usage: SigningKeyUsages,
    ) -> Result<Self, SigningPublicKeyError> {
        if format == CryptoKeyFormat::DerPkcs8PrivateKeyInfo {
            return Err(SigningPublicKeyError {
                inner: ErrorInner::BadKeyFormat { format },
            });
        }
        if public_key.is_empty() {
            return Err(SigningPublicKeyError {
                inner: ErrorInner::EmptyKey,
            });
        }

        Ok(Self {
            format,
            public_key,
            key_spec,
            usage,
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
    pub fn key_spec(&self) -> SigningKeySpec {
        self.key_spec
    }

    /// Explicitly state the key's intended use
    pub fn usage(&self) -> &SigningKeyUsages {
        &self.usage
    }

    /// Get the fingerprint of the key
    pub fn fingerprint(&self) -> Result<Fingerprint, SigningPublicKeyError> {
        let decoded = self.as_decoded()?;

        let material = match decoded.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyRef::Spki(key) => match decoded.key_spec {
                SigningKeySpec::EcCurve25519 => key
                    .subject_public_key
                    .as_bytes()
                    .ok_or(spki::Error::KeyMalformed)?,
                SigningKeySpec::EcP256
                | SigningKeySpec::EcP384
                | SigningKeySpec::EcSecp256k1
                | SigningKeySpec::MlDsa65 => self.public_key.as_slice(),
            },
            DecodedKeyRef::Raw(key) | DecodedKeyRef::Symbolic(key) => key,
        };

        let mut input = PublicKeyFingerprint::ID.to_be_bytes().to_vec();
        input.extend_from_slice(material);

        let multihash = Code::Sha2_256.digest(&input);
        let hex_str = hex::encode(multihash.to_bytes());

        debug_assert_eq!(hex_str.len(), 68);
        // 68-digit hex string is always a valid fingerprint, so it's safe to use new_unchecked here
        Ok(Fingerprint::new_unchecked(hex_str))
    }

    /// Decode signing key according to the format into referenced type
    pub fn as_decoded(&self) -> Result<DecodedSigningPublicKeyRef<'_>, SigningPublicKeyError> {
        let public_key = match self.format {
            CryptoKeyFormat::Raw => DecodedKeyRef::Raw(&self.public_key),
            CryptoKeyFormat::Symbolic => DecodedKeyRef::Symbolic(&self.public_key),

            #[cfg(feature = "spki")]
            CryptoKeyFormat::Der | CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                DecodedKeyRef::Spki(spki::SubjectPublicKeyInfoRef::try_from(
                    self.public_key.as_slice(),
                )?)
            }
            #[cfg(not(feature = "spki"))]
            CryptoKeyFormat::Der | CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                return Err(SigningPublicKeyError {
                    inner: ErrorInner::UnsupportedFormat {
                        format: self.format,
                    },
                });
            }

            CryptoKeyFormat::DerPkcs8PrivateKeyInfo => {
                unreachable!(
                    "'{}' must never be a format of a public signing key",
                    CryptoKeyFormat::DerPkcs8PrivateKeyInfo
                )
            }
        };

        Ok(DecodedSigningPublicKeyRef {
            public_key,
            key_spec: self.key_spec,
            usage: &self.usage,
        })
    }

    /// Decode signing key according to the format into owned type
    pub fn to_decoded(&self) -> Result<DecodedSigningPublicKeyOwned, SigningPublicKeyError> {
        self.as_decoded().map(|key| key.to_owned_key())
    }

    /// Convert the key into decoded form
    pub fn into_decoded(self) -> Result<DecodedSigningPublicKeyOwned, SigningPublicKeyError> {
        let public_key = match self.format {
            CryptoKeyFormat::Raw => DecodedKeyOwned::Raw(self.public_key),
            CryptoKeyFormat::Symbolic => DecodedKeyOwned::Symbolic(self.public_key),

            #[cfg(feature = "spki")]
            CryptoKeyFormat::Der | CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                DecodedKeyOwned::Spki(spki::SubjectPublicKeyInfoOwned::try_from(
                    self.public_key.as_slice(),
                )?)
            }
            #[cfg(not(feature = "spki"))]
            CryptoKeyFormat::Der | CryptoKeyFormat::DerX509SubjectPublicKeyInfo => {
                return Err(SigningPublicKeyError {
                    inner: ErrorInner::UnsupportedFormat {
                        format: self.format,
                    },
                });
            }

            CryptoKeyFormat::DerPkcs8PrivateKeyInfo => {
                unreachable!(
                    "'{}' must never be a format of a public signing key",
                    CryptoKeyFormat::DerPkcs8PrivateKeyInfo
                )
            }
        };

        Ok(DecodedSigningPublicKeyOwned {
            public_key,
            key_spec: self.key_spec,
            usage: self.usage,
        })
    }
}

impl From<EncodedSigningPublicKey> for proto::SigningPublicKey {
    fn from(value: EncodedSigningPublicKey) -> Self {
        Self {
            format: proto::CryptoKeyFormat::from(value.format).into(),
            public_key: value.public_key,
            #[allow(deprecated)]
            scheme: proto::SigningKeyScheme::Unspecified.into(),
            usage: value
                .usage
                .into_iter()
                .map(proto::SigningKeyUsage::from)
                .map(Into::into)
                .collect(),
            key_spec: proto::SigningKeySpec::from(value.key_spec).into(),
        }
    }
}

impl TryFrom<proto::SigningPublicKey> for EncodedSigningPublicKey {
    type Error = ConvertError;

    fn try_from(value: proto::SigningPublicKey) -> Result<Self, Self::Error> {
        let format = proto::CryptoKeyFormat::try_from(value.format)
            .validated_of::<proto::SigningPublicKey>("format")?
            .try_into()
            .validated_of::<proto::SigningPublicKey>("format")?;

        let key_spec = proto::SigningKeySpec::try_from(value.key_spec)
            .validated_of::<proto::SigningPublicKey>("key_spec")?
            .try_into()
            // If key specification is UNSPECIFIED, fallback to checking the scheme
            .or_else(|error| {
                #[allow(deprecated, reason = "support old keys")]
                let scheme = proto::SigningKeyScheme::try_from(value.scheme)
                    .validated_of::<proto::SigningPublicKey>("scheme")?;
                match scheme {
                    proto::SigningKeyScheme::Unspecified => {
                        // Reporting the original error here, so that the error message
                        // won't refer to a deprecated field. Basically this error means,
                        // that key_spec is unspecified and there is nothing to fallback to.
                        Err(error).validated_of::<proto::SigningPublicKey>("key_spec")
                    }
                    proto::SigningKeyScheme::Ed25519 => Ok(SigningKeySpec::EcCurve25519),
                    proto::SigningKeyScheme::EcDsaP256 => Ok(SigningKeySpec::EcP256),
                    proto::SigningKeyScheme::EcDsaP384 => Ok(SigningKeySpec::EcP384),
                }
            })?;

        let usage = value
            .usage
            .into_iter()
            .map(|usage_i32| {
                let usage_proto = proto::SigningKeyUsage::try_from(usage_i32)
                    .validated_of::<proto::SigningPublicKey>("usage")?;
                let usage = SigningKeyUsage::try_from(usage_proto)
                    .validated_of::<proto::SigningPublicKey>("usage")?;
                Ok::<_, InvalidProtoFieldValue>(usage)
            })
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .validated_of::<proto::SigningPublicKey>("usage")?;

        Self::new(format, value.public_key, key_spec, usage)
            .map_err(|error| {
                let field_name = match error.inner {
                    ErrorInner::EmptyKey => "public_key",
                    ErrorInner::BadKeyFormat { .. } => "format",
                    // Only the variants above are returned from Self::new
                    _ => unreachable!(),
                };
                InvalidProtoFieldValue::new_of::<proto::SigningPublicKey>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Referenced decoded signing public key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedSigningPublicKeyRef<'a> {
    public_key: DecodedKeyRef<'a>,
    key_spec: SigningKeySpec,
    usage: &'a SigningKeyUsages,
}

impl<'a> DecodedSigningPublicKeyRef<'a> {
    /// Create a new referenced decoded signing public key
    pub fn new(
        public_key: DecodedKeyRef<'a>,
        key_spec: SigningKeySpec,
        usage: &'a SigningKeyUsages,
    ) -> Result<Self, SigningPublicKeyError> {
        match &public_key {
            DecodedKeyRef::Raw(key) | DecodedKeyRef::Symbolic(key) if key.is_empty() => {
                return Err(SigningPublicKeyError {
                    inner: ErrorInner::EmptyKey,
                });
            }
            _ => {}
        }
        Ok(Self {
            public_key,
            key_spec,
            usage,
        })
    }

    /// Public key
    pub fn public_key(&self) -> &DecodedKeyRef<'a> {
        &self.public_key
    }

    /// The key specification that was used to generate the key pair
    pub fn key_spec(&self) -> SigningKeySpec {
        self.key_spec
    }

    /// Explicitly state the key's intended use
    pub fn usage(&self) -> &'a SigningKeyUsages {
        self.usage
    }

    /// Create an owned version of the key
    pub fn to_owned_key(&self) -> DecodedSigningPublicKeyOwned {
        let public_key = match &self.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyRef::Spki(key) => {
                use spki::der::referenced::RefToOwned as _;

                DecodedKeyOwned::Spki(key.ref_to_owned())
            }
            DecodedKeyRef::Raw(key) => DecodedKeyOwned::Raw(key.to_vec()),
            DecodedKeyRef::Symbolic(key) => DecodedKeyOwned::Symbolic(key.to_vec()),
        };

        DecodedSigningPublicKeyOwned {
            public_key,
            key_spec: self.key_spec,
            usage: self.usage.to_owned(),
        }
    }

    /// Create an encoded version of the key
    pub fn to_encoded(&self) -> Result<EncodedSigningPublicKey, SigningPublicKeyError> {
        let (format, public_key) = match &self.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyRef::Spki(key) => {
                use spki::der::Encode as _;

                let encoded = key.to_der().map_err(spki::Error::from)?;
                (CryptoKeyFormat::DerX509SubjectPublicKeyInfo, encoded)
            }
            DecodedKeyRef::Raw(key) => (CryptoKeyFormat::Raw, key.to_vec()),
            DecodedKeyRef::Symbolic(key) => (CryptoKeyFormat::Symbolic, key.to_vec()),
        };

        Ok(EncodedSigningPublicKey {
            format,
            public_key,
            key_spec: self.key_spec,
            usage: self.usage.clone(),
        })
    }

    /// Get the fingerprint of the key
    pub fn fingerprint(&self) -> Result<Fingerprint, SigningPublicKeyError> {
        self.to_encoded()?.fingerprint()
    }
}

/// Owned decoded signing public key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedSigningPublicKeyOwned {
    public_key: DecodedKeyOwned,
    key_spec: SigningKeySpec,
    usage: SigningKeyUsages,
}

impl DecodedSigningPublicKeyOwned {
    // Create a new owned decoded signing public key
    pub fn new(
        public_key: DecodedKeyOwned,
        key_spec: SigningKeySpec,
        usage: SigningKeyUsages,
    ) -> Result<Self, SigningPublicKeyError> {
        match &public_key {
            DecodedKeyOwned::Raw(key) | DecodedKeyOwned::Symbolic(key) if key.is_empty() => {
                return Err(SigningPublicKeyError {
                    inner: ErrorInner::EmptyKey,
                });
            }
            _ => {}
        }

        Ok(Self {
            public_key,
            key_spec,
            usage,
        })
    }

    /// Public key
    pub fn public_key(&self) -> &DecodedKeyOwned {
        &self.public_key
    }

    /// The key specification that was used to generate the key pair
    pub fn key_spec(&self) -> SigningKeySpec {
        self.key_spec
    }

    /// Explicitly state the key's intended use
    pub fn usage(&self) -> &SigningKeyUsages {
        &self.usage
    }

    /// Get referenced version of the key
    pub fn as_referenced_key(&self) -> DecodedSigningPublicKeyRef<'_> {
        let public_key = match &self.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyOwned::Spki(key) => {
                use spki::der::referenced::OwnedToRef as _;

                DecodedKeyRef::Spki(key.owned_to_ref())
            }
            DecodedKeyOwned::Raw(key) => DecodedKeyRef::Raw(key),
            DecodedKeyOwned::Symbolic(key) => DecodedKeyRef::Symbolic(key),
        };

        DecodedSigningPublicKeyRef {
            public_key,
            key_spec: self.key_spec,
            usage: &self.usage,
        }
    }

    /// Create an encoded version of the key
    pub fn to_encoded(&self) -> Result<EncodedSigningPublicKey, SigningPublicKeyError> {
        let (format, public_key) = match &self.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyOwned::Spki(key) => {
                use spki::der::Encode as _;

                let encoded = key.to_der().map_err(spki::Error::from)?;
                (CryptoKeyFormat::DerX509SubjectPublicKeyInfo, encoded)
            }
            DecodedKeyOwned::Raw(key) => (CryptoKeyFormat::Raw, key.to_vec()),
            DecodedKeyOwned::Symbolic(key) => (CryptoKeyFormat::Symbolic, key.to_vec()),
        };

        Ok(EncodedSigningPublicKey {
            format,
            public_key,
            key_spec: self.key_spec,
            usage: self.usage.clone(),
        })
    }

    /// Convert into encoded version of the key
    pub fn into_encoded(self) -> Result<EncodedSigningPublicKey, SigningPublicKeyError> {
        let (format, public_key) = match self.public_key {
            #[cfg(feature = "spki")]
            DecodedKeyOwned::Spki(key) => {
                use spki::der::Encode as _;

                let encoded = key.to_der().map_err(spki::Error::from)?;
                (CryptoKeyFormat::DerX509SubjectPublicKeyInfo, encoded)
            }
            DecodedKeyOwned::Raw(key) => (CryptoKeyFormat::Raw, key),
            DecodedKeyOwned::Symbolic(key) => (CryptoKeyFormat::Symbolic, key),
        };

        Ok(EncodedSigningPublicKey {
            format,
            public_key,
            key_spec: self.key_spec,
            usage: self.usage,
        })
    }

    /// Get the fingerprint of the key
    pub fn fingerprint(&self) -> Result<Fingerprint, SigningPublicKeyError> {
        self.to_encoded()?.fingerprint()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SigningPublicKeyError {
    inner: ErrorInner,
}

#[cfg(feature = "spki")]
impl From<spki::Error> for SigningPublicKeyError {
    fn from(value: spki::Error) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorInner {
    #[error("key is empty")]
    EmptyKey,

    #[error("bad crypto key format for a public signing key: '{format}'")]
    BadKeyFormat { format: CryptoKeyFormat },

    #[cfg(feature = "spki")]
    #[error(transparent)]
    SpkiError(#[from] spki::Error),

    #[allow(dead_code)]
    #[error("encoding format '{format}' is unsupported")]
    UnsupportedFormat { format: CryptoKeyFormat },
}
