//! A Daml LF variant: every constructor carries one payload (`()` for none).
use canton::ledger_api::types::value::v2::{
    HasIdentifier, IntoValue as _, TryFromValue as _, Value,
    errors::TryFromVariantError,
    value::{self, Variant},
};

#[derive(Clone, Debug, PartialEq, HasIdentifier, Value)]
#[value(crate_path = ::canton)]
#[identifier(package_id = "ffff", package_name = "my-pack", module = "A.B.C", name = "AnyValue", crate_path = ::canton)]
pub enum AnyValue {
    #[name = "AV_None"]
    None(()),
    #[name = "AV_Text"]
    Text(String),
    /// Recursive through a list, like CIP-56's `AnyValue`.
    #[name = "AV_List"]
    List(Vec<AnyValue>),
}

fn round_trip(v: AnyValue) {
    assert_eq!(AnyValue::try_from_value(v.clone().into_value()).unwrap(), v);
}

#[test]
fn round_trips_every_constructor() {
    round_trip(AnyValue::None(()));
    round_trip(AnyValue::Text(
        "0x70997970c51812dc3a010c7d01b50e0d17dc79c8".into(),
    ));
    round_trip(AnyValue::List(vec![
        AnyValue::Text("a".into()),
        AnyValue::List(vec![AnyValue::None(())]),
    ]));
}

#[test]
fn encodes_the_constructor_name_and_identifier() {
    let value::Value::Variant(variant) = AnyValue::Text("x".into()).into_value() else {
        panic!("not a variant");
    };
    assert_eq!(variant.constructor.as_str(), "AV_Text");
    assert_eq!(
        variant.variant_id,
        Some(AnyValue::identifier_with_package_id())
    );
}

#[test]
fn refuses_unknown_constructors_foreign_identifiers_and_bad_payloads() {
    let value::Value::Variant(good) = AnyValue::Text("x".into()).into_value() else {
        unreachable!()
    };
    let unknown = Variant {
        constructor: canton::types::Name::new_static_unchecked("AV_Int"),
        ..(*good).clone()
    };
    assert!(matches!(
        AnyValue::try_from_value(value::Value::Variant(Box::new(unknown))),
        Err(TryFromVariantError::UnexpectedConstructorName(_))
    ));
    let mut foreign_id = AnyValue::identifier_with_package_id();
    foreign_id.package_id = canton::types::PackageId::new_unchecked("eeee");
    let foreign = Variant {
        variant_id: Some(foreign_id),
        ..(*good).clone()
    };
    assert!(matches!(
        AnyValue::try_from_value(value::Value::Variant(Box::new(foreign))),
        Err(TryFromVariantError::UnexpectedIdentifier(_))
    ));
    let bad_payload = Variant {
        value: ().into_value(),
        ..(*good).clone()
    };
    assert!(matches!(
        AnyValue::try_from_value(value::Value::Variant(Box::new(bad_payload))),
        Err(TryFromVariantError::PayloadError(_))
    ));
    assert!(matches!(
        AnyValue::try_from_value(().into_value()),
        Err(TryFromVariantError::ValueKindError(_))
    ));
}

#[test]
fn daml_time_and_date_round_trip_and_refuse_other_kinds() {
    use canton::types::{Date, Timestamp};
    let t = Timestamp(1_790_000_000_000_000);
    assert_eq!(Timestamp::try_from_value(t.into_value()).unwrap(), t);
    let d = Date(20_700);
    assert_eq!(Date::try_from_value(d.into_value()).unwrap(), d);
    assert!(Timestamp::try_from_value(d.into_value()).is_err());
    assert!(Date::try_from_value(t.into_value()).is_err());
}

/// A generic variant (`data Either a b`, `data Result a = ...`): the payloads that
/// mention a type parameter are bounded by the conversion traits.
#[derive(Clone, Debug, PartialEq, HasIdentifier, Value)]
#[value(crate_path = ::canton)]
#[identifier(package_id = "ffff", package_name = "my-pack", module = "A.B.C", name = "Gen", crate_path = ::canton)]
pub enum Gen<A> {
    GenL(A),
    GenR(i64),
    GenList(Vec<A>),
}

#[test]
fn generic_variants_round_trip() {
    for v in [
        Gen::GenL("a".to_string()),
        Gen::GenR(7),
        Gen::GenList(vec!["b".to_string()]),
    ] {
        assert_eq!(
            Gen::<String>::try_from_value(v.clone().into_value()).unwrap(),
            v
        );
    }
}
