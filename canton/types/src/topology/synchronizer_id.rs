use std::{
    borrow::{Borrow, Cow},
    fmt,
    str::FromStr,
};

use crate::topology::{Identifier, Namespace, errors::IdentifierError};

/// A synchronizer ID consisting of an identifier and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SynchronizerId(Identifier);

impl SynchronizerId {
    pub fn from_identifier(id: Identifier) -> Self {
        Self(id)
    }

    /// Create new synchronizer ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, IdentifierError> {
        Identifier::new(identifier, namespace).map(Self)
    }

    /// Underlying identifier
    pub fn id(&self) -> &Identifier {
        &self.0
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, IdentifierError> {
        Identifier::parse(s).map(Self)
    }
}

impl From<Identifier> for SynchronizerId {
    fn from(value: Identifier) -> Self {
        Self::from_identifier(value)
    }
}

impl From<SynchronizerId> for crate::SynchronizerId {
    fn from(value: SynchronizerId) -> Self {
        // Both components use the Ledger API's permitted characters, and their
        // maximum lengths plus the delimiter are 185 + 2 + 68 = 255 bytes.
        Self::new_unchecked(value.into())
    }
}

impl TryFrom<crate::SynchronizerId> for SynchronizerId {
    type Error = IdentifierError;

    fn try_from(value: crate::SynchronizerId) -> Result<Self, Self::Error> {
        Self::try_from(String::from(value))
    }
}

impl From<SynchronizerId> for Identifier {
    fn from(value: SynchronizerId) -> Self {
        value.0
    }
}

impl AsRef<Identifier> for SynchronizerId {
    fn as_ref(&self) -> &Identifier {
        &self.0
    }
}

impl Borrow<Identifier> for SynchronizerId {
    fn borrow(&self) -> &Identifier {
        &self.0
    }
}

impl FromStr for SynchronizerId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for SynchronizerId {
    type Error = IdentifierError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Identifier::try_from(value).map(Self)
    }
}

impl TryFrom<&'_ str> for SynchronizerId {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for SynchronizerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<SynchronizerId> for String {
    fn from(value: SynchronizerId) -> Self {
        value.0.into()
    }
}

impl PartialEq<Identifier> for SynchronizerId {
    fn eq(&self, other: &Identifier) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<SynchronizerId> for Identifier {
    fn eq(&self, other: &SynchronizerId) -> bool {
        other.eq(self)
    }
}

impl PartialEq<str> for SynchronizerId {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<&str> for SynchronizerId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<String> for SynchronizerId {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for SynchronizerId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<SynchronizerId> for str {
    fn eq(&self, other: &SynchronizerId) -> bool {
        other.eq(self)
    }
}

impl PartialEq<SynchronizerId> for &str {
    fn eq(&self, other: &SynchronizerId) -> bool {
        other.eq(*self)
    }
}

impl PartialEq<SynchronizerId> for String {
    fn eq(&self, other: &SynchronizerId) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<SynchronizerId> for Cow<'_, str> {
    fn eq(&self, other: &SynchronizerId) -> bool {
        other.eq(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        SynchronizerId::new("synchronizer".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "synchronizer::namespace"
    )]
    #[case(
        SynchronizerId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        ":id::namespace:"
    )]
    #[case(
        SynchronizerId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        SynchronizerId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        SynchronizerId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        " :: "
    )]
    #[case(
        SynchronizerId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "foo:bar-baz_9::one:two"
    )]
    #[case(
        SynchronizerId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_string(#[case] synchronizer: SynchronizerId, #[case] expected: String) {
        let actual: String = synchronizer.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("synchronizer::namespace", "synchronizer", "namespace")]
    #[case(":id::namespace:", ":id", "namespace:")]
    #[case("id:::namespace", "id", ":namespace")]
    #[case(":id:::namespace", ":id", ":namespace")]
    #[case(" :: ", " ", " ")]
    #[case("foo:bar-baz_9::one:two", "foo:bar-baz_9", "one:two")]
    #[case(
        "a".repeat(185) + "::" + &"b".repeat(68),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "empty first part")]
    #[case("", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace", "", "")]
    #[should_panic(expected = "missing a namespace")]
    #[case("synchronizer", "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case("synchronizer::", "", "")]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("bad%::namespace", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in identifier")]
    #[case("ਊ::namespace", "", "")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("synchronizer::bad%", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("synchronizer::ਊ", "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("synchronizer::namespace::extra", "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace", "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("synchronizer::".to_owned() + &"b".repeat(69), "", "")]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let synchronizer = SynchronizerId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(synchronizer.identifier(), expected_identifier);
        assert_eq!(synchronizer.namespace().as_str(), expected_namespace);
    }

    #[rstest]
    #[case(
        SynchronizerId::new("synchronizer".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "synchronizer::namespace"
    )]
    #[case(
        SynchronizerId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        ":id::namespace:"
    )]
    #[case(
        SynchronizerId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        SynchronizerId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        SynchronizerId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        " :: "
    )]
    #[case(
        SynchronizerId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "foo:bar-baz_9::one:two"
    )]
    #[case(
        SynchronizerId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_ledger_api_synchronizer_id(
        #[case] synchronizer: SynchronizerId,
        #[case] expected: String,
    ) {
        let actual: crate::SynchronizerId = synchronizer.into();
        assert_eq!(actual.as_str(), expected);
    }

    #[rstest]
    #[case(
        crate::SynchronizerId::new("synchronizer::namespace".to_owned()).unwrap(),
        "synchronizer",
        "namespace"
    )]
    #[case(
        crate::SynchronizerId::new(":id::namespace:".to_owned()).unwrap(),
        ":id",
        "namespace:"
    )]
    #[case(
        crate::SynchronizerId::new("id:::namespace".to_owned()).unwrap(),
        "id",
        ":namespace"
    )]
    #[case(
        crate::SynchronizerId::new(":id:::namespace".to_owned()).unwrap(),
        ":id",
        ":namespace"
    )]
    #[case(
        crate::SynchronizerId::new(" :: ".to_owned()).unwrap(),
        " ",
        " "
    )]
    #[case(
        crate::SynchronizerId::new("foo:bar-baz_9::one:two".to_owned()).unwrap(),
        "foo:bar-baz_9",
        "one:two"
    )]
    #[case(
        crate::SynchronizerId::new("a".repeat(185) + "::" + &"b".repeat(68)).unwrap(),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "missing a namespace")]
    #[case(crate::SynchronizerId::new("synchronizer".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "empty first part")]
    #[case(crate::SynchronizerId::new("::namespace".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case(crate::SynchronizerId::new("synchronizer::".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case(crate::SynchronizerId::new("synchronizer::namespace::extra".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case(crate::SynchronizerId::new("a".repeat(186) + "::namespace").unwrap(), "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case(crate::SynchronizerId::new("synchronizer::".to_owned() + &"b".repeat(69)).unwrap(), "", "")]
    fn test_try_from_ledger_api_synchronizer_id(
        #[case] input: crate::SynchronizerId,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let synchronizer = SynchronizerId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(synchronizer.identifier(), expected_identifier);
        assert_eq!(synchronizer.namespace().as_str(), expected_namespace);
    }
}
