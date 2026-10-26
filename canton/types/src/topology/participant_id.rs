use std::{fmt, str::FromStr};

use crate::topology::{
    MemberCode, Namespace, UniqueIdentifier,
    errors::{UniqueIdentifierError, UnknownMemberCode},
};

/// A participant ID consisting of member code, an identifier and a namespace.
///
/// String representation: `PAR::identifier::namespace`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParticipantId(UniqueIdentifier);

impl ParticipantId {
    pub const fn from_uid(uid: UniqueIdentifier) -> Self {
        Self(uid)
    }

    pub fn new_unchecked(identifier: String, namespace: String) -> Self {
        Self(UniqueIdentifier::new_unchecked(identifier, namespace))
    }

    /// Create new participant ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, ParticipantIdError> {
        UniqueIdentifier::new(identifier, namespace)
            .map(Self)
            .map_err(|err| ParticipantIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }

    /// Underlying unique identifier
    pub fn uid(&self) -> &UniqueIdentifier {
        &self.0
    }

    pub const fn member_code() -> MemberCode {
        MemberCode::Participant
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, ParticipantIdError> {
        let (identifier, namespace) = Self::parse_parts(s)?;
        Ok(Self(UniqueIdentifier {
            identifier: identifier.to_owned(),
            namespace,
        }))
    }

    fn parse_parts(s: &str) -> Result<(&str, Namespace), ParticipantIdError> {
        let (code, uid) = s
            .split_once(UniqueIdentifier::DELIMITER)
            .ok_or(ParticipantIdError {
                kind: ErrorKind::MissingMemberCode,
            })?;
        let code = MemberCode::parse(code).map_err(|err| ParticipantIdError {
            kind: ErrorKind::MemberCode(err),
        })?;
        if code != Self::member_code() {
            return Err(ParticipantIdError {
                kind: ErrorKind::UnexpectedMemberCode { got: code },
            });
        }
        UniqueIdentifier::parse_parts(uid).map_err(|err| ParticipantIdError {
            kind: ErrorKind::UniqueIdentifier(err),
        })
    }
}

impl AsRef<UniqueIdentifier> for ParticipantId {
    fn as_ref(&self) -> &UniqueIdentifier {
        &self.0
    }
}

impl fmt::Display for ParticipantId {
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

impl FromStr for ParticipantId {
    type Err = ParticipantIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<UniqueIdentifier> for ParticipantId {
    fn eq(&self, other: &UniqueIdentifier) -> bool {
        self.0.eq(other)
    }
}

impl From<UniqueIdentifier> for ParticipantId {
    fn from(value: UniqueIdentifier) -> Self {
        Self::from_uid(value)
    }
}

impl From<ParticipantId> for UniqueIdentifier {
    fn from(value: ParticipantId) -> Self {
        value.0
    }
}

impl TryFrom<String> for ParticipantId {
    type Error = ParticipantIdError;

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

impl From<ParticipantId> for String {
    fn from(value: ParticipantId) -> Self {
        let len = ParticipantId::member_code().as_str().len()
            + UniqueIdentifier::DELIMITER.len() * 2
            + value.0.identifier.len()
            + value.0.namespace.len();

        let mut result = String::with_capacity(len);
        result.push_str(ParticipantId::member_code().as_str());
        result.push_str(UniqueIdentifier::DELIMITER);
        result.push_str(value.0.identifier.as_str());
        result.push_str(UniqueIdentifier::DELIMITER);
        result.push_str(value.0.namespace.as_str());
        result
    }
}

impl From<ParticipantId> for crate::ParticipantId {
    fn from(value: ParticipantId) -> Self {
        // The Ledger API uses the underlying unique identifier without the member code.
        Self::new_unchecked(value.0.into())
    }
}

impl TryFrom<crate::ParticipantId> for ParticipantId {
    type Error = ParticipantIdError;

