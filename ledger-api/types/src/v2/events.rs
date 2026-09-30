use std::time::SystemTime;

use canton_types::{ContractId, Name, PackageId, PackageName, PartyId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::{
    HasIdentifier, Identifier, TryFromRecord, TryFromValue,
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
    /// The views of the interfaces the event's filter requested
    /// (`InterfaceFilter::include_interface_view`).
    pub interface_views: Vec<InterfaceView>,
    pub witness_parties: NonEmpty<PartyId>,
    pub signatories: NonEmpty<PartyId>,
    pub observers: Vec<PartyId>,
    pub created_at: SystemTime,
    pub package_name: PackageName,
    pub acs_delta: bool,
    // TODO: implement missing fields
}

/// The view of one interface on a created event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceView {
    pub interface_id: Identifier<PackageId>,
    /// The computed view, or why the ledger could not compute it.
    pub view: Result<Record, ViewFailure>,
    /// The package whose implementation computed the view, when it succeeded.
    pub implementation_package_id: Option<PackageId>,
}

/// The ledger's reason for a view it could not compute (`google.rpc.Status`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewFailure {
    pub code: i32,
    pub message: String,
}

/// Why `CreatedEvent::view` has no value: the ledger could not compute the view (a
/// problem of this one contract), or the view does not decode as the requested type
/// (the caller's bindings do not match the interface).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError<E> {
    Failed(ViewFailure),
    Decode(E),
}

impl CreatedEvent {
    /// The view of interface `I` on this event, decoded as `V` (the interface's view
    /// type). `None` if the event carries no view of `I`: the filter did not request
    /// it, the template does not implement `I`, or the filter's party is not a
    /// witness. A view of `I`'s own package is preferred; otherwise the first view of
    /// the same module and entity is used (on a verbose stream its record id then
    /// names the other package, and a derived `V` refuses it as `Decode`).
    pub fn view<I: HasIdentifier, V: TryFromRecord>(
        &self,
    ) -> Option<Result<V, ViewError<V::Error>>> {
        let wanted = I::identifier_with_package_id();
        let view = self
            .interface_views
            .iter()
            .find(|v| v.interface_id == wanted)
            .or_else(|| {
                self.interface_views.iter().find(|v| {
                    v.interface_id.module_name == wanted.module_name
                        && v.interface_id.entity_name == wanted.entity_name
                })
            })?;
        Some(match &view.view {
            Ok(record) => V::try_from_record(record.clone()).map_err(ViewError::Decode),
            Err(failure) => Err(ViewError::Failed(failure.clone())),
        })
    }

