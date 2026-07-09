//! This module provides some implementations of convertion traits.

use ledger_api_types::{
    canton_types::primitives::LedgerString,
    v2::{self as value, AcsDelta, LedgerEffects},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::com::daml::ledger::api::v2::{
    ArchivedEvent, Command, Commands, CreateAndExerciseCommand, CreateCommand, CreatedEvent,
    CumulativeFilter, Event, EventFormat, ExerciseByKeyCommand, ExerciseCommand, ExercisedEvent,
    Filters, InterfaceFilter, OffsetCheckpoint, Reassignment, TemplateFilter, TopologyTransaction,
    Transaction, TransactionFormat, TransactionShape, UpdateFormat, WildcardFilter, command,
    cumulative_filter::IdentifierFilter,
    get_update_response,
    get_updates_response::{self},
};

pub mod errors;

use errors::ValueError;

impl From<value::CreateCommand> for CreateCommand {
    fn from(value: value::CreateCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            create_arguments: Some(value.create_arguments.into()),
        }
    }
}

impl From<value::ExerciseCommand> for ExerciseCommand {
    fn from(value: value::ExerciseCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            contract_id: value.contract_id.into(),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

impl From<value::ExerciseByKeyCommand> for ExerciseByKeyCommand {
    fn from(value: value::ExerciseByKeyCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            contract_key: Some(value.contract_key.into()),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

impl From<value::CreateAndExerciseCommand> for CreateAndExerciseCommand {
    fn from(value: value::CreateAndExerciseCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            create_arguments: Some(value.create_arguments.into()),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

impl From<value::Command> for command::Command {
    fn from(value: value::Command) -> Self {
        match value {
            value::Command::Create(cmd) => Self::Create(cmd.into()),
            value::Command::Exercise(cmd) => Self::Exercise(cmd.into()),
            value::Command::ExerciseByKey(cmd) => Self::ExerciseByKey(cmd.into()),
            value::Command::CreateAndExercise(cmd) => Self::CreateAndExercise(cmd.into()),
        }
    }
}

impl From<value::Command> for Command {
    fn from(value: value::Command) -> Self {
        Self {
            command: Some(value.into()),
        }
    }
}

impl From<value::Commands> for Commands {
    fn from(value: value::Commands) -> Self {
        Self {
            workflow_id: value.workflow_id.map(Into::into).unwrap_or_default(),
            user_id: value.user_id.map(Into::into).unwrap_or_default(),
            command_id: value.command_id.into(),
            commands: value.commands.into_iter().map(Into::into).collect(),
            act_as: value.act_as.into_iter().map(Into::into).collect(),
            read_as: value.read_as.into_iter().map(Into::into).collect(),
            submission_id: value.submission_id.map(Into::into).unwrap_or_default(),
            taps_max_passes: value.taps_max_passes,
            ..Default::default()
        }
    }
}

impl From<value::AnyTransactionShape> for TransactionShape {
    fn from(value: value::AnyTransactionShape) -> Self {
        match value {
            value::AnyTransactionShape::AcsDelta => TransactionShape::AcsDelta,
            value::AnyTransactionShape::LedgerEffects => TransactionShape::LedgerEffects,
        }
    }
}

impl<S: value::TransactionShape> From<value::TransactionFormat<S>> for TransactionFormat {
    fn from(value: value::TransactionFormat<S>) -> Self {
        Self {
            event_format: Some(value.event_format.into()),
            transaction_shape: TransactionShape::from(S::to_any()).into(),
        }
    }
}

impl From<value::EventFormat> for EventFormat {
    fn from(value: value::EventFormat) -> Self {
        Self {
            filters_by_party: value
                .filters_by_party
                .into_iter()
                .map(|(k, v)| (String::from(k), v.into()))
                .collect(),
            filters_for_any_party: value.filters_for_any_party.map(Into::into),
            verbose: value.verbose,
        }
    }
}

impl<S, T, R, P> From<value::UpdateFormat<S, T, R, P>> for UpdateFormat
where
    S: value::TransactionShape,
    T: value::IncludeTransactions<S>,
    R: value::IncludeReassignments,
    P: value::IncludeTopologyEvents,
{
    fn from(value: value::UpdateFormat<S, T, R, P>) -> Self {
        Self {
            include_transactions: value.include_transactions.into().map(Into::into),
            include_reassignments: value.include_reassignments.into().map(Into::into),
            include_topology_events: None,
        }
    }
}

impl From<value::Filters> for Filters {
    fn from(value: value::Filters) -> Self {
        Self {
            cumulative: value.cumulative.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<value::CumulativeFilter> for CumulativeFilter {
    fn from(value: value::CumulativeFilter) -> Self {
        use IdentifierFilter::*;
        Self {
            identifier_filter: Some(match value {
                value::CumulativeFilter::Wildcard(f) => WildcardFilter(f.into()),
                value::CumulativeFilter::Interface(f) => InterfaceFilter(f.into()),
                value::CumulativeFilter::Template(f) => TemplateFilter(f.into()),
            }),
        }
    }
}

impl From<value::WildcardFilter> for WildcardFilter {
    fn from(value: value::WildcardFilter) -> Self {
        Self {
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

impl From<value::InterfaceFilter> for InterfaceFilter {
    fn from(value: value::InterfaceFilter) -> Self {
        Self {
            interface_id: Some(value.interface_id.into()),
            include_interface_view: value.include_interface_view,
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

impl From<value::TemplateFilter> for TemplateFilter {
    fn from(value: value::TemplateFilter) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            include_created_event_blob: value.include_created_event_blob,
        }
    }
}

impl TryFrom<CreatedEvent> for value::CreatedEvent {
    type Error = ValueError;

    fn try_from(value: CreatedEvent) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<ArchivedEvent> for value::ArchivedEvent {
    type Error = ValueError;

    fn try_from(value: ArchivedEvent) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<ExercisedEvent> for value::ExercisedEvent {
    type Error = ValueError;

    fn try_from(value: ExercisedEvent) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<Event> for value::AcsDeltaEvent {
    type Error = ValueError;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        use crate::com::daml::ledger::api::v2::event::Event::*;
        let event = value.event.required_of::<Event>("event")?;
        match event {
            Created(event) => Ok(Self::Created(event.try_into()?)),
            Archived(event) => Ok(Self::Archived(event.try_into()?)),
            Exercised(_) => Err(ValueError::UnexpectedEvent {
                expected: "Created | Archived",
                got: "Exercised",
            }),
        }
    }
}

impl TryFrom<Event> for value::LedgerEffectsEvent {
    type Error = ValueError;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        use crate::com::daml::ledger::api::v2::event::Event::*;
        let event = value.event.required_of::<Event>("event")?;
        match event {
            Created(event) => Ok(Self::Created(event.try_into()?)),
            Archived(_) => Err(ValueError::UnexpectedEvent {
                expected: "Created | Exercised",
                got: "Archived",
            }),
            Exercised(event) => Ok(Self::Exercised(event.try_into()?)),
        }
    }
}

impl<S, E> TryFrom<Transaction> for value::Transaction<S>
where
    S: value::TransactionShape<Event = E>,
    E: TryFrom<Event, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: Transaction) -> Result<Self, Self::Error> {
        let update_id =
            LedgerString::new(value.update_id).validated_of::<Transaction>("update_id")?;

        let command_id = if value.command_id.is_empty() {
            None
        } else {
            Some(LedgerString::new(value.command_id).validated_of::<Transaction>("command_id")?)
        };

        let workflow_id = if value.workflow_id.is_empty() {
            None
        } else {
            Some(LedgerString::new(value.workflow_id).validated_of::<Transaction>("workflow_id")?)
        };

        let effective_at = value
            .effective_at
            .required_of::<Transaction>("effective_at")?
            .try_into()?;

        let record_time = value
            .record_time
            .required_of::<Transaction>("record_time")?
            .try_into()?;

        let events = value
            .events
            .into_iter()
            .map(|event| S::Event::try_from(event))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            update_id,
            command_id,
            workflow_id,
            effective_at,
            events,
            offset: value.offset,
            record_time,
            paid_traffic_cost: value.paid_traffic_cost,
        })
    }
}

impl TryFrom<Reassignment> for value::Reassignment {
    type Error = ValueError;

    fn try_from(value: Reassignment) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<OffsetCheckpoint> for value::OffsetCheckpoint {
    type Error = ValueError;

    fn try_from(value: OffsetCheckpoint) -> Result<Self, Self::Error> {
        Ok(Self {
            offset: value.offset,
        })
    }
}

impl TryFrom<Transaction> for value::Empty {
    type Error = ValueError;

    fn try_from(_: Transaction) -> Result<Self, Self::Error> {
        // TODO: Always error
        todo!()
    }
}

impl TryFrom<TopologyTransaction> for value::Empty {
    type Error = ValueError;

    fn try_from(_: TopologyTransaction) -> Result<Self, Self::Error> {
        // TODO: always error
        todo!()
    }
}

impl TryFrom<Reassignment> for value::Empty {
    type Error = ValueError;

    fn try_from(_: Reassignment) -> Result<Self, Self::Error> {
        // TODO: always error
        todo!()
    }
}

impl<T, R, P> TryFrom<get_updates_response::Update>
    for value::Update<T, R, value::OffsetCheckpoint, P>
where
    T: TryFrom<Transaction, Error = ValueError>,
    R: TryFrom<Reassignment, Error = ValueError>,
    P: TryFrom<TopologyTransaction, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: get_updates_response::Update) -> Result<Self, Self::Error> {
        Ok(match value {
            get_updates_response::Update::Transaction(transaction) => {
                Self::Transaction(transaction.try_into()?)
            }
            get_updates_response::Update::Reassignment(reassignment) => {
                Self::Reassignment(reassignment.try_into()?)
            }
            get_updates_response::Update::OffsetCheckpoint(offset_checkpoint) => {
                Self::OffsetCheckpoint(offset_checkpoint.try_into()?)
            }
            get_updates_response::Update::TopologyTransaction(topology_transaction) => {
                Self::TopologyTransaction(topology_transaction.try_into()?)
            }
        })
    }
}

impl<T, R, P> TryFrom<get_update_response::Update> for value::Update<T, R, value::Empty, P> {
    type Error = ValueError;

    fn try_from(value: get_update_response::Update) -> Result<Self, Self::Error> {
        todo!()
    }
}
