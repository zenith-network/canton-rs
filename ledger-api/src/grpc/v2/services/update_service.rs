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

use crate::grpc::v2::{client::InterceptedService, error::CantonError};

// FIXME: add topology tx type
/// Update type of [`UpdateServiceClient::get_updates()`]
pub type StreamingUpdate<S> =
    Update<Transaction<<S as TxShape>::Event>, Reassignment, OffsetCheckpoint, Empty>;

// FIXME: add topology tx type
/// Update type for other methods
pub type SingleUpdate<S> = Update<Transaction<<S as TxShape>::Event>, Reassignment, Empty, Empty>;

/// Wrapped for [`svc_proto::UpdateServiceClient`]
pub struct UpdateServiceClient {
    service: svc_proto::UpdateServiceClient<InterceptedService>,
}

impl UpdateServiceClient {
    /// Create a wrapper from underlying tonic service client
    pub fn from_tonic(service: svc_proto::UpdateServiceClient<InterceptedService>) -> Self {
        Self { service }
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
        let streaming = self
            .service
            .get_updates(GetUpdatesRequest {
                begin_exclusive,
                end_inclusive,
                update_format: Some(update_format.into()),
                descending_order: false,
            })
            .await
            .map_err(CantonError::from)?
            .into_inner();

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
        self.service
            .get_update_by_id(GetUpdateByIdRequest {
                update_id: update_id.into(),
                update_format: Some(update_format.into()),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner()
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
        self.service
            .get_update_by_offset(GetUpdateByOffsetRequest {
                offset,
                update_format: Some(update_format.into()),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner()
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
        let response = self
            .service
            .get_updates_page(GetUpdatesPageRequest {
                begin_offset_exclusive: begin_exclusive,
                end_offset_inclusive: end_inclusive,
                max_page_size: max_page_size,
                update_format: Some(update_format.into()),
                descending_order: false,
                page_token: page_token.map(Into::into),
            })
            .await
            .map_err(CantonError::from)?
            .into_inner();
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

#[cfg(test)]
mod tests {
    // use super::*;

    // #[allow(dead_code)]
    // async fn compilation_test_1(mut client: UpdateServiceClient) {
    //     let update_format = UpdateFormat::new();

    //     let mut stream = client.get_updates(0, None, update_format).await.unwrap();

    //     while let Some(update) = stream.try_next().await.unwrap() {
    //         match update {
    //             Update::OffsetCheckpoint(checkpoint) => {
    //                 println!("Offset checkpoint: {}", checkpoint.offset);
    //             }
    //         }
    //     }
    // }

    // #[allow(dead_code)]
    // async fn compilation_test_2(mut client: UpdateServiceClient) {
    //     use ledger_api_types::v2::{
    //         AcsDelta,
    //         formats::static_::{EventFormat, TransactionFormat},
    //     };

    //     let event_format = EventFormat::new();
    //     let txformat = TransactionFormat::<AcsDelta>::new(event_format);
    //     let update_format = UpdateFormat::new().include_transactions(txformat);

    //     let mut stream = client.get_updates(0, None, update_format).await.unwrap();

    //     while let Some(update) = stream.try_next().await.unwrap() {
    //         match update {
    //             Update::Transaction(_) => todo!(),
    //             Update::Reassignment(_) => todo!(),
    //             Update::OffsetCheckpoint(_) => todo!(),
    //             Update::TopologyTransaction(_) => todo!(),
    //         }
    //     }
    // }
}