    /// Cast to typed event
    pub fn cast<T: TemplateValue>(self) -> Result<Created<T>, CastError> {
        let expected_id = T::identifier_with_package_id();
        let expected_package_name = T::package_name();

        if expected_id != self.template_id {
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

        if expected_id != self.template_id {
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

        let interface_views = value
            .interface_views
            .into_iter()
            .map(|v| {
                let interface_id = v
                    .interface_id
                    .required_of::<proto::InterfaceView>("interface_id")
                    .no_msg()?
                    .try_into()
                    .validated_of::<proto::InterfaceView>("interface_id")
                    .no_msg()?;
                let failed = v
                    .view_status
                    .as_ref()
                    .filter(|s| s.code != 0)
                    .map(|s| ViewFailure {
                        code: s.code,
                        message: s.message.clone(),
                    });
                let view = match (failed, v.view_value) {
                    (Some(failure), _) => Err(failure),
                    (None, Some(record)) => Ok(record
                        .try_into()
                        .validated_of::<proto::InterfaceView>("view_value")
                        .no_msg()?),
                    (None, None) => Err(ViewFailure {
                        code: 0,
                        message: "no view value".to_string(),
                    }),
                };
                let implementation_package_id = if v.implementation_package_id.is_empty() {
                    None
                } else {
                    Some(
                        PackageId::new(v.implementation_package_id)
                            .validated_of::<proto::InterfaceView>("implementation_package_id")
                            .no_msg()?,
                    )
                };
                Ok(InterfaceView {
                    interface_id,
                    view,
                    implementation_package_id,
                })
            })
            .collect::<Result<Vec<_>, ValueError>>()?;

        Ok(Self {
            offset: value.offset,
            node_id: value.node_id,
            contract_id,
            template_id,
            contract_key,
            contract_key_hash: value.contract_key_hash,
            create_arguments,
            created_event_blob: value.created_event_blob,
            interface_views,
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

        if expected_id != self.template_id {
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

        if expected_id != self.template_id {
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

#[cfg(test)]
mod interface_view_tests {
    use canton_types::DottedName;
    use ledger_api_value::v2::value::{Record, RecordField};

    use super::*;

    /// An interface marker defined in package `iface_pkg`.
    struct Iface;
    impl HasIdentifier for Iface {
        fn package_id() -> PackageId {
            PackageId::new_unchecked("iface_pkg")
        }
        fn package_name() -> PackageName {
            PackageName::new_unchecked("iface")
        }
        fn module_name() -> DottedName {
            DottedName::single(Name::new_static_unchecked("M"))
        }
        fn entity_name() -> DottedName {
            DottedName::single(Name::new_static_unchecked("I"))
        }
    }

    /// The interface's view type: one Int64 field.
    #[derive(Debug, PartialEq)]
    struct View(i64);
    impl TryFromRecord for View {
        type Error = std::fmt::Error;
        fn try_from_record(record: Record) -> Result<Self, Self::Error> {
            match record.fields.as_slice() {
                [
                    RecordField {
                        value: Value::Int64(n),
                        ..
                    },
                ] => Ok(View(*n)),
                _ => Err(std::fmt::Error),
            }
        }
    }

    fn id(package: &str) -> proto::Identifier {
        proto::Identifier {
            package_id: package.into(),
            module_name: "M".into(),
            entity_name: "I".into(),
        }
    }

    fn record(fields: Vec<proto::Value>) -> proto::Record {
        proto::Record {
            record_id: None,
            fields: fields
                .into_iter()
                .map(|value| proto::RecordField {
                    label: String::new(),
                    value: Some(value),
                })
                .collect(),
        }
    }

    fn int(n: i64) -> proto::Value {
        proto::Value {
            sum: Some(proto::value::Sum::Int64(n)),
        }
    }

    fn view(
        package: &str,
        status: Option<(i32, &str)>,
        value: Option<proto::Record>,
    ) -> proto::InterfaceView {
        proto::InterfaceView {
            interface_id: Some(id(package)),
            view_status: status.map(|(code, message)| ledger_api_proto::google::rpc::Status {
                code,
                message: message.into(),
                details: vec![],
            }),
            view_value: value,
            implementation_package_id: if value_ok(status) {
                "impl_pkg".into()
            } else {
                String::new()
            },
        }
    }

    fn value_ok(status: Option<(i32, &str)>) -> bool {
        status.is_none_or(|(code, _)| code == 0)
    }

    fn event(views: Vec<proto::InterfaceView>) -> CreatedEvent {
        let party = format!("alice::1220{}", "ab".repeat(32));
        proto::CreatedEvent {
            contract_id: format!("00{}", "cd".repeat(33)),
            template_id: Some(proto::Identifier {
                package_id: "tpl_pkg".into(),
                module_name: "M".into(),
                entity_name: "T".into(),
            }),
            create_arguments: Some(record(vec![])),
            interface_views: views,
            witness_parties: vec![party.clone()],
            signatories: vec![party],
            created_at: Some(Default::default()),
            package_name: "tpl".into(),
            ..Default::default()
        }
        .try_into()
        .expect("a valid created event")
    }

    #[test]
    fn a_computed_view_decodes_with_its_implementation_package() {
        let e = event(vec![view(
            "iface_pkg",
            Some((0, "")),
            Some(record(vec![int(7)])),
        )]);
        assert_eq!(e.view::<Iface, View>(), Some(Ok(View(7))));
        assert_eq!(
            e.interface_views[0].implementation_package_id,
            Some(PackageId::new_unchecked("impl_pkg"))
        );
    }

    #[test]
    fn a_failed_view_keeps_the_ledgers_reason() {
        let e = event(vec![view("iface_pkg", Some((9, "view failed")), None)]);
        assert_eq!(
            e.view::<Iface, View>(),
            Some(Err(ViewError::Failed(ViewFailure {
                code: 9,
                message: "view failed".into()
            })))
        );
    }

    #[test]
    fn a_view_of_another_shape_is_a_decode_error() {
        let e = event(vec![view("iface_pkg", None, Some(record(vec![])))]);
        assert_eq!(
            e.view::<Iface, View>(),
            Some(Err(ViewError::Decode(std::fmt::Error)))
        );
    }

    #[test]
    fn neither_status_nor_value_is_a_failure() {
        let e = event(vec![view("iface_pkg", None, None)]);
        assert!(matches!(
            e.view::<Iface, View>(),
            Some(Err(ViewError::Failed(_)))
        ));
    }

    #[test]
    fn no_view_of_the_interface_is_none() {
        assert_eq!(event(vec![]).view::<Iface, View>(), None);
    }

    #[test]
    fn the_interfaces_own_package_is_preferred() {
        let e = event(vec![
            view("other_pkg", None, Some(record(vec![int(1)]))),
            view("iface_pkg", None, Some(record(vec![int(2)]))),
        ]);
        assert_eq!(e.view::<Iface, View>(), Some(Ok(View(2))));
        // Without it, a view of the same module and entity is used.
        let e = event(vec![view("other_pkg", None, Some(record(vec![int(1)])))]);
        assert_eq!(e.view::<Iface, View>(), Some(Ok(View(1))));
    }
}
