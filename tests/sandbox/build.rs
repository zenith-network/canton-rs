use std::env::{self, VarError};

use canton_version::semver::Version;

/// Var with this name is expected in build environment
const SB_BUILD_VER: &str = "SANDBOX_BUILD_CANTON_VERSION";

/// Var with this name is passed to rustc for harness code compilation
const HARNESS_BUILD_VER: &str = "SANDBOX_CANTON_VERSION";

fn main() {
    println!("cargo::rerun-if-env-changed={SB_BUILD_VER}");
    match env::var(SB_BUILD_VER) {
        Ok(version) => {
            if let Err(error) = Version::parse(&version) {
                panic!(
                    "failed to parse '{SB_BUILD_VER}': {error} ('{SB_BUILD_VER}' must be a valid SemVer)"
                );
            }

            println!("cargo::rustc-env={HARNESS_BUILD_VER}={version}");
        }
        Err(VarError::NotPresent) => {
            println!(
                "cargo::rustc-env={HARNESS_BUILD_VER}={}",
                canton_version::canton_version_str()
            );
        }
        Err(error) => panic!("'{SB_BUILD_VER}' is malformed: {error}"),
    }
}
