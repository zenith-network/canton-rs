# Example Daml project

This is an example of using Canton Rust SDK with Daml project.

- `daml` - Daml package sources
- `my-contracts` - Rust bindings crate for Daml code

## Build dependencies

- Rust
- `dpm` + Daml SDK (for compiling Daml code)
- `protoc` (for Canton SDK)

## Build

`build.rs` in `my-contracts` will build Daml sources using DPM and generate Rust bindings
for produced .dar file.

## Run

This example is not supposed to be executed. It's provided only as a demo code.
It has dummy values inside it and there is no participant provided to interact with.
