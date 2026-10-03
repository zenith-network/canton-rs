use std::{fmt, str::FromStr};

use crate::{
    package_id::{PackageId, PackageIdError},
    package_name::{DISCRIMINATOR, PackageName, PackageNameError},
};

/// Package ID in any reference format: package-id or package-name
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageIdAny {
    /// package-id reference format
    Id(PackageId),

    /// package name reference format
    Name(PackageName),
}

impl PackageIdAny {
    pub fn new(mut value: String) -> Result<Self, PackageIdAnyError> {
        if value.starts_with(DISCRIMINATOR) {
            value.remove(0);
            Self::new_name(value)
        } else {
            Self::new_id(value)
        }
    }

    pub fn new_id(value: String) -> Result<Self, PackageIdAnyError> {
        Ok(Self::Id(PackageId::new(value)?))
    }

    pub fn new_name(value: String) -> Result<Self, PackageIdAnyError> {
        Ok(Self::Name(PackageName::new(value)?))
    }

    pub const fn is_id(&self) -> bool {
        matches!(self, Self::Id(_))
    }

    pub const fn is_name(&self) -> bool {
        matches!(self, Self::Name(_))
    }

    pub const fn as_id(&self) -> Option<&PackageId> {
        match self {
            Self::Id(package_id) => Some(package_id),
            Self::Name(_) => None,
        }
    }

    pub const fn as_name(&self) -> Option<&PackageName> {
        match self {
            Self::Id(_) => None,
            Self::Name(package_name) => Some(package_name),
        }
    }

    pub fn into_id(self) -> Option<PackageId> {
        match self {
            Self::Id(package_id) => Some(package_id),
            Self::Name(_) => None,
        }
    }

    pub fn into_name(self) -> Option<PackageName> {
        match self {
            Self::Id(_) => None,
            Self::Name(package_name) => Some(package_name),
        }
    }

    pub const fn as_str(&self) -> &str {
        match self {
            Self::Id(package_id) => package_id.as_str(),
            Self::Name(package_name) => package_name.as_str(),
        }
    }

    pub fn parse(input: impl AsRef<str>) -> Result<Self, PackageIdAnyError> {
        let input = input.as_ref();
        if let Some(input) = input.strip_prefix(DISCRIMINATOR) {
            Self::new_name(input.to_owned())
        } else {
            Self::new_id(input.to_owned())
        }
    }
}

impl From<PackageId> for PackageIdAny {
    fn from(id: PackageId) -> Self {
        Self::Id(id)
    }
}

impl From<PackageName> for PackageIdAny {
    fn from(name: PackageName) -> Self {
        Self::Name(name)
    }
}

