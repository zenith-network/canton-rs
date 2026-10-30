//! Fixtures and helpers for real-participant integration tests only.

include!(concat!(env!("OUT_DIR"), "/main_package.rs"));

/// DAR used to generate the fixture bindings and preload the sandboxes.
pub const DAR_BYTES: &[u8] = include_bytes!(env!("FIXTURE_DAR_PATH"));
