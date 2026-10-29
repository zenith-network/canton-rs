# Canton Ledger API integration tests

These tests are aiming to test SDK against a real Canton participant.
They use sandbox (`dpm component run canton-open-source <version> -- sandbox`) and fixture Daml
contracts. The tests will run only against a specified version of the participant.

For sandboxes we use a harness, see [`sandbox`](../sandbox/README.md) crate.

## Prerequisites

In addition to SDK build dependencies, required:

- Java runtime
- `dpm` with a Daml SDK for fixture compilation and a Canton open-source component of desired
  version

## Running

```sh
cargo test -p ledger-api-integration --test ledger_api_integration
```

This will run all Ledger API integration tests for selected Canton version (defaults to the one
specified in `canton-version`).

## Coverage notes

What is not covered by these tests (at least right now):

- authentication
- TLS
- restarts
- reassignment/multiple synchronizers
- malformed protobuf inputs/outputs
