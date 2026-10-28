use std::{borrow::Cow, fmt, str::FromStr};

use crate::topology::{Namespace, errors::NamespaceError};

/// Max identifier length
const MAX_IDENTIFIER_LEN: usize = 185;

/// Unique identifier of an entity in the topology (e.g. party, participant etc.)
///
/// Consists of two parts: an identifier (typically human-readable) and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueIdentifier {
    pub(super) identifier: String,
    pub(super) namespace: Namespace,
}

impl UniqueIdentifier {
    /// Delimiter of the unique identifier
    pub const DELIMITER: &str = Namespace::RESERVER_DELIM;

    /// Create new unique identifier
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, UniqueIdentifierError> {
        Self::validate_identifier(&identifier)?;
        Ok(Self {
            identifier,
            namespace,
        })
    }

    /// Create new unique identifier without validating the input
    pub const fn new_unchecked(identifier: String, namespace: String) -> Self {
        Self {
            identifier,
            namespace: Namespace::new_unchecked(namespace),
        }
    }

    /// Validate identifier part of the unique identifier
    pub fn validate_identifier(identifier: &str) -> Result<(), UniqueIdentifierError> {
        if identifier.is_empty() {
            return Err(UniqueIdentifierError {
                kind: ErrorKind::EmptyIdentifier,
            });
        }
        if identifier.len() > MAX_IDENTIFIER_LEN {
            return Err(UniqueIdentifierError {
                kind: ErrorKind::IdentifierTooLong,
            });
        }

        let mut previous_colon = false;
        for c in identifier.chars() {
            if !(c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' ') {
                return Err(UniqueIdentifierError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
            if previous_colon && c == ':' {
                return Err(UniqueIdentifierError {
                    kind: ErrorKind::ReservedDelimiter,
                });
            }
            previous_colon = c == ':';
        }

        Ok(())
    }

    /// Identifier part in unique identifier
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Namespace part in unique identifier
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Parse unique identifier from string
    pub fn parse(s: &str) -> Result<Self, UniqueIdentifierError> {
        let (identifier, namespace) = Self::parse_parts(s)?;
        Ok(Self {
            identifier: identifier.to_owned(),
            namespace,
        })
    }

    pub(super) fn parse_parts(s: &str) -> Result<(&str, Namespace), UniqueIdentifierError> {
        let (identifier, namespace) =
            s.split_once(Self::DELIMITER).ok_or(UniqueIdentifierError {
                kind: if s.is_empty() {
                    ErrorKind::EmptyIdentifier
                } else {
                    ErrorKind::MissingNamespace
                },
            })?;

        Self::validate_identifier(identifier)?;
        let namespace =
            Namespace::new(namespace.to_owned()).map_err(|source| UniqueIdentifierError {
                kind: ErrorKind::Namespace { source },
            })?;
        Ok((identifier, namespace))
    }
}

impl fmt::Display for UniqueIdentifier {
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

impl FromStr for UniqueIdentifier {
    type Err = UniqueIdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<str> for UniqueIdentifier {
    fn eq(&self, other: &str) -> bool {
        other
            .strip_prefix(self.identifier.as_str())
            .and_then(|suffix| suffix.strip_prefix(Self::DELIMITER))
            == Some(self.namespace.as_str())
    }
}

impl PartialEq<&str> for UniqueIdentifier {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<String> for UniqueIdentifier {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for UniqueIdentifier {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.eq(other.as_ref())
    }
}

// NOTE: UID "id:::ns" could be treated as both: ("id:", "ns") and ("id", ":ns").
//       Therefore implementing `PartialEq<UniqueIdentifier> for String` (and similar) would break
//       transitive requirements of `PartialEq`. That's why we do not implement them.

impl TryFrom<String> for UniqueIdentifier {
    type Error = UniqueIdentifierError;

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

impl From<UniqueIdentifier> for String {
    fn from(value: UniqueIdentifier) -> Self {
        let mut result = value.identifier;
        result.reserve(UniqueIdentifier::DELIMITER.len() + value.namespace.len());
        result.push_str(UniqueIdentifier::DELIMITER);
        result.push_str(value.namespace.as_str());
        result
    }
}

// NOTE: There is an inconsistency in string conversions for values created directly from
//       constructor: ("id:", "ns") will become "id:::ns" and then will be parsed as ("id", ":ns").
//       However this inconsistency exists in Canton sources as well. For that reason, to match
//       Canton behavior, we will keep it here - we don't want to make the parser stricter than it
//       is in Canton.
// TODO: When Canton resolves this issue, fix must be applied here.

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct UniqueIdentifierError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error("unique identifier has an empty first part")]
    EmptyIdentifier,

    #[error("identifier is too long (max identifier length: {MAX_IDENTIFIER_LEN})")]
    IdentifierTooLong,

    #[error("unexpected character {c:?} in unique identifier")]
    UnexpectedChar { c: char },

    #[error(
        "identifier contains reserved delimiter {:?}",
        UniqueIdentifier::DELIMITER
    )]
    ReservedDelimiter,

    #[error("unique identifier is missing a namespace")]
    MissingNamespace,

    #[error("invalid namespace: {source}")]
    Namespace {
        #[source]
        source: NamespaceError,
    },
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::topology::Namespace;

    use super::UniqueIdentifier;

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
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("test%", "namespace")]
    #[should_panic(expected = "unexpected character '@' in unique identifier")]
    #[case("test@", "namespace")]
    #[should_panic(expected = "unexpected character '.' in unique identifier")]
    #[case("test.", "namespace")]
    #[should_panic(expected = "unexpected character '#' in unique identifier")]
    #[case("test#", "namespace")]
    #[should_panic(expected = "unexpected character '/' in unique identifier")]
    #[case("test/", "namespace")]
    #[should_panic(expected = "unexpected character '\\\\' in unique identifier")]
    #[case("test\\", "namespace")]
    #[should_panic(expected = "unexpected character '\\t' in unique identifier")]
    #[case("test\t", "namespace")]
    #[should_panic(expected = "unexpected character '\\n' in unique identifier")]
    #[case("test\n", "namespace")]
    #[should_panic(expected = "unexpected character 'à' in unique identifier")]
    #[case("à", "namespace")]
    #[should_panic(expected = "unexpected character 'ਊ' in unique identifier")]
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
    fn test_new(#[case] identifier: String, #[case] namespace: String) {
        let namespace = Namespace::new_unchecked(namespace);
        let uid = match UniqueIdentifier::new(identifier.clone(), namespace.clone()) {
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
    #[should_panic(expected = "unexpected character '%' in unique identifier")]
    #[case("bad%::namespace")]
    #[should_panic(expected = "unexpected character 'à' in unique identifier")]
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
    fn test_parse(#[case] input: String) {
        if let Err(err) = UniqueIdentifier::parse(&input) {
            panic!("{}", err);
        }
    }

    #[rstest]
    #[case(
        UniqueIdentifier::new_unchecked("id".to_owned(), "namespace".to_owned()),
        "id::namespace"
    )]
    #[case(
        UniqueIdentifier::new_unchecked(":id".to_owned(),"namespace:".to_owned()),
        ":id::namespace:"
    )]
    #[case(
        UniqueIdentifier::new_unchecked("id:".to_owned(),"namespace".to_owned()),
        "id:::namespace"
    )]
    #[case(
        UniqueIdentifier::new_unchecked("id".to_owned(), ":namespace".to_owned()),
        "id:::namespace"
    )]
    #[case(
        UniqueIdentifier::new_unchecked(" ".to_owned(), " ".to_owned()),
        " :: "
    )]
    fn test_string_equality(#[case] uid: UniqueIdentifier, #[case] expected: &str) {
        assert_eq!(uid, expected);
    }
}
