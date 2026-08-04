use std::collections::{BTreeMap, BTreeSet, btree_map, btree_set};

use canton_types::PackageId;

use crate::ids::OwnedDottedName;

pub type OwnedTypeName = OwnedDottedName;

/// Set of data types in a single module
#[derive(Clone, Debug, Default)]
pub struct ModuleTypeSet(pub BTreeSet<OwnedTypeName>);

impl AsRef<BTreeSet<OwnedTypeName>> for ModuleTypeSet {
    fn as_ref(&self) -> &BTreeSet<OwnedTypeName> {
        &self.0
    }
}

impl AsMut<BTreeSet<OwnedTypeName>> for ModuleTypeSet {
    fn as_mut(&mut self) -> &mut BTreeSet<OwnedTypeName> {
        &mut self.0
    }
}

impl IntoIterator for ModuleTypeSet {
    type Item = OwnedTypeName;

    type IntoIter = btree_set::IntoIter<OwnedTypeName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ModuleTypeSet {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn join(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

pub type OwnedModuleName = OwnedDottedName;

/// Set of data types in a single package (module name -> module type set)
#[derive(Clone, Debug, Default)]
pub struct PackageTypeSet(pub BTreeMap<OwnedModuleName, ModuleTypeSet>);

impl AsRef<BTreeMap<OwnedModuleName, ModuleTypeSet>> for PackageTypeSet {
    fn as_ref(&self) -> &BTreeMap<OwnedModuleName, ModuleTypeSet> {
        &self.0
    }
}

impl AsMut<BTreeMap<OwnedModuleName, ModuleTypeSet>> for PackageTypeSet {
    fn as_mut(&mut self) -> &mut BTreeMap<OwnedModuleName, ModuleTypeSet> {
        &mut self.0
    }
}

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

    pub fn insert_type(&mut self, module_name: OwnedModuleName, type_name: OwnedTypeName) {
        if let Some(existing_module) = self.0.get_mut(&module_name) {
            existing_module.0.insert(type_name);
        } else {
            self.0
                .insert(module_name, ModuleTypeSet(BTreeSet::from([type_name])));
        }
    }

    pub fn get(&self, module_id: &OwnedModuleName) -> Option<&ModuleTypeSet> {
        self.0.get(module_id)
    }
}

/// Set of data types in multiple packages
#[derive(Clone, Debug, Default)]
pub struct TypeSet(pub BTreeMap<PackageId, PackageTypeSet>);

impl AsRef<BTreeMap<PackageId, PackageTypeSet>> for TypeSet {
    fn as_ref(&self) -> &BTreeMap<PackageId, PackageTypeSet> {
        &self.0
    }
}

impl AsMut<BTreeMap<PackageId, PackageTypeSet>> for TypeSet {
    fn as_mut(&mut self) -> &mut BTreeMap<PackageId, PackageTypeSet> {
        &mut self.0
    }
}

impl IntoIterator for TypeSet {
    type Item = (PackageId, PackageTypeSet);

    type IntoIter = btree_map::IntoIter<PackageId, PackageTypeSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

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

    pub fn insert_type(
        &mut self,
        package_id: PackageId,
        module_name: OwnedModuleName,
        type_name: OwnedTypeName,
    ) {
        if let Some(existing_package) = self.0.get_mut(&package_id) {
            existing_package.insert_type(module_name, type_name);
        } else {
            self.0.insert(
                package_id,
                PackageTypeSet(BTreeMap::from([(
                    module_name,
                    ModuleTypeSet(BTreeSet::from([type_name])),
                )])),
            );
        }
    }
}
