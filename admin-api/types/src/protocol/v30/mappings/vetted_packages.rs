use std::{collections::HashSet, time::SystemTime};

use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::{
    PackageId,
    topology::{ParticipantId, UniqueIdentifier},
};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::error::ValueError;

/// List of packages supported by this participant
///
/// ## Authorization
///
/// Whoever controls the participant uid
///
/// ## Uniqueness key
///
/// ```plaintext
/// participant
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VettedPackages {
    participant_uid: ParticipantId,
    packages: Vec<VettedPackage>,
}

impl VettedPackages {
    /// Create new vetted packages mapping
    ///
    /// Only one entry per `package_id` is permitted.
    pub fn new(
        participant_uid: ParticipantId,
        packages: Vec<VettedPackage>,
    ) -> Result<Self, VettedPackageError> {
        let mut package_ids = HashSet::new();
        for package in &packages {
            if !package_ids.insert(&package.package_id) {
                return Err(VettedPackageError::DuplicatePackageId);
            }
        }
        Ok(Self {
            participant_uid,
            packages,
        })
    }

    /// Participant vetting the packages
    pub fn participant_uid(&self) -> &ParticipantId {
        &self.participant_uid
    }

    /// Hashes of vetted packages with a validity period.
    pub fn packages(&self) -> &[VettedPackage] {
        &self.packages
    }

    /// Hashes of the vetted packages.
    pub fn package_ids(&self) -> impl Iterator<Item = &PackageId> {
        self.packages.iter().map(|package| &package.package_id)
    }
}

impl From<VettedPackages> for proto::VettedPackages {
    fn from(value: VettedPackages) -> Self {
        Self {
            participant_uid: UniqueIdentifier::from(value.participant_uid).into(),
            #[allow(deprecated)]
            package_ids: Default::default(),
            packages: value.packages.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::VettedPackages> for VettedPackages {
    type Error = ValueError;

    fn try_from(value: proto::VettedPackages) -> Result<Self, Self::Error> {
        let participant_uid = UniqueIdentifier::parse(&value.participant_uid)
            .validated_of::<proto::VettedPackages>("participant_uid")?
            .into();

        let mut packages = value
            .packages
            .into_iter()
            .map(VettedPackage::try_from)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<proto::VettedPackages>("packages")?;

        #[allow(deprecated, reason = "support legacy mappings")]
        let package_ids = value.package_ids;
        // Package hashes may only be listed in one of the two fields: `package_ids` or `packages`.
        // A package listed in `package_ids` is equivalent to a package listed in packages with
        // unbounded validity.
        packages.extend(
            package_ids
                .into_iter()
                .map(|id| PackageId::new(id).map(VettedPackage::new_unbounded_validity))
                .collect::<Result<Vec<_>, _>>()
                .validated_of::<proto::VettedPackages>("package_ids")?,
        );

        Self::new(participant_uid, packages)
            .map_err(|error| {
                let field_name = match error {
                    VettedPackageError::DuplicatePackageId => "packages",
                    // Only variants above are returned from Self::new
                    _ => unreachable!(),
                };
                InvalidProtoFieldValue::new_of::<proto::VettedPackages>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Vetted package
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VettedPackage {
    package_id: PackageId,
    valid_from_inclusive: Option<SystemTime>,
    valid_until_exclusive: Option<SystemTime>,
}

impl VettedPackage {
    /// Create new vetted package mapping
    ///
    /// `valid_from_inclusive` must be less or equal to `valid_until_exclusive` if both are present.
    pub fn new(
        package_id: PackageId,
        valid_from_inclusive: Option<SystemTime>,
        valid_until_exclusive: Option<SystemTime>,
    ) -> Result<Self, VettedPackageError> {
        if let (Some(from), Some(until)) = (valid_from_inclusive, valid_until_exclusive)
            && from > until
        {
            return Err(VettedPackageError::BadValidityTimeRange);
        }
        Ok(Self {
            package_id,
            valid_from_inclusive,
            valid_until_exclusive,
        })
    }

    /// Create new vetted package mapping setting validity timestamps to `None`
    pub const fn new_unbounded_validity(package_id: PackageId) -> Self {
        Self {
            package_id,
            valid_from_inclusive: None,
            valid_until_exclusive: None,
        }
    }

    /// The hash of the vetted package (package ID)
    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    /// Earliest ledger effective time (inclusive) as of which the package is considered valid.
    ///
    /// Always less than or equal to [`valid_until_exclusive`][Self::valid_until_exclusive()] if
    /// both are present.
    pub fn valid_from_inclusive(&self) -> Option<SystemTime> {
        self.valid_from_inclusive
    }

    /// Latest ledger effective time (exclusive) until which the package is considered valid.
    ///
    /// Always greater than or equal to [`valid_from_inclusive`][Self::valid_from_inclusive()] if
    /// both are present.
    pub fn valid_until_exclusive(&self) -> Option<SystemTime> {
        self.valid_until_exclusive
    }
}

impl From<VettedPackage> for proto::vetted_packages::VettedPackage {
    fn from(value: VettedPackage) -> Self {
        Self {
            package_id: value.package_id.into(),
            valid_from_inclusive: value.valid_from_inclusive.map(Into::into),
            valid_until_exclusive: value.valid_until_exclusive.map(Into::into),
        }
    }
}

impl TryFrom<proto::vetted_packages::VettedPackage> for VettedPackage {
    type Error = ValueError;

    fn try_from(value: proto::vetted_packages::VettedPackage) -> Result<Self, Self::Error> {
        let package_id = PackageId::new(value.package_id)
            .validated_of::<proto::vetted_packages::VettedPackage>("package_id")?;
        let valid_from_inclusive = value
            .valid_from_inclusive
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<proto::vetted_packages::VettedPackage>("valid_from_inclusive")?;
        let valid_until_exclusive = value
            .valid_until_exclusive
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<proto::vetted_packages::VettedPackage>("valid_until_exclusive")?;

        Self::new(package_id, valid_from_inclusive, valid_until_exclusive)
            .map_err(|error| {
                let field_name = match error {
                    // Field name here is a bit weird, those two fields are validated together and I
                    // think it will be understood by user, what does that means, given also the
                    // source error message
                    VettedPackageError::BadValidityTimeRange => {
                        "valid_from_inclusive|valid_until_exclusive"
                    }
                    // Only variants above are returned from Self::new
                    _ => unreachable!(),
                };
                InvalidProtoFieldValue::new_of::<proto::vetted_packages::VettedPackage>(
                    field_name, error,
                )
            })
            .map_err(Into::into)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VettedPackageError {
    #[error("bad validity time range ('from' is after 'until')")]
    BadValidityTimeRange,
    #[error("duplicate package IDs")]
    DuplicatePackageId,
}
