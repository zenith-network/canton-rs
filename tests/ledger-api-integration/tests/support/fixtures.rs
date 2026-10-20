//! Representative Daml values and unique identifiers.

use canton::types::{Date, LedgerString, TextMap};
use ledger_api_integration::integration::{Color, Detail, Nested, Payload};

/// Generate a unique ledger identifier with a recognizable prefix.
pub fn id(prefix: &str) -> LedgerString {
    LedgerString::new(format!("{prefix}-{}", uuid::Uuid::new_v4())).unwrap()
}

/// Build a representative payload; the number distinguishes test values.
pub fn payload(number: i64) -> Payload {
    Payload {
        number,
        amount: "123456.1234567890".parse().unwrap(),
        text: "Canton: café λ 🚀".into(),
        day: Date::from_ymd_opt(2024, 2, 29).unwrap(),
        moment: "2024-02-29T12:34:56.123456Z".parse().unwrap(),
        color: Color::Green,
        detail: Detail::Count(-17),
        nested: Nested {
            enabled: true,
            numbers: vec![i64::MIN, 0, i64::MAX],
            note: Some("nested".into()),
        },
        labels: TextMap(
            [
                ("alpha".into(), "first".into()),
                ("β".into(), "second".into()),
            ]
            .into(),
        ),
    }
}
