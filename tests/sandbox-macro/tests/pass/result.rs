use sandbox::{Sandbox, sandbox};

#[sandbox(fresh)]
async fn result(sandbox: &Sandbox) -> Result<(), String> {
    Ok::<(), String>(())?;
    if !sandbox.id().is_empty() {
        return Ok(());
    }
    Err("missing sandbox id".into())
}

fn main() {
    drop(result());
}
