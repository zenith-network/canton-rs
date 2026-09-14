# Implementation order and required tests

Start with accurate support information and broken checks. Move code after its owners and
dependencies are clear. A new directory layout alone will not make an unfinished decoder safe to
use.

## Step 1: make current support explicit

- Inventory schema revisions and implemented operations.
- Finish LF parsing and ordering; replace generic compatibility checks with operation checks.
- Preserve and check archive patches.
- Separate stable LF acceptance from preview acceptance.
- Fix feature conversion and panic paths used by version checks.
- Mark unfinished Admin and signing operations as unavailable.
- Make the integration runner accept independently selected compiler and server versions.

Finish this step when every advertised operation has a support record and unsupported inputs produce
an error or an explicit unknown result. For example, an unsupported LF dependency must name the
package that prevented generation.

## Step 2: separate code ownership

Add `canton-version`, `canton-compat`, and `canton-protocol`. Move protocol models out of Admin
types. Give shared generated messages one owner and move topology service bindings to Admin without
creating crate cycles.

Make generated crate packages self-contained. Keep compatible old paths as re-exports during
migration. Finish this step when a program can read a DAR or use a `PackageId` without compiling a
network client, and packaged crates build without sibling directories.

## Step 3: make several versions usable together

Add checked client connection, per-operation requirements, and per-synchronizer protocol contexts.
Add generated interface requirements and explicit handling of unknown event data. Generate support
documentation from the same records used by the SDK.

Finish this step when one application can use the supported old and new server combinations at the
same time. For example, it should hold two clients with different capability reports without one
connection changing the other's codec choices.

## Step 4: test released Canton versions and prepare 1.0

Assess released Canton 3.5 and 3.6 lines first; retain 3.4 if users or a network upgrade require it.
Choose minimum patches after reviewing behavior and known problems. The current `3.5.9` compiler
fixture is not evidence that it should be the server support floor.

Review released 3.6.1 schema and behavior changes. Implement and test LF 2.4 generation if required.
Add scheme-4 signing only if signing is part of the SDK product. Do not advertise it merely because
its enum was imported.

Publish tested bundles, minimum Rust version, supported platforms, removal dates, and user migration
instructions. A 1.0 release should make promises the release tests actually cover.

## Tests that must support those promises

| Test | Concrete failure it should catch |
| --- | --- |
| Public Rust API comparison | A new generated field breaks existing struct literals. |
| Minimal and combined features | Enabling two codecs makes the crate fail to compile. |
| Packaged-crate build in isolation | `build.rs` reads a missing sibling schema directory. |
| Minimum Rust and supported target builds | A dependency update raises the compiler requirement. |
| Codec byte fixtures and negative cases | An unknown schema falls back to the newest decoder. |
| LF version and patch fixtures | A construct is absent from the declared LF version. |
| Mixed-version DAR generation | The main package passes but an unsupported dependency is ignored. |
| Generated code with runtime versions | A trait is absent from the oldest promised runtime. |
| Released-server Ledger and Admin tests | A declared method or response conversion does not work. |
| Mixed-server rollout tests | Discovery from a new backend enables writes on an old one. |
| Synchronizer upgrade tests | A cached context survives a physical generation change. |
| Signing vectors, when implemented | Rust and Canton produce different digests. |
| Malformed and bounded input tests | Recursive data causes a panic or unbounded allocation. |

Test renamed runtime dependencies too, and verify that forwarding signed data preserves its original
bytes.

Do not test every possible combination of releases and features. Choose the oldest supported
behavior, newest supported release, each codec or LF transition, known exceptions, and real upgrade
paths. Publish the exact combinations tested.

Record compiler and server artifacts separately. A fixture compiled with one toolchain can run on a
different compatible server. Use explicit LF targets and artifact hashes where available.

Make the release workflow invoke the separate integration runner. A successful root-workspace test
job does not currently mean the real-participant tests ran.

[Back to the overview][s1]

[s1]: README.md
