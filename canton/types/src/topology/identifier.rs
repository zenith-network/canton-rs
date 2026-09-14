use std::{borrow::Cow, fmt, str::FromStr};

use crate::topology::{Namespace, errors::NamespaceError};

/// Max identifier length
const MAX_IDENTIFIER_LEN: usize = 185;

/// Identifier of an entity in the topology (e.g. party, participant etc.)
///
/// Consists of two parts: an identifier (typically human-readable) and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier {
    pub(super) identifier: String,
    pub(super) namespace: Namespace,
}

impl Identifier {
    /// Delimiter of the identifier
    pub const DELIMITER: &str = Namespace::RESERVER_DELIM;

    /// Create new identifier
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, IdentifierError> {
        Self::validate_identifier(&identifier)?;
        Ok(Self {
            identifier,
            namespace,
        })
    }

    /// Validate identifier part of the topology identifier
    pub fn validate_identifier(identifier: &str) -> Result<(), IdentifierError> {
        if identifier.is_empty() {
            return Err(IdentifierError {
                kind: ErrorKind::EmptyIdentifier,
            });
        }
        if identifier.len() > MAX_IDENTIFIER_LEN {
            return Err(IdentifierError {
                kind: ErrorKind::IdentifierTooLong,
            });
        }

        let mut previous_colon = false;
        for c in identifier.chars() {
            if !(c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' ') {
                return Err(IdentifierError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
            if previous_colon && c == ':' {
                return Err(IdentifierError {
                    kind: ErrorKind::ReservedDelimiter,
                });
            }
            previous_colon = c == ':';
        }

        Ok(())
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn parse(s: &str) -> Result<Self, IdentifierError> {
        let (identifier, namespace) = Self::parse_parts(s)?;
        Ok(Self {
            identifier: identifier.to_owned(),
            namespace,
        })
    }

    pub(super) fn parse_parts(s: &str) -> Result<(&str, Namespace), IdentifierError> {
        let (identifier, namespace) = s.split_once(Self::DELIMITER).ok_or(IdentifierError {
            kind: if s.is_empty() {
                ErrorKind::EmptyIdentifier
            } else {
                ErrorKind::MissingNamespace
            },
        })?;

        Self::validate_identifier(identifier)?;
        let namespace = Namespace::new(namespace.to_owned()).map_err(|source| IdentifierError {
            kind: ErrorKind::Namespace { source },
        })?;
        Ok((identifier, namespace))
    }
}

impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Identifier {
    type Error = IdentifierError;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        let (identifier, namespace) = Self::parse_parts(&value)?;
        // Reuse the input allocation for the identifier component.
        value.truncate(identifier.len());
        Ok(Self {
            identifier: value,
            namespace,
        })
    }
}

impl TryFrom<&'_ str> for Identifier {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.identifier,
            Self::DELIMITER,
            self.namespace
        )
    }
}

impl From<Identifier> for String {
    fn from(value: Identifier) -> Self {
        let mut result = value.identifier;
        result.reserve(Identifier::DELIMITER.len() + value.namespace.len());
        result.push_str(Identifier::DELIMITER);
        result.push_str(value.namespace.as_str());
        result
    }
}

impl PartialEq<str> for Identifier {
    fn eq(&self, other: &str) -> bool {
        other
            .strip_prefix(self.identifier.as_str())
            .and_then(|suffix| suffix.strip_prefix(Self::DELIMITER))
            == Some(self.namespace.as_str())
    }
}

impl PartialEq<&str> for Identifier {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for Identifier {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for Identifier {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.eq(other.as_ref())
    }
}

impl PartialEq<Identifier> for str {
    fn eq(&self, other: &Identifier) -> bool {
        other.eq(self)
    }
}

impl PartialEq<Identifier> for &str {
    fn eq(&self, other: &Identifier) -> bool {
        other.eq(*self)
    }
}

impl PartialEq<Identifier> for String {
    fn eq(&self, other: &Identifier) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<Identifier> for Cow<'_, str> {
    fn eq(&self, other: &Identifier) -> bool {
        other.eq(self.as_ref())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IdentifierError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("identifier has an empty first part")]
    EmptyIdentifier,

    #[error("identifier is too long (max identifier length: {MAX_IDENTIFIER_LEN})")]
    IdentifierTooLong,

    #[error("unexpected character {c:?} in identifier")]
    UnexpectedChar { c: char },

    #[error("identifier contains reserved delimiter {:?}", Identifier::DELIMITER)]
    ReservedDelimiter,

    #[error("identifier is missing a namespace")]
    MissingNamespace,

