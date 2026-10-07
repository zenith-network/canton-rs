# Versioning for the Canton Rust SDK

This proposal explains how to release SDK crates, support several Canton versions, and organize the
code. It describes planned changes, not support already implemented in the SDK. The source analysis
was made on 7 October 2026.

The main recommendation is to give each Rust crate its own version and publish a separate list of
what it can do with Canton. For example, `ledger-api` version `1.4.0` could support command
submission on both Canton 3.5 and 3.6. Its crate version does not need to match either server
version. That example is hypothetical.

Start with the version model and code organization. The other documents cover specific
implementation and release questions.

| Document | Question it answers |
| --- | --- |
| [Current state](current-state.md) | What does the SDK already have, and what needs fixing? |
| [Version model](version-model.md) | Which things have versions, and which can coexist? |
| [Code organization](code-organization.md) | Which crates and modules should own each part? |
| [Supported versions](supported-versions.md) | What support should we publish? |
| [Runtime checks](runtime-checks.md) | How does a client check a particular server? |
| [Messages and signing](protocol-and-signing.md) | How do we choose codecs and keep signed bytes? |
| [LF and codegen](lf-and-codegen.md) | How do we handle LF and package upgrades? |
| [Releases and upgrades](releases-and-upgrades.md) | How do we release and upgrade? |
| [Implementation plan](implementation-plan.md) | What do we build and test first? |

Three rules run through the proposal:

- Check the operation the user needs. Reading a DAR and generating Rust bindings from it can have
  different version limits.
- Keep the relevant version with the data. A topology transaction needs its message schema and
  protocol context; a DALF needs its LF version and archive patch.
- Publish what was tested. Having a protobuf definition in the repository does not prove that the
  high-level client works against a released server.
