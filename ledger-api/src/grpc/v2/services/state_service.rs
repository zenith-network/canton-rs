use ledger_api_proto::com::daml::ledger::api::v2::{
    GetActiveContractsRequest, GetActiveContractsResponse, GetLedgerEndRequest,
    state_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::LedgerString,
    v2::{ContractEntry, EventFormat},
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};
use tokio_stream::{Stream, StreamExt as _};
use tonic::Status;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::StateServiceClient`]
#[derive(Clone, Debug)]
pub struct StateServiceClient {
    service: svc_proto::StateServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl StateServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::StateServiceClient<InterceptedService>,
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

    /// Get the current ledger end.
    ///
    /// Subscriptions started with the returned offset will serve events after this RPC was called.
    pub async fn get_ledger_end(&mut self) -> Result<i64, CantonError> {
        let request = GetLedgerEndRequest {};
        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_ledger_end(req).await
            })
            .await?;
        Ok(response.offset)
    }

    /// Returns a stream of the snapshot of the active contracts and incomplete (un)assignments at a
    /// ledger offset.
    ///
    /// Once the stream of `ActiveContractResponse` completes, the client SHOULD begin streaming
    /// updates from the update service, starting at the `active_at_offset` specified in this
    /// request. Clients SHOULD NOT assume that the set of active contracts they receive reflects
    /// the state at the ledger end.
    pub async fn get_active_contracts(
        &mut self,
        active_at_offset: i64,
        event_format: EventFormat,
        stream_continuation_token: Option<Vec<u8>>,
    ) -> Result<impl Stream<Item = Result<ActiveContractResponse, CantonError>>, CantonError> {
        let request = GetActiveContractsRequest {
            active_at_offset,
            event_format: Some(event_format.into()),
            stream_continuation_token,
        };

        let streaming = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_active_contracts(req).await
            })
            .await?;

        let converter = |result: Result<GetActiveContractsResponse, Status>| {
            result
                .map_err(CantonError::from)?
                .try_into()
                .map_err(CantonError::value_error)
        };

        Ok(streaming.map(converter))
    }
}

#[derive(Clone, Debug)]
pub struct ActiveContractResponse {
    pub workflow_id: LedgerString,
    pub stream_continuation_token: Option<Vec<u8>>,
    pub contract_entry: ContractEntry,
}

impl TryFrom<GetActiveContractsResponse> for ActiveContractResponse {
    type Error = ValueError;

    fn try_from(value: GetActiveContractsResponse) -> Result<Self, Self::Error> {
        Ok(ActiveContractResponse {
            workflow_id: LedgerString::new(value.workflow_id)
                .validated_of::<GetActiveContractsResponse>("workflow_id")
                .no_msg()?,
            stream_continuation_token: (!value.stream_continuation_token.is_empty())
                .then_some(value.stream_continuation_token),
            contract_entry: value
                .contract_entry
                .required_of::<GetActiveContractsResponse>("contract_entry")
                .no_msg()?
                .try_into()
                .validated_of::<GetActiveContractsResponse>("contract_entry")
                .no_msg()?,
        })
    }
}
