use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;
use protobuf_utils::{InvalidProtoField, RequiredProtoField as _};

use crate::v2::{
    errors::{MalformedPackage, MalformedPackageContext as _},
    seal::seal_interned_str,
    sealed::Package,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackageMetadata<'a> {
    name: &'a str,
    version: &'a str,
    upgraded_package_id: Option<&'a str>,
}

impl<'a> PackageMetadata<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn version(&self) -> &'a str {
        self.version
    }

    pub fn upgraded_package_id(&self) -> Option<&'a str> {
        self.upgraded_package_id
    }

    pub(crate) fn seal(package: &'a proto::Package) -> Result<(), MalformedPackage> {
        let metadata = package
            .metadata
            .required_of::<proto::Package>("metadata")
            .default_context()?;
        seal_interned_str(metadata.name_interned_str, package)
            .validated_of::<proto::PackageMetadata>("name_interned_str")
            .default_context()?;
        seal_interned_str(metadata.version_interned_str, package)
            .validated_of::<proto::PackageMetadata>("version_interned_str")
            .default_context()?;
        metadata
            .upgraded_package_id
            .map(|upgraded_package_id| {
                seal_interned_str(
                    upgraded_package_id.upgraded_package_id_interned_str,
                    package,
                )
                .validated_of::<proto::UpgradedPackageId>("upgraded_package_id_interned_str")
                .default_context()
            })
            .transpose()?;
        Ok(())
    }

    pub(crate) fn from_unsealed(package: Package<'a>) -> Self {
        let metadata = package.as_unsealed().metadata.as_ref().unwrap();
        let name = package.get_interned_string(metadata.name_interned_str);
        let version = package.get_interned_string(metadata.version_interned_str);
        let upgraded_package_id = metadata
            .upgraded_package_id
            .map(|upid| package.get_interned_string(upid.upgraded_package_id_interned_str));
        Self {
            name,
            version,
            upgraded_package_id,
        }
    }
}
