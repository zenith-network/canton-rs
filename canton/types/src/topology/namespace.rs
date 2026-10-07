use std::{borrow::Cow, fmt, str::FromStr};

use crate::topology::{
    Fingerprint,
    fingerprint::{FingerprintError, MAX_LEN},
};

/// A namespace spanned by the fingerprint of a public key.
///
/// Non-empty strings with length <= 68 that match the regexp `[A-Za-z0-9:\-_ ]+`
/// and do not contain the reserved delimiter `::`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(Fingerprint);

impl Namespace {
    /// Reserved delimiter
    pub const RESERVER_DELIM: &str = "::";

    /// Cast fingerprint to namespace type
    pub fn from_fingerprint(fingerprint: Fingerprint) -> Self {
        fingerprint.into()
    }

    /// Check if given string is a valid namespace. If not, return corresponding error.
    pub fn validate(namespace: &str) -> Result<(), NamespaceError> {
        Fingerprint::validate(namespace).map_err(NamespaceError::from_fingerprint)
    }

    /// Create a new namespace.
    ///
    /// Return error if provided value is not a valid namespace.
    pub fn new(namespace: String) -> Result<Self, NamespaceError> {
        Fingerprint::new(namespace)
            .map(Self)
            .map_err(NamespaceError::from_fingerprint)
    }

    /// Returns a byte slice of this namespace's contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire namespace.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // A namespace is always non-empty.
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this namespace, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<&str> for Namespace {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for Namespace {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Namespace> for &str {
    fn eq(&self, other: &Namespace) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<Namespace> for Cow<'_, str> {
    fn eq(&self, other: &Namespace) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for Namespace {
    type Error = NamespaceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Namespace> for Fingerprint {
    fn from(value: Namespace) -> Self {
        value.0
    }
}

impl From<Fingerprint> for Namespace {
    fn from(value: Fingerprint) -> Self {
        Self(value)
    }
}

impl From<Namespace> for String {
    fn from(value: Namespace) -> Self {
        value.0.into()
    }
}

impl FromStr for Namespace {
    type Err = NamespaceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

// This mirrors FingerprintError to provide appropriate error messages.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct NamespaceError {
    kind: ErrorKind,
}

impl NamespaceError {
    const fn from_fingerprint(error: FingerprintError) -> Self {
        Self {
            kind: ErrorKind::from_fingerprint(error.kind),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("namespace is empty")]
    Empty,

    #[error("namespace is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in namespace")]
    UnexpectedChar { c: char },

    #[error(
        "namespace contains reserved delimiter {:?}",
        Namespace::RESERVER_DELIM
    )]
    ReservedDelimiter,
}

impl ErrorKind {
    const fn from_fingerprint(kind: super::fingerprint::ErrorKind) -> Self {
        match kind {
            super::fingerprint::ErrorKind::Empty => Self::Empty,
            super::fingerprint::ErrorKind::TooLong => Self::TooLong,
            super::fingerprint::ErrorKind::UnexpectedChar { c } => Self::UnexpectedChar { c },
            super::fingerprint::ErrorKind::ReservedDelimiter => Self::ReservedDelimiter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(68))]
    #[case("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789:-_ ")]
    #[case("1220".to_owned() + &"ab".repeat(32))]
    #[case("_")]
    #[case(" ")]
    #[case(":")]
    #[case(":foo:")]
    #[case("foo:bar:baz")]
    #[case("foo_bar-baz")]
    #[should_panic(expected = "unexpected character '%' in namespace")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '@' in namespace")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character '.' in namespace")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in namespace")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character '/' in namespace")]
    #[case("test/")]
    #[should_panic(expected = "unexpected character '\\t' in namespace")]
    #[case("test\t")]
    #[should_panic(expected = "unexpected character '\\n' in namespace")]
    #[case("test\n")]
    #[should_panic(expected = "unexpected character 'à' in namespace")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in namespace")]
    #[case("ਊ")]
    #[should_panic(expected = "namespace is empty")]
    #[case("")]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("a".repeat(69))]
    #[should_panic(expected = "namespace is too long (max: 68)")]
    #[case("a".repeat(10000))]
    #[should_panic(expected = "namespace contains reserved delimiter \"::\"")]
    #[case("::")]
    #[should_panic(expected = "namespace contains reserved delimiter \"::\"")]
    #[case("foo::bar")]
    #[should_panic(expected = "namespace contains reserved delimiter \"::\"")]
    #[case("::foo")]
    #[should_panic(expected = "namespace contains reserved delimiter \"::\"")]
    #[case("foo::")]
    #[should_panic(expected = "namespace contains reserved delimiter \"::\"")]
    #[case("foo:::bar")]
    fn test_namespace_new(#[case] input: String) {
        if let Err(err) = Namespace::new(input) {
            panic!("{}", err);
        }
    }
}
