use canton_types::{NonEmpty, PackageId};

pub type OwnedDottedName = NonEmpty<String>;

pub struct Identifier {
    pub package_id: PackageId,
    pub module_name: OwnedDottedName,
    pub name: OwnedDottedName,
}

pub struct IdentifierWithinPackage {
    pub module_name: OwnedDottedName,
    pub name: OwnedDottedName,
}
