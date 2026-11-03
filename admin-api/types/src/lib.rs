//! Admin API common types

#[cfg(feature = "v30")]
mod api_name;

pub mod error;

#[cfg(feature = "v30")]
pub use api_name::{ApiName, UnknownApiName};
