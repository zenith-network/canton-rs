# How to publish supported versions

Yes, each crate should publish a list of supported versions. Each entry must name the operation it
applies to.

For example, “LF 2.3 supported” is incomplete. It could mean that we can read the package header,
decode its types, generate Rust bindings, or execute its code. The Rust SDK does not currently
implement a Daml execution engine.

## Publish three different views

| View | Example question |
| --- | --- |
| Release support | Does this crate release implement schema-30 decoding? |
| Compiled support | Was that decoder included in this application's build? |
| Deployment result | Can it decode this message in its protocol-35 context? |

A Cargo feature answers only the second question. A server capability answers part of the third.
Neither proves that the SDK implements the operation correctly.

Keep support declarations beside the code changes that add or remove support. Generate the human
documentation and Rust support constants from those declarations. Each domain should own its
records; `canton` can combine them for users of the facade.

## What an entry contains

| Field | Example |
| --- | --- |
| Owning crate and release | `canton-protocol`, a specific crate version |
| Operation | `topology_transaction.decode` |
| Format | Message schema `30` |
| Required context | Protocols `{34, 35}` |
| Build feature | `topology` |
| Support status | Stable or preview |
| Limits | Decoding only; no signing |
| Sources | A pinned Canton codec table |
| Test evidence | Byte fixtures and a named integration run |

Here is a proposed declaration format. It is an example, not current SDK support:

```yaml
manifest_schema: 1
crate: canton-protocol
crate_version: "<released version>"
upstream_bundle: "<pinned sources>"
operations:
  topology_transaction.decode:
    status: stable
    required_features: [topology]
    payload_schema: 30
    allowed_protocols: [34, 35]
    source: "<pinned Canton codec table>"
    tests: ["<byte fixture>", "<integration run>"]
```

The inspected Canton table maps topology transaction schema 30 to a protocol representative starting
at 34. That is a concrete example of different schema and protocol numbers. The SDK must still
review which protocol contexts it will promise to support. See [Canton's transaction codec
table][s1].

Use explicit sets for protocols, LF formats, and signing schemes. Use bounded server-release ranges
where the reviewed API behavior allows it, with exceptions for known incompatible patches. Record
exact tested server releases separately.

For example, a reviewed promise covering a Canton minor line can include an untested patch in that
line. The report must say that the patch falls within the promise, while listing which patches were
actually tested. It must not call that patch tested.

## Support and test evidence are separate fields

A record can reference a specification, unit tests, byte fixtures, and integration tests. Production
support needs tests for the claimed behavior. Compiling a schema or skipping an integration test
cannot produce a successful test record.

Keep preview status separate from evidence. A development format may have excellent tests but still
change in the next source revision. Pin preview support to the exact revision or artifact.

Protobuf descriptors tell us field shapes. They do not tell us every allowed protocol context or
server behavior. The declarations also need reviewed Canton codec tables, LF feature rules, and
release notes.

## Reports should explain the failed requirement

Expose functions equivalent to these; the names are proposed:

```text
compiled_support()
support_for(operation, context)
assess(requirements, observations, policy)
```

Return one of `compatible`, `incompatible`, or `unknown` for each requirement. Include the reason
and its source. For example:

```text
Operation: generate Rust bindings
Package: dependency P, LF 2.4, archive patch 0
Result: incompatible
Reason: this build only supports generation for LF 2.1, 2.2, and 2.3
```

Another useful result is “server feature information unavailable: permission denied.” That is
unknown support, not proof that the server lacks the feature.

A future CLI can inspect a DAR and a server, then emit the same report as JSON. Version the report
format so deployment tooling can keep reading it after SDK updates.

[Back to the overview][s2] · [Next: runtime checks][s3]

[s1]: ../../../canton/community/base/src/main/scala/com/digitalasset/canton/topology/transaction/TopologyTransaction.scala
[s2]: README.md
[s3]: runtime-checks.md