    #[error("invalid namespace: {source}")]
    Namespace {
        #[source]
        source: NamespaceError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(185), "b".repeat(68))]
    #[case(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789:-_ ",
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789:-_ "
    )]
    #[case("aAbbZ09-", "1220".to_owned() + &"ab".repeat(32))]
    #[case("_", "a")]
    #[case(" ", " ")]
    #[case(":", ":")]
    #[case(":foo:", ":namespace:")]
    #[case("foo:bar:baz", "foo_bar-baz")]
    #[case("identifier", "_")]
    #[case("identifier", ":namespace")]
    #[case("identifier", "namespace:")]
    #[case("identifier", "one:two:three")]
    #[case("is", "ok")]
    #[case(":is", "ok:")]
    #[case("is", ":ok")]
    #[case("is", "o:k:r:l:y")]
    #[should_panic(expected = "empty first part")]
    #[case("", "namespace")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186), "namespace")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(10000), "namespace")]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("test%", "namespace")]
    #[should_panic(expected = "unexpected character '@' in identifier")]
    #[case("test@", "namespace")]
    #[should_panic(expected = "unexpected character '.' in identifier")]
    #[case("test.", "namespace")]
    #[should_panic(expected = "unexpected character '#' in identifier")]
    #[case("test#", "namespace")]
    #[should_panic(expected = "unexpected character '/' in identifier")]
    #[case("test/", "namespace")]
    #[should_panic(expected = "unexpected character '\\\\' in identifier")]
    #[case("test\\", "namespace")]
    #[should_panic(expected = "unexpected character '\\t' in identifier")]
    #[case("test\t", "namespace")]
    #[should_panic(expected = "unexpected character '\\n' in identifier")]
    #[case("test\n", "namespace")]
    #[should_panic(expected = "unexpected character 'à' in identifier")]
    #[case("à", "namespace")]
    #[should_panic(expected = "unexpected character 'ਊ' in identifier")]
    #[case("ਊ", "namespace")]
    #[should_panic(expected = "reserved delimiter \"::\"")]
    #[case("::", "namespace")]
    #[should_panic(expected = "reserved delimiter \"::\"")]
    #[case("foo::bar", "namespace")]
    #[should_panic(expected = "reserved delimiter \"::\"")]
    #[case("::foo", "namespace")]
    #[should_panic(expected = "reserved delimiter \"::\"")]
    #[case("foo::", "namespace")]
    #[should_panic(expected = "reserved delimiter \"::\"")]
    #[case("foo:::bar", "namespace")]
    fn test_identifier_new(#[case] identifier: String, #[case] namespace: String) {
        let namespace = Namespace::new(namespace).unwrap();
        let uid = match Identifier::new(identifier.clone(), namespace.clone()) {
            Ok(uid) => uid,
            Err(err) => panic!("{}", err),
        };
        assert_eq!(uid.identifier(), identifier);
        assert_eq!(uid.namespace(), &namespace);
    }

    #[rstest]
    #[case("is::ok")]
    #[case(":is::ok")]
    #[case(":is::ok:")]
    #[case("is::o:k:r:l:y")]
    #[case("is:::ok")]
    #[case(" :: ")]
    #[case("a".repeat(185) + "::" + &"b".repeat(68))]
    #[should_panic(expected = "empty first part")]
    #[case("")]
    #[should_panic(expected = "empty first part")]
    #[case("::namespace")]
    #[should_panic(expected = "empty first part")]
    #[case("::")]
    #[should_panic(expected = "missing a namespace")]
    #[case("identifier")]
    #[should_panic(expected = "missing a namespace")]
    #[case("identifier:namespace")]
    #[should_panic(expected = "invalid namespace: namespace is empty")]
    #[case("identifier::")]
    #[should_panic(expected = "max identifier length: 185")]
    #[case("a".repeat(186) + "::namespace")]
    #[should_panic(expected = "invalid namespace: namespace is too long (max: 68)")]
    #[case("identifier::".to_owned() + &"b".repeat(69))]
    #[should_panic(expected = "unexpected character '%' in identifier")]
    #[case("bad%::namespace")]
    #[should_panic(expected = "unexpected character 'à' in identifier")]
    #[case("à::namespace")]
    #[should_panic(expected = "invalid namespace: unexpected character '%' in namespace")]
    #[case("identifier::bad%")]
    #[should_panic(expected = "invalid namespace: unexpected character 'ਊ' in namespace")]
    #[case("identifier::ਊ")]
    #[should_panic(expected = "invalid namespace: namespace contains reserved delimiter \"::\"")]
    #[case("identifier::namespace::extra")]
    #[should_panic(expected = "invalid namespace: namespace contains reserved delimiter \"::\"")]
    #[case("identifier::::namespace")]
    #[should_panic(expected = "invalid namespace: namespace contains reserved delimiter \"::\"")]
    #[case("identifier::namespace::")]
    fn test_identifier_parse(#[case] input: String) {
        if let Err(err) = Identifier::parse(&input) {
            panic!("{}", err);
        }
    }

    #[rstest]
    #[case(
        Identifier::new("id".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "id::namespace"
    )]
    #[case(
        Identifier::new(":id".to_owned(), Namespace::new("namespace:".to_owned()).unwrap()).unwrap(),
        ":id::namespace:"
    )]
    #[case(
        Identifier::new("id:".to_owned(), Namespace::new("namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        Identifier::new("id".to_owned(), Namespace::new(":namespace".to_owned()).unwrap()).unwrap(),
        "id:::namespace"
    )]
    #[case(
        Identifier::new(" ".to_owned(), Namespace::new(" ".to_owned()).unwrap()).unwrap(),
        " :: "
    )]
    fn test_string_equality(#[case] uid: Identifier, #[case] expected: &str) {
        assert_eq!(uid, expected);
        assert_eq!(expected, uid);
    }
}
