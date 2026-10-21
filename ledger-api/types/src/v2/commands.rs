use std::time::{Duration, SystemTime};

use canton_types::{
    ContractId, LedgerString, Name, NonEmpty, PackageId, PackageName, PartyId, SynchronizerId,
    UserId,
};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::{
    Identifier, IntoValue as _,
    errors::{IntoValueError as _, ValueError},
    value::{Record, Value},
};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};
use uuid::Uuid;

use crate::v2::{ChoiceByKeyValue, ChoiceValue, TemplateValue, TemplateValueWithKey};

/// A composite command that groups multiple erased commands together
#[derive(Clone, Debug)]
pub struct Commands {
    pub workflow_id: Option<LedgerString>,
    pub user_id: Option<UserId>,
    pub command_id: LedgerString,
    pub commands: Vec<Command>,
    pub min_ledger_time_abs: Option<SystemTime>,
    pub min_ledger_time_rel: Option<Duration>,
    pub act_as: NonEmpty<PartyId>,
    pub read_as: Vec<PartyId>,
    pub submission_id: Option<LedgerString>,
    pub disclosed_contracts: Vec<DisclosedContract>,
    pub synchronizer_id: Option<SynchronizerId>,
    // pub package_id_selection_preference: Vec<PackageId>,
    // pub prefetch_contract_keys: Vec<PrefetchContractKey>,
    pub taps_max_passes: Option<u32>,
    // pub deduplication_period: Option<DeduplicationPeriod>,

    // TODO: implement missing fields
}

impl Commands {
    pub fn new(command_id: LedgerString, act_as: NonEmpty<PartyId>) -> Self {
        Self {
            workflow_id: None,
            user_id: None,
            command_id,
            commands: Vec::new(),
            min_ledger_time_abs: None,
            min_ledger_time_rel: None,
            act_as,
            read_as: Vec::new(),
            submission_id: None,
            disclosed_contracts: Vec::new(),
            synchronizer_id: None,
            taps_max_passes: None,
        }
    }

    pub fn with_read_as(&mut self, party: PartyId) -> &mut Self {
        self.read_as.push(party);
        self
    }

    pub fn with_workflow_id(&mut self, workflow_id: Option<LedgerString>) -> &mut Self {
        self.workflow_id = workflow_id;
        self
    }

    pub fn with_user_id(&mut self, user_id: Option<UserId>) -> &mut Self {
        self.user_id = user_id;
        self
    }

    pub fn with_commands(&mut self, commands: impl IntoIterator<Item = Command>) -> &mut Self {
        self.commands.extend(commands);
        self
    }

    pub fn with_command(&mut self, command: Command) -> &mut Self {
        self.commands.push(command);
        self
    }

    pub fn with_submission_id(&mut self, submission_id: Option<LedgerString>) -> &mut Self {
        self.submission_id = submission_id;
        self
    }

    /// Set submission ID to randomly generated UUIDv7
    pub fn with_random_submission_id(&mut self) -> &mut Self {
        self.submission_id = Some(
            LedgerString::new(Uuid::now_v7().to_string())
                .expect("UUIDv7 must always be a valid LedgerString"),
        );
        self
    }

    pub fn with_disclosed_contract(&mut self, contract: DisclosedContract) -> &mut Self {
        self.disclosed_contracts.push(contract);
        self
    }

    pub fn with_disclosed_contracts(
        &mut self,
        contracts: impl IntoIterator<Item = DisclosedContract>,
    ) -> &mut Self {
        self.disclosed_contracts.extend(contracts);
        self
    }

    // TODO: implement other helpers
}

/// A contract the submitter does not see, attached to the command for its interpretation,
/// as a stakeholder's participant exported it (`CreatedEvent.created_event_blob`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DisclosedContract {
    pub template_id: Identifier<PackageId>,
    pub contract_id: ContractId,
    pub created_event_blob: Vec<u8>,
    /// The synchronizer the contract is assigned to; `None` lets the participant infer it.
    pub synchronizer_id: Option<SynchronizerId>,
}

impl From<DisclosedContract> for proto::DisclosedContract {
    fn from(value: DisclosedContract) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            contract_id: value.contract_id.into(),
            created_event_blob: value.created_event_blob,
            synchronizer_id: value.synchronizer_id.map(Into::into).unwrap_or_default(),
        }
    }
}

impl TryFrom<proto::DisclosedContract> for DisclosedContract {
    type Error = ValueError;

