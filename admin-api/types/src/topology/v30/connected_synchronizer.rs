use canton_proto::com::digitalasset::canton::admin::participant::v30 as proto;
use canton_types::topology::PhysicalSynchronizerId;
use protobuf_utils::{InvalidProtoField as _, UnspecifiedEnumValue};

use crate::error::ValueError;

/// Connected synchronizer
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ConnectedSynchronizer {
    pub physical_synchronizer_id: PhysicalSynchronizerId,
    pub health: Health,
}

impl TryFrom<proto::ConnectedSynchronizer> for ConnectedSynchronizer {
    type Error = ValueError;

    fn try_from(value: proto::ConnectedSynchronizer) -> Result<Self, Self::Error> {
        Ok(Self {
            physical_synchronizer_id: PhysicalSynchronizerId::try_from(
                value.physical_synchronizer_id,
            )
            .validated_of::<proto::ConnectedSynchronizer>("physical_synchronizer_id")?,
            health: value
                .health
                .try_into()
                .validated_of::<proto::ConnectedSynchronizer>("health")?,
        })
    }
}

/// Health of a connected synchronizer
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Health {
    Healthy,
    Unhealthy,
}

impl TryFrom<proto::connected_synchronizer::Health> for Health {
    type Error = ValueError;

    fn try_from(value: proto::connected_synchronizer::Health) -> Result<Self, Self::Error> {
        match value {
            proto::connected_synchronizer::Health::Unspecified => {
                Err(UnspecifiedEnumValue::new().into())
            }
            proto::connected_synchronizer::Health::Healthy => Ok(Self::Healthy),
            proto::connected_synchronizer::Health::Unhealthy => Ok(Self::Unhealthy),
        }
    }
}

impl TryFrom<i32> for Health {
    type Error = ValueError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        proto::connected_synchronizer::Health::try_from(value)?.try_into()
    }
}
