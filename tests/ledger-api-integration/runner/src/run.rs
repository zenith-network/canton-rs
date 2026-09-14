use std::ffi::OsString;

use crate::{
    cancellation::Cancellation,
    config::Config,
    error::{Result, after_cleanup},
    process, sandbox,
};

pub(crate) async fn run(filters: &[OsString]) -> Result<()> {
    let cancellation = Cancellation::install()?;
    let config = Config::new()?;
    println!("Artifacts and logs: {}", config.logs.display());
    let outcome = execute(&config, &cancellation, filters)
        .await
        .and_then(|()| cancellation.check());
    if outcome.is_err() {
        eprintln!("Logs: {}", config.logs.display());
    }
    outcome
}

async fn execute(config: &Config, cancellation: &Cancellation, filters: &[OsString]) -> Result<()> {
    process::run(
        config.fixture_build(),
        &config.logs.join("daml-build.log"),
        cancellation,
    )
    .await?;
    process::run(
        config.suite_build(),
        &config.logs.join("cargo-build.log"),
        cancellation,
    )
    .await?;
    let sandbox = sandbox::start(config, cancellation).await?;
    let outcome = process::run(
        config.suite_tests(filters, &sandbox.endpoint),
        &config.logs.join("tests.log"),
        cancellation,
    )
    .await;
    let cleanup = sandbox.shutdown().await;
    if cleanup.is_ok() {
        println!("Sandbox and test processes stopped.");
    }
    after_cleanup(outcome, cleanup)
}
