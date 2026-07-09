use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetActiveContractsRequest, GetLedgerEndRequest,
    state_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::LedgerString,
    v2::{ContractEntry, EventFormat},
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};
use tokio_stream::{Stream, StreamExt as _};

use crate::grpc::v2::{client::InterceptedService, error::CantonError};

/// Wrapped for [`svc_proto::StateServiceClient`]
pub struct StateServiceClient {
    service: svc_proto::StateServiceClient<InterceptedService>,
}

impl StateServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn from_tonic(service: svc_proto::StateServiceClient<InterceptedService>) -> Self {
        Self { service }
    }

    pub async fn get_ledger_end(&mut self) -> Result<i64, CantonError> {
        Ok(self
            .service
            .get_ledger_end(GetLedgerEndRequest {})
            .await
            .map_err(CantonError::from)?
            .into_inner()
            .offset)
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
        Ok(self
            .service
            .get_active_contracts(GetActiveContractsRequest {
                active_at_offset,
                event_format: Some(event_format.into()),
                stream_continuation_token,
            })
            .await
            .map_err(CantonError::from)?
            .into_inner()
            .map(|result| {
                result
                    .map_err(CantonError::from)?
                    .try_into()
                    .map_err(CantonError::value_error)
            }))
    }
}

#[derive(Clone, Debug)]
pub struct ActiveContractResponse {
    pub workflow_id: LedgerString,
    pub stream_continuation_token: Option<Vec<u8>>,
    pub contract_entry: ContractEntry,
}

impl TryFrom<proto::GetActiveContractsResponse> for ActiveContractResponse {
    type Error = ValueError;

    fn try_from(value: proto::GetActiveContractsResponse) -> Result<Self, Self::Error> {
        Ok(ActiveContractResponse {
            workflow_id: LedgerString::new(value.workflow_id)
                .validated_of::<proto::GetActiveContractsResponse>("workflow_id")
                .no_msg()?,
            stream_continuation_token: (!value.stream_continuation_token.is_empty())
                .then_some(value.stream_continuation_token),
            contract_entry: value
                .contract_entry
                .required_of::<proto::GetActiveContractsResponse>("contract_entry")
                .no_msg()?
                .try_into()
                .validated_of::<proto::GetActiveContractsResponse>("contract_entry")
                .no_msg()?,
        })
    }
}
