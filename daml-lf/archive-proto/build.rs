use std::path::PathBuf;

extern crate prost_build;

pub fn main() {
    let protos = collect_protos();
    let mut prost_build = prost_build::Config::new();
    prost_build.enable_type_names();
    prost_build
        .compile_protos(&protos, &["proto"])
        .expect("failed to generate Rust code from protobuf");
}

fn collect_protos() -> Vec<PathBuf> {
    let mut protos = vec![PathBuf::from(
        "proto/com/digitalasset/daml/lf/archive/daml_lf.proto",
    )];
    if std::env::var_os("CARGO_FEATURE_V2").is_some() {
        protos.push(PathBuf::from(
            "proto/com/digitalasset/daml/lf/archive/daml_lf2.proto",
        ));
    }
    protos
}
