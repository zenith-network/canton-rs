use std::collections::BTreeSet;

use daml_lf::v2::sealed::Module;

use crate::type_sets::{ModuleTypeSet, PackageTypeSet, TypeSet};

pub enum ModuleGenMode {
    Full,
    FromRoots(),
    ResolveTemplates,
}

pub struct ModuleGenSetBuilder;

impl ModuleGenSetBuilder {
    pub fn build(
        module: Module<'_>,
        mode: ModuleGenMode,
    ) -> (ModuleTypeSet, PackageTypeSet, TypeSet) {
        match mode {
            ModuleGenMode::Full => todo!(),
            ModuleGenMode::FromRoots() => todo!(),
            ModuleGenMode::ResolveTemplates => Self::build_resolve_templates(module),
        }
    }

    fn build_resolve_templates(module: Module<'_>) -> (ModuleTypeSet, PackageTypeSet, TypeSet) {
        let mut res = BTreeSet::new();
        let templates = module.templates();
        for template in templates {
            use crate::v2::dotted_name_to_owned;

            let template_name = dotted_name_to_owned(&template.tycon_name());
            res.insert(template_name);

            let choices = template.choices();
        }
        (ModuleTypeSet(res), todo!(), todo!())
    }
}
