use crate::{package_id::PackageId, package_id_any::PackageIdAny, package_name::PackageName};

/// Type which is used as a package identifier
///
/// Two formats are available: [`PackageId`] and [`PackageName`]. There is also [`PackageIdAny`],
/// which covers both variants.
pub trait PackageIdentifier: ToString + AsRef<str> + private::PackageIdentifier {
    /// Encode identifier in a form expected in the Protobuf string
    fn encode_proto(&self) -> String;
}

impl PackageIdentifier for PackageId {
    fn encode_proto(&self) -> String {
        self.to_string()
    }
}

impl PackageIdentifier for PackageName {
    fn encode_proto(&self) -> String {
        format!("{self:#}")
    }
}

impl PackageIdentifier for PackageIdAny {
    fn encode_proto(&self) -> String {
        match self {
            PackageIdAny::Id(package_id) => package_id.to_string(),
            PackageIdAny::Name(package_name) => format!("{package_name:#}"),
        }
    }
}

// Sealing `PackageIdentifier` trait, so that we have a fixed set of identifier types
mod private {
    use super::{PackageId, PackageIdAny, PackageName};

    pub trait PackageIdentifier {}

    impl PackageIdentifier for PackageId {}
    impl PackageIdentifier for PackageName {}
    impl PackageIdentifier for PackageIdAny {}
}
