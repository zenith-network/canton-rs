use canton::types::{Value, value::traits};

#[derive(Value)]
#[value(package_id = "ff", module_name = "A.B.C")]
pub struct MyType {
    value: i64,
    #[value(name = "otherValue")]
    other_value: String,
}

fn test<V: traits::Value>(_value: V) {}

fn main() {
    let m = MyType {
        value: 1,
        other_value: "123".to_string(),
    };
    test(m);
}
