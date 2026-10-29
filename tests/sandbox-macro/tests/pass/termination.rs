use sandbox::{Sandbox, sandbox};
use std::process::{ExitCode, Termination};

struct Status;

impl Termination for Status {
    fn report(self) -> ExitCode {
        ExitCode::SUCCESS
    }
}

#[sandbox(fresh)]
async fn exit_code(_sandbox: &Sandbox) -> ExitCode {
    ExitCode::SUCCESS
}

#[sandbox(shared)]
async fn custom_status(_sandbox: &Sandbox) -> Status {
    Status
}

fn main() {
    drop(exit_code());
    drop(custom_status());
}
