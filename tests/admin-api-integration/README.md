# Canton Admin API integration tests

These tests are aiming to test SDK against a real Canton participant.
They use [Canton sandbox harness](../sandbox/README.md).
The tests will run only against a specified version of the participant.

## Dependencies

See [runtime dependencies](../sandbox/README.md#runtime-dependencies) of the harness.

## Running

```sh
cargo test -p admin-api-integration --test admin_api_integration
```

This will run all Admin API integration tests for selected Canton version

By default Canton version pinned in `canton-version` crate is used. To run against a different
Canton version, specify `SANDBOX_BUILD_CANTON_VERSION`:

```sh
SANDBOX_BUILD_CANTON_VERSION=3.5.19 cargo test -p admin-api-integration --test admin_api_integration
```

## Coverage notes

What is **not** covered by these tests (at least right now):

- authentication
- TLS
- restarts
- reassignment/multiple synchronizers
- malformed protobuf inputs/outputs
