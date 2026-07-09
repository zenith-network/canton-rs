use crate::{package_id::PackageId, package_id_any::PackageIdAny, package_name::PackageName};

/// Type which is used as a package identifier
///
/// Two formats are available: [`PackageId`] and [`PackageName`]. There is also [`PackageIdAny`],
/// which covers both variants.
pub trait PackageIdentifier: ToString + AsRef<str> + private::PackageIdentifier {}

impl PackageIdentifier for PackageId {}

impl PackageIdentifier for PackageName {}

impl PackageIdentifier for PackageIdAny {}

// Sealing `PackageIdentifier` trait, so that we have a fixed set of identifier types
mod private {
    use super::{PackageId, PackageIdAny, PackageName};

    pub trait PackageIdentifier {}

    impl PackageIdentifier for PackageId {}
    impl PackageIdentifier for PackageName {}
    impl PackageIdentifier for PackageIdAny {}
}
