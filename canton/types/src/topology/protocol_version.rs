use std::{fmt, num::ParseIntError};

// TODO: no supported protocol versions list for now, just accept all versions

/// Canton protocol version
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion(i32);

impl ProtocolVersion {
    /// Create new version without any checks.
    pub const fn new_unchecked(version: i32) -> Self {
        Self(version)
    }

    /// Return error if integer is not a valid version.
    pub const fn validate(version: i32) -> Result<(), ProtocolVersionError> {
        if version < 0 {
            Err(ProtocolVersionError {
                kind: ErrorKind::NegativeVersion,
            })
        } else {
            Ok(())
        }
    }

    /// Create new version.
    ///
    /// Returns error if integer is not a valid version.
    pub const fn new(version: i32) -> Result<Self, ProtocolVersionError> {
        match Self::validate(version) {
            Ok(()) => Ok(Self::new_unchecked(version)),
            Err(err) => Err(err),
        }
    }

    /// Create new `dev` version
    pub const fn new_dev() -> Self {
        Self(i32::MAX)
    }

    /// Return `true` if this version is a dev version of the protocol
    ///
    /// Dev version is encoded in Canton as `i32::MAX`.
    pub const fn is_dev(&self) -> bool {
        self.0 == i32::MAX
    }

    /// Parse version from string
    ///
    /// This recognizes `"dev"` string as dev version of the protocol.
    pub const fn parse(s: &str) -> Result<Self, ProtocolVersionError> {
        if s.eq_ignore_ascii_case("dev") {
            return Ok(Self::new_dev());
        }

        match i32::from_str_radix(s, 10) {
            Ok(version) => Self::new(version),
            Err(error) => Err(ProtocolVersionError {
                kind: ErrorKind::ParseIntError(error),
            }),
        }
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dev() {
            f.write_str("dev")
        } else {
            self.0.fmt(f)
        }
    }
}

impl From<ProtocolVersion> for i32 {
    fn from(value: ProtocolVersion) -> Self {
        value.0
    }
}

impl TryFrom<i32> for ProtocolVersion {
    type Error = ProtocolVersionError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("protocol version error: {kind}")]
pub struct ProtocolVersionError {
    kind: ErrorKind,
}

#[derive(Clone, Debug, thiserror::Error)]
enum ErrorKind {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("version is negative")]
    NegativeVersion,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::ProtocolVersion;

    #[rstest]
    #[case(0)]
    #[case(34)]
    #[case(i32::MAX)]
    #[should_panic(expected = "version is negative")]
    #[case(-1)]
    fn test_new(#[case] input: i32) {
        if let Err(error) = ProtocolVersion::new(input) {
            panic!("{error}")
        }
    }

    #[rstest]
    #[case("0")]
    #[case("34")]
    #[case("0034")]
    #[case("+34")]
    #[case("-0")]
    #[case("2147483647")]
    #[case("dev")]
    #[case("DeV")]
    #[should_panic(expected = "version is negative")]
    #[case("-1")]
    #[should_panic(expected = "number too large to fit in target type")]
    #[case("2147483648")]
    #[should_panic(expected = "number too small to fit in target type")]
    #[case("-2147483649")]
    #[should_panic(expected = "cannot parse integer from empty string")]
    #[case("")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case("bad")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case(" 34")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case("34 ")]
    fn test_parse(#[case] input: &str) {
        if let Err(error) = ProtocolVersion::parse(input) {
            panic!("{error}")
        }
    }

    #[rstest]
    #[case(ProtocolVersion(0), "0")]
    #[case(ProtocolVersion(34), "34")]
    #[case(ProtocolVersion(i32::MAX - 1), "2147483646")]
    #[case(ProtocolVersion(i32::MAX), "dev")]
    fn test_display(#[case] input: ProtocolVersion, #[case] expected: &str) {
        let display = input.to_string();
        assert_eq!(display, expected);
    }

    #[rstest]
    #[case(i32::MAX - 1, false)]
    #[case(i32::MAX, true)]
    fn test_is_dev(#[case] input: i32, #[case] expected: bool) {
        let version = ProtocolVersion(input);

        assert_eq!(version.is_dev(), expected);
    }
}
