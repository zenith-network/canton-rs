use std::{collections::HashSet, time::SystemTime};

use canton_proto::com::digitalasset::canton::protocol::v30;
use canton_types::{
    PackageId,
    topology::{ParticipantId, UniqueIdentifier},
};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue};

use crate::{
    TopologyMappingCode, TopologyMappingVariant,
    error::{ProtoDecodeError, ProtoEncodeError, UnexpectedTopologyMapping},
    mappings::TopologyMapping,
};

/// List of packages supported by this participant (VTP)
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
    /// Code of the topology mapping
    pub const CODE: TopologyMappingCode = TopologyMappingCode::VettedPackages;

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

impl TopologyMapping for VettedPackages {
    type ProtoV30 = v30::VettedPackages;

    /// In v31 VTP represented with the same message as in v30
    type ProtoV31 = v30::VettedPackages;

    fn code(&self) -> TopologyMappingCode {
        Self::CODE
    }

    fn into_variant(self) -> TopologyMappingVariant {
        TopologyMappingVariant::VettedPackages(self)
    }

    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping> {
        match variant {
            TopologyMappingVariant::VettedPackages(mapping) => Ok(mapping),
            other => Err(UnexpectedTopologyMapping::new(Self::CODE, other.code())),
        }
    }

    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError> {
        Ok(v30::VettedPackages {
            participant_uid: UniqueIdentifier::from(self.participant_uid).into(),
            #[allow(deprecated)]
            package_ids: Default::default(),
            packages: self
                .packages
                .into_iter()
                .map(VettedPackage::into_proto_v30)
                .collect(),
        })
    }

    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::VettedPackages;

        let participant_uid = UniqueIdentifier::parse(&proto.participant_uid)
            .validated_of::<Proto>("participant_uid")?
            .into();

        let mut packages = proto
            .packages
            .into_iter()
            .map(VettedPackage::from_proto_v30)
            .collect::<Result<Vec<_>, _>>()
            .validated_of::<Proto>("packages")?;

        #[allow(deprecated, reason = "support legacy mappings")]
        let package_ids = proto.package_ids;
        // Package hashes may only be listed in one of the two fields: `package_ids` or `packages`.
        // A package listed in `package_ids` is equivalent to a package listed in packages with
        // unbounded validity.
        packages.extend(
            package_ids
                .into_iter()
                .map(|id| PackageId::new(id).map(VettedPackage::new_unbounded_validity))
                .collect::<Result<Vec<_>, _>>()
                .validated_of::<Proto>("package_ids")?,
        );

        Self::new(participant_uid, packages)
            .map_err(|error| {
                let field_name = match error {
                    VettedPackageError::DuplicatePackageId => "packages",
                    // Only variants above are returned from Self::new
                    _ => unreachable!(),
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
    /// `valid_from_inclusive` must be less than `valid_until_exclusive` if both are present.
    pub fn new(
        package_id: PackageId,
        valid_from_inclusive: Option<SystemTime>,
        valid_until_exclusive: Option<SystemTime>,
    ) -> Result<Self, VettedPackageError> {
        if let (Some(from), Some(until)) = (valid_from_inclusive, valid_until_exclusive)
            && from >= until
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

    pub fn into_proto_v30(self) -> v30::vetted_packages::VettedPackage {
        v30::vetted_packages::VettedPackage {
            package_id: self.package_id.into(),
            valid_from_inclusive: self.valid_from_inclusive.map(Into::into),
            valid_until_exclusive: self.valid_until_exclusive.map(Into::into),
        }
    }

    pub fn from_proto_v30(
        proto: v30::vetted_packages::VettedPackage,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::vetted_packages::VettedPackage;

        let package_id = PackageId::new(proto.package_id).validated_of::<Proto>("package_id")?;

        let valid_from_inclusive = proto
            .valid_from_inclusive
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<Proto>("valid_from_inclusive")?;

        let valid_until_exclusive = proto
            .valid_until_exclusive
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<Proto>("valid_until_exclusive")?;

        Self::new(package_id, valid_from_inclusive, valid_until_exclusive)
            .map_err(ProtoDecodeError::other)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VettedPackageError {
    #[error("bad validity time range ('from' is after 'until')")]
    BadValidityTimeRange,
    #[error("duplicate package IDs")]
    DuplicatePackageId,
}
