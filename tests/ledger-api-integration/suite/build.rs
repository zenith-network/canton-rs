use prost::Message;
use std::{env, fs::File, io::Read, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=CANTON_TEST_DAR");
    let dar = PathBuf::from(env::var_os("CANTON_TEST_DAR").expect(
        "Run cargo run -p runner from tests/ledger-api-integration to build the fixture DAR first",
    ));
    println!("cargo:rerun-if-changed={}", dar.display());
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
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("main-package-payload.bin"), dalf.payload).unwrap();
    println!("cargo:rustc-env=FIXTURE_PACKAGE_ID={}", dalf.hash);
}
