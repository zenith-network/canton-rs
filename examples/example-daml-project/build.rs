use canton::{codegen, dpm_build};

fn main() {
    // TODO: this should be part of dpm_build
    println!("cargo::rerun-if-changed=daml");

    let result = dpm_build::build().unwrap();
    let dar = result.output;

    println!("cargo::rerun-if-changed={}", dar.display());
    codegen::generate(dar, Default::default()).expect("should be able to generate bindings");
}
