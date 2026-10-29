//! Shared setup and assertions for the real-participant suite.
//!
//! Context creates isolated test identities and clients. Fixtures provide Daml
//! values, RPC helpers enforce deadlines, and comparisons check SDK results
//! against the participant's protobuf responses.

use std::{sync::OnceLock, time::Duration};

use sandbox::SandboxConfig;
pub use sandbox::{Sandbox, sandbox};

mod context;
mod errors;
mod fixtures;
mod ledger;
mod rpc;

pub use context::Context;
pub use errors::{assert_error, expect_decoded};
pub use fixtures::{id, payload};
pub use ledger::{assert_created, assert_transaction, created, raw_created, strings, transaction};
pub use rpc::{endpoint, ok, within};

/// Apply the same deadline to RPC calls and stream consumption.
pub const LIMIT: Duration = Duration::from_secs(30);

/// Preload the fixture DAR into every sandbox, with Canton selected independently.
pub fn sandbox_config() -> SandboxConfig {
    static CONFIG: OnceLock<SandboxConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            SandboxConfig::builder()
                .with_dar(include_bytes!(concat!(env!("OUT_DIR"), "/fixtures.dar")).as_slice())
                .build()
                .expect("valid integration sandbox configuration")
        })
        .clone()
}
