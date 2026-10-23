# Canton Rust SDK

> NOTE: This SDK is under active development.

## Build dependencies

Required to build SDK:

- Rust
- `protoc`

```sh
cargo build
```

## Development dependencies

In addition to build dependencies, required to run tests:

- Java runtime
- DPM, Daml SDK

```sh
cargo build --workspace
cargo test --workspace --all-features
```

## Synchronization with Canton

We use [`sync-upstream`](https://github.com/zenith-network/sync-upstream) to get Protobuf files from
Canton repository. Canton revision is specified through `upstream.toml`.

After updating `upstream.toml`, run:

```sh
sync-upstream --all --dry-run
```

and

```sh
sync-upstream --all
```
