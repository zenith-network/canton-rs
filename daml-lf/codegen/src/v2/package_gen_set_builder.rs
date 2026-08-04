use daml_lf::v2::sealed::Package;

use crate::{
    gen_set_builder::{PackageGenMode, PackageGenSetResult},
    type_sets::{PackageTypeSet, TypeSet},
    v2::{
        dotted_name_to_owned,
        module_deps_resolver::Deps,
        module_gen_set_builder::{ModuleGenMode, ModuleGenSetBuilder},
    },
};

pub struct PackageGenSetBuilder;

impl PackageGenSetBuilder {
    pub fn build(package: Package<'_>, mode: PackageGenMode) -> PackageGenSetResult {
        let modules = package.modules();
        let mut genset = PackageTypeSet::new();
        let mut external_deps = TypeSet::new();

        for module in modules {
            let module_name = dotted_name_to_owned(&module.name());

            let mode = match &mode {
                PackageGenMode::ResolveTemplates => ModuleGenMode::ResolveTemplates,
                PackageGenMode::FromRoots(module_type_set) => {
                    if let Some(x) = module_type_set.get(&module_name).cloned() {
                        ModuleGenMode::FromRoots(x)
                    } else {
                        // this module is not mentioned, skip it
                        continue;
                    }
                }
            };

            let Deps {
                direct: module_genset,
                local: local_deps,
                external: module_external_deps,
            } = ModuleGenSetBuilder::build(module, mode);

            if let Some(existing_gen_set) = genset.0.get_mut(&module_name) {
                existing_gen_set.join(module_genset);
            } else {
                genset.0.insert(module_name, module_genset);
            }

            genset.join(local_deps);
            external_deps.join(module_external_deps);
        }

        PackageGenSetResult {
            genset,
            external_deps,
        }
    }
}
