use canton_types::{Choice, ContractId, Template, TemplateWithKey};
use ledger_api_value::v2::{HasIdentifier, Record, Value};

use crate::v2::{Create, CreateAndExercise, Exercise, ExerciseByKey};

/// A [`Template`] type which can be used in Ledger API
pub trait TemplateValue: Template + HasIdentifier + Record {
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

pub trait TemplateValueWithKey: TemplateValue + TemplateWithKey<Key: Value> {}

/// A [`Choice`] type which can be used in Ledger API
pub trait ChoiceValue<T: TemplateValue>: Choice<T, Result: Value> + Value {
    /// Construct `Exercise` command from self
    fn exercise(self, contract_id: ContractId<T>) -> Exercise<T, Self> {
        Exercise {
            contract_id,
            choice_argument: self,
        }
    }
}

pub trait ChoiceByKeyValue<T: TemplateValueWithKey>: ChoiceValue<T> {
    /// Construct `ExerciseByKey` command from self
    fn exercise_by_key(self, contract_key: T::Key) -> ExerciseByKey<T, Self> {
        ExerciseByKey {
            contract_key,
            choice_argument: self,
        }
    }
}
