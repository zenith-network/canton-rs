# Canton Ledger API integration tests

These tests are aiming to test SDK against a real Canton participant.
They use sandbox (`dpm sandbox`) and a bunch of fixture Daml contracts for that.
The tests will run only against a specified version of the participant.

## Prerequisites

- Rust/Cargo
- `protoc`
- Java runtime
- `dpm` with open-source SDK (note the SDK version in `daml.yaml`)

## Running

From this directory:

```sh
cargo run
```

This launches a runner, which will:

- Build Daml code into `*.dar`
- Build Rust testing suite (`./suite`) with Cargo
- Start `dpm sandbox`
- Run test suite agains the sandbox

Artifacts and logs are retained under the repository's
`target/ledger-api-integration/runs/<UTC timestamp>-<PID>/`; each run prints its path.
`tests.log` contains assertions and raw response diagnostics, `canton-*.log`
contains detailed Canton logs, and `probe.log` records readiness failures.

You can also run specific tests, e.g.:

```sh
cargo run -- commands
```

This "filter" arguments is passed as an argument to `cargo test`.

## Directory structure

- `fixtures` - contains Daml source code used in tests
- `runner` - runner implementation, which orchestrates the testing process
- `suite` - bindings generated for `fixtures` and test cases based on them

## Coverage notes

What is not covered by these tests (at least right now):

- authentication
- TLS
- restarts
- reassignment/multiple synchronizers
- malformed protobuf inputs/outputs
