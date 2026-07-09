use std::{
    fmt::{self, Write as _},
    num::NonZeroUsize,
    str::FromStr,
};

use crate::{NonEmpty, errors::NameError, name::Name};

/// Max DottedName length
const MAX_LEN: usize = 1000;

const DOT: char = '.';

/// Dotted name (like `"Mod.Submob.A"`)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DottedName {
    segments: NonEmpty<Name>,
}

impl DottedName {
    /// Create a DottedName from a single segment
    pub fn single(name: Name) -> Self {
        Self {
            segments: NonEmpty::single(name),
        }
    }

    /// Create DottedName from given segments
    pub fn from_segments(segments: NonEmpty<Name>) -> Self {
        Self { segments }
    }

    /// Returns segments of this DottedName
    pub fn segments(&self) -> &NonEmpty<Name> {
        &self.segments
    }

    /// Returns the length of this DottedName, in bytes, not chars or graphemes.
    ///
    /// This length includes all symbols of all segments and dots between them. This value
    /// represents full length of the joined string.
    ///
    /// If you need to get the number of segments, use `self.segments().len()` instead.
    pub fn len(&self) -> NonZeroUsize {
        // Count joining dots
        let dots_count = usize::from(self.segments.len()) - 1;

        let total_segments_len = self
            .segments
            .iter()
            .fold(0, |acc, segment| acc + usize::from(segment.len()));
        debug_assert!(total_segments_len > 0);

        unsafe { NonZeroUsize::new_unchecked(total_segments_len + dots_count) }
    }

    /// Join DottedName into a single string with `.`
    pub fn join(&self) -> String {
        // calculate size to pre-allocate enough capacity in resulting string
        let mut result = String::with_capacity(self.len().into());
        for segment in &self.segments.base {
            result.push_str(segment);
            result.push(DOT);
        }
        result.push_str(&self.segments.tail);
        result
    }

    /// Parse string into dotted name
    pub fn parse(input: impl AsRef<str>) -> Result<Self, DottedNameError> {
        let input = input.as_ref();

        if input.len() > MAX_LEN {
            return Err(DottedNameError {
                kind: DottedNameErrorKind::TooLong,
            });
        }

        let split = input
            .split(DOT)
            .enumerate()
            .map(|(idx, segment)| {
                Name::new(segment.to_owned()).map_err(|source| DottedNameError {
                    kind: DottedNameErrorKind::NameError { idx, source },
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        NonEmpty::try_from_iter(split.into_iter())
            .ok_or(DottedNameError {
                kind: DottedNameErrorKind::Empty,
            })
            .map(|segments| Self { segments })
    }
}

impl FromStr for DottedName {
    type Err = DottedNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Debug for DottedName {
    /// Custom implementation to print dotted name in string-like format `"a.b.c"`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        for segment in &self.segments.base {
            f.write_str(segment)?;
            f.write_char(DOT)?;
        }
        f.write_str(&self.segments.tail)?;
        f.write_char('"')
    }
}

impl fmt::Display for DottedName {
    /// Prints in joined format `a.b.c`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments.base {
            f.write_str(segment)?;
            f.write_char(DOT)?;
        }
        f.write_str(&self.segments.tail)
    }
}

impl PartialEq<[&str]> for DottedName {
    fn eq(&self, other: &[&str]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

impl PartialEq<[&str]> for &DottedName {
    fn eq(&self, other: &[&str]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

impl PartialEq<&[&str]> for DottedName {
    fn eq(&self, other: &&[&str]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

impl PartialEq<[&String]> for DottedName {
    fn eq(&self, other: &[&String]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

impl PartialEq<[&String]> for &DottedName {
    fn eq(&self, other: &[&String]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

impl PartialEq<&[&String]> for DottedName {
    fn eq(&self, other: &&[&String]) -> bool {
        usize::from(self.segments.len()) == other.len()
            && self
                .segments
                .iter()
                .zip(other.iter())
                .fold(true, |acc, (self_, other)| acc && self_ == other)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid dotted name: {kind}")]
pub struct DottedNameError {
    kind: DottedNameErrorKind,
}

#[derive(Debug, thiserror::Error)]
enum DottedNameErrorKind {
    #[error("dotted name is empty")]
    Empty,
    #[error("dotted name is too long (max: {MAX_LEN})")]
    TooLong,
    #[error("segment #{idx} is invalid")]
    NameError {
        idx: usize,
        #[source]
        source: NameError,
    },
}

// TODO: Maybe this implementation is not the best, cause it forces allocations in many use-cases.
//       Alternatively we can store Cow<'static, str> alongside with non-empty list of dot indices.
//       This way it's gonne be cheap to get joined version of the dotted name.
