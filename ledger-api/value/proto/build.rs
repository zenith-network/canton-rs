use std::{env, path::PathBuf};

extern crate prost_build;

pub fn main() {
    let protos = get_protos();
    if !protos.is_empty() {
        let mut prost_build = prost_build::Config::new();
        prost_build.enable_type_names();
        prost_build
            .compile_protos(&protos, &["proto"])
            .expect("failed to generate Rust code from protobuf");
    }
}

fn get_protos() -> Vec<PathBuf> {
    let mut protos = Vec::new();

    if env::var_os("CARGO_FEATURE_V2").is_some() {
        protos.push(PathBuf::from("proto/com/daml/ledger/api/v2/value.proto"));
    }

    protos
}
