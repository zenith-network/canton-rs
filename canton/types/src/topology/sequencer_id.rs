use std::{
    borrow::{Borrow, Cow},
    fmt,
    str::FromStr,
};

use crate::topology::{
    Identifier, MemberCode, Namespace,
    errors::{IdentifierError, UnknownMemberCode},
};

/// A sequencer ID consisting of member code, an identifier and a namespace.
///
/// String representation: `SEQ::identifier::namespace`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequencerId(Identifier);

impl SequencerId {
    pub fn from_identifier(id: Identifier) -> Self {
        Self(id)
    }

    /// Create new sequencer ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, SequencerIdError> {
        Identifier::new(identifier, namespace)
            .map(Self)
            .map_err(|err| SequencerIdError {
                kind: ErrorKind::Identifier(err),
            })
    }

    /// Underlying identifier
    pub fn id(&self) -> &Identifier {
        &self.0
    }

    pub const fn member_code() -> MemberCode {
        MemberCode::Sequencer
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, SequencerIdError> {
        let (identifier, namespace) = Self::parse_parts(s)?;
        Ok(Self(Identifier {
            identifier: identifier.to_owned(),
            namespace,
        }))
    }

    fn parse_parts(s: &str) -> Result<(&str, Namespace), SequencerIdError> {
        let (code, identifier) = s
            .split_once(Identifier::DELIMITER)
            .ok_or(SequencerIdError {
                kind: ErrorKind::MissingMemberCode,
            })?;
        let code = MemberCode::parse(code).map_err(|err| SequencerIdError {
            kind: ErrorKind::MemberCode(err),
        })?;
        if code != Self::member_code() {
            return Err(SequencerIdError {
                kind: ErrorKind::UnexpectedMemberCode { got: code },
            });
        }
        Identifier::parse_parts(identifier).map_err(|err| SequencerIdError {
            kind: ErrorKind::Identifier(err),
        })
    }
}

impl From<Identifier> for SequencerId {
    fn from(value: Identifier) -> Self {
        Self::from_identifier(value)
    }
}

impl AsRef<Identifier> for SequencerId {
    fn as_ref(&self) -> &Identifier {
        &self.0
    }
}

impl Borrow<Identifier> for SequencerId {
    fn borrow(&self) -> &Identifier {
        &self.0
    }
}

impl FromStr for SequencerId {
    type Err = SequencerIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for SequencerId {
    type Error = SequencerIdError;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        let (identifier, namespace) = Self::parse_parts(&value)?;
        let identifier_len = identifier.len();
        let prefix_len = Self::member_code().as_str().len() + Identifier::DELIMITER.len();
        // Keep the input allocation and move only the identifier component.
        value.truncate(prefix_len + identifier_len);
        value.drain(..prefix_len);
        Ok(Self(Identifier {
            identifier: value,
            namespace,
        }))
    }
}

impl TryFrom<&'_ str> for SequencerId {
    type Error = SequencerIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for SequencerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}",
            Self::member_code().as_str(),
            Identifier::DELIMITER,
            self.0.identifier,
            Identifier::DELIMITER,
            self.0.namespace
        )
    }
}

impl From<SequencerId> for String {
    fn from(value: SequencerId) -> Self {
        let len = SequencerId::member_code().as_str().len()
            + Identifier::DELIMITER.len() * 2
            + value.0.identifier.len()
            + value.0.namespace.len();

        let mut result = String::with_capacity(len);
        result.push_str(SequencerId::member_code().as_str());
        result.push_str(Identifier::DELIMITER);
        result.push_str(value.0.identifier.as_str());
        result.push_str(Identifier::DELIMITER);
        result.push_str(value.0.namespace.as_str());
        result
    }
}

impl PartialEq<str> for SequencerId {
    fn eq(&self, other: &str) -> bool {
        other
            .strip_prefix(Self::member_code().as_str())
            .and_then(|suffix| suffix.strip_prefix(Identifier::DELIMITER))
            .is_some_and(|identifier| self.0.eq(identifier))
    }
}

impl PartialEq<&str> for SequencerId {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for SequencerId {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for SequencerId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.eq(other.as_ref())
    }
}

impl PartialEq<SequencerId> for str {
    fn eq(&self, other: &SequencerId) -> bool {
        other.eq(self)
    }
}

