use std::time::Duration;

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExperimentalFeatures {
    /// Ledger is in the static time mode and exposes a time service
    pub static_time: Option<bool>,

    /// Whether the Ledger API supports command inspection service
    pub command_inspection_service: Option<bool>,
}

impl From<proto::ExperimentalFeatures> for ExperimentalFeatures {
    fn from(value: proto::ExperimentalFeatures) -> Self {
        Self {
            static_time: value.static_time.map(|inner| inner.supported),
            command_inspection_service: value.static_time.map(|inner| inner.supported),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserManagementFeature {
    /// Whether the Ledger API server provides the user management service.
    pub supported: bool,

    /// The maximum number of rights that can be assigned to a single user.
    /// Servers MUST support at least 100 rights per user.
    /// A value of 0 means that the server enforces no rights per user limit.
    pub max_rights_per_user: i32,

    /// The maximum number of users the server can return in a single response (page).
    /// Servers MUST support at least a 100 users per page.
    /// A value of 0 means that the server enforces no page size limit.
    pub max_users_page_size: i32,
}

impl From<proto::UserManagementFeature> for UserManagementFeature {
    fn from(value: proto::UserManagementFeature) -> Self {
        Self {
            supported: value.supported,
            max_rights_per_user: value.max_rights_per_user,
            max_users_page_size: value.max_users_page_size,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PartyManagementFeature {
    /// The maximum number of parties the server can return in a single response (page).
    pub max_parties_page_size: i32,
}

impl From<proto::PartyManagementFeature> for PartyManagementFeature {
    fn from(value: proto::PartyManagementFeature) -> Self {
        Self {
            max_parties_page_size: value.max_parties_page_size,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OffsetCheckpointFeature {
    /// The maximum delay to emmit a new OffsetCheckpoint if it exists
    pub max_offset_checkpoint_emission_delay: Duration,
}

impl TryFrom<proto::OffsetCheckpointFeature> for OffsetCheckpointFeature {
    type Error = ValueError;

    fn try_from(value: proto::OffsetCheckpointFeature) -> Result<Self, Self::Error> {
        Ok(Self {
            max_offset_checkpoint_emission_delay: value
                .max_offset_checkpoint_emission_delay
                .required_of::<proto::OffsetCheckpointFeature>(
                    "max_offset_checkpoint_emission_delay",
                )
                .no_msg()?
                .try_into()
                .unwrap(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackageFeature {
    /// The maximum number of vetted packages the server can return in a single
    /// response (page) when listing them.
    pub max_vetted_packages_page_size: i32,
}

impl From<proto::PackageFeature> for PackageFeature {
    fn from(value: proto::PackageFeature) -> Self {
        Self {
            max_vetted_packages_page_size: value.max_vetted_packages_page_size,
        }
    }
}

/// Description of features enabled on the participant
#[derive(Clone, Debug)]
pub struct FeaturesDescriptor {
    /// Features under development or features that are used
    /// for ledger implementation testing purposes only.
    ///
    /// Daml applications SHOULD not depend on these in production.
    pub experimental: ExperimentalFeatures,

    /// If set, then the Ledger API server supports user management.
    /// It is recommended that clients query this field to gracefully adjust their behavior for
    /// ledgers that do not support user management.
    pub user_management: UserManagementFeature,

    /// If set, then the Ledger API server supports party management configurability.
    /// It is recommended that clients query this field to gracefully adjust their behavior to
    /// maximum party page size.
    pub party_management: PartyManagementFeature,

    /// It contains the timeouts related to the periodic offset checkpoint emission
    pub offset_checkpoint: OffsetCheckpointFeature,

    /// If set, then the Ledger API server supports package listing
    /// configurability. It is recommended that clients query this field to
    /// gracefully adjust their behavior to maximum package listing page size.
    pub package_feature: PackageFeature,
}

impl TryFrom<proto::FeaturesDescriptor> for FeaturesDescriptor {
    type Error = ValueError;

    fn try_from(value: proto::FeaturesDescriptor) -> Result<Self, Self::Error> {
        Ok(Self {
            experimental: value
                .experimental
                .required_of::<proto::FeaturesDescriptor>("experimental")
                .no_msg()?
                .into(),
            user_management: value
                .user_management
                .required_of::<proto::FeaturesDescriptor>("user_management")
                .no_msg()?
                .into(),
            party_management: value
                .party_management
                .required_of::<proto::FeaturesDescriptor>("party_management")
                .no_msg()?
                .into(),
            offset_checkpoint: value
                .offset_checkpoint
                .required_of::<proto::FeaturesDescriptor>("offset_checkpoint")
                .no_msg()?
                .try_into()
                .validated_of::<proto::FeaturesDescriptor>("offset_checkpoint")
                .no_msg()?,
            package_feature: value
                .package_feature
                .required_of::<proto::FeaturesDescriptor>("package_feature")
                .no_msg()?
                .into(),
        })
    }
}
