use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use canton_types::PackageId;
use daml_lf::{
    proto::daml_lf_version::Version,
    v2::sealed::{PackageMetadata, SealedPackage},
};
use syn::Ident;
use tracing::debug;

use crate::{
    external_paths::ExternalPaths,
    helpers::{empty_mod, is_empty_mod, mod_with_items, push_module},
    type_sets::PackageTypeSet,
    v2::{
        dotted_name_to_owned,
        module_generator::{ModuleGenError, ModuleGenerator},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum PackageGenError {
    #[error("failed to generate module <{module}>")]
    ModuleGenError {
        module: String,
        #[source]
        source: ModuleGenError,
    },
}

pub struct PackageGenerator<'a> {
    daml_lf_version: Version,
    package_id: PackageId,
    package: SealedPackage<'a>,
    package_identifier: Ident,
    package_identifiers: Arc<HashMap<PackageId, Ident>>,
    external_paths: Arc<ExternalPaths>,
    gen_set: PackageTypeSet,
}

impl<'a> PackageGenerator<'a> {
    pub fn new(
        daml_lf_version: Version,
        package_id: PackageId,
        package: SealedPackage<'a>,
        package_identifier: Ident,
        package_identifiers: Arc<HashMap<PackageId, Ident>>,
        external_paths: Arc<ExternalPaths>,
        gen_set: PackageTypeSet,
    ) -> Self {
        Self {
            daml_lf_version,
            package_id,
            package,
            package_identifier,
            package_identifiers,
            external_paths,
            gen_set,
        }
    }

    /// Generate module for package `pub mod package_XXX { ... }`
    pub fn gen_package(&mut self) -> Result<syn::ItemMod, PackageGenError> {
        let metadata = self.package.metadata();
        let name = metadata.name();
        let version = metadata.version();
        let package_id = self.package_id.as_str();
        let daml_lf_version = self.daml_lf_version.to_string();
        debug!(
            name,
            version, package_id, daml_lf_version, "Entering package"
        );

        let package_header = self.gen_header(&metadata);
        let mut package_module = mod_with_items(self.package_identifier.clone(), package_header);

        // `true` if all the modules inside the package are currently empty
        // If finally this is true, we emit empty module without the header
        let mut empty = true;

        let modules = self
            .package
            .modules()
            .into_iter()
            .map(|module| (dotted_name_to_owned(&module.name()), module))
            .collect::<BTreeMap<_, _>>();

        for (module_name, module_gen_set) in &self.gen_set.0 {
            let module = modules[module_name];

            let rmodule = ModuleGenerator::new(
                self.package_identifiers.clone(),
                self.external_paths.clone(),
                module,
                module_gen_set.clone(),
            )
            .gen_module()
            .map_err(|err| PackageGenError::ModuleGenError {
                module: format!("{module:#?}"),
                source: err,
            })?;

            if !is_empty_mod(&rmodule) {
                empty = false;
                let module_name = module.name();
                let parent_path = module_name.base();
                push_module(&mut package_module, parent_path, rmodule);
            }
        }

        let result = if empty {
            empty_mod(self.package_identifier.clone())
        } else {
            package_module
        };

        Ok(result)
    }

    fn gen_header(&self, metadata: &PackageMetadata<'a>) -> Vec<syn::Item> {
        let name = metadata.name();
        let version = metadata.version();
        let package_id = self.package_id.as_str();
        let package_id = syn::parse_quote! {
            pub const PACKAGE_ID: ::canton::types::PackageId =
                ::canton::types::PackageId::new_unchecked(#package_id);
        };
        let package_name = syn::parse_quote! {
            pub const PACKAGE_NAME: ::canton::types::PackageName =
                ::canton::types::PackageName::new_unchecked(#name);
        };
        let package_version = syn::parse_quote! {
            pub const PACKAGE_VERSION: &str = #version;
        };
        vec![package_id, package_name, package_version]
    }
}
