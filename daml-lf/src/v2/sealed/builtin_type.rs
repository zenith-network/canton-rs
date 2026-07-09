use daml_lf_archive_proto::com::digitalasset::daml::lf::archive::v2 as proto;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BuiltinType {
    Unit,
    Bool,
    Int64,
    Date,
    Timestamp,
    Numeric,
    Party,
    Text,
    ContractId,
    Optional,
    List,
    Genmap,
    Any,
    AnyException,
    TypeRep,
    Arrow,
    Update,
    FailureCategory,
    Textmap,
    Bignumeric,
    RoundingMode,
}

impl BuiltinType {
    pub fn from_unsealed(unsealed: proto::BuiltinType) -> Self {
        unsealed.into()
    }
}

impl From<proto::BuiltinType> for BuiltinType {
    fn from(value: proto::BuiltinType) -> Self {
        match value {
            proto::BuiltinType::Unit => BuiltinType::Unit,
            proto::BuiltinType::Bool => BuiltinType::Bool,
            proto::BuiltinType::Int64 => BuiltinType::Int64,
            proto::BuiltinType::Date => BuiltinType::Date,
            proto::BuiltinType::Timestamp => BuiltinType::Timestamp,
            proto::BuiltinType::Numeric => BuiltinType::Numeric,
            proto::BuiltinType::Party => BuiltinType::Party,
            proto::BuiltinType::Text => BuiltinType::Text,
            proto::BuiltinType::ContractId => BuiltinType::ContractId,
            proto::BuiltinType::Optional => BuiltinType::Optional,
            proto::BuiltinType::List => BuiltinType::List,
            proto::BuiltinType::Genmap => BuiltinType::Genmap,
            proto::BuiltinType::Any => BuiltinType::Any,
            proto::BuiltinType::AnyException => BuiltinType::AnyException,
            proto::BuiltinType::TypeRep => BuiltinType::TypeRep,
            proto::BuiltinType::Arrow => BuiltinType::Arrow,
            proto::BuiltinType::Update => BuiltinType::Update,
            proto::BuiltinType::FailureCategory => BuiltinType::FailureCategory,
            proto::BuiltinType::Textmap => BuiltinType::Textmap,
            proto::BuiltinType::Bignumeric => BuiltinType::Bignumeric,
            proto::BuiltinType::RoundingMode => BuiltinType::RoundingMode,
        }
    }
}
