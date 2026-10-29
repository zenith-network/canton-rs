use sandbox::{Sandbox, SandboxConfig, SandboxConfigBuilder, sandbox};

fn config() -> SandboxConfig {
    let builder: SandboxConfigBuilder = SandboxConfig::builder()
        .canton_version("3.5.12")
        .max_sandboxes(3);
    builder.build().unwrap()
}

#[sandbox(shared, group = "configured", config = config)]
async fn configured(sandbox: &Sandbox) {
    assert!(!sandbox.id().is_empty());
}

fn main() {
    drop(configured());
}
