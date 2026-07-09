use ledger_api_proto::com::daml::ledger::api::v2 as proto;

use crate::v2::{AcsDelta, EventFormat, LedgerEffects, TransactionShape, TxShape};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionFormat<S: TxShape = TransactionShape> {
    pub event_format: EventFormat,
    pub transaction_shape: S,
}

impl TransactionFormat<TransactionShape> {
    pub fn new(event_format: EventFormat, transaction_shape: TransactionShape) -> Self {
        Self {
            event_format,
            transaction_shape,
        }
    }
}

impl TransactionFormat<AcsDelta> {
    pub fn new(event_format: EventFormat) -> Self {
        Self {
            event_format,
            transaction_shape: AcsDelta,
        }
    }
}

impl TransactionFormat<LedgerEffects> {
    pub fn new(event_format: EventFormat) -> Self {
        Self {
            event_format,
            transaction_shape: LedgerEffects,
        }
    }
}

impl<S: TxShape> From<TransactionFormat<S>> for proto::TransactionFormat {
    fn from(value: TransactionFormat<S>) -> Self {
        let transaction_shape: proto::TransactionShape = value.transaction_shape.into();
        Self {
            event_format: Some(value.event_format.into()),
            transaction_shape: transaction_shape.into(),
        }
    }
}
