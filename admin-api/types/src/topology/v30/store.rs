use canton_proto::com::digitalasset::canton::topology::admin::v30 as proto;
use protobuf_utils::RequiredProtoField as _;

use crate::{error::ValueError, topology::v30::Synchronizer};

/// Authorized store of topology transaction
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Authorized {}

impl From<Authorized> for proto::store_id::Authorized {
    fn from(_: Authorized) -> Self {
        Self {}
    }
}

impl TryFrom<proto::store_id::Authorized> for Authorized {
    type Error = ValueError;

    fn try_from(_: proto::store_id::Authorized) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

/// Temporary topology transaction store
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Temporary {
    pub name: String,
}

impl Temporary {
    pub fn with_name(name: String) -> Self {
        Self { name }
    }
}

impl From<Temporary> for proto::store_id::Temporary {
    fn from(value: Temporary) -> Self {
        Self { name: value.name }
    }
}

impl TryFrom<proto::store_id::Temporary> for Temporary {
    type Error = ValueError;

    fn try_from(value: proto::store_id::Temporary) -> Result<Self, Self::Error> {
        Ok(Self { name: value.name })
    }
}

/// Topology transaction store identifier
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum StoreId {
    Authorized(Authorized),
    Synchronizer(Synchronizer),
    Temporary(Temporary),
}

impl From<StoreId> for proto::StoreId {
    fn from(value: StoreId) -> Self {
        Self {
            store: Some(match value {
                StoreId::Authorized(authorized) => {
                    proto::store_id::Store::Authorized(authorized.into())
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
    type Error = ValueError;

    fn try_from(value: proto::StoreId) -> Result<Self, Self::Error> {
        use proto::store_id::Store;

        let store = value.store.required_of::<proto::StoreId>("store")?;
        match store {
            Store::Authorized(authorized) => authorized.try_into().map(Self::Authorized),
            Store::Synchronizer(synchronizer) => synchronizer.try_into().map(Self::Synchronizer),
            Store::Temporary(temporary) => temporary.try_into().map(Self::Temporary),
        }
    }
}
