//! Shared sandbox access and bounded RPC assertions.

use std::time::Duration;

pub use sandbox::{Sandbox, sandbox};

mod rpc;

pub use rpc::ok;

/// Apply the same deadline to connections and RPC calls.
const LIMIT: Duration = Duration::from_secs(30);
