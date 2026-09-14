use canton_proto::com::digitalasset::canton::topology::admin::v30 as proto;
use canton_types::topology::{PhysicalSynchronizerId, SynchronizerId};

#[derive(Clone, Debug)]
pub enum StoreId {
    Authorized,
    Synchronizer(Synchronizer),
    Temporary(Temporary),
}

impl From<StoreId> for proto::StoreId {
    fn from(value: StoreId) -> Self {
        Self {
            store: Some(match value {
                StoreId::Authorized => {
                    proto::store_id::Store::Authorized(proto::store_id::Authorized {})
                }
                StoreId::Synchronizer(synchronizer) => {
                    proto::store_id::Store::Synchronizer(synchronizer.into())
                }
                StoreId::Temporary(temporary) => {
                    proto::store_id::Store::Temporary(temporary.into())
                }
            }),
        }
    }
}

impl TryFrom<proto::StoreId> for StoreId {
    type Error = ();

    fn try_from(value: proto::StoreId) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug)]
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
    type Error = ();

    fn try_from(value: proto::Synchronizer) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct Temporary {
    pub name: String,
}

impl From<Temporary> for proto::store_id::Temporary {
    fn from(value: Temporary) -> Self {
        Self { name: value.name }
    }
}

impl TryFrom<proto::store_id::Temporary> for Temporary {
    type Error = ();

    fn try_from(value: proto::store_id::Temporary) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ForceFlag {
    // TODO: implement variants here
}

impl From<ForceFlag> for proto::ForceFlag {
    fn from(value: ForceFlag) -> Self {
        todo!()
    }
}

impl From<ForceFlag> for i32 {
    fn from(value: ForceFlag) -> Self {
        proto::ForceFlag::from(value).into()
    }
}
