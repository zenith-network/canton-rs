//! Shared sandbox access and bounded RPC assertions.

use std::time::Duration;

pub use sandbox::{Sandbox, sandbox};

mod rpc;
pub mod topology;

pub use rpc::{ok, within};

/// Apply the same deadline to connections and RPC calls.
const LIMIT: Duration = Duration::from_secs(30);
