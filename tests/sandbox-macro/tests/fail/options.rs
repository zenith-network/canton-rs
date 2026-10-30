#[sandbox::sandbox()]
async fn missing_mode(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(shared, fresh)]
async fn conflicting_modes(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(shared, shared)]
async fn repeated_mode(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(shared, group = "")]
async fn empty_group(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh, group = "my-group")]
async fn group_with_fresh(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(shared, group = "a", group = "b")]
async fn repeated_group(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh, config = config, config = config)]
async fn repeated_config(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(unknown)]
async fn unknown_option(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh, canton_version = "latest")]
async fn invalid_version(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh, canton_version = "^3", canton_version = "^4")]
async fn repeated_version(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh, canton_version = 42)]
async fn non_string_version(_sandbox: &sandbox::Sandbox) {}

fn main() {}
