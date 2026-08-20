use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, CompletionStreamRequest, CompletionStreamResponse,
    command_completion_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::{NonEmpty, PartyId, UserId},
    v2::{Completion, OffsetCheckpoint},
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::RequiredProtoField as _;
use tokio_stream::{Stream, StreamExt as _};
use tonic::Status;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::CommandCompletionServiceClient`]
///
/// Allows clients to observe the status of their submissions.
/// Commands may be submitted via the Command Submission Service.
/// The on-ledger effects of their submissions are disclosed by the Update Service.
///
/// Commands may fail in 2 distinct manners:
///
/// 1. Failure communicated synchronously in the gRPC error of the submission.
/// 2. Failure communicated asynchronously in a Completion.
///
/// Note that not only successfully submitted commands MAY produce a completion event. For example,
/// the participant MAY choose to produce a completion event for a rejection of a duplicate command.
///
/// Clients that do not receive a successful completion about their submission MUST NOT assume that
/// it was successful.
/// Clients SHOULD subscribe to the CompletionStream before starting to submit commands to prevent
/// race conditions.
#[derive(Clone, Debug)]
pub struct CommandCompletionServiceClient {
    service: svc_proto::CommandCompletionServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl CommandCompletionServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::CommandCompletionServiceClient<InterceptedService>,
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

    /// Subscribe to command completion events
    ///
    /// # Parameters
    ///
    /// - `user_id` - Only completions of commands submitted with the same user_id will be visible
    ///     in the stream. Required unless authentication is used with a user token. In that case,
    ///     the token's user-id will be used for the request's user_id.
    /// - `parties` - Non-empty list of parties whose data should be included. The stream shows only
    ///     completions of commands for which at least one of the `act_as` parties is in the given
    ///     set of parties.
    /// - `begin_exclusive` - This field indicates the minimum offset for completions. This can be
    ///     used to resume an earlier completion stream. It must be a valid absolute offset
    ///     (positive integer) or zero (ledger begin offset). If the ledger has been pruned, this
    ///     parameter must be specified and greater than the pruning offset.
    pub async fn completion_stream(
        &mut self,
        user_id: Option<UserId>,
        parties: NonEmpty<PartyId>,
        begin_exclusive: i64,
    ) -> Result<impl Stream<Item = Result<CompletionResponse, CantonError>>, CantonError> {
        let request = CompletionStreamRequest {
            user_id: user_id.map(Into::into).unwrap_or_default(),
            parties: parties.into_iter().map(Into::into).collect(),
            begin_exclusive,
        };

        let streaming = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.completion_stream(req).await
            })
            .await?;

        // we don't retry on item erros, user is supposed to re-create the stream in that case
        let convertor =
            |response: Result<CompletionStreamResponse, Status>| -> Result<CompletionResponse, CantonError> {
                response
                    .map_err(CantonError::from)?
                    .completion_response
                    .required_of::<CompletionStreamResponse>("completion_response")
                    .with_msg("completion stream yielded bad item")
                    .map_err(CantonError::value_error)?
                    .try_into()
                    .map_err(CantonError::value_error)
            };

        Ok(streaming.map(convertor))
    }
}

#[derive(Clone, Debug)]
pub enum CompletionResponse {
    Completion(Completion),
    OffsetCheckpoint(OffsetCheckpoint),
}

impl TryFrom<proto::completion_stream_response::CompletionResponse> for CompletionResponse {
    type Error = ValueError;

    fn try_from(
        value: proto::completion_stream_response::CompletionResponse,
    ) -> Result<Self, Self::Error> {
        use proto::completion_stream_response::CompletionResponse::*;
        match value {
            Completion(completion) => completion.try_into().map(Self::Completion),
            OffsetCheckpoint(checkpoint) => checkpoint.try_into().map(Self::OffsetCheckpoint),
        }
    }
}
