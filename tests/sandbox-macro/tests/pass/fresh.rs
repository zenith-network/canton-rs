use sandbox::{Sandbox, sandbox};

#[sandbox(fresh)]
async fn fresh(sandbox: &Sandbox) {
    assert!(!sandbox.id().is_empty());
}

fn main() {
    drop(fresh());
}
