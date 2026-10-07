//! Fixtures and helpers for real-participant integration tests only.

include!(concat!(env!("OUT_DIR"), "/main_package.rs"));

pub const PACKAGE_ID: &str = env!("FIXTURE_PACKAGE_ID");
pub const PACKAGE_PAYLOAD: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/main-package-payload.bin"));
