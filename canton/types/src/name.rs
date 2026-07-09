use std::{
    borrow::{Borrow, Cow},
    fmt,
    num::NonZeroUsize,
    ops::Deref,
};

/// Max name length
const MAX_LEN: usize = 1000;

const fn is_valid_first(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '$' || c == '_'
}

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '$' || c == '_'
}

/// Non-emty string with length <= 1000 that match the regexp `[A-Za-z\$_][A-Za-z0-9\$_]*`
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(Cow<'static, str>);

impl Name {
    /// Create a new name.
    ///
    /// Return error if provided value is not a valid name.
    pub fn new(value: impl Into<Cow<'static, str>>) -> Result<Self, NameError> {
        let value = value.into();
        Self::validate(&value).map(|_| Self(value))
    }

    /// Check if given string is a valid name. If not, return corresponding error.
    pub fn validate(value: impl AsRef<str>) -> Result<(), NameError> {
        let value = value.as_ref();

        if value.len() > MAX_LEN {
            return Err(NameError {
                kind: ErrorKind::TooLong,
            });
        }

        let mut chars = value.chars();

        if let Some(first) = chars.next() {
            if !is_valid_first(first) {
                return Err(NameError {
                    kind: ErrorKind::UnexpectedChar { c: first },
                });
            }
        } else {
            return Err(NameError {
                kind: ErrorKind::Empty,
            });
        }

        for c in chars {
            if !is_valid(c) {
                return Err(NameError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(())
    }

    /// Create new name bypassing correctness checks.
    pub fn new_unchecked(value: String) -> Self {
        Self(Cow::Owned(value))
    }

    /// Create new name bypassing correctness checks.
    pub const fn new_static_unchecked(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }

    /// Returns a byte slice of this name's contents.
    pub const fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Cow::Borrowed(s) => s.as_bytes(),
            Cow::Owned(s) => s.as_bytes(),
        }
    }

    /// Extracts a string slice containing the entire name.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        }
    }

    /// Returns the length of this name, in bytes, not [`char`]s or graphemes.
    pub const fn len(&self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(match &self.0 {
                Cow::Borrowed(s) => s.len(),
                Cow::Owned(s) => s.len(),
            })
        }
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for Name {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Cow::Borrowed(s) => s.as_ref(),
            Cow::Owned(s) => s.as_ref(),
        }
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for Name {
    type Target = <String as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl PartialEq<Name> for &Name {
    fn eq(&self, other: &Name) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<str> for &Name {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<&String> for Name {
    fn eq(&self, other: &&String) -> bool {
        &self.0 == *other
    }
}

impl PartialEq<String> for &Name {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<Cow<'_, str>> for Name {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == other.0
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        self == &other.0
    }
}

impl PartialEq<Name> for Cow<'_, str> {
    fn eq(&self, other: &Name) -> bool {
        self == &other.0
    }
}

impl TryFrom<String> for Name {
    type Error = NameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Name> for String {
    fn from(value: Name) -> Self {
        match value.0 {
            Cow::Borrowed(s) => s.to_owned(),
            Cow::Owned(s) => s,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid name: {kind}")]
pub struct NameError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("name is empty")]
    Empty,

    #[error("name is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in name")]
    UnexpectedChar { c: char },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("a".repeat(1000))]
    #[case("$")]
    #[case("$blAH9")]
    #[case("foo$bar")]
    #[case("baz$")]
    #[case("_")]
    #[case("_blAH9")]
    #[case("foo_bar")]
    #[case("baz_")]
    #[should_panic(expected = "unexpected character '9' in name")]
    #[case("9test")]
    #[should_panic(expected = "unexpected character '%' in name")]
    #[case("test%")]
    #[should_panic(expected = "unexpected character '-' in name")]
    #[case("test-")]
    #[should_panic(expected = "unexpected character '@' in name")]
    #[case("test@")]
    #[should_panic(expected = "unexpected character ':' in name")]
    #[case("test:")]
    #[should_panic(expected = "unexpected character '.' in name")]
    #[case("test.")]
    #[should_panic(expected = "unexpected character '#' in name")]
    #[case("test#")]
    #[should_panic(expected = "unexpected character 'à' in name")]
    #[case("à")]
    #[should_panic(expected = "unexpected character 'ਊ' in name")]
    #[case("ਊ")]
    #[should_panic(expected = "name is empty")]
    #[case("")]
    #[should_panic(expected = "name is too long (max: 1000)")]
    #[case("a".repeat(1001))]
    #[should_panic(expected = "name is too long (max: 1000)")]
    #[case("a".repeat(10000))]
    fn test_name_new(#[case] input: String) {
        if let Err(err) = Name::new(input) {
            panic!("{}", err);
        }
    }
}
