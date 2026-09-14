use std::{borrow::Cow, fmt, str::FromStr};

/// Max namespace length
pub(super) const MAX_LEN: usize = 68;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' '
}

/// A fingerprint of a public key.
///
/// Non-empty strings with length <= 68 that match the regexp `[A-Za-z0-9:\-_ ]+`
/// and do not contain the reserved delimiter `::`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Reserved delimiter
    pub const RESERVER_DELIM: &str = "::";

    /// Check if given string is a valid fingerprint. If not, return corresponding error.
    pub fn validate(fingerprint: &str) -> Result<(), FingerprintError> {
        if fingerprint.is_empty() {
            return Err(FingerprintError {
                kind: ErrorKind::Empty,
            });
        }

        if fingerprint.len() > MAX_LEN {
            return Err(FingerprintError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in fingerprint.chars() {
            if !is_valid(c) {
                return Err(FingerprintError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        // Canton fingerprints must be representable as part of a UniqueIdentifier.
        if fingerprint.contains(Self::RESERVER_DELIM) {
            return Err(FingerprintError {
                kind: ErrorKind::ReservedDelimiter,
            });
        }

        Ok(())
    }

    /// Create a new fingerprint.
    ///
    /// Return error if provided value is not a valid fingerprint.
    pub fn new(fingerprint: String) -> Result<Self, FingerprintError> {
        Self::validate(&fingerprint)?;
        Ok(Self(fingerprint))
    }

    /// Returns a byte slice of this fingerprint's contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire fingerprint.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // A fingerprint is always non-empty.
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this fingerprint, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<&str> for Fingerprint {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for Fingerprint {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Fingerprint> for &str {
    fn eq(&self, other: &Fingerprint) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<Fingerprint> for Cow<'_, str> {
    fn eq(&self, other: &Fingerprint) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for Fingerprint {
    type Error = FingerprintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Fingerprint> for String {
    fn from(value: Fingerprint) -> Self {
        value.0
    }
}

impl FromStr for Fingerprint {
    type Err = FingerprintError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FingerprintError {
    // Kept pub-super so that wrapping type Namespace can inspect
    pub(super) kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ErrorKind {
    #[error("fingerprint is empty")]
    Empty,

    #[error("fingerprint is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in fingerprint")]
    UnexpectedChar { c: char },

    #[error(
        "fingerprint contains reserved delimiter {:?}",
        Fingerprint::RESERVER_DELIM
    )]
    ReservedDelimiter,
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
    #[should_panic(expected = "unexpected character '%' in fingerprint")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '@' in fingerprint")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character '.' in fingerprint")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in fingerprint")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character '/' in fingerprint")]
    #[case("test/")]
    #[should_panic(expected = "unexpected character '\\t' in fingerprint")]
    #[case("test\t")]
    #[should_panic(expected = "unexpected character '\\n' in fingerprint")]
    #[case("test\n")]
    #[should_panic(expected = "unexpected character 'à' in fingerprint")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in fingerprint")]
    #[case("ਊ")]
    #[should_panic(expected = "fingerprint is empty")]
    #[case("")]
    #[should_panic(expected = "fingerprint is too long (max: 68)")]
    #[case("a".repeat(69))]
    #[should_panic(expected = "fingerprint is too long (max: 68)")]
    #[case("a".repeat(10000))]
    #[should_panic(expected = "fingerprint contains reserved delimiter \"::\"")]
    #[case("::")]
    #[should_panic(expected = "fingerprint contains reserved delimiter \"::\"")]
    #[case("foo::bar")]
    #[should_panic(expected = "fingerprint contains reserved delimiter \"::\"")]
    #[case("::foo")]
    #[should_panic(expected = "fingerprint contains reserved delimiter \"::\"")]
    #[case("foo::")]
    #[should_panic(expected = "fingerprint contains reserved delimiter \"::\"")]
    #[case("foo:::bar")]
    fn test_new(#[case] input: String) {
        if let Err(err) = Fingerprint::new(input) {
            panic!("{}", err);
        }
    }
}
