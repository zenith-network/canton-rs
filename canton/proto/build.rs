use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));

    let source_dir = manifest_dir.join("protobuf");

    let protos = collect_protos(&source_dir);

    if protos.is_empty() {
        // TODO: leave a comment about why is it needed to exit early here
        return;
    }

    let includes = vec![source_dir, manifest_dir.join("vendored")];

    let mut config = tonic_prost_build::Config::new();
    config.enable_type_names();

    // Keep the public map types identical with and without std.
    config.btree_map(["."]);

    tonic_prost_build::configure()
        .build_client(cfg!(feature = "client"))
        .build_server(cfg!(feature = "server"))
        .build_transport(cfg!(feature = "transport"))
        .compile_with_config(config, &protos, &includes)
        .expect("failed to generate code from Protobuf sources")
}

fn collect_protos(source_dir: &Path) -> Vec<PathBuf> {
    let escaped_source_dir = glob::Pattern::escape(&source_dir.to_string_lossy());
    glob::glob(&format!("{escaped_source_dir}/**/*.proto"))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