impl AsRef<str> for PackageIdAny {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for PackageIdAny {
    type Err = PackageIdAnyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for PackageIdAny {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(package_id) => package_id.fmt(f),
            Self::Name(package_name) => package_name.fmt(f),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct PackageIdAnyError {
    kind: ErrorKind,
}

impl From<PackageIdError> for PackageIdAnyError {
    fn from(error: PackageIdError) -> Self {
        Self {
            kind: ErrorKind::PackageId(error),
        }
    }
}

impl From<PackageNameError> for PackageIdAnyError {
    fn from(error: PackageNameError) -> Self {
        Self {
            kind: ErrorKind::PackageName(error),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error(transparent)]
    PackageId(PackageIdError),
    #[error(transparent)]
    PackageName(PackageNameError),
}

impl PartialEq<PackageId> for PackageIdAny {
    fn eq(&self, other: &PackageId) -> bool {
        match self {
            Self::Id(id) => id == other,
            Self::Name(_) => false,
        }
    }
}

impl PartialEq<&PackageId> for PackageIdAny {
    fn eq(&self, other: &&PackageId) -> bool {
        self == *other
    }
}

impl PartialEq<PackageIdAny> for PackageId {
    fn eq(&self, other: &PackageIdAny) -> bool {
        other == self
    }
}

impl PartialEq<PackageIdAny> for &PackageId {
    fn eq(&self, other: &PackageIdAny) -> bool {
        *self == other
    }
}

impl PartialEq<PackageName> for PackageIdAny {
    fn eq(&self, other: &PackageName) -> bool {
        match self {
            Self::Id(_) => false,
            Self::Name(name) => name == other,
        }
    }
}

impl PartialEq<&PackageName> for PackageIdAny {
    fn eq(&self, other: &&PackageName) -> bool {
        self == *other
    }
}

impl PartialEq<PackageIdAny> for PackageName {
    fn eq(&self, other: &PackageIdAny) -> bool {
        other == self
    }
}

impl PartialEq<PackageIdAny> for &PackageName {
    fn eq(&self, other: &PackageIdAny) -> bool {
        *self == other
    }
}

impl PartialOrd<PackageId> for PackageIdAny {
    fn partial_cmp(&self, other: &PackageId) -> Option<std::cmp::Ordering> {
        match self {
            Self::Id(id) => id.partial_cmp(other),
            Self::Name(_) => None,
        }
    }
}

impl PartialOrd<PackageIdAny> for PackageId {
    fn partial_cmp(&self, other: &PackageIdAny) -> Option<std::cmp::Ordering> {
        match other {
            PackageIdAny::Id(id) => self.partial_cmp(id),
            PackageIdAny::Name(_) => None,
        }
    }
}

impl PartialOrd<PackageName> for PackageIdAny {
    fn partial_cmp(&self, other: &PackageName) -> Option<std::cmp::Ordering> {
        match self {
            Self::Id(_) => None,
            Self::Name(name) => name.partial_cmp(other),
        }
    }
}

impl PartialOrd<PackageIdAny> for PackageName {
    fn partial_cmp(&self, other: &PackageIdAny) -> Option<std::cmp::Ordering> {
        match other {
            PackageIdAny::Id(_) => None,
            PackageIdAny::Name(name) => self.partial_cmp(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_package_id_any_id_equality() {
        let id = PackageId::new_unchecked("pkg-123");
        let any_id = PackageIdAny::new_id("pkg-123".into()).unwrap();
        let any_id2 = PackageIdAny::new_id("pkg-456".into()).unwrap();

        assert_eq!(any_id, id);
        assert_eq!(&any_id, &id);
        assert_eq!(id, any_id);
        assert_eq!(&id, &any_id);
        assert_ne!(any_id2, id);

        assert_eq!(any_id.as_str(), "pkg-123");
    }

    #[test]
    fn test_package_id_any_name_equality() {
        let name = PackageName::new_unchecked("my-pkg");
        let any_name = PackageIdAny::new_name("my-pkg".into()).unwrap();
        let any_name2 = PackageIdAny::new_name("other-pkg".into()).unwrap();

        assert_eq!(any_name, name);
        assert_eq!(&any_name, &name);
        assert_eq!(name, any_name);
        assert_eq!(&name, &any_name);
        assert_ne!(any_name2, name);

        assert_eq!(any_name.as_str(), "my-pkg");
    }

    #[test]
    fn test_package_id_any_cross_equality() {
        let id = PackageId::new_unchecked("pkg-123");
        let name = PackageName::new_unchecked("pkg-123");
        let any_id = PackageIdAny::new_id("pkg-123".into()).unwrap();
        let any_name = PackageIdAny::new_name("pkg-123".into()).unwrap();

        assert_eq!(any_id, id);
        assert_ne!(any_id, name);
        assert_eq!(any_name, name);
        assert_ne!(any_name, id);
        assert_ne!(any_id, any_name);
    }

    #[test]
    fn test_package_id_any_partial_ord() {
        let id1 = PackageId::new_unchecked("aaa");
        let id2 = PackageId::new_unchecked("bbb");
        let any_id1 = PackageIdAny::new_id("aaa".into()).unwrap();
        let any_id2 = PackageIdAny::new_id("bbb".into()).unwrap();
        let any_name = PackageIdAny::new_name("aaa".into()).unwrap();

        assert!(any_id1 < id2);
        assert!(id1 < any_id2);
        assert!(any_id1 < any_id2);
        assert_eq!(
            any_id1.partial_cmp(&any_name),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(any_name.partial_cmp(&id1), None);
    }

    #[test]
    fn test_package_id_any_hash_and_collections() {
        let any_id = PackageIdAny::new_id("pkg-123".into()).unwrap();
        let any_name = PackageIdAny::new_name("pkg-123".into()).unwrap();

        let mut set = HashSet::new();
        set.insert(any_id.clone());
        set.insert(any_name.clone());

        assert_eq!(set.len(), 2);
        assert!(set.contains(&any_id));
        assert!(set.contains(&any_name));
    }

    #[test]
    fn test_parse() {
        let id = PackageIdAny::parse("my-package-id").unwrap();
        assert!(id.is_id());
        assert_eq!(id.as_str(), "my-package-id");

        let name = PackageIdAny::parse("#my-package-name").unwrap();
        assert!(name.is_name());
        assert_eq!(name.as_str(), "my-package-name");
    }
}
