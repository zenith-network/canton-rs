use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::ValueError;

use crate::v2::{
    AcsDeltaEvent, ArchivedEvent, CreatedEvent, Event, ExercisedEvent, LedgerEffectEvent,
};

/// Transaction shape type
///
/// Sealed trait
pub trait TxShape: private::TransactionShape + Into<proto::TransactionShape> {
    /// Type which defines event shape
    type Event: TryFrom<proto::Event, Error: Into<ValueError>>;
}

mod private {
    pub trait TransactionShape {}
    impl TransactionShape for super::AcsDelta {}
    impl TransactionShape for super::LedgerEffects {}
    impl TransactionShape for super::TransactionShape {}
}

/// Transaction shape that is sufficient to maintain an accurate ACS view.
///
/// The field `witness_parties` in events are populated as stakeholders, transaction filter will
/// apply accordingly. This translates to create and archive events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct AcsDelta;

impl From<AcsDelta> for proto::TransactionShape {
    fn from(_: AcsDelta) -> Self {
        Self::AcsDelta
    }
}

impl TxShape for AcsDelta {
    type Event = AcsDeltaEvent<CreatedEvent, ArchivedEvent>;
}

/// Transaction shape that allows maintaining an ACS and also conveys detailed information about
/// all exercises.
///
/// The field `witness_parties` in events are populated as cumulative informees, transaction filter
/// will apply accordingly. This translates to create, consuming exercise and non-consuming
/// exercise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct LedgerEffects;

impl From<LedgerEffects> for proto::TransactionShape {
    fn from(_: LedgerEffects) -> Self {
        Self::LedgerEffects
    }
}

impl TxShape for LedgerEffects {
    type Event = LedgerEffectEvent<CreatedEvent, ExercisedEvent>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum TransactionShape {
    /// Transaction shape that is sufficient to maintain an accurate ACS view.
    ///
    /// The field `witness_parties` in events are populated as stakeholders, transaction filter will
    /// apply accordingly. This translates to create and archive events.
    #[default]
    AcsDelta,

    /// Transaction shape that allows maintaining an ACS and also conveys detailed information about
    /// all exercises.
    ///
    /// The field `witness_parties` in events are populated as cumulative informees, transaction
    /// filter will apply accordingly. This translates to create, consuming exercise and
    /// non-consuming exercise.
    LedgerEffects,
}

impl From<TransactionShape> for proto::TransactionShape {
    fn from(value: TransactionShape) -> Self {
        match value {
            TransactionShape::AcsDelta => proto::TransactionShape::AcsDelta,
            TransactionShape::LedgerEffects => proto::TransactionShape::LedgerEffects,
        }
    }
}

impl TxShape for TransactionShape {
    type Event = Event<CreatedEvent, ArchivedEvent, ExercisedEvent>;
}
