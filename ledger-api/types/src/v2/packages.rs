use std::time::SystemTime;

use canton_types::{PackageId, PackageName, PackageVersion, ParticipantId, SynchronizerId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

/// The list of packages vetted on a given participant and synchronizer, modelled after
/// `VettedPackages` in `topology.proto`.
#[derive(Clone, Debug)]
pub struct VettedPackages {
    /// Sorted by `package_name` and `package_version` where known, and `package_id` as a last
    /// resort.
    pub packages: Vec<VettedPackage>,

    /// Participant on which these packages are vetted.
    pub participant_id: ParticipantId,

    /// Synchronizer on which these packages are vetted.
    pub synchronizer_id: SynchronizerId,

    /// Serial of last `VettedPackages` topology transaction of this participant and on this
    /// synchronizer.
    pub topology_serial: u32,
}

impl TryFrom<proto::VettedPackages> for VettedPackages {
    type Error = ValueError;

    fn try_from(value: proto::VettedPackages) -> Result<Self, Self::Error> {
        Ok(Self {
            packages: (!value.packages.is_empty())
                .then_some(value.packages)
                .required_of::<proto::VettedPackages>("packages")
                .no_msg()?
                .into_iter()
                .enumerate()
                .map(|(idx, package)| {
                    package
                        .try_into()
                        .validated_of::<proto::VettedPackages>("packages")
                        .with_msg_owned(format!("failed to convert packages[{idx}]"))
                })
                .collect::<Result<_, _>>()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<proto::VettedPackages>("participant_id")
                .no_msg()?,
            synchronizer_id: SynchronizerId::new(value.synchronizer_id)
                .validated_of::<proto::VettedPackages>("synchronizer_id")
                .no_msg()?,
            topology_serial: value.topology_serial,
        })
    }
}

/// A package that is vetting on a given participant and synchronizer, modelled after
/// `VettedPackage` in `topology.proto, enriched with the package name and version.
#[derive(Clone, Debug)]
pub struct VettedPackage {
    /// Package ID of this package
    pub package_id: PackageId,

    /// The time from which this package is vetted. Empty if vetting time has no lower bound.
    pub valid_from_inclusive: Option<SystemTime>,

    /// The time until which this package is vetted. Empty if vetting time has no upper bound.
    pub valid_until_exclusive: Option<SystemTime>,

    /// Name of this package.
    ///
    /// Only available if the package has been uploaded to the current participant.
    pub package_name: Option<PackageName>,

    /// Version of this package.
    ///
    /// Only available if the package has been uploaded to the current participant.
    pub package_version: Option<PackageVersion>,
}

impl TryFrom<proto::VettedPackage> for VettedPackage {
    type Error = ValueError;

    fn try_from(value: proto::VettedPackage) -> Result<Self, Self::Error> {
        Ok(Self {
            package_id: PackageId::new(value.package_id)
                .validated_of::<proto::VettedPackage>("package_id")
                .no_msg()?,
            valid_from_inclusive: value
                .valid_from_inclusive
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::VettedPackage>("valid_from_inclusive")
                .no_msg()?,
            valid_until_exclusive: value
                .valid_until_exclusive
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::VettedPackage>("valid_until_exclusive")
                .no_msg()?,
            package_name: (!value.package_name.is_empty())
                .then(|| PackageName::new(value.package_name))
                .transpose()
                .validated_of::<proto::VettedPackage>("package_name")
                .no_msg()?,
            package_version: (!value.package_version.is_empty())
                .then(|| PackageVersion::parse(&value.package_version))
                .transpose()
                .validated_of::<proto::VettedPackage>("package_version")
                .no_msg()?,
        })
    }
}
