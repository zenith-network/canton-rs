use std::time::{Duration, SystemTime};

use canton_types::{
    ContractId, LedgerString, Name, NonEmpty, PackageName, PartyId, SynchronizerId, UserId,
};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::{
    Identifier, IntoValue as _,
    value::{Record, Value},
};

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
    // pub disclosed_contracts: Vec<DisclosedContract>,
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
            synchronizer_id: None,
            taps_max_passes: None,
        }
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

    pub fn with_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    // TODO: implement other helpers
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
