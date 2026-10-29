# Canton Ledger API integration tests

These tests are aiming to test SDK against a real Canton participant.
They use [Canton sandbox harness](../sandbox/README.md) and fixture Daml contracts.
The tests will run only against a specified version of the participant.

## Dependencies

To build fixture Daml contracts, Daml SDK is needed. We're using the latest stable SDK release by
default. If you want to compile fixtures with different SDK or for different Daml LF target, use
the following environment variables during build:

| Compile-time variable | Default | Description |
| --- | --- | --- |
| `DPM_SDK_VERSION` | specified in `daml.yaml` | Version of SDK used to build Daml package |
| `DAML_LF_TARGET` | `2.3` | Version of Daml LF for which Daml package is compiled |

See also [runtime dependencies](../sandbox/README.md#runtime-dependencies) of the harness.

> NOTE: Daml SDK version and Canton version are not the same thing. These versions sometimes match,
> which may cause confusion (e.g. Daml SDK 3.6.2 provides Canton 3.6.2). However they are generally
> independent. E.g. Daml SDK 3.5.12 provides Canton 3.5.19.
> We use some version of SDK here (typically latest stable) to build the contracts, however tests
> are supposed to be run against various versions of Canton nodes. The compatibility requirement is
> that Canton node must support used Daml LF target.

## Running

```sh
cargo test -p ledger-api-integration --test ledger_api_integration
```

This will run all Ledger API integration tests for selected Canton version

By default Canton version pinned in `canton-version` crate is used. To run against a different
Canton version, specify `SANDBOX_BUILD_CANTON_VERSION`:

```sh
SANDBOX_BUILD_CANTON_VERSION=3.5.19 cargo test -p ledger-api-integration --test ledger_api_integration
```

## Coverage notes

What is not covered by these tests (at least right now):

- authentication
- TLS
- restarts
- reassignment/multiple synchronizers
- malformed protobuf inputs/outputs
