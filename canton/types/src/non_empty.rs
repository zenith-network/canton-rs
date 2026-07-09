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
