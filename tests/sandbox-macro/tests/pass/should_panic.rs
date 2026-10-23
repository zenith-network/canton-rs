use sandbox::{Sandbox, sandbox};

#[sandbox(fresh)]
#[should_panic]
async fn any_panic(_sandbox: &Sandbox) {
    panic!("body panic");
}

#[sandbox(fresh)]
#[should_panic(expected = "body panic")]
async fn expected_panic(_sandbox: &Sandbox) {
    panic!("body panic");
}

fn main() {
    drop(any_panic());
    drop(expected_panic());
}
