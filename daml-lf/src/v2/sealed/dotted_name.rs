use std::{fmt, num::NonZeroUsize};

use canton_types::NonEmpty;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DottedName<'a>(NonEmpty<&'a str>);

impl<'a> DottedName<'a> {
    pub fn new(base: Vec<&'a str>, tail: &'a str) -> Self {
        Self(NonEmpty::new(base, tail))
    }

    pub fn base(&self) -> &[&'a str] {
        &self.0.base
    }

    pub fn tail(&self) -> &'a str {
        self.0.tail
    }

    /// Returns `None` if given iterator is empty
    pub fn try_from_iter(iter: impl Iterator<Item = &'a str>) -> Option<Self> {
        NonEmpty::try_from_iter(iter).map(Self)
    }

    pub fn iter(&self) -> impl Iterator<Item = &&'a str> {
        self.0.iter()
    }

    pub fn segments_count(&self) -> NonZeroUsize {
        self.0.len()
    }
}

impl<'a> IntoIterator for DottedName<'a> {
    type Item = <NonEmpty<&'a str> as IntoIterator>::Item;

    type IntoIter = <NonEmpty<&'a str> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> From<(Vec<&'a str>, &'a str)> for DottedName<'a> {
    fn from(value: (Vec<&'a str>, &'a str)) -> Self {
        Self(NonEmpty::from(value))
    }
}

impl<'a> PartialEq<[&str]> for DottedName<'a> {
    fn eq(&self, other: &[&str]) -> bool {
        if other.is_empty() {
            return false;
        }
        &other[..other.len() - 1] == self.base() && other[other.len() - 1] == self.tail()
    }
}

impl<'a> PartialEq<[&str]> for &DottedName<'a> {
    fn eq(&self, other: &[&str]) -> bool {
        *self == other
    }
}

impl fmt::Debug for DottedName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.0.base {
            f.write_str(segment)?;
            f.write_str(".")?;
        }
        f.write_str(self.0.tail)
    }
}
