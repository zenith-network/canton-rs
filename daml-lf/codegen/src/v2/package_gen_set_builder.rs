use daml_lf::v2::sealed::Package;

use crate::{
    gen_set_builder::PackageGenMode,
    type_sets::{PackageTypeSet, TypeSet},
    v2::{
        dotted_name_to_owned,
        module_gen_set_builder::{ModuleGenMode, ModuleGenSetBuilder},
    },
};

pub struct PackageGenSetBuilder;

impl PackageGenSetBuilder {
    pub fn build(package: Package<'_>, mode: PackageGenMode) -> (PackageTypeSet, TypeSet) {
        let modules = package.modules();
        let mut genset = PackageTypeSet::new();
        let mut external_deps = TypeSet::new();

        for module in modules {
            let module_name = dotted_name_to_owned(&module.name());

            let mode = match &mode {
                PackageGenMode::Full => ModuleGenMode::Full,
                PackageGenMode::ResolveTemplates => ModuleGenMode::ResolveTemplates,
                PackageGenMode::FromRoots(module_type_set) => {
                    if let Some(x) = module_type_set.get(&module_name).cloned() {
                        todo!()
                    } else {
                        continue;
                    }
                }
            };

            let (module_gen_set, local_deps, module_external_deps) =
                ModuleGenSetBuilder::build(module, mode);

            if let Some(existing_gen_set) = genset.0.get_mut(&module_name) {
                existing_gen_set.join(module_gen_set);
            } else {
                genset.0.insert(module_name, module_gen_set);
            }

            genset.join(local_deps);
            external_deps.join(module_external_deps);
        }

        (genset, external_deps)
    }
}
