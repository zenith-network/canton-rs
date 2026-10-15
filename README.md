# Canton Rust SDK

WIP

## Synchronization with Canton

We use [`sync-upstream`](https://github.com/zenith-network/sync-upstream) to get Protobuf files from
Canton repository.

Canton revision is specified through `upstream.toml`.

After updating `upstream.toml`, run:

```sh
sync-upstream --all --dry-run
```

and

```sh
sync-upstream --all
```
