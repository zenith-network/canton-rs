use std::collections::{BTreeMap, HashMap};

use canton_types::PackageId;
use daml_lf::v2::sealed::{
    DefDataType, DottedName, Module, SelfOrImportedPackageId, Type,
    def_data_type::{DataCons, Fields},
};

use crate::{
    ids::OwnedDottedName,
    type_sets::{ModuleTypeSet, PackageTypeSet, TypeSet},
    v2::dotted_name_to_owned,
};

/// Matrix which defines existing "paths" from one type to another
///
/// An element is set to true if there is a dependency path A -> ... -> B
///
/// Note that this type represents dependencies between type within a single module only.
/// It's used to identify recursive dependencies between types (we need to Box them in codegen).
#[derive(Clone, Debug)]
pub struct LocalDepMatrix(HashMap<OwnedDottedName, HashMap<OwnedDottedName, bool>>);

impl LocalDepMatrix {
    pub fn new(names: Vec<OwnedDottedName>) -> Self {
        let row = names
            .iter()
            .cloned()
            .map(|name| (name, false))
            .collect::<HashMap<_, _>>();
        Self(names.into_iter().map(|name| (name, row.clone())).collect())
    }

    pub fn get(&self, src: &OwnedDottedName, dst: &OwnedDottedName) -> Option<bool> {
        Some(*self.0.get(src)?.get(dst)?)
    }

    pub fn insert(&mut self, src: OwnedDottedName, dst: OwnedDottedName, value: bool) {
        self.0
            .entry(src)
            .and_modify(|x| {
                x.insert(dst.clone(), value);
            })
            .or_insert_with(|| HashMap::from([(dst, value)]));
    }
}

// pub struct ResolvedModuleDeps<'a> {
//     gen_set: ModuleGenSet<'a>,
//     local_deps_matrix: LocalDepMatrix<'a>,
// }

// impl<'a> ResolvedModuleDeps<'a> {
//     pub fn resolve(module: Module<'a>) -> Self {
//         let r = ModuleDepsResolver::resolve(module);
//         todo!()
//     }
// }

#[derive(Clone, Debug, Default)]
pub struct Deps {
    /// Dependencies from the same module
    pub direct: ModuleTypeSet,

    /// Dependencies from the same package, but different module
    pub local: PackageTypeSet,

    /// Dependencies from external packages
    pub external: TypeSet,
}

impl Deps {
    pub fn new() -> Self {
        Self {
            direct: ModuleTypeSet::new(),
            local: PackageTypeSet::new(),
            external: TypeSet::new(),
        }
    }

    pub fn extend(&mut self, other: Self) {
        let Self {
            direct,
            local,
            external,
        } = other;

        self.direct.join(direct);
        self.local.join(local);
        self.external.join(external);
    }
}

impl FromIterator<Deps> for Deps {
    fn from_iter<T: IntoIterator<Item = Deps>>(iter: T) -> Self {
        iter.into_iter().fold(Deps::new(), |mut acc, deps| {
            acc.extend(deps);
            acc
        })
    }
}

/// Type depenecies resolver
#[derive(Clone, Debug)]
pub struct ModuleDepsResolver<'a> {
    module: Module<'a>,
    module_name: DottedName<'a>,
    local_types: HashMap<OwnedDottedName, DefDataType<'a>>,
    local_deps_matrix: LocalDepMatrix,
}

impl<'a> ModuleDepsResolver<'a> {
    pub fn new(module: Module<'a>) -> Self {
        let module_name = module.name();
        let data_types = module.data_types();
        let local_types = data_types
            .iter()
            .map(|dt| (dotted_name_to_owned(&dt.name()), *dt))
            .collect::<HashMap<_, _>>();
        let names = data_types
            .iter()
            .map(|dt| dotted_name_to_owned(&dt.name()))
            .collect();
        let local_deps_matrix = LocalDepMatrix::new(names);

        Self {
            module,
            module_name,
            local_types,
            local_deps_matrix,
        }
    }

    pub fn local_deps_matrix(&self) -> &LocalDepMatrix {
        &self.local_deps_matrix
    }

    /// Find all dependencies of the local type with given name (recursive search)
    pub fn find_deps(&self, name: &OwnedDottedName) -> Deps {
        let dt = self.local_types[name];
        match dt.data_cons() {
            DataCons::Record(fields) => self.find_deps_from_fields(fields),
            DataCons::Variant(fields) => self.find_deps_from_fields(fields),
            _ => Deps::new(),
        }
    }

    pub fn find_deps_from_fields(&self, fields: Fields<'a>) -> Deps {
        let mut ret = Deps::new();
        for field in fields.fields() {
            ret.extend(self.find_deps_from_type(field.type_()));
        }
        ret
    }

    pub fn find_deps_from_type(&self, type_: Type<'a>) -> Deps {
        let mut deps = Deps::new();
        match type_ {
            Type::Var(var) => {
                deps.extend(
                    var.args()
                        .into_iter()
                        .map(|t| self.find_deps_from_type(t))
                        .collect(),
                );
            }
            Type::Con(con) => {
                let type_con_id = con.tycon();
                let module_id = type_con_id.module();
                let module_name = module_id.module_name();
                let package_id = module_id.package_id();
                let type_name = dotted_name_to_owned(&type_con_id.name());

                match package_id {
                    SelfOrImportedPackageId::SelfPackageId => {
                        if module_name == self.module_name {
                            deps.direct.as_mut().insert(type_name);
                        } else {
                            // Recursive search for mentioned module
                            let module = *self
                                .module
                                .package()
                                .modules()
                                .iter()
                                .find(|m| m.name() == module_name)
                                .unwrap();
                            let resolver = ModuleDepsResolver::new(module);
                            let subdeps = resolver.find_deps(&type_name);

                            let mut local = PackageTypeSet(BTreeMap::from([(
                                dotted_name_to_owned(&module_name),
                                subdeps.direct,
                            )]));
                            local.join(subdeps.local);

                            let resolved_subdeps = Deps {
                                direct: ModuleTypeSet::new(),
                                local,
                                external: subdeps.external,
                            };
                            deps.extend(resolved_subdeps);

                            deps.local
                                .insert_type(dotted_name_to_owned(&module_name), type_name);
                        }
                    }
                    SelfOrImportedPackageId::ImportedPackageId(package_id) => {
                        deps.external.insert_type(
                            PackageId::new_unchecked_owned(package_id.to_string()),
                            dotted_name_to_owned(&module_name),
                            type_name,
                        );
                    }
                }

                deps.extend(
                    con.args()
                        .into_iter()
                        .map(|t| self.find_deps_from_type(t))
                        .collect(),
                );
            }
            Type::Builtin(builtin) => {
                deps.extend(
                    builtin
                        .args()
                        .into_iter()
                        .map(|t| self.find_deps_from_type(t))
                        .collect(),
                );
            }
            Type::Nat => {}
            Type::Tapp(tapp) => {
                deps.extend(self.find_deps_from_type(tapp.lhs()));
                deps.extend(self.find_deps_from_type(tapp.rhs()));
            }
        }
        deps
    }

    pub fn build_dep_matrix(&mut self) {
        let data_types = self.module.data_types();
        for dt in data_types {
            let deps = self.find_deps(&dotted_name_to_owned(&dt.name()));
            for dep in deps.direct {
                self.local_deps_matrix
                    .insert(dotted_name_to_owned(&dt.name()), dep, true);
            }
        }
    }
}
