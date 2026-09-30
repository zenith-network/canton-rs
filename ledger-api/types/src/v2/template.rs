use canton_types::{self as daml, ContractId};
use ledger_api_value::v2::{HasIdentifier, Record, Value};

use crate::v2::{Create, CreateAndExercise, Exercise, ExerciseByKey};

/// Template or interface, which can be used in Ledger API
///
/// Auto-implemented for all matching types
pub trait TemplateOrInterface: daml::TemplateOrInterface + HasIdentifier {}

impl<R> TemplateOrInterface for R where R: daml::TemplateOrInterface + HasIdentifier {}

/// A [`Template`][daml::Template] type which can be used in Ledger API
///
/// Auto-implemented for all matching types
pub trait TemplateValue: TemplateOrInterface + daml::Template + Record {
    /// Construct `Create` command from self
    fn create(self) -> Create<Self> {
        Create {
            create_arguments: self,
        }
    }

    /// Construct `CreateAndExercise` command from self and choice arguments
    fn create_and_exercise<C: ChoiceValue<Self>>(
        self,
        choice_argument: C,
    ) -> CreateAndExercise<Self, C> {
        CreateAndExercise {
            create_arguments: self,
            choice_argument,
        }
    }
}

impl<T> TemplateValue for T where T: daml::Template + TemplateOrInterface + Record {}

/// A template with key which can be used in Ledger API
///
/// Auto-implemented for all matching types
pub trait TemplateValueWithKey: TemplateValue + daml::TemplateWithKey<Key: Value> {}

impl<T> TemplateValueWithKey for T where T: TemplateValue + daml::TemplateWithKey<Key: Value> {}

/// A [`Choice`][daml::Choice] type which can be used in Ledger API
///
/// Auto-implemented for all matching types
pub trait ChoiceValue<R: TemplateOrInterface>: daml::Choice<R, Result: Value> + Value {
    /// Construct `Exercise` command from self
    fn exercise(self, contract_id: ContractId<R>) -> Exercise<R, Self> {
        Exercise {
            contract_id,
            choice_argument: self,
        }
    }

    /// Construct `Exercise` command from self, which exercises an interface choice using contract
    /// ID of a template
    fn exercise_interface<T>(self, contract_id: ContractId<T>) -> Exercise<R, Self>
    where
        R: Interface,
        T: Implements<R>,
    {
        Self::exercise(self, contract_id.into_interface())
    }
}

impl<C, R> ChoiceValue<R> for C
where
    C: daml::Choice<R, Result: Value> + Value,
    R: TemplateOrInterface,
{
}

/// A choice type of a template with a key which can be used in Ledger API
///
/// Auto-implemented for all matching types
pub trait ChoiceByKeyValue<T: TemplateValueWithKey + TemplateOrInterface>: ChoiceValue<T> {
    /// Construct `ExerciseByKey` command from self
    fn exercise_by_key(self, contract_key: T::Key) -> ExerciseByKey<T, Self> {
        ExerciseByKey {
            contract_key,
            choice_argument: self,
        }
    }
}

impl<C, T> ChoiceByKeyValue<T> for C
where
    C: ChoiceValue<T>,
    T: TemplateValueWithKey + TemplateOrInterface,
{
}

/// An [`Interface`][daml::Interface] type which can be used in Ledger API
///
/// Auto-implemented for all matching types.
pub trait Interface: daml::Interface<View: Record> + HasIdentifier {}

impl<I> Interface for I where I: daml::Interface<View: Record> + HasIdentifier {}

/// A [`TemplateValue`] which implements an [`Interface`]
///
/// Auto-implemented for all matching types.
pub trait Implements<I: Interface>: daml::Implements<I> + TemplateValue {}

impl<T, I> Implements<I> for T
where
    T: daml::Implements<I> + TemplateValue,
    I: Interface,
{
}
