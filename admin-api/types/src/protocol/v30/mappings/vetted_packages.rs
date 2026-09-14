use std::time::SystemTime;

use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::{PackageId, topology::Identifier};

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
    /// Participant vetting the packages
    pub participant_uid: Identifier,

    /// Hashes of the vetted packages.
    ///
    /// Package hashes may only be listed in one of the two fields: `package_ids` or `packages`.
    /// A package listed in `package_ids` is equivalent to a package listed in packages with
    /// unbounded validity.
    #[deprecated = "no longer used, but kept for backwards compatibility"]
    pub package_ids: Vec<PackageId>,

    /// Hashes of vetted packages with a validity period.
    ///
    /// Only one entry per `package_id` is permitted.
    pub packages: Vec<VettedPackage>,
}

impl From<VettedPackages> for proto::VettedPackages {
    fn from(value: VettedPackages) -> Self {
        Self {
            participant_uid: value.participant_uid.into(),
            #[allow(deprecated)]
            package_ids: Vec::new(),
            packages: value.packages.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::VettedPackages> for VettedPackages {
    type Error = ();

    fn try_from(value: proto::VettedPackages) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// Vetted package
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VettedPackage {
    /// The hash of the vetted package (package ID)
    pub package_id: PackageId,

    /// Earliest ledger effective time (inclusive) as of which the package is considered valid.
    ///
    /// Must be less than or equal to [`valid_until_exclusive`][Self::valid_until_exclusive] if both
    /// are present.
    pub valid_from_inclusive: Option<SystemTime>,

    /// Latest ledger effective time (exclusive) until which the package is considered valid.
    ///
    /// Must be greater than or equal to [`valid_from_inclusive`][Self::valid_from_inclusive] if
    /// both are present.
    pub valid_until_exclusive: Option<SystemTime>,
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
    type Error = ();

    fn try_from(value: proto::vetted_packages::VettedPackage) -> Result<Self, Self::Error> {
        todo!()
    }
}
