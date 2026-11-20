use std::time::SystemTime;

use admin_api_types::error::ValueError;
use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::com::digitalasset::canton::admin::participant::v30::{
    self as proto, package_service_client as svc_proto,
};
use canton_protocol::error::ProtoDecodeError;
use canton_types::{NonZeroU31, PackageId, PackageName, PackageVersion, topology::SynchronizerId};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`PackageServiceClient`][svc_proto::PackageServiceClient].
#[derive(Clone, Debug)]
pub struct PackageClient {
    service: svc_proto::PackageServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl PackageClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::PackageServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
    ) -> Self {
        Self {
            service,
            retry_handler,
        }
    }

    /// Set retry config for the client
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// List all Daml-LF archives on the participant node.
    ///
    /// # Arguments
    ///
    /// - `limit` - Maximum number of packages to retrieve. Set to `None` for no limit.
    /// - `filter_name` - Match package names starting with the given prefix. Set to `None` for
    ///   no name filter. Canton applies the limit before filtering by name.
    pub async fn list_packages(
        &self,
        limit: Option<NonZeroU31>,
        filter_name: Option<String>,
    ) -> Result<Vec<PackageDescription>, AdminApiError> {
        let request = proto::ListPackagesRequest {
            limit: limit.map(Into::into).unwrap_or_default(),
            filter_name: filter_name.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_packages(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .package_descriptions
            .into_iter()
            .map(|description| description.try_into().map_err(AdminApiError::value_error))
            .collect()
    }

    /// Performs the same checks that uploading a DAR would perform, but doesn't upload the DAR.
    ///
    /// # Arguments
    ///
    /// - `data` - Bytes of the DAR file to validate.
    /// - `filename` - Filename of the DAR being validated.
    /// - `synchronizer_id` - Used to specify the synchronizer on which to validate that vetting
    ///   is correct. If omitted, Canton autodetects the synchronizer when the participant is
    ///   connected to a single one.
    pub async fn validate_dar(
        &self,
        data: Vec<u8>,
        filename: String,
        synchronizer_id: Option<SynchronizerId>,
    ) -> Result<PackageId, AdminApiError> {
        let request = proto::ValidateDarRequest {
            data,
            filename,
            synchronizer_id: synchronizer_id.map(Into::into),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.validate_dar(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?;

        PackageId::new(response.main_package_id)
            .validated_of::<proto::ValidateDarResponse>("main_package_id")
            .map_err(AdminApiError::value_error)
    }

    /// Upload DAR files and all packages inside to the participant node.
    ///
    /// Returns the main package ID of each uploaded DAR in request order.
    ///
    /// # Arguments
    ///
    /// - `dars` - DAR files to upload.
    /// - `vet_all_packages` - If set to `true`, the node will register the vetting topology
    ///   transactions on the specified synchronizer or autodetect the synchronizer if
    ///   the participant is only connected to a single one. If multiple synchronizers are connected
    ///   and no synchronizer is specified, an error will be returned.
    /// - `synchronize_vetting` - If set to `true`, we'll wait until the vetting transaction has
    ///   been observed by this participant for the target synchronizer.
    /// - `synchronizer_id` - Used to specify the synchronizer to vet packages on when
    ///   `vet_all_packages` is true.
    pub async fn upload_dars(
        &self,
        dars: Vec<impl Into<UploadDarData>>,
        vet_all_packages: bool,
        synchronize_vetting: bool,
        synchronizer_id: Option<SynchronizerId>,
    ) -> Result<Vec<PackageId>, AdminApiError> {
        let request = proto::UploadDarRequest {
            dars: dars
                .into_iter()
                .map(|inp| inp.into())
                .map(Into::into)
                .collect(),
            vet_all_packages,
            synchronize_vetting,
            synchronizer_id: synchronizer_id.map(Into::into),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.upload_dar(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .dar_ids
            .into_iter()
            .map(|id| {
                PackageId::new(id)
                    .validated_of::<proto::UploadDarResponse>("dar_ids")
                    .map_err(AdminApiError::value_error)
            })
            .collect()
    }

    /// Same as [`Self::upload_dars`], but for a single DAR.
    pub async fn upload_dar(
        &self,
        dar: impl Into<UploadDarData>,
        vet_all_packages: bool,
        synchronize_vetting: bool,
        synchronizer_id: Option<SynchronizerId>,
    ) -> Result<PackageId, AdminApiError> {
        let ids = self
            .upload_dars(
                vec![dar],
                vet_all_packages,
                synchronize_vetting,
                synchronizer_id,
            )
            .await?;

        let [id]: [PackageId; 1] = ids
            .try_into()
            .map_err(|_| ProtoDecodeError::other("expected exactly one DAR package ID"))
            .validated_of::<proto::UploadDarResponse>("dar_ids")
            .map_err(AdminApiError::value_error)?;
        Ok(id)
    }

    /// List all DARs on the participant node.
    ///
    /// # Arguments
    ///
    /// - `limit` - Maximum number of DARs to retrieve. Set to `None` for no limit.
    /// - `filter_name` - Match DAR names starting with the given prefix. Set to `None` for
    ///   no name filter. Canton applies the limit before filtering by name.
    pub async fn list_dars(
        &self,
        limit: Option<NonZeroU31>,
        filter_name: Option<String>,
    ) -> Result<Vec<DarDescription>, AdminApiError> {
        let request = proto::ListDarsRequest {
            limit: limit.map(Into::into).unwrap_or_default(),
            filter_name: filter_name.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_dars(req).await.map_err(AdminApiError::from_status)
            })
            .await?
            .dars
            .into_iter()
            .map(|description| description.try_into().map_err(AdminApiError::value_error))
            .collect()
    }
}

/// DAR file to upload
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct UploadDarData {
    /// Bytes of the DAR file
    pub bytes: Vec<u8>,

    /// Description of the DAR
    ///
    /// Could be set to human-readable description of the DAR. If omitted, Canton will set it to
    /// `DAR_<main_package_id>`.
    pub description: Option<String>,

    /// Expected package ID of the main package.
    ///
    /// Used to ensure that you're uploading the DAR with desired package inside. When this is set,
    /// Canton will return error if the resolved package ID doesn't match this value and the DAR
    /// won't be uploaded.
    pub expected_main_package_id: Option<PackageId>,
}

impl UploadDarData {
    /// Create new DAR uploading data
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            description: None,
            expected_main_package_id: None,
        }
    }

    /// Set description for uploading DAR
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set expected main package ID for uploading DAR
    pub fn with_expected_main_package_id(mut self, package_id: PackageId) -> Self {
        self.expected_main_package_id = Some(package_id);
        self
    }
}

impl From<Vec<u8>> for UploadDarData {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

impl From<UploadDarData> for proto::upload_dar_request::UploadDarData {
    fn from(value: UploadDarData) -> Self {
        Self {
            bytes: value.bytes,
            description: value.description,
            expected_main_package_id: value.expected_main_package_id.map(Into::into),
        }
    }
}

/// Daml-LF archive on the participant node
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PackageDescription {
    /// Package ID
    pub package_id: PackageId,

    /// Package name
    pub name: PackageName,

    /// Package version
    pub version: PackageVersion,

    /// Time the package was uploaded
    pub uploaded_at: SystemTime,

    /// Package size in bytes
    pub size: u32,
}

impl TryFrom<proto::PackageDescription> for PackageDescription {
    type Error = ValueError;

    fn try_from(value: proto::PackageDescription) -> Result<Self, Self::Error> {
        Ok(Self {
            package_id: PackageId::new(value.package_id)
                .validated_of::<proto::PackageDescription>("package_id")?,
            name: PackageName::new(value.name).validated_of::<proto::PackageDescription>("name")?,
            version: PackageVersion::parse(&value.version)
                .validated_of::<proto::PackageDescription>("version")?,
            uploaded_at: value
                .uploaded_at
                .required_of::<proto::PackageDescription>("uploaded_at")?
                .try_into()
                .validated_of::<proto::PackageDescription>("uploaded_at")?,
            size: value.size,
        })
    }
}

/// DAR on the participant node
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DarDescription {
    /// Main package ID
    pub main: PackageId,

    /// Package name
    pub name: PackageName,

    /// Package version
    pub version: PackageVersion,

    /// DAR description
    pub description: String,
}

impl TryFrom<proto::DarDescription> for DarDescription {
    type Error = ValueError;

    fn try_from(value: proto::DarDescription) -> Result<Self, Self::Error> {
        Ok(Self {
            main: PackageId::new(value.main).validated_of::<proto::DarDescription>("main")?,
            name: PackageName::new(value.name).validated_of::<proto::DarDescription>("name")?,
            version: PackageVersion::parse(&value.version)
                .validated_of::<proto::DarDescription>("version")?,
            description: value.description,
        })
    }
}
