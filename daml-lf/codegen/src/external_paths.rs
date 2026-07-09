use std::collections::HashMap;

use canton_types::{DottedName, PackageId};

#[derive(Clone, Debug, Default)]
pub struct UnresolvedExternalPaths {
    pub extern_packages: HashMap<String, String>,
    pub extern_modules: HashMap<String, HashMap<String, String>>,
    pub extern_entities: HashMap<String, HashMap<String, HashMap<String, String>>>,
}

impl UnresolvedExternalPaths {
    pub fn extern_package(
        &mut self,
        package_id_or_name: impl Into<String>,
        target: impl Into<String>,
    ) {
        self.extern_packages
            .insert(package_id_or_name.into(), target.into());
    }

    pub fn extern_module(
        &mut self,
        package_id_or_name: impl Into<String>,
        module: impl Into<String>,
        target: impl Into<String>,
    ) {
        let module = module.into();
        let target = target.into();
        self.extern_modules
            .entry(package_id_or_name.into())
            .and_modify(|modules| {
                modules.insert(module.clone(), target.clone());
            })
            .or_insert_with(|| [(module, target)].into());
    }

    pub fn extern_entity(
        &mut self,
        package_id_or_name: impl Into<String>,
        module: impl Into<String>,
        entity: impl Into<String>,
        target: impl Into<String>,
    ) {
        let module = module.into();
        let entity = entity.into();
        let target = target.into();
        self.extern_entities
            .entry(package_id_or_name.into())
            .and_modify(|modules| {
                modules
                    .entry(module.clone())
                    .and_modify(|entities| {
                        entities.insert(entity.clone(), target.clone());
                    })
                    .or_insert_with(|| [(entity.clone(), target.clone())].into());
            })
            .or_insert_with(|| [(module, [(entity, target)].into())].into());
    }
}

#[derive(Default)]
pub struct ExternalPaths {
    pub extern_packages: HashMap<PackageId, syn::Path>,
    pub extern_modules: HashMap<PackageId, HashMap<DottedName, syn::Path>>,
    pub extern_entities: HashMap<PackageId, HashMap<DottedName, HashMap<DottedName, syn::Path>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExternalPathsError {}
