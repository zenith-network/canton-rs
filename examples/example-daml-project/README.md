# Example Daml project

This is an example of using Canton Rust SDK with Daml project.

- `daml` - Daml package sources
- `my-contracts` - Rust bindings crate for Daml code

## Build dependencies

- `dpm` + Daml SDK v3.6.1-snapshot.20260611.81.0.vb9b28f28 (for compiling Daml code)
- `protoc` (for Canton SDK)

## Build

`build.rs` in `my-contracts` will build Daml sources using DPM and generate Rust bindings
for produced .dar file.
