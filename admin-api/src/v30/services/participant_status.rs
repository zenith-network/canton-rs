use admin_api_types::{
    error::ValueError,
    topology::v30::{
        ConnectedSynchronizer, Synchronizer,
        health::{NotInitialized, Status},
    },
};
use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::admin_api_proto::com::digitalasset::canton::admin::participant::v30::{
    self as proto, participant_status_service_client as svc_proto,
};
use canton_types::topology::ProtocolVersion;
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`ParticipantStatusServiceClient`][svc_proto::ParticipantStatusServiceClient].
#[derive(Clone, Debug)]
pub struct ParticipantStatusClient {
    service: svc_proto::ParticipantStatusServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl ParticipantStatusClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::ParticipantStatusServiceClient<InterceptedService>,
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

    /// Get participant status.
    pub async fn participant_status(&self) -> Result<ParticipantStatus, AdminApiError> {
        let request = proto::ParticipantStatusRequest::default();
        self.participant_status_inner(&request).await
    }

    /// Same as [`Self::participant_status`], but restricts the response to the given synchronizer:
    /// node-level components are always included, while connected synchronizers and
    /// synchronizer-labeled components are included only if they match.
    ///
    /// A logical synchronizer ID matches all its physical instances.
    pub async fn participant_status_for(
        &mut self,
        synchronizer: Synchronizer,
    ) -> Result<ParticipantStatus, AdminApiError> {
        let request = proto::ParticipantStatusRequest {
            synchronizer_id: Some(synchronizer.into()),
        };
        self.participant_status_inner(&request).await
    }

    async fn participant_status_inner(
        &self,
        request: &proto::ParticipantStatusRequest,
    ) -> Result<ParticipantStatus, AdminApiError> {
        self.retry_handler
            .call(&self.service, request, |mut svc, req| async move {
                svc.participant_status(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .try_into()
            .map_err(Into::into)
    }
}

/// Status of the participant
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ParticipantStatus {
    /// Participant is initialized
    Initialized(InitializedParticipantStatus),

    /// Participant is not initialized
    NotInitialized(NotInitialized),
}

impl TryFrom<proto::ParticipantStatusResponse> for ParticipantStatus {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantStatusResponse) -> Result<Self, Self::Error> {
        use proto::participant_status_response::Kind;
        match value
            .kind
            .required_of::<proto::ParticipantStatusResponse>("kind")?
        {
            Kind::Status(status) => status.try_into().map(Self::Initialized),
            Kind::NotInitialized(status) => status.try_into().map(Self::NotInitialized),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InitializedParticipantStatus {
    /// Common status data
    pub common_status: Status,

    /// Connected synchronizers
    pub connected_synchronizers: Vec<ConnectedSynchronizer>,

    /// Indicate if the participant node is active.
    ///
    /// `true` if the participant node is replicated and is the active replica, or `true` if not
    /// replicated.
    pub active: bool,

    /// Protocol versions which this node supports.
    ///
    /// Take into account config parameters such as alpha/beta version support and minimum protocol
    /// version.
    pub supported_protocol_versions: Vec<ProtocolVersion>,
}

impl TryFrom<proto::participant_status_response::ParticipantStatusResponseStatus>
    for InitializedParticipantStatus
{
    type Error = ValueError;

    fn try_from(
        value: proto::participant_status_response::ParticipantStatusResponseStatus,
    ) -> Result<Self, Self::Error> {
        use proto::participant_status_response::ParticipantStatusResponseStatus;

        let common_status = value
            .common_status
            .required_of::<ParticipantStatusResponseStatus>("common_status")?
            .try_into()
            .validated_of::<ParticipantStatusResponseStatus>("common_status")?;

        let connected_synchronizers = value
            .connected_synchronizers
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
            .validated_of::<ParticipantStatusResponseStatus>("connected_synchronizers")?;

        let supported_protocol_versions = value
            .supported_protocol_versions
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
            .validated_of::<ParticipantStatusResponseStatus>("supported_protocol_versions")?;

        Ok(Self {
            common_status,
            connected_synchronizers,
            active: value.active,
            supported_protocol_versions,
        })
    }
}
