use std::time::{Duration, SystemTime};

use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as proto, identity_initialization_service_client as svc_proto,
};
use canton_protocol::error::ProtoDecodeError;
use canton_types::topology::UniqueIdentifier;
use protobuf_utils::InvalidProtoField as _;

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`IdentityInitializationServiceClient`][svc_proto::IdentityInitializationServiceClient].
#[derive(Clone, Debug)]
pub struct IdentityInitializationClient {
    service: svc_proto::IdentityInitializationServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl IdentityInitializationClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::IdentityInitializationServiceClient<InterceptedService>,
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

    /// Returns the ID of the node (or `None` if not initialized).
    pub async fn get_id(&self) -> Result<Option<UniqueIdentifier>, AdminApiError> {
        let request = proto::GetIdRequest::default();
        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_id(req).await.map_err(AdminApiError::from_status)
            })
            .await?;

        if !response.initialized {
            return Ok(None);
        }

        UniqueIdentifier::try_from(response.unique_identifier)
            .validated_of::<proto::GetIdResponse>("unique_identifier")
            .map(Some)
            .map_err(AdminApiError::value_error)
    }

    /// Returns the current time of the node (used for testing with static time).
    pub async fn current_time(&self) -> Result<SystemTime, AdminApiError> {
        let request = proto::CurrentTimeRequest::default();
        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.current_time(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?;

        // `current_time` is a number of micro-seconds since epoch
        let micros = response.current_time;

        let duration = Duration::from_micros(micros.unsigned_abs());
        // Technically current_time can be negative
        let time = if micros < 0 {
            SystemTime::UNIX_EPOCH.checked_sub(duration)
        } else {
            SystemTime::UNIX_EPOCH.checked_add(duration)
        };

        time.ok_or_else(|| ProtoDecodeError::other("timestamp is out of range for SystemTime"))
            .validated_of::<proto::CurrentTimeResponse>("current_time")
            .map_err(AdminApiError::value_error)
    }
}
