use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

use crate::v2::builder::{
    Type, TypeParameter,
    lower::{Lower, Lowerer},
};

/// An owned named field with its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    name: String,
    type_: Type,
}

impl Field {
    pub fn new(name: impl Into<String>, type_: Type) -> Self {
        Self {
            name: name.into(),
            type_,
        }
    }
}

impl Lower<proto::FieldWithType> for Field {
    fn lower(self, lowerer: &mut Lowerer) -> proto::FieldWithType {
        proto::FieldWithType {
            field_interned_str: lowerer.intern_string(self.name),
            r#type: Some(self.type_.lower(lowerer)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DataCons {
    Record(Vec<Field>),
    Variant(Vec<Field>),
    Enum(Vec<String>),
    Interface,
}

/// Builder for an LF data type definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataTypeBuilder {
    name: String,
    params: Vec<TypeParameter>,
    serializable: bool,
    cons: DataCons,
}

impl DataTypeBuilder {
    pub fn record(name: impl Into<String>, fields: impl IntoIterator<Item = Field>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            serializable: true,
            cons: DataCons::Record(fields.into_iter().collect()),
        }
    }

    pub fn variant(name: impl Into<String>, constructors: impl IntoIterator<Item = Field>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            serializable: true,
            cons: DataCons::Variant(constructors.into_iter().collect()),
        }
    }

    pub fn enumeration<S>(
        name: impl Into<String>,
        constructors: impl IntoIterator<Item = S>,
    ) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            params: Vec::new(),
            serializable: true,
            cons: DataCons::Enum(constructors.into_iter().map(Into::into).collect()),
        }
    }

    pub fn interface(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            serializable: true,
            cons: DataCons::Interface,
        }
    }

    pub fn type_parameter(mut self, parameter: TypeParameter) -> Self {
        self.params.push(parameter);
        self
    }

    pub fn serializable(mut self, serializable: bool) -> Self {
        self.serializable = serializable;
        self
    }
}

impl Lower<proto::DefDataType> for DataTypeBuilder {
    fn lower(self, lowerer: &mut Lowerer) -> proto::DefDataType {
        let data_cons = match self.cons {
            DataCons::Record(fields) => {
                proto::def_data_type::DataCons::Record(proto::def_data_type::Fields {
                    fields: fields
                        .into_iter()
                        .map(|field| field.lower(lowerer))
                        .collect(),
                })
            }
            DataCons::Variant(fields) => {
                proto::def_data_type::DataCons::Variant(proto::def_data_type::Fields {
                    fields: fields
                        .into_iter()
                        .map(|field| field.lower(lowerer))
                        .collect(),
                })
            }
            DataCons::Enum(constructors) => {
                proto::def_data_type::DataCons::Enum(proto::def_data_type::EnumConstructors {
                    constructors_interned_str: constructors
                        .into_iter()
                        .map(|constructor| lowerer.intern_string(constructor))
                        .collect(),
                })
            }
            DataCons::Interface => proto::def_data_type::DataCons::Interface(proto::Unit {}),
        };

        proto::DefDataType {
            location: None,
            name_interned_dname: lowerer.intern_dotted(&self.name),
            params: self
                .params
                .into_iter()
                .map(|parameter| parameter.lower(lowerer))
                .collect(),
            serializable: self.serializable,
            data_cons: Some(data_cons),
        }
    }
}
