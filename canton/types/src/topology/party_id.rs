use std::{borrow::Cow, fmt, str::FromStr};

use crate::topology::{Namespace, UniqueIdentifier, errors::UniqueIdentifierError};

/// A party ID consisting of an identifier and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyId(UniqueIdentifier);

impl PartyId {
    pub const fn from_uid(uid: UniqueIdentifier) -> Self {
        Self(uid)
    }

    pub const fn new_unchecked(identifier: String, namespace: String) -> Self {
        Self(UniqueIdentifier::new_unchecked(identifier, namespace))
    }

    /// Create new party ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, PartyIdError> {
        UniqueIdentifier::new(identifier, namespace)
            .map(Self)
            .map_err(|err| PartyIdError {
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

    pub fn parse(s: &str) -> Result<Self, PartyIdError> {
        UniqueIdentifier::parse(s)
            .map(Self)
            .map_err(|err| PartyIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }
}

impl AsRef<UniqueIdentifier> for PartyId {
    fn as_ref(&self) -> &UniqueIdentifier {
        &self.0
    }
}

impl fmt::Display for PartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for PartyId {
    type Err = PartyIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl PartialEq<UniqueIdentifier> for PartyId {
    fn eq(&self, other: &UniqueIdentifier) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<str> for PartyId {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<&str> for PartyId {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<String> for PartyId {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Cow<'_, str>> for PartyId {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.0.eq(other)
    }
}

impl From<UniqueIdentifier> for PartyId {
    fn from(value: UniqueIdentifier) -> Self {
        Self::from_uid(value)
    }
}

impl From<PartyId> for UniqueIdentifier {
    fn from(value: PartyId) -> Self {
        value.0
    }
}

impl TryFrom<String> for PartyId {
    type Error = PartyIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        UniqueIdentifier::try_from(value)
            .map(Self)
            .map_err(|err| PartyIdError {
                kind: ErrorKind::UniqueIdentifier(err),
            })
    }
}

impl From<PartyId> for String {
    fn from(value: PartyId) -> Self {
        value.0.into()
    }
}

impl From<PartyId> for crate::PartyId {
    fn from(value: PartyId) -> Self {
        // Both components use the Ledger API's permitted characters, and their
        // maximum lengths plus the delimiter are 185 + 2 + 68 = 255 bytes.
        Self::new_unchecked(value.into())
    }
}

impl TryFrom<crate::PartyId> for PartyId {
    type Error = PartyIdError;

    fn try_from(value: crate::PartyId) -> Result<Self, Self::Error> {
        Self::try_from(String::from(value))
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct PartyIdError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error(transparent)]
    UniqueIdentifier(#[from] UniqueIdentifierError),
}
