use std::fmt;

use canton_types::{DottedName, PackageId, PackageIdAny, PackageIdentifier, PackageName};
use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;

use super::errors::{IntoValueError as _, ValueError};

/// Identifier of an entity (template, interface, data type, etc.)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier<T: PackageIdentifier = PackageIdAny> {
    pub package_id: T,
    pub module_name: DottedName,
    pub entity_name: DottedName,
}

impl Identifier<PackageName> {
    pub fn into_any(self) -> Identifier {
        Identifier {
            package_id: self.package_id.into_any(),
            module_name: self.module_name,
            entity_name: self.entity_name,
        }
    }
}

impl Identifier<PackageId> {
    pub fn into_any(self) -> Identifier {
        Identifier {
            package_id: self.package_id.into_any(),
            module_name: self.module_name,
            entity_name: self.entity_name,
        }
    }
}

impl<T: PackageIdentifier> fmt::Display for Identifier<T> {
    /// Default formatting is `<pkg_id>:<module_name>:<entity_name>`
    ///
    /// Alternate formatting (`#`) will print a short format:
    /// `<module_name>:<entity_name>@<short_pkg_id>`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{}:{}@{}",
                self.module_name,
                self.entity_name,
                short_pkg_id(&self.package_id)
            )
        } else {
            write!(
                f,
                "{}:{}:{}",
                self.package_id.as_ref(),
                self.module_name,
                self.entity_name
            )
        }
    }
}

fn short_pkg_id<T: PackageIdentifier>(pkg_id: &T) -> &str {
    let string = pkg_id.as_ref();
    match string.char_indices().nth(8) {
        Some((idx, _)) => &string[..idx],
        None => string,
    }
}

impl<T: PackageIdentifier> From<Identifier<T>> for proto::Identifier {
    fn from(value: Identifier<T>) -> Self {
        Self {
            package_id: value.package_id.to_string(),
            module_name: value.module_name.join(),
            entity_name: value.entity_name.join(),
        }
    }
}

impl TryFrom<proto::Identifier> for Identifier {
    type Error = ValueError;

    fn try_from(value: proto::Identifier) -> Result<Self, Self::Error> {
        Ok(Self {
            package_id: PackageIdAny::new(value.package_id).no_msg()?,
            module_name: DottedName::parse(&value.module_name).no_msg()?,
            entity_name: DottedName::parse(&value.entity_name).no_msg()?,
        })
    }
}

impl TryFrom<proto::Identifier> for Identifier<PackageId> {
    type Error = ValueError;

    fn try_from(value: proto::Identifier) -> Result<Self, Self::Error> {
        Ok(Self {
            package_id: PackageId::new(value.package_id).no_msg()?,
            module_name: DottedName::parse(&value.module_name).no_msg()?,
            entity_name: DottedName::parse(&value.entity_name).no_msg()?,
        })
    }
}

impl TryFrom<proto::Identifier> for Identifier<PackageName> {
    type Error = ValueError;

    fn try_from(value: proto::Identifier) -> Result<Self, Self::Error> {
        Ok(Self {
            package_id: PackageName::new(value.package_id).no_msg()?,
            module_name: DottedName::parse(&value.module_name).no_msg()?,
            entity_name: DottedName::parse(&value.entity_name).no_msg()?,
        })
    }
}

/// Type which has an attached [`Identifier`]
pub trait HasIdentifier {
    /// Get assigned package ID of the type
    fn package_id() -> PackageId;

    /// Get assigned package name of the type
    fn package_name() -> PackageName;

    /// Get assigned module name of the type
    fn module_name() -> DottedName;

    /// Get assigned entity name of the type
    fn entity_name() -> DottedName;

    /// Get identifier of the type (package-name reference format)
    fn identifier() -> Identifier<PackageName> {
        Identifier {
            package_id: Self::package_name(),
            module_name: Self::module_name(),
            entity_name: Self::entity_name(),
        }
    }

    /// Get identifier of the type (package-id reference format)
    fn identifier_with_package_id() -> Identifier<PackageId> {
        Identifier {
            package_id: Self::package_id(),
            module_name: Self::module_name(),
            entity_name: Self::entity_name(),
        }
    }
}
