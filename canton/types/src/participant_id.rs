use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
    str::FromStr,
};

/// Max participant ID length
const MAX_LEN: usize = 255;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' '
}

/// Participant ID
///
/// Non-empty strings with length <= 255 that match the regexp `[A-Za-z0-9:\-_ ]+`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParticipantId(String);

impl ParticipantId {
    /// Create a new participant ID without validating the input.
    pub fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Return error if input is not a valid participant ID.
    pub fn validate(value: &str) -> Result<(), ParticipantIdError> {
        if value.is_empty() {
            return Err(ParticipantIdError {
                kind: ErrorKind::Empty,
            });
        }
        if value.len() > MAX_LEN {
            return Err(ParticipantIdError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in value.chars() {
            if !is_valid(c) {
                return Err(ParticipantIdError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(())
    }

    /// Create a new participant ID.
    ///
    /// Return error if provided value is not a valid participant ID.
    pub fn new(value: String) -> Result<Self, ParticipantIdError> {
        Self::validate(&value)?;
        Ok(Self::new_unchecked(value))
    }

    /// Returns a byte slice of this `ParticipantId`'s contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire `ParticipantId`.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // A participant ID is always non-empty.
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this `ParticipantId`, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for ParticipantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ParticipantId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for ParticipantId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<str> for ParticipantId {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for ParticipantId {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<&str> for ParticipantId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for ParticipantId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<ParticipantId> for &str {
    fn eq(&self, other: &ParticipantId) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<ParticipantId> for Cow<'_, str> {
    fn eq(&self, other: &ParticipantId) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for ParticipantId {
    type Error = ParticipantIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ParticipantId> for String {
    fn from(value: ParticipantId) -> Self {
        value.0
    }
}

impl FromStr for ParticipantId {
    type Err = ParticipantIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParticipantIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("participant ID is empty")]
    Empty,

    #[error("participant ID is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in participant ID")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(255))]
    #[case("participant::default")]
    #[case("_")]
    #[case("_blAH:9")]
    #[case("foo_bar-baz")]
    #[case("baz_")]
    #[should_panic(expected = "unexpected character '%' in participant ID")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '@' in participant ID")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character '.' in participant ID")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in participant ID")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character 'à' in participant ID")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in participant ID")]
    #[case("ਊ")]
    #[should_panic(expected = "participant ID is empty")]
    #[case("")]
    #[should_panic(expected = "participant ID is too long (max: 255)")]
    #[case("a".repeat(256))]
    #[should_panic(expected = "participant ID is too long (max: 255)")]
    #[case("a".repeat(10000))]
    fn test_participant_id_new(#[case] input: String) {
        if let Err(err) = ParticipantId::new(input) {
            panic!("{}", err);
        }
    }
}
