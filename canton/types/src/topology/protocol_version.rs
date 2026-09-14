use std::{fmt, num::ParseIntError};

// TODO: this implementation is currently trivial, works for now, but could be improved

// FIXME: dev version needs explicit support

/// Canton protocol version
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion(i32);

impl ProtocolVersion {
    /// Create new version
    pub fn new(version: i32) -> Self {
        Self(version)
    }

    /// Parse version from string
    pub fn parse(s: &str) -> Result<Self, ParseIntError> {
        s.parse().map(Self)
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<ProtocolVersion> for i32 {
    fn from(value: ProtocolVersion) -> Self {
        value.0
    }
}
