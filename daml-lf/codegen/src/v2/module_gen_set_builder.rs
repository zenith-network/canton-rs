use daml_lf::v2::sealed::Module;

use crate::{
    type_sets::ModuleTypeSet,
    v2::{
        dotted_name_to_owned,
        module_deps_resolver::{Deps, ModuleDepsResolver},
    },
};

pub enum ModuleGenMode {
    FromRoots(ModuleTypeSet),
    ResolveTemplates,
}

pub struct ModuleGenSetBuilder;

impl ModuleGenSetBuilder {
    pub fn build(module: Module<'_>, mode: ModuleGenMode) -> Deps {
        match mode {
            ModuleGenMode::FromRoots(roots) => Self::build_from_roots(module, roots),
            ModuleGenMode::ResolveTemplates => Self::build_resolve_templates(module),
        }
    }

    fn build_resolve_templates(module: Module<'_>) -> Deps {
        let mut deps = Deps::new();
        let resolver = ModuleDepsResolver::new(module);

        for template in module.templates() {
            let template_name = dotted_name_to_owned(&template.tycon_name());
            let template_deps = resolver.find_deps(&template_name);

            deps.direct.as_mut().insert(template_name);
            deps.extend(template_deps);

            let choices = template.choices();

            for choice in choices {
                let arg_binder_deps = resolver.find_deps_from_type(choice.arg_binder().type_());
                deps.extend(arg_binder_deps);

                let ret_type_deps = resolver.find_deps_from_type(choice.ret_type());
                deps.extend(ret_type_deps);
            }
        }

        for interface in module.interfaces() {
            Self::add_interface(&resolver, &mut deps, interface);
        }

        deps
    }

    /// An interface's view type and its choices' argument and result types: what a
    /// client needs to read the interface's views (`CreatedEvent::view`) and to
    /// exercise its choices.
    fn add_interface(
        resolver: &ModuleDepsResolver<'_>,
        deps: &mut Deps,
        interface: daml_lf::v2::sealed::DefInterface<'_>,
    ) {
        deps.extend(resolver.find_deps_from_type(interface.view()));
        for choice in interface.choices() {
            deps.extend(resolver.find_deps_from_type(choice.arg_binder().type_()));
            deps.extend(resolver.find_deps_from_type(choice.ret_type()));
        }
    }

    fn build_from_roots(module: Module<'_>, roots: ModuleTypeSet) -> Deps {
        let mut deps = Deps::new();
        let resolver = ModuleDepsResolver::new(module);

        for typename in roots {
            let type_deps = resolver.find_deps(&typename);
            // A root that is an interface (reached as `ContractId I`) brings its view
            // and choice types with it.
            if let Some(interface) = module
                .interfaces()
                .into_iter()
                .find(|i| dotted_name_to_owned(&i.tycon_name()) == typename)
            {
                Self::add_interface(&resolver, &mut deps, interface);
            }
            deps.direct.as_mut().insert(typename);
            deps.extend(type_deps);
        }

        deps
    }
}
