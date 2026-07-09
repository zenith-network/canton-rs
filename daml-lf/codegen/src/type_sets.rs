use std::collections::{BTreeMap, BTreeSet};

use canton_types::PackageId;

use crate::ids::OwnedDottedName;

pub type OwnedTypeName = OwnedDottedName;

/// Set of data types in a single module
#[derive(Clone, Debug, Default)]
pub struct ModuleTypeSet(pub BTreeSet<OwnedTypeName>);

impl ModuleTypeSet {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn join(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

pub type OwnedModuleId = OwnedDottedName;

/// Set of data types in a single package (module name -> module type set)
#[derive(Clone, Debug, Default)]
pub struct PackageTypeSet(pub BTreeMap<OwnedModuleId, ModuleTypeSet>);

impl PackageTypeSet {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn join(&mut self, other: Self) {
        for (module_id, other) in other.0 {
            if let Some(module_type_set) = self.0.get_mut(&module_id) {
                module_type_set.join(other);
            } else {
                self.0.insert(module_id, other);
            }
        }
    }

    pub fn get(&self, module_id: &OwnedModuleId) -> Option<&ModuleTypeSet> {
        self.0.get(module_id)
    }
}

/// Set of data types in multiple packages
#[derive(Clone, Debug, Default)]
pub struct TypeSet(pub BTreeMap<PackageId, PackageTypeSet>);

impl TypeSet {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn join(&mut self, other: Self) {
        for (package_id, other) in other.0 {
            if let Some(package_type_set) = self.0.get_mut(&package_id) {
                package_type_set.join(other);
            } else {
                self.0.insert(package_id, other);
            }
        }
    }
}
