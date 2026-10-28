use std::{borrow::Cow, fmt, str::FromStr};

use crate::topology::{Namespace, UniqueIdentifier, errors::UniqueIdentifierError};

/// A synchronizer ID consisting of an identifier and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SynchronizerId(UniqueIdentifier);

impl SynchronizerId {
    pub const fn from_uid(uid: UniqueIdentifier) -> Self {
        Self(uid)
    }

    pub const fn new_unchecked(identifier: String, namespace: String) -> Self {
        Self(UniqueIdentifier::new_unchecked(identifier, namespace))
    }

    /// Create new synchronizer ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, SynchronizerIdError> {
        UniqueIdentifier::new(identifier, namespace)
            .map(Self)
            .map_err(|err| SynchronizerIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }

    /// Underlying unique identifier
    pub fn uid(&self) -> &UniqueIdentifier {
        &self.0
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, SynchronizerIdError> {
        UniqueIdentifier::parse(s)
            .map(Self)
            .map_err(|err| SynchronizerIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }
}

impl AsRef<UniqueIdentifier> for SynchronizerId {
    fn as_ref(&self) -> &UniqueIdentifier {
        &self.0
    }
}

impl fmt::Display for SynchronizerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SynchronizerId {
    type Err = SynchronizerIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<UniqueIdentifier> for SynchronizerId {
    fn eq(&self, other: &UniqueIdentifier) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<str> for SynchronizerId {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<&str> for SynchronizerId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<String> for SynchronizerId {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for SynchronizerId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl From<UniqueIdentifier> for SynchronizerId {
    fn from(value: UniqueIdentifier) -> Self {
        Self::from_uid(value)
    }
}

impl From<SynchronizerId> for UniqueIdentifier {
    fn from(value: SynchronizerId) -> Self {
        value.0
    }
}

impl TryFrom<String> for SynchronizerId {
    type Error = SynchronizerIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        UniqueIdentifier::try_from(value)
            .map(Self)
            .map_err(|err| SynchronizerIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }
}

impl From<SynchronizerId> for String {
    fn from(value: SynchronizerId) -> Self {
        value.0.into()
    }
}

impl From<SynchronizerId> for crate::SynchronizerId {
    fn from(value: SynchronizerId) -> Self {
        // Both components use the Ledger API's permitted characters, and their
        // maximum lengths plus the delimiter are 185 + 2 + 68 = 255 bytes.
        Self::new_unchecked(value.into())
    }
}

impl TryFrom<crate::SynchronizerId> for SynchronizerId {
    type Error = SynchronizerIdError;

    fn try_from(value: crate::SynchronizerId) -> Result<Self, Self::Error> {
        Self::try_from(String::from(value))
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct SynchronizerIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error(transparent)]
    UniqueIdentifier(#[from] UniqueIdentifierError),
}
