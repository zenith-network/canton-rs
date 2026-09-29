//! A Daml record that gained a trailing `Optional` field: the Ledger API omits
//! trailing `None` fields, so a payload may carry fewer fields than the type.
use canton::ledger_api::types::value::v2::{
    HasIdentifier, IntoValue as _, TryFromRecord as _, TryFromValue as _, Value,
    value::{self, Record, RecordField},
};

#[derive(Clone, Debug, PartialEq, HasIdentifier, Value)]
#[value(crate_path = ::canton)]
#[identifier(package_id = "ffff", package_name = "my-pack", module = "A.B.C", name = "Upgraded", crate_path = ::canton)]
pub struct Upgraded {
    pub name: String,
    pub count: i64,
    /// Added in an upgrade, last.
    pub note: Option<String>,
}

fn record(fields: Vec<value::Value>) -> Record {
    Record {
        record_id: None,
        fields: fields
            .into_iter()
            .map(|value| RecordField { label: None, value })
            .collect(),
    }
}

#[test]
fn omitted_trailing_optional_fields_decode_as_none() {
    let full = Upgraded {
        name: "a".into(),
        count: 1,
        note: None,
    };
    assert_eq!(
        Upgraded::try_from_value(full.clone().into_value()).unwrap(),
        full
    );
    let normalized = record(vec![value::Value::Text("a".into()), value::Value::Int64(1)]);
    assert_eq!(Upgraded::try_from_record(normalized).unwrap(), full);
}

#[test]
fn missing_fields_that_are_not_optional_are_refused() {
    assert!(Upgraded::try_from_record(record(vec![value::Value::Text("a".into())])).is_err());
    assert!(Upgraded::try_from_record(record(vec![])).is_err());
}

#[test]
fn extra_fields_are_refused() {
    let extra = record(vec![
        value::Value::Text("a".into()),
        value::Value::Int64(1),
        value::Value::Optional(None),
        value::Value::Int64(2),
    ]);
    assert!(Upgraded::try_from_record(extra).is_err());
}
