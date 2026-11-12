use std::collections::HashSet;

use canton_crypto::v30::{EncodedPublicKey, SigningPublicKeyError};
use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::{NonEmpty, topology::MemberId};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::error::ValueError;

/// Mapping a member (participant, mediator, sequencer) to a key (OTK)
///
/// ## Authorization
///
/// Whoever controls the member uid
///
/// ## Uniqueness key
///
/// ```plaintext
/// member
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct OwnerToKeyMapping {
    member: MemberId,
    public_keys: NonEmpty<EncodedPublicKey>,
}

impl OwnerToKeyMapping {
    pub fn new(
        member: MemberId,
        public_keys: NonEmpty<EncodedPublicKey>,
    ) -> Result<Self, OtkError> {
        // check that all keys have unique fingerprints
        let mut fingerprints = HashSet::new();
        for key in public_keys.iter() {
            let fingerprint = key.fingerprint()?;
            if !fingerprints.insert(fingerprint) {
                return Err(OtkError::DuplicatingKeys);
            }
        }

        Ok(Self {
            member,
            public_keys,
        })
    }

    /// The sequencing process member
    pub fn member(&self) -> &MemberId {
        &self.member
    }

    /// Designated keys
    pub fn public_keys(&self) -> &NonEmpty<EncodedPublicKey> {
        &self.public_keys
    }
}

impl From<OwnerToKeyMapping> for proto::OwnerToKeyMapping {
    fn from(value: OwnerToKeyMapping) -> Self {
        Self {
            member: value.member.into(),
            public_keys: value.public_keys.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::OwnerToKeyMapping> for OwnerToKeyMapping {
    type Error = ValueError;

    fn try_from(value: proto::OwnerToKeyMapping) -> Result<Self, Self::Error> {
        let member =
            MemberId::parse(&value.member).validated_of::<proto::OwnerToKeyMapping>("member")?;
        let public_keys_vec = value
            .public_keys
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<proto::OwnerToKeyMapping>("public_keys")?;
        let public_keys = NonEmpty::try_from(public_keys_vec)
            .validated_of::<proto::OwnerToKeyMapping>("public_keys")?;

        Self::new(member, public_keys)
            .map_err(|error| {
                let field_name = match error {
                    OtkError::DuplicatingKeys | OtkError::SigningPublicKeyError(_) => "public_keys",
                };
                InvalidProtoFieldValue::new_of::<proto::OwnerToKeyMapping>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Owner to key mapping error
#[derive(Debug, thiserror::Error)]
pub enum OtkError {
    #[error("keys with duplicating fingerprints are not allowed")]
    DuplicatingKeys,

    #[error(transparent)]
    SigningPublicKeyError(#[from] SigningPublicKeyError),
}
