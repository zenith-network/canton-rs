//! Common Canton types.

mod contract_id;
mod dotted_name;
mod ledger_string;
mod name;
mod non_empty;
mod numeric;
mod package_id;
mod package_id_any;
mod package_identifier;
mod package_name;
mod party_id;
mod synchronizer_id;
mod traits;
mod user_id;

#[cfg(feature = "testing")]
pub mod test_fixtures;

pub use contract_id::ContractId;
pub use dotted_name::DottedName;
pub use ledger_string::LedgerString;
pub use name::Name;
pub use non_empty::NonEmpty;
pub use numeric::Numeric;
pub use package_id::PackageId;
pub use package_id_any::PackageIdAny;
pub use package_identifier::PackageIdentifier;
pub use package_name::PackageName;
pub use party_id::PartyId;
pub use synchronizer_id::SynchronizerId;
pub use traits::{Choice, Template, TemplateWithKey};
pub use user_id::UserId;

/// Error types
pub mod errors {
    // Re-export errors from a single module for convenience
    pub use super::contract_id::ContractIdError;
    pub use super::dotted_name::DottedNameError;
    pub use super::ledger_string::LedgerStringError;
    pub use super::name::NameError;
    pub use super::numeric::NumericError;
    pub use super::package_id::PackageIdError;
    pub use super::package_id_any::PackageIdAnyError;
    pub use super::package_name::PackageNameError;
    pub use super::party_id::PartyIdError;
    pub use super::synchronizer_id::SynchronizerIdError;
    pub use super::user_id::UserIdError;
}

/// ZST for representing unspecified template.
///
/// This type __does not__ implement [`Template`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnyTemplate;

pub use bigdecimal;
pub use bigdecimal::BigDecimal;

pub use frunk;

// TODO: add serde support
// TODO: add tracing support

// #[macro_export]
// macro_rules! cons {
//     () => { crate::EmptyList };

//     ($head:ty $(, $tail:ty)* $(,)?) => {
//         crate::Cons<$head, crate::cons!($($tail),*)>
//     };
// }
