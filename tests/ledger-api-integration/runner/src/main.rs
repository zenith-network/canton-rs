//! Run the Ledger API integration suite against a disposable Canton participant.

mod cancellation;
mod config;
mod error;
mod output;
mod process;
mod run;
mod sandbox;

use std::{env, process::ExitCode};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let filters = env::args_os().skip(1).collect::<Vec<_>>();
    match run::run(&filters).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