    fn try_from(value: crate::ParticipantId) -> Result<Self, Self::Error> {
        UniqueIdentifier::try_from(String::from(value))
            .map(Self::from_uid)
            .map_err(|err| ParticipantIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParticipantIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error("participant ID is missing a member code prefix")]
    MissingMemberCode,
    #[error(
        "expected participant member code '{}', got '{}'",
        MemberCode::Participant,
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
        ParticipantId::new("participant".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "PAR::participant::namespace"
    )]
    #[case(
        ParticipantId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        "PAR:::id::namespace:"
    )]
    #[case(
        ParticipantId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "PAR::id:::namespace"
    )]
    #[case(
        ParticipantId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "PAR::id:::namespace"
    )]
    #[case(
        ParticipantId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        "PAR:: :: "
    )]
    #[case(
        ParticipantId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "PAR::foo:bar-baz_9::one:two"
    )]
    #[case(
        ParticipantId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "PAR::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_string(#[case] participant: ParticipantId, #[case] expected: String) {
        let actual: String = participant.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("PAR::participant::namespace", "participant", "namespace")]
    #[case("PAR:::id::namespace:", ":id", "namespace:")]
    #[case("PAR::id:::namespace", "id", ":namespace")]
    #[case("PAR:::id:::namespace", ":id", ":namespace")]
    #[case("PAR:: :: ", " ", " ")]
    #[case("PAR::foo:bar-baz_9::one:two", "foo:bar-baz_9", "one:two")]
    #[case(
        "PAR::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("", "", "")]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("PAR", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("participant::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("par::participant::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("XYZ::participant::namespace", "", "")]
    #[should_panic(expected = "expected participant member code 'PAR', got 'MED'")]
    #[case("MED::participant::namespace", "", "")]
    #[should_panic(expected = "expected participant member code 'PAR', got 'SEQ'")]
    #[case("SEQ::participant::namespace", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("PAR::", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("PAR::::namespace", "", "")]
    #[should_panic(expected = "missing a namespace")]
    #[case("PAR::participant", "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case("PAR::participant::", "", "")]
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("PAR::bad%::namespace", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in unique identifier")]
    #[case("PAR::ਊ::namespace", "", "")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("PAR::participant::bad%", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("PAR::participant::ਊ", "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("PAR::participant::namespace::extra", "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("PAR::".to_owned() + &"a".repeat(186) + "::namespace", "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("PAR::participant::".to_owned() + &"b".repeat(69), "", "")]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let participant = ParticipantId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(participant.identifier(), expected_identifier);
        assert_eq!(participant.namespace().as_str(), expected_namespace);
    }

    #[rstest]
    #[case(
        ParticipantId::new("participant".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "participant::namespace"
    )]
    #[case(
        ParticipantId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        ":id::namespace:"
    )]
    #[case(
        ParticipantId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        ParticipantId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        ParticipantId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        " :: "
    )]
    #[case(
        ParticipantId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "foo:bar-baz_9::one:two"
    )]
    #[case(
        ParticipantId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_ledger_api_participant_id(
        #[case] participant: ParticipantId,
        #[case] expected: String,
    ) {
        let actual: crate::ParticipantId = participant.into();
        assert_eq!(actual.as_str(), expected);
    }

    #[rstest]
    #[case(
        crate::ParticipantId::new("participant::namespace".to_owned()).unwrap(),
        "participant",
        "namespace"
    )]
    #[case(
        crate::ParticipantId::new(":id::namespace:".to_owned()).unwrap(),
        ":id",
        "namespace:"
    )]
    #[case(
        crate::ParticipantId::new("id:::namespace".to_owned()).unwrap(),
        "id",
        ":namespace"
    )]
    #[case(
        crate::ParticipantId::new(":id:::namespace".to_owned()).unwrap(),
        ":id",
        ":namespace"
    )]
    #[case(
        crate::ParticipantId::new(" :: ".to_owned()).unwrap(),
        " ",
        " "
    )]
    #[case(
        crate::ParticipantId::new("foo:bar-baz_9::one:two".to_owned()).unwrap(),
        "foo:bar-baz_9",
        "one:two"
    )]
    #[case(
        crate::ParticipantId::new("a".repeat(185) + "::" + &"b".repeat(68)).unwrap(),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "missing a namespace")]
    #[case(crate::ParticipantId::new("participant".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "empty first part")]
    #[case(crate::ParticipantId::new("::namespace".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case(crate::ParticipantId::new("participant::".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case(crate::ParticipantId::new("participant::namespace::extra".to_owned()).unwrap(), "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case(crate::ParticipantId::new("a".repeat(186) + "::namespace").unwrap(), "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case(crate::ParticipantId::new("participant::".to_owned() + &"b".repeat(69)).unwrap(), "", "")]
    fn test_try_from_ledger_api_participant_id(
        #[case] input: crate::ParticipantId,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let participant = ParticipantId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(participant.identifier(), expected_identifier);
        assert_eq!(participant.namespace().as_str(), expected_namespace);
    }
}
