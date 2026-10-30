use std::{env, path::PathBuf};

/// Name of the env var, which handles the LF target selection
const LF_TARGET: &str = "DAML_LF_TARGET";

/// Default LF target we're using
// TODO: For now just stick with 2.3. We need to do some sort of discovery,
// based on version of Canton. E.g. Canton 3.5.X doesn't support 2.4, that's
// why we use 2.3 for now.
const DEFAULT_LF_TARGET: &str = "2.3";

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let outdir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"));

    let fixtures = manifest_dir.join("fixtures");

    let mut config = dpm_build::Config::default();
    config.package_root(&fixtures);
    config.output(outdir.join("fixtures.dar"));
    config.disable_multi_package();

    println!("cargo::rerun-if-env-changed={LF_TARGET}");
    if env::var_os(LF_TARGET).is_none() {
        config.env_var(LF_TARGET, DEFAULT_LF_TARGET);
    }

    let build_result = config.build().expect("build fixture DAR");
    println!(
        "cargo::rustc-env=FIXTURE_DAR_PATH={}",
        build_result.output.display()
    );

    daml_lf_codegen::generate(&build_result.output, Default::default())
        .expect("generate fixture bindings");
}
