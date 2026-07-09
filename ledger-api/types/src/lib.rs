//! Values used by Ledger API

#[cfg(feature = "v2")]
pub mod v2;

// Re-export because types from these crates are in public API
pub use canton_types;
pub use nonempty;
pub use ledger_api_value as value;
