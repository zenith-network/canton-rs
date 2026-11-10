use std::{collections::BTreeMap, fmt};

use canton_proto::com::digitalasset::canton::admin::health::v30 as proto;
use canton_types::topology::UniqueIdentifier;
use canton_version::semver::Version;
use chrono::Duration;
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::error::ValueError;

/// Status of the component
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ComponentStatus {
    /// Component name
    pub name: String,

    /// Optional metadata labels
    ///
    /// For participant nodes, it can be  "synchronizer" -> physical synchronizer id for components
    /// that exist once per connected synchronizer.
    pub labels: BTreeMap<String, String>,

    /// Component status
    pub status: ComponentStatusVariants,
}

impl TryFrom<proto::ComponentStatus> for ComponentStatus {
    type Error = ValueError;

    fn try_from(value: proto::ComponentStatus) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            labels: value.labels,
            status: value
                .status
                .required_of::<proto::ComponentStatus>("status")?
                .try_into()?,
        })
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ComponentStatusVariants {
    Ok(StatusData),
    Degraded(StatusData),
    Failed(StatusData),
    Fatal(StatusData),
}

impl ComponentStatusVariants {
    // Returns `Option` for future compatibility
    pub fn status_data(&self) -> Option<&StatusData> {
        match self {
            ComponentStatusVariants::Ok(status_data) => Some(status_data),
            ComponentStatusVariants::Degraded(status_data) => Some(status_data),
            ComponentStatusVariants::Failed(status_data) => Some(status_data),
            ComponentStatusVariants::Fatal(status_data) => Some(status_data),
        }
    }

    /// Static string representation of a status
    pub const fn as_str(&self) -> &'static str {
        match self {
            ComponentStatusVariants::Ok(_) => "OK",
            ComponentStatusVariants::Degraded(_) => "DEGRADED",
            ComponentStatusVariants::Failed(_) => "FAILED",
            ComponentStatusVariants::Fatal(_) => "FATAL",
        }
    }
}

impl fmt::Display for ComponentStatusVariants {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())?;
        if let Some(desc) = self
            .status_data()
            .and_then(|data| data.description.as_ref())
        {
            write!(f, " ({desc})")?;
        }
        Ok(())
    }
}

impl TryFrom<proto::component_status::Status> for ComponentStatusVariants {
    type Error = ValueError;

    fn try_from(value: proto::component_status::Status) -> Result<Self, Self::Error> {
        Ok(match value {
            proto::component_status::Status::Ok(status_data) => {
                status_data.try_into().map(Self::Ok)?
            }
            proto::component_status::Status::Degraded(status_data) => {
                status_data.try_into().map(Self::Degraded)?
            }
            proto::component_status::Status::Failed(status_data) => {
                status_data.try_into().map(Self::Failed)?
            }
            proto::component_status::Status::Fatal(status_data) => {
                status_data.try_into().map(Self::Fatal)?
            }
        })
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StatusData {
    pub description: Option<String>,
}

impl TryFrom<proto::component_status::StatusData> for StatusData {
    type Error = ValueError;

    fn try_from(value: proto::component_status::StatusData) -> Result<Self, Self::Error> {
        Ok(Self {
            description: value.description,
        })
    }
}

/// Common status of a node
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Status {
    /// Unique identifier of a node
    pub uid: UniqueIdentifier,

    /// Uptime
    pub uptime: Option<Duration>,

    /// Ports
    pub ports: BTreeMap<String, i32>,

    /// Indicate if the node is active, usually true unless it's a replicated node that is passive
    pub active: bool,

    /// Health state of component dependencies of the node
    pub components: Vec<ComponentStatus>,

    /// Version of the binary
    pub version: Version,
    // TODO: implement missing fields
    // pub topology_queues: Option<TopologyQueueStatus>,
}

impl TryFrom<proto::Status> for Status {
    type Error = ValueError;

    fn try_from(value: proto::Status) -> Result<Self, Self::Error> {
        Ok(Self {
            uid: UniqueIdentifier::try_from(value.uid).validated_of::<proto::Status>("uid")?,
            uptime: value
                .uptime
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::Status>("uptime")?,
            ports: value.ports,
            active: value.active,
            components: value
                .components
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()
                .validated_of::<proto::Status>("components")?,
            version: Version::parse(&value.version).validated_of::<proto::Status>("version")?,
        })
    }
}

/// Status of a not initialized node
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NotInitialized {
    /// Indicate if the node is active, usually true unless it's a replicated node that is passive
    pub active: bool,

    /// Indicates whether the node is waiting for external input
    pub waiting_for_external_input: Option<WaitingForExternalInput>,

    /// Version of the binary
    ///
    /// This will be `Some(_)` only if returned version is parsed as a valid SemVer version.
    ///
    /// Older node versions may not populate this field.
    pub version: Option<Version>,
}

impl TryFrom<proto::NotInitialized> for NotInitialized {
    type Error = ValueError;

    fn try_from(value: proto::NotInitialized) -> Result<Self, Self::Error> {
        // Empty case is also covered here
        let version = Version::parse(&value.version).ok();

        let input_variant = proto::not_initialized::WaitingForExternalInput::try_from(
            value.waiting_for_external_input,
        )
        .validated_of::<proto::NotInitialized>("waiting_for_external_input")?;
        // See note on WaitingForExternalInput type
        let waiting_for_external_input = match input_variant {
            proto::not_initialized::WaitingForExternalInput::Unspecified => None,
            proto::not_initialized::WaitingForExternalInput::Id => {
                Some(WaitingForExternalInput::Id)
            }
            proto::not_initialized::WaitingForExternalInput::NodeTopology => {
                Some(WaitingForExternalInput::NodeTopology)
            }
            proto::not_initialized::WaitingForExternalInput::Initialization => {
                Some(WaitingForExternalInput::Initialization)
            }
        };

        Ok(Self {
            active: value.active,
            waiting_for_external_input,
            version,
        })
    }
}

// UNSPECIFIED variant in proto is used by Canton as a valid value,
// meaning that no WaitingForExternalInput was set. So we cannot implement
// TryFrom here for this type. Instead we provide the implementation inside
// NotInitialized, converting UNSPECIFIED to None.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WaitingForExternalInput {
    Id,
    NodeTopology,
    Initialization,
}
