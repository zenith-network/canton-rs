use std::collections::HashSet;

use canton_proto::com::digitalasset::canton::crypto::v30 as proto;
use canton_types::{NonEmpty, U31};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::v30::{ConvertError, EncodedSigningPublicKey, SigningPublicKeyError};

#[derive(Debug, thiserror::Error)]
pub enum SigningKeysWithThresholdError {
    #[error("threshold is zero")]
    ZeroThreshold,

    #[error("keys with duplicating fingerprints are not allowed")]
    DuplicatingKeys,

    #[error(transparent)]
    SigningPublicKeyError(#[from] SigningPublicKeyError),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SigningKeysWithThreshold {
    keys: NonEmpty<EncodedSigningPublicKey>,
    // When std::num::ZeroablePrimitive becomes public and stable, we can implement it for U31 and
    // use std::num::NonZero<U31> here.
    threshold: U31,
}

impl SigningKeysWithThreshold {
    /// Create new signing keys with threshold
    pub fn new(
        keys: NonEmpty<EncodedSigningPublicKey>,
        threshold: U31,
    ) -> Result<Self, SigningKeysWithThresholdError> {
        // check that all keys have unique fingerprints
        let mut fingerprints = HashSet::new();
        for key in keys.iter() {
            let fingerprint = key.as_decoded()?.fingerprint()?;
            if !fingerprints.insert(fingerprint) {
                return Err(SigningKeysWithThresholdError::DuplicatingKeys);
            }
        }

        if threshold == 0u8.into() {
            return Err(SigningKeysWithThresholdError::ZeroThreshold);
        }

        Ok(Self { keys, threshold })
    }

    /// Designated signing keys
    pub fn keys(&self) -> &NonEmpty<EncodedSigningPublicKey> {
        &self.keys
    }

    /// Authorization threshold
    pub fn threshold(&self) -> U31 {
        self.threshold
    }
}

impl From<SigningKeysWithThreshold> for proto::SigningKeysWithThreshold {
    fn from(value: SigningKeysWithThreshold) -> Self {
        Self {
            keys: value.keys.into_iter().map(Into::into).collect(),
            threshold: value.threshold.into(),
        }
    }
}

impl TryFrom<proto::SigningKeysWithThreshold> for SigningKeysWithThreshold {
    type Error = ConvertError;

    fn try_from(value: proto::SigningKeysWithThreshold) -> Result<Self, Self::Error> {
        let keys_vec = value
            .keys
            .into_iter()
            .map(EncodedSigningPublicKey::try_from)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<proto::SigningKeysWithThreshold>("keys")?;
        let keys =
            NonEmpty::try_from(keys_vec).validated_of::<proto::SigningKeysWithThreshold>("keys")?;

        let threshold = value
            .threshold
            .try_into()
            .validated_of::<proto::SigningKeysWithThreshold>("threshold")?;

        Self::new(keys, threshold)
            .map_err(|error| {
                let field_name = match error {
                    SigningKeysWithThresholdError::ZeroThreshold => "threshold",
                    SigningKeysWithThresholdError::SigningPublicKeyError(_)
                    | SigningKeysWithThresholdError::DuplicatingKeys => "keys",
                };
                InvalidProtoFieldValue::new_of::<proto::SigningKeysWithThreshold>(field_name, error)
            })
            .map_err(Into::into)
    }
}
