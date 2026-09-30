//! Dependency tree definitions

use canton_types::PackageId;
use daml_lf::v2::sealed::{DefDataType, DefInterface, DefTemplate, DottedName, Module, Package};

/// Definition of an item in Daml LF v2 package ([`Def`] with a package ID)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Definition<'a> {
    package_id: &'a PackageId,
    definition: Def<'a>,
}

impl<'a> Definition<'a> {
    pub fn new(package_id: &'a PackageId, definition: Def<'a>) -> Self {
        Self {
            package_id,
            definition,
        }
    }

    pub fn module(&self) -> Module<'a> {
        self.definition.module()
    }

    pub fn package(&self) -> Package<'a> {
        self.definition.package()
    }

    pub fn package_id(&self) -> &'a PackageId {
        self.package_id
    }

    pub fn definition(&self) -> Def<'a> {
        self.definition
    }
}

/// Definition of an item in Daml LF v2
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Def<'a> {
    /// Definition of a template
    Template {
        template: DefTemplate<'a>,
        /// Data types of templates are tightly bounded with templates, so we can store it here
        data_type: DefDataType<'a>,
    },

    /// Definition of a regular data type
    DataType(DefDataType<'a>),

    /// Definition of an interface
    Interface {
        interface: DefInterface<'a>,
        /// Interfaces have "empty" data types attached to them (with Interface data constructor)
        data_type: DefDataType<'a>,
    },
}

impl<'a> Def<'a> {
    /// Underlying data type (attached argument type for templates and interfaces or type itself
    /// for data type definitions)
    pub fn data_type(&self) -> DefDataType<'a> {
        match self {
            Def::Template { data_type, .. } => *data_type,
            Def::DataType(def_data_type) => *def_data_type,
            Def::Interface { data_type, .. } => *data_type,
        }
    }

    /// Module of the definition (Daml LF v2)
    pub fn module(&self) -> Module<'a> {
        self.data_type().module()
    }

    /// Package of the definition (Daml LF v2)
    pub fn package(&self) -> Package<'a> {
        self.data_type().package()
    }

    /// Name of the defined entity (Daml LF v2)
    ///
    /// For choices returns dotted name of the underlying data type
    pub fn name(&self) -> DottedName<'a> {
        self.data_type().name()
    }
}
