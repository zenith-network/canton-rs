#[sandbox::sandbox(fresh)]
#[should_panic]
async fn result(_sandbox: &sandbox::Sandbox) -> Result<(), String> {
    Ok(())
}

#[sandbox::sandbox(fresh)]
#[should_panic(expected = 42)]
async fn invalid_expected(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
#[should_panic]
#[should_panic]
async fn repeated_attribute(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
#[should_panic = 42]
async fn invalid_name_value(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
#[should_panic()]
async fn missing_expected(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
#[should_panic(expected = "first", expected = "second")]
async fn repeated_expected(_sandbox: &sandbox::Sandbox) {}

fn main() {}
