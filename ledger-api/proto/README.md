# ledger-api-proto

This crate defines the Protobuf types and services of the Ledger API.

## Revision

Git commit SHA is written in `./rev` of Canton source repository.
Protobuf files are taken from `CANTON/community/ledger-api-proto/src/main/protobuf`.

`vendored` defines dependencies taken from the same repository:
`CANTON/community/lib/google-common-protos-scala/target/protobuf_external`.
They are vendored for now to avoid conflicts with versions used by Canton.
