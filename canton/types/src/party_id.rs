use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
    str::FromStr,
};

/// Max party ID length
const MAX_LEN: usize = 255;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' '
}

/// Party ID
///
/// Strings with length <= 255 that match the regexp `[A-Za-z0-9:\-_ ]+`
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyId(String);

impl PartyId {
    /// Create a new party ID.
    ///
    /// Return error if provided value is not a valid `Party`.
    pub fn new(value: String) -> Result<Self, PartyIdError> {
        if value.is_empty() {
            return Err(PartyIdError {
                kind: ErrorKind::Empty,
            });
        }
        if value.len() > MAX_LEN {
            return Err(PartyIdError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in value.chars() {
            if !is_valid(c) {
                return Err(PartyIdError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(Self(value))
    }

    /// Returns a byte slice of this `Party`'s contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire `Party`.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // Disable this clippy lint, because is_empty() method is meaningless for this type: party ID is
    // always non-empty
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this `Party`, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for PartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for PartyId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for PartyId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<str> for PartyId {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for PartyId {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<&str> for PartyId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for PartyId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<PartyId> for &str {
    fn eq(&self, other: &PartyId) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<PartyId> for Cow<'_, str> {
    fn eq(&self, other: &PartyId) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for PartyId {
    type Error = PartyIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PartyId> for String {
    fn from(value: PartyId) -> Self {
        value.0
    }
}

impl FromStr for PartyId {
    type Err = PartyIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PartyIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("party ID is empty")]
    Empty,

    #[error("party ID is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in party ID")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(255))]
    #[case("_")]
    #[case("_blAH:9")]
    #[case("foo_bar-baz")]
    #[case("baz_")]
    #[should_panic(expected = "unexpected character '%' in party ID")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '@' in party ID")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character '.' in party ID")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in party ID")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character 'à' in party ID")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in party ID")]
    #[case("ਊ")]
    #[should_panic(expected = "party ID is empty")]
    #[case("")]
    #[should_panic(expected = "party ID is too long (max: 255)")]
    #[case("a".repeat(256))]
    #[should_panic(expected = "party ID is too long (max: 255)")]
    #[case("a".repeat(10000))]
    fn test_party_id_new(#[case] input: String) {
        if let Err(err) = PartyId::new(input) {
            panic!("{}", err);
        }
    }
}
