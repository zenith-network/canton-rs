//! Shared setup and assertions for the real-participant suite.
//!
//! Context creates isolated test identities and clients. Fixtures provide Daml
//! values, RPC helpers enforce deadlines, and comparisons check SDK results
//! against the participant's protobuf responses.

use std::time::Duration;

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
