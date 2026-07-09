use std::{borrow::Cow, fmt};

use crate::PackageIdAny;

/// Max package ID length
const MAX_LEN: usize = 64;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' '
}

/// Package ID
///
/// Non-empty string with length <= 64 that matches the regexp `[A-Za-z0-9\-_ ]+`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(Cow<'static, str>);

impl PackageId {
    pub const fn new_unchecked(package_id: &'static str) -> Self {
        Self(Cow::Borrowed(package_id))
    }

    pub const fn new_unchecked_owned(package_id: String) -> Self {
        Self(Cow::Owned(package_id))
    }

    pub fn validate(package_id: impl AsRef<str>) -> Result<(), PackageIdError> {
        let package_id = package_id.as_ref();

        if package_id.is_empty() {
            return Err(PackageIdError {
                kind: ErrorKind::Empty,
            });
        }

        if package_id.len() > MAX_LEN {
            return Err(PackageIdError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in package_id.chars() {
            if !is_valid(c) {
                return Err(PackageIdError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(())
    }

    pub fn new(package_id: String) -> Result<Self, PackageIdError> {
        Self::validate(package_id.as_str())?;
        Ok(Self(Cow::Owned(package_id)))
    }

    pub const fn as_str(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        match &self.0 {
            Cow::Borrowed(s) => s.as_bytes(),
            Cow::Owned(s) => s.as_bytes(),
        }
    }

    // Disable this clippy lint, because is_empty() method is meaningless for this type:
    // Package ID is always non-empty
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        match &self.0 {
            Cow::Borrowed(s) => s.len(),
            Cow::Owned(s) => s.len(),
        }
    }

    pub fn into_any(self) -> PackageIdAny {
        PackageIdAny::Id(self)
    }
}

impl AsRef<str> for PackageId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<PackageId> for String {
    fn from(value: PackageId) -> Self {
        match value.0 {
            Cow::Borrowed(s) => s.to_owned(),
            Cow::Owned(s) => s,
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<str> for PackageId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for PackageId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for PackageId {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<&String> for PackageId {
    fn eq(&self, other: &&String) -> bool {
        &self.0 == *other
    }
}

impl PartialEq<PackageId> for str {
    fn eq(&self, other: &PackageId) -> bool {
        self == other.0
    }
}

impl PartialEq<PackageId> for &str {
    fn eq(&self, other: &PackageId) -> bool {
        self == &other.0
    }
}

impl PartialEq<PackageId> for String {
    fn eq(&self, other: &PackageId) -> bool {
        self == &other.0
    }
}

impl PartialEq<PackageId> for &String {
    fn eq(&self, other: &PackageId) -> bool {
        *self == &other.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid package ID: {kind}")]
pub struct PackageIdError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("package ID is empty")]
    Empty,

    #[error("package ID is too long (max: {MAX_LEN})")]
    TooLong,

    #[error("unexpected character {c:?} in PackageId")]
    UnexpectedChar { c: char },
}
