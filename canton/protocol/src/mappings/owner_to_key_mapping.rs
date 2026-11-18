use std::collections::HashSet;

use canton_crypto::v30::{EncodedPublicKey, SigningPublicKeyError};
use canton_proto::com::digitalasset::canton::protocol::v30;
use canton_types::{NonEmpty, topology::MemberId};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::{
    TopologyMappingCode, TopologyMappingVariant,
    error::{ProtoDecodeError, ProtoEncodeError, UnexpectedTopologyMapping},
    mappings::TopologyMapping,
};

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
    /// Code of the topology mapping
    pub const CODE: TopologyMappingCode = TopologyMappingCode::OwnerToKeyMapping;

    pub fn new(
        member: MemberId,
        public_keys: NonEmpty<EncodedPublicKey>,
    ) -> Result<Self, OtkError> {
        // TODO: require max number of keys to be 20
        // check that all keys have unique fingerprints
        let mut fingerprints = HashSet::new();
        for key in public_keys.iter() {
            let fingerprint = key.fingerprint()?;
            if !fingerprints.insert(fingerprint) {
                return Err(OtkError::DuplicateFingerprints);
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

impl TopologyMapping for OwnerToKeyMapping {
    type ProtoV30 = v30::OwnerToKeyMapping;

    /// In v31 OTK represented with the same message as in v30
    type ProtoV31 = v30::OwnerToKeyMapping;

    fn code(&self) -> TopologyMappingCode {
        Self::CODE
    }

    fn into_variant(self) -> TopologyMappingVariant {
        TopologyMappingVariant::OwnerToKeyMapping(self)
    }

    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping> {
        match variant {
            TopologyMappingVariant::OwnerToKeyMapping(mapping) => Ok(mapping),
            other => Err(UnexpectedTopologyMapping::new(Self::CODE, other.code())),
        }
    }

    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError> {
        Ok(v30::OwnerToKeyMapping {
            member: self.member.into(),
            public_keys: self.public_keys.into_iter().map(Into::into).collect(),
        })
    }

    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::OwnerToKeyMapping;

        let member = MemberId::parse(&proto.member).validated_of::<Proto>("member")?;

        let public_keys_vec = proto
            .public_keys
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<Proto>("public_keys")?;
        let public_keys =
            NonEmpty::try_from(public_keys_vec).validated_of::<Proto>("public_keys")?;

        Self::new(member, public_keys)
            .map_err(|error| {
                let field_name = match error {
                    OtkError::DuplicateFingerprints | OtkError::SigningPublicKeyError(_) => {
                        "public_keys"
                    }
                };
                InvalidProtoFieldValue::new_of::<Proto>(field_name, error)
            })
            .map_err(Into::into)
    }

    fn into_proto_v31(self) -> Result<Self::ProtoV31, ProtoEncodeError> {
        self.into_proto_v30()
    }

    fn from_proto_v31(proto: Self::ProtoV31) -> Result<Self, ProtoDecodeError> {
        Self::from_proto_v30(proto)
    }
}

/// Owner to key mapping error
#[derive(Debug, thiserror::Error)]
pub enum OtkError {
    #[error("multiple keys with the same fingerprint are not allowed")]
    DuplicateFingerprints,

    #[error(transparent)]
    SigningPublicKeyError(#[from] SigningPublicKeyError),
}
