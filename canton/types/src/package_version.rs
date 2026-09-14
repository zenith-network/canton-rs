use std::{borrow::Cow, fmt, num::ParseIntError, str::FromStr};

/// Max allowed length of string representation of a package version
const MAX_STR_LEN: usize = 255;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageVersion(Cow<'static, [u32]>);

impl PackageVersion {
    pub fn new_unchecked(version: Vec<u32>) -> Self {
        Self(Cow::Owned(version))
    }

    pub const fn new_unchecked_static(version: &'static [u32]) -> Self {
        Self(Cow::Borrowed(version))
    }

    pub fn new_static(version: &'static [u32]) -> Result<Self, PackageVersionError> {
        Self::validate(version)?;
        Ok(Self::new_unchecked_static(version))
    }

    pub fn new(version: Vec<u32>) -> Result<Self, PackageVersionError> {
        Self::validate(&version)?;
        Ok(Self::new_unchecked(version))
    }

    pub fn validate(version: &[u32]) -> Result<(), PackageVersionError> {
        if version.is_empty() {
            return Err(PackageVersionError {
                kind: ErrorKind::Empty,
            });
        }
        // We count string representation length, cause it must not exceed 255 chars
        // First we count dots between numbers
        let mut len = version.len() - 1;
        let max = i32::MAX as u32;
        for num in version {
            if *num > max {
                return Err(PackageVersionError {
                    kind: ErrorKind::NumberTooBig { num: *num },
                });
            }
            // TODO: optimize to avoid string allocation
            len += num.to_string().len();
        }
        if len > MAX_STR_LEN {
            return Err(PackageVersionError {
                kind: ErrorKind::TooLong { len },
            });
        }
        Ok(())
    }

    pub fn parse(s: &str) -> Result<Self, PackageVersionError> {
        if s.is_empty() {
            return Err(PackageVersionError {
                kind: ErrorKind::Empty,
            });
        }
        let version = s
            .split('.')
            .map(|segment| {
                if segment != "0" && !matches!(segment.as_bytes().first(), Some(b'1'..=b'9')) {
                    return Err(PackageVersionError {
                        kind: ErrorKind::InvalidNumberStart {
                            num: segment.to_owned(),
                        },
                    });
                }
                segment.parse::<u32>().map_err(|err| PackageVersionError {
                    kind: ErrorKind::ParseIntError(err),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(version)
    }
}

impl FromStr for PackageVersion {
    type Err = PackageVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: optimize to avoid vec allocation
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PackageVersionError {
    kind: ErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("package version is empty")]
    Empty,
    #[error("number is too big: '{num}'")]
    NumberTooBig { num: u32 },
    #[error("invalid number: {num:?} (non-zero number must start from 1..9))")]
    InvalidNumberStart { num: String },
    #[error("package version as string is too long: {len} (max: {MAX_STR_LEN})")]
    TooLong { len: usize },
    #[error(transparent)]
    ParseIntError(ParseIntError),
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::PackageVersion;

    #[rstest]
    #[case::single("42")]
    #[case::double("42.42")]
    #[case::triple("42.32.22")]
    #[case::zero("0")]
    #[case::zeros("0.0.0")]
    #[case::one("1")]
    #[case::zero_major("0.1")]
    #[case::zero_minor("1.0")]
    #[case::zero_middle("1.0.2")]
    #[case::zero_tail("1.2.0")]
    #[case::four_segments("1.2.3.4")]
    #[case::maximum_segment("2147483647")]
    #[case::maximum_segment_in_version("1.2147483647.0")]
    #[case::maximum_length(&("0".to_owned() + &".123".repeat(63) + ".1"))]
    #[case::maximum_segment_count(&("0.".repeat(127) + "0"))]
    #[should_panic(expected = "package version is empty")]
    #[case::empty("")]
    #[should_panic(expected = "invalid number")]
    #[case::dot(".")]
    #[should_panic(expected = "invalid number")]
    #[case::leading_dot(".1")]
    #[should_panic(expected = "invalid number")]
    #[case::trailing_dot("1.")]
    #[should_panic(expected = "invalid number")]
    #[case::consecutive_dots("1..2")]
    #[should_panic(expected = "invalid number")]
    #[case::double_zero("00")]
    #[should_panic(expected = "invalid number")]
    #[case::leading_zero("01")]
    #[should_panic(expected = "invalid number")]
    #[case::leading_zero_later_segment("1.02")]
    #[should_panic(expected = "invalid number")]
    #[case::negative_segment("-1")]
    #[should_panic(expected = "invalid number")]
    #[case::negative_later_segment("1.-2")]
    #[should_panic(expected = "invalid number")]
    #[case::leading_plus("+1")]
    #[should_panic(expected = "invalid number")]
    #[case::plus_later_segment("1.+2")]
    #[should_panic(expected = "invalid number")]
    #[case::invalid_start("a.1")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::invalid_character("1a")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::invalid_character_later_segment("1.2a")]
    #[should_panic(expected = "invalid number")]
    #[case::leading_whitespace(" 1")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::trailing_whitespace("1 ")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::trailing_newline("1\n")]
    #[should_panic(expected = "invalid number")]
    #[case::non_ascii_digit("١")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::non_ascii_digit_after_ascii("1١")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::underscore("1_000")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::prerelease("1.2.3-beta")]
    #[should_panic(expected = "invalid digit found in string")]
    #[case::build_metadata("1.2.3+build")]
    #[should_panic(expected = "number is too big")]
    #[case::segment_too_large("2147483648")]
    #[should_panic(expected = "number is too big")]
    #[case::later_segment_too_large("1.2147483648")]
    #[should_panic(expected = "number too large to fit in target type")]
    #[case::integer_overflow("4294967296")]
    #[should_panic(expected = "package version as string is too long")]
    #[case::too_long(&("0".to_owned() + &".123".repeat(63) + ".12"))]
    #[should_panic(expected = "package version as string is too long")]
    #[case::too_many_segments(&("0.".repeat(128) + "0"))]
    fn test_parse(#[case] input: &str) {
        if let Err(err) = PackageVersion::parse(input) {
            panic!("{err}");
        }
    }
}
