#[sandbox::sandbox(fresh)]
async fn unsupported_output(_sandbox: &sandbox::Sandbox) -> u32 {
    42
}

fn main() {}
