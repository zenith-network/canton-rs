use canton_types::PackageId;
use daml_lf_archive_proto::com::digitalasset::daml::lf::archive as proto;
use daml_lf_version::Version;

use crate::MalformedPackage;

#[derive(Clone, Debug)]
pub struct Package {
    daml_lf_version: Version,
    package_id: PackageId,
    versioned: VersionedPackage,
}

impl Package {
    pub(crate) fn new(
        daml_lf_version: Version,
        package_id: PackageId,
        versioned: VersionedPackage,
    ) -> Self {
        Self {
            daml_lf_version,
            package_id,
            versioned,
        }
    }

    pub fn daml_lf_version(&self) -> Version {
        self.daml_lf_version
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn versioned(&self) -> &VersionedPackage {
        &self.versioned
    }

    pub fn seal(&self) -> Result<SealedPackage<'_>, MalformedPackage> {
        match &self.versioned {
            #[cfg(feature = "v2")]
            VersionedPackage::V2(package) => {
                let versioned =
                    VersionedSealedPackage::V2(crate::v2::sealed::Package::seal(package)?);
                Ok(SealedPackage {
                    daml_lf_version: self.daml_lf_version,
                    package_id: self.package_id.clone(),
                    versioned,
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum VersionedPackage {
    #[cfg(feature = "v2")]
    V2(proto::v2::Package),
}

#[cfg(feature = "v2")]
impl From<proto::v2::Package> for VersionedPackage {
    fn from(value: proto::v2::Package) -> Self {
        Self::V2(value)
    }
}

#[derive(Clone, Debug)]
pub struct SealedPackage<'a> {
    daml_lf_version: Version,
    package_id: PackageId,
    versioned: VersionedSealedPackage<'a>,
}

impl<'a> SealedPackage<'a> {
    pub fn daml_lf_version(&self) -> Version {
        self.daml_lf_version
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn versioned(&self) -> VersionedSealedPackage<'a> {
        self.versioned
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VersionedSealedPackage<'a> {
    #[cfg(feature = "v2")]
    V2(crate::v2::sealed::Package<'a>),
}

#[cfg(feature = "v2")]
impl<'a> From<crate::v2::sealed::Package<'a>> for VersionedSealedPackage<'a> {
    fn from(value: crate::v2::sealed::Package<'a>) -> Self {
        Self::V2(value)
    }
}
