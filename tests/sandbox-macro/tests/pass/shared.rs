use sandbox::{Sandbox, sandbox};

#[sandbox(shared)]
async fn shared(sandbox: &Sandbox) {
    assert!(!sandbox.ledger_api_endpoint().is_empty());
}

#[sandbox(shared, group = "my-group")]
async fn grouped(sandbox: &Sandbox) {
    assert!(!sandbox.id().is_empty());
}

fn main() {
    // Construct futures without polling them, so no sandbox is started.
    drop(shared());
    drop(grouped());
}
