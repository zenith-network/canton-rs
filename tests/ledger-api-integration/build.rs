use semver::Version;
use std::{env, path::PathBuf};

// Keep fixtures/daml.yaml's sdk-version in sync with this local default.
const DEFAULT_CANTON_VERSION: &str = "3.6.2";
// Register a threshold here before using #[cfg_attr(not(canton_since = "..."), ignore = "...")].
const CANTON_VERSION_THRESHOLDS: &[&str] = &["3.6.0"];

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"));
    let outdir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"));

    let fixtures = manifest_dir.join("fixtures");

    let build = dpm_build::Config::default()
        .package_root(&fixtures)
        .output(outdir.join("fixtures.dar"))
        .disable_multi_package()
        .build()
        .expect("build fixture DAR");
    println!(
        "cargo::rustc-env=FIXTURE_DAR_PATH={}",
        build.output.display()
    );

    let default = Version::parse(DEFAULT_CANTON_VERSION)
        .expect("DEFAULT_CANTON_VERSION must be valid SemVer");
    let fixture_version = build
        .manifest
        .expect("fixture build must return a readable daml.yaml manifest")
        .sdk_version
        .expect("fixtures/daml.yaml must define sdk-version");
    assert_eq!(
        fixture_version, DEFAULT_CANTON_VERSION,
        "fixtures/daml.yaml sdk-version differs from DEFAULT_CANTON_VERSION; \
         update both defaults together"
    );
    let version = match env::var("DPM_SDK_VERSION") {
        Ok(version) => Version::parse(&version)
            .unwrap_or_else(|error| panic!("invalid DPM_SDK_VERSION {version:?}: {error}")),
        Err(env::VarError::NotPresent) => default,
        Err(error) => panic!("DPM_SDK_VERSION must be valid SemVer: {error}"),
    };
    println!("cargo:rustc-env=TEST_CANTON_SDK_VERSION={version}");
    for &threshold in CANTON_VERSION_THRESHOLDS {
        let minimum = Version::parse(threshold).expect("Canton threshold must be valid SemVer");
        println!("cargo::rustc-check-cfg=cfg(canton_since, values({threshold:?}))");
        if version >= minimum {
            println!("cargo::rustc-cfg=canton_since={threshold:?}");
        }
    }

    daml_lf_codegen::generate(&build.output, Default::default())
        .expect("generate fixture bindings");
}
