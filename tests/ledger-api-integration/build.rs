use prost::Message;
use std::{env, fs::File, io::Read, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=DPM");
    println!("cargo:rerun-if-env-changed=DPM_SDK_VERSION");
    println!("cargo:rerun-if-changed=fixtures");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let fixtures = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("fixtures");
    // Keep Daml's compiler cache alongside Cargo's build artifacts.
    let package = out.join("fixtures");
    std::fs::create_dir_all(package.join("daml")).unwrap();
    std::fs::copy(fixtures.join("daml.yaml"), package.join("daml.yaml")).unwrap();
    std::fs::copy(
        fixtures.join("daml/Integration.daml"),
        package.join("daml/Integration.daml"),
    )
    .unwrap();
    let dar = dpm_build::Config::default()
        .package_root(&package)
        .output(out.join("fixtures.dar"))
        .disable_multi_package()
        .build()
        .expect("build fixture DAR")
        .output;
    daml_lf_codegen::generate(&dar, Default::default()).expect("generate fixture bindings");

    // Keep an independent copy of the archive payload for GetPackage comparisons.
    let mut archive = zip::ZipArchive::new(File::open(&dar).unwrap()).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("META-INF/MANIFEST.MF")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest = manifest.replace("\r\n", "\n").replace("\n ", "");
    let main = manifest
        .lines()
        .find_map(|line| line.strip_prefix("Main-Dalf: "))
        .unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(main)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let dalf = daml_lf_archive_proto::com::digitalasset::daml::lf::archive::Archive::decode(
        bytes.as_slice(),
    )
    .unwrap();
    std::fs::write(out.join("main-package-payload.bin"), dalf.payload).unwrap();
    println!("cargo:rustc-env=FIXTURE_PACKAGE_ID={}", dalf.hash);
}
