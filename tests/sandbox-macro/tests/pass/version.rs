// This compile fixture constructs futures without running a test harness.
#![allow(unused_attributes)]

#[sandbox::sandbox(canton_version = ">=3.5, <4", shared, group = "versioned")]
async fn versioned(_sandbox: &sandbox::Sandbox) {}

fn main() {
    drop(versioned());
}