    fn try_from(value: proto::DisclosedContract) -> Result<Self, Self::Error> {
        let template_id = value
            .template_id
            .required_of::<proto::DisclosedContract>("template_id")
            .no_msg()?
            .try_into()
            .validated_of::<proto::DisclosedContract>("template_id")
            .no_msg()?;
        let contract_id = ContractId::new(value.contract_id)
            .validated_of::<proto::DisclosedContract>("contract_id")
            .no_msg()?;
        let synchronizer_id = if value.synchronizer_id.is_empty() {
            None
        } else {
            Some(
                SynchronizerId::new(value.synchronizer_id)
                    .validated_of::<proto::DisclosedContract>("synchronizer_id")
                    .no_msg()?,
            )
        };
        Ok(Self {
            template_id,
            contract_id,
            created_event_blob: value.created_event_blob,
            synchronizer_id,
        })
    }
}

impl From<Commands> for proto::Commands {
    fn from(value: Commands) -> Self {
        Self {
            workflow_id: value.workflow_id.map(Into::into).unwrap_or_default(),
            user_id: value.user_id.map(Into::into).unwrap_or_default(),
            command_id: value.command_id.into(),
            commands: value.commands.into_iter().map(Into::into).collect(),
            act_as: value.act_as.into_iter().map(Into::into).collect(),
            read_as: value.read_as.into_iter().map(Into::into).collect(),
            submission_id: value.submission_id.map(Into::into).unwrap_or_default(),
            disclosed_contracts: value
                .disclosed_contracts
                .into_iter()
                .map(Into::into)
                .collect(),
            synchronizer_id: value.synchronizer_id.map(Into::into).unwrap_or_default(),
            taps_max_passes: value.taps_max_passes,
            min_ledger_time_abs: value.min_ledger_time_abs.map(Into::into),
            min_ledger_time_rel: value.min_ledger_time_rel.map(|t| t.try_into().unwrap()), // FIXME: do something about this unwrap
            ..Default::default() // TODO: convert other fields when they are implemented
        }
    }
}

/// Command with erased types
#[derive(Clone, Debug)]
pub enum Command {
    Create(CreateCommand),
    Exercise(ExerciseCommand),
    ExerciseByKey(ExerciseByKeyCommand),
    CreateAndExercise(CreateAndExerciseCommand),
}

impl From<Command> for proto::Command {
    fn from(value: Command) -> Self {
        Self {
            command: Some(match value {
                Command::Create(erased_create) => {
                    proto::command::Command::Create(erased_create.into())
                }
                Command::Exercise(erased_exercise) => {
                    proto::command::Command::Exercise(erased_exercise.into())
                }
                Command::ExerciseByKey(erased_exercise_by_key) => {
                    proto::command::Command::ExerciseByKey(erased_exercise_by_key.into())
                }
                Command::CreateAndExercise(erased_create_and_exercise) => {
                    proto::command::Command::CreateAndExercise(erased_create_and_exercise.into())
                }
            }),
        }
    }
}

impl From<CreateCommand> for Command {
    fn from(value: CreateCommand) -> Self {
        Self::Create(value)
    }
}

impl From<ExerciseCommand> for Command {
    fn from(value: ExerciseCommand) -> Self {
        Self::Exercise(value)
    }
}

impl From<ExerciseByKeyCommand> for Command {
    fn from(value: ExerciseByKeyCommand) -> Self {
        Self::ExerciseByKey(value)
    }
}

