//! Error types
use std::borrow::Cow;

use canton_types::errors::{
    ContractIdError, DottedNameError, LedgerStringError, NameError, NumericError,
    PackageIdAnyError, PackageIdError, PackageNameError, PartyIdError, SynchronizerIdError,
    UserIdError,
};
use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField};

use canton_types::PackageId;

use crate::v2::{Identifier, value::ValueKind};

/// Aggregated error for compound types implementing
/// [`TryFromValue`][crate::v2::traits::TryFromValue] (e.g. `Option<T>`)
#[derive(Debug, thiserror::Error)]
pub enum AggregatedValueError<E> {
    ValueKindError(#[from] ValueKindError),
    Other(E),
}

/// Aggregated error for compound types implementing
/// [`TryFromValue`][crate::v2::traits::TryFromValue] (e.g. `BTreeMap<K, V>`)
#[derive(Debug, thiserror::Error)]
pub enum Aggregated2ValueError<E1, E2> {
    ValueKindError(#[from] ValueKindError),
    Other1(E1),
    Other2(E2),
}

/// Error raised when there is a mismatch between expected and real value kind
#[derive(Debug, thiserror::Error)]
#[error("unexpected value kind: expected '{expected}', got '{got}'")]
pub struct ValueKindError {
    pub expected: ValueKind,
    pub got: ValueKind,
}

impl ValueKindError {
    pub fn new(expected: ValueKind, got: ValueKind) -> Self {
        Self { expected, got }
    }
}

/// Error on converting record value to a tuple
#[derive(Debug, thiserror::Error)]
pub enum TupleFromRecordError {
    #[error("unexpected record identifier: expected '{expected}', got '{got}'")]
    UnexpectedIdentifier { expected: String, got: String },
    #[error("unexpected length of the tuple: expected {expected}, got {got}")]
    UnexpectedLength { expected: usize, got: usize },
    #[error("unexpected label on record: expected '{expected}', got '{got}'")]
    UnexpectedLabel { expected: String, got: String },
}

impl TupleFromRecordError {
    pub fn unexpected_identifier(
        expected: &Identifier<PackageId>,
        got: &Identifier<PackageId>,
    ) -> Self {
        Self::UnexpectedIdentifier {
            expected: expected.to_string(),
            got: got.to_string(),
        }
    }

    pub fn unexpected_length(expected: usize, got: usize) -> Self {
        Self::UnexpectedLength { expected, got }
    }

    pub fn unexpected_label(expected: String, got: String) -> Self {
        Self::UnexpectedLabel { expected, got }
    }
}

/// Error on converting record value to a tuple (T1, T2)
#[derive(Debug, thiserror::Error)]
pub enum Tuple2Error<E1, E2> {
    #[error(transparent)]
    TupleFromRecordError(#[from] TupleFromRecordError),
    #[error(transparent)]
    T1Error(E1),
    #[error(transparent)]
    T2Error(E2),
}

/// Error on converting record value to a tuple (T1, T2, T3)
#[derive(Debug, thiserror::Error)]
pub enum Tuple3Error<E1, E2, E3> {
    #[error(transparent)]
    TupleFromRecordError(#[from] TupleFromRecordError),
    #[error(transparent)]
    T1Error(E1),
    #[error(transparent)]
    T2Error(E2),
    #[error(transparent)]
    T3Error(E3),
}

/// Helper trait for creating [`ValueError`] from source error with a message
///
/// # Example
///
/// ```rust,no_run
/// # use daml_primitives::party_id::PartyIdError;
/// # use daml_lf_ledger_api_value::errors::{IntoValueError, ValueError};
/// fn func() -> Result<(), PartyIdError> { Ok(()) }
///
/// fn another_func() -> Result<(), ValueError> {
///     func().with_msg("something went wrong")?;
///     Ok(())
/// }
/// ```
pub trait IntoValueError {
    type Ok;

