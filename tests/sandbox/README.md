# Canton sandbox harness

In-memory Canton sandboxes for async integration tests, with sharing, capacity, and process cleanup
managed internally.

## Example

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

## Usage notes

Place `sandbox` above `tokio::test` or `should_panic`. Tests with `sandbox` must take one argument
of type `&Sandbox`.

Tests which are using `shared` sandbox may share ledger state within a single group.
They are usually faster to run, because they don't need to wait for a fresh sandbox to start,
but they must be written in a way, which tolerates possible state changes by other tests
of the same group.

If no group name was specified, a default group (named `"0"`) will be used.

## Harness configuration

You can use environment variables to tweak harness settings:

| Compile time variable | Default | Description |
| --- | --- | --- |
| `SANDBOX_BUILD_CANTON_VERSION` | `canton-version` crate's pin | Canton release to run against |

Sandbox harness is built against a specific version of Canton (not Daml SDK!). This version
is defined at compilation time of the crate and cannot be changed in runtime. This allows to write
tests for a specific Canton versions:

```rust
#[sandbox(canton_version = "^3.5.12", shared)]
#[tokio::test]
async fn test_for_3_5_12(sandbox: &sandbox::Sandbox) {}
```

If the test requirements don't match with the configured harness, the test will be ignored.

Following values can be set at runtime:

| Runtime variable | Default | Description |
| --- | --- | --- |
| `SANDBOX_CAPACITY` | `2` | Max number of concurrent sandboxes allowed to run |
| `SANDBOX_IDLE_TIMEOUT_MS` | `3000` | How long to wait before destroying sandbox |
| `DPM` | `"dpm"` | Path to `dpm` executable |

For custom settings in runtime, use a provider function returning configuration:

```rust
fn sandbox_config() -> sandbox::SandboxConfig {
    sandbox::SandboxConfig::builder()
        .max_sandboxes(3)
        .with_dar(include_bytes!("my_package.dar").as_slice())
        .build()
        .expect("valid sandbox configuration")
}

#[sandbox::sandbox(shared, config = sandbox_config)]
#[tokio::test]
async fn test_with_custom_config(sandbox: &sandbox::Sandbox) {
    // Acquisition and cleanup of the sandbox remain automatic.
}
```

Environment variables take precedence over runtime-defined settings defined through
`SandboxConfig::builder()`.

> IMPORTANT NOTE: Currently the sandboxes pool supports only one configuration per process. This
> means that if you're using custom config through `SandboxConfig::builder()`, you **must** pass it
> to all of your tests. Otherwise the pool will fail to initialize.

## Runtime dependencies

- DPM
- JRE (Java Runtime Environment)
- Canton `open-source` component for DPM

If you set `DPM_AUTO_INSTALL=true` at runtime, you don't need to install Canton component manually.
It will be fetched automatically on the first access.

## What harness does

The harness starts Canton, waits for readiness, and passes a sandbox to each test.
Shared tests reuse ledger state within their group; fresh tests get a new instance.
A bounded pool of sandboxes manages both.

Fresh instances stop after their test. Idle shared instances stop after the idle
timeout or sooner to free capacity. Cleanup also runs after errors and panics and
on SIGINT/SIGTERM, force-killing unresponsive process groups if needed.

## Limitations

- Currently only `async` tests are supported
