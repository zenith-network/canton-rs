use canton::types::Value;

#[derive(Value)]
#[value(module_name = "A.B.C")]
pub struct MyType {
    value: i64,
}

fn main() {}
