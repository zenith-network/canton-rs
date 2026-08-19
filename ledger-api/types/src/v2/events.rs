use std::time::SystemTime;

use canton_types::{ContractId, Name, PackageId, PackageName, PartyId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::{
    Identifier, TryFromValue,
    errors::{IntoValueError as _, ValueError},
    value::{Record, Value},
};
use nonempty::NonEmpty;
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v2::{ChoiceValue, Empty, TemplateValue, TemplateValueWithKey};

/// Generic event type
#[derive(Clone, Debug)]
pub enum Event<C = Empty, A = Empty, E = Empty> {
    Created(C),
    Archived(A),
    Exercised(E),
}

/// ACS delta event type
pub type AcsDeltaEvent<C, A> = Event<C, A, Empty>;

/// Ledger effects event type
pub type LedgerEffectEvent<C, E> = Event<C, Empty, E>;

impl<C, A, E> TryFrom<proto::Event> for Event<C, A, E>
where
    C: TryFrom<proto::CreatedEvent, Error = ValueError>,
    A: TryFrom<proto::ArchivedEvent, Error = ValueError>,
    E: TryFrom<proto::ExercisedEvent, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: proto::Event) -> Result<Self, Self::Error> {
        use proto::event::Event::*;

        let event = value.event.required_of::<proto::Event>("event").no_msg()?;
        Ok(match event {
            Created(event) => Self::Created(event.try_into()?),
            Archived(event) => Self::Archived(event.try_into()?),
            Exercised(event) => Self::Exercised(event.try_into()?),
        })
    }
}

// TODO: implement this error
#[derive(Clone, Debug, thiserror::Error)]
#[error("failed to cast event")]
pub struct CastError {}

/// `Created` event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created<T: TemplateValue> {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId<T>,
    pub create_arguments: T,
    pub created_event_blob: Vec<u8>,
    pub witness_parties: NonEmpty<PartyId>,
    pub signatories: NonEmpty<PartyId>,
    pub observers: Vec<PartyId>,
    pub created_at: SystemTime,
    pub acs_delta: bool,
}

/// `Created` event for a template with key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedWithKey<T: TemplateValueWithKey> {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId<T>,
    pub contract_key: T::Key,
    pub contract_key_hash: Vec<u8>,
    pub create_arguments: T,
    pub created_event_blob: Vec<u8>,
    pub witness_parties: NonEmpty<PartyId>,
    pub signatories: NonEmpty<PartyId>,
    pub observers: Vec<PartyId>,
    pub created_at: SystemTime,
    pub acs_delta: bool,
}

/// `Created` event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedEvent {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId,
    pub template_id: Identifier<PackageId>,
    pub contract_key: Option<Value>,
    pub contract_key_hash: Vec<u8>,
    pub create_arguments: Record,
    pub created_event_blob: Vec<u8>,
    // pub inteface_views: Vec<InterfaceView>,
    pub witness_parties: NonEmpty<PartyId>,
    pub signatories: NonEmpty<PartyId>,
    pub observers: Vec<PartyId>,
    pub created_at: SystemTime,
    pub package_name: PackageName,
    pub acs_delta: bool,
    // TODO: implement missing fields
}

impl CreatedEvent {
    /// Cast to typed event
    pub fn cast<T: TemplateValue>(self) -> Result<Created<T>, CastError> {
        let expected_id = T::identifier_with_package_id();
        let expected_package_name = T::package_name();

        if expected_id == self.template_id {
            return Err(CastError {});
        }
        if expected_package_name != self.package_name {
            return Err(CastError {});
        }
        let create_arguments = match T::try_from_record(self.create_arguments) {
            Ok(value) => value,
            Err(_) => return Err(CastError {}),
        };

        let contract_id = self.contract_id.into_typed();

        Ok(Created {
            offset: self.offset,
            node_id: self.node_id,
            contract_id,
            create_arguments,
            created_event_blob: self.created_event_blob,
            witness_parties: self.witness_parties,
            signatories: self.signatories,
            observers: self.observers,
            created_at: self.created_at,
            acs_delta: self.acs_delta,
        })
    }

    pub fn cast_keyed<T: TemplateValueWithKey>(self) -> Result<CreatedWithKey<T>, CastError> {
        let expected_id = T::identifier_with_package_id();
        let expected_package_name = T::package_name();

        if expected_id == self.template_id {
            return Err(CastError {});
        }
        if expected_package_name != self.package_name {
            return Err(CastError {});
        }
        let create_arguments = match T::try_from_record(self.create_arguments) {
            Ok(value) => value,
            Err(_) => return Err(CastError {}),
        };

        let contract_id = self.contract_id.into_typed();

        let contract_key = TryFromValue::try_from_value(self.contract_key.ok_or(CastError {})?)
            .map_err(|_| CastError {})?;

        Ok(CreatedWithKey {
            offset: self.offset,
            node_id: self.node_id,
            contract_id,
            contract_key,
            contract_key_hash: self.contract_key_hash,
            create_arguments,
            created_event_blob: self.created_event_blob,
            witness_parties: self.witness_parties,
            signatories: self.signatories,
            observers: self.observers,
            created_at: self.created_at,
            acs_delta: self.acs_delta,
        })
    }
}

