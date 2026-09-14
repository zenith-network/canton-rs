use std::fs;

use semver::Version;

fn main() {
    println!("cargo::rerun-if-changed=CANTON_VERSION");
    let contents = fs::read_to_string("CANTON_VERSION").expect("failed to read CANTON_VERSION");
    let version = Version::parse(contents.trim())
        .expect("CANTON_VERSION must contain a valid SemVer version");
    println!("cargo::rustc-env=CANTON_VERSION={version}");
}
