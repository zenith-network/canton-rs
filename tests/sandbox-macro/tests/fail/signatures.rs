#[sandbox::sandbox(fresh)]
fn synchronous(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
async fn missing_argument() {}

#[sandbox::sandbox(fresh)]
async fn by_value(_sandbox: sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
async fn mutable(_sandbox: &mut sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
async fn generic<T>(_sandbox: &sandbox::Sandbox) {}

#[sandbox::sandbox(fresh)]
async fn extra_argument(_sandbox: &sandbox::Sandbox, _extra: ()) {}

fn main() {}
