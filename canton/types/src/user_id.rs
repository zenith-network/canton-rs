use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
};

/// Max user ID length
const MAX_LEN: usize = 128;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '@'
        || c == '^'
        || c == '$'
        || c == '.'
        || c == '!'
        || c == '`'
        || c == '-'
        || c == '#'
        || c == '+'
        || c == '\''
        || c == '~'
        || c == '_'
        || c == '|'
        || c == ':'
}

/// User ID
///
/// Non-empry string with length <= 128 that match the regexp ``[a-zA-Z0-9@^$.!`\-#+'~_|:]+``
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(String);

impl UserId {
    /// Create a new user ID.
    ///
    /// Return error if provided value is not a valid user ID.
    pub fn new(value: String) -> Result<Self, UserIdError> {
        if value.is_empty() {
            return Err(UserIdError {
                kind: ErrorKind::Empty,
            });
        }
        if value.len() > MAX_LEN {
            return Err(UserIdError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in value.chars() {
            if !is_valid(c) {
                return Err(UserIdError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(Self(value))
    }

    /// Returns a byte slice of this user ID's contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire user ID.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    // Disable this clippy lint, because is_empty() method is meaningless for this type: user ID is
    // always non-empty
    #[allow(clippy::len_without_is_empty)]
    /// Returns the length of this user ID, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for UserId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<str> for UserId {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for UserId {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<&str> for UserId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for UserId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<UserId> for &str {
    fn eq(&self, other: &UserId) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<UserId> for Cow<'_, str> {
    fn eq(&self, other: &UserId) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for UserId {
    type Error = UserIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UserId> for String {
    fn from(value: UserId) -> Self {
        value.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct UserIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("user ID is empty")]
    Empty,

    #[error("user ID is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in user ID")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(128))]
    fn test_name_string_new(#[case] input: String) {
        if let Err(err) = UserId::new(input) {
            panic!("{}", err);
        }
    }
}
