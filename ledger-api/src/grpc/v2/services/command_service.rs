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
    error::{CantonError, ErrorCodeId},
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
    ///
    /// ## Notes on retry behavior
    ///
    /// - If the submission ID is set in the `commands`, it will be re-generated to random UUIDv7
    /// on retry to avoid duplication.
    /// - If the RPC call encountered `DUPLICATE_COMMAND` error, the function will attempt to
    /// determine, was it our submission or not. If it can see, that it was our submission within
    /// this function call, it will return `Ok(_)`. Otherwise original `DUPLICATE_COMMAND` error
    /// will be returned. Check relies on metadata fields `existing_submission_id` and
    /// `completion_offset` being set on error info. If they are missing, the function cannot
    /// determine the output and original error will be returned.
    pub async fn submit_and_wait(
        &mut self,
        commands: Commands,
    ) -> Result<UpdateIdAndOffset, CantonError> {
        // We use this list in case of DUPLICATE_COMMAND error to determine if it was our local
        // submission which succeeded or not
        let mut submission_ids_used: Vec<LedgerString> = Vec::new();

        let result = self
            .retry_handler
            .call_with_attempt(&self.service, &commands, |mut svc, mut cmds, attempt| {
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

                // store used submission ID
                if let Some(submission_id) = &cmds.submission_id {
                    submission_ids_used.push(submission_id.clone());
                }

                // TODO: do some logging here, including submission ID

                let request = SubmitAndWaitRequest {
                    commands: Some(cmds.into()),
                };

                // Since DUPLICATE_COMMAND errors are non-retryable, we can skip catching this error
                // here - it will be rejected by retry policy anyway
                // So in case of DUPLICATE_COMMAND we immediately return Err(_) here
                async move { svc.submit_and_wait(request).await }
            })
            .await;

        match result {
            Ok(response) => Ok(UpdateIdAndOffset {
                update_id: LedgerString::new(response.update_id)
                    .validated_of::<SubmitAndWaitResponse>("update_id")
                    .no_msg()
                    .map_err(CantonError::value_error)?,
                completion_offset: response.completion_offset,
            }),

            // This is the case, when we got DUPLICATE_COMMAND and the submission ID is known.
            // We also require completion offset to be defined, otherwise we can't construct output.
            Err(CantonError::Decoded(decoded))
                if matches!(decoded.error_code_id(), ErrorCodeId::DuplicateCommand)
                    && let Some(id) = decoded.existing_submission_id()
                    && submission_ids_used.contains(&id)
                    && let Some(offset) = decoded.completion_offset() =>
            {
                // The command was submitted previously by this function
                // So we can return Ok
                Ok(UpdateIdAndOffset {
                    update_id: id,
                    completion_offset: offset,
                })
            }

            // Other errors are returned
            Err(error) => Err(error),
        }
    }

    /// Submits a single composite command, waits for its result, and returns the transaction.
    /// Propagates the gRPC error of failed submissions including Daml interpretation errors.
    ///
    /// ## Notes on retry behavior
    ///
    /// - If the submission ID is set in the `commands`, it will be re-generated to random UUIDv7
    /// on retry to avoid duplication.
    /// - Unlike [`Self::submit_and_wait`], in case of `DUPLICATE_COMMAND` the error will be
    /// returned as it is. This is done because unlike `submit_and_wait`, this function cannot
    /// reconstruct the result (committed transaction). So the user has to catch this error and
    /// reconstruct the transaction himself.
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
