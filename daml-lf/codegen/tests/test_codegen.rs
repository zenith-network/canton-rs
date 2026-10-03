use daml_lf_codegen::{Config, generate};
use dpm_build::{Config as DpmConfig, MultiPackage};
use std::fs;
use tracing_test::traced_test;
use trybuild::TestCases;

#[test]
#[traced_test]
fn test_codegen_my_contracts() {
    let mut dpm = DpmConfig::default();
    dpm.multi_package(MultiPackage::Yes)
        .output("tests/assets/codegen/my-contracts.dar")
        .package_root("tests/assets/codegen/daml/my-contracts");
    let path = match dpm.build() {
        Ok(res) => res.output,
        Err(dpm_build::DpmError::DpmExecutionFailed(err))
            if err.kind() == std::io::ErrorKind::NotFound && std::env::var_os("DPM").is_none() =>
        {
            eprintln!("skipping test: default dpm binary not found in PATH");
            return;
        }
        Err(err) => panic!("should be able to build Daml: {err:?}"),
    };

    fs::create_dir_all("tests/assets/codegen/generated").unwrap();
    let mut config = Config::new();
    config.outdir("tests/assets/codegen/generated");

    let output = generate(path, config).expect("should be able to generate code");

    let test_path = output.main.with_file_name("compile_test.rs");
    fs::write(
        &test_path,
        format!("include!({:?});\nfn main() {{}}\n", output.main),
    )
    .expect("should be able to write compile test");

    let t = TestCases::new();
    t.pass(test_path);
}
