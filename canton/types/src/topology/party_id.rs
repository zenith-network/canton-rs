use std::{
    borrow::{Borrow, Cow},
    fmt,
    str::FromStr,
};

use crate::topology::{Identifier, Namespace, errors::IdentifierError};

/// A party ID consisting of an identifier and a namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyId(Identifier);

impl PartyId {
    pub fn from_identifier(id: Identifier) -> Self {
        Self(id)
    }

    /// Create new party ID
    pub fn new(identifier: String, namespace: Namespace) -> Result<Self, IdentifierError> {
        Identifier::new(identifier, namespace).map(Self)
    }

    /// Underlying identifier
    pub fn id(&self) -> &Identifier {
        &self.0
    }

    pub fn identifier(&self) -> &str {
        self.0.identifier()
    }

    pub fn namespace(&self) -> &Namespace {
        self.0.namespace()
    }

    pub fn parse(s: &str) -> Result<Self, IdentifierError> {
        Identifier::parse(s).map(Self)
    }
}

impl From<Identifier> for PartyId {
    fn from(value: Identifier) -> Self {
        Self::from_identifier(value)
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
    type Error = IdentifierError;

    fn try_from(value: crate::PartyId) -> Result<Self, Self::Error> {
        Self::try_from(String::from(value))
    }
}

impl From<PartyId> for Identifier {
    fn from(value: PartyId) -> Self {
        value.0
    }
}

impl AsRef<Identifier> for PartyId {
    fn as_ref(&self) -> &Identifier {
        &self.0
    }
}

impl Borrow<Identifier> for PartyId {
    fn borrow(&self) -> &Identifier {
        &self.0
    }
}

impl FromStr for PartyId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for PartyId {
    type Error = IdentifierError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Identifier::try_from(value).map(Self)
    }
}

impl TryFrom<&'_ str> for PartyId {
    type Error = IdentifierError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for PartyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<PartyId> for String {
    fn from(value: PartyId) -> Self {
        value.0.into()
    }
}

impl PartialEq<Identifier> for PartyId {
    fn eq(&self, other: &Identifier) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<PartyId> for Identifier {
    fn eq(&self, other: &PartyId) -> bool {
        other.eq(self)
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

impl PartialEq<PartyId> for str {
    fn eq(&self, other: &PartyId) -> bool {
        other.eq(self)
    }
}

impl PartialEq<PartyId> for &str {
    fn eq(&self, other: &PartyId) -> bool {
        other.eq(*self)
    }
}

impl PartialEq<PartyId> for String {
    fn eq(&self, other: &PartyId) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<PartyId> for Cow<'_, str> {
    fn eq(&self, other: &PartyId) -> bool {
        other.eq(self.as_ref())
    }
}
