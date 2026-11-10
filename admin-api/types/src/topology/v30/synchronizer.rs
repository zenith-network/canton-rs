use canton_proto::com::digitalasset::canton::topology::admin::v30 as proto;
use canton_types::topology::{PhysicalSynchronizerId, SynchronizerId};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::error::ValueError;

/// Synchronizer ID (logical or physical)
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Synchronizer {
    Id(SynchronizerId),
    PhysicalId(PhysicalSynchronizerId),
}

impl From<Synchronizer> for proto::Synchronizer {
    fn from(value: Synchronizer) -> Self {
        use proto::synchronizer::Kind;
        Self {
            kind: Some(match value {
                Synchronizer::Id(synchronizer_id) => Kind::Id(synchronizer_id.into()),
                Synchronizer::PhysicalId(physical_synchronizer_id) => {
                    Kind::PhysicalId(physical_synchronizer_id.into())
                }
            }),
        }
    }
}

impl TryFrom<proto::Synchronizer> for Synchronizer {
    type Error = ValueError;

    fn try_from(value: proto::Synchronizer) -> Result<Self, Self::Error> {
        use proto::synchronizer::Kind;

        let kind = value.kind.required_of::<proto::Synchronizer>("kind")?;
        match kind {
            Kind::Id(id) => SynchronizerId::parse(&id)
                .map(Self::Id)
                .validated_of::<proto::Synchronizer>("kind")
                .map_err(Into::into),
            Kind::PhysicalId(id) => PhysicalSynchronizerId::parse(&id)
                .map(Self::PhysicalId)
                .validated_of::<proto::Synchronizer>("kind")
                .map_err(Into::into),
        }
    }
}
