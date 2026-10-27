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

/// Preload the fixture DAR into every sandbox using the fixture's SDK version.
pub fn sandbox_config() -> SandboxConfig {
    static CONFIG: OnceLock<SandboxConfig> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let version = env!("TEST_CANTON_SDK_VERSION");
            if let Some(runtime_version) = std::env::var_os("DPM_SDK_VERSION") {
                assert_eq!(
                    runtime_version,
                    std::ffi::OsStr::new(version),
                    "DPM_SDK_VERSION differs from the SDK used to build this test binary \
                     ({version}); select the version through \
                     DPM_SDK_VERSION=<version> cargo test -p ledger-api-integration \
                     --all-features --test ledger_api_integration so Cargo rebuilds the fixtures \
                     and minimum-version configuration"
                );
            }
            SandboxConfig::builder()
                .sdk_version(version)
                .with_dar(ledger_api_integration::DAR_BYTES)
                .build()
                .expect("valid integration sandbox configuration")
        })
        .clone()
}
