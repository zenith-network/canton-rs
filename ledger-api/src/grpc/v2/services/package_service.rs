use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetPackageRequest, GetPackageStatusRequest, ListPackagesRequest,
    ListPackagesResponse, package_service_client as svc_proto,
};
use ledger_api_types::{canton_types::PackageId, value::v2::errors::IntoValueError as _};
use protobuf_utils::InvalidProtoField as _;

use crate::grpc::v2::{client::InterceptedService, error::CantonError};

/// Wrapped for [`svc_proto::StateServiceClient`]
pub struct PackageServiceClient {
    service: svc_proto::PackageServiceClient<InterceptedService>,
}

impl PackageServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn from_tonic(service: svc_proto::PackageServiceClient<InterceptedService>) -> Self {
        Self { service }
    }

    /// Returns the contents of a single package
    pub async fn get_package(&mut self, package_id: PackageId) -> Result<Vec<u8>, CantonError> {
        Ok(self
            .service
            .get_package(GetPackageRequest {
                package_id: package_id.into(),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner()
            .archive_payload)
    }

    /// Returns `true` if package status is `Regirstered`
    pub async fn package_registered(&mut self, package_id: PackageId) -> Result<bool, CantonError> {
        let response = self
            .service
            .get_package_status(GetPackageStatusRequest {
                package_id: package_id.into(),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner();

        // FIXME: replace unwrap with error
        let status = proto::PackageStatus::try_from(response.package_status).unwrap();

        Ok(status == proto::PackageStatus::Registered)
    }

    /// Returns the identifiers of all supported packages
    pub async fn list_packages(&mut self) -> Result<Vec<PackageId>, CantonError> {
        let response = self
            .service
            .list_packages(ListPackagesRequest {})
            .await
            .map_err(CantonError::from)?
            .into_inner();
        response
            .package_ids
            .into_iter()
            .enumerate()
            .map(|(idx, package_id)| {
                PackageId::new(package_id)
                    .validated_of::<ListPackagesResponse>("package_ids")
                    .with_msg_owned(format!("failed to convert package_ids[{idx}]"))
                    .map_err(CantonError::value_error)
            })
            .collect::<Result<_, _>>()
    }

    pub async fn list_vetted_packages(&mut self) {
        todo!()
    }
}
