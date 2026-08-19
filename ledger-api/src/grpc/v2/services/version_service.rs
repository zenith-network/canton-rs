use ledger_api_proto::com::daml::ledger::api::v2::{
    GetLedgerApiVersionRequest, GetLedgerApiVersionResponse, version_service_client as svc_proto,
};
use ledger_api_types::{
    v2::FeaturesDescriptor,
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::VersionServiceClient`]
#[derive(Clone, Debug)]
pub struct VersionServiceClient {
    service: svc_proto::VersionServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl VersionServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::VersionServiceClient<InterceptedService>,
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

    /// Read the Ledger API version
    pub async fn get_ledger_api_version(&mut self) -> Result<ApiVersion, CantonError> {
        let response = self
            .retry_handler
            .call(&self.service, &(), |mut svc, _| async move {
                svc.get_ledger_api_version(GetLedgerApiVersionRequest {})
                    .await
            })
            .await?;

        response.try_into().map_err(CantonError::value_error)
    }
}

#[derive(Clone, Debug)]
pub struct ApiVersion {
    pub version: String,
    pub features: FeaturesDescriptor,
}

impl TryFrom<GetLedgerApiVersionResponse> for ApiVersion {
    type Error = ValueError;

    fn try_from(value: GetLedgerApiVersionResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            version: value.version,
            features: value
                .features
                .required_of::<GetLedgerApiVersionResponse>("features")
                .no_msg()?
                .try_into()
                .validated_of::<GetLedgerApiVersionResponse>("features")
                .no_msg()?,
        })
    }
}
