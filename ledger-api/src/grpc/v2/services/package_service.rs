use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetPackageRequest, GetPackageStatusRequest, ListPackagesRequest,
    ListPackagesResponse, package_service_client as svc_proto,
};
use ledger_api_types::{canton_types::PackageId, value::v2::errors::IntoValueError as _};
use protobuf_utils::InvalidProtoField as _;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::StateServiceClient`]
#[derive(Clone, Debug)]
pub struct PackageServiceClient {
    service: svc_proto::PackageServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl PackageServiceClient {
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

    /// Returns the contents of a single package
    pub async fn get_package(&mut self, package_id: PackageId) -> Result<Vec<u8>, CantonError> {
        let request = GetPackageRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package(req).await
            })
            .await?;

        Ok(response.archive_payload)
    }

    /// Returns `true` if package status is `Regirstered`
    pub async fn package_registered(&mut self, package_id: PackageId) -> Result<bool, CantonError> {
        let request = GetPackageStatusRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package_status(req).await
            })
            .await?;

        // FIXME: replace unwrap with error
        let status = proto::PackageStatus::try_from(response.package_status).unwrap();

        Ok(status == proto::PackageStatus::Registered)
    }

    /// Returns the identifiers of all supported packages
    pub async fn list_packages(&mut self) -> Result<Vec<PackageId>, CantonError> {
        let response = self
            .retry_handler
            .call(&self.service, &(), |mut svc, _| async move {
                svc.list_packages(ListPackagesRequest {}).await
            })
            .await?;

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
}
