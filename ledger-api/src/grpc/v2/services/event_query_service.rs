use ledger_api_proto::com::daml::ledger::api::v2::{
    self as proto, GetEventsByContractIdRequest, event_query_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::ContractId,
    v2::{
        Archived, ArchivedEvent, Created, CreatedEvent, CreatedWithKey, EventFormat, TemplateValue,
        TemplateValueWithKey,
    },
    value::v2::errors::{IntoValueError as _, ValueError},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

/// Wrapped for [`svc_proto::CommandCompletionServiceClient`]
///
/// Query events by contract ID.
#[derive(Clone, Debug)]
pub struct EventQueryServiceClient {
    service: svc_proto::EventQueryServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl EventQueryServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::EventQueryServiceClient<InterceptedService>,
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

    /// Get the create and the consuming exercise event for the contract with the provided ID.
    ///
    /// No events will be returned for contracts that have been pruned because they have already
    /// been archived before the latest pruning offset. If the contract cannot be found for the
    /// request, or all the contract-events are filtered, a `CONTRACT_EVENTS_NOT_FOUND` error will
    /// be raised.
    pub async fn get_events_by_contract_id<T: TemplateValue>(
        &mut self,
        contract_id: ContractId<T>,
        event_format: EventFormat,
    ) -> Result<CreatedAndArchived<Created<T>, Archived<T>>, CantonError> {
        let event = self
            .get_events_by_contract_id_any(contract_id.into_any(), event_format)
            .await?;

        let created = event
            .created
            .map(CreatedEvent::cast)
            .transpose()
            .map_err(|err| ValueError::raw_message_owned(format!("{err}"))) // FIXME: this is hacky
            .map_err(CantonError::value_error)?;

        let archived = event
            .archived
            .map(ArchivedEvent::cast)
            .transpose()
            .map_err(|err| ValueError::raw_message_owned(format!("{err}"))) // FIXME: this is hacky
            .map_err(CantonError::value_error)?;

        Ok(CreatedAndArchived { created, archived })
    }

    /// See [`Self::get_events_by_contract_id`]
    ///
    /// This method is the same but additionally handles keyed templates
    pub async fn get_events_by_contract_id_keyed<T: TemplateValueWithKey>(
        &mut self,
        contract_id: ContractId<T>,
        event_format: EventFormat,
    ) -> Result<CreatedAndArchived<CreatedWithKey<T>, Archived<T>>, CantonError> {
        let event = self
            .get_events_by_contract_id_any(contract_id.into_any(), event_format)
            .await?;

        let created = event
            .created
            .map(CreatedEvent::cast_keyed)
            .transpose()
            .map_err(|err| ValueError::raw_message_owned(format!("{err}"))) // FIXME: this is hacky
            .map_err(CantonError::value_error)?;

        let archived = event
            .archived
            .map(ArchivedEvent::cast)
            .transpose()
            .map_err(|err| ValueError::raw_message_owned(format!("{err}"))) // FIXME: this is hacky
            .map_err(CantonError::value_error)?;

        Ok(CreatedAndArchived { created, archived })
    }

    /// See [`Self::get_events_by_contract_id`]
    ///
    /// This method is the same, but returns non-casted event types
    pub async fn get_events_by_contract_id_any(
        &mut self,
        contract_id: ContractId,
        event_format: EventFormat,
    ) -> Result<CreatedAndArchived<CreatedEvent, ArchivedEvent>, CantonError> {
        let request = GetEventsByContractIdRequest {
            contract_id: contract_id.into(),
            event_format: Some(event_format.into()),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_events_by_contract_id(req).await
            })
            .await?;

        let created = response
            .created
            .map(|created| {
                created
                    .created_event
                    .required_of::<proto::Created>("created_event")
                    .no_msg()
                    .map_err(CantonError::value_error)?
                    .try_into()
                    .validated_of::<proto::Created>("created_event")
                    .no_msg()
                    .map_err(CantonError::value_error)
            })
            .transpose()?;

        let archived = response
            .archived
            .map(|archived| {
                archived
                    .archived_event
                    .required_of::<proto::Archived>("archived_event")
                    .no_msg()
                    .map_err(CantonError::value_error)?
                    .try_into()
                    .validated_of::<proto::Archived>("archived_event")
                    .no_msg()
                    .map_err(CantonError::value_error)
            })
            .transpose()?;

        Ok(CreatedAndArchived { created, archived })
    }
}

// TODO: this return type drops synchronizer ID from created and archived, need to deal with it

#[derive(Clone, Debug)]
pub struct CreatedAndArchived<C, A> {
    /// The create event for the contract with the `contract_id` given in the request provided it
    /// exists and has not yet been pruned.
    pub created: Option<C>,

    /// The archive event for the contract with the `contract_id` given in the request provided such
    /// an archive event exists and it has not yet been pruned.
    pub archived: Option<A>,
}
