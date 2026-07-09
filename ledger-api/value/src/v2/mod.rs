//! Ledger API v2

mod identifier;
mod traits;

pub mod errors;
#[cfg(feature = "testing")]
pub mod test_fixtures;
pub mod value;

pub use identifier::{HasIdentifier, Identifier};
pub use traits::{IntoRecord, IntoValue, Record, TryFromRecord, TryFromValue, Value};

// TODO: maybe it will be convenient to have a dedicated representations for verbose and non-verbose

#[cfg(feature = "derive")]
pub use ledger_api_value_derive::Value;
