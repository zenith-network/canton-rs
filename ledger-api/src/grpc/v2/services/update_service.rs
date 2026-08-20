use ledger_api_proto::com::daml::ledger::api::v2::{
    GetUpdateByIdRequest, GetUpdateByOffsetRequest, GetUpdateResponse, GetUpdatesPageRequest,
    GetUpdatesRequest, GetUpdatesResponse, update_service_client as svc_proto,
};
use ledger_api_types::{
    canton_types::LedgerString,
    v2::{
        Empty, OffsetCheckpoint, Page, PageToken, Reassignment, Transaction, TxShape, Update,
        UpdateFormat,
    },
    value::v2::errors::IntoValueError,
};
use protobuf_utils::RequiredProtoField as _;
use tokio_stream::{Stream, StreamExt as _};
use tonic::Status;

use crate::grpc::v2::{
    client::InterceptedService,
    error::CantonError,
    retry::{RetryConfig, RetryHandler},
};

// FIXME: add topology tx type
/// Update type of [`UpdateServiceClient::get_updates()`]
pub type StreamingUpdate<S> =
    Update<Transaction<<S as TxShape>::Event>, Reassignment, OffsetCheckpoint, Empty>;

// FIXME: add topology tx type
/// Update type for other methods
pub type SingleUpdate<S> = Update<Transaction<<S as TxShape>::Event>, Reassignment, Empty, Empty>;

/// Wrapped for [`svc_proto::UpdateServiceClient`]
#[derive(Clone, Debug)]
pub struct UpdateServiceClient {
    service: svc_proto::UpdateServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl UpdateServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::UpdateServiceClient<InterceptedService>,
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

    /// Read the ledger's filtered update stream for the specified contents and filters.
    /// It returns the event types in accordance with the stream contents selected. Also the
    /// selection criteria for individual events depends on the transaction shape chosen.
    ///
    /// - ACS delta: a requesting party must be a stakeholder of an event for it to be included.
    /// - ledger effects: a requesting party must be a witness of an event for it to be included.
    pub async fn get_updates<S: TxShape>(
        &mut self,
        begin_exclusive: i64,
        end_inclusive: Option<i64>,
        update_format: UpdateFormat<S>,
    ) -> Result<impl Stream<Item = Result<StreamingUpdate<S>, CantonError>>, CantonError> {
        let request = GetUpdatesRequest {
            begin_exclusive,
            end_inclusive,
            update_format: Some(update_format.into()),
            descending_order: false,
        };

        let streaming = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_updates(req).await
            })
            .await?;
        // we only retry on the initial request
        // getting items from the stream cannot be retries, the user has to re-create the stream

        let converter = |result: Result<GetUpdatesResponse, Status>| -> Result<
            StreamingUpdate<S>,
            CantonError,
        > {
            result
                .map_err(CantonError::from)?
                .update
                .required_of::<GetUpdatesResponse>("update")
                .with_msg("updates stream yielded bad item")
                .map_err(CantonError::value_error)?
                .try_into()
                .map_err(CantonError::value_error)
        };

        Ok(streaming.map(converter))
    }

    /// Lookup an update by its ID.
    ///
    /// If there is no update with this ID, or all the events are filtered, an `UPDATE_NOT_FOUND`
    /// error will be raised.
    pub async fn get_update_by_id<S: TxShape>(
        &mut self,
        update_id: LedgerString,
        update_format: UpdateFormat<S>,
    ) -> Result<SingleUpdate<S>, CantonError> {
        let request = GetUpdateByIdRequest {
            update_id: update_id.into(),
            update_format: Some(update_format.into()),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_update_by_id(req).await
            })
            .await?;

        response
            .update
            .required_of::<GetUpdateResponse>("update")
            .no_msg()
            .map_err(CantonError::value_error)?
            .try_into()
            .no_msg()
            .map_err(CantonError::value_error)
    }

    /// Lookup an update by its offset.
    ///
    /// If there is no update with this offset, or all the events are filtered, an
    /// `UPDATE_NOT_FOUND` error will be raised.
    pub async fn get_update_by_offset<S: TxShape>(
        &mut self,
        offset: i64,
        update_format: UpdateFormat<S>,
    ) -> Result<SingleUpdate<S>, CantonError> {
        let request = GetUpdateByOffsetRequest {
            offset,
            update_format: Some(update_format.into()),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_update_by_offset(req).await
            })
            .await?;

        response
            .update
            .required_of::<GetUpdateResponse>("update")
            .no_msg()
            .map_err(CantonError::value_error)?
            .try_into()
            .map_err(CantonError::value_error)
    }

    /// Read a page of ledger's filtered updates.
    ///
    /// It returns the event types in accordance with the specified contents and filters.
    /// Additionally, the selection criteria for individual events depends on the transaction shape
    /// chosen.
    ///
    /// - ACS delta: an event is included only if the requesting party is a stakeholder.
    /// - ledger effects: an event is included if the requesting party is a witness.
    pub async fn get_updates_page<S: TxShape>(
        &mut self,
        begin_exclusive: Option<i64>,
        end_inclusive: Option<i64>,
        max_page_size: Option<i32>,
        update_format: UpdateFormat<S>,
        page_token: Option<PageToken>,
    ) -> Result<Page<SingleUpdate<S>>, CantonError> {
        let request = GetUpdatesPageRequest {
            begin_offset_exclusive: begin_exclusive,
            end_offset_inclusive: end_inclusive,
            max_page_size,
            update_format: Some(update_format.into()),
            descending_order: false,
            page_token: page_token.map(Into::into),
        };

        let response = self
            .retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.get_updates_page(req).await
            })
            .await?;

        Ok(Page {
            items: response
                .updates
                .into_iter()
                .map(|response| {
                    response
                        .update
                        .required_of::<GetUpdateResponse>("update")
                        .no_msg()
                        .map_err(CantonError::value_error)?
                        .try_into()
                        .map_err(CantonError::value_error)
                })
                .collect::<Result<_, _>>()?,
            lowest_page_offset_exclusive: response.lowest_page_offset_exclusive,
            highest_page_offset_inclusive: response.highest_page_offset_inclusive,
            next_page_token: response.next_page_token.map(|inner| PageToken::new(inner)),
        })
    }
}
