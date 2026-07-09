use canton::types::Value;

#[derive(Value)]
#[value(package_id = "ff", module_name = "A.B.C", name = "My.Type")]
pub struct MyType {
    value: i64,
}

fn main() {}
