use std::{iter, num::NonZeroUsize, vec::IntoIter};

/// Non-empty sequence with a tail element
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmpty<T> {
    pub base: Vec<T>,
    pub tail: T,
}

impl<T> NonEmpty<T> {
    pub const fn new(base: Vec<T>, tail: T) -> Self {
        Self { base, tail }
    }

    pub const fn single(tail: T) -> Self {
        Self {
            base: Vec::new(),
            tail,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.base.iter().chain(iter::once(&self.tail))
    }

    /// Returns `None` if given iterator is empty
    pub fn try_from_iter(iter: impl Iterator<Item = T>) -> Option<Self> {
        let mut base = Vec::new();
        let mut tail = None;
        for elem in iter {
            if let Some(old_tail) = tail {
                base.push(old_tail);
            }
            tail = Some(elem);
        }
        tail.map(|tail| Self { base, tail })
    }

    pub const fn len(&self) -> NonZeroUsize {
        unsafe { NonZeroUsize::new_unchecked(self.base.len() + 1) }
    }

    pub fn as_ref(&self) -> NonEmpty<&T> {
        NonEmpty {
            base: self.base.iter().collect(),
            tail: &self.tail,
        }
    }
}

impl<T> IntoIterator for NonEmpty<T> {
    type Item = T;

    type IntoIter = iter::Chain<IntoIter<T>, iter::Once<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().chain(iter::once(self.tail))
    }
}

impl<T> From<(Vec<T>, T)> for NonEmpty<T> {
    fn from((base, tail): (Vec<T>, T)) -> Self {
        Self { base, tail }
    }
}

impl<T> From<NonEmpty<T>> for (Vec<T>, T) {
    fn from(value: NonEmpty<T>) -> Self {
        (value.base, value.tail)
    }
}

impl<T> From<NonEmpty<T>> for Vec<T> {
    fn from(mut value: NonEmpty<T>) -> Self {
        value.base.push(value.tail);
        value.base
    }
}

/// This error is returned when a conversion from vector to non-empty fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("source vector is empty")]
pub struct EmptyVecError(());

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = EmptyVecError;

    /// Try to create [`NonEmpty`] from a vector and return error if given vector is empty.
    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        let mut base = value;
        let tail = base.pop().ok_or(EmptyVecError(()))?;
        Ok(Self { base, tail })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{EmptyVecError, NonEmpty};

    #[rstest]
    #[case::empty(vec![], Err(EmptyVecError(())))]
    #[case::one_elem(vec![1], Ok(NonEmpty { base: Vec::new(), tail: 1 }))]
    #[case::three_elem(vec![1, 2, 3], Ok(NonEmpty { base: vec![1, 2], tail: 3 }))]
    fn test_try_from_vec(
        #[case] source: Vec<u32>,
        #[case] result: Result<NonEmpty<u32>, EmptyVecError>,
    ) {
        assert_eq!(NonEmpty::try_from(source), result);
    }

    #[rstest]
    #[case::single(NonEmpty::single(42), vec![42])]
    #[case::multiple(NonEmpty { base: vec![1, 2], tail: 3 }, vec![1, 2, 3])]
    fn test_into_vec(#[case] source: NonEmpty<u32>, #[case] expected: Vec<u32>) {
        assert_eq!(Vec::from(source), expected);
    }
}
