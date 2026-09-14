# Current state of the SDK and Canton

These findings come from the original source review on 7 October 2026. The review did not run the
SDK test suites.

The SDK already separates Ledger API, Admin API, common types, values, LF reading, and code
generation. We should keep those divisions and fix the dependencies between them.

## Crates and generated schemas

The root workspace has 20 crates, all at `0.1.0`. Internal dependencies use local paths without
registry version requirements. Before publishing, those dependencies need version requirements too.
See the [workspace manifest][s1].

The raw bindings expose Ledger API `v2`, Admin API `v30`, and Canton message namespaces `v30`,
`v31`, and `v32`. These names describe APIs or message schemas. They do not mean that the SDK
supports Canton protocols 30, 31, and 32. See the [generated Canton modules][s2].

Canton/Admin schemas are pinned to commit `eaa9e7a4bf48793acb35aba270b85a970afe6006`.
Ledger/value/LF archive schemas are pinned to `0dc3b765a90326fa887830236417ce0187ecb7fc`. We need to
record both sources and test the combination.

Some generation scripts find schemas in sibling crate directories. A crate downloaded from crates.io
cannot assume those directories exist. See the [Ledger generation script][s3] and the publication
changes in [releases and upgrades][s4].

## Version and capability handling

| Finding | Required change |
| --- | --- |
| LF compatibility and ordering have `todo!()`. | Finish these; check support per operation. |
| Protocol identifiers accept any `i32`. | Check recognition and enabled support separately. |
| DALF reading drops archive `patch`. | Keep and check it before decoding and generation. |
| Stable and preview LF support share a list. | Require explicit preview permission. |
| Admin conversions contain `todo!()`. | Mark unfinished operations as unavailable. |

Sources: [LF version types][s5], [protocol version type][s6], [DALF reader][s7], [LF support
list][s8], [Admin protocol conversions][s9], and [Admin crypto conversions][s10].

Connecting a Ledger client currently establishes transport. It does not check server capabilities.
The feature conversion also reads command-inspection support from the static-time field and treats
several optional descriptors as required. These need fixing before client decisions depend on them.
See the [client builder][s11] and [feature conversion][s12].

## Tests and newer Canton releases

Root CI installs Daml SDK `3.5.9`. The real-participant suite uses a separate workspace and pins
`3.6.1-snapshot.20260924.39.0.v85561690`, with LF target `2.3`. Root CI does not invoke that suite's
runner. This does not yet give us a list of tested released servers. See [root CI][s13] and
[integration configuration][s14].

The reviewed Canton checkout is commit `eaa9e7a4...`, with `VERSION` set to `3.6.0-SNAPSHOT`. Its
[versioning explanation][s15] describes the separate version families, but its concrete tables
belong to that checkout.

Released Canton 3.6.1 adds a useful test case for this proposal: it supports protocol 36, stable LF
2.4, and hashing scheme 4 for external-call transactions. The SDK's Ledger schema currently exposes
hashing schemes 2 and 3, while the LF support list contains `2.4-rc1`. We must review those
separately before claiming the new operations work. [Canton 3.6.1 release notes][s16]

[Back to the overview][s17]

[s1]: ../../Cargo.toml
[s2]: ../../canton/proto/src/lib.rs
[s3]: ../../ledger-api/proto/build.rs
[s4]: releases-and-upgrades.md
[s5]: ../../daml-lf/version/src/lib.rs
[s6]: ../../canton/types/src/topology/protocol_version.rs
[s7]: ../../daml-lf/src/dalf.rs
[s8]: ../../daml-lf/archive-proto/src/lib.rs
[s9]: ../../admin-api/types/src/protocol/v30/mod.rs
[s10]: ../../admin-api/types/src/crypto/v30/mod.rs
[s11]: ../../ledger-api/src/grpc/v2/client.rs
[s12]: ../../ledger-api/types/src/v2/version.rs
[s13]: ../../.github/workflows/rust-checks.yaml
[s14]: ../../tests/ledger-api-integration/runner/src/config.rs
[s15]: ../../../canton/versioning.md
[s16]: https://github.com/digital-asset/canton/releases/tag/v3.6.1
[s17]: README.md
