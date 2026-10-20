# Canton Ledger API integration tests

These tests are aiming to test SDK against a real Canton participant.
They use sandbox (`dpm sandbox`) and a bunch of fixture Daml contracts for that.
The tests will run only against a specified version of the participant.

For sandboxes we use a harness, see [`sandbox`](../sandbox/README.md) crate.

## Prerequisites

In addition to SDK build dependencies, required:

- Java runtime
- `dpm` with open-source SDK (note the SDK version in `daml.yaml`)

## Running

```sh
cargo test --test ledger_api_integration
```

This will run all Ledger API integration tests.

## Coverage notes

What is not covered by these tests (at least right now):

- authentication
- TLS
- restarts
- reassignment/multiple synchronizers
- malformed protobuf inputs/outputs
