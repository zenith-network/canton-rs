use std::{fmt, str::FromStr};

use crate::{
    package_id::{PackageId, PackageIdError},
    package_name::{DISCRIMINATOR, PackageName, PackageNameError},
};

/// Package ID in any reference format: package-id or package-name
#[derive(Clone, Debug)]
pub enum PackageIdAny {
    /// package-id reference format
    Id(PackageId),

    /// package name reference format
    Name(PackageName),
}

impl PackageIdAny {
    pub fn new(value: String) -> Result<Self, PackageIdAnyError> {
        if value.starts_with(DISCRIMINATOR) {
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
        if input.starts_with(DISCRIMINATOR) {
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

// TODO: add PartialEq, Eq, PartialOrd, Ord impls, so that you can compare any to name or id
