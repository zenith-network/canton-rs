use std::{env, path::PathBuf};

use canton::{codegen, dpm_build::Config as DpmConfig};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap();

    let dar = DpmConfig::default()
        .package_root(workspace_dir)
        .build()
        .expect("should be able to build Daml code")
        .output;

    codegen::generate(dar, Default::default()).expect("should be able to generate bindings");
}
