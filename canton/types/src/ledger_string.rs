use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
};

/// Max LedgerString length
const MAX_LEN: usize = 255;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '#'
        || c == ':'
        || c == '-'
        || c == '_'
        || c == '/'
        || c == ' '
        || c == '.'
}

// Disable this clippy lint, because is_empty() method is meaningless for this type: LedgerString is
// always non-empty
#[allow(clippy::len_without_is_empty)]
/// Strings with length <= 255 that match the regexp `[A-Za-z0-9#:\-_/ .]+`
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerString(String);

impl LedgerString {
    /// Create a new `LedgerString`.
    ///
    /// Return error if provided value is not a valid `LedgerString`.
    pub fn new(value: String) -> Result<Self, LedgerStringError> {
        if value.is_empty() {
            return Err(LedgerStringError {
                kind: ErrorKind::Empty,
            });
        }

        if value.len() > MAX_LEN {
            return Err(LedgerStringError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in value.chars() {
            if !is_valid(c) {
                return Err(LedgerStringError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(Self(value))
    }

    /// Returns a byte slice of this `LedgerString`'s contents.
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire `LedgerString`.
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the length of this `LedgerString`, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Debug for LedgerString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for LedgerString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for LedgerString {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for LedgerString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<str> for LedgerString {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for LedgerString {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<&str> for LedgerString {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<String> for LedgerString {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<Cow<'_, str>> for LedgerString {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<LedgerString> for &str {
    fn eq(&self, other: &LedgerString) -> bool {
        self.eq(&other.0)
    }
}

impl PartialEq<LedgerString> for Cow<'_, str> {
    fn eq(&self, other: &LedgerString) -> bool {
        self.eq(&other.0)
    }
}

impl TryFrom<String> for LedgerString {
    type Error = LedgerStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<LedgerString> for String {
    fn from(value: LedgerString) -> Self {
        value.into_string()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct LedgerStringError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("LedgerString is empty")]
    Empty,

    #[error("LedgerString is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in LedgerString")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(255))]
    #[case("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._:-#/ ")]
    #[should_panic(expected = "LedgerString is empty")]
    #[case("")]
    #[should_panic(expected = "LedgerString is too long (max: 255)")]
    #[case("a".repeat(256))]
    #[should_panic(expected = "unexpected character 'ñ' in LedgerString")]
    #[case("español")]
    #[should_panic(expected = "unexpected character '東' in LedgerString")]
    #[case("東京")]
    #[should_panic(expected = "unexpected character 'Λ' in LedgerString")]
    #[case("Λ (τ : ⋆) (σ: ⋆ → ⋆). λ (e : ∀ (α : ⋆). σ α) → (( e @τ ))")]
    fn test_ledger_string_new(#[case] input: String) {
        if let Err(err) = LedgerString::new(input) {
            panic!("{}", err);
        }
    }
}
