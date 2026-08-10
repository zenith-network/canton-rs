use canton::ledger_api::types::value::v2::{HasIdentifier, Value};

#[derive(HasIdentifier, Value)]
#[identifier(package_id = "ffff", package_name = "my-pack", module = "A.B.C")]
pub struct MyType {
    value: i64,
    #[name = "otherValue"]
    other_value: String,
}

fn test<V: Value>(_value: V) {}

fn main() {
    let m = MyType {
        value: 1,
        other_value: "123".to_string(),
    };
    test(m);
}