impl PartialEq<SequencerId> for &str {
    fn eq(&self, other: &SequencerId) -> bool {
        other.eq(*self)
    }
}

impl PartialEq<SequencerId> for String {
    fn eq(&self, other: &SequencerId) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<SequencerId> for Cow<'_, str> {
    fn eq(&self, other: &SequencerId) -> bool {
        other.eq(self.as_ref())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequencerIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("sequencer ID is missing a member code prefix")]
    MissingMemberCode,
    #[error(
        "expected sequencer member code '{}', got '{}'",
        MemberCode::Sequencer,
        got
    )]
    UnexpectedMemberCode { got: MemberCode },
    #[error(transparent)]
    Identifier(#[from] IdentifierError),
    #[error(transparent)]
    MemberCode(#[from] UnknownMemberCode),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        SequencerId::new("sequencer".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "SEQ::sequencer::namespace"
    )]
    #[case(
        SequencerId::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        "SEQ:::id::namespace:"
    )]
    #[case(
        SequencerId::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "SEQ::id:::namespace"
    )]
    #[case(
        SequencerId::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "SEQ::id:::namespace"
    )]
    #[case(
        SequencerId::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        "SEQ:: :: "
    )]
    #[case(
        SequencerId::new("foo:bar-baz_9".to_owned(), Namespace::new("one:two".to_owned()).unwrap()).unwrap(),
        "SEQ::foo:bar-baz_9::one:two"
    )]
    #[case(
        SequencerId::new("a".repeat(185), Namespace::new("b".repeat(68)).unwrap()).unwrap(),
        "SEQ::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68)
    )]
    fn test_into_string(#[case] sequencer: SequencerId, #[case] expected: String) {
        let actual: String = sequencer.into();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case("SEQ::sequencer::namespace", "sequencer", "namespace")]
    #[case("SEQ:::id::namespace:", ":id", "namespace:")]
    #[case("SEQ::id:::namespace", "id", ":namespace")]
    #[case("SEQ:::id:::namespace", ":id", ":namespace")]
    #[case("SEQ:: :: ", " ", " ")]
    #[case("SEQ::foo:bar-baz_9::one:two", "foo:bar-baz_9", "one:two")]
    #[case(
        "SEQ::".to_owned() + &"a".repeat(185) + "::" + &"b".repeat(68),
        "a".repeat(185),
        "b".repeat(68)
    )]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("", "", "")]
    #[should_panic(expected = "missing a member code prefix")]
    #[case("SEQ", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("sequencer::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("seq::sequencer::namespace", "", "")]
    #[should_panic(expected = "unknown member code")]
    #[case("XYZ::sequencer::namespace", "", "")]
    #[should_panic(expected = "expected sequencer member code 'SEQ', got 'MED'")]
    #[case("MED::sequencer::namespace", "", "")]
    #[should_panic(expected = "expected sequencer member code 'SEQ', got 'PAR'")]
    #[case("PAR::sequencer::namespace", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("SEQ::", "", "")]
    #[should_panic(expected = "empty first part")]
    #[case("SEQ::::namespace", "", "")]
    #[should_panic(expected = "missing a namespace")]
    #[case("SEQ::sequencer", "", "")]
    #[should_panic(expected = "namespace is empty")]
    #[case("SEQ::sequencer::", "", "")]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("SEQ::bad%::namespace", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in identifier")]
    #[case("SEQ::ਊ::namespace", "", "")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("SEQ::sequencer::bad%", "", "")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("SEQ::sequencer::ਊ", "", "")]
    #[should_panic(expected = "namespace contains reserved delimiter")]
    #[case("SEQ::sequencer::namespace::extra", "", "")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("SEQ::".to_owned() + &"a".repeat(186) + "::namespace", "", "")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("SEQ::sequencer::".to_owned() + &"b".repeat(69), "", "")]
    fn test_try_from_string(
        #[case] input: String,
        #[case] expected_identifier: String,
        #[case] expected_namespace: String,
    ) {
        let sequencer = SequencerId::try_from(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(sequencer.identifier(), expected_identifier);
        assert_eq!(sequencer.namespace().as_str(), expected_namespace);
    }
}