impl TryFrom<proto::CreatedEvent> for CreatedEvent {
    type Error = ValueError;

    fn try_from(value: proto::CreatedEvent) -> Result<Self, Self::Error> {
        let contract_id = ContractId::new(value.contract_id)
            .validated_of::<proto::CreatedEvent>("contract_id")
            .no_msg()?;
        let template_id = value
            .template_id
            .required_of::<proto::CreatedEvent>("template_id")
            .no_msg()?
            .try_into()
            .validated_of::<proto::CreatedEvent>("template_id")
            .no_msg()?;
        let contract_key = value.contract_key.map(TryInto::try_into).transpose()?;
        let create_arguments = value
            .create_arguments
            .required_of::<proto::CreatedEvent>("create_arguments")
            .no_msg()?
            .try_into()
            .validated_of::<proto::CreatedEvent>("create_arguments")
            .no_msg()?;

        let mut witness_parties = value
            .witness_parties
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::CreatedEvent>("witness_parties")
                    .with_msg_owned(format!("failed to convert witness_parties[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = witness_parties
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::CreatedEvent>("witness_parties")
            .no_msg()?;
        let tail = witness_parties.collect();
        let witness_parties = NonEmpty { head, tail };

        let mut signatories = value
            .signatories
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::CreatedEvent>("signatories")
                    .with_msg_owned(format!("failed to convert signatories[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = signatories
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::CreatedEvent>("signatories")
            .no_msg()?;
        let tail = signatories.collect();
        let signatories = NonEmpty { head, tail };

        let observers = value
            .observers
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::CreatedEvent>("observers")
                    .with_msg_owned(format!("failed to convert observers[{idx}]"))
            })
            .collect::<Result<_, _>>()?;

        let created_at = value
            .created_at
            .required_of::<proto::CreatedEvent>("created_at")
            .no_msg()?
            .try_into()
            .unwrap(); // FIXME: replace unwrap with error

        let package_name = PackageName::new(value.package_name)
            .validated_of::<proto::CreatedEvent>("package_name")
            .no_msg()?;

        Ok(Self {
            offset: value.offset,
            node_id: value.node_id,
            contract_id,
            template_id,
            contract_key,
            contract_key_hash: value.contract_key_hash,
            create_arguments,
            created_event_blob: value.created_event_blob,
            witness_parties,
            signatories,
            observers,
            created_at,
            package_name,
            acs_delta: value.acs_delta,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Archived<T: TemplateValue> {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId<T>,
    pub witness_parties: NonEmpty<PartyId>,
    // TODO: pub implemented_interfaces: ...
}

/// Archived event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivedEvent {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId,
    pub template_id: Identifier<PackageId>,
    pub witness_parties: NonEmpty<PartyId>,
    pub package_name: PackageName,
    // TODO: pub implemented_interfaces: ...
}

impl ArchivedEvent {
    pub fn cast<T: TemplateValue>(self) -> Result<Archived<T>, CastError> {
        let expected_id = T::identifier_with_package_id();
        let expected_package_name = T::package_name();

        if expected_id == self.template_id {
            return Err(CastError {});
        }
        if expected_package_name != self.package_name {
            return Err(CastError {});
        }

        let contract_id = self.contract_id.into_typed();

        Ok(Archived {
            offset: self.offset,
            node_id: self.node_id,
            contract_id,
            witness_parties: self.witness_parties,
        })
    }
}

impl TryFrom<proto::ArchivedEvent> for ArchivedEvent {
    type Error = ValueError;

    fn try_from(value: proto::ArchivedEvent) -> Result<Self, Self::Error> {
        let contract_id = ContractId::new(value.contract_id)
            .validated_of::<proto::ArchivedEvent>("contract_id")
            .no_msg()?;
        let template_id = value
            .template_id
            .required_of::<proto::ArchivedEvent>("template_id")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ArchivedEvent>("template_id")
            .no_msg()?;

        let mut witness_parties = value
            .witness_parties
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::ArchivedEvent>("witness_parties")
                    .with_msg_owned(format!("failed to convert witness_parties[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = witness_parties
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::ArchivedEvent>("witness_parties")
            .no_msg()?;
        let tail = witness_parties.collect();
        let witness_parties = NonEmpty { head, tail };

        let package_name = PackageName::new(value.package_name)
            .validated_of::<proto::ArchivedEvent>("package_name")
            .no_msg()?;

        Ok(Self {
            offset: value.offset,
            node_id: value.node_id,
            contract_id,
            template_id,
            witness_parties,
            package_name,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exercised<T: TemplateValue, C: ChoiceValue<T>> {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId<T>,
    pub choice_argument: C,
    pub acting_parties: NonEmpty<PartyId>,
    pub witness_parties: NonEmpty<PartyId>,
    pub last_descendant_node_id: i32,
    pub exercise_result: C::Result,
    pub acs_delta: bool,
    // TODO: pub implemented_interfaces: ...
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExercisedEvent {
    pub offset: i64,
    pub node_id: i32,
    pub contract_id: ContractId,
    pub template_id: Identifier<PackageId>,
    pub interface_id: Option<Identifier<PackageId>>,
    pub choice: Name,
    pub choice_argument: Value,
    pub acting_parties: NonEmpty<PartyId>,
    pub consuming: bool,
    pub witness_parties: NonEmpty<PartyId>,
    pub last_descendant_node_id: i32,
    pub exercise_result: Option<Value>,
    pub package_name: PackageName,
    pub acs_delta: bool,
    // TODO: pub implemented_interfaces: ...
}

impl ExercisedEvent {
    pub fn cast<T: TemplateValue, C: ChoiceValue<T>>(self) -> Result<Exercised<T, C>, CastError> {
        let expected_id = T::identifier_with_package_id();
        let expected_package_name = T::package_name();

        if expected_id == self.template_id {
            return Err(CastError {});
        }
        if expected_package_name != self.package_name {
            return Err(CastError {});
        }

        let contract_id = self.contract_id.into_typed();

        let choice_argument = C::try_from_value(self.choice_argument).map_err(|_| CastError {})?;

        // FIXME: not sure this is correct
        let exercise_result =
            C::Result::try_from_value(self.exercise_result.unwrap_or(Value::Unit))
                .map_err(|_| CastError {})?;

        Ok(Exercised {
            offset: self.offset,
            node_id: self.node_id,
            contract_id,
            choice_argument,
            acting_parties: self.acting_parties,
            witness_parties: self.witness_parties,
            last_descendant_node_id: self.last_descendant_node_id,
            exercise_result,
            acs_delta: self.acs_delta,
        })
    }
}

impl TryFrom<proto::ExercisedEvent> for ExercisedEvent {
    type Error = ValueError;

    fn try_from(value: proto::ExercisedEvent) -> Result<Self, Self::Error> {
        let contract_id = ContractId::new(value.contract_id)
            .validated_of::<proto::ExercisedEvent>("contract_id")
            .no_msg()?;
        let template_id = value
            .template_id
            .required_of::<proto::ExercisedEvent>("template_id")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ExercisedEvent>("template_id")
            .no_msg()?;
        let interface_id = value
            .interface_id
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<proto::ExercisedEvent>("interface_id")
            .no_msg()?;
        let choice = Name::new(value.choice)
            .validated_of::<proto::ExercisedEvent>("choice")
            .no_msg()?;
        let choice_argument = value
            .choice_argument
            .required_of::<proto::ExercisedEvent>("choice_argument")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ExercisedEvent>("choice_argument")
            .no_msg()?;

        let mut acting_parties = value
            .acting_parties
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::ExercisedEvent>("acting_parties")
                    .with_msg_owned(format!("failed to convert acting_parties[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = acting_parties
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::ExercisedEvent>("acting_parties")
            .no_msg()?;
        let tail = acting_parties.collect();
        let acting_parties = NonEmpty { head, tail };

        let mut witness_parties = value
            .witness_parties
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                PartyId::new(p)
                    .validated_of::<proto::ExercisedEvent>("witness_parties")
                    .with_msg_owned(format!("failed to convert witness_parties[{idx}]"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = witness_parties
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::ExercisedEvent>("witness_parties")
            .no_msg()?;
        let tail = witness_parties.collect();
        let witness_parties = NonEmpty { head, tail };

        let exercise_result = value
            .exercise_result
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<proto::ExercisedEvent>("exercise_result")
            .no_msg()?;

        let package_name = PackageName::new(value.package_name)
            .validated_of::<proto::ExercisedEvent>("package_name")
            .no_msg()?;

        Ok(Self {
            offset: value.offset,
            node_id: value.node_id,
            contract_id,
            template_id,
            interface_id,
            choice,
            choice_argument,
            acting_parties,
            consuming: value.consuming,
            witness_parties,
            last_descendant_node_id: value.last_descendant_node_id,
            exercise_result,
            package_name,
            acs_delta: value.acs_delta,
        })
    }
}
