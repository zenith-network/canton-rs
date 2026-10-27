use std::{
    env,
    path::{Path, PathBuf},
};

use glob::{Pattern, glob};
use tonic_prost_build::{Builder, Config};

pub fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo"),
    );

    let source_dir = manifest_dir.join("protobuf");

    let protos = collect_protos(&source_dir);

    if protos.is_empty() {
        return;
    }

    let includes = collect_include_dirs(&manifest_dir, source_dir);

    let mut config = Config::new();
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
        .expect("failed to generate code from Protobuf sources");
}

fn collect_protos(source_dir: &Path) -> Vec<PathBuf> {
    let mut protos = Vec::new();

    if env::var_os("CARGO_FEATURE_V2").is_some() {
        let source_dir_v2 =
            Pattern::escape(&source_dir.join("com/daml/ledger/api/v2").to_string_lossy());
        let patterns = vec![
            format!("{source_dir_v2}/*.proto"),
            format!("{source_dir_v2}/admin/**/*.proto"),
            format!("{source_dir_v2}/interactive/**/*.proto"),
        ];

        for pattern in patterns {
            for path in glob(&pattern).unwrap() {
                protos.push(path.unwrap());
            }
        }

        if env::var_os("CARGO_FEATURE_V2_TESTING").is_some() {
            for path in glob(&format!("{source_dir_v2}/testing/**/*.proto")).unwrap() {
                protos.push(path.unwrap());
            }
        }
    }

    protos
}

fn collect_include_dirs(manifest_dir: &Path, source_dir: PathBuf) -> Vec<PathBuf> {
    let ledger_api_value_proto_dir = manifest_dir
        .parent()
        .expect("cannot locate ledger-api-value-proto (parent directory doesn't exist)")
        .join("value/proto/proto");
    let vendored_dir = manifest_dir.join("vendored");
    vec![source_dir, vendored_dir, ledger_api_value_proto_dir]
}

fn extern_paths(builder: Builder) -> Builder {
    // we can't extern the whole package, because it has the same package name
    builder
        .extern_path(
            ".com.daml.ledger.api.v2.Value",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Value",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.Record",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Record",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.RecordField",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::RecordField",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.Identifier",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Identifier",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.Variant",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Variant",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.Enum",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Enum",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.List",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::List",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.Optional",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::Optional",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.TextMap",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::TextMap",
        )
        .extern_path(
            ".com.daml.ledger.api.v2.GenMap",
            "::ledger_api_value_proto::com::daml::ledger::api::v2::GenMap",
        )
}
