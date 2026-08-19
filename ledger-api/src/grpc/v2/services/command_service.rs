use ledger_api_proto::com::daml::ledger::api::v2::{
    SubmitAndWaitForTransactionRequest, SubmitAndWaitForTransactionResponse, SubmitAndWaitRequest,
    SubmitAndWaitResponse, command_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::LedgerString,
    v2::{Commands, Transaction, TransactionFormat, TxShape},
    value::v2::errors::IntoValueError as _,
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Convenient wrapper for [`svc_proto::CommandServiceClient`]
#[derive(Clone, Debug)]
pub struct CommandServiceClient {
    service: svc_proto::CommandServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl CommandServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::CommandServiceClient<InterceptedService>,
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

    /// Submits a single composite command and waits for its result. Propagates the gRPC error of
    /// failed submissions including Daml interpretation errors.
    pub async fn submit_and_wait(
        &mut self,
        commands: Commands,
    ) -> Result<UpdateIdAndOffset, CantonError> {
        let response = self
            .retry_handler
            .call_with_attempt(
                &self.service,
                &commands,
                |mut svc, mut cmds, attempt| async move {
                    // we want to avoid retries with the same submission ID
                    // that's why we check if it's set and modify it on retries
                    if cmds.submission_id.is_some() {
                        if attempt == 0 {
                            // leave it as it is on the first attempt
                        } else {
                            // re-generate it to avoid sending the same submission ID
                            cmds.with_random_submission_id();
                        }
                    }
                    // if submission ID is not set, leave it empty for all attempts

                    let request = SubmitAndWaitRequest {
                        commands: Some(cmds.into()),
                    };

                    svc.submit_and_wait(request).await
                },
            )
            .await?;

        Ok(UpdateIdAndOffset {
            update_id: LedgerString::new(response.update_id)
                .validated_of::<SubmitAndWaitResponse>("update_id")
                .no_msg()
                .map_err(CantonError::value_error)?,
            completion_offset: response.completion_offset,
        })
    }

    /// Submits a single composite command, waits for its result, and returns the transaction.
    /// Propagates the gRPC error of failed submissions including Daml interpretation errors.
    pub async fn submit_and_wait_for_transaction<S: TxShape>(
        &mut self,
        commands: Commands,
        format: Option<TransactionFormat<S>>,
    ) -> Result<Transaction<S::Event>, CantonError> {
        let transaction_format = format.map(Into::into);

        let response = self
            .retry_handler
            .call_with_attempt(
                &self.service,
                &(commands, transaction_format),
                |mut svc, (mut cmds, txformat), attempt| async move {
                    // we want to avoid retries with the same submission ID
                    // that's why we check if it's set and modify it on retries
                    if cmds.submission_id.is_some() {
                        if attempt == 0 {
                            // leave it as it is on the first attempt
                        } else {
                            // re-generate it to avoid sending the same submission ID
                            cmds.with_random_submission_id();
                        }
                    }
                    // if submission ID is not set, leave it empty for all attempts

                    let request = SubmitAndWaitForTransactionRequest {
                        commands: Some(cmds.into()),
                        transaction_format: txformat,
                    };
                    svc.submit_and_wait_for_transaction(request).await
                },
            )
            .await?;

        response
            .transaction
            .required_of::<SubmitAndWaitForTransactionResponse>("transaction")
            .no_msg()
            .map_err(CantonError::value_error)?
            .try_into()
            .map_err(Into::into)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UpdateIdAndOffset {
    pub update_id: LedgerString,
    pub completion_offset: i64,
}
