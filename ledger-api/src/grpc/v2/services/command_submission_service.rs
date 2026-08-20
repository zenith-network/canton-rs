use ledger_api_proto::com::daml::ledger::api::v2::{
    SubmitRequest, command_submission_service_client as svc_proto,
};
use ledger_api_types::v2::Commands;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::CommandSubmissionServiceClient`]
///
/// Allows clients to attempt advancing the ledger's state by submitting commands.
/// The final states of their submissions are disclosed by the Command Completion Service.
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
pub struct CommandSubmissionServiceClient {
    service: svc_proto::CommandSubmissionServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl CommandSubmissionServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::CommandSubmissionServiceClient<InterceptedService>,
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

    /// Submit a single composite command
    pub async fn submit(&mut self, commands: Commands) -> Result<(), CantonError> {
        let request = SubmitRequest {
            commands: Some(commands.into()),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.submit(req).await
            })
            .await?;

        Ok(())
    }
}
