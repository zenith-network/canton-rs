use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};

use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::sealed::{DottedName, FieldWithType, Module, Package, TypeVarWithKind};

#[derive(Clone, Copy)]
pub struct DefDataType<'a> {
    module: Module<'a>,
    index: usize,
}

impl<'a> DefDataType<'a> {
    pub fn as_unsealed(&self) -> &'a proto::DefDataType {
        &self.module.as_unsealed().data_types[self.index]
    }

    pub fn module(&self) -> Module<'a> {
        self.module
    }

    pub fn package(&self) -> Package<'a> {
        self.module.package()
    }

    pub fn name(&self) -> DottedName<'a> {
        self.package()
            .get_interned_dotted_names(self.as_unsealed().name_interned_dname)
    }

    pub fn serializable(&self) -> bool {
        self.as_unsealed().serializable
    }

    pub fn params(&self) -> Vec<TypeVarWithKind<'a>> {
        self.as_unsealed()
            .params
            .iter()
            .map(|t| TypeVarWithKind::from_unsealed(t, self.package()))
            .collect()
    }

    pub fn data_cons(&self) -> DataCons<'a> {
        use proto::def_data_type::DataCons::*;

        match self.as_unsealed().data_cons.as_ref().unwrap() {
            Record(fields) => DataCons::Record(Fields::from_unsealed(fields, self.package())),
            Variant(fields) => DataCons::Variant(Fields::from_unsealed(fields, self.package())),
            Enum(enum_constructors) => DataCons::Enum(EnumConstructors::from_unsealed(
                enum_constructors,
                self.package(),
            )),
            Interface(_) => DataCons::Interface,
        }
    }

    pub(crate) fn from_unsealed(module: Module<'a>, index: usize) -> Self {
        Self { module, index }
    }
}

impl fmt::Debug for DefDataType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefDataType")
            .field("module", &format!("{:#?}", self.module))
            .field("serializable", &self.serializable())
            .field("params", &self.params())
            .field("data_cons", &self.data_cons())
            .finish()
    }
}

impl<'a, 'b> PartialEq<DefDataType<'b>> for DefDataType<'a> {
    fn eq(&self, other: &DefDataType<'b>) -> bool {
        self.module == other.module && self.index == other.index
    }
}

impl Eq for DefDataType<'_> {}

impl Hash for DefDataType<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.index.hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataCons<'a> {
    Record(Fields<'a>),
    Variant(Fields<'a>),
    Enum(EnumConstructors<'a>),
    Interface,
}

#[derive(Clone, Copy)]
pub struct Fields<'a> {
    package: Package<'a>,
    unsealed: &'a proto::def_data_type::Fields,
}

impl<'a> Fields<'a> {
    pub fn as_unsealed(&self) -> &'a proto::def_data_type::Fields {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn fields(&self) -> Vec<FieldWithType<'a>> {
        self.unsealed
            .fields
            .iter()
            .map(|f| FieldWithType::from_unsealed(f, self.package))
            .collect()
    }

    fn from_unsealed(unsealed: &'a proto::def_data_type::Fields, package: Package<'a>) -> Self {
        Self { package, unsealed }
    }
}

impl fmt::Debug for Fields<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fields")
            .field("package", &format_args!("{:#?}", self.package))
            .field("fields", &self.fields())
            .finish()
    }
}

impl<'a, 'b> PartialEq<Fields<'b>> for Fields<'a> {
    fn eq(&self, other: &Fields<'b>) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for Fields<'_> {}

impl Hash for Fields<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}

#[derive(Clone, Copy)]
pub struct EnumConstructors<'a> {
    package: Package<'a>,
    unsealed: &'a proto::def_data_type::EnumConstructors,
}

impl<'a> EnumConstructors<'a> {
    pub fn as_unsealed(&self) -> &'a proto::def_data_type::EnumConstructors {
        self.unsealed
    }

    pub fn package(&self) -> Package<'a> {
        self.package
    }

    pub fn constructors(&self) -> Vec<&'a str> {
        self.unsealed
            .constructors_interned_str
            .iter()
            .map(|idx| self.package.get_interned_string(*idx))
            .collect()
    }

    fn from_unsealed(
        unsealed: &'a proto::def_data_type::EnumConstructors,
        package: Package<'a>,
    ) -> Self {
        Self { package, unsealed }
    }
}

impl fmt::Debug for EnumConstructors<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnumConstructors")
            .field("package", &format_args!("{:#?}", self.package))
            .field("constructors", &self.constructors())
            .finish()
    }
}

impl<'a, 'b> PartialEq<EnumConstructors<'b>> for EnumConstructors<'a> {
    fn eq(&self, other: &EnumConstructors<'b>) -> bool {
        self.package == other.package && ptr::eq(self.unsealed, other.unsealed)
    }
}

impl Eq for EnumConstructors<'_> {}

impl Hash for EnumConstructors<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        ptr::hash(self.unsealed, state);
    }
}