    /// Wrap source error with a message
    fn with_msg(self, msg: &'static str) -> Result<Self::Ok, ValueError>;

    /// Wrap source error with a message (as owned String)
    fn with_msg_owned(self, msg: String) -> Result<Self::Ok, ValueError>;

    /// Wrap the source with a default message
    fn no_msg(self) -> Result<Self::Ok, ValueError>;
}

impl<T, E: Into<ValueErrorKind>> IntoValueError for Result<T, E> {
    type Ok = T;

    fn with_msg(self, msg: &'static str) -> Result<T, ValueError> {
        self.map_err(|err| ValueError {
            message: Some(msg.into()),
            kind: Some(err.into()),
        })
    }

    fn with_msg_owned(self, msg: String) -> Result<T, ValueError> {
        self.map_err(|err| ValueError {
            message: Some(msg.into()),
            kind: Some(err.into()),
        })
    }

    fn no_msg(self) -> Result<T, ValueError> {
        self.map_err(|err| ValueError {
            message: None,
            kind: Some(err.into()),
        })
    }
}

// /// Aggregated error of converting [`TryFromLedgerApiValue`] type from proto value
// #[derive(Debug, thiserror::Error)]
// pub enum TryFromProtoError<T> {
//     ValueError(#[from] ValueError),
//     Other(T),
// }

/// Error on converting values from protobuf representation
#[derive(Debug, thiserror::Error)]
#[error(
    "{}",
    message.as_ref().map(ToString::to_string).unwrap_or_else(|| Self::DEFAULT_MESSAGE.to_string()),
)]
pub struct ValueError {
    message: Option<Cow<'static, str>>,
    #[source]
    kind: Option<ValueErrorKind>,
}

impl ValueError {
    /// Default message which is printed, if no message was specified
    const DEFAULT_MESSAGE: &str = "value conversion error";

    /// Create `ValueError` which doesn't have source error, only a message
    pub fn raw_message(message: &'static str) -> Self {
        Self {
            message: Some(message.into()),
            kind: None,
        }
    }

    /// Create `ValueError` which doesn't have source error, only a message (as owned String)
    pub fn raw_message_owned(message: String) -> Self {
        Self {
            message: Some(message.into()),
            kind: None,
        }
    }

    /// Create `'sum' proto field is missing` error
    pub fn no_value_found() -> Self {
        Self {
            message: None,
            kind: Some(ValueErrorKind::NoValueFound),
        }
    }
}

impl From<ValueErrorKind> for ValueError {
    fn from(kind: ValueErrorKind) -> Self {
        Self {
            message: None,
            kind: Some(kind),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ValueErrorKind {
    #[error("no value found ('sum' proto field is missing)")]
    NoValueFound,
    #[error(transparent)]
    PartyIdError(#[from] PartyIdError),
    #[error(transparent)]
    ContractIdError(#[from] ContractIdError),
    #[error(transparent)]
    PackageIdError(#[from] PackageIdError),
    #[error(transparent)]
    PackageNameError(#[from] PackageNameError),
    #[error(transparent)]
    PackageIdAnyError(#[from] PackageIdAnyError),
    #[error(transparent)]
    NameStringError(#[from] NameError),
    #[error(transparent)]
    LedgerStringError(#[from] LedgerStringError),
    #[error(transparent)]
    UserIdError(#[from] UserIdError),
    #[error(transparent)]
    SynchronizerIdError(#[from] SynchronizerIdError),
    #[error(transparent)]
    NumericError(#[from] NumericError),
    #[error(transparent)]
    DottedNameError(#[from] DottedNameError),
    #[error(transparent)]
    MissingProtoField(#[from] MissingProtoField),
    #[error(transparent)]
    InvalidProtoFieldValue(#[from] InvalidProtoFieldValue),
    #[error(transparent)]
    Nested(Box<ValueError>),
}

impl From<ValueError> for ValueErrorKind {
    fn from(error: ValueError) -> Self {
        Self::Nested(Box::new(error))
    }
}
