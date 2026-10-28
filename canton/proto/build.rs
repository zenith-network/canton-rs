use std::{
    env,
    path::{Path, PathBuf},
};

use tonic_prost_build::Builder;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));

    let source_dir = manifest_dir.join("protobuf");

    let protos = collect_protos(&source_dir);

    if protos.is_empty() {
        // TODO: leave a comment about why is it needed to exit early here
        return;
    }

    // canton-proto uses some types
    let admin_api_proto = PathBuf::from(
        env::var("DEP_ADMIN_API_PROTO_INCLUDE_DIR")
            .expect("DEP_ADMIN_API_PROTO_INCLUDE_DIR should be set"),
    );

    let includes = vec![source_dir, manifest_dir.join("vendored"), admin_api_proto];

    let mut config = tonic_prost_build::Config::new();
    config.enable_type_names();

    // Keep the public map types identical with and without std.
    config.btree_map(["."]);

    let mut builder = tonic_prost_build::configure()
        .build_client(cfg!(feature = "client"))
        .build_server(cfg!(feature = "server"))
        .build_transport(cfg!(feature = "transport"));

    builder = extern_paths(builder);

    builder
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

// Extern paths from admin-api-proto crate
fn extern_paths(builder: Builder) -> Builder {
    builder
        .extern_path(
            ".com.digitalasset.canton.topology.admin.v30.TopologyTransactions",
            "::admin_api_proto::com::digitalasset::canton::topology::admin::v30::TopologyTransactions",
        )
        .extern_path(
            ".com.digitalasset.canton.topology.admin.v30.Synchronizer",
            "::admin_api_proto::com::digitalasset::canton::topology::admin::v30::Synchronizer",
        )
        .extern_path(
            ".com.digitalasset.canton.topology.admin.v30.StoreId",
            "::admin_api_proto::com::digitalasset::canton::topology::admin::v30::StoreId",
        )
}
