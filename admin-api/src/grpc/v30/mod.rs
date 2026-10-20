//! Canton Admin gRPC API

pub mod auth;
pub mod client;
pub mod error;
pub mod services;

pub use client::{AdminClient, AdminClientBuilder, ClientTlsConfig};
