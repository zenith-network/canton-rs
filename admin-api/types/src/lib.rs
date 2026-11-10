//! Admin API common types

#[cfg(feature = "v30")]
mod api_name;

pub mod error;
pub mod topology;

#[cfg(feature = "v30")]
pub use api_name::{ApiName, UnknownApiName};
