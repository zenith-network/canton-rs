//! Admin API

pub mod grpc;

#[cfg(feature = "v30")]
pub use canton_proto as proto;
