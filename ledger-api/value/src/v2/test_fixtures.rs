use std::convert::Infallible;

use canton_types::{DottedName, Name, PackageId, PackageName};

use crate::v2::{HasIdentifier, IntoRecord, IntoValue, TryFromRecord, TryFromValue, Value};

use super::{Record, value};

pub use canton_types::test_fixtures::{TestChoiceA, TestChoiceB, TestKey, TestTemplate};

impl HasIdentifier for TestTemplate {
    fn package_id() -> PackageId {
        PackageId::new_unchecked("test_package_id")
    }

    fn package_name() -> PackageName {
        PackageName::new_unchecked("test_package_name")
    }

    fn module_name() -> DottedName {
        DottedName::single(Name::new_static_unchecked("TestModule"))
    }

    fn entity_name() -> DottedName {
        DottedName::single(Name::new_static_unchecked("TestTemplate"))
    }
}

impl IntoRecord for TestTemplate {
    fn into_record(self) -> value::Record {
        unreachable!()
    }
}

impl TryFromRecord for TestTemplate {
    type Error = Infallible;

    fn try_from_record(_: value::Record) -> Result<Self, Self::Error> {
        unreachable!()
    }
}

impl Record for TestTemplate {}

impl IntoValue for TestChoiceA {
    fn into_value(self) -> value::Value {
        unreachable!()
    }
}

impl TryFromValue for TestChoiceA {
    type Error = Infallible;

    fn try_from_value(_: value::Value) -> Result<Self, Self::Error> {
        unreachable!()
    }
}

impl Value for TestChoiceA {}

impl IntoValue for TestChoiceB {
    fn into_value(self) -> value::Value {
        unreachable!()
    }
}

impl TryFromValue for TestChoiceB {
    type Error = Infallible;

    fn try_from_value(_: value::Value) -> Result<Self, Self::Error> {
        unreachable!()
    }
}

impl Value for TestChoiceB {}

impl IntoValue for TestKey {
    fn into_value(self) -> value::Value {
        unreachable!()
    }
}

impl TryFromValue for TestKey {
    type Error = std::convert::Infallible;

    fn try_from_value(_: value::Value) -> Result<Self, Self::Error> {
        unreachable!()
    }
}

impl Value for TestKey {}
