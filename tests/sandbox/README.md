# Canton sandbox harness

In-memory Canton sandboxes for async integration tests, with sharing, capacity, and process cleanup
managed internally. Requires DPM, Java, and the Canton component; supports Linux and macOS.

```rust
use sandbox::{Sandbox, sandbox};

#[sandbox(shared)]
#[tokio::test]
async fn contract_test(sandbox: &Sandbox) {
    let endpoint = sandbox.ledger_api_endpoint();
    // Connect SDK clients and allocate unique parties for this test.
}

#[sandbox(shared, group = "my-group")]
#[tokio::test]
async fn grouped_test(sandbox: &Sandbox) {
    // Share ledger state with other tests in "my-group".
}

#[sandbox(fresh)]
#[tokio::test]
async fn isolated_test(sandbox: &Sandbox) {
    // A new sandbox, stopped after this test.
}
```

Place `sandbox` above `tokio::test`. Tests take one `&Sandbox` and return
`()` or `Result<(), E>` where `E: Debug`. Cleanup runs after success, a returned
error, or a body panic. `#[should_panic]` checks the body only; setup and cleanup
failures still fail the test.

`shared` uses the default group (`"0"`). Named groups have separate ledger state;
concurrent members of one group must tolerate each other's writes. Use `fresh`
for assertions requiring exact global state. Choose a mode explicitly.

There is one internal sandboxes pool per test process. All groups and fresh instances use that pool.
Separate test binaries have independent limits.

## Harness configuration

You can use environment variables to tweak harness settings:

| Variable | Default | Description |
| --- | --- | --- |
| `SANDBOX_CAPACITY` | `2` | Max number of concurrent sandboxes allowed to run |
| `SANDBOX_IDLE_TIMEOUT_MS` | `3000` | How long to wait before destroying sandbox |
| `SANDBOX_CANTON_VERSION` | `canton-version` crate's pin | Exact Canton server release |
| `DPM` | `"dpm"` | Path to `dpm` executable |

For custom settings in runtime, supply a function returning configuration:

```rust
fn sandbox_config() -> sandbox::SandboxConfig {
    sandbox::SandboxConfig::builder()
        .canton_version("3.5.12")
        .max_sandboxes(3)
        // .with_dar(include_bytes!("fixtures.dar").as_slice())
        .build()
        .expect("valid sandbox configuration")
}

#[sandbox::sandbox(shared, config = sandbox_config)]
#[tokio::test]
async fn configured_test(sandbox: &sandbox::Sandbox) {
    // Acquisition and cleanup remain automatic.
}
```

Runtime-defined settings take precedence over environment variables.

## What harness does

The harness starts Canton, waits for readiness, and passes a sandbox to each test.
Shared tests reuse ledger state within their group; fresh tests get a new instance.
A bounded pool manages both.

Fresh instances stop after their test. Idle shared instances stop after the idle
timeout or sooner to free capacity. Cleanup also runs after errors and panics and
on SIGINT/SIGTERM, force-killing unresponsive process groups if needed.
