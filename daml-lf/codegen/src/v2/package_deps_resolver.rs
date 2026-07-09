use daml_lf::v2::sealed::Package;

use crate::v2::module_deps_resolver::ModuleDepsResolver;

pub struct PackageDepsResolver<'a> {
    package: Package<'a>,
    gen_set: PackageGenSet<'a>,
}

impl<'a> PackageDepsResolver<'a> {
    pub fn resolve(package: Package<'a>) -> Self {
        let modules = package.modules();
        let mut gen_set = PackageGenSet::new();

        for module in modules {
            let name = module.name();
            let mut resolver = ModuleDepsResolver::resolve(module);
            let module_gen_set = resolver.take_gen_set();
            gen_set.insert(name, module_gen_set);
        }

        Self { package, gen_set }
    }

    pub fn gen_set(&self) -> &PackageGenSet<'a> {
        &self.gen_set
    }
}
