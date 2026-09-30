use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

/// Transparent newtype for `BTreeMap<String, T>`
///
/// Represents Daml Text map, needed to distinguish type from Gen map.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TextMap<T>(pub BTreeMap<String, T>);

impl<T> Deref for TextMap<T> {
    type Target = BTreeMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TextMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
