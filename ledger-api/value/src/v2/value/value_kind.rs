use std::fmt;

/// Kind of a value
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueKind {
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
    TextMap,
    GenMap,
    Record,
    Variant,
    Enum,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ValueKind::Unit => "Unit",
            ValueKind::Bool => "Bool",
            ValueKind::Int64 => "Int64",
            ValueKind::Date => "Date",
            ValueKind::Timestamp => "Timestamp",
            ValueKind::Numeric => "Numeric",
            ValueKind::Party => "Party",
            ValueKind::Text => "Text",
            ValueKind::ContractId => "ContractId",
            ValueKind::Optional => "Optional",
            ValueKind::List => "List",
            ValueKind::TextMap => "TextMap",
            ValueKind::GenMap => "GenMap",
            ValueKind::Record => "Record",
            ValueKind::Variant => "Variant",
            ValueKind::Enum => "Enum",
        })
    }
}
