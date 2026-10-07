use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::topology::MemberId;

use crate::crypto::v30::PublicKey;

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
pub struct OwnerToKeyMapping {
    /// The sequencing process member
    pub member: MemberId,

    /// Designated keys
    pub public_keys: Vec<PublicKey>,
}

impl From<OwnerToKeyMapping> for proto::OwnerToKeyMapping {
    fn from(value: OwnerToKeyMapping) -> Self {
        todo!()
    }
}

impl TryFrom<proto::OwnerToKeyMapping> for OwnerToKeyMapping {
    type Error = ();

    fn try_from(value: proto::OwnerToKeyMapping) -> Result<Self, Self::Error> {
        todo!()
    }
}