impl From<CreateAndExerciseCommand> for Command {
    fn from(value: CreateAndExerciseCommand) -> Self {
        Self::CreateAndExercise(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Create<T: TemplateValue> {
    pub create_arguments: T,
}

impl<T: TemplateValue> Create<T> {
    /// Erase type information, converting to raw command
    pub fn erase(self) -> CreateCommand {
        CreateCommand {
            template_id: T::identifier(),
            create_arguments: self.create_arguments.into_record(),
        }
    }
}

/// Create command with erased type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateCommand {
    /// We use package-name here because package-id is marked as deprecated in Protobuf
    pub template_id: Identifier<PackageName>,
    pub create_arguments: Record,
}

impl From<CreateCommand> for proto::CreateCommand {
    fn from(value: CreateCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            create_arguments: Some(value.create_arguments.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exercise<T: TemplateValue, C: ChoiceValue<T>> {
    pub contract_id: ContractId<T>,
    pub choice_argument: C,
}

impl<T: TemplateValue, C: ChoiceValue<T>> Exercise<T, C> {
    /// Erase type information, converting to raw command
    pub fn erase(self) -> ExerciseCommand {
        ExerciseCommand {
            template_id: T::identifier(),
            contract_id: self.contract_id.into_any(),
            choice: C::NAME,
            choice_argument: self.choice_argument.into_value(),
        }
    }
}

/// Exercise command with erased type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExerciseCommand {
    /// We use package-name here because package-id is marked as deprecated in Protobuf
    pub template_id: Identifier<PackageName>,
    pub contract_id: ContractId,
    pub choice: Name,
    pub choice_argument: Value,
}

impl From<ExerciseCommand> for proto::ExerciseCommand {
    fn from(value: ExerciseCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            contract_id: value.contract_id.into(),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExerciseByKey<T: TemplateValueWithKey, C: ChoiceByKeyValue<T>> {
    pub contract_key: T::Key,
    pub choice_argument: C,
}

impl<T: TemplateValueWithKey, C: ChoiceByKeyValue<T>> ExerciseByKey<T, C> {
    /// Erase type information, converting to raw command
    pub fn erase(self) -> ExerciseByKeyCommand {
        ExerciseByKeyCommand {
            template_id: T::identifier(),
            contract_key: self.contract_key.into_value(),
            choice: C::NAME,
            choice_argument: self.choice_argument.into_value(),
        }
    }
}

/// ExerciseByKey command with erased type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExerciseByKeyCommand {
    pub template_id: Identifier<PackageName>,
    pub contract_key: Value,
    pub choice: Name,
    pub choice_argument: Value,
}

impl From<ExerciseByKeyCommand> for proto::ExerciseByKeyCommand {
    fn from(value: ExerciseByKeyCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            contract_key: Some(value.contract_key.into()),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateAndExercise<T: TemplateValue, C: ChoiceValue<T>> {
    pub create_arguments: T,
    pub choice_argument: C,
}

impl<T: TemplateValue, C: ChoiceValue<T>> CreateAndExercise<T, C> {
    /// Erase type information, converting to raw command
    pub fn erase(self) -> CreateAndExerciseCommand {
        CreateAndExerciseCommand {
            template_id: T::identifier(),
            create_arguments: self.create_arguments.into_record(),
            choice: C::NAME,
            choice_argument: self.choice_argument.into_value(),
        }
    }
}

/// CreateAndExercise command with erased type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateAndExerciseCommand {
    pub template_id: Identifier<PackageName>,
    pub create_arguments: Record,
    pub choice: Name,
    pub choice_argument: Value,
}

impl From<CreateAndExerciseCommand> for proto::CreateAndExerciseCommand {
    fn from(value: CreateAndExerciseCommand) -> Self {
        Self {
            template_id: Some(value.template_id.into()),
            create_arguments: Some(value.create_arguments.into()),
            choice: value.choice.into(),
            choice_argument: Some(value.choice_argument.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use canton_types::DottedName;
    use pretty_assertions::assert_eq;

    use super::*;

    fn disclosed(synchronizer_id: Option<&str>) -> DisclosedContract {
        DisclosedContract {
            template_id: Identifier {
                package_id: PackageId::new(
                    "3ca1343ab26b453d38c8adb70dca5f1ead8440c42b59b68f070786955cbf9ec1".to_string(),
                )
                .unwrap(),
                module_name: DottedName::parse("Splice.AmuletRules").unwrap(),
                entity_name: DottedName::parse("AmuletRules").unwrap(),
            },
            contract_id: ContractId::new(
                "00a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90ca1112"
                    .to_string(),
            )
            .unwrap(),
            created_event_blob: vec![0x0a, 0x00, 0xff, 0x42],
            synchronizer_id: synchronizer_id.map(|s| SynchronizerId::new(s.to_string()).unwrap()),
        }
    }

    #[test]
    fn disclosed_contract_proto_round_trip() {
        let sync =
            "global-domain::1220be58c29e65de40bf273be1dc2b266d43a9a002ea5b18955aeef7aac881bb471a";
        for contract in [disclosed(Some(sync)), disclosed(None)] {
            let encoded: proto::DisclosedContract = contract.clone().into();
            assert_eq!(encoded.created_event_blob, contract.created_event_blob);
            let decoded = DisclosedContract::try_from(encoded).unwrap();
            assert_eq!(decoded, contract);
        }
    }

    #[test]
    fn disclosed_contract_requires_template_id() {
        let mut encoded: proto::DisclosedContract = disclosed(None).into();
        encoded.template_id = None;
        assert!(DisclosedContract::try_from(encoded).is_err());
    }

    #[test]
    fn commands_carry_disclosed_contracts_into_proto() {
        let mut commands = Commands::new(
            LedgerString::new("cmd-1".to_string()).unwrap(),
            NonEmpty::single(PartyId::new("alice::1220abcd".to_string()).unwrap()),
        );
        commands.with_disclosed_contracts([disclosed(None), disclosed(Some("sync::1220ef"))]);
        let encoded: proto::Commands = commands.into();
        let decoded: Vec<DisclosedContract> = encoded
            .disclosed_contracts
            .into_iter()
            .map(|d| d.try_into().unwrap())
            .collect();
        assert_eq!(
            decoded,
            vec![disclosed(None), disclosed(Some("sync::1220ef"))]
        );
    }
}
