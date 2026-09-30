use std::{error::Error as StdError, fmt, str::FromStr};

use canton_types::{PackageId, PackageName};

#[derive(Debug, thiserror::Error)]
#[error("invalid package reference: '{content}'")]
pub struct InvalidPackageRef {
    content: String,
    #[source]
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl InvalidPackageRef {
    fn new_with_source(
        content: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            content: content.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Package ref formats:
///
/// - `#my_package_name`
/// - `#my_package_name@v0.1.0`
/// - `mypackageidffff`
#[derive(Clone, Debug)]
pub enum PackageRef {
    Id(PackageId),
    Name {
        name: PackageName,
        version: Option<String>,
    },
}

impl fmt::Display for PackageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageRef::Id(package_id) => package_id.fmt(f),
            PackageRef::Name { name, version } => {
                if let Some(version) = version {
                    write!(f, "{}@{}", name, version)
                } else {
                    write!(f, "{}", name)
                }
            }
        }
    }
}

impl PackageRef {
    pub fn parse_str(input: &str) -> Result<Self, InvalidPackageRef> {
        if let Some(name_version) = input.strip_prefix('#') {
            let (name, version) = Self::parse_name_version(name_version)?;
            Ok(Self::Name { name, version })
        } else {
            let package_id = PackageId::new(input.to_owned())
                .map_err(|error| InvalidPackageRef::new_with_source(input, error))?;
            Ok(Self::Id(package_id))
        }
    }

    fn parse_name_version(
        name_version: &str,
    ) -> Result<(PackageName, Option<String>), InvalidPackageRef> {
        let (raw_name, version) =
            if let Some((raw_name, raw_version)) = name_version.split_once('@') {
                (raw_name, Some(raw_version.to_owned()))
            } else {
                (name_version, None)
            };

        let name = PackageName::new(raw_name.to_owned())
            .map_err(|error| InvalidPackageRef::new_with_source(name_version, error))?;

        Ok((name, version))
    }
}

impl FromStr for PackageRef {
    type Err = InvalidPackageRef;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}
