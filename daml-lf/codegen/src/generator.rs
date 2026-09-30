#![allow(dead_code, reason = "Ignoring external paths for now")]

use std::collections::BTreeMap;

use canton_paths::Paths;
use canton_types::PackageId;
use daml_lf::package::VersionedSealedPackage;

use crate::{
    dispatcher::Packages,
    external_paths::ExternalPaths,
    ir::{Definition, GenGraph, GeneratedModule, GeneratedPackage},
    type_attributes::TypeAttributes,
};

/// Generation context
#[derive(Clone, Debug)]
pub struct GenCtx<'a> {
    packages: &'a Packages<'a>,
    paths: Paths,
    external_paths: ExternalPaths,
    type_attrs: TypeAttributes,
}

impl<'a> GenCtx<'a> {
    /// Create new generation context
    pub fn new(
        packages: &'a Packages<'a>,
        paths: Paths,
        external_paths: ExternalPaths,
        type_attrs: TypeAttributes,
    ) -> Self {
        Self {
            packages,
            paths,
            external_paths,
            type_attrs,
        }
    }

    /// All packages map
    pub fn packages(&self) -> &'a Packages<'a> {
        self.packages
    }

    /// Canton crate paths
    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// Configured additional type attributes
    pub fn type_attributes(&self) -> &TypeAttributes {
        &self.type_attrs
    }

    /// Configured external paths
    pub fn external_paths(&self) -> &ExternalPaths {
        &self.external_paths
    }
}

/// Produces Rust syntax tree from generation graph
pub struct Generator<'a> {
    ctx: &'a GenCtx<'a>,
    generated: BTreeMap<PackageId, GeneratedPackage>,
}

impl<'a> Generator<'a> {
    pub fn generate(
        graph: GenGraph<'a>,
        ctx: &'a GenCtx<'a>,
    ) -> BTreeMap<PackageId, GeneratedPackage> {
        let mut generator = Self {
            ctx,
            generated: BTreeMap::new(),
        };
        for node in graph.nodes() {
            generator.generate_node(*node);
        }
        generator.generated
    }

    fn generate_node(&mut self, node: Definition<'_>) {
        let package = node.package();
        let package_id = node.package_id();

        let item = ItemGenerator::generate(node, self.ctx);

        let generated_package = self.generated.entry(package_id.clone()).or_insert_with(|| {
            let package_ident = &self.ctx.packages()[package_id].ident;
            let header = ItemGenerator::generate_package_header(package_id, package);
            GeneratedPackage::new(package_ident.clone(), header)
        });

        Self::insert_generated_item(generated_package, item, node);
    }

    fn insert_generated_item(
        package: &mut GeneratedPackage,
        item: syn::Item,
        node: Definition<'_>,
    ) {
        match node {
            #[cfg(feature = "v2")]
            Definition::V2(node) => {
                use crate::v2::item_generator::ItemGenerator as ItemGeneratorV2;

                let daml_module = node.module();
                let mut module_path = crate::path::generate_module_path(daml_module.name());
                let daml_module_depth = module_path.segments.len();
                let entity_name = node.definition().name();
                module_path
                    .segments
                    .extend(crate::path::generate_module_path(entity_name.base()).segments);

                let mut modules = package.modules_mut();

                // Since module name is non-empty, this is guaranteed to be non-empty too
                let mut module_path = module_path
                    .segments
                    .into_iter()
                    .map(|s| s.ident)
                    .enumerate()
                    .peekable();

                while let Some((depth, module_ident)) = module_path.next() {
                    let module_idx = if let Some(idx) =
                        modules.iter().position(|m| m.ident() == &module_ident)
                    {
                        idx
                    } else {
                        modules.push(GeneratedModule::new(module_ident));
                        modules.len() - 1
                    };

                    let module = &mut modules[module_idx];

                    if depth + 1 == daml_module_depth && module.header().is_empty() {
                        module.set_header(ItemGeneratorV2::generate_module_header(daml_module));
                    }

                    if module_path.peek().is_none() {
                        module.push_item(item);
                        return;
                    }

                    modules = module.submodules_mut();
                }

                // Since the module path is non-empty and finite, the return statement will be hit
                // eventually, thus this is unreachable
                unreachable!()
            }
        }
    }
}

pub struct ItemGenerator {}

impl ItemGenerator {
    pub fn generate<'a>(node: Definition<'a>, ctx: &'a GenCtx) -> syn::Item {
        match node {
            #[cfg(feature = "v2")]
            Definition::V2(node) => {
                use crate::v2::item_generator::ItemGenerator as ItemGeneratorV2;

                ItemGeneratorV2::generate(node, ctx)
            }
        }
    }

    pub fn generate_package_header(
        package_id: &PackageId,
        package: VersionedSealedPackage<'_>,
    ) -> Vec<syn::Item> {
        match package {
            #[cfg(feature = "v2")]
            VersionedSealedPackage::V2(package_v2) => {
                use crate::v2::item_generator::ItemGenerator as ItemGeneratorV2;

                ItemGeneratorV2::generate_package_header(package_id, package_v2)
            }
        }
    }
}
