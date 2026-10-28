use std::{fmt, str::FromStr};

use crate::topology::{
    MemberCode, Namespace, UniqueIdentifier,
    errors::{UniqueIdentifierError, UnknownMemberCode},
};

/// A mediator ID consisting of member code, an identifier and a namespace.
///
/// String representation: `MED::identifier::namespace`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediatorId(UniqueIdentifier);

impl MediatorId {
    pub const fn from_uid(uid: UniqueIdentifier) -> Self {
        Self(uid)
    }

    pub const fn new_unchecked(identifier: String, namespace: String) -> Self {
        Self(UniqueIdentifier::new_unchecked(identifier, namespace))
    }

    /// Create new mediator ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, MediatorIdError> {
        UniqueIdentifier::new(identifier, namespace)
            .map(Self)
            .map_err(|err| MediatorIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }

    /// Underlying unique identifier
    pub fn uid(&self) -> &UniqueIdentifier {
        &self.0
    }

    pub const fn member_code() -> MemberCode {
        MemberCode::Mediator
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, MediatorIdError> {
        let (identifier, namespace) = Self::parse_parts(s)?;
        Ok(Self(UniqueIdentifier {
            identifier: identifier.to_owned(),
            namespace,
        }))
    }

    fn parse_parts(s: &str) -> Result<(&str, Namespace), MediatorIdError> {
        let (code, uid) = s
            .split_once(UniqueIdentifier::DELIMITER)
            .ok_or(MediatorIdError {
                kind: ErrorKind::MissingMemberCode,
            })?;
        let code = MemberCode::parse(code).map_err(|err| MediatorIdError {
            kind: ErrorKind::MemberCode(err),
        })?;
        if code != Self::member_code() {
            return Err(MediatorIdError {
                kind: ErrorKind::UnexpectedMemberCode { got: code },
            });
        }
        UniqueIdentifier::parse_parts(uid).map_err(|err| MediatorIdError {
            kind: ErrorKind::UniqueIdentifier(err),
        })
    }
}

impl AsRef<UniqueIdentifier> for MediatorId {
    fn as_ref(&self) -> &UniqueIdentifier {
        &self.0
    }
}

impl fmt::Display for MediatorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}",
            Self::member_code().as_str(),
            UniqueIdentifier::DELIMITER,
            self.0.identifier,
            UniqueIdentifier::DELIMITER,
            self.0.namespace
        )
    }
}

impl FromStr for MediatorId {
    type Err = MediatorIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<UniqueIdentifier> for MediatorId {
    fn eq(&self, other: &UniqueIdentifier) -> bool {
        self.0.eq(other)
    }
}

impl From<UniqueIdentifier> for MediatorId {
    fn from(value: UniqueIdentifier) -> Self {
        Self::from_uid(value)
    }
}

impl From<MediatorId> for UniqueIdentifier {
    fn from(value: MediatorId) -> Self {
        value.0
    }
}

impl TryFrom<String> for MediatorId {
    type Error = MediatorIdError;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        let (identifier, namespace) = Self::parse_parts(&value)?;
        let identifier_len = identifier.len();
        let prefix_len = Self::member_code().as_str().len() + UniqueIdentifier::DELIMITER.len();
        // Keep the input allocation and move only the identifier component.
        value.truncate(prefix_len + identifier_len);
        value.drain(..prefix_len);
        Ok(Self(UniqueIdentifier {
            identifier: value,
            namespace,
        }))
    }
}

impl From<MediatorId> for String {
    fn from(value: MediatorId) -> Self {
        let len = MediatorId::member_code().as_str().len()
            + UniqueIdentifier::DELIMITER.len() * 2
            + value.0.identifier.len()
            + value.0.namespace.len();

        let mut result = String::with_capacity(len);
        result.push_str(MediatorId::member_code().as_str());
        result.push_str(UniqueIdentifier::DELIMITER);
        result.push_str(value.0.identifier.as_str());
        result.push_str(UniqueIdentifier::DELIMITER);
        result.push_str(value.0.namespace.as_str());
        result
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct MediatorIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error("mediator ID is missing a member code prefix")]
    MissingMemberCode,
    #[error(
        "expected mediator member code '{}', got '{}'",
        MemberCode::Mediator,
        got
    )]
    UnexpectedMemberCode { got: MemberCode },
    #[error(transparent)]
    UniqueIdentifier(#[from] UniqueIdentifierError),
    #[error(transparent)]
    MemberCode(#[from] UnknownMemberCode),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        MediatorId::new("mediator".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "MED::mediator::namespace"
    )]
    #[case(
        MediatorId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        "MED:::id::namespace:"
    )]
    #[case(
        MediatorId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "MED::id:::namespace"
    )]
    #[case(
        MediatorId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "MED::id:::namespace"
    )]
    #[case(
        MediatorId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        "MED:: :: "
    )]
    #[case(
        MediatorId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "MED::foo:bar-baz_9::one:two"
    )]
    #[case(
        MediatorId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "MED::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_string(#[case] mediator: MediatorId, #[case] expected: String) {
        let actual: String = mediator.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("MED::mediator::namespace", "mediator", "namespace")]
    #[case("MED:::id::namespace:", ":id", "namespace:")]
    #[case("MED::id:::namespace", "id", ":namespace")]
    #[case("MED:::id:::namespace", ":id", ":namespace")]
    #[case("MED:: :: ", " ", " ")]
    #[case("MED::foo:bar-baz_9::one:two", "foo:bar-baz_9", "one:two")]
    #[case(
        "MED::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("", "", "")]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("MED", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("mediator::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("med::mediator::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("XYZ::mediator::namespace", "", "")]
    #[should_panic(expected = "expected mediator member code 'MED', got 'PAR'")]
    #[case("PAR::mediator::namespace", "", "")]
    #[should_panic(expected = "expected mediator member code 'MED', got 'SEQ'")]
    #[case("SEQ::mediator::namespace", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("MED::", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("MED::::namespace", "", "")]
    #[should_panic(expected = "missing a namespace")]
    #[case("MED::mediator", "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case("MED::mediator::", "", "")]
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("MED::bad%::namespace", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in unique identifier")]
    #[case("MED::ਊ::namespace", "", "")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("MED::mediator::bad%", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("MED::mediator::ਊ", "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("MED::mediator::namespace::extra", "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("MED::".to_owned() + &"a".repeat(186) + "::namespace", "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("MED::mediator::".to_owned() + &"b".repeat(69), "", "")]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let mediator = MediatorId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(mediator.identifier(), expected_identifier);
        assert_eq!(mediator.namespace().as_str(), expected_namespace);
    }
}
