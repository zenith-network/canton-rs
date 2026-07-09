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

use crate::grpc::v2::{client::InterceptedService, error::CantonError};

/// Convenient wrapper for [`svc_proto::CommandServiceClient`]
pub struct CommandServiceClient {
    service: svc_proto::CommandServiceClient<InterceptedService>,
}

impl CommandServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn from_tonic(service: svc_proto::CommandServiceClient<InterceptedService>) -> Self {
        Self { service }
    }

    /// Submits a single composite command and waits for its result. Propagates the gRPC error of
    /// failed submissions including Daml interpretation errors.
    pub async fn submit_and_wait(
        &mut self,
        commands: Commands,
    ) -> Result<UpdateIdAndOffset, CantonError> {
        let response = self
            .service
            .submit_and_wait(SubmitAndWaitRequest {
                commands: Some(commands.into()),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner();
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
        self.service
            .submit_and_wait_for_transaction(SubmitAndWaitForTransactionRequest {
                commands: Some(commands.into()),
                transaction_format: format.map(Into::into),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner()
            .transaction
            .required_of::<SubmitAndWaitForTransactionResponse>("transaction")
            .no_msg()
            .map_err(CantonError::value_error)?
            .try_into()
            .map_err(Into::into)
    }

    pub async fn submit_and_wait_for_reassignment(&mut self) {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UpdateIdAndOffset {
    pub update_id: LedgerString,
    pub completion_offset: i64,
}
