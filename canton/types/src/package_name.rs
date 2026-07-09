use std::{borrow::Cow, fmt, str::FromStr};

use crate::PackageIdAny;

pub(crate) const DISCRIMINATOR: char = '#';

/// Max length of package name
const MAX_LEN: usize = 255;

const fn is_valid(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Package name
///
/// Non-empty string with length <= 255 that matches the regexp `[A-Za-z0-9_-]+`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(Cow<'static, str>);

impl PackageName {
    pub const fn new_unchecked(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    pub fn new(name: String) -> Result<Self, PackageNameError> {
        if name.is_empty() {
            return Err(PackageNameError {
                kind: ErrorKind::Empty,
            });
        }

        if name.len() > MAX_LEN {
            return Err(PackageNameError {
                kind: ErrorKind::TooLong,
            });
        }

        for c in name.chars() {
            if !is_valid(c) {
                return Err(PackageNameError {
                    kind: ErrorKind::UnexpectedChar { c },
                });
            }
        }

        Ok(Self(Cow::Owned(name)))
    }

    pub const fn as_str(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(s) => s,
            Cow::Owned(s) => s.as_str(),
        }
    }

    pub fn into_any(self) -> PackageIdAny {
        PackageIdAny::Name(self)
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for PackageName {
    /// If alternate flag (`#`) is set, this formatting will contain discriminator `#`:
    ///
    /// ```rust,no_run
    /// # let package_name = daml_primitives::package_name::PackageName::new_unchecked("");
    /// format!("{}", package_name) // "mypackage"
    /// format!("{:#}", package_name) // "#mypackage"
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{DISCRIMINATOR}{}", self.0)
        } else {
            self.0.fmt(f)
        }
    }
}

impl FromStr for PackageName {
    type Err = PackageNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(name) = s.strip_prefix(DISCRIMINATOR) {
            Self::new(name.to_owned())
        } else {
            Err(PackageNameError {
                kind: ErrorKind::MissingDiscriminator,
            })
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PackageNameError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("package name is empty")]
    Empty,
    #[error("package name is too long (max: {MAX_LEN})")]
    TooLong,
    #[error("package name is expected to start with a discriminator '{DISCRIMINATOR}'")]
    MissingDiscriminator,
    #[error("unexpected character {c:?} in NameString")]
    UnexpectedChar { c: char },
}
