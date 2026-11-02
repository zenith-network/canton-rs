use canton_base::retry::{RetryConfig, RetryHandler};
use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetPackageRequest, GetPackageStatusRequest, ListPackagesRequest,
    ListPackagesResponse, package_service_client as svc_proto,
};
use ledger_api_types::{canton_types::PackageId, value::v2::errors::IntoValueError as _};
use protobuf_utils::InvalidProtoField as _;

use crate::grpc::v2::{client::InterceptedService, error::LedgerApiError};

/// Wrapped for [`svc_proto::PackageServiceClient`]
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
    pub async fn get_package(&self, package_id: PackageId) -> Result<Vec<u8>, LedgerApiError> {
        let request = GetPackageRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package(req)
                    .await
                    .map_err(LedgerApiError::from_status)
            })
            .await?;

        Ok(response.archive_payload)
    }

    /// Returns `true` if package status is `Registered`
    pub async fn package_registered(&self, package_id: PackageId) -> Result<bool, LedgerApiError> {
        let request = GetPackageStatusRequest {
            package_id: package_id.into(),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_package_status(req)
                    .await
                    .map_err(LedgerApiError::from_status)
            })
            .await?;

        // FIXME: replace unwrap with error
        let status = proto::PackageStatus::try_from(response.package_status).unwrap();

        Ok(status == proto::PackageStatus::Registered)
    }

    /// Returns the identifiers of all supported packages
    pub async fn list_packages(&self) -> Result<Vec<PackageId>, LedgerApiError> {
        let response = self
            .retry_handler
            .call(&self.service, &(), |mut svc, _| async move {
                svc.list_packages(ListPackagesRequest {})
                    .await
                    .map_err(LedgerApiError::from_status)
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
                    .map_err(LedgerApiError::value_error)
            })
            .collect::<Result<_, _>>()
    }
}
