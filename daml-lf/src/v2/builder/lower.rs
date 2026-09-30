use std::{collections::BTreeMap, mem};

use canton_types::PackageId;
use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

pub trait Lower<T> {
    /// Lower a builder type into its target protobuf type.
    fn lower(self, lowerer: &mut Lowerer) -> T;
}

#[derive(Default)]
pub struct Lowerer {
    strings: Vec<String>,
    string_indices: BTreeMap<String, i32>,
    dotted_names: Vec<proto::InternedDottedName>,
    dotted_name_indices: BTreeMap<Vec<String>, i32>,
    imports: Vec<String>,
    import_indices: BTreeMap<PackageId, i32>,
}

impl Lowerer {
    pub fn finish_package(
        &mut self,
        name_interned_str: i32,
        version_interned_str: i32,
        modules: Vec<proto::Module>,
    ) -> proto::Package {
        let Self {
            strings,
            dotted_names,
            imports,
            ..
        } = mem::take(self);
        let imports_sum = if imports.is_empty() {
            Some(proto::package::ImportsSum::NoImportedPackagesReason(
                "builder package has no imports".to_owned(),
            ))
        } else {
            Some(proto::package::ImportsSum::PackageImports(
                proto::PackageImports {
                    imported_packages: imports,
                },
            ))
        };

        proto::Package {
            modules,
            interned_strings: strings,
            interned_dotted_names: dotted_names,
            metadata: Some(proto::PackageMetadata {
                name_interned_str,
                version_interned_str,
                upgraded_package_id: None,
            }),
            interned_types: Vec::new(),
            interned_kinds: Vec::new(),
            interned_exprs: Vec::new(),
            imports_sum,
        }
    }

    pub fn intern_string(&mut self, value: String) -> i32 {
        if let Some(index) = self.string_indices.get(&value) {
            return *index;
        }

        let index =
            i32::try_from(self.strings.len()).expect("LF string table exceeds the i32 index range");
        self.strings.push(value.clone());
        self.string_indices.insert(value, index);
        index
    }

    pub fn intern_dotted(&mut self, value: &str) -> i32 {
        let segments = value.split('.').map(str::to_owned).collect::<Vec<_>>();
        if let Some(index) = self.dotted_name_indices.get(&segments) {
            return *index;
        }

        let segment_indices = segments
            .iter()
            .cloned()
            .map(|segment| self.intern_string(segment))
            .collect();
        let index = i32::try_from(self.dotted_names.len())
            .expect("LF dotted-name table exceeds the i32 index range");
        self.dotted_names.push(proto::InternedDottedName {
            segments_interned_str: segment_indices,
        });
        self.dotted_name_indices.insert(segments, index);
        index
    }

    pub fn intern_import(&mut self, package: PackageId) -> i32 {
        if let Some(index) = self.import_indices.get(&package) {
            return *index;
        }

        let index =
            i32::try_from(self.imports.len()).expect("LF import table exceeds the i32 index range");
        self.imports.push(package.to_string());
        self.import_indices.insert(package, index);
        index
    }
}
