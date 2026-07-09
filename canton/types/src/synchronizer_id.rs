use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
};

/// Max synchronizer ID length
const MAX_LEN: usize = 255;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ':' || c == '-' || c == '_' || c == ' '
}

/// Synchronizer ID
///
/// Strings with length <= 255 that match the regexp `[A-Za-z0-9:\-_ ]+`
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SynchronizerId(String);

impl SynchronizerId {
    /// Create a new synchronizer ID.
    ///
    /// Return error if provided value is not a valid `SynchronizerId`.
    pub fn new(value: String) -> Result<Self, SynchronizerIdError> {
        if value.is_empty() {
            return Err(SynchronizerIdError {
                kind: ErrorKind::Empty,
            });
        }
        if value.len() > MAX_LEN {
            return Err(SynchronizerIdError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in value.chars() {
            if !is_valid(c) {
                return Err(SynchronizerIdError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(Self(value))
    }

    /// Returns a byte slice of this `SynchronizerId`'s contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire `SynchronizerId`.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // Disable this clippy lint, because is_empty() method is meaningless for this type:
    // synchronizer ID is always non-empty
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this `SynchronizerId`, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for SynchronizerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for SynchronizerId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for SynchronizerId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<str> for SynchronizerId {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for SynchronizerId {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<&str> for SynchronizerId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for SynchronizerId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<SynchronizerId> for &str {
    fn eq(&self, other: &SynchronizerId) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<SynchronizerId> for Cow<'_, str> {
    fn eq(&self, other: &SynchronizerId) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for SynchronizerId {
    type Error = SynchronizerIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SynchronizerId> for String {
    fn from(value: SynchronizerId) -> Self {
        value.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SynchronizerIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("synchronizer ID is empty")]
    Empty,

    #[error("synchronizer ID is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in synchronizer ID")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(255))]
    #[case("da::default")]
    #[case("_")]
    #[case("_blAH:9")]
    #[case("foo_bar-baz")]
    #[case("baz_")]
    #[should_panic(expected = "unexpected character '%' in synchronizer ID")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '@' in synchronizer ID")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character '.' in synchronizer ID")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in synchronizer ID")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character 'à' in synchronizer ID")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in synchronizer ID")]
    #[case("ਊ")]
    #[should_panic(expected = "synchronizer ID is empty")]
    #[case("")]
    #[should_panic(expected = "synchronizer ID is too long (max: 255)")]
    #[case("a".repeat(256))]
    #[should_panic(expected = "synchronizer ID is too long (max: 255)")]
    #[case("a".repeat(10000))]
    fn test_synchronizer_id_new(#[case] input: String) {
        if let Err(err) = SynchronizerId::new(input) {
            panic!("{}", err);
        }
    }
}
