use std::collections::HashMap;

use daml_lf::v2::sealed::{
    DefDataType, DottedName, Module, ModuleId, Type, TypeConId,
    def_data_type::{DataCons, Fields},
};

use crate::type_sets::ModuleTypeSet;

/// Matrix which defines existing "paths" from one type to another
///
/// An element is set to true if there is a dependency path A -> ... -> B
///
/// Note that this type represents dependencies between type within a single module only.
/// It's used to identify recursive dependencies between types (we need to Box them in codegen).
#[derive(Clone, Debug)]
pub struct LocalDepMatrix<'a>(HashMap<DottedName<'a>, HashMap<DottedName<'a>, bool>>);

impl<'a> LocalDepMatrix<'a> {
    pub fn new(names: Vec<DottedName<'a>>) -> Self {
        let row = names
            .iter()
            .cloned()
            .map(|name| (name, false))
            .collect::<HashMap<_, _>>();
        Self(names.into_iter().map(|name| (name, row.clone())).collect())
    }

    pub fn get(&self, src: &DottedName<'a>, dst: &DottedName<'a>) -> Option<bool> {
        Some(*self.0.get(src)?.get(dst)?)
    }

    pub fn insert(&mut self, src: DottedName<'a>, dst: DottedName<'a>, value: bool) {
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

/// Type depenecies resolver
#[derive(Clone, Debug)]
pub struct ModuleDepsResolver<'a> {
    module: Module<'a>,
    module_name: DottedName<'a>,
    local_types: HashMap<DottedName<'a>, DefDataType<'a>>,
    local_deps_matrix: LocalDepMatrix<'a>,
}

impl<'a> ModuleDepsResolver<'a> {
    pub fn resolve(module: Module<'a>) -> Self {
        let module_name = module.name();
        let data_types = module.data_types();
        let local_types = data_types
            .iter()
            .map(|dt| (dt.name(), *dt))
            .collect::<HashMap<_, _>>();
        let names = data_types.iter().map(|dt| dt.name()).collect();
        let local_deps_matrix = LocalDepMatrix::new(names);

        let mut self_ = Self {
            module,
            module_name,
            local_types,
            local_deps_matrix,
        };

        self_.build_dep_matrix();

        self_
    }

    pub fn take_gen_set(&mut self) -> ModuleTypeSet {
        todo!()
    }

    // Data type from this module
    pub fn is_local(&self, type_con_id: TypeConId<'a>) -> bool {
        self.is_self(type_con_id.module())
    }

    // Module ID points to this module
    pub fn is_self(&self, module_id: ModuleId<'a>) -> bool {
        module_id.package_id().is_self() && module_id.module_name() == self.module_name
    }

    /// Find all dependencies of the local type with given name (recursive search)
    pub fn find_deps(&self, name: DottedName<'a>) -> Vec<DottedName<'a>> {
        let dt = self.local_types[&name];
        match dt.data_cons() {
            DataCons::Record(fields) => self.find_deps_from_fields(fields),
            DataCons::Variant(fields) => self.find_deps_from_fields(fields),
            _ => Vec::new(),
        }
    }

    fn find_deps_from_fields(&self, fields: Fields<'a>) -> Vec<DottedName<'a>> {
        let mut ret = Vec::new();
        for field in fields.fields() {
            ret.extend(self.find_deps_from_type(field.type_()));
        }
        ret
    }

    fn find_deps_from_type(&self, type_: Type<'a>) -> Vec<DottedName<'a>> {
        let mut ret = Vec::new();
        match type_ {
            Type::Var(var) => {
                ret.extend(
                    var.args()
                        .into_iter()
                        .flat_map(|t| self.find_deps_from_type(t)),
                );
            }
            Type::Con(con) => {
                let type_con_id = con.tycon();
                if self.is_local(type_con_id) {
                    ret.push(type_con_id.name());
                }
                ret.extend(self.find_deps(type_con_id.name()));
                ret.extend(
                    con.args()
                        .into_iter()
                        .flat_map(|t| self.find_deps_from_type(t)),
                );
            }
            Type::Builtin(builtin) => {
                ret.extend(
                    builtin
                        .args()
                        .into_iter()
                        .flat_map(|t| self.find_deps_from_type(t)),
                );
            }
            Type::Nat => {}
            Type::Tapp(tapp) => {
                ret.extend(self.find_deps_from_type(tapp.lhs()));
                ret.extend(self.find_deps_from_type(tapp.rhs()));
            }
        }
        ret
    }

    fn build_dep_matrix(&mut self) {
        let data_types = self.module.data_types();
        for dt in data_types {
            let deps = self.find_deps(dt.name());
            for dep in deps {
                self.local_deps_matrix.insert(dt.name(), dep, true);
            }
        }
    }
}
